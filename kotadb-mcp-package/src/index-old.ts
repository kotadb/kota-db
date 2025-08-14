#!/usr/bin/env node

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { spawn } from 'child_process';
import { promisify } from 'util';
import { exec } from 'child_process';
import * as path from 'path';
import * as fs from 'fs/promises';
import * as os from 'os';
import { z } from 'zod';
import { BinaryInstaller } from './install-binary.js';

const execAsync = promisify(exec);

// KotaDB binary interface
class KotaDBClient {
  private binaryPath: string;
  private dataDir: string;

  constructor(dataDir?: string) {
    this.dataDir = dataDir || path.join(process.env.HOME || process.cwd(), '.kotadb', 'data');
    this.binaryPath = this.findKotaDBBinary();
  }

  private findKotaDBBinary(): string {
    // Use the binary installer to find the correct binary path
    try {
      return BinaryInstaller.findBinary();
    } catch (error) {
      console.warn('Warning: Could not locate KotaDB binary, falling back to PATH');
      return 'kotadb';
    }
  }

  private validatePath(inputPath: string): string {
    // Basic path validation to prevent command injection
    if (!inputPath || inputPath.trim() === '') {
      throw new Error('Path cannot be empty');
    }
    
    // Remove dangerous characters
    const sanitized = inputPath.replace(/[;&|`$(){}\[\]]/g, '');
    
    // Ensure it's a reasonable path
    if (sanitized !== inputPath) {
      throw new Error('Path contains invalid characters');
    }
    
    // Ensure it starts with / for absolute paths or is relative
    if (sanitized.includes('..')) {
      throw new Error('Path cannot contain .. for security reasons');
    }
    
    return sanitized;
  }

  async ensureBinary(): Promise<void> {
    try {
      await execAsync(`${this.binaryPath} --version`);
    } catch (error) {
      throw new Error(
        `KotaDB binary not found at ${this.binaryPath}. Please ensure KotaDB is installed or provide the correct path.`
      );
    }
  }

  async runCommand(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const child = spawn(this.binaryPath, args, {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          KOTADB_DATA_DIR: this.dataDir,
        },
      });

      let stdout = '';
      let stderr = '';

      child.stdout?.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr?.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('close', (code) => {
        if (code === 0) {
          resolve(stdout);
        } else {
          reject(new Error(`KotaDB command failed with code ${code}: ${stderr}`));
        }
      });

      child.on('error', (error) => {
        reject(new Error(`Failed to spawn KotaDB process: ${error.message}`));
      });
    });
  }

  // Document operations
  async createDocument(params: {
    path: string;
    title?: string;
    content: string;
    tags?: string[];
  }): Promise<any> {
    try {
      // Validate and sanitize the path
      const validatedPath = this.validatePath(params.path);
      
      // Create temporary file for content
      const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'kotadb-'));
      const tempFile = path.join(tempDir, 'document.md');
      
      try {
        // Write content to temporary file
        await fs.writeFile(tempFile, params.content, 'utf8');
        
        // Build command arguments
        const args = ['add', validatedPath, '--file', tempFile];
        
        if (params.title) {
          // Sanitize title to prevent injection
          const sanitizedTitle = params.title.replace(/[;&|`$()]/g, '');
          args.push('--title', sanitizedTitle);
        }
        
