# KotaDB Examples

This directory contains comprehensive real-world examples demonstrating KotaDB's capabilities for different use cases. Each example shows how to use KotaDB's production-ready Stage 6 component library with realistic data and scenarios.

**New: Complete Application Examples** - Production-ready applications showing real-world integration patterns.

## Available Examples

## 🌐 Complete Application Examples

### [Flask Web App](flask-web-app/) - Production Web Application
**Complete web application with REST API and UI**

- **Features**: Document CRUD, full-text search, RESTful API, web interface
- **Tech Stack**: Flask, KotaDB Python client, HTML/CSS/JS
- **Use Case**: Document management system with web interface
- **Run**: `cd flask-web-app && pip install -r requirements.txt && python app.py`
- **Access**: http://localhost:5000

Perfect for understanding how to integrate KotaDB into web applications.

### [Note-Taking App](note-taking-app/) - Advanced Document Management  
**Sophisticated note organization with hierarchical folders**

- **Features**: Hierarchical folders, advanced search, auto-tagging, export, statistics
- **Tech Stack**: Flask, KotaDB Python client, advanced UI
- **Use Case**: Personal knowledge management, note organization
- **Run**: `cd note-taking-app && pip install -r requirements.txt && python note_app.py`  
- **Access**: http://localhost:5001

Demonstrates advanced KotaDB features like complex queries and metadata management.

### [RAG Pipeline](rag-pipeline/) - AI-Powered Question Answering
**Complete retrieval-augmented generation system**

- **Features**: Document ingestion, vector embeddings, semantic search, AI Q&A
- **Tech Stack**: Python, OpenAI API, scikit-learn, KotaDB client
- **Use Case**: Knowledge base for AI applications, chatbots, research assistants
- **Setup**: `export OPENAI_API_KEY=your_key` (optional - has fallbacks)
- **Run**: `cd rag-pipeline && pip install -r requirements.txt && python rag_demo.py`

Shows how to build production-ready RAG systems with KotaDB as the vector database.

---

## 🦀 Rust Core Examples

### 1. Personal Knowledge Base (`01_personal_knowledge_base.rs`)

**Use Case**: Managing personal knowledge and documentation  
**Features Demonstrated**:
- Document storage with markdown content and metadata
- Full-text search across knowledge base
- Tag-based filtering and organization
- Temporal queries for recent activity
- Performance testing with bulk operations

**Data**: Programming guides, database design principles, learning notes

```bash
cargo run --example 01_personal_knowledge_base
```

**Expected Output**:
- 8 realistic knowledge documents created
- 100 performance test documents generated
- Search demonstrations across different content types
- Storage statistics and performance metrics
- Sub-1ms search performance validation

### 2. Research Project Manager (`02_research_project_manager.rs`)

**Use Case**: Academic research management and literature review  
**Features Demonstrated**:
- Academic paper and citation tracking
- Research note organization
- Literature review workflows
- Citation network analysis
- Progress tracking over time

**Data**: Research papers, academic notes, citation tracking, progress reports

```bash
cargo run --example 02_research_project_manager
```

### 3. Meeting Notes System (`03_meeting_notes_system.rs`)

**Use Case**: Meeting management and organizational memory  
**Features Demonstrated**:
- Meeting note organization and storage
- Temporal queries (find meetings by date/time)
- Action item tracking across meetings
- Participant and decision analysis
- Meeting effectiveness analytics

**Data**: Team standups, client meetings, retrospectives, one-on-ones

```bash
cargo run --example 03_meeting_notes_system
```

## Running the Examples

### Prerequisites

1. **Rust**: Version 1.70+ (specified in `rust-toolchain.toml`)
2. **Just**: Task runner (optional, can use cargo directly)

### Quick Start

```bash
# Run all examples in sequence
just examples

# Or run individual examples
cargo run --example 01_personal_knowledge_base
cargo run --example 02_research_project_manager  
cargo run --example 03_meeting_notes_system

# View example output with less logging
RUST_LOG=warn cargo run --example 01_personal_knowledge_base
```

### Data Storage

Each example creates its own data directory:
- Personal KB: `./examples-data/personal-kb/`
- Research Manager: `./examples-data/research-manager/`
- Meeting Notes: `./examples-data/meeting-notes/`

