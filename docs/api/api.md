# KotaDB API Documentation

## Overview

KotaDB is a custom database for distributed human-AI cognition built in Rust. It provides high-performance document storage, indexing, and search capabilities. Semantic search support has been retired until the cloud-first relaunch.

## Core Features

- **Document Storage**: Efficient file-based storage with Write-Ahead Logging (WAL)
- **Full-Text Search**: Trigram-based indexing for fast text search
- **Semantic Search** *(retired)*: Vector/embedding search will return with the cloud-first relaunch
- **Graph Relationships**: Document relationship mapping and traversal
- **Component Library**: Validated types, builders, and safety wrappers

## API Endpoints

⚠️ **Migration Notice**: Document CRUD endpoints have been removed. Use the **codebase intelligence API** via MCP server or client libraries instead.

### Available HTTP Endpoints

### Analytics

#### Health Check
```http
GET /health
```

Get system health status and metrics.

#### System Metrics
```http
GET /metrics
```

Get detailed system performance metrics.

## Data Types

### Document
Core document structure with validation and metadata support.

**Fields:**
- `id`: UUID identifier
- `path`: Unique path within the database
- `title`: Optional human-readable title
- `content`: Document content (bytes)
- `tags`: Array of categorization tags
- `metadata`: Key-value metadata map
- `created_at`: Creation timestamp
- `updated_at`: Last modification timestamp

### Query
Search query structure with filtering options.

**Fields:**
- `text`: Text search query
- `tags`: Tag filters
- `path_pattern`: Path pattern filter
- `limit`: Maximum results

### SearchResult
Search result with scoring and metadata.

**Fields:**
- `document`: Matched document
- `score`: Relevance score (0.0-1.0)
- `snippet`: Content preview

## Error Handling

All API endpoints return standardized error responses:

**Error Response:**
```json
{
  "error": {
    "code": "DOCUMENT_NOT_FOUND",
    "message": "Document with ID '123...' not found",
    "details": {}
  }
}
```

**Common Error Codes:**
- `DOCUMENT_NOT_FOUND`: Requested document does not exist
- `VALIDATION_ERROR`: Input validation failed
- `STORAGE_ERROR`: Storage operation failed
- `INDEX_ERROR`: Indexing operation failed
- `SEARCH_ERROR`: Search operation failed

## Performance

KotaDB is designed for high performance with specific targets:

- **Document Retrieval**: <1ms
- **Text Search**: <10ms  
- **Semantic Search**: Retired
- **Graph Traversals**: <50ms

## Configuration

KotaDB uses TOML configuration files:

```toml
[database]
data_dir = "./kotadb-data"
max_cache_size = 1000
enable_wal = true

[server]
host = "0.0.0.0"
port = 8080



[search]
max_results = 1000

[performance]
worker_threads = 4
max_blocking_threads = 16
```

## Security

- **Input Validation**: All inputs are validated using the validation layer
- **Type Safety**: Rust's type system prevents common vulnerabilities
- **Memory Safety**: No buffer overflows or memory leaks
- **Rate Limiting**: Configurable request rate limiting

## Integration

### Model Context Protocol (MCP)

KotaDB provides a built-in MCP server for seamless LLM integration:

```bash
kotadb-mcp --config kotadb-mcp.toml --port 3000
```

### Docker Deployment

Production-ready Docker containers are available:

```bash
docker run -p 8080:8080 -v ./data:/app/data kotadb:latest
```

## Examples

### Basic Usage

```rust
use kotadb::*;

// Create storage
let storage = create_file_storage("./data", Some(1000)).await?;

// Create document
let doc = DocumentBuilder::new()
    .path("/docs/example.md")?
    .title("Example")?
    .content(b"Hello, World!")?
    .build()?;

// Store document
storage.insert(doc).await?;

// Search documents
let results = storage.search("Hello").await?;
```

### MCP Integration

```typescript
// Connect to KotaDB MCP server
const client = new MCPClient("http://localhost:3000");

// Create document via MCP
const result = await client.call("kotadb://document_create", {
    path: "/docs/example.md",
    title: "Example Document",
    content: "Hello from MCP!"
});

// Search documents
const searchResults = await client.call("kotadb://text_search", {
    query: "Hello",
    limit: 10
});
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/jayminwest/kota-db/issues
- Documentation: https://github.com/jayminwest/kota-db/docs
- MCP Integration Guide: See MCP_INTEGRATION_PLAN.md

## License

MIT License - see LICENSE file for details.
