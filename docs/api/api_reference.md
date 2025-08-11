# KotaDB API Reference

## Overview

KotaDB provides multiple API layers for different use cases:

1. **Native Rust API** - Direct library usage
2. **MCP Server API** - JSON-RPC for LLM integration
3. **REST API** - HTTP endpoints (planned)
4. **CLI Interface** - Command-line tools

## Native Rust API

### Storage Operations

#### Document Management

```rust
use kotadb::{DocumentBuilder, create_file_storage};

// Create storage with Stage 6 safety wrappers
let mut storage = create_file_storage("./data", Some(1000)).await?;

// Create a document
let doc = DocumentBuilder::new()
    .path("/knowledge/rust-patterns.md")?
    .title("Advanced Rust Design Patterns")?
    .content(b"# Advanced Rust Patterns\n\nThis covers...")?
    .build()?;

// Store document (automatically traced, validated, cached, with retries)
storage.insert(doc.clone()).await?;

// Retrieve document (cache-optimized)
let retrieved = storage.get(&doc.id).await?;
```

#### Query Operations

```rust
use kotadb::{QueryBuilder, create_primary_index};

// Create index
let mut index = create_primary_index("./index", 1000)?;

// Build query
let query = QueryBuilder::new()
    .with_text("rust patterns")?
    .with_tag("programming")?
    .with_date_range(start_time, end_time)?
    .with_limit(25)?
    .build()?;

// Execute search
let results = index.search(&query).await?;
```

### Performance Optimization

```rust
use kotadb::{create_optimized_index_with_defaults, OptimizationConfig};

// Create optimized index with automatic bulk operations
let primary_index = create_primary_index("/data/index", 1000)?;
let mut optimized_index = create_optimized_index_with_defaults(primary_index);

// Bulk operations automatically applied for 10x speedup
let pairs = vec![(id1, path1), (id2, path2), /* ... */];
let result = optimized_index.bulk_insert(pairs)?;
assert!(result.meets_performance_requirements(10.0)); // 10x speedup
```

## MCP Server API

### Connection

```bash
# Start MCP server
kotadb mcp-server --config kotadb.toml --port 8080
```

### Tools

#### Semantic Search

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
        "name": "kotadb://semantic_search",
        "arguments": {
            "query": "machine learning algorithms for natural language processing",
            "limit": 10,
            "include_metadata": true,
            "min_relevance": 0.7
        }
    }
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "content": [
            {
                "type": "text",
                "text": "Found 8 documents related to machine learning algorithms for NLP"
            }
        ],
        "documents": [
            {
                "id": "doc_123",
                "path": "/ml/transformers.md",
                "title": "Transformer Architecture for NLP",
                "relevance_score": 0.94,
                "summary": "Comprehensive overview of transformer models...",
                "metadata": {
                    "created": "2024-01-15T10:30:00Z",
                    "updated": "2024-01-20T14:22:00Z",
                    "word_count": 2450,
                    "tags": ["ml", "nlp", "transformers"]
                }
            }
        ]
    }
}
```

#### Document Operations

```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
        "name": "kotadb://insert_document",
        "arguments": {
            "path": "/knowledge/new-insights.md",
            "title": "New AI Research Insights",
            "content": "# AI Research\n\nRecent developments...",
            "tags": ["ai", "research", "insights"],
            "metadata": {
                "source": "research_paper",
                "author": "Dr. Smith"
            }
        }
    }
}
```

#### Graph Traversal

```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "kotadb://graph_search",
        "arguments": {
            "start_document": "/projects/ai-research.md",
            "relationship_types": ["references", "related_to", "cites"],
            "max_depth": 3,
            "min_relevance": 0.7,
            "include_path": true
        }
    }
}
```

### Resources

#### Document Collections

```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "resources/read",
    "params": {
        "uri": "kotadb://documents/?filter=recent&limit=20"
    }
}
```

#### Analytics Data

```json
{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "resources/read",
    "params": {
        "uri": "kotadb://analytics/patterns?timeframe=30d"
    }
}
```

### Error Handling

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "error": {
        "code": -32602,
        "message": "Invalid params",
        "data": {
            "type": "ValidationError",
            "details": "Query text cannot be empty",
            "field": "query"
        }
    }
}
```

## CLI Interface

### Basic Operations

```bash
# Initialize database
kotadb init --data-dir ./data

# Index documents
kotadb index ./documents --recursive

# Search
kotadb search "rust programming patterns"

# Semantic search
kotadb search --semantic "concepts related to database optimization"

# Graph traversal
kotadb graph --start "/docs/architecture.md" --depth 2
```

### Advanced Operations

```bash
# Performance analysis
kotadb analyze --performance --timeframe 30d

# Index maintenance
kotadb reindex --type trigram --optimize

# Export data
kotadb export --format json --output backup.json

# Health check
kotadb health --verbose
```

## Configuration

### Database Configuration

```toml
[database]
data_directory = "./data"
cache_size_mb = 512
enable_wal = true
sync_mode = "normal"

[indices]
primary_cache_size = 100
trigram_cache_size = 200
vector_cache_size = 300

[performance]
bulk_operation_threshold = 100
concurrent_readers = 8
enable_optimization = true

[mcp_server]
enabled = true
host = "localhost"
port = 8080
max_connections = 100
timeout_seconds = 30
enable_cors = false
allowed_origins = []

[logging]
level = "info"
format = "json"
log_to_file = true
log_directory = "./logs"

[security]
enable_auth = false
api_key_required = false
rate_limit_per_minute = 1000
```

## Performance Characteristics

| Operation | Latency Target | Throughput | Notes |
|-----------|---------------|------------|--------|
| Document Insert | <1ms | 1,250/sec | Single document |
| Bulk Insert | <200ms | 10,000/sec | Batch of 1,000 |
| Text Search | <3ms | 333/sec | Trigram index |
| Semantic Search | <10ms | 100/sec | Vector similarity |
| Graph Traversal | <8ms | 125/sec | Depth 2 |

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 1001 | DocumentNotFound | Document ID not found |
| 1002 | InvalidPath | Invalid document path |
| 1003 | ValidationError | Data validation failed |
| 1004 | IndexCorruption | Index integrity check failed |
| 1005 | StorageError | Storage operation failed |
| 1006 | PerformanceLimit | Query exceeded performance limits |
| 1007 | AuthenticationError | Invalid credentials |
| 1008 | RateLimitExceeded | Too many requests |

## Examples Repository

Complete examples available in the [`examples/` directory](../examples/):

- `basic_usage.rs` - Getting started with KotaDB
- `advanced_queries.rs` - Complex search operations
- `mcp_client.rs` - MCP server integration
- `performance_optimization.rs` - Bulk operations and caching
- `custom_indices.rs` - Building custom index types

## SDK Integrations

### Python Client (Planned)

```python
import kotadb

# Connect to MCP server
client = kotadb.MCPClient("http://localhost:8080")

# Semantic search
results = await client.semantic_search(
    "machine learning algorithms",
    limit=10,
    min_relevance=0.8
)

for doc in results:
    print(f"{doc.title}: {doc.relevance_score}")
```

### TypeScript Client (Planned)

```typescript
import { KotaDBClient } from '@kotadb/client';

const client = new KotaDBClient('http://localhost:8080');

const results = await client.semanticSearch({
  query: 'database optimization techniques',
  limit: 5,
  includeMetadata: true
});
```

## Support

- **Documentation**: [docs/](../docs/)
- **Issues**: [GitHub Issues](https://github.com/jayminwest/kota-db/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jayminwest/kota-db/discussions)
- **Examples**: [examples/](../examples/)
