#!/usr/bin/env cargo run --bin
//! # Personal Knowledge Base Example
//!
//! This example demonstrates using KotaDB as a personal knowledge management system.
//! It shows:
//! - Document storage with markdown content
//! - Full-text search across documents
//! - Document relationships and tagging
//! - Temporal queries (finding recent documents)
//! - Performance with realistic data loads
//!
//! ## Usage
//! ```bash
//! cargo run --example 01_personal_knowledge_base
//! ```

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, init_logging, DocumentBuilder,
    Index, Query, Storage, ValidatedPath,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize observability
    let _ = init_logging();
    println!("🧠 Personal Knowledge Base - KotaDB Example");
    println!("============================================\n");

    // Create storage with all Stage 6 safety features
    let mut storage = create_file_storage("./examples-data/personal-kb", Some(1000)).await?;

    // Create indices for different search patterns
    let mut primary_index =
        create_primary_index("./examples-data/personal-kb-primary", Some(1000)).await?;
    let mut search_index =
        create_trigram_index("./examples-data/personal-kb-search", Some(1000)).await?;

    println!("📚 Setting up personal knowledge base...");

    // Create realistic knowledge base content
    let knowledge_docs = create_sample_knowledge_base();

    println!(
        "📝 Adding {} documents to knowledge base...",
        knowledge_docs.len()
    );

    // Insert documents with progress tracking
    for (i, (path, title, content, tags)) in knowledge_docs.iter().enumerate() {
        let mut builder = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(content.as_bytes());

        // Add tags individually
        for tag in tags {
            builder = builder.tag(tag)?;
        }

        let doc = builder.build()?;

        // Store in both storage and indices
        storage.insert(doc.clone()).await?;
        primary_index
            .insert(doc.id, ValidatedPath::new(path)?)
            .await?;
        search_index
            .insert(doc.id, ValidatedPath::new(path)?)
            .await?;

        if i % 10 == 0 {
            println!("  📄 Added {} documents...", i + 1);
        }
    }

    println!("✅ Knowledge base populated successfully!\n");

    // Demonstrate different search patterns
    demonstrate_search_capabilities(&storage, &primary_index, &search_index).await?;

    // Show performance characteristics
    demonstrate_performance(&mut storage, &mut primary_index, &mut search_index).await?;

    println!("\n🎉 Personal Knowledge Base example completed!");
    println!("   Data stored in: ./examples-data/personal-kb/");
    println!("   You can inspect the markdown files directly!");

    Ok(())
}

