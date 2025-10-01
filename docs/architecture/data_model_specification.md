---
title: "KOTA Database Data Model Specification"
tags: [database, data-model, specification]
related: ["IMPLEMENTATION_PLAN.md", "TECHNICAL_ARCHITECTURE.md"]
key_concepts: [document-structure, indexing, compression, relationships]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KOTA Database Data Model Specification

## Overview

This document specifies the complete data model for KotaDB, including storage formats, index structures, compression schemes, and query representations. The model is designed to efficiently support KOTA's unique requirements for distributed cognition.

## 1. Core Data Types

### 1.1 Primitive Types

```rust
// Document identifier - 128-bit UUID for global uniqueness
pub type DocumentId = uuid::Uuid;

// Timestamp with nanosecond precision
pub type Timestamp = i64;  // Unix timestamp in nanoseconds

// Version counter for MVCC
pub type Version = u64;

// Page identifier for storage engine
pub type PageId = u32;

// Compressed path representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompressedPath {
    // Common prefix ID (e.g., "/Users/jaymin/kota_md/" = 0)
    prefix_id: u16,
    // Remaining path components
    components: Vec<SmallString>,
}

// Small string optimization for paths
pub type SmallString = smallstr::SmallString<[u8; 23]>;

// Vector for embeddings
pub type Vector = Vec<f32>;

// Tag representation with interning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TagId(u32);
```

### 1.2 Frontmatter Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    // Core metadata
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub key_concepts: Vec<String>,
    pub personal_contexts: Vec<String>,
    
    // Timestamps
    pub created: NaiveDate,
    pub updated: NaiveDate,
    
    // Optional fields
    pub date: Option<NaiveDate>,
    pub participants: Option<Vec<String>>,
    pub duration: Option<String>,
    pub meeting_type: Option<String>,
    
    // Custom fields stored as JSON
    pub custom: serde_json::Map<String, serde_json::Value>,
}

// Compressed representation for storage
#[repr(C)]
pub struct CompressedFrontmatter {
    // Offsets into data buffer
    title_offset: u16,
    title_len: u16,
    
    // Tag bitmap for common tags
    common_tags: u64,  // Bit flags for 64 most common tags
    custom_tags_offset: u16,
    custom_tags_count: u8,
    
    // Related documents as ID list
    related_offset: u16,
    related_count: u8,
    
    // Dates as days since epoch
    created_days: u16,
    updated_days: u16,
    
    // Flags for optional fields
    flags: FrontmatterFlags,
    
    // Variable-length data follows
    data: [u8],
}

bitflags! {
    pub struct FrontmatterFlags: u8 {
        const HAS_DATE = 0b00000001;
        const HAS_PARTICIPANTS = 0b00000010;
        const HAS_DURATION = 0b00000100;
        const HAS_MEETING_TYPE = 0b00001000;
        const HAS_CUSTOM = 0b00010000;
    }
}
```

### 1.3 Document Storage Format

```rust
// On-disk document representation
#[repr(C)]
pub struct StoredDocument {
    // Fixed header (64 bytes)
    header: DocumentHeader,
    
    // Variable-length sections
    frontmatter: CompressedFrontmatter,
    content: CompressedContent,
    metadata: DocumentMetadata,
}

#[repr(C, packed)]
pub struct DocumentHeader {
    // Magic number: "KOTA" in ASCII
    magic: [u8; 4],
    
    // Format version for upgrades
    version: u16,
    
    // Document ID
    id: [u8; 16],  // UUID bytes
    
    // Checksums
    header_crc: u32,
    content_crc: u32,
    
    // Compression info
    compression_type: CompressionType,
    uncompressed_size: u32,
    compressed_size: u32,
    
    // Section offsets
    frontmatter_offset: u32,
    content_offset: u32,
    metadata_offset: u32,
    
    // Timestamps (seconds since epoch)
    created: u32,
    updated: u32,
    accessed: u32,
    
    // Version for MVCC
    version: u64,
    
    // Reserved for future use
    reserved: [u8; 8],
}

#[repr(u8)]
pub enum CompressionType {
    None = 0,
    Lz4 = 1,
    Zstd = 2,
    ZstdDict = 3,  // With domain-specific dictionary
}
```

## 2. Index Structures

### 2.1 Primary Index (B+ Tree)

```rust
pub struct BPlusTreeIndex {
    root: PageId,
    height: u16,
    key_count: u64,
    
    // Index configuration
    order: u16,  // Max keys per node (typically 100-200)
    key_size: u16,
    value_size: u16,
}

