# Stage 6: Component Library Documentation

## Overview

Stage 6 of the KotaDB risk reduction methodology implements a **Component Library** that provides reusable, battle-tested components with validated inputs and automatic best practices. This stage achieves **-1.0 risk reduction points** by making it impossible to construct invalid states and automatically applying proven patterns.

## Architecture

The component library consists of three main categories:

```
Stage 6 Components
├── Validated Types (src/types.rs)
│   ├── Path validation and safety
│   ├── Document lifecycle state machines  
│   ├── Temporal constraints enforcement
│   └── Bounded numeric types
├── Builder Patterns (src/builders.rs)
│   ├── Fluent API construction
│   ├── Sensible defaults
│   ├── Validation during building
│   └── Ergonomic error handling
└── Wrapper Components (src/wrappers.rs)
    ├── Automatic tracing and metrics
    ├── Transparent caching layers
    ├── Retry logic with backoff
    └── RAII transaction safety
```

## Validated Types (src/types.rs)

### Core Principle: Invalid States Unrepresentable

All validated types follow the principle that invalid data cannot be constructed. Instead of runtime checks scattered throughout the codebase, invariants are enforced at the type level.

#### Path Safety: `ValidatedPath`

```rust
pub struct ValidatedPath {
    inner: PathBuf,
}

impl ValidatedPath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        // Enforces:
        // - Non-empty paths
        // - No directory traversal (..)
        // - No null bytes
        // - Valid UTF-8
        // - Not Windows reserved names
    }
}
```

**Why this matters**: Path traversal vulnerabilities are eliminated at compile time. No need to remember to validate paths throughout the codebase.

#### Document Identity: `ValidatedDocumentId`

```rust
pub struct ValidatedDocumentId {
    inner: Uuid,
}

impl ValidatedDocumentId {
    pub fn from_uuid(uuid: Uuid) -> Result<Self> {
        ensure!(!uuid.is_nil(), "Document ID cannot be nil");
        Ok(Self { inner: uuid })
    }
}
```

**Why this matters**: Nil UUIDs are a common source of bugs. This type guarantees every document has a valid identifier.

#### Document Lifecycle: `TypedDocument<State>`

```rust
pub struct TypedDocument<S: DocumentState> {
    pub id: ValidatedDocumentId,
    pub path: ValidatedPath,
    pub timestamps: TimestampPair,
    // ... other fields
    _state: PhantomData<S>,
}

// State machine transitions
impl TypedDocument<Draft> {
    pub fn into_persisted(self) -> TypedDocument<Persisted> { ... }
}

impl TypedDocument<Persisted> {
    pub fn into_modified(self) -> TypedDocument<Modified> { ... }
}
```

**Why this matters**: Documents can only transition through valid states. Attempting to modify a draft or persist a non-existent document becomes a compile error.

#### Temporal Constraints: `TimestampPair`

```rust
pub struct TimestampPair {
    created: ValidatedTimestamp,
    updated: ValidatedTimestamp,
}

impl TimestampPair {
    pub fn new(created: ValidatedTimestamp, updated: ValidatedTimestamp) -> Result<Self> {
        ensure!(updated.as_secs() >= created.as_secs(), 
                "Updated timestamp must be >= created timestamp");
        Ok(Self { created, updated })
    }
}
```

**Why this matters**: Time paradoxes (documents updated before they were created) are impossible to represent.

## Builder Patterns (src/builders.rs)

### Core Principle: Ergonomic Construction with Validation

Builders provide fluent APIs that make it easy to construct complex objects while ensuring all required fields are provided and validation occurs at build time.

#### Document Construction: `DocumentBuilder`

```rust
let doc = DocumentBuilder::new()
    .path("/knowledge/rust-patterns.md")?
    .title("Rust Design Patterns")?
    .content(b"# Rust Patterns\n\nKey patterns...")
    .word_count(150)  // Optional - will be calculated if not provided
    .timestamps(1000, 2000)?  // Optional - will use current time if not provided
    .build()?;
```

