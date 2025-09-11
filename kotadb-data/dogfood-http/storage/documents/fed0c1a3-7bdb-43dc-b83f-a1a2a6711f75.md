---
tags:
- file
- kota-db
- ext_js
---
"use strict";
/**
 * KotaDB TypeScript/JavaScript Client
 *
 * Provides a simple, PostgreSQL-like interface for document operations.
 */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.KotaDB = void 0;
exports.connect = connect;
const axios_1 = __importDefault(require("axios"));
const types_1 = require("./types");
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
class KotaDB {
    constructor(config = {}) {
        this.baseUrl = this.parseUrl(config.url);
        this.client = axios_1.default.create({
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
    parseUrl(url) {
        if (!url) {
            url = process.env.KOTADB_URL;
            if (!url) {
                throw new types_1.ConnectionError('No URL provided and KOTADB_URL environment variable not set');
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
    setupInterceptors(retries) {
        // Request interceptor
        this.client.interceptors.request.use((config) => config, (error) => Promise.reject(new types_1.ConnectionError(`Request setup failed: ${error.message}`)));
        // Response interceptor
        this.client.interceptors.response.use((response) => response, (error) => {
            if (error.response) {
                const status = error.response.status;
                const data = error.response.data;
                const message = data?.error || error.message;
                if (status === 404) {
                    throw new types_1.NotFoundError(message);
                }
                else if (status >= 400) {
                    throw new types_1.ServerError(message, status, JSON.stringify(data));
                }
            }
            else if (error.request) {
                throw new types_1.ConnectionError(`Network error: ${error.message}`);
            }
            else {
                throw new types_1.KotaDBError(`Request error: ${error.message}`);
            }
            return Promise.reject(error);
        });
    }
    /**
     * Test connection to the database.
     */
    async testConnection() {
        try {
            const response = await this.client.get('/health');
            return response.data;
        }
        catch (error) {
            throw new types_1.ConnectionError(`Failed to connect to KotaDB at ${this.baseUrl}: ${error}`);
        }
    }
    /**
     * Search documents using text query.
     */
    async query(query, options = {}) {
        const params = { q: query };
        if (options.limit)
            params.limit = options.limit;
        if (options.offset)
            params.offset = options.offset;
        const response = await this.client.get('/documents/search', { params });
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
    /**
     * Search documents using QueryBuilder for type safety.
     */
    async queryWithBuilder(builder) {
        const params = builder.build();
        const query = params.q;
        delete params.q;
        return this.query(query, params);
    }
    getContentPreview(doc) {
        const content = Array.isArray(doc.content)
            ? new TextDecoder().decode(new Uint8Array(doc.content))
            : doc.content;
        return content.substring(0, 200) + (content.length > 200 ? '...' : '');
    }
    /**
     * Perform semantic search using embeddings.
     */
    async semanticSearch(query, options = {}) {
        const data = { query };
        if (options.limit)
            data.limit = options.limit;
        if (options.offset)
            data.offset = options.offset;
        if (options.model)
            data.model = options.model;
        const response = await this.client.post('/search/semantic', data);
        return response.data;
    }
    /**
     * Perform semantic search using QueryBuilder for type safety.
     */
    async semanticSearchWithBuilder(builder) {
        const data = builder.buildForSemantic();
        const query = data.query;
        delete data.query;
        return this.semanticSearch(query, data);
    }
    /**
     * Perform hybrid search combining text and semantic search.
     */
    async hybridSearch(query, options = {}) {
        const data = {
            query,
            semantic_weight: options.semantic_weight || 0.7
        };
        if (options.limit)
            data.limit = options.limit;
        if (options.offset)
            data.offset = options.offset;
        const response = await this.client.post('/search/hybrid', data);
        return response.data;
    }
    /**
     * Perform hybrid search using QueryBuilder for type safety.
     */
    async hybridSearchWithBuilder(builder) {
        const data = builder.buildForHybrid();
        const query = data.query;
        delete data.query;
        return this.hybridSearch(query, data);
    }
    convertContentToString(doc) {
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
    async get(docId) {
        const response = await this.client.get(`/documents/${docId}`);
        return this.convertContentToString(response.data);
    }
    /**
     * Insert a new document.
     */
    async insert(document) {
        // Validate required fields
        const required = ['path', 'title', 'content'];
        for (const field of required) {
            if (!(field in document)) {
                throw new types_1.ValidationError(`Required field '${field}' missing`);
            }
        }
        // Convert content to byte array if it's a string
        const processedDocument = { ...document };
        if (typeof processedDocument.content === 'string') {
            processedDocument.content = Array.from(new TextEncoder().encode(processedDocument.content));
        }
        const response = await this.client.post('/documents', processedDocument);
        return response.data.id;
    }
    /**
     * Insert a new document using DocumentBuilder for type safety.
     */
    async insertWithBuilder(builder) {
        const document = builder.build();
        return this.insert(document);
    }
    /**
     * Update an existing document.
     */
    async update(docId, updates) {
        // Convert content to byte array if it's a string
        const processedUpdates = { ...updates };
        if ('content' in processedUpdates && typeof processedUpdates.content === 'string') {
            processedUpdates.content = Array.from(new TextEncoder().encode(processedUpdates.content));
        }
        const response = await this.client.put(`/documents/${docId}`, processedUpdates);
        return this.convertContentToString(response.data);
    }
    /**
     * Delete a document.
     */
    async delete(docId) {
        await this.client.delete(`/documents/${docId}`);
        return true;
    }
    /**
     * List all documents.
     */
    async listAll(options = {}) {
        const params = {};
        if (options.limit)
            params.limit = options.limit;
        if (options.offset)
            params.offset = options.offset;
        const response = await this.client.get('/documents', { params });
        return response.data.documents.map(doc => this.convertContentToString(doc));
    }
    /**
     * Check database health status.
     */
    async health() {
        const response = await this.client.get('/health');
        return response.data;
    }
    /**
     * Get database statistics.
     */
    async stats() {
        const response = await this.client.get('/stats');
        return response.data;
    }
}
exports.KotaDB = KotaDB;
/**
 * Convenience function for creating a KotaDB client connection.
 */
function connect(config = {}) {
    return new KotaDB(config);
}
exports.default = KotaDB;
//# sourceMappingURL=client.js.map