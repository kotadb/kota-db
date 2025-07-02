# KotaDB Quick Reference

## FileStorage Quick Start

### Basic Setup
```rust
use kotadb::{create_file_storage, DocumentBuilder, Storage};

// Create production-ready storage with all Stage 6 wrappers
let mut storage = create_file_storage("/path/to/db", Some(1000)).await?;
```

### Document Operations

#### Create Document
```rust
let doc = DocumentBuilder::new()
    .path("/notes/example.md")?
    .title("Example Document")?
    .content(b"# Example\n\nDocument content here...")?
    .build()?;
```

#### Store Document
```rust
storage.insert(doc.clone()).await?;
```

#### Retrieve Document
```rust
let retrieved = storage.get(&doc.id).await?;
match retrieved {
    Some(doc) => println!("Found: {}", doc.title),
    None => println!("Document not found"),
}
```

#### Update Document
```rust
let mut updated_doc = doc;
updated_doc.title = "Updated Title".to_string();
updated_doc.updated = chrono::Utc::now().timestamp();
storage.update(updated_doc).await?;
```

#### Delete Document
```rust
storage.delete(&doc.id).await?;
```

## Validated Types (Import: `use kotadb::types::*;`)

```rust
// Safe file paths
let path = ValidatedPath::new("/knowledge/notes.md")?;

// Non-nil document IDs  
let id = ValidatedDocumentId::new();  // or from_uuid(uuid)?

// Non-empty, trimmed titles
let title = ValidatedTitle::new("My Document")?;

// Positive file sizes
let size = NonZeroSize::new(1024)?;

// Valid timestamps (> 0, < far future)
let timestamp = ValidatedTimestamp::now();  // or new(secs)?

// Ordered timestamp pairs (updated >= created)
let timestamps = TimestampPair::new(created, updated)?;

// Sanitized tags (alphanumeric, dash, underscore only)
let tag = ValidatedTag::new("rust-lang")?;

// Validated search queries (min length, trimmed)
let query = ValidatedSearchQuery::new("search term", 3)?;

// Non-zero page identifiers
let page_id = ValidatedPageId::new(42)?;

// Bounded result limits
let limit = ValidatedLimit::new(25, 100)?;  // value, max
```

## Document State Machine

```rust
// Create draft document
let draft = TypedDocument::<Draft>::new(path, hash, size, title, word_count);

// State transitions (compile-time enforced)
let persisted = draft.into_persisted();
let modified = persisted.into_modified();
let persisted_again = modified.into_persisted();

// Invalid transitions won't compile:
// let bad = draft.into_modified();  // Error!
```

## Builder Patterns (Import: `use kotadb::builders::*;`)

```rust
// Document builder with validation and defaults
let doc = DocumentBuilder::new()
    .path("/notes/rust-patterns.md")?      // Required, validated
    .title("Rust Design Patterns")?        // Required, validated  
    .content(b"# Patterns\n\nContent...")  // Required, auto word count
    .word_count(150)                       // Optional override
    .timestamps(1000, 2000)?               // Optional, defaults to now
    .build()?;

// Query builder with fluent API
let query = QueryBuilder::new()
    .with_text("machine learning")?        // Text search
    .with_tag("ai")?                       // Single tag
    .with_tags(vec!["rust", "ml"])?        // Multiple tags
    .with_date_range(start, end)?          // Time bounds
    .with_limit(50)?                       // Result limit
    .build()?;

// Storage configuration with defaults
let config = StorageConfigBuilder::new()
    .path("/data/kotadb")?                 // Required
    .cache_size(256 * 1024 * 1024)         // 256MB, default 100MB
    .compression(true)                     // Default true
    .no_cache()                            // Disable caching
    .encryption_key([0u8; 32])             // Optional
    .build()?;

// Index configuration
let index_config = IndexConfigBuilder::new()
    .name("semantic_index")                // Required
    .max_memory(100 * 1024 * 1024)         // 100MB, default 50MB
    .fuzzy_search(true)                    // Default true
    .similarity_threshold(0.85)?           // 0-1 range, default 0.8
    .persistence(false)                    // Default true
    .build()?;

// Metrics collection
let metrics = MetricsBuilder::new()
    .document_count(1000)
    .total_size(50 * 1024 * 1024)          // 50MB
    .index_size("full_text", 5 * 1024 * 1024)
    .index_size("semantic", 10 * 1024 * 1024)
    .build()?;
```

## Wrapper Components (Import: `use kotadb::wrappers::*;`)

