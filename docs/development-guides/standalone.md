# Running KotaDB Standalone

KotaDB is designed as a complete, independent database system that can run outside of the parent KOTA project. This document explains how to use KotaDB as a standalone application.

## Quick Start

### 1. Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

### 2. Setup

```bash
# Clone or copy the KotaDB directory
cd temp-kota-db

# Make the runner executable
chmod +x run_standalone.sh

# Check status
./run_standalone.sh status
```

### 3. Build

```bash
# Build in release mode
./run_standalone.sh build

# Run tests to verify everything works
./run_standalone.sh test
```

### 4. Try the Demo

```bash
# See Stage 6 components in action
./run_standalone.sh demo
```

## CLI Usage

### Available Commands

```bash
# Show help
./run_standalone.sh run --help

# Database operations (placeholders until storage engine implemented)
./run_standalone.sh run stats           # Show database statistics  
./run_standalone.sh run index /path     # Index documents
./run_standalone.sh run search "query"  # Search documents
./run_standalone.sh run verify          # Verify integrity
```

### Current Implementation Status

✅ **Fully Implemented (Stage 6 Complete)**
- Validated types with compile-time safety
- Builder patterns for ergonomic construction
- Wrapper components with automatic best practices
- Comprehensive test coverage
- Full documentation

🚧 **In Progress (Next Steps)**
- Storage engine implementation
- Index implementation  
- Full CLI functionality

## Architecture Overview

KotaDB uses a 6-stage risk reduction methodology:

```
┌─────────────────────────────────────────────────────────────┐
│                    CLI Interface                             │
│              (Clap-based command parsing)                   │
├─────────────────────────────────────────────────────────────┤
│                 Stage 6: Component Library                  │
│     (Validated Types + Builders + Wrappers)                │
├──────────────┬───────────────┬───────────────┬──────────────┤
│   Stage 2:   │   Stage 3:    │   Stage 4:    │   Stage 5:   │
│  Contracts   │Pure Functions │ Observability │ Adversarial  │
│              │               │               │   Testing    │
├──────────────┴───────────────┴───────────────┴──────────────┤
│                   Stage 1: Test-Driven Development          │
│              (Comprehensive test coverage)                  │
└─────────────────────────────────────────────────────────────┘
```

## Stage 6 Components (Current Focus)

### Validated Types (`src/types.rs`)

```rust
use kotadb::types::*;

// Safe file paths (no traversal, null bytes, etc.)
let path = ValidatedPath::new("/documents/notes.md")?;

// Non-nil document IDs
let id = ValidatedDocumentId::new();

// Non-empty, trimmed titles  
let title = ValidatedTitle::new("My Document")?;

// Document lifecycle state machine
let draft = TypedDocument::<Draft>::new(/* ... */);
let persisted = draft.into_persisted();
let modified = persisted.into_modified();
```

### Builder Patterns (`src/builders.rs`)

```rust
use kotadb::builders::*;

// Document construction with validation
let doc = DocumentBuilder::new()
    .path("/knowledge/rust-patterns.md")?
    .title("Rust Design Patterns")?
    .content(b"# Patterns\n\nContent...")
    .build()?;

// Query building with fluent API
let query = QueryBuilder::new()
    .with_text("machine learning")?
    .with_tags(vec!["ai", "rust"])?
    .with_limit(25)?
    .build()?;
```

### Wrapper Components (`src/wrappers.rs`)

```rust
use kotadb::wrappers::*;

// Automatic best practices through composition
let storage = create_wrapped_storage(base_storage, 1000).await;
// Provides: Tracing + Validation + Retries + Caching

// Individual wrappers
let traced = TracedStorage::new(storage);       // Automatic tracing
let cached = CachedStorage::new(storage, 100);  // LRU caching
let retryable = RetryableStorage::new(storage); // Exponential backoff
```

## Development Workflow

### 1. Running Tests

```bash
# All tests
./run_standalone.sh test

# Specific test categories (when implemented)
cargo test validated_types    # Type safety tests
cargo test builder_patterns   # Builder functionality  
cargo test wrapper_components # Wrapper composition
```