/// Create realistic knowledge base content covering various topics
fn create_sample_knowledge_base() -> Vec<(String, String, String, Vec<String>)> {
    vec![
        // Programming & Technology
        (
            "/programming/rust-ownership.md".to_string(),
            "Understanding Rust Ownership".to_string(),
            r#"# Understanding Rust Ownership

Rust's ownership system is what makes it memory-safe without garbage collection.

## Key Concepts

### Ownership Rules
1. Each value has a single owner
2. When the owner goes out of scope, the value is dropped
3. Ownership can be transferred (moved)

### Borrowing
- References allow you to use values without taking ownership
- Immutable references: `&T`
- Mutable references: `&mut T`

## Example
```rust
fn main() {
    let s = String::from("hello");
    takes_ownership(s); // s is moved here
    // s is no longer valid
}

fn takes_ownership(some_string: String) {
    println!("{}", some_string);
} // some_string goes out of scope and is dropped
```

## See Also
- [Rust Book Chapter 4](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- Related: lifetimes, borrowing, move semantics
"#.to_string(),
            vec!["rust".to_string(), "programming".to_string(), "memory-safety".to_string()],
        ),

        (
            "/programming/database-design.md".to_string(),
            "Database Design Principles".to_string(),
            r#"# Database Design Principles

Good database design is crucial for performance and maintainability.

## Normalization
- **1NF**: Atomic values, no repeating groups
- **2NF**: 1NF + no partial dependencies
- **3NF**: 2NF + no transitive dependencies

## Indexing Strategy
- Primary keys for unique identification
- Foreign keys for relationships
- Composite indices for multi-column queries
- Full-text indices for search

## Performance Considerations
- Query patterns drive index design
- Avoid over-indexing (write performance cost)
- Consider read vs write workload balance

## KotaDB Example
KotaDB uses multiple index types:
- B+ tree for primary access
- Trigram index for full-text search
- Graph index for relationships
- Vector index for semantic similarity
"#.to_string(),
            vec!["databases".to_string(), "design".to_string(), "indexing".to_string()],
        ),

        // Learning & Education
        (
            "/learning/distributed-systems.md".to_string(),
            "Distributed Systems Fundamentals".to_string(),
            r#"# Distributed Systems Fundamentals

Understanding how to build reliable systems across multiple machines.

## CAP Theorem
You can only guarantee 2 out of 3:
- **Consistency**: All nodes see the same data simultaneously
- **Availability**: System remains operational
- **Partition Tolerance**: System continues despite network failures

## Consensus Algorithms
- **Raft**: Leader-based consensus with log replication
- **PBFT**: Byzantine fault tolerant consensus
- **Paxos**: Classical consensus (complex but foundational)

## Patterns
- **Event Sourcing**: Store events, not state
- **CQRS**: Separate read and write models
- **Saga Pattern**: Distributed transactions
- **Circuit Breaker**: Fail fast pattern

## Real-World Examples
- Cassandra: AP system (eventually consistent)
- PostgreSQL: CP system (strong consistency)
- DNS: AP system (availability over consistency)
"#.to_string(),
            vec!["distributed-systems".to_string(), "consensus".to_string(), "architecture".to_string()],
        ),

        // Project Notes
        (
            "/projects/kotadb-roadmap.md".to_string(),
            "KotaDB Development Roadmap".to_string(),
            r#"# KotaDB Development Roadmap

## Current Status (August 2025)
✅ All 6 risk reduction stages complete
✅ File storage implementation
✅ Primary and trigram indices
✅ Production-ready with full observability

## Next Phase: MCP Integration
- [ ] Model Context Protocol server
- [ ] Natural language query interface
- [ ] Real-time collaboration features
- [ ] Advanced analytics dashboard

## Future Enhancements
- [ ] Distributed deployment support
- [ ] Advanced semantic search
- [ ] Graph query language
- [ ] Multi-tenant architecture

## Performance Targets
- Sub-10ms query latency ✅
- 10,000+ docs/sec throughput ✅
- <2.5x memory overhead ✅
- Zero-downtime deployments (planned)

## Architecture Decisions
- Rust for performance and safety
- Component library pattern for reliability
- Multiple index types for different query patterns
- Human-readable storage format (markdown)
"#.to_string(),
            vec!["kotadb".to_string(), "roadmap".to_string(), "development".to_string()],
        ),

        // Research & Ideas
        (
            "/research/ai-cognition-patterns.md".to_string(),
            "AI Cognition and Knowledge Representation".to_string(),
            r#"# AI Cognition and Knowledge Representation

Exploring how AI systems can effectively represent and manipulate knowledge.

## Key Challenges
1. **Grounding Problem**: Connecting symbols to real-world meaning
2. **Frame Problem**: What changes and what stays the same
3. **Symbol Grounding**: How do symbols acquire meaning?

## Representation Approaches

### Symbolic AI
- Logic-based systems (Prolog, CLIPS)
- Semantic networks
- Frames and schemas
- Production rule systems

### Connectionist AI
- Neural networks for pattern recognition
- Distributed representations
- Emergent behavior from simple rules
- Vector embeddings for semantic similarity

### Hybrid Approaches
- Neuro-symbolic integration
- Knowledge graphs with embeddings
- Differentiable programming
- Causal reasoning with neural networks

## KotaDB's Approach
- Documents as first-class knowledge units
- Multiple index types for different reasoning patterns
- Human-readable format for interpretability
- Temporal awareness for knowledge evolution
"#.to_string(),
            vec!["ai".to_string(), "cognition".to_string(), "knowledge-representation".to_string(), "research".to_string()],
        ),

        // Personal reflections
        (
            "/journal/2025-08-07-kotadb-progress.md".to_string(),
            "KotaDB Progress Reflection".to_string(),
            r#"# KotaDB Progress Reflection - August 7, 2025

## What We've Accomplished
Today we reached a major milestone - KotaDB is production-ready! All the core components are working:

- ✅ **Storage Engine**: Robust file-based storage with full ACID guarantees
- ✅ **Indexing**: B+ tree primary index and trigram full-text search
- ✅ **Quality**: 195+ tests passing, zero clippy warnings
- ✅ **Observability**: Complete tracing and metrics
- ✅ **Performance**: Sub-10ms queries, 10K+ docs/sec throughput

## Key Insights

### Risk Reduction Works
The 6-stage methodology really paid off. By reducing risk from ~22 points to ~3 points, we achieved 99% reliability. The stages build on each other perfectly.

### Component Library Pattern
Stage 6's component library is brilliant. Every component gets tracing, validation, caching, and retry logic automatically. No need to remember to add these manually.

### Real-World Validation Needed
Now we need to validate with actual use cases. The examples we're building today will show if the abstractions work for real problems.

## Next Steps
1. Complete the examples for user validation
2. MCP server integration for LLM workflows
3. Performance testing at scale
4. Community feedback and iteration

## Lessons Learned
- Systematic risk reduction beats ad-hoc development
- Observability from day one is crucial
- Component composition scales better than inheritance
- Tests are documentation that never lies
"#.to_string(),
            vec!["journal".to_string(), "reflection".to_string(), "progress".to_string(), "kotadb".to_string()],
        ),

        // Meeting notes
        (
            "/meetings/2025-08-07-architecture-review.md".to_string(),
            "Architecture Review Meeting".to_string(),
            r#"# Architecture Review Meeting - August 7, 2025

**Attendees**: Development team, stakeholders
**Duration**: 2 hours
**Status**: KotaDB production readiness assessment

## Key Decisions

### ✅ Production Readiness Confirmed
- All 195+ tests passing
- Zero clippy warnings with strict linting
- Performance targets met (sub-10ms queries)
- Complete observability implementation

### 📋 Next Priority: Examples & Validation
- Create real-world usage examples
- Validate with actual use cases
- Performance testing under realistic loads
- User experience validation

### 🚀 MCP Integration Planning
- Model Context Protocol server implementation
- Natural language query interface
- Integration with LLM workflows
- Real-time collaboration features

## Technical Highlights

### Index Performance
- B+ tree: O(log n) guaranteed
- Trigram search: Sub-10ms full-text queries
- Intelligent query routing between indices
- Perfect tree balance maintained

### Quality Metrics
- 18 comprehensive test suites
- Chaos and adversarial testing
- Property-based testing for algorithms
- Integration tests for end-to-end workflows

### Architecture Strengths
- Component library ensures consistency
- Multiple index types for different query patterns
- Human-readable storage (git compatible)
- Comprehensive observability

## Action Items
- [ ] Complete examples directory (high priority)
- [ ] MCP server implementation
- [ ] Scale testing with 10K+ documents
- [ ] Community feedback collection
- [ ] Documentation improvements

## Risk Assessment
Current risk level: **Very Low** (3 points remaining)
- All major technical risks mitigated
- Production infrastructure complete
- Quality gates in place
"#.to_string(),
            vec!["meeting".to_string(), "architecture".to_string(), "review".to_string(), "decisions".to_string()],
        ),

        // Technical documentation
        (
            "/technical/performance-optimization.md".to_string(),
            "Performance Optimization Techniques".to_string(),
            r#"# Performance Optimization Techniques

## Database Performance

### Indexing Strategies
- **Covering Indices**: Include all query columns
- **Partial Indices**: Index subset of rows
- **Composite Indices**: Multi-column optimization
- **Function-Based Indices**: Index computed values

### Query Optimization
- Use EXPLAIN to understand query plans
- Avoid SELECT * in production queries
- Use appropriate WHERE clause ordering
- Consider query result caching

### Storage Optimization
- **Partitioning**: Split large tables
- **Compression**: Reduce I/O overhead
- **Memory Mapping**: Efficient file access
- **Write-Ahead Logging**: Crash recovery

## Application Performance

### Memory Management
- Pool expensive resources
- Use appropriate data structures
- Monitor garbage collection patterns
- Implement backpressure mechanisms

### Concurrency
- Lock-free data structures where possible
- Use async/await for I/O-bound operations
- Consider work-stealing thread pools
- Profile contention points

## KotaDB Optimizations
- Memory-mapped file access
- Intelligent index selection
- Batch operations for bulk work
- Component composition for efficiency
- Zero-copy operations where possible

## Monitoring
- Track key performance indicators
- Set up alerting for performance degradation
- Use distributed tracing for complex workflows
- Profile production workloads regularly
"#.to_string(),
            vec!["performance".to_string(), "optimization".to_string(), "databases".to_string(), "technical".to_string()],
        ),
    ]
}

/// Demonstrate various search and query capabilities
async fn demonstrate_search_capabilities(
    storage: &impl Storage,
    _primary_index: &impl Index,
    search_index: &impl Index,
) -> Result<()> {
    println!("🔍 Demonstrating Search Capabilities");
    println!("===================================\n");

    // 1. Full-text search (trigram index)
    println!("1. 🔎 Full-text search for 'ownership':");
    let query = Query::new(Some("ownership".to_string()), None, None, 10)?;
    let results = search_index.search(&query).await?;

    // Get actual documents from storage
    for doc_id in results.iter().take(3) {
        if let Some(doc) = storage.get(doc_id).await? {
            println!("   📄 {} - {}", doc.title, doc.path.as_str());
        }
    }
    println!("   Found {} total matches\n", results.len());

    // 2. Tag-based filtering
    println!("2. 🏷️  Documents tagged with 'rust':");
    let all_docs = storage.list_all().await?;
    let rust_docs: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "rust"))
        .collect();
    for doc in rust_docs.iter().take(3) {
        println!("   📄 {} - {}", doc.title, doc.path.as_str());
    }
    println!("   Found {} rust-related documents\n", rust_docs.len());

    // 3. Temporal queries (recent documents)
    println!("3. ⏰ Recent documents (by creation time):");
    let mut recent_docs = all_docs.clone();
    recent_docs.sort_by_key(|doc| std::cmp::Reverse(doc.created_at));
    for doc in recent_docs.iter().take(3) {
        println!(
            "   📄 {} - {} ({})",
            doc.title,
            doc.path.as_str(),
            doc.created_at.format("%Y-%m-%d")
        );
    }
    println!();

    // 4. Search for different terms
    println!("4. 🔍 Search for 'database' content:");
    let database_query = Query::new(Some("database".to_string()), None, None, 5)?;
    let database_results = search_index.search(&database_query).await?;
    for doc_id in database_results.iter().take(3) {
        if let Some(doc) = storage.get(doc_id).await? {
            println!("   📄 {} - {}", doc.title, doc.path.as_str());
        }
    }
    println!(
        "   Found {} database-related documents\n",
        database_results.len()
    );

    Ok(())
}

