/**
 * KotaDB TypeScript/JavaScript Client
 * 
 * Provides a simple, PostgreSQL-like interface for document operations.
 */

import axios, { AxiosInstance, AxiosResponse, AxiosError } from 'axios';
import {
  Document,
  SearchResult,
  QueryResult,
  CreateDocumentRequest,
  UpdateDocumentRequest,
  SearchOptions,
  SemanticSearchOptions,
  HybridSearchOptions,
  ConnectionConfig,
  HealthStatus,
  DatabaseStats,
  DocumentInput,
  DocumentUpdate,
  KotaDBError,
  ConnectionError,
  ValidationError,
  NotFoundError,
  ServerError
} from './types';

/**
 * KotaDB client for easy database operations.
 * 
 * Provides a simple, PostgreSQL-like interface for document operations.
 * 
 * @example
 * ```typescript
 * // Connect using URL
 * const db = new KotaDB({ url: 'http://localhost:8080' });
 * 
 * // Connect using environment variable
 * const db = new KotaDB(); // Uses KOTADB_URL
 * 
 * // Connect with connection string
 * const db = new KotaDB({ url: 'kotadb://localhost:8080/myapp' });
 * 
 * // Basic operations
 * const results = await db.query('rust patterns');
 * const docId = await db.insert({
 *   path: '/notes/meeting.md',
 *   title: 'My Note',
 *   content: '...',
 *   tags: ['work']
 * });
 * const doc = await db.get(docId);
 * await db.delete(docId);
 * ```
 */
export class KotaDB {
  private client: AxiosInstance;
  private baseUrl: string;

  constructor(config: ConnectionConfig = {}) {
    this.baseUrl = this.parseUrl(config.url);
    
    this.client = axios.create({
      baseURL: this.baseUrl,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...config.headers
      }
    });

