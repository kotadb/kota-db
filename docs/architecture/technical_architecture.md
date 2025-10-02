---
title: "KOTA Custom Database Technical Architecture"
tags: [database, architecture, technical-design]
related: ["IMPLEMENTATION_PLAN.md"]
key_concepts: [storage-engine, query-engine, indexing, compression]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KOTA Custom Database Technical Architecture

## System Overview

The KOTA Database (KotaDB) is a codebase intelligence platform designed to transform source code into a queryable knowledge graph. It combines symbol extraction, dependency analysis, and impact assessment with high-performance search capabilities, enabling developers and AI systems to understand code relationships at scale.

### Design Philosophy

1. **Code as a Knowledge Graph**: Symbols, dependencies, and relationships are first-class entities
2. **Repository-First Storage**: Versioned file snapshots feed symbol and relationship stores
3. **Lightning-Fast Search**: <3ms trigram search with 210x performance improvement
4. **Symbol-Aware Analysis**: Automatic extraction of functions, classes, traits, and their relationships
5. **Impact Understanding**: Know what breaks when code changes

## Core Architecture Components

### 1. Storage Layer

```
┌─────────────────────────────────────────────────────────────┐
│                        Storage Engine                        │
├─────────────────┬────────────────┬──────────────────────────┤
│   Page Manager  │  Write-Ahead   │   Memory-Mapped Files    │
│   (4KB pages)   │   Log (WAL)    │   (hot data cache)       │
├─────────────────┴────────────────┴──────────────────────────┤
│                    Compression Layer                         │
│        (Code-aware chunking + symbol dictionaries)           │
├──────────────────────────────────────────────────────────────┤
│                   Filesystem Interface                       │
│         (Indexed source files + Symbol binaries)             │
└──────────────────────────────────────────────────────────────┘
```

#### Page Manager
- **Fixed 4KB pages**: Matches OS page size for optimal I/O
- **Copy-on-Write**: Enables versioning without duplication
- **Free space management**: Bitmap allocation for efficiency
- **Checksums**: CRC32C for corruption detection

#### Write-Ahead Log (WAL)
- **Append-only design**: Sequential writes for performance
- **Group commit**: Batch multiple transactions
- **Checkpoint mechanism**: Periodic state snapshots
- **Recovery protocol**: Fast startup after crashes

- **Domain-specific dictionaries**: 
  - Source-code tokens (keywords, identifiers)
  - Tree-sitter symbol metadata
  - Git diff headers and repository metadata
- **Adaptive compression levels**:
  - Hot data: LZ4 (fast)
  - Warm data: ZSTD level 3
  - Cold data: ZSTD level 19
- **Estimated ratios**: 3-5x for typical KOTA content