// Internal node structure
#[repr(C)]
pub struct InternalNode {
    is_leaf: bool,
    key_count: u16,
    keys: [IndexKey; MAX_KEYS],
    children: [PageId; MAX_KEYS + 1],
}

// Leaf node structure with next pointer for scanning
#[repr(C)]
pub struct LeafNode {
    is_leaf: bool,
    key_count: u16,
    next_leaf: Option<PageId>,
    entries: [IndexEntry; MAX_KEYS],
}

pub struct IndexEntry {
    key: CompressedPath,
    doc_id: DocumentId,
    metadata: QuickMetadata,  // For covering index queries
}

// Minimal metadata to avoid document fetch
#[repr(C, packed)]
pub struct QuickMetadata {
    title_hash: u32,
    updated: u32,
    word_count: u16,
    flags: u8,
}
```

### 2.2 Full-Text Index (Trigram Inverted Index)

```rust
pub struct TrigramIndex {
    // Trigram to document mapping
    trigrams: HashMap<Trigram, PostingList>,
    
    // Document positions for snippet extraction
    positions: HashMap<DocumentId, DocumentPositions>,
    
    // Statistics for relevance scoring
    doc_count: u64,
    total_trigrams: u64,
    avg_doc_length: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Trigram([u8; 3]);

// Compressed posting list using Roaring Bitmaps
pub struct PostingList {
    // Document IDs containing this trigram
    docs: RoaringBitmap,
    
    // Term frequency for BM25 scoring
    frequencies: Vec<(DocumentId, u16)>,
}

pub struct DocumentPositions {
    // Trigram positions within document
    positions: HashMap<Trigram, Vec<u32>>,
    
    // Word boundaries for highlighting
    word_boundaries: Vec<(u32, u32)>,
}
```

### 2.3 Graph Index (Adjacency List)

```rust
pub struct GraphIndex {
    // Forward edges: document -> related
    forward_edges: HashMap<DocumentId, EdgeList>,
    
    // Backward edges: document <- referencing
    backward_edges: HashMap<DocumentId, EdgeList>,
    
    // Edge metadata storage
    edge_data: HashMap<EdgeId, EdgeMetadata>,
    
    // Graph statistics
    node_count: u64,
    edge_count: u64,
    avg_degree: f32,
}

pub struct EdgeList {
    edges: Vec<Edge>,
    // Bloom filter for O(1) existence checks
    bloom: BloomFilter,
}

#[derive(Debug, Clone)]
pub struct Edge {
    target: DocumentId,
    edge_id: EdgeId,
    weight: f32,
}

#[derive(Debug, Clone)]
pub struct EdgeMetadata {
    edge_type: EdgeType,
    created: Timestamp,
    attributes: HashMap<String, Value>,
}

#[repr(u8)]
pub enum EdgeType {
    Related = 0,
    References = 1,
    ChildOf = 2,
    TaggedWith = 3,
    SimilarTo = 4,
    Custom = 255,
}
```

### 2.4 Semantic Index (HNSW)

```rust
pub struct HnswIndex {
    // Hierarchical layers
    layers: Vec<Layer>,
    
    // Entry point for search
    entry_point: Option<DocumentId>,
    
    // Vector storage
    vectors: HashMap<DocumentId, Vector>,
    
    // Index parameters
    m: usize,  // Number of connections
    ef_construction: usize,  // Size of dynamic candidate list
    max_m: usize,  // Max connections for layer 0
    seed: u64,  // Random seed for level assignment
}

pub struct Layer {
    level: u8,
    // Adjacency list for this layer
    connections: HashMap<DocumentId, Vec<DocumentId>>,
}

// Distance metrics
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}
```

### 2.5 Temporal Index (Time-Series Optimized)

```rust
pub struct TemporalIndex {
    // Time-partitioned B+ trees
    partitions: BTreeMap<TimePartition, PartitionIndex>,
    
    // Hot partition cache
    hot_partition: Arc<RwLock<PartitionIndex>>,
    
    // Aggregation cache
    aggregations: HashMap<AggregationKey, AggregationResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimePartition {
    year: u16,
    month: u8,
    day: u8,
}

pub struct PartitionIndex {
    // Hour -> Minute -> Documents
    hours: [Option<HourIndex>; 24],
}

pub struct HourIndex {
    minutes: BTreeMap<u8, Vec<DocumentId>>,
}
```

## 3. Query Representation

### 3.1 Query AST

```rust
#[derive(Debug, Clone)]
pub enum Query {
    // Text search
    Text {
        query: String,
        fields: Vec<Field>,
        fuzzy: bool,
        boost: f32,
    },
    
    // Relationship traversal
    Graph {
        start: QueryNode,
        pattern: GraphPattern,
        depth: Depth,
    },
    