    // Setup request/response interceptors for error handling
    this.setupInterceptors(config.retries || 3);
  }

  private parseUrl(url?: string): string {
    if (!url) {
      url = process.env.KOTADB_URL;
      if (!url) {
        throw new ConnectionError('No URL provided and KOTADB_URL environment variable not set');
      }
    }

    // Handle kotadb:// connection strings
    if (url.startsWith('kotadb://')) {
      const parsed = new URL(url);
      return `http://${parsed.host}`;
    }

    // Ensure URL has protocol
    if (!url.startsWith('http://') && !url.startsWith('https://')) {
      url = `http://${url}`;
    }

    // Remove trailing slash
    return url.replace(/\/$/, '');
  }

  private setupInterceptors(retries: number): void {
    // Request interceptor
    this.client.interceptors.request.use(
      (config) => config,
      (error) => Promise.reject(new ConnectionError(`Request setup failed: ${error.message}`))
    );

    // Response interceptor
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        if (error.response) {
          const status = error.response.status;
          const data = error.response.data as any;
          const message = data?.error || error.message;

          if (status === 404) {
            throw new NotFoundError(message);
          } else if (status >= 400) {
            throw new ServerError(message, status, JSON.stringify(data));
          }
        } else if (error.request) {
          throw new ConnectionError(`Network error: ${error.message}`);
        } else {
          throw new KotaDBError(`Request error: ${error.message}`);
        }
        
        return Promise.reject(error);
      }
    );
  }

  /**
   * Test connection to the database.
   */
  async testConnection(): Promise<HealthStatus> {
    try {
      const response = await this.client.get<HealthStatus>('/health');
      return response.data;
    } catch (error) {
      throw new ConnectionError(`Failed to connect to KotaDB at ${this.baseUrl}: ${error}`);
    }
  }

  /**
   * Search documents using text query.
   */
  async query(query: string, options: SearchOptions = {}): Promise<QueryResult> {
    const params: Record<string, any> = { q: query };
    if (options.limit) params.limit = options.limit;
    if (options.offset) params.offset = options.offset;

    const response = await this.client.get<{documents: Document[], total_count: number}>('/documents/search', { params });
    
    // Transform server response to expected format
    return {
      results: response.data.documents.map(doc => ({
        document: this.convertContentToString(doc),
        score: 1.0, // Server doesn't provide scores yet
        content_preview: this.getContentPreview(doc)
      })),
      total_count: response.data.total_count,
      query_time_ms: 0 // Server doesn't provide query time yet
    };
  }

  private getContentPreview(doc: Document): string {
    const content = Array.isArray(doc.content) 
      ? new TextDecoder().decode(new Uint8Array(doc.content))
      : doc.content;
    return content.substring(0, 200) + (content.length > 200 ? '...' : '');
  }

  /**
   * Perform semantic search using embeddings.
   */
  async semanticSearch(query: string, options: SemanticSearchOptions = {}): Promise<QueryResult> {
    const data: Record<string, any> = { query };
    if (options.limit) data.limit = options.limit;
    if (options.offset) data.offset = options.offset;
    if (options.model) data.model = options.model;

    const response = await this.client.post<QueryResult>('/search/semantic', data);
    return response.data;
  }

  /**
   * Perform hybrid search combining text and semantic search.
   */
  async hybridSearch(query: string, options: HybridSearchOptions = {}): Promise<QueryResult> {
    const data: Record<string, any> = { 
      query,
      semantic_weight: options.semantic_weight || 0.7
    };
    if (options.limit) data.limit = options.limit;
    if (options.offset) data.offset = options.offset;

    const response = await this.client.post<QueryResult>('/search/hybrid', data);
    return response.data;
  }

  private convertContentToString(doc: Document): Document {
    // Convert byte array content back to string for better UX
    if (Array.isArray(doc.content)) {
      return {
        ...doc,
        content: new TextDecoder().decode(new Uint8Array(doc.content))
      };
    }
    return doc;
  }

  /**
   * Get a document by ID.
   */
  async get(docId: string): Promise<Document> {
    const response = await this.client.get<Document>(`/documents/${docId}`);
    return this.convertContentToString(response.data);
  }

  /**
   * Insert a new document.
   */
  async insert(document: DocumentInput): Promise<string> {
    // Validate required fields
    const required = ['path', 'title', 'content'];
    for (const field of required) {
      if (!(field in document)) {
        throw new ValidationError(`Required field '${field}' missing`);
      }
    }

    // Convert content to byte array if it's a string
    const processedDocument = { ...document };
    if (typeof processedDocument.content === 'string') {
      processedDocument.content = Array.from(new TextEncoder().encode(processedDocument.content));
    }

    const response = await this.client.post<{ id: string }>('/documents', processedDocument);
    return response.data.id;
  }

  /**
   * Update an existing document.
   */
  async update(docId: string, updates: DocumentUpdate): Promise<Document> {
    // Convert content to byte array if it's a string
    const processedUpdates = { ...updates };
    if ('content' in processedUpdates && typeof processedUpdates.content === 'string') {
      processedUpdates.content = Array.from(new TextEncoder().encode(processedUpdates.content));
    }

    const response = await this.client.put<Document>(`/documents/${docId}`, processedUpdates);
    return this.convertContentToString(response.data);
  }

  /**
   * Delete a document.
   */
  async delete(docId: string): Promise<boolean> {
    await this.client.delete(`/documents/${docId}`);
    return true;
  }

  /**
   * List all documents.
   */
  async listAll(options: SearchOptions = {}): Promise<Document[]> {
    const params: Record<string, any> = {};
    if (options.limit) params.limit = options.limit;
    if (options.offset) params.offset = options.offset;

    const response = await this.client.get<{ documents: Document[] }>('/documents', { params });
    return response.data.documents.map(doc => this.convertContentToString(doc));
  }

  /**
   * Check database health status.
   */
  async health(): Promise<HealthStatus> {
    const response = await this.client.get<HealthStatus>('/health');
    return response.data;
  }

  /**
   * Get database statistics.
   */
  async stats(): Promise<DatabaseStats> {
    const response = await this.client.get<DatabaseStats>('/stats');
    return response.data;
  }
}

/**
 * Convenience function for creating a KotaDB client connection.
 */
export function connect(config: ConnectionConfig = {}): KotaDB {
  return new KotaDB(config);
}

export default KotaDB;
