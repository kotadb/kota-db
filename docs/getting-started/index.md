# Getting Started with KotaDB

This guide will help you get KotaDB up and running quickly. We'll cover installation, basic configuration, and your first database operations.

## Quick Start (60 Seconds)

### Using Python Client (Easiest)

```bash
# Install the Python client
pip install kotadb-client
```

```python
from kotadb import KotaDB

# Connect to KotaDB server
db = KotaDB("http://localhost:8080")

# Insert a document
doc_id = db.insert({
    "path": "/notes/quickstart.md",
    "title": "Quick Start Note",
    "content": "My first KotaDB document!"
})

# Search for documents
results = db.query("first document")
for result in results.results:
    print(f"Found: {result.document.title}")
```

## Installation Options

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

#### Go
```bash
go get github.com/jayminwest/kota-db/clients/go
```

### Server Installation

#### Prerequisites

For building from source, ensure you have:

- **Rust** 1.75.0 or later ([Install Rust](https://rustup.rs/))
- **Git** for cloning the repository
- **Just** command runner (optional but recommended)

#### Quick Installation

##### Using Docker (Recommended)

```bash
# Pull the pre-built Docker image
docker pull ghcr.io/jayminwest/kota-db:latest

# Run KotaDB server
docker run -p 8080:8080 -v $(pwd)/data:/data ghcr.io/jayminwest/kota-db:latest serve
```

##### From Source

```bash
# Clone the repository
git clone https://github.com/jayminwest/kota-db.git
cd kota-db

# Build the project
cargo build --release

# Start the server
cargo run --bin kotadb -- serve

# Run tests to verify installation
cargo test --lib
```

##### Using Just

If you have `just` installed:

```bash
# Build and test
just build
just test

# Start development server
just dev
```

##### Using Cargo Install

```bash
# Install from crates.io
cargo install kotadb

# Start the server
kotadb serve
```

## First Steps

### 1. Create a Configuration File

Create a `kotadb.toml` configuration file:

```toml
[storage]
path = "./data"
cache_size = 1000
wal_enabled = true

[server]
host = "127.0.0.1"
port = 8080
max_connections = 100

[indices]
primary_enabled = true
trigram_enabled = true
vector_enabled = false
```

### 2. Start the Server

```bash
# Using cargo
cargo run -- --config kotadb.toml

# Or with the built binary
./target/release/kotadb --config kotadb.toml
```

### 3. Verify Installation

Check that the server is running:

```bash
# Check server status
curl http://localhost:8080/health

# View database statistics
cargo run stats
```

## Basic Operations

### Insert a Document

```rust
use kotadb::{DocumentBuilder, create_file_storage};

#[tokio::main]
async fn main() -> Result<()> {
    // Create storage instance
    let storage = create_file_storage("./data", Some(1000)).await?;
    
    // Build a document
    let doc = DocumentBuilder::new()
        .path("/docs/example.md")?
        .title("My First Document")?
        .content(b"# Hello KotaDB\nThis is my first document.")?
        .build()?;
    
    // Insert the document
    storage.insert(doc).await?;
    
    Ok(())
}
```

### Search Documents

```rust
// Full-text search
let results = storage.search("Hello KotaDB").await?;

// Wildcard search
let all_docs = storage.search("*").await?;

// Path-based search
let docs_in_folder = storage.search("/docs/*").await?;
```

## Next Steps

Now that you have KotaDB running, explore:

- [Configuration Options](configuration.md) - Detailed configuration guide
- [Basic Operations](basic-operations.md) - CRUD operations and queries
- [API Reference](../api/index.md) - Complete API documentation
- [Architecture Overview](../architecture/index.md) - Understanding KotaDB internals

## Getting Help

If you encounter issues:

1. Check the [Troubleshooting Guide](../operations/troubleshooting.md)
2. Search [GitHub Issues](https://github.com/jayminwest/kota-db/issues)
3. Ask in [GitHub Discussions](https://github.com/jayminwest/kota-db/discussions)
4. Review the [FAQ](../reference/faq.md)

## Example Projects

Explore complete examples in the [examples directory](https://github.com/jayminwest/kota-db/tree/main/examples):

- **Basic CRUD** - Simple document operations
- **Search Examples** - Various search patterns
- **MCP Integration** - LLM integration examples
- **Performance Testing** - Benchmark scripts