You can inspect the stored markdown files directly in these directories.

## Example Architecture

All examples use KotaDB's production patterns:

### Stage 6 Component Library
- **Validated Types**: Compile-time safety with `ValidatedPath`, `ValidatedTitle`, etc.
- **Builder Patterns**: Fluent APIs for `DocumentBuilder`, `Query`, etc.
- **Wrapper Components**: Automatic tracing, validation, caching, retry logic

### Storage and Indexing
- **FileStorage**: Production-ready file-based storage with ACID guarantees
- **Primary Index**: B+ tree for fast path-based lookups
- **Trigram Index**: Full-text search with relevance ranking
- **Query Engine**: Structured search with filters and limits

### Observability
- **Tracing**: Every operation gets unique trace IDs
- **Metrics**: Performance counters and timing data
- **Logging**: Structured logs with full context

## Performance Expectations

Based on Apple Silicon M-series hardware:

| Operation | Latency | Throughput | Notes |
|-----------|---------|------------|-------|
| Document Insert | <1ms | >1,000/sec | Including indexing |
| Document Retrieval | <1ms | >5,000/sec | With caching |
| Full-Text Search | <10ms | >100/sec | Trigram index |
| Bulk Operations | 5x faster | Batch optimized | vs individual |

## Common Patterns

### Document Creation
```rust
let doc = DocumentBuilder::new()
    .path("/knowledge/rust-patterns.md")?
    .title("Advanced Rust Design Patterns")?
    .content(content.as_bytes())
    .tag("rust")?
    .tag("programming")?
    .build()?;
```

### Search Operations
```rust
// Full-text search
let query = Query::new(Some("ownership".to_string()), None, None, 10)?;
let results = search_index.search(&query).await?;

// Get actual documents
for doc_id in results {
    if let Some(doc) = storage.get(&doc_id).await? {
        println!("Found: {}", doc.title);
    }
}
```

### Storage Operations
```rust
// Insert with automatic tracing and validation
storage.insert(doc.clone()).await?;

// Update indices
primary_index.insert(doc.id.clone(), ValidatedPath::new(&path)?).await?;
search_index.insert(doc.id.clone(), ValidatedPath::new(&path)?).await?;
```

## Extending the Examples

### Adding New Use Cases

1. **Copy an existing example** as a starting point
2. **Update the sample data** generation function
3. **Modify the demonstration functions** for your use case
4. **Add any specialized queries** or analytics

### Custom Content Types

Examples support any UTF-8 content:
- Markdown documents with YAML frontmatter
- Plain text files
- JSON data (as text content)
- Code files with syntax highlighting
- Academic papers in various formats

### Integration Patterns

These examples show how to integrate KotaDB into larger applications:
- **CLI tools**: Command-line interfaces with search
- **Web services**: HTTP APIs with JSON responses  
- **Desktop apps**: Local data management
- **Academic tools**: Research and citation management

## Troubleshooting

### Common Issues

1. **"No documents found"**: Check that documents are being inserted before search
2. **Compilation errors**: Ensure Rust 1.70+ and all dependencies are current
3. **Performance issues**: Verify examples run with `--release` for production speed
4. **Storage permissions**: Ensure write access to example data directories

### Debug Mode

```bash
# Enable debug logging for detailed operation traces
RUST_LOG=debug cargo run --example 01_personal_knowledge_base

# Enable only KotaDB logs
RUST_LOG=kotadb=debug cargo run --example 01_personal_knowledge_base
```

### Performance Profiling

```bash
# Build with optimizations
cargo build --release --examples

# Run with timing
time ./target/release/examples/01_personal_knowledge_base
```

## Contributing

When adding new examples:

1. **Follow the established pattern**: Use the same structure and error handling
2. **Include realistic data**: Examples should demonstrate real-world usage
3. **Document clearly**: Add comprehensive comments and documentation
4. **Test thoroughly**: Ensure examples run reliably on different systems
5. **Update this README**: Add your example to the list above

## Related Documentation

- [AGENT.md](../AGENT.md): Essential development guide
- [README.md](../README.md): Project overview and features  
- [src/examples/](./): Example source code
- [API Documentation](https://docs.rs/kotadb): Generated API docs

Each example is self-contained and can be studied independently to understand different aspects of KotaDB's capabilities.