**Features**:
- **Fluent API**: Method chaining for readability
- **Automatic Calculation**: Word count computed from content if not specified
- **Sensible Defaults**: Timestamps default to current time
- **Early Validation**: Errors caught at method call, not build time
- **Required Fields**: Build fails if path, title, or content missing

#### Query Construction: `QueryBuilder`

```rust
let query = QueryBuilder::new()
    .with_text("rust patterns")?
    .with_tag("programming")?
    .with_tag("design")?
    .with_date_range(start_time, end_time)?
    .with_limit(50)?
    .build()?;
```

**Features**:
- **Incremental Building**: Add constraints one at a time
- **Validation per Method**: Each method validates its input immediately
- **Flexible Composition**: Mix text, tags, date ranges, and limits
- **Default Limits**: Reasonable defaults prevent accidental large queries

## Wrapper Components (src/wrappers.rs)

### Core Principle: Automatic Best Practices

Wrappers implement cross-cutting concerns like tracing, caching, validation, and retry logic automatically. They can be composed together to create fully-featured implementations.

#### Automatic Tracing: `TracedStorage<S>`

```rust
pub struct TracedStorage<S: Storage> {
    inner: S,
    trace_id: Uuid,
    operation_count: Arc<Mutex<u64>>,
}
```

**Capabilities**:
- **Unique Trace IDs**: Every storage instance gets a UUID for correlation
- **Operation Logging**: All operations logged with context and timing
- **Metrics Collection**: Duration and success/failure metrics automatically recorded
- **Operation Counting**: Track how many operations performed

**Usage Pattern**:
```rust
let storage = MockStorage::new();
let traced = TracedStorage::new(storage);
// All operations now automatically traced and timed
```

#### Input/Output Validation: `ValidatedStorage<S>`

```rust
pub struct ValidatedStorage<S: Storage> {
    inner: S,
    existing_ids: Arc<RwLock<std::collections::HashSet<Uuid>>>,
}
```

**Capabilities**:
- **Precondition Validation**: All inputs validated before processing
- **Postcondition Validation**: All outputs validated before returning
- **Duplicate Prevention**: Tracks existing IDs to prevent duplicates
- **Update Validation**: Ensures updates are valid transitions

#### Automatic Retries: `RetryableStorage<S>`

```rust
pub struct RetryableStorage<S: Storage> {
    inner: S,
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
}
```

**Capabilities**:
- **Exponential Backoff**: Intelligent retry timing with jitter
- **Configurable Limits**: Set max retries and delay bounds
- **Transient Error Handling**: Retries on temporary failures only
- **Operation-Specific Logic**: Different retry behavior per operation type

#### LRU Caching: `CachedStorage<S>`

```rust
pub struct CachedStorage<S: Storage> {
    inner: S,
    cache: Arc<Mutex<LruCache<Uuid, Document>>>,
    cache_hits: Arc<Mutex<u64>>,
    cache_misses: Arc<Mutex<u64>>,
}
```

**Capabilities**:
- **LRU Eviction**: Intelligent cache management
- **Cache Statistics**: Track hit/miss ratios for optimization
- **Automatic Invalidation**: Updates and deletes invalidate cache entries
- **Configurable Size**: Set cache capacity based on memory constraints

#### Wrapper Composition

The real power comes from composing wrappers together:

```rust
pub type FullyWrappedStorage<S> = TracedStorage<ValidatedStorage<RetryableStorage<CachedStorage<S>>>>;

pub async fn create_wrapped_storage<S: Storage>(
    inner: S,
    cache_capacity: usize,
) -> FullyWrappedStorage<S> {
    let cached = CachedStorage::new(inner, cache_capacity);
    let retryable = RetryableStorage::new(cached);
    let validated = ValidatedStorage::new(retryable);
    let traced = TracedStorage::new(validated);
    traced
}
```

**Layer Composition**:
1. **Base Storage**: Your implementation
2. **Caching Layer**: Reduces I/O operations
3. **Retry Layer**: Handles transient failures
4. **Validation Layer**: Ensures data integrity
5. **Tracing Layer**: Provides observability