### 2. Index Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              Codebase Intelligence Manager                   │
├──────────────┬───────────────┬───────────────┬──────────────┤
│   Symbol     │  Dependency   │    Impact     │  Semantic    │
│  Extraction  │    Graph      │   Analysis    │    (HNSW)    │
│      ✅      │       ✅      │      ✅       │      ✅      │
├──────────────┴───────────────┴───────────────┴──────────────┤
│                      Index Manager                           │
├──────────────┬───────────────┬───────────────┬──────────────┤
│   Primary    │   Full-Text   │     Graph     │   Wildcard   │
│   (B+ Tree)  │   (Trigram)   │  (Relations)  │   Patterns   │
│      ✅      │   ✅ (<3ms)   │      ✅       │      ✅      │
├──────────────┼───────────────┼───────────────┼──────────────┤
│   Temporal   │      Tag      │   Metadata    │   Spatial    │
│   (Planned)  │   (Basic)     │    (Hash)     │  (Planned)   │
│      🚧      │       ✅      │      ✅       │      🚧      │
└──────────────┴───────────────┴───────────────┴──────────────┘
```

- **Key**: Repository path (for filesystem compatibility)
- **Value**: File node ID + metadata
- **Features**: Range queries, ordered traversal
- **Performance**: O(log n) lookups

#### Full-Text Index (Trigram)
- **Trigram extraction**: "hello" → ["hel", "ell", "llo"]
- **Inverted index**: Trigram → Document IDs (RoaringBitmap)
- **Fuzzy matching**: Levenshtein distance calculation
- **Position tracking**: For snippet extraction

- **Forward edges**: Symbol/File → Dependent symbols or files
- **Backward edges**: Symbol/File ← Referencing callers or imports
- **Edge metadata**: Relationship type, strength, timestamp
- **Traversal optimization**: Bloom filters for existence checks

#### Semantic Index (HNSW)
- **Hierarchical Navigable Small World**: Fast approximate search
- **Vector dimensions**: 384 (all-MiniLM-L6-v2) or 1536 (OpenAI)
- **Distance metrics**: Cosine similarity, L2 distance
- **Performance**: Sub-linear search time

### 3. Query Engine

```
┌─────────────────────────────────────────────────────────────┐
│                    Query Interface                           │
│                  (Natural Language)                          │
├─────────────────────────────────────────────────────────────┤
│                    Query Parser                              │
│              (KQL - KOTA Query Language)                     │
├─────────────────────────────────────────────────────────────┤
│                   Query Planner                              │
│            (Cost-based optimization)                         │
├─────────────────────────────────────────────────────────────┤
│                  Query Executor                              │
│              (Parallel, streaming)                           │
├─────────────────────────────────────────────────────────────┤
│                  Result Processor                            │
│           (Ranking, aggregation, projection)                 │
└─────────────────────────────────────────────────────────────┘
```

#### KOTA Query Language (KQL)
```
// Natural language queries
"meetings about rust programming last week"
"documents similar to distributed cognition"
"show my productivity patterns"

// Structured queries
{
  "type": "semantic",
  "query": "consciousness evolution",
  "filters": {
    "created": { "$gte": "2025-01-01" },
    "tags": { "$contains": "philosophy" }
  },
  "limit": 10
}

// Graph traversal
{
  "type": "graph",
  "start": "projects/kota-ai/README.md",
  "depth": 3,
  "direction": "outbound",
  "edge_filter": { "type": "implements" }
}
```

#### Query Planning
1. **Parse**: Convert natural language to AST
2. **Analyze**: Determine required indices
3. **Optimize**: Choose best execution path
4. **Estimate**: Predict cost and result size

#### Execution Strategy
- **Index selection**: Use most selective index first
- **Parallel execution**: Split independent subqueries
- **Pipeline processing**: Stream results as available
- **Memory budget**: Spill to disk if needed

### 4. Transaction Management

```
┌─────────────────────────────────────────────────────────────┐
│                 Transaction Manager                          │
├─────────────────┬────────────────┬──────────────────────────┤
│      MVCC       │   Lock Manager  │   Deadlock Detector     │
│  (Multi-Version)│  (Row-level)    │   (Wait-for graph)      │
└─────────────────┴────────────────┴──────────────────────────┘
```

#### MVCC Implementation
- **Version chains**: Each document has version history
- **Snapshot isolation**: Consistent reads
- **Garbage collection**: Clean old versions
- **Read-write separation**: No read locks needed

### 5. Repository Ingestion Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│              Repository Intake Stages                        │
├──────────────┬────────────────┬─────────────────────────────┤
│   Git Clone  │  File Snapshot │   Symbol Extraction         │
│    & Fetch   │   & Chunking   │   (tree-sitter)             │
├──────────────┼────────────────┼─────────────────────────────┤
│  Diff Queue  │  Dependency    │   Analysis Jobs             │
│   Builder    │    Graph       │   & Persistence             │
└──────────────┴────────────────┴─────────────────────────────┘
```

#### Pipeline Benefits
- **Fresh context**: Repo events trigger targeted re-indexing
- **Selective updates**: Diffs determine which files and symbols to refresh
- **Graph fidelity**: Dependency edges stay aligned with the latest commit
- **Job visibility**: Supabase-backed telemetry tracks ingestion progress

## Data Model

