# KotaDB

**A custom database for distributed human-AI cognition, built entirely by LLM agents.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-271%20passing-brightgreen?style=for-the-badge)](https://github.com/jayminwest/kota-db/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

## 🚀 Quick Start - Choose Your Language

<div align="center">

### Python
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)
[![Python Downloads](https://img.shields.io/pypi/dm/kotadb-client)](https://pypi.org/project/kotadb-client/)

```bash
pip install kotadb-client
```

### TypeScript/JavaScript  
[![npm version](https://badge.fury.io/js/kotadb-client.svg)](https://www.npmjs.com/package/kotadb-client)
[![npm downloads](https://img.shields.io/npm/dm/kotadb-client)](https://www.npmjs.com/package/kotadb-client)

```bash
npm install kotadb-client
```

### Rust
[![Crates.io](https://img.shields.io/crates/v/kotadb.svg)](https://crates.io/crates/kotadb)
[![Crates.io Downloads](https://img.shields.io/crates/d/kotadb)](https://crates.io/crates/kotadb)

```bash
cargo add kotadb
```

### Go (Coming Soon)
🚧 **Work in Progress** - Go client is currently under development. See [#114](https://github.com/jayminwest/kota-db/issues/114) for progress.

```bash
# Will be available soon at:
# go get github.com/jayminwest/kota-db/clients/go
```

</div>

---

## ⚡ 60-Second Quick Start

**Get from zero to first query in under 60 seconds:**

### Option 1: Docker (Easiest)
```bash
# One command to start everything
docker-compose -f docker-compose.quickstart.yml up -d

# Run Python demo (shows all features)
docker-compose -f docker-compose.quickstart.yml --profile demo up python-demo
```

### Option 2: Shell Script (Local Install)
```bash
# One-liner installation and demo
curl -sSL https://raw.githubusercontent.com/jayminwest/kota-db/main/quickstart/install.sh | bash
```

### Option 3: Manual Setup
```bash
# Start server
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve

# Install client and try it
pip install kotadb-client
python -c "
from kotadb import KotaDB, DocumentBuilder
db = KotaDB('http://localhost:8080')
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path('/hello.md')
    .title('Hello KotaDB!')
    .content('My first document')
)
print(f'Created document: {doc_id}')
results = db.query('hello')
print(f'Found {len(results.get(\"documents\", []))} documents')
"
```

**🎉 That's it! You're now running KotaDB with type-safe client libraries.**

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

## 🎯 Complete Examples

**Production-ready applications demonstrating real-world usage:**

### 🌐 [Flask Web App](examples/flask-web-app/)
Complete web application with REST API and UI
```bash
cd examples/flask-web-app && pip install -r requirements.txt && python app.py
# Visit http://localhost:5000
```

### 📝 [Note-Taking App](examples/note-taking-app/) 
Advanced document management with folders and tags
```bash
cd examples/note-taking-app && pip install -r requirements.txt && python note_app.py
# Visit http://localhost:5001  
```

### 🧠 [RAG Pipeline](examples/rag-pipeline/)
AI-powered question answering with document retrieval
```bash
cd examples/rag-pipeline && pip install -r requirements.txt && python rag_demo.py
# Requires OPENAI_API_KEY for best results
```

### ⚡ Quick Examples
```bash
# Python type-safe usage
from kotadb import KotaDB, DocumentBuilder, ValidatedPath

db = KotaDB("http://localhost:8080")
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path(ValidatedPath("/notes/meeting.md"))
    .title("Team Meeting")
    .content("Discussion about project timeline...")
    .add_tag("meeting")
    .add_tag("important")
)

# Advanced search with filters
from kotadb import QueryBuilder
results = db.query_with_builder(
    QueryBuilder()
    .text("project timeline") 
    .tag_filter("meeting")
    .limit(10)
)
```

### 🦀 Rust (Full Feature Access)
```bash
# Clone and build
git clone https://github.com/jayminwest/kota-db.git
cd kota-db && cargo build --release

# Start server
cargo run --bin kotadb -- serve

# CLI operations  
cargo run --bin kotadb -- insert /docs/rust.md "Rust Guide" "Ownership concepts..."
cargo run --bin kotadb -- search "ownership"  # Full-text search
cargo run --bin kotadb -- search "*"          # List all documents  
cargo run --bin kotadb -- stats              # Database statistics
```

<details>
<summary><strong>Development Commands</strong></summary>

```bash
just dev              # Auto-reload development server
just test             # Run comprehensive test suite
just check            # Format, lint, and test everything
just bench            # Performance benchmarks
just release-preview  # Preview next release
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
- **Type Safety**: Validated types (Rust compile-time, Python/TypeScript runtime)
- **Observability**: Distributed tracing on every operation (Rust only)
- **Resilience**: Automatic retries with exponential backoff (all client libraries)

---

## Code Examples

### Rust (Full Feature Access)
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

### Python (Client Library)
```python
from kotadb import KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath

# Connect to KotaDB server
db = KotaDB("http://localhost:8080")

# Type-safe document construction (runtime validation)
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path(ValidatedPath("/knowledge/python-patterns.md"))
    .title("Python Design Patterns")
    .content("# Python Patterns\n\n...")
    .add_tag("python")
    .add_tag("patterns")
)

# Query with builder pattern
results = db.query_with_builder(
    QueryBuilder()
    .text("design patterns")
    .limit(10)
    .tag_filter("python")
)
```

### TypeScript (Client Library)
```typescript
import { KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath } from 'kotadb-client';

// Connect to KotaDB server
const db = new KotaDB({ url: 'http://localhost:8080' });

// Type-safe document construction (runtime validation)
const docId = await db.insertWithBuilder(
  new DocumentBuilder()
    .path("/knowledge/typescript-patterns.md")
    .title("TypeScript Design Patterns")
    .content("# TypeScript Patterns\n\n...")
    .addTag("typescript")
    .addTag("patterns")
);

// Query with builder pattern and full IntelliSense support
const results = await db.queryWithBuilder(
  new QueryBuilder()
    .text("design patterns")
    .limit(10)
    .tagFilter("typescript")
);
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

[Architecture](https://jayminwest.github.io/kota-db/stable/architecture/technical_architecture/) • [API Reference](https://jayminwest.github.io/kota-db/stable/api/api_reference/) • [Development Guide](https://jayminwest.github.io/kota-db/stable/development-guides/dev_guide/) • [Agent Guide](AGENT.md)

---

## Installation

### Client Libraries

#### Python
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)
```bash
pip install kotadb-client
```

#### TypeScript/JavaScript
```bash
npm install kotadb-client
# or
yarn add kotadb-client
```

#### Go (Coming Soon)
```bash
# Go client is currently under development
# See https://github.com/jayminwest/kota-db/issues/114
# Will be available at: github.com/jayminwest/kota-db/clients/go
```

### Server Installation

#### As a CLI Tool
```bash
cargo install kotadb
# or from source:
cargo install --path .

kotadb serve                    # Start HTTP server
kotadb insert /path "Title" "Content"  # Insert document
kotadb search "query"           # Search documents
```

#### As a Rust Library
[![Crates.io](https://img.shields.io/crates/v/kotadb.svg)](https://crates.io/crates/kotadb)
```toml
[dependencies]
kotadb = "0.3.0"
# or from git:
kotadb = { git = "https://github.com/jayminwest/kota-db" }
```

#### Docker
```bash
# Using pre-built image (recommended)
docker pull ghcr.io/jayminwest/kota-db:latest
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve

# Or build from source
docker build -t kotadb .
docker run -p 8080:8080 kotadb serve
```

---

## Language Support Matrix

| Feature | Rust | Python | TypeScript | Go |
|---------|------|--------|------------|-----|
| **Basic Operations** | | | | |
| Document CRUD | ✅ | ✅ | ✅ | ✅ |
| Text Search | ✅ | ✅ | ✅ | ✅ |
| Semantic Search | ✅ | ✅ | ✅ | 🚧 |
| Hybrid Search | ✅ | ✅ | ✅ | 🚧 |
| **Type Safety** | | | | |
| Validated Types | ✅ | ✅ | ✅ | ❌ |
| Builder Patterns | ✅ | ✅ | ✅ | ❌ |
| **Advanced Features** | | | | |
| Query Routing | ✅ | ❌* | ❌* | ❌* |
| Graph Queries | 🚧 | ❌ | ❌ | ❌ |
| Direct Storage Access | ✅ | ❌ | ❌ | ❌ |
| Observability/Tracing | ✅ | ❌ | ❌ | ❌ |
| **Development** | | | | |
| Connection Pooling | ✅ | ✅ | ✅ | ✅ |
| Retry Logic | ✅ | ✅ | ✅ | ✅ |
| Error Handling | ✅ | ✅ | ✅ | ✅ |

**Legend**: ✅ Complete • 🚧 In Progress • ❌ Not Available

*Query routing happens automatically on the server for client libraries

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
