# Quick Start Guide

Get KotaDB running in under 5 minutes!

## 1. Install KotaDB

=== "From Source"

    ```bash
    git clone https://github.com/jayminwest/kota-db.git
    cd kota-db
    cargo build --release
    ```

=== "Using Docker"

    ```bash
    docker run -p 8080:8080 kotadb/kotadb:latest
    ```

## 2. Run Your First Command

```bash
# Start the database with development configuration
cargo run -- --config kotadb-dev.toml

# In another terminal, check status
cargo run stats
```

## 3. Insert and Search Documents

### Using the Rust API

```rust
use kotadb::{DocumentBuilder, create_file_storage};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize storage
    let storage = create_file_storage("./data", Some(1000)).await?;
    
    // Create a document
    let doc = DocumentBuilder::new()
        .path("/hello.md")?
        .content(b"Hello, KotaDB!")?
        .build()?;
    
    // Insert it
    storage.insert(doc).await?;
    
    // Search for it
    let results = storage.search("Hello").await?;
    println!("Found {} documents", results.len());
    
    Ok(())
}
```

### Using the HTTP API

```bash
# Insert a document
curl -X POST http://localhost:8080/documents \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/api-test.md",
    "content": "Testing the HTTP API"
  }'

# Search for documents
curl http://localhost:8080/search?q=Testing
```

### Using Python Client

```python
from kotadb import KotaDBClient

# Connect to KotaDB
client = KotaDBClient("http://localhost:8080")

# Insert a document
client.insert({
    "path": "/python-test.md",
    "content": "Hello from Python!"
})

# Search documents
results = client.search("Python")
print(f"Found {len(results)} documents")
```

## 4. What's Next?

**Congratulations!** You've successfully:
- ✅ Installed KotaDB
- ✅ Started the database server
- ✅ Inserted your first document
- ✅ Performed a search query

### Explore Further

- 📖 [Full Installation Guide](getting-started/installation.md) - Detailed installation options
- ⚙️ [Configuration](getting-started/configuration.md) - Customize KotaDB settings
- 🔍 [Search Features](architecture/query-engine.md) - Advanced search capabilities
- 🚀 [Performance Tuning](advanced/performance-tuning.md) - Optimize for your workload
- 🤖 [MCP Integration](api/mcp-server.md) - Connect with LLMs

### Join the Community

- ⭐ [Star us on GitHub](https://github.com/jayminwest/kota-db)
- 💬 [Join Discussions](https://github.com/jayminwest/kota-db/discussions)
- 🐛 [Report Issues](https://github.com/jayminwest/kota-db/issues)
- 🤝 [Contribute](contributing.md) - We welcome contributions!