#### RAII Transaction Safety: `SafeTransaction`

```rust
pub struct SafeTransaction {
    inner: Transaction,
    committed: bool,
}

impl Drop for SafeTransaction {
    fn drop(&mut self) {
        if !self.committed {
            warn!("Transaction {} dropped without commit - automatic rollback", 
                  self.inner.id);
            // Triggers rollback
        }
    }
}
```

**Capabilities**:
- **Automatic Rollback**: Uncommitted transactions roll back on drop
- **Explicit Commit**: Must explicitly commit to persist changes
- **RAII Safety**: Impossible to forget transaction cleanup

## Testing Strategy

### Test Coverage by Component

#### Validated Types Tests (`tests/validated_types_tests.rs`)
- **Edge Case Validation**: Empty strings, null bytes, reserved names
- **Boundary Testing**: Maximum lengths, extreme timestamps
- **State Machine Testing**: Valid and invalid state transitions
- **Invariant Testing**: Type constraints cannot be violated

#### Builder Tests (`tests/builder_tests.rs`)
- **Fluent API**: Method chaining works correctly
- **Validation**: Each method validates its input
- **Default Behavior**: Sensible defaults applied correctly
- **Error Propagation**: Validation errors surface immediately

#### Wrapper Tests (`tests/wrapper_tests.rs`)
- **Composition**: Wrappers can be stacked together
- **Automatic Behavior**: Tracing, caching, retries work transparently
- **Performance**: Cache hit/miss ratios, retry counts measured
- **Error Handling**: Failure scenarios handled gracefully

### Property-Based Testing Integration

Stage 6 components integrate with the existing property-based testing from Stage 5:

```rust
#[test]
fn validated_path_never_allows_traversal() {
    proptest!(|(path_input in any_string())| {
        if let Ok(validated) = ValidatedPath::new(&path_input) {
            // If validation succeeded, path is guaranteed safe
            assert!(!validated.as_str().contains(".."));
            assert!(!validated.as_str().contains('\0'));
        }
        // If validation failed, that's also correct behavior
    });
}
```

## Performance Characteristics

### Validated Types
- **Zero Runtime Cost**: Validation only at construction time
- **Compile-Time Optimization**: NewType patterns optimize away
- **Memory Efficiency**: No additional overhead beyond wrapped types

### Builder Patterns
- **Allocation Efficient**: Builders reuse allocations where possible
- **Lazy Validation**: Only validate when needed, cache results
- **Move Semantics**: Take ownership to avoid unnecessary copies

### Wrapper Components
- **Composable Overhead**: Each wrapper adds minimal overhead
- **Async-Optimized**: All wrappers designed for async/await patterns
- **Zero-Copy Where Possible**: Pass-through wrappers avoid data copies

## Integration with Previous Stages

### Stage 1-2 Integration: Contracts and Tests
```rust
#[async_trait]
impl<S: Storage> Storage for TracedStorage<S> {
    async fn insert(&mut self, doc: Document) -> Result<()> {
        // Stage 2: Contract validation
        validation::document::validate_for_insert(&doc, &HashSet::new())?;
        
        // Stage 6: Automatic tracing
        with_trace_id("storage.insert", async {
            self.inner.insert(doc).await
        }).await
    }
}
```

### Stage 3-4 Integration: Pure Functions and Observability
```rust
impl DocumentBuilder {
    fn calculate_word_count(content: &[u8]) -> u32 {
        // Stage 3: Pure function for word counting
        pure::text::count_words(content)
    }
    
    pub fn build(self) -> Result<Document> {
        // Stage 4: Automatic metric recording
        let start = Instant::now();
        let result = self.build_internal();
        record_metric(MetricType::Histogram {
            name: "document_builder.build.duration".to_string(),
            value: start.elapsed().as_millis() as f64,
            tags: vec![],
        });
        result
    }
}
```

### Stage 5 Integration: Adversarial Testing
All Stage 6 components are tested against the adversarial scenarios from Stage 5:
- **Concurrent Access**: Multiple threads using builders simultaneously
- **Invalid Inputs**: Fuzz testing with random byte sequences
- **Resource Exhaustion**: Large caches, many retry attempts
- **Failure Injection**: Wrapped storage that simulates failures

