/**
 * KotaDB TypeScript/JavaScript Client
 *
 * Provides a simple, PostgreSQL-like interface for document operations.
 */
import { Document, QueryResult, SearchOptions, SemanticSearchOptions, HybridSearchOptions, ConnectionConfig, HealthStatus, DatabaseStats, DocumentInput, DocumentUpdate } from './types';
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
export declare class KotaDB {
    private client;
    private baseUrl;
    constructor(config?: ConnectionConfig);
    private parseUrl;
    private setupInterceptors;
    /**
     * Test connection to the database.
     */
    testConnection(): Promise<HealthStatus>;
    /**
     * Search documents using text query.
     */
    query(query: string, options?: SearchOptions): Promise<QueryResult>;
    /**
     * Perform semantic search using embeddings.
     */
    semanticSearch(query: string, options?: SemanticSearchOptions): Promise<QueryResult>;
    /**
     * Perform hybrid search combining text and semantic search.
     */
    hybridSearch(query: string, options?: HybridSearchOptions): Promise<QueryResult>;
    /**
     * Get a document by ID.
     */
    get(docId: string): Promise<Document>;
    /**
     * Insert a new document.
     */
    insert(document: DocumentInput): Promise<string>;
    /**
     * Update an existing document.
     */
    update(docId: string, updates: DocumentUpdate): Promise<Document>;
    /**
     * Delete a document.
     */
    delete(docId: string): Promise<boolean>;
    /**
     * List all documents.
     */
    listAll(options?: SearchOptions): Promise<Document[]>;
    /**
     * Check database health status.
     */
    health(): Promise<HealthStatus>;
    /**
     * Get database statistics.
     */
    stats(): Promise<DatabaseStats>;
}
/**
 * Convenience function for creating a KotaDB client connection.
 */
export declare function connect(config?: ConnectionConfig): KotaDB;
export default KotaDB;
//# sourceMappingURL=client.d.ts.map