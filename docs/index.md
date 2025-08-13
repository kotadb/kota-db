# KotaDB Documentation

## A Custom Database for Distributed Human-AI Cognition

Welcome to the KotaDB documentation! KotaDB is a high-performance, custom database built entirely in Rust with zero external database dependencies, designed specifically for distributed human-AI cognitive workflows.

## Quick Installation

### Python Client (with Type Safety)
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)
```bash
pip install kotadb-client
```

### TypeScript/JavaScript Client (with Type Safety)
[![npm version](https://img.shields.io/npm/v/kotadb-client.svg)](https://www.npmjs.com/package/kotadb-client)
```bash
npm install kotadb-client
# or
yarn add kotadb-client
```

### Server (Docker)
```bash
docker pull ghcr.io/jayminwest/kota-db:latest
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve
```

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } **Quick Start**

    ---

    Get up and running with KotaDB in minutes

    [:octicons-arrow-right-24: Getting started](getting-started/index.md)

-   :material-book-open-variant:{ .lg .middle } **Architecture**

    ---

    Deep dive into KotaDB's design and internals

    [:octicons-arrow-right-24: Learn more](architecture/index.md)

-   :material-api:{ .lg .middle } **API Reference**

    ---

    Complete API documentation and client libraries

    [:octicons-arrow-right-24: Explore APIs](api/index.md)

-   :material-code-tags:{ .lg .middle } **Developer Guide**

    ---

    Build, test, and contribute to KotaDB

    [:octicons-arrow-right-24: Start developing](developer/index.md)

</div>

## Key Features

### 🚀 Performance
- **Sub-10ms query latency** for most operations
- **10x faster bulk operations** compared to traditional databases
- **Memory-efficient** with <2.5x overhead over raw data

### 🛡️ Reliability
- **99% success rate** through 6-stage risk reduction methodology
- **Write-Ahead Logging (WAL)** for data durability
- **Crash recovery** with automatic rollback

### 🔍 Advanced Search
- **Full-text search** with trigram indexing
- **Vector search** for semantic queries (HNSW algorithm)
- **Graph traversal** for relationship queries
- **Natural language** query support

### 🏗️ Architecture
- **Zero external dependencies** - pure Rust implementation
- **Page-based storage** with 4KB pages and checksums
- **Multiple index types** - B+ tree, trigram, vector, graph
- **Component library** with safety wrappers

### 🔧 Developer Experience
- **100% LLM-developed** with comprehensive documentation
- **Type-safe APIs** with compile-time validation
- **Extensive testing** - 243+ tests with property-based testing
- **Observable** with distributed tracing and metrics

## System Requirements

- **Rust**: 1.75.0 or later
- **Operating System**: Linux, macOS, or Windows
- **Memory**: 512MB minimum, 2GB recommended
- **Disk Space**: 100MB for installation + data storage

## Use Cases

KotaDB is designed for applications that require:

- **Human-AI collaboration** with shared cognitive spaces
- **High-performance document storage** with full-text search
- **Semantic search** capabilities with vector embeddings
- **Graph-based relationships** between documents
- **Real-time indexing** with sub-second query response

## Getting Help

<div class="grid cards" markdown>

-   :material-github:{ .lg .middle } **GitHub Issues**

    ---

    Report bugs or request features

    [:octicons-arrow-right-24: Create issue](https://github.com/jayminwest/kota-db/issues)

-   :material-chat:{ .lg .middle } **Discussions**

    ---

    Ask questions and share ideas

    [:octicons-arrow-right-24: Join discussion](https://github.com/jayminwest/kota-db/discussions)

-   :material-book:{ .lg .middle } **Examples**

    ---

    Learn from code examples

    [:octicons-arrow-right-24: View examples](https://github.com/jayminwest/kota-db/tree/main/examples)

</div>

## Latest Updates

!!! success "Version 0.3.0 Released - Production-Ready Type Safety"
    **Major release** with comprehensive TypeScript and Python client type safety! Features validated types, builder patterns, and security protection against path injection attacks. **164 TypeScript tests** and **84 Python tests** ensure production reliability.

!!! tip "TypeScript Client - Complete Type Safety"
    Full runtime validation with `ValidatedPath`, `ValidatedDocumentId`, builder patterns (`DocumentBuilder`, `QueryBuilder`), and comprehensive security validation. Zero breaking changes with backward compatibility.

!!! info "Python Client - Enhanced with Builders"
    Advanced builder patterns and validated types bring the Python client to feature parity with Rust. Includes `UpdateBuilder`, comprehensive error handling, and 79.54% test coverage.

## License

KotaDB is open-source software licensed under the MIT License. See the [LICENSE](https://github.com/jayminwest/kota-db/blob/main/LICENSE) file for details.