        if (params.tags && params.tags.length > 0) {
          // Sanitize tags
          const sanitizedTags = params.tags
            .map(tag => tag.replace(/[;&|`$(),]/g, ''))
            .filter(tag => tag.length > 0);
          if (sanitizedTags.length > 0) {
            args.push('--tags', sanitizedTags.join(','));
          }
        }
        
        const result = await this.runCommand(args);
        
        // Parse the result if it's JSON, otherwise return as text
        let parsedResult;
        try {
          parsedResult = JSON.parse(result);
        } catch {
          parsedResult = { success: true, message: result.trim() };
        }
        
        return parsedResult;
        
      } finally {
        // Clean up temporary file and directory
        try {
          await fs.unlink(tempFile);
          await fs.rmdir(tempDir);
        } catch (cleanupError) {
          console.warn('Failed to clean up temp file:', cleanupError);
        }
      }
    } catch (error) {
      throw new Error(`Document creation failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async searchDocuments(query: string, limit?: number): Promise<any> {
    try {
      // Sanitize query to prevent command injection
      if (!query || query.trim() === '') {
        throw new Error('Query cannot be empty');
      }
      
      const sanitizedQuery = query.replace(/[;&|`$()]/g, '');
      const args = ['search', sanitizedQuery];
      
      if (limit && limit > 0) {
        const safeLimit = Math.min(Math.max(1, Math.floor(limit)), 1000); // Limit between 1-1000
        args.push('--limit', safeLimit.toString());
      }
      
      const result = await this.runCommand(args);
      
      // Try to parse as JSON, fallback to text if not JSON
      let parsedResult;
      try {
        parsedResult = JSON.parse(result);
      } catch {
        // If not JSON, wrap the text result
        parsedResult = {
          results: result.trim().split('\n').filter(line => line.trim() !== ''),
          query: sanitizedQuery,
          limit: limit || 10
        };
      }
      
      return parsedResult;
    } catch (error) {
      throw new Error(`Search failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async getStats(): Promise<any> {
    try {
      const result = await this.runCommand(['stats']);
      
      // Try to parse as JSON, fallback to text
      let parsedResult;
      try {
        parsedResult = JSON.parse(result);
      } catch {
        parsedResult = {
          stats: result.trim(),
          timestamp: new Date().toISOString()
        };
      }
      
      return parsedResult;
    } catch (error) {
      throw new Error(`Failed to get stats: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  // Additional document operations to match Rust MCP server
  async getDocument(docId: string): Promise<any> {
    try {
      const validatedId = this.validatePath(docId);
      const result = await this.runCommand(['get', validatedId]);
      
      let parsedResult;
      try {
        parsedResult = JSON.parse(result);
      } catch {
        parsedResult = {
          id: validatedId,
          content: result.trim()
        };
      }
      
      return parsedResult;
    } catch (error) {
      throw new Error(`Failed to get document: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async updateDocument(docId: string, content: string): Promise<any> {
    try {
      const validatedId = this.validatePath(docId);
      
      // Create temporary file for new content
      const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'kotadb-update-'));
      const tempFile = path.join(tempDir, 'update.md');
      
      try {
        await fs.writeFile(tempFile, content, 'utf8');
        const result = await this.runCommand(['update', validatedId, '--file', tempFile]);
        
        let parsedResult;
        try {
          parsedResult = JSON.parse(result);
        } catch {
          parsedResult = { success: true, message: result.trim() };
        }
        
        return parsedResult;
        
      } finally {
        try {
          await fs.unlink(tempFile);
          await fs.rmdir(tempDir);
        } catch (cleanupError) {
          console.warn('Failed to clean up temp file:', cleanupError);
        }
      }
    } catch (error) {
      throw new Error(`Failed to update document: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async deleteDocument(docId: string): Promise<any> {
    try {
      const validatedId = this.validatePath(docId);
      const result = await this.runCommand(['delete', validatedId]);
      
      let parsedResult;
      try {
        parsedResult = JSON.parse(result);
      } catch {
        parsedResult = { success: true, message: result.trim() };
      }
      
      return parsedResult;
    } catch (error) {
      throw new Error(`Failed to delete document: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async listDocuments(limit?: number, offset?: number): Promise<any> {
    try {
      const args = ['list'];
      
      if (limit && limit > 0) {
        const safeLimit = Math.min(Math.max(1, Math.floor(limit)), 1000);
        args.push('--limit', safeLimit.toString());
      }
      
      if (offset && offset >= 0) {
        args.push('--offset', Math.floor(offset).toString());
      }
      
      const result = await this.runCommand(args);
      
      let parsedResult;
      try {
        parsedResult = JSON.parse(result);
      } catch {
        // Parse line-based output into structured format
        const lines = result.trim().split('\n').filter(line => line.trim() !== '');
        parsedResult = {
          documents: lines.map(line => ({ path: line.trim() })),
          total: lines.length,
          limit: limit || 100,
          offset: offset || 0
        };
      }
      
      return parsedResult;
    } catch (error) {
      throw new Error(`Failed to list documents: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}

// Define tool schemas
const DocumentCreateSchema = z.object({
  path: z.string().describe('Unique path identifier for the document'),
  title: z.string().optional().describe('Optional title for the document'),
  content: z.string().describe('The main content of the document'),
  tags: z.array(z.string()).optional().describe('Optional tags for categorization'),
});

const DocumentGetSchema = z.object({
  id: z.string().describe('Document ID or path to retrieve'),
});

const DocumentUpdateSchema = z.object({
  id: z.string().describe('Document ID or path to update'),
  content: z.string().describe('New content for the document'),
});

const DocumentDeleteSchema = z.object({
  id: z.string().describe('Document ID or path to delete'),
});

const DocumentListSchema = z.object({
  limit: z.number().optional().default(100).describe('Maximum number of documents to return'),
  offset: z.number().optional().default(0).describe('Number of documents to skip'),
});

const SearchSchema = z.object({
  query: z.string().describe('Search query text'),
  limit: z.number().optional().default(10).describe('Maximum number of results'),
});

const server = new Server(
  {
    name: 'kotadb-mcp',
    version: '0.1.0',
  },
  {
    capabilities: {
      tools: {},
      resources: {},
    },
  }
);

// Initialize KotaDB client
const kotadb = new KotaDBClient(process.env.KOTADB_DATA_DIR);

// Tool definitions
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: 'kotadb_document_create',
        description: 'Create a new document in KotaDB with content, title, and tags',
        inputSchema: DocumentCreateSchema,
      },
      {
        name: 'kotadb_document_get',
        description: 'Retrieve a document by its ID or path',
        inputSchema: DocumentGetSchema,
      },
      {
        name: 'kotadb_document_update',
        description: 'Update the content of an existing document',
        inputSchema: DocumentUpdateSchema,
      },
      {
        name: 'kotadb_document_delete',
        description: 'Delete a document by its ID or path',
        inputSchema: DocumentDeleteSchema,
      },
      {
        name: 'kotadb_document_list',
        description: 'List all documents with optional pagination',
        inputSchema: DocumentListSchema,
      },
      {
        name: 'kotadb_search',
        description: 'Search documents in KotaDB using natural language queries',
        inputSchema: SearchSchema,
      },
      {
        name: 'kotadb_stats',
        description: 'Get database statistics and information',
        inputSchema: z.object({}),
      },
    ],
  };
});

// Tool handlers
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  try {
    switch (request.params.name) {
      case 'kotadb_document_create': {
        const args = DocumentCreateSchema.parse(request.params.arguments);
        const result = await kotadb.createDocument(args);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_document_get': {
        const args = DocumentGetSchema.parse(request.params.arguments);
        const result = await kotadb.getDocument(args.id);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_document_update': {
        const args = DocumentUpdateSchema.parse(request.params.arguments);
        const result = await kotadb.updateDocument(args.id, args.content);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_document_delete': {
        const args = DocumentDeleteSchema.parse(request.params.arguments);
        const result = await kotadb.deleteDocument(args.id);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_document_list': {
        const args = DocumentListSchema.parse(request.params.arguments);
        const result = await kotadb.listDocuments(args.limit, args.offset);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_search': {
        const args = SearchSchema.parse(request.params.arguments);
        const result = await kotadb.searchDocuments(args.query, args.limit);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      case 'kotadb_stats': {
        const result = await kotadb.getStats();
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      }

      default:
        throw new Error(`Unknown tool: ${request.params.name}`);
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify({ error: errorMessage }, null, 2),
        },
      ],
      isError: true,
    };
  }
});

// Resource handlers (for browsing documents)
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: 'kotadb://documents',
        name: 'All Documents',
        description: 'Browse all documents in KotaDB',
      },
    ],
  };
});

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;
  
  if (uri === 'kotadb://documents') {
    try {
      const documents = await kotadb.listDocuments(100, 0);
      return {
        contents: [
          {
            uri: uri,
            mimeType: 'application/json',
            text: JSON.stringify(documents, null, 2),
          },
        ],
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to read resource: ${errorMessage}`);
    }
  }
  
  throw new Error(`Unknown resource: ${uri}`);
});

async function main() {
  try {
    // Ensure KotaDB binary is available
    await kotadb.ensureBinary();
    
    const transport = new StdioServerTransport();
    await server.connect(transport);
    
    console.error('KotaDB MCP server running on stdio - 7 tools available');
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('Failed to start KotaDB MCP server:', errorMessage);
    process.exit(1);
  }
}

// Handle graceful shutdown
process.on('SIGINT', async () => {
  await server.close();
  process.exit(0);
});

if (require.main === module) {
  main().catch((error) => {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('Error:', errorMessage);
    process.exit(1);
  });
}