### File Snapshot Structure
```rust
pub struct FileSnapshot {
    // Identity
    id: FileId,              // Stable UUID per repo + path
    repository_id: RepoId,
    path: NormalizedPath,

    // Content metadata
    content_hash: ContentHash,
    size_bytes: u64,
    language: Option<String>,

    // Symbols & relationships
    symbols: Vec<SymbolId>,
    imports: Vec<SymbolId>,
    outgoing_edges: Vec<DependencyEdge>,

    // Audit trail
    last_indexed_commit: Option<GitOid>,
    last_indexed_at: Timestamp,
}
```

### Index Entry Structure
```rust
pub struct IndexEntry {
    doc_id: DocumentId,
    score: f32,           // Relevance score
    positions: Vec<u32>,  // Word positions for highlighting
    metadata: Metadata,   // Quick-access fields
}
```

## Performance Characteristics

### Time Complexity
| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| Insert | O(log n) | <1ms |
| Update | O(log n) | <1ms |
| Delete | O(log n) | <1ms |
| Lookup by path | O(log n) | <0.1ms |
| Full-text search | O(k) | <10ms |
| Graph traversal | O(V + E) | <50ms |
| Semantic search (retired) | N/A | N/A |

### Space Complexity
| Component | Memory Usage | Disk Usage |
|-----------|--------------|------------|
| Document | ~8KB avg | ~3KB compressed |
| Indices | ~500B/doc | ~200B/doc |
| WAL | 10MB active | Configurable |
| Page cache | 100MB default | N/A |

### Throughput Targets
- **Writes**: 10,000+ documents/second
- **Reads**: 100,000+ queries/second
- **Mixed**: 50% read, 50% write maintaining targets

## Security Architecture

### Encryption
- **At rest**: AES-256-GCM for sensitive documents
- **In transit**: TLS 1.3 for network operations
- **Key management**: OS keychain integration

### Access Control
- **Document-level**: Read/write permissions
- **Field-level**: Redaction for sensitive fields
- **Audit logging**: All operations tracked

### Privacy Features
- **PII detection**: Automatic flagging
- **Retention policies**: Automatic expiry
- **Right to forget**: Complete removal

## Extensibility Points

### Plugin System
```rust
pub trait KotaPlugin {
    fn on_insert(&mut self, doc: &Document) -> Result<()>;
    fn on_query(&mut self, query: &Query) -> Result<()>;
    fn on_index(&mut self, index: &Index) -> Result<()>;
}
```

### Custom Index Types
- **Bloom filter index**: For existence checks
- **Geospatial index**: For location data
- **Phonetic index**: For name matching
- **Custom embeddings**: Domain-specific vectors

### Query Extensions
- **Custom functions**: User-defined computations
- **External data sources**: Federation support
- **Streaming queries**: Real-time updates

## Operational Considerations

### Monitoring
- **Prometheus metrics**: Performance and health
- **OpenTelemetry traces**: Distributed tracing
- **Custom dashboards**: Grafana integration

### Maintenance
- **Online defragmentation**: No downtime
- **Index rebuilding**: Background operation
- **Backup coordination**: Consistent snapshots

### Disaster Recovery
- **Point-in-time recovery**: Any timestamp
- **Geo-replication**: Optional for critical data
- **Incremental backups**: Efficient storage

## Future Optimizations

### Hardware Acceleration
- **SIMD instructions**: Batch operations
- **GPU indexing**: Parallel vector search
- **Persistent memory**: Intel Optane support

### Advanced Features
- **Learned indices**: ML-based optimization
- **Adaptive compression**: Content-aware
- **Predictive caching**: Access pattern learning

### Cognitive Enhancements
- **Thought chains**: Native support
- **Memory consolidation**: Sleep-like processing
- **Attention mechanisms**: Priority-based indexing

## Conclusion

This architecture provides a solid foundation for KOTA's evolution from a tool collection to a genuine cognitive partner. The custom database design specifically addresses the unique requirements of human-AI distributed cognition while maintaining practical considerations like Git compatibility and human readability.

The modular design allows for incremental implementation and testing, reducing risk while enabling rapid innovation in areas like consciousness integration and semantic understanding.
