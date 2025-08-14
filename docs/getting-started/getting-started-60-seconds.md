# Getting Started in 60 Seconds

⏱️ **From zero to first query in under 60 seconds**

## 🚀 Super Quick Start (30 seconds)

### Option 1: One-Command Docker Setup
```bash
# Start KotaDB with demo data and run Python example
docker-compose -f docker-compose.quickstart.yml up -d
docker-compose -f docker-compose.quickstart.yml --profile demo up python-demo

# Or TypeScript example
docker-compose -f docker-compose.quickstart.yml --profile demo up typescript-demo

# Optional: Web UI at http://localhost:3000
docker-compose -f docker-compose.quickstart.yml --profile ui up web-ui
```

### Option 2: Shell Script Installation
```bash
# Automatic installation with demo data
curl -sSL https://raw.githubusercontent.com/jayminwest/kota-db/main/quickstart/install.sh | bash

# That's it! Server running on http://localhost:8080 with sample data
```

## 📦 Manual Setup (If you prefer step-by-step)

### 1. Choose Your Language (15 seconds)

#### Python (Recommended for Quick Testing)
```bash
pip install kotadb-client
```

#### TypeScript/JavaScript (Full Type Safety)  
```bash
npm install kotadb-client
# or
yarn add kotadb-client
```

#### Rust (Building from Source)
```bash
git clone https://github.com/jayminwest/kota-db.git
cd kota-db && cargo build --release
```

### 2. Start the Server (20 seconds)

#### Using Docker (Easiest)
```bash
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve
```

#### Using Rust Binary
```bash
cargo run --bin kotadb -- serve
# Server starts at http://localhost:8080
```

### 3. Your First Document (25 seconds)

### Python - Type-Safe with Builders
```python
from kotadb import KotaDB, DocumentBuilder

# Connect and insert
db = KotaDB("http://localhost:8080")
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path("/notes/first.md")
    .title("My First Note")
    .content("Hello KotaDB!")
    .add_tag("quickstart")
)
```

### TypeScript - Runtime Validated
```typescript
import { KotaDB, DocumentBuilder } from 'kotadb-client';

const db = new KotaDB({ url: 'http://localhost:8080' });
const docId = await db.insertWithBuilder(
  new DocumentBuilder()
    .path("/notes/first.md")
    .title("My First Note")
    .content("Hello KotaDB!")
    .addTag("quickstart")
);
```

### Rust - Compile-Time Safety
```rust
use kotadb::{create_file_storage, DocumentBuilder};

let storage = create_file_storage("./data", Some(1000)).await?;
let doc = DocumentBuilder::new()
    .path("/notes/first.md")?
    .title("My First Note")?
    .content(b"Hello KotaDB!")?
    .add_tag("quickstart")?
    .build()?;
    
storage.insert(doc).await?;
```

### 4. Search Your Data

### Full-Text Search
```python
results = db.query("quickstart")
print(f"Found {len(results.documents)} documents")
```

### Structured Query with Builder
```python
from kotadb import QueryBuilder

results = db.query_with_builder(
    QueryBuilder()
    .text("Hello")
    .tag_filter("quickstart")
    .limit(10)
)
```

## Next Steps

✅ **You're up and running!** Your first document is stored and searchable.

### 🌐 Complete Application Examples
- **[Flask Web App](../../examples/flask-web-app/)** - Full web application with UI and REST API
- **[Note-Taking App](../../examples/note-taking-app/)** - Advanced document management 
- **[RAG Pipeline](../../examples/rag-pipeline/)** - AI-powered question answering system

### 📚 Documentation & Guides  
- **[All Examples](../../examples/)** - Comprehensive example collection
- **[API Reference](../api/api_reference.md)** - Full API documentation
- **[Architecture Guide](../architecture/technical_architecture.md)** - How KotaDB works internally

### Production Ready
- **Type Safety**: Runtime validation in Python/TypeScript, compile-time in Rust
- **Performance**: Sub-10ms queries, 3,600+ ops/sec
- **Reliability**: WAL persistence, automatic retries, distributed tracing

---

**Total time: ~60 seconds** ⚡️

KotaDB is ready for your human-AI cognitive workflows!