## Usage Examples

### Basic Document Processing

```rust
use kotadb::{DocumentBuilder, TracedStorage, CachedStorage};

async fn process_document(content: &[u8], path: &str) -> Result<()> {
    // Stage 6: Builder with validation
    let doc = DocumentBuilder::new()
        .path(path)?  // Validated path
        .title("Auto-Generated")?  // Validated title
        .content(content)  // Auto-calculated word count
        .build()?;
    
    // Stage 6: Wrapped storage with automatic best practices
    let storage = create_wrapped_storage(BaseStorage::new(), 1000).await;
    storage.insert(doc).await?;  // Traced, cached, retried, validated
    
    Ok(())
}
```

### Advanced Query Building

```rust
use kotadb::{QueryBuilder, ValidatedTag};

async fn build_complex_query() -> Result<Query> {
    let query = QueryBuilder::new()
        .with_text("machine learning")?
        .with_tags(vec!["ai", "algorithms", "rust"])?
        .with_date_range(
            chrono::Utc::now().timestamp() - 86400 * 7,  // Last week
            chrono::Utc::now().timestamp()
        )?
        .with_limit(25)?
        .build()?;
    
    Ok(query)
}
```

### Storage Configuration

```rust
use kotadb::{StorageConfigBuilder, IndexConfigBuilder};

async fn setup_optimized_storage() -> Result<()> {
    let storage_config = StorageConfigBuilder::new()
        .path("/data/knowledge-base")?
        .cache_size(512 * 1024 * 1024)  // 512MB cache
        .compression(true)
        .encryption_key([0u8; 32])  // Use real key in production
        .build()?;
    
    let index_config = IndexConfigBuilder::new()
        .name("semantic_search")
        .max_memory(100 * 1024 * 1024)  // 100MB
        .fuzzy_search(true)
        .similarity_threshold(0.85)?
        .build()?;
    
    // Use configurations...
    Ok(())
}
```

## Best Practices

### When to Use Validated Types
- **Always** for user inputs (paths, queries, identifiers)
- **Always** for data with invariants (timestamps, sizes, limits)
- **Consider** for internal types that have constraints

### When to Use Builders
- **Complex objects** with many optional fields
- **Configuration objects** with sensible defaults
- **Objects requiring validation** of field combinations

### When to Use Wrappers
- **Cross-cutting concerns** like logging, metrics, caching
- **Infrastructure patterns** like retries, circuit breakers
- **Behavioral modification** without changing core logic

### Composition Guidelines
- **Layer by responsibility**: Group related concerns together
- **Optimize for readability**: Most important wrapper outermost
- **Consider performance**: Expensive operations (validation) inner
- **Test composition**: Verify wrappers work together correctly

## Future Extensions

### Additional Validated Types
- `ValidatedEmail`: Email address validation
- `ValidatedUrl`: URL format and reachability
- `ValidatedLanguageCode`: ISO language codes
- `ValidatedMimeType`: MIME type validation

### Additional Builders
- `FilterBuilder`: Complex query filters
- `IndexBuilder`: Index configuration with optimization hints
- `BackupConfigBuilder`: Backup and restore configurations

### Additional Wrappers
- `RateLimitedStorage`: Rate limiting for external APIs
- `EncryptedStorage`: Transparent encryption/decryption
- `VersionedStorage`: Automatic versioning and rollback
- `DistributedStorage`: Multi-node consistency

## Conclusion

Stage 6's Component Library provides the foundation for reliable, maintainable code by:

1. **Eliminating Invalid States**: Validated types make bugs unrepresentable
2. **Encoding Best Practices**: Wrappers automatically apply proven patterns
3. **Improving Developer Experience**: Builders make complex construction ergonomic
4. **Enabling Composition**: Components combine to create powerful functionality

The -1.0 risk reduction is achieved through **prevention rather than detection** - problems that can't happen don't need to be debugged.