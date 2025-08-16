#!/usr/bin/env node

import { fileURLToPath } from 'url';
import { dirname } from 'path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { z } from 'zod';
import { KotaDBStorage, Document, SearchResult } from './kotadb-storage.js';

// Tool schemas
const DocumentCreateSchema = z.object({
  path: z.string().describe('Unique path identifier for the document'),
  title: z.string().optional().describe('Optional title for the document'),
  content: z.string().describe('The main content of the document'),
  tags: z.array(z.string()).optional().describe('Optional tags for categorization'),
});

const DocumentGetSchema = z.object({
  id: z.string().describe('Document ID to retrieve'),
});

const DocumentUpdateSchema = z.object({
  id: z.string().describe('Document ID to update'),
  content: z.string().describe('New content for the document'),
});

const DocumentDeleteSchema = z.object({
  id: z.string().describe('Document ID to delete'),
});

const DocumentListSchema = z.object({
  limit: z.number().optional().default(100).describe('Maximum number of documents to return'),
  offset: z.number().optional().default(0).describe('Number of documents to skip'),
});

const SearchSchema = z.object({
  query: z.string().describe('Search query text'),
  limit: z.number().optional().default(10).describe('Maximum number of results'),
});

class KotaDBMCPServer {
  private server: Server;
  private storage: KotaDBStorage;

  constructor() {
    this.server = new Server(
      {
        name: 'kotadb-mcp',
        version: '0.5.0',
      },
      {
        capabilities: {
          tools: {},
          resources: {},
        },
      }
    );

    this.storage = new KotaDBStorage(process.env.KOTADB_DATA_DIR);
    this.setupHandlers();
  }