    // Temporal queries
    Temporal {
        range: TimeRange,
        granularity: TimeGranularity,
        aggregation: Option<Aggregation>,
    },
    
    // Semantic similarity
    Semantic {
        vector: SemanticQuery,
        threshold: f32,
        limit: usize,
    },
    
    // Compound queries
    And(Vec<Query>),
    Or(Vec<Query>),
    Not(Box<Query>),
    
    // Filters
    Filter {
        query: Box<Query>,
        filter: FilterExpression,
    },
}

#[derive(Debug, Clone)]
pub enum QueryNode {
    Id(DocumentId),
    Path(String),
    Pattern(String),  // Glob pattern
}

#[derive(Debug, Clone)]
pub struct GraphPattern {
    edge_types: Vec<EdgeType>,
    direction: Direction,
    filters: Vec<EdgeFilter>,
}

#[derive(Debug, Clone)]
pub enum SemanticQuery {
    Vector(Vector),
    Document(DocumentId),
    Text(String),  // Will be embedded
}
```

### 3.2 Query Plan

```rust
#[derive(Debug, Clone)]
pub struct QueryPlan {
    steps: Vec<PlanStep>,
    estimated_cost: f64,
    estimated_rows: usize,
    required_indices: Vec<IndexType>,
}

#[derive(Debug, Clone)]
pub enum PlanStep {
    // Index scans
    IndexScan {
        index: IndexType,
        bounds: ScanBounds,
        projection: Vec<Field>,
    },
    
    // Sequential scan with filter
    SeqScan {
        filter: FilterExpression,
        projection: Vec<Field>,
    },
    
    // Join operations
    NestedLoopJoin {
        outer: Box<PlanStep>,
        inner: Box<PlanStep>,
        condition: JoinCondition,
    },
    
    HashJoin {
        build: Box<PlanStep>,
        probe: Box<PlanStep>,
        keys: Vec<Field>,
    },
    
    // Graph operations
    GraphTraversal {
        start: Box<PlanStep>,
        pattern: GraphPattern,
        algorithm: TraversalAlgorithm,
    },
    
    // Aggregations
    Aggregate {
        input: Box<PlanStep>,
        groups: Vec<Field>,
        aggregates: Vec<AggregateFunction>,
    },
    
    // Sorting and limiting
    Sort {
        input: Box<PlanStep>,
        keys: Vec<SortKey>,
    },
    
    Limit {
        input: Box<PlanStep>,
        count: usize,
        offset: usize,
    },
}
```

## 4. Compression Schemes

### 4.1 Dictionary Compression

```rust
pub struct CompressionDictionary {
    // Domain-specific dictionaries
    markdown_dict: ZstdDict,
    frontmatter_dict: ZstdDict,
    
    // Common strings table
    string_table: StringTable,
    
    // Tag vocabulary
    tag_vocab: HashMap<String, TagId>,
    tag_lookup: Vec<String>,
}

pub struct StringTable {
    // Interned strings with reference counting
    strings: HashMap<String, StringId>,
    lookup: Vec<Arc<String>>,
    refcounts: Vec<AtomicU32>,
}

// Compressed string reference
#[derive(Debug, Clone, Copy)]
pub struct StringId(u32);
```

### 4.2 Columnar Storage for Analytics

```rust
pub struct ColumnarBatch {
    // Schema definition
    schema: Schema,
    
    // Column data
    columns: Vec<Column>,
    
    // Row count
    num_rows: usize,
}

pub enum Column {
    // Fixed-width columns
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    
    // Variable-width columns
    String(StringColumn),
    Binary(BinaryColumn),
    
    // Nested types
    List(ListColumn),
    Struct(StructColumn),
}

pub struct StringColumn {
    // Offsets into data buffer
    offsets: Vec<u32>,
    // Concatenated string data
    data: Vec<u8>,
    // Optional dictionary encoding
    dictionary: Option<Vec<String>>,
}
```

## 5. Transaction Log Format

### 5.1 WAL Entry Structure

```rust
#[repr(C)]
pub struct WalEntry {
    // Entry header (16 bytes)
    header: WalHeader,
    
    // Entry payload
    payload: WalPayload,
    
    // CRC32 checksum
    checksum: u32,
}

#[repr(C, packed)]
pub struct WalHeader {
    // Log sequence number
    lsn: u64,
    
    // Transaction ID
    tx_id: u64,
    
    // Entry type
    entry_type: WalEntryType,
    
    // Payload size
    payload_size: u32,
    
