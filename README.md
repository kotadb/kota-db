# KotaDB

**A custom database for distributed human-AI cognition, built entirely by LLM agents.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-271%20passing-brightgreen?style=for-the-badge)](https://github.com/jayminwest/kota-db/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

```
KotaDB combines document storage, graph relationships, and semantic search
into a unified system designed for the way humans and AI think together.
```

---

## Performance

Real-world benchmarks on Apple Silicon:

| Operation | Latency | Throughput |
|-----------|---------|------------|
| **B+ Tree Search** | **489 µs** | 2,000 queries/sec |
| **Trigram Search** | **<10 ms** | 100+ queries/sec |
| **Document Insert** | **277 µs** | 3,600 ops/sec |
| **Bulk Operations** | **20 ms** | 50,000 ops/sec |

*10,000 document dataset, Apple Silicon M-series*

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo build

# Start HTTP server
cargo run --bin kotadb -- serve

# CLI examples
cargo run --bin kotadb -- insert /test/doc "My Document" "Document content"
cargo run --bin kotadb -- search "rust"     # Full-text search
cargo run --bin kotadb -- search "*"        # Wildcard search
cargo run --bin kotadb -- stats            # Database statistics
```

<details>
<summary><strong>Development Commands</strong></summary>

```bash
just dev              # Start with auto-reload
just test             # Run all tests
just check            # Format, lint, test
just bench            # Performance benchmarks
```

</details>

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Query Interface                           │
│              Natural Language + Structured                   │
├─────────────────────────────────────────────────────────────┤
│                    Query Router                              │
│         Automatic index selection based on query             │
├──────────────┬───────────────┬───────────────┬──────────────┤
│   Primary    │   Full-Text   │     Graph     │   Semantic   │
│   B+ Tree    │    Trigram    │  (Planned)    │     HNSW     │
├──────────────┴───────────────┴───────────────┴──────────────┤
│                    Storage Engine                            │
│        Pages + WAL + Compression + Memory Map                │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Features

### Storage
- **Native Format**: Markdown files with YAML frontmatter
- **Git Compatible**: Human-readable, diff-friendly
- **Crash-Safe**: WAL ensures data durability
- **Zero Database Dependencies**: No external database required

### Indexing
- **B+ Tree**: O(log n) path-based lookups
- **Trigram**: Fuzzy-tolerant full-text search
- **Graph**: Relationship traversal (MCP tools only, not fully implemented)
- **Vector**: Semantic similarity with HNSW

### Safety
- **Systematic Testing**: 6-stage risk reduction methodology
- **Type Safety**: Validated types at compile time
- **Observability**: Distributed tracing on every operation
- **Resilience**: Automatic retries with exponential backoff

---

## Code Example

```rust
use kotadb::{create_file_storage, DocumentBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    // Production-ready storage with all safety features
    let mut storage = create_file_storage("~/.kota/db", Some(1000)).await?;
    
    // Type-safe document construction
    let doc = DocumentBuilder::new()
        .path("/knowledge/rust-patterns.md")?
        .title("Advanced Rust Design Patterns")?
        .content(b"# Advanced Rust Patterns\n\n...")?
        .build()?;
    
    // Automatically traced, validated, cached, with retries
    storage.insert(doc).await?;
    
    Ok(())
}
```

---

## Query Language

Natural, intuitive queries designed for human-AI interaction:

```javascript
// Natural language
"meetings about rust programming last week"

// Structured precision
{
  type: "semantic",
  query: "distributed systems",
  filter: { tags: { $contains: "architecture" } },
  limit: 10
}

// Graph traversal
GRAPH {
  start: "projects/kota-ai/README.md",
  follow: ["related", "references"],
  depth: 2
}
```

---

## Project Status

### Complete
- Storage engine with WAL and compression
- B+ tree primary index with persistence
- Trigram full-text search with ranking
- Intelligent query routing
- CLI interface
- Performance benchmarks

### In Progress
- [x] Model Context Protocol (MCP) server
- [x] Python/TypeScript client libraries
- [ ] Semantic vector search
- [ ] Graph relationship queries

---

## Documentation

[Architecture](docs/ARCHITECTURE.md) • [API Reference](docs/API.md) • [Development Guide](DEV_GUIDE.md) • [Agent Guide](AGENT.md)

---

## Installation

### As a CLI Tool
```bash
cargo install --path .
kotadb serve                    # Start HTTP server
kotadb insert /path "Title" "Content"  # Insert document
kotadb search "query"           # Search documents
```

### As a Library
```toml
[dependencies]
kotadb = { git = "https://github.com/jayminwest/kota-db" }
```

### Docker
```bash
docker build -t kotadb .
docker run -p 8080:8080 kotadb serve
```

---

## Benchmarks Detail

<details>
<summary><strong>Apple M2 Ultra (192GB RAM)</strong></summary>

| Operation | Size | Latency | Throughput |
|-----------|------|---------|------------|
| BTree Insert | 100 | 15.8 µs | 63,300 ops/sec |
| BTree Insert | 1,000 | 325 µs | 3,080 ops/sec |
| BTree Insert | 10,000 | 4.77 ms | 210 ops/sec |
| BTree Search | 100 | 2.08 µs | 482,000 queries/sec |
| BTree Search | 1,000 | 33.2 µs | 30,100 queries/sec |
| BTree Search | 10,000 | 546 µs | 1,830 queries/sec |
| Bulk Operations | 1,000 | 25.4 ms | 39,400 ops/sec |
| Bulk Operations | 5,000 | 23.7 ms | 211,000 ops/sec |

</details>

---

## Contributing

This project is developed entirely by LLM agents. Human contributions follow the same process:

1. Open an issue describing the change
2. Agents will review and implement
3. Changes are validated through comprehensive testing
4. Documentation is automatically updated

See [AGENT.md](AGENT.md) for the agent collaboration protocol.

---

## License

MIT - See [LICENSE](LICENSE) for details.

---

<sub>Built for [KOTA](https://github.com/jayminwest/kota) • Inspired by LevelDB, Tantivy, and FAISS</sub>

<sub>**The best database is the one designed specifically for your problem.**</sub>