  private setupHandlers() {
    // Tool definitions
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: [
          {
            name: 'kotadb_document_create',
            description: 'Create a new document in KotaDB with content, title, and tags',
            inputSchema: {
              type: 'object',
              properties: {
                path: { type: 'string', description: 'Unique path identifier for the document' },
                title: { type: 'string', description: 'Optional title for the document' },
                content: { type: 'string', description: 'The main content of the document' },
                tags: { 
                  type: 'array', 
                  items: { type: 'string' },
                  description: 'Optional tags for categorization'
                },
              },
              required: ['path', 'content'],
            },
          },
          {
            name: 'kotadb_document_get',
            description: 'Retrieve a document by its ID',
            inputSchema: {
              type: 'object',
              properties: {
                id: { type: 'string', description: 'Document ID to retrieve' },
              },
              required: ['id'],
            },
          },
          {
            name: 'kotadb_document_update',
            description: 'Update the content of an existing document',
            inputSchema: {
              type: 'object',
              properties: {
                id: { type: 'string', description: 'Document ID to update' },
                content: { type: 'string', description: 'New content for the document' },
              },
              required: ['id', 'content'],
            },
          },
          {
            name: 'kotadb_document_delete',
            description: 'Delete a document by its ID',
            inputSchema: {
              type: 'object',
              properties: {
                id: { type: 'string', description: 'Document ID to delete' },
              },
              required: ['id'],
            },
          },
          {
            name: 'kotadb_document_list',
            description: 'List all documents with optional pagination',
            inputSchema: {
              type: 'object',
              properties: {
                limit: { type: 'number', description: 'Maximum number of documents to return' },
                offset: { type: 'number', description: 'Number of documents to skip' },
              },
            },
          },
          {
            name: 'kotadb_search',
            description: 'Search documents in KotaDB using text queries',
            inputSchema: {
              type: 'object',
              properties: {
                query: { type: 'string', description: 'Search query text' },
                limit: { type: 'number', description: 'Maximum number of results' },
              },
              required: ['query'],
            },
          },
          {
            name: 'kotadb_stats',
            description: 'Get database statistics and information',
            inputSchema: {
              type: 'object',
              properties: {},
            },
          },
        ],
      };
    });

    // Tool handlers
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      try {
        switch (request.params.name) {
          case 'kotadb_document_create': {
            const args = DocumentCreateSchema.parse(request.params.arguments);
            const document = await this.storage.createDocument(args);
            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    document: {
                      id: document.id,
                      path: document.path,
                      title: document.title,
                      content: document.content,
                      tags: document.tags,
                      created_at: document.createdAt,
                      updated_at: document.updatedAt,
                    },
                    message: `Document created successfully at ${document.path}`,
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_document_get': {
            const args = DocumentGetSchema.parse(request.params.arguments);
            const document = await this.storage.getDocument(args.id);
            
            if (!document) {
              return {
                content: [
                  {
                    type: 'text',
                    text: JSON.stringify({
                      success: false,
                      error: `Document with ID ${args.id} not found`,
                    }, null, 2),
                  },
                ],
                isError: true,
              };
            }

            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    document,
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_document_update': {
            const args = DocumentUpdateSchema.parse(request.params.arguments);
            const document = await this.storage.updateDocument(args.id, args.content);
            
            if (!document) {
              return {
                content: [
                  {
                    type: 'text',
                    text: JSON.stringify({
                      success: false,
                      error: `Document with ID ${args.id} not found`,
                    }, null, 2),
                  },
                ],
                isError: true,
              };
            }

            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    document: {
                      id: document.id,
                      path: document.path,
                      title: document.title,
                      content: document.content,
                      tags: document.tags,
                      created_at: document.createdAt,
                      updated_at: document.updatedAt,
                    },
                    message: 'Document updated successfully',
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_document_delete': {
            const args = DocumentDeleteSchema.parse(request.params.arguments);
            const deleted = await this.storage.deleteDocument(args.id);
            
            if (!deleted) {
              return {
                content: [
                  {
                    type: 'text',
                    text: JSON.stringify({
                      success: false,
                      error: `Document with ID ${args.id} not found`,
                    }, null, 2),
                  },
                ],
                isError: true,
              };
            }

            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    message: `Document ${args.id} deleted successfully`,
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_document_list': {
            const args = DocumentListSchema.parse(request.params.arguments);
            const result = await this.storage.listDocuments(args.limit, args.offset);
            
            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    documents: result.documents.map(doc => ({
                      id: doc.id,
                      path: doc.path,
                      title: doc.title,
                      tags: doc.tags,
                      created_at: doc.createdAt,
                      updated_at: doc.updatedAt,
                    })),
                    total: result.total,
                    limit: args.limit,
                    offset: args.offset,
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_search': {
            const args = SearchSchema.parse(request.params.arguments);
            const results = await this.storage.searchDocuments(args.query, args.limit);
            
            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    query: args.query,
                    results: results,
                    total_results: results.length,
                  }, null, 2),
                },
              ],
            };
          }

          case 'kotadb_stats': {
            const stats = await this.storage.getStats();
            
            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify({
                    success: true,
                    stats,
                  }, null, 2),
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
              text: JSON.stringify({
                success: false,
                error: errorMessage,
              }, null, 2),
            },
          ],
          isError: true,
        };
      }
    });

    // Resource handlers
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      return {
        resources: [
          {
            uri: 'kotadb://documents',
            name: 'All Documents',
            description: 'Browse all documents in KotaDB',
            mimeType: 'application/json',
          },
        ],
      };
    });

    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const uri = request.params.uri;
      
      if (uri === 'kotadb://documents') {
        const { documents, total } = await this.storage.listDocuments();
        return {
          contents: [
            {
              uri: uri,
              mimeType: 'application/json',
              text: JSON.stringify({
                documents: documents.map(doc => ({
                  id: doc.id,
                  path: doc.path,
                  title: doc.title,
                  tags: doc.tags,
                  created_at: doc.createdAt,
                  updated_at: doc.updatedAt,
                })),
                total,
              }, null, 2),
            },
          ],
        };
      }
      
      throw new Error(`Unknown resource: ${uri}`);
    });
  }

  async start() {
    try {
      await this.storage.initialize();
      const transport = new StdioServerTransport();
      await this.server.connect(transport);
      console.error('KotaDB MCP server running on stdio - 7 tools available');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error('Failed to start KotaDB MCP server:', errorMessage);
      process.exit(1);
    }
  }
}

// Handle graceful shutdown
process.on('SIGINT', () => {
  console.error('KotaDB MCP server shutting down...');
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.error('KotaDB MCP server shutting down...');
  process.exit(0);
});

// Start the server
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Check if this module is the entry point
if (import.meta.url === `file://${process.argv[1]}`) {
  const server = new KotaDBMCPServer();
  server.start().catch((error) => {
    console.error('Error starting server:', error);
    process.exit(1);
  });
}