/// Demonstrate performance characteristics with bulk operations
async fn demonstrate_performance(
    storage: &mut impl Storage,
    primary_index: &mut impl Index,
    search_index: &mut impl Index,
) -> Result<()> {
    println!("⚡ Performance Demonstration");
    println!("===========================\n");

    // Bulk insert performance
    println!("📊 Testing bulk insert performance...");
    let start = std::time::Instant::now();

    for i in 0..100 {
        let path = format!("/generated/doc-{i:03}.md");
        let doc = DocumentBuilder::new()
            .path(&path)?
            .title(format!("Generated Document {i}"))?
            .content(format!("This is generated content for document {i}. It contains various keywords for testing search performance including: database, search, performance, indexing, rust, programming, and technical documentation.").as_bytes())
            .tag("generated")?
            .tag("performance")?
            .tag("test")?
            .build()?;

        storage.insert(doc.clone()).await?;
        primary_index
            .insert(doc.id, ValidatedPath::new(&path)?)
            .await?;
        search_index
            .insert(doc.id, ValidatedPath::new(&path)?)
            .await?;
    }

    let bulk_duration = start.elapsed();
    println!(
        "   ✅ Inserted 100 documents in {:.2}ms",
        bulk_duration.as_secs_f64() * 1000.0
    );
    println!(
        "   📈 Throughput: {:.0} docs/sec",
        100.0 / bulk_duration.as_secs_f64()
    );
    println!();

    // Search performance
    println!("🔍 Testing search performance...");
    let search_terms = vec!["database", "performance", "rust", "technical", "indexing"];

    for term in search_terms {
        let start = std::time::Instant::now();
        let query = Query::new(Some(term.to_string()), None, None, 100)?;
        let results = search_index.search(&query).await?;
        let search_duration = start.elapsed();

        println!(
            "   🔎 '{}': {} results in {:.2}ms",
            term,
            results.len(),
            search_duration.as_secs_f64() * 1000.0
        );
    }
    println!();

    // Storage statistics
    let all_docs = storage.list_all().await?;
    let total_size: usize = all_docs.iter().map(|doc| doc.content.len()).sum();

    println!("📈 Storage Statistics:");
    println!("   📄 Total documents: {}", all_docs.len());
    println!(
        "   💾 Total content size: {:.2} KB",
        total_size as f64 / 1024.0
    );
    println!(
        "   📊 Average document size: {:.0} bytes",
        total_size as f64 / all_docs.len() as f64
    );

    Ok(())
}
