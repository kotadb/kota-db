# KotaDB

**A codebase intelligence platform that understands your code's relationships, dependencies, and structure.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-271%20passing-brightgreen?style=for-the-badge)](https://github.com/jayminwest/kota-db/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

## 🚀 Quick Start - Choose Your Language

<div align="center">

### Python
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)
[![Python Downloads](https://img.shields.io/pypi/dm/kotadb-client)](https://pypi.org/project/kotadb-client/)

```bash
# Install with automatic server management
pip install kotadb-client

# Start server and connect (no compilation needed!)
python -c "
from kotadb import KotaDB, start_server
server = start_server(port=8080)  # Auto-downloads binary
db = KotaDB('http://localhost:8080')
print('KotaDB is running!')
"
```

### TypeScript/JavaScript  
[![npm version](https://badge.fury.io/js/kotadb-client.svg)](https://www.npmjs.com/package/kotadb-client)
[![npm downloads](https://img.shields.io/npm/dm/kotadb-client)](https://www.npmjs.com/package/kotadb-client)

```bash
# Install with automatic server management
npm install kotadb-client

# Start server and connect (no compilation needed!)
npx tsx -e "
import { KotaDB, startServer } from 'kotadb-client';
const server = await startServer({ port: 8080 });  // Auto-downloads binary
const db = new KotaDB({ url: 'http://localhost:8080' });
console.log('KotaDB is running!');
"
```

### Rust
[![Crates.io](https://img.shields.io/crates/v/kotadb.svg)](https://crates.io/crates/kotadb)
[![Crates.io Downloads](https://img.shields.io/crates/d/kotadb)](https://crates.io/crates/kotadb)

```bash
cargo add kotadb
```

### Pre-Built Binaries
[![GitHub Release](https://img.shields.io/github/v/release/jayminwest/kota-db)](https://github.com/jayminwest/kota-db/releases)

Download pre-compiled binaries for your platform - no build time required!

```bash
# macOS (Apple Silicon)
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-macos-arm64.tar.gz | tar xz

# macOS (Intel)
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-macos-x64.tar.gz | tar xz

# Linux x64
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-linux-x64.tar.gz | tar xz

# Windows x64
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-windows-x64.zip -o kotadb.zip
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
from kotadb import KotaDB
db = KotaDB('http://localhost:8080')

# Index a codebase
stats = db.index_codebase('/path/to/project')
print(f'Indexed {stats[\"symbols\"]} symbols')

# Search for code
results = db.search_code('function_name')
print(f'Found {len(results)} matches')
"
```

**🎉 That's it! You're now running KotaDB with type-safe client libraries.**

```
KotaDB transforms your codebase into a queryable knowledge graph, enabling
instant symbol lookup, dependency analysis, and impact assessment for safer refactoring.
```

---

## Performance

Real-world benchmarks on Apple Silicon:

| Operation | Latency | Throughput | Notes |
|-----------|---------|------------|-------|
| **B+ Tree Search** | **489 µs** | 2,000 queries/sec | Path lookups |
| **Trigram Search** | **<3 ms** | 333+ queries/sec | 210x faster! |
| **Symbol Extraction** | **~100 ms/file** | 10 files/sec | Tree-sitter parsing |
| **Symbol Index** | **277 µs** | 3,600 ops/sec | With relationships |
| **Bulk Operations** | **20 ms** | 50,000 ops/sec | Batched writes |

*Tested on KotaDB's own codebase (21,613+ symbols, 248+ source files)*

---

## 🎯 Complete Examples

**Production-ready applications demonstrating real-world usage:**

### 🌐 [Flask Web App](examples/flask-web-app/)
Complete web application with REST API and UI
```bash
cd examples/flask-web-app && pip install -r requirements.txt && python app.py
# Visit http://localhost:5000
```

### 🔍 [Code Analysis Tool](examples/code-analysis/) 
Analyze your codebase structure and dependencies
```bash
cd examples/code-analysis && pip install -r requirements.txt && python analyzer.py
# Analyzes the current directory by default
```

### 🤖 [AI Assistant Integration](examples/ai-assistant/)
Power your AI coding assistant with code understanding
```bash
cd examples/ai-assistant && pip install -r requirements.txt && python assistant.py
# Integrates with Claude, GPT, or local models
```

### ⚡ Quick Examples
```bash
# Python codebase intelligence
from kotadb import KotaDB

db = KotaDB("http://localhost:8080")

# Index your codebase
stats = db.index_codebase("./my-project")
print(f"Indexed {stats['symbols']} symbols")

# Search for symbols
symbols = db.search_symbols("FileStorage")
for sym in symbols:
    print(f"{sym['type']}: {sym['name']} at {sym['location']}")

# Find all references to a function
callers = db.find_callers("process_data")
for caller in callers:
    print(f"Called from {caller['file']}:{caller['line']}")

# Analyze impact of changes
impact = db.analyze_impact("DatabaseConnection")
print(f"Changing this would affect {len(impact['affected'])} files")
```

### 🦀 Rust (Full Feature Access)
```bash
# Clone and build
git clone https://github.com/jayminwest/kota-db.git
cd kota-db && cargo build --release

# Start server
cargo run --bin kotadb -- serve

# Codebase Intelligence Features
cargo run --bin kotadb -- index-codebase .         # Analyze entire repository
cargo run --bin kotadb -- stats --symbols          # View extracted symbols
cargo run --bin kotadb -- find-callers FileStorage # Who calls this?
cargo run --bin kotadb -- analyze-impact StorageError  # What breaks if changed?

# Search operations  
cargo run --bin kotadb -- search-code "ownership"  # Full-text code search (<3ms)
cargo run --bin kotadb -- search-symbols "*.rs"    # Find symbols by pattern
cargo run --bin kotadb -- stats                    # Database statistics
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
│              Codebase Intelligence Layer                     │
│    Symbol Extraction + Dependency Analysis + Impact          │
├─────────────────────────────────────────────────────────────┤
│                    Query Interface                           │
│      Text Search + Vector Search + Relationship Queries      │
├─────────────────────────────────────────────────────────────┤
│                    Query Router                              │
│         Automatic index selection based on query             │
├──────────────┬───────────────┬───────────────┬──────────────┤
│   Primary    │   Full-Text   │  Relationship │   Semantic   │
│   B+ Tree    │   Trigram     │     Graph     │     HNSW     │
│     ✅       │   ✅ (<3ms)   │      ✅       │      ✅      │
├──────────────┴───────────────┴───────────────┴──────────────┤
│              Dual Storage Architecture                       │
│     Document Storage (MD/JSON) + Graph Storage (Native)      │
├─────────────────────────────────────────────────────────────┤
│                    Storage Engine                            │
│        Pages + WAL + Compression + Memory Map                │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Features

### Codebase Intelligence (New!)
- **Symbol Extraction**: Automatically extracts functions, classes, traits, and their relationships
- **Dependency Analysis**: Tracks what calls what, enabling impact analysis
- **Dual Storage**: Separates documents and graph data for optimal performance
- **Git Integration**: Ingest entire repositories with full history preservation

### Storage & Performance
- **210x Faster Search**: Trigram search optimized to <3ms (from 591ms)
- **Native Format**: Markdown files with YAML frontmatter
- **Crash-Safe**: WAL ensures data durability
- **Zero Database Dependencies**: No external database required

### Indexing Capabilities
- **B+ Tree**: ✅ O(log n) path-based lookups with wildcard support
- **Trigram**: ✅ Fuzzy-tolerant full-text search with <3ms latency
- **Vector**: ✅ Semantic similarity search using HNSW algorithm
- **Graph**: ✅ Relationship tracking for code dependencies

### Safety
- **Systematic Testing**: 6-stage risk reduction methodology
- **Type Safety**: Validated types (Rust compile-time, Python/TypeScript runtime)
- **Observability**: Distributed tracing on every operation (Rust only)
- **Resilience**: Automatic retries with exponential backoff (all client libraries)

---

## Code Examples

### Rust (Full Feature Access)
```rust
use kotadb::{Database, SymbolExtractor};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database with codebase intelligence
    let db = Database::new("~/.kota/db").await?;
    
    // Index a Rust project
    let stats = db.index_codebase("./my-project").await?;
    println!("Indexed {} symbols from {} files", stats.symbols, stats.files);
    
    // Search for symbols
    let symbols = db.search_symbols("FileStorage").await?;
    for symbol in symbols {
        println!("{}: {} at {}", symbol.kind, symbol.name, symbol.location);
    }
    
    // Find all callers of a function
    let callers = db.find_callers("process_data").await?;
    println!("Function called from {} locations", callers.len());
    
    Ok(())
}
```

### Python (Client Library)
```python
from kotadb import KotaDB

# Connect to KotaDB server
db = KotaDB("http://localhost:8080")

# Index a codebase
stats = db.index_codebase("/path/to/your/project")
print(f"Indexed {stats['symbols']} symbols from {stats['files']} files")

# Search for specific symbols
symbols = db.search_symbols("DatabaseConnection")
for symbol in symbols:
    print(f"{symbol['type']}: {symbol['name']} at {symbol['location']}")

# Find dependencies
callers = db.find_callers("process_data")
print(f"Function called from {len(callers)} locations")

# Analyze impact
impact = db.analyze_impact("StorageError")
print(f"Changing this affects {len(impact['affected'])} files")
```

### TypeScript (Client Library)
```typescript
import { KotaDB } from 'kotadb-client';

// Connect to KotaDB server
const db = new KotaDB({ url: 'http://localhost:8080' });

// Index a codebase
const stats = await db.indexCodebase('/path/to/your/project');
console.log(`Indexed ${stats.symbols} symbols from ${stats.files} files`);

// Search for specific symbols
const symbols = await db.searchSymbols('DatabaseConnection');
for (const symbol of symbols) {
    console.log(`${symbol.type}: ${symbol.name} at ${symbol.location}`);
}

// Find dependencies
const callers = await db.findCallers('processData');
console.log(`Function called from ${callers.length} locations`);

// Analyze impact
const impact = await db.analyzeImpact('StorageError');
console.log(`Changing this affects ${impact.affected.length} files`);
```

---

## Query Capabilities

### Currently Implemented

**Text Search** - Full-text search with fuzzy matching:
```python
# Simple text search
results = db.query("rust programming")

# With filters and limits
results = db.query("design patterns", limit=10)
```

**Vector Search** - Find similar documents using embeddings:
```python
# Vector similarity search (requires embeddings)
results = db.semantic_search("distributed systems concepts")
```

**Path Queries** - Wildcard path matching:
```bash
# CLI wildcard search
kotadb search "*"  # List all documents
kotadb search "/projects/*"  # Documents in projects folder
```

### Recently Added (v0.5.0+)

✨ **New Codebase Intelligence Features**:
- **Symbol Extraction**: Parse and index all code symbols
- **Dependency Graph**: Track function calls and usage
- **Impact Analysis**: See what breaks if you change something
- **Dual Storage**: Optimized separation of documents and graphs

### Planned Enhancements

⚠️ **Note**: The following features are part of our roadmap but are **not currently available**:

- **Relationship Queries**: Find callers, analyze impact, track dependencies 
- **Cross-Language Support**: Beyond Rust (Python, JS, Go)
- **Real-time Updates**: Live code change tracking
- **Advanced Refactoring**: Automated safe refactoring suggestions

See the [Roadmap](#roadmap) section for implementation timeline.

---

## Current Features (What's Actually Working)

### ✅ Production Ready
- **Codebase Analysis**: Symbol extraction with 17,128+ symbols from KotaDB itself
- **Dependency Tracking**: Full relationship graph of function calls and usage
- **Impact Analysis**: Understand what breaks when you change code
- **Lightning Fast Search**: <3ms trigram search (210x improvement)
- **Storage Engine**: WAL, compression, crash recovery
- **B+ Tree Index**: Path-based lookups, wildcard queries
- **Vector Search**: HNSW-based similarity search
- **Client Libraries**: Python, TypeScript/JavaScript, Rust
- **Binary Distribution**: Pre-built binaries for all platforms
- **MCP Server**: Model Context Protocol integration

### 🔧 Currently Limited
- **Search Filters**: Basic tag and path filtering only
- **Query Builder**: Simple text queries (no complex operators)
- **Bulk Operations**: Available but not optimized

## Roadmap

### Phase 1: Core Stability (Current)
- ✅ Storage engine with persistence
- ✅ Basic indexing (B+ tree, trigram)
- ✅ Client libraries (Python, TypeScript)
- ✅ Binary distribution

### Phase 2: Enhanced Search (Q1 2025)
- 🚧 Advanced query filters and operators
- 🚧 Hybrid search (text + semantic combined)
- 🚧 Field-specific search capabilities
- 🚧 Performance optimizations

### Phase 3: Graph & Relationships (Q2 2025)
- ⏳ Graph index implementation
- ⏳ Document relationship tracking
- ⏳ Relationship-based queries
- ⏳ Dependency analysis

### Phase 4: Temporal & Analytics (Q3 2025)
- ⏳ Temporal indexing and queries
- ⏳ Time-based aggregations
- ⏳ Pattern analysis
- ⏳ Productivity metrics

### Phase 5: Advanced Intelligence (Q4 2025)
- ⏳ Enhanced code analysis patterns
- ⏳ Intelligent query optimization
- ⏳ Context-aware suggestions
- ⏳ Query suggestions

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
kotadb = "0.5.0"
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
| Document CRUD | ✅ | ✅ | ✅ | ❌ |
| Text Search | ✅ | ✅ | ✅ | ❌ |
| Semantic Search | ✅ | ✅ | ✅ | ❌ |
| Hybrid Search | ✅ | ✅ | ✅ | ❌ |
| **Type Safety** | | | | |
| Validated Types | ✅ | ✅ | ✅ | ❌ |
| Builder Patterns | ✅ | ✅ | ✅ | ❌ |
| **Advanced Features** | | | | |
| Query Routing | ✅ | ❌* | ❌* | ❌* |
| Graph Queries | 🚧 | ❌ | ❌ | ❌ |
| Direct Storage Access | ✅ | ❌ | ❌ | ❌ |
| Observability/Tracing | ✅ | ❌ | ❌ | ❌ |
| **Development** | | | | |
| Connection Pooling | ✅ | ✅ | ✅ | ❌ |
| Retry Logic | ✅ | ✅ | ✅ | ❌ |
| Error Handling | ✅ | ✅ | ✅ | ❌ |

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
