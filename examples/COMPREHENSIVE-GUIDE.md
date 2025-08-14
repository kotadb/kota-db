# KotaDB Comprehensive Examples Guide

🎯 **Complete working examples across all client libraries and use cases**

This guide consolidates all KotaDB examples into a single comprehensive resource, making it easy to find the right example for your language and use case.

## Quick Navigation

| Language | Basic Usage | Builder Patterns | Integration Tests | Performance |
|----------|-------------|-----------------|-------------------|-------------|
| **Python** | [basic_usage.py](../clients/python/examples/basic_usage.py) | [builder_patterns.py](../clients/python/examples/builder_patterns.py) | [integration_test.py](../clients/python/examples/integration_test.py) | [performance_test.py](../clients/python/examples/performance_test.py) |
| **TypeScript** | [basic-usage.ts](../clients/typescript/examples/basic-usage.ts) | [type-safety.ts](../clients/typescript/examples/type-safety.ts) | [integration-test.ts](../clients/typescript/examples/integration-test.ts) | [builder-patterns.ts](../clients/typescript/examples/builder-patterns.ts) |
| **Rust** | [01_personal_knowledge_base.rs](01_personal_knowledge_base.rs) | [02_research_project_manager.rs](02_research_project_manager.rs) | [03_meeting_notes_system.rs](03_meeting_notes_system.rs) | [standalone_usage.rs](standalone_usage.rs) |

## 🚀 Quick Start by Language

### Python (Recommended for Quick Testing)
```bash
# Install client
pip install kotadb-client

# Start server (in separate terminal)
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve

# Run example
cd clients/python/examples
python basic_usage.py
```

### TypeScript/JavaScript (Full Type Safety)
```bash
# Install client
npm install kotadb-client

# Start server (in separate terminal)  
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve

# Run example
cd clients/typescript/examples
npx ts-node basic-usage.ts
```

### Rust (Full Feature Access)
```bash
# Clone and build
git clone https://github.com/jayminwest/kota-db.git
cd kota-db

# Run comprehensive example
cargo run --example 01_personal_knowledge_base
```

## 📚 Examples by Use Case

### Personal Knowledge Management
Perfect for developers, researchers, and knowledge workers organizing personal notes and documentation.

| Language | Example | Features |
|----------|---------|----------|
| **Rust** | [01_personal_knowledge_base.rs](01_personal_knowledge_base.rs) | Full storage access, B+ tree indexing, trigram search |
| **Python** | [comprehensive_usage.py](../clients/python/examples/comprehensive_usage.py) | Builder patterns, type safety, HTTP client |
| **TypeScript** | [basic-usage.ts](../clients/typescript/examples/basic-usage.ts) | Type safety, async operations, error handling |

**Demo data:** Programming guides, database principles, learning notes
**Key features:** Document storage, full-text search, tag filtering, temporal queries

### Research Project Management
Ideal for academic researchers, literature review, and citation tracking.

| Language | Example | Features |
|----------|---------|----------|
| **Rust** | [02_research_project_manager.rs](02_research_project_manager.rs) | Academic paper tracking, citation networks |
| **Python** | [integration_test.py](../clients/python/examples/integration_test.py) | Research workflow testing, validation |
| **TypeScript** | [integration-test.ts](../clients/typescript/examples/integration-test.ts) | Academic data structures, testing |

**Demo data:** Research papers, academic notes, citation tracking, progress reports
**Key features:** Literature review workflows, citation analysis, progress tracking

### Meeting Notes & Organizational Memory
Enterprise-ready meeting management and organizational knowledge capture.

| Language | Example | Features |
|----------|---------|----------|
| **Rust** | [03_meeting_notes_system.rs](03_meeting_notes_system.rs) | Meeting organization, action items, analytics |
| **Python** | [builder_patterns.py](../clients/python/examples/builder_patterns.py) | Structured meeting data, type safety |
| **TypeScript** | [type-safety.ts](../clients/typescript/examples/type-safety.ts) | Meeting metadata, participant tracking |

**Demo data:** Team standups, client meetings, retrospectives, one-on-ones
**Key features:** Temporal queries, action item tracking, meeting analytics

