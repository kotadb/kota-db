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
        const response = await this.client.get('/api/documents/search', { params });
        return response.data;
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
        const response = await this.client.post('/api/search/semantic', data);
        return response.data;
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
        const response = await this.client.post('/api/search/hybrid', data);
        return response.data;
    }
    /**
     * Get a document by ID.
     */
    async get(docId) {
        const response = await this.client.get(`/api/documents/${docId}`);
        return response.data;
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
        const response = await this.client.post('/api/documents', document);
        return response.data.id;
    }
    /**
     * Update an existing document.
     */
    async update(docId, updates) {
        const response = await this.client.put(`/api/documents/${docId}`, updates);
        return response.data;
    }
    /**
     * Delete a document.
     */
    async delete(docId) {
        await this.client.delete(`/api/documents/${docId}`);
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
        const response = await this.client.get('/api/documents', { params });
        return response.data.documents;
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
        const response = await this.client.get('/api/stats');
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