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
import { z } from 'zod';

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
    // Try different locations for the KotaDB binary
    const possiblePaths = [
      'kotadb',
      './kotadb',
      '../target/release/kotadb',
      '../target/debug/kotadb',
      path.join(process.env.HOME || '', '.cargo', 'bin', 'kotadb'),
    ];

    // For now, assume the binary is in PATH or return the first option
    return 'kotadb';
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
    // For now, use the CLI interface - in the future we could use the MCP server directly
    const args = ['add', params.path];
    if (params.title) {
      args.push('--title', params.title);
    }
    if (params.tags && params.tags.length > 0) {
      args.push('--tags', params.tags.join(','));
    }

    // Write content to temporary file and pass path
    // This is a simplified implementation - real implementation would handle this better
    const result = await this.runCommand(args);
    return { success: true, message: result };
  }

  async searchDocuments(query: string, limit?: number): Promise<any> {
    const args = ['search', query];
    if (limit) {
      args.push('--limit', limit.toString());
    }

    const result = await this.runCommand(args);
    return { results: result };
  }

  async getStats(): Promise<any> {
    const result = await this.runCommand(['stats']);
    return { stats: result };
  }
}

// Define tool schemas
const DocumentCreateSchema = z.object({
  path: z.string().describe('Unique path identifier for the document'),
  title: z.string().optional().describe('Optional title for the document'),
  content: z.string().describe('The main content of the document'),
  tags: z.array(z.string()).optional().describe('Optional tags for categorization'),
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
        name: 'kotadb_search',
        description: 'Search documents in KotaDB using natural language queries',
        inputSchema: SearchSchema,
      },
      {
        name: 'kotadb_create_document',
        description: 'Create a new document in KotaDB',
        inputSchema: DocumentCreateSchema,
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

      case 'kotadb_create_document': {
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
    throw new Error(`Tool execution failed: ${error}`);
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
      const stats = await kotadb.getStats();
      return {
        contents: [
          {
            uri: uri,
            mimeType: 'application/json',
            text: JSON.stringify(stats, null, 2),
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to read resource: ${error}`);
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
    
    console.error('KotaDB MCP server running on stdio');
  } catch (error) {
    console.error('Failed to start KotaDB MCP server:', error);
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
    console.error('Error:', error);
    process.exit(1);
  });
}