## ⚡ Performance Examples

### Benchmarking & Load Testing
Measure KotaDB performance in your environment.

| Language | Example | Benchmarks |
|----------|---------|------------|
| **Python** | [performance_test.py](../clients/python/examples/performance_test.py) | Insert/query latency, throughput measurement |
| **Rust** | Built-in benchmarks | `cargo bench` - Production performance testing |
| **TypeScript** | [integration-test.ts](../clients/typescript/examples/integration-test.ts) | Basic performance validation |

**Expected Performance (Apple Silicon):**
- Document Insert: `<1ms`, >1,000/sec
- Document Retrieval: `<1ms`, >5,000/sec  
- Full-Text Search: `<10ms`, >100/sec
- Bulk Operations: 5x faster than individual

## 🛡️ Type Safety Examples

### Builder Patterns & Validation
Production-ready type safety across all client libraries.

| Language | Type Safety Level | Example |
|----------|------------------|---------|
| **Rust** | Compile-time | [All examples] - Zero runtime overhead |
| **Python** | Runtime validation | [builder_patterns.py](../clients/python/examples/builder_patterns.py) - Validated types |
| **TypeScript** | Compile + runtime | [type-safety.ts](../clients/typescript/examples/type-safety.ts) - Full IntelliSense |

**Validated Types Available:**
- `ValidatedPath` - Prevents directory traversal attacks
- `ValidatedDocumentId` - Ensures proper UUID format
- `ValidatedTitle` - Non-empty titles with length limits
- `ValidatedTimestamp` - Reasonable time range validation

### Builder Pattern Examples
```python
# Python - Runtime validation
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path(ValidatedPath("/secure/path.md"))
    .title("Validated Title")
    .content("Safe content")
    .add_tag("security")
)
```

```typescript
// TypeScript - Compile-time + runtime safety
const docId = await db.insertWithBuilder(
  new DocumentBuilder()
    .path("/secure/path.md")      // IDE autocomplete
    .title("Validated Title")     // Type checking
    .content("Safe content")      // Runtime validation
    .addTag("security")
);
```

```rust
// Rust - Zero-cost compile-time safety
let doc = DocumentBuilder::new()
    .path("/secure/path.md")?     // Compile-time validation
    .title("Validated Title")?    // No runtime overhead
    .content(b"Safe content")?    // Memory safe
    .add_tag("security")?
    .build()?;
```

## 🧪 Integration Testing

### End-to-End Validation
Comprehensive test suites for CI/CD and production validation.

| Language | Test Suite | Coverage |
|----------|-----------|----------|
| **Python** | [integration_test.py](../clients/python/examples/integration_test.py) | CRUD, search, builders, error handling |
| **TypeScript** | [integration-test.ts](../clients/typescript/examples/integration-test.ts) | Full API coverage, concurrent operations |
| **Rust** | `cargo test` | Unit + integration + property tests |

**Test Categories:**
- ✅ Basic CRUD operations
- ✅ Search capabilities (text, semantic, hybrid)
- ✅ Builder pattern validation
- ✅ Error handling and edge cases
- ✅ Concurrent operation safety
- ✅ Performance regression testing

## 📋 Running Examples Step-by-Step

### 1. Start KotaDB Server
Choose one method:

```bash
# Option A: Docker (recommended)
docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve

# Option B: From source
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo run --bin kotadb -- serve
```

### 2. Choose Your Language & Example

#### Python Examples
```bash
# Install client
pip install kotadb-client

# Navigate to examples
cd clients/python/examples

# Run basic usage
python basic_usage.py

# Run builder patterns (recommended for production)
python builder_patterns.py

# Run comprehensive demo
python comprehensive_usage.py

# Test your setup
python integration_test.py

# Benchmark performance
python performance_test.py
```

#### TypeScript Examples
```bash
# Install client
npm install kotadb-client

# Navigate to examples
cd clients/typescript/examples

# Run basic usage
npx ts-node basic-usage.ts

# Run type safety demo
npx ts-node type-safety.ts

# Run builder patterns
npx ts-node builder-patterns.ts

# Test your setup
npx ts-node integration-test.ts
```