```rust
// Individual wrappers
let storage = MockStorage::new();

// Add automatic tracing with unique trace IDs
let traced = TracedStorage::new(storage);
let trace_id = traced.trace_id();
let op_count = traced.operation_count().await;

// Add input/output validation  
let validated = ValidatedStorage::new(storage);

// Add retry logic with exponential backoff
let retryable = RetryableStorage::new(storage)
    .with_retry_config(
        3,                                     // max_retries
        Duration::from_millis(100),            // base_delay  
        Duration::from_secs(5)                 // max_delay
    );

// Add LRU caching
let cached = CachedStorage::new(storage, 1000);  // 1000 item capacity
let (hits, misses) = cached.cache_stats().await;

// Composed wrapper (recommended)
let fully_wrapped = create_wrapped_storage(base_storage, 1000).await;
// Type: TracedStorage<ValidatedStorage<RetryableStorage<CachedStorage<BaseStorage>>>>

// Index with automatic metrics
let index = MeteredIndex::new(base_index, "my_index".to_string());
let timing_stats = index.timing_stats().await;  // (min, avg, max) per operation

// RAII transaction safety
let mut tx = SafeTransaction::begin(1)?;
tx.add_operation(Operation::StorageWrite { doc_id, size_bytes });
tx.commit().await?;  // Must explicitly commit
// Automatic rollback if dropped without commit
```

## Common Patterns

### Error Handling
```rust
// All Stage 6 types return Result<T, anyhow::Error>
match ValidatedPath::new(user_input) {
    Ok(path) => /* path is guaranteed safe */,
    Err(e) => eprintln!("Invalid path: {}", e),
}

// Or use ? operator for propagation
let path = ValidatedPath::new(user_input)?;
```

### Conversion and Display
```rust
// All validated types implement Display and common conversions
let path = ValidatedPath::new("/notes/file.md")?;
println!("Path: {}", path);                    // Display
let path_str: &str = path.as_str();            // &str
let path_string: String = path.to_string();    // String
let path_buf: &Path = path.as_path();          // &Path

// Document IDs
let id = ValidatedDocumentId::new();
let uuid: Uuid = id.as_uuid();
let id_string: String = id.to_string();
```

### Async Patterns
```rust
// All storage operations are async
async fn example_usage() -> Result<()> {
    let mut storage = create_wrapped_storage(BaseStorage::new(), 1000).await;
    
    let doc = DocumentBuilder::new()
        .path("/test.md")?
        .title("Test")?  
        .content(b"content")
        .build()?;
    
    storage.insert(doc.clone()).await?;
    let retrieved = storage.get(&doc.id).await?;
    
    Ok(())
}
```

### Testing Helpers
```rust
// Create test documents easily
fn create_test_doc() -> Document {
    DocumentBuilder::new()
        .path("/test/doc.md").unwrap()
        .title("Test Document").unwrap()
        .content(b"Test content")
        .build().unwrap()
}

// Mock storage for testing
struct MockStorage { /* ... */ }

#[async_trait]
impl Storage for MockStorage {
    // Implement required methods
}
```

## Performance Tips

### Validated Types
- **Construction Cost**: Validation only happens once at creation
- **Runtime Cost**: Zero overhead after construction (newtype pattern)
- **Memory**: Same size as wrapped type

### Builders
- **Reuse**: Builders can be cloned before final build
- **Validation**: Happens incrementally, not just at build()
- **Memory**: Minimal overhead, optimized for move semantics

### Wrappers  
- **Composition Order**: Put expensive operations (validation) inner
- **Caching**: Size cache appropriately for your working set
- **Tracing**: Negligible overhead when logging level is appropriate
- **Retries**: Configure timeouts to match your failure characteristics

### Best Practices
```rust
// Good: Validate once, use many times
let path = ValidatedPath::new(user_input)?;
for item in items {
    process_with_path(&path, item).await?;
}

// Good: Compose wrappers for automatic best practices  
let storage = create_wrapped_storage(base, cache_size).await;

// Good: Use builders for complex objects
let query = QueryBuilder::new()
    .with_text(&search_term)?
    .with_limit(page_size)?
    .build()?;

// Good: RAII transactions
{
    let mut tx = SafeTransaction::begin(next_id())?;
    // ... operations
    tx.commit().await?;
}  // Automatic cleanup
```

## Integration with Other Stages

### Stage 1-2: Tests and Contracts
- All components have comprehensive test coverage
- Contracts validated automatically by wrappers
- Property-based testing for edge cases

### Stage 3-4: Pure Functions and Observability  
- Builders use pure functions for calculations
- Wrappers provide automatic tracing and metrics
- All operations have unique trace IDs

### Stage 5: Adversarial Testing
- Components tested against failure scenarios
- Concurrent access patterns validated
- Fuzz testing for input validation

This reference covers the essential Stage 6 components. For detailed documentation, see `docs/STAGE6_COMPONENT_LIBRARY.md`.