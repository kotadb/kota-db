# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands for Development

### Build and Run
```bash
# Build the project
cargo build
cargo build --release  # Production build

# Run the main server with configuration
cargo run -- --config kotadb-dev.toml

# Run with specific commands
cargo run stats              # Show database statistics
cargo run search "rust"      # Full-text search (trigram index)
cargo run search "*"         # Wildcard search (primary index)

# Development server with auto-reload
just dev                     # Uses cargo watch for auto-reload
```

### Testing
```bash
# Run all tests
cargo test --all
just test

# Run specific test types
cargo test --lib                           # Unit tests only
cargo test --test '*'                      # Integration tests only
cargo test test_name                       # Run tests matching name
cargo test --test phase2b_concurrent_stress  # Run specific test file

# Performance and stress tests
cargo test --release --features bench performance_regression_test
just test-perf

# Run single test with output
cargo test test_name -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt --all
just fmt

# Run clippy linter (MUST pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings
just clippy

# Run all quality checks (format check + clippy + unit tests)
just check
```

### Benchmarking
```bash
# Run database benchmarks
just db-bench
cargo run --release -- benchmark --operations 10000

# Run performance benchmarks
cargo bench --features bench
```

## High-Level Architecture

KotaDB is a custom database designed for distributed human-AI cognition, built in Rust with zero external database dependencies.

### Core Components

#### Storage Layer (`src/file_storage.rs`)
- **FileStorage**: Page-based storage engine with Write-Ahead Log (WAL)
- **Factory Function**: `create_file_storage()` returns production-ready wrapped storage with tracing, validation, retries, and caching
- **Documents**: Stored as both Markdown (`.md`) and JSON (`.json`) files
- **Persistence**: 4KB page-based architecture with checksums

#### Index Systems
1. **Primary Index** (`src/primary_index.rs`)
   - B+ tree implementation for O(log n) path-based lookups
   - Handles wildcard queries and range scans
   - Full persistence with crash recovery

2. **Trigram Index** (`src/trigram_index.rs`)
   - Full-text search using trigram tokenization
   - Dual-index architecture: trigram → documents and document → trigrams
   - Fuzzy search tolerance with ranking

3. **Vector Index** (`src/vector_index.rs`)
   - HNSW (Hierarchical Navigable Small World) for semantic search
   - Supports embeddings from multiple providers

#### Wrapper System (`src/wrappers.rs`)
Production-ready wrappers that compose around storage and indices:
- **TracedStorage**: Distributed tracing with unique IDs
- **ValidatedStorage**: Runtime contract validation
- **RetryableStorage**: Automatic retry with exponential backoff
- **CachedStorage**: LRU caching for frequently accessed documents
- **MeteredIndex**: Performance metrics and monitoring

#### Type Safety (`src/types.rs`, `src/validation.rs`)
Validated types ensure compile-time and runtime safety:
- `ValidatedPath`, `ValidatedDocumentId`, `ValidatedTimestamp`
- Builder patterns for safe construction
- Comprehensive validation rules

#### Query System
- **Query Routing**: Automatic selection between primary and trigram indices based on query pattern
- **Natural Language**: Designed for LLM interaction patterns
- **Performance**: Sub-10ms query latency for most operations

### MCP Server (`src/mcp/`)
Model Context Protocol server for LLM integration:
- `src/bin/mcp_server.rs`: Full MCP server implementation
- `src/bin/mcp_server_minimal.rs`: Minimal implementation for testing
- Configuration via `kotadb-mcp-dev.toml`

### Testing Infrastructure
- **Unit Tests**: In-module tests throughout codebase
- **Integration Tests**: `tests/` directory with comprehensive scenarios
- **Performance Tests**: `benches/` directory with criterion benchmarks
- **Stress Tests**: Chaos testing, concurrent access, adversarial inputs
- **Test Constants**: `tests/test_constants.rs` for shared test configuration

### Key Design Patterns

1. **6-Stage Risk Reduction**: Test-driven, contract-first, pure functions, observability, adversarial testing, component library
2. **Builder Pattern**: Safe construction of complex types (DocumentBuilder, QueryBuilder)
3. **Factory Functions**: `create_*` functions return fully-wrapped production components
4. **Async-First**: All I/O operations use async/await with Tokio
5. **Zero-Copy**: Extensive use of references and memory-mapped I/O

## Working with the Codebase

### Adding New Features
1. Start with tests in the appropriate test file
2. Implement using existing patterns and wrappers
3. Ensure all tests pass including integration tests
4. Run `just check` to verify code quality

### Performance Considerations
- Use `MeteredIndex` wrapper for new indices
- Leverage connection pooling for concurrent operations
- Profile with `cargo bench` before and after changes
- Target sub-10ms query latency

### Common Patterns
- Use factory functions (`create_*`) instead of direct construction
- Wrap all storage/index implementations with appropriate wrappers
- Use validated types for all user input
- Include tracing spans for observability

### Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for library errors
- Always include context with `.context()`
- Log errors at appropriate levels with `tracing`