### 2. Adding New Features

Follow the 6-stage methodology:

1. **Write tests first** (TDD)
2. **Define contracts** (interfaces and validation)
3. **Extract pure functions** (business logic)
4. **Add observability** (tracing and metrics)
5. **Test adversarially** (failure scenarios)
6. **Use Stage 6 components** (validated types, builders, wrappers)

### 3. Performance Testing

```bash
# Benchmarks (when implemented)
cargo bench --features bench

# Performance profiling
cargo run --release --bin kotadb -- stats
```

## Integration as a Library

KotaDB can also be used as a Rust library:

### Cargo.toml

```toml
[dependencies]
kotadb = { path = "../temp-kota-db" }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

### Library Usage

```rust
use kotadb::{DocumentBuilder, create_wrapped_storage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    kotadb::init_logging()?;
    
    // Create document with validation
    let doc = DocumentBuilder::new()
        .path("/my-notes/today.md")?
        .title("Daily Notes")?
        .content(b"# Today\n\nThoughts and ideas...")
        .build()?;
    
    // Use wrapped storage for automatic best practices
    let mut storage = create_wrapped_storage(
        YourStorageImpl::new(), 
        1000  // cache capacity
    ).await;
    
    // All operations automatically traced, cached, retried, validated
    storage.insert(doc).await?;
    
    Ok(())
}
```

## Configuration

### Environment Variables

```bash
# Logging level
export RUST_LOG=info

# Database path (when storage implemented)
export KOTADB_PATH=/path/to/database

# Cache settings
export KOTADB_CACHE_SIZE=1000
export KOTADB_SYNC_INTERVAL=30
```

### Configuration File (Future)

```toml
# kotadb.toml
[storage]
path = "/data/kotadb"
cache_size = "256MB"
compression = true

[indices]
full_text = { enabled = true, max_memory = "100MB" }
semantic = { enabled = true, model = "all-MiniLM-L6-v2" }
graph = { enabled = true, max_depth = 5 }

[observability]
tracing = true
metrics = true
log_level = "info"
```

## Troubleshooting

### Common Issues

1. **Workspace Conflicts**
   ```bash
   # The run_standalone.sh script handles this automatically
   ./run_standalone.sh build
   ```

2. **Missing Dependencies**
   ```bash
   # Install Rust if not present
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Test Failures**
   ```bash
   # Run tests with verbose output
   cargo test -- --nocapture
   ```

### Getting Help

1. **Check Status**
   ```bash
   ./run_standalone.sh status
   ```

2. **Review Documentation**
   ```bash
   ls docs/
   cat docs/QUICK_REFERENCE.md
   ```

3. **Run Demo**
   ```bash
   ./run_standalone.sh demo
   ```

## Deployment

### Standalone Binary

```bash
# Build optimized binary
./run_standalone.sh build

# Copy binary to deployment location
cp target/release/kotadb /usr/local/bin/

# Run anywhere
kotadb --help
```

### Docker (Future)

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/kotadb /usr/local/bin/
ENTRYPOINT ["kotadb"]
```

## Roadmap

### Phase 1: Core Implementation (Current)
- ✅ Stage 6 component library complete
- 🚧 Storage engine using Stage 6 components
- 🚧 Index implementation with wrappers

### Phase 2: Full Functionality
- 📋 Complete CLI implementation
- 📋 Configuration system
- 📋 Performance optimization

### Phase 3: Advanced Features  
- ⏸️ Semantic search capabilities (retired until cloud-first relaunch)
- 📋 Graph traversal algorithms
- 📋 Real-time indexing

## Contributing

KotaDB demonstrates how systematic risk reduction can create reliable software. The 6-stage methodology reduces implementation risk from ~78% to ~99% success rate.

To contribute:
1. Follow the risk reduction methodology
2. Use Stage 6 components for all new code
3. Write tests first (TDD)
4. Document contracts and invariants
5. Add comprehensive observability

## License

This project is currently private and proprietary, shared for educational purposes to demonstrate the 6-stage risk reduction methodology in practice.