    // Timestamp
    timestamp: u64,
}

#[repr(u8)]
pub enum WalEntryType {
    Begin = 1,
    Commit = 2,
    Abort = 3,
    Insert = 4,
    Update = 5,
    Delete = 6,
    Checkpoint = 7,
}

pub enum WalPayload {
    Begin { tx_id: u64 },
    Commit { tx_id: u64 },
    Abort { tx_id: u64 },
    Insert { tx_id: u64, doc: Document },
    Update { tx_id: u64, id: DocumentId, delta: Delta },
    Delete { tx_id: u64, id: DocumentId },
    Checkpoint { snapshot: DatabaseSnapshot },
}
```

### 5.2 Delta Encoding for Updates

```rust
pub struct Delta {
    // Field-level changes
    changes: Vec<FieldChange>,
    
    // Old version for rollback
    old_version: Version,
    
    // New version after update
    new_version: Version,
}

pub enum FieldChange {
    SetField { path: FieldPath, value: Value },
    RemoveField { path: FieldPath },
    AppendToArray { path: FieldPath, values: Vec<Value> },
    RemoveFromArray { path: FieldPath, indices: Vec<usize> },
}

pub struct FieldPath {
    segments: Vec<PathSegment>,
}

pub enum PathSegment {
    Field(String),
    Index(usize),
}
```

## 6. Memory Layout

### 6.1 Page Layout

```rust
// 4KB page structure
#[repr(C, align(4096))]
pub struct Page {
    header: PageHeader,
    data: [u8; PAGE_SIZE - size_of::<PageHeader>()],
}

#[repr(C, packed)]
pub struct PageHeader {
    // Page metadata (64 bytes)
    page_id: PageId,
    page_type: PageType,
    lsn: u64,  // Last modification LSN
    checksum: u32,
    free_space: u16,
    item_count: u16,
    
    // Free space pointers
    free_space_start: u16,
    free_space_end: u16,
    
    // Reserved
    reserved: [u8; 32],
}

#[repr(u8)]
pub enum PageType {
    Data = 1,
    Index = 2,
    Overflow = 3,
    Free = 4,
}
```

### 6.2 Buffer Pool Structure

```rust
pub struct BufferPool {
    // Page frames in memory
    frames: Vec<Frame>,
    
    // Page table (page_id -> frame_id)
    page_table: HashMap<PageId, FrameId>,
    
    // Free frame list
    free_list: Vec<FrameId>,
    
    // LRU eviction policy
    lru: LruCache<FrameId, ()>,
    
    // Statistics
    stats: BufferPoolStats,
}

pub struct Frame {
    page: Page,
    dirty: AtomicBool,
    pin_count: AtomicU32,
    last_access: AtomicU64,
}
```

## 7. Configuration Schema

### 7.1 Database Configuration

```toml
[database]
# Storage configuration
data_dir = "~/.kota/db"
page_size = 4096
cache_size_mb = 100

# Compression settings
compression_level = 3
use_dictionaries = true
dictionary_sample_size = 100000

# Index configuration
[database.indices]
btree_order = 128
trigram_cache_size = 10000
hnsw_m = 16
hnsw_ef_construction = 200

# WAL settings
[database.wal]
segment_size_mb = 16
checkpoint_interval_sec = 300
compression = true

# Query engine
[database.query]
max_parallel_queries = 10
query_timeout_ms = 5000
cache_size_mb = 50
```

### 7.2 Runtime Statistics

```rust
#[derive(Debug, Default)]
pub struct DatabaseStats {
    // Storage stats
    pub total_pages: u64,
    pub used_pages: u64,
    pub free_pages: u64,
    
    // Index stats
    pub index_stats: HashMap<String, IndexStats>,
    
    // Query stats
    pub queries_executed: u64,
    pub avg_query_time_ms: f64,
    pub cache_hit_rate: f32,
    
    // Transaction stats
    pub transactions_committed: u64,
    pub transactions_aborted: u64,
    pub deadlocks_detected: u64,
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub entries: u64,
    pub size_bytes: u64,
    pub height: u32,
    pub lookups: u64,
    pub updates: u64,
    pub hit_rate: f32,
}
```

## Conclusion

This data model provides a comprehensive foundation for KotaDB, optimized for KOTA's specific use cases while maintaining flexibility for future enhancements. The design prioritizes:

1. **Efficiency**: Compressed storage, optimized indices
2. **Flexibility**: Extensible schema, custom fields
3. **Performance**: Memory-aware layouts, parallel processing
4. **Reliability**: ACID transactions, crash recovery
5. **Integration**: Native support for KOTA's cognitive features

The model can be implemented incrementally, starting with core storage and gradually adding advanced features like semantic search (planned for the cloud relaunch) and graph traversal.
