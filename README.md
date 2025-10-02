# KotaDB

**A repository intelligence platform that ingests your code, extracts symbols, and builds a queryable knowledge graph for engineering questions.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-271%20passing-brightgreen?style=for-the-badge)](https://github.com/jayminwest/kota-db/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Features](#features)
- [Examples](#examples)
- [Performance](#performance)
- [Documentation](#documentation)
- [Contributing](#contributing)

## Quick Start

### Option 1: Python Client (Recommended)
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)

```bash
# Install and start server
pip install kotadb-client

# Index your codebase and start querying
python -c "
from kotadb import KotaDB, start_server
server = start_server(port=8080)
db = KotaDB('http://localhost:8080')

# Index and search your code
stats = db.index_codebase('./my-project')
results = db.search_code('function_name')
print(f'Found {len(results)} matches')
"
```

### Option 2: Pre-built Binaries
[![GitHub Release](https://img.shields.io/github/v/release/jayminwest/kota-db)](https://github.com/jayminwest/kota-db/releases)

```bash
# macOS (Apple Silicon)
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-macos-arm64.tar.gz | tar xz
./kotadb serve --port 8080

# Linux x64
curl -L https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-linux-x64.tar.gz | tar xz
./kotadb serve --port 8080
```

### Option 3: Docker
```bash
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve
```

## Installation

### Client Libraries

**Python** [![PyPI](https://img.shields.io/pypi/v/kotadb-client)](https://pypi.org/project/kotadb-client/)
```bash
pip install kotadb-client
```

**TypeScript/JavaScript** [![npm](https://img.shields.io/npm/v/kotadb-client)](https://www.npmjs.com/package/kotadb-client)
```bash
npm install kotadb-client
```

**Rust** [![Crates.io](https://img.shields.io/crates/v/kotadb.svg)](https://crates.io/crates/kotadb)
```bash
cargo add kotadb
```

**Go** - Coming Soon (Track progress: [#114](https://github.com/jayminwest/kota-db/issues/114))

### Server Installation

**From Binaries**: Download from [releases](https://github.com/jayminwest/kota-db/releases)  
**From Source**: `cargo install kotadb`  
**Docker**: `docker pull ghcr.io/jayminwest/kota-db:latest`

## Features

### Current Capabilities

✅ **Codebase Intelligence**
- Symbol extraction from source code (functions, classes, variables)
- Dependency tracking and impact analysis
- Cross-reference detection and caller analysis

✅ **High-Performance Search**
- Full-text search with <3ms latency (210x improvement)
- Symbol-based search with pattern matching
- Path-based queries with wildcard support

✅ **Production Ready**
- Crash-safe storage with Write-Ahead Logging
- Type-safe client libraries for Python, TypeScript, Rust
- Comprehensive test coverage (271 passing tests)
- Zero external database dependencies

✅ **Developer Experience**
- REST API for HTTP integration
- MCP server for AI assistant integration
- Pre-built binaries for all platforms

### Performance Highlights

Real-world benchmarks on Apple Silicon:

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Symbol Search | 277 µs | 3,600 ops/sec |
| Text Search | <3 ms | 333+ queries/sec |
| B+ Tree Lookup | 489 µs | 2,000 queries/sec |

*Tested on KotaDB's own codebase (21,000+ symbols)*

### Known Limitations

⚠️ **Currently Limited**
- Limited cross-language support (Rust focus)
- Basic query operators (no complex filtering)
- UX improvements needed for CLI interface

## Examples

### Python
```python
from kotadb import KotaDB

db = KotaDB("http://localhost:8080")

# Index your codebase
stats = db.index_codebase("./my-project")
print(f"Indexed {stats['symbols']} symbols")

# Search for symbols
symbols = db.search_symbols("DatabaseConnection")
for symbol in symbols:
    print(f"{symbol['type']}: {symbol['name']} at {symbol['location']}")

# Find function callers
callers = db.find_callers("process_data")
print(f"Called from {len(callers)} locations")

# Analyze change impact
impact = db.analyze_impact("StorageError")
print(f"Would affect {len(impact['affected'])} files")
```

### Rust
```rust
use kotadb::Database;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::new("~/.kota/db").await?;
    
    // Index and search
    let stats = db.index_codebase("./my-project").await?;
    let symbols = db.search_symbols("FileStorage").await?;
    let callers = db.find_callers("process_data").await?;
    
    Ok(())
}
```

### CLI
```bash
# Index your codebase
kotadb index-codebase ./my-project

# Search operations
kotadb search-code "async fn"
kotadb search-symbols "Storage*"
kotadb find-callers FileStorage
kotadb analyze-impact Config

# Database operations
kotadb stats --symbols
kotadb validate
```

## Performance

KotaDB achieves sub-10ms query latency through:

- **Optimized Indices**: B+ tree, trigram, and vector search
- **Native Storage**: Custom page-based storage engine
- **Memory Efficiency**: <2.5x memory overhead vs raw data
- **Concurrent Access**: Lock-free read operations

See the architecture notes on performance in docs/architecture/technical_architecture.md for detailed analysis.

## Documentation

### HTTP API & MCP Bridge

When built with the `mcp-server` feature, the server also exposes MCP-over-HTTP endpoints under `/mcp/*` for AI assistants:

```bash
# List tools (GET preferred; POST supported)
curl -sS -H "Authorization: Bearer $API_KEY" http://localhost:8080/mcp/tools

# Run text search tool
curl -sS -X POST -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query":"storage","limit":10}' \
  http://localhost:8080/mcp/tools/search_code

# Bridge stats/discovery
curl -sS -H "Authorization: Bearer $API_KEY" \
  http://localhost:8080/mcp/tools/stats
```

### Search Sanitization & Thresholds

See `docs/search_sanitization_and_thresholds.md` for details on:
- Differences between `sanitize_search_query` and `sanitize_path_aware_query`.
- Optional `strict-sanitization` feature for high-threat environments.
- Trigram matching thresholds and how they balance precision vs recall.

### CI-Aware Test Thresholds

Stress/performance tests support CI-aware, env-overridable thresholds. See `docs/ci_aware_test_thresholds.md` for variables, defaults, and examples.

Bridge errors use a stable schema `{ success: false, error: { code, message } }`. Common codes:
`feature_disabled`, `tool_not_found`, `registry_unavailable`, `internal_error`.

- **[Getting Started](docs/getting-started/)** - Installation and first steps
- **[API Reference](docs/api/)** - Complete API documentation  
- **[Architecture](docs/architecture/)** - Repository ingestion, symbol extraction, and analysis pipeline
- **[Developer Guide](docs/development-guides/dev_guide.md)** - Development workflow
- **[Agent Guide](AGENT.md)** - LLM agent collaboration protocol

## Contributing

KotaDB is developed 100% by LLM agents following a structured workflow:

1. Open an issue describing the change
2. Agents review and implement following [AGENT.md](AGENT.md) protocols
3. Changes validated through comprehensive testing
4. Documentation updated automatically

See [Contributing Guide](docs/contributing.md) for details.

## License

MIT - See [LICENSE](LICENSE) for details.

---

<sub>Built for AI-assisted development • Inspired by LevelDB, Tantivy, and FAISS</sub>