#### Rust Examples
```bash
# From project root
cargo run --example 01_personal_knowledge_base
cargo run --example 02_research_project_manager  
cargo run --example 03_meeting_notes_system

# Run with reduced logging
RUST_LOG=warn cargo run --example 01_personal_knowledge_base

# Performance benchmarks
cargo bench
```

## 🎯 Use Case Selection Guide

### Choose Python If:
- ✅ Rapid prototyping and testing
- ✅ Data science and ML workflows
- ✅ Quick integration with existing Python systems
- ✅ Runtime type safety is acceptable

### Choose TypeScript If:
- ✅ Web application integration
- ✅ Node.js backend services
- ✅ Full IDE support with IntelliSense
- ✅ Modern JavaScript ecosystem

### Choose Rust If:
- ✅ Maximum performance required
- ✅ Embedded or system-level integration
- ✅ Compile-time guarantees essential
- ✅ Direct storage engine access needed

## 🔧 Advanced Examples

### Custom Integration Patterns
All examples demonstrate production-ready patterns:

```python
# Python: Async context management
async with KotaDB("http://localhost:8080") as db:
    doc_id = await db.insert_with_builder(
        DocumentBuilder().path("/async/doc.md").title("Async Doc")
    )
```

```typescript
// TypeScript: Error handling with custom types
try {
    const result = await db.query('search term');
    result.results.forEach(doc => processDocument(doc));
} catch (error: KotaDBError) {
    console.error(`KotaDB error: ${error.message}`);
}
```

```rust
// Rust: Zero-copy operations with tracing
#[tracing::instrument]
async fn process_documents(storage: &impl Storage) -> Result<()> {
    let docs = storage.bulk_get(&document_ids).await?;
    // Process with zero allocations...
}
```

## 🚨 Troubleshooting

### Common Issues & Solutions

#### Connection Problems
```
❌ Failed to connect: Connection refused
```
**Solution:** Start KotaDB server: `docker run -p 8080:8080 ghcr.io/jayminwest/kota-db:latest serve`

#### Client Library Installation
```
❌ Module not found: kotadb-client
```
**Solutions:**
- Python: `pip install kotadb-client`
- TypeScript: `npm install kotadb-client`  
- Rust: Add to `Cargo.toml`: `kotadb = "0.3.0"`

#### Validation Errors (Expected Behavior)
```
❌ ValidationError: Path contains null bytes
```
**This is working correctly** - KotaDB prevents directory traversal attacks.

#### Performance Issues
```
❌ Slow query performance
```
**Solutions:**
- Use `--release` builds for Rust
- Check server is running, not just client
- Review query patterns in performance examples

## 📖 Next Steps

### Learning Path
1. **Start here:** [Getting Started in 60 Seconds](../docs/getting-started/getting-started-60-seconds.md)
2. **Try examples:** Pick your language above and run basic examples
3. **Build something:** Use builder patterns for production code
4. **Scale up:** Review performance examples and benchmarks
5. **Deploy:** Check out Docker configurations and monitoring

### Production Deployment
- **Docker:** `ghcr.io/jayminwest/kota-db:latest`
- **Monitoring:** Built-in metrics and tracing (Rust)
- **High Availability:** Review clustering documentation
- **Security:** All examples demonstrate input validation

### Contributing Examples
When adding new examples:
1. **Follow existing patterns** in this guide
2. **Include all three languages** when possible
3. **Add comprehensive documentation** 
4. **Ensure examples are tested** and working
5. **Update this guide** with your new examples

---

## 📚 Related Documentation

- **[Main README](../README.md)** - Project overview and installation
- **[API Reference](../docs/api/api_reference.md)** - Complete API documentation
- **[Architecture Guide](../docs/architecture/technical_architecture.md)** - System internals
- **[Performance Tuning](../docs/PERFORMANCE.md)** - Optimization guide
- **[Development Guide](../docs/development-guides/dev_guide.md)** - Contributing guide

---

<sub>**All examples are tested and working.** Choose your language, run an example, and start building with KotaDB!</sub>