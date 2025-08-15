# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Branching Strategy

This repository uses Git Flow. **Always work on feature branches, never directly on main or develop.**

### Quick Reference
- **Current branch to work from**: `develop`
- **Create feature branches**: `git checkout -b feature/your-feature`
- **Create PRs to**: `develop` branch
- **Production branch**: `main` (protected, requires reviews)

### Workflow
```bash
# Start new work
git checkout develop
git pull origin develop
git checkout -b feature/your-feature

# After making changes
git add .
git commit -m "feat: your changes"
git push -u origin feature/your-feature

# Create PR
gh pr create --base develop
```

For more details, see `docs/BRANCHING_STRATEGY.md`.

## GitHub Issue Management

### Label Management Protocol
Before creating issues, always check existing labels and create new ones if needed:

```bash
# Check what labels are available
gh label list --limit 100

# Search for specific label types
gh label list --search "bug"
gh label list --search "performance"

# Create new labels when needed
gh label create "database" --description "Database-related issues" --color "1d76db"
gh label create "embedding" --description "Embedding and vector search issues" --color "0e8a16"
gh label create "mcp-server" --description "MCP server related issues" --color "6f42c1"

# Create issues with appropriate labels
gh issue create --title "Issue title" --body "Description" --label "bug,storage,priority-high"
```

### Standard Label Categories for KotaDB
- **Component**: `storage`, `index`, `mcp`, `embedding`, `vector-search`, `trigram`, `primary-index`
- **Type**: `bug`, `enhancement`, `feature`, `refactor`, `documentation`, `test`  
- **Priority**: `priority-critical`, `priority-high`, `priority-medium`, `priority-low`
- **Effort**: `effort-small` (< 1 day), `effort-medium` (1-3 days), `effort-large` (> 3 days)
- **Status**: `needs-investigation`, `blocked`, `in-progress`, `ready-for-review`

## Commands for Development

### Versioning and Release

**🚨 IMPORTANT: Always perform releases from the `develop` branch!**

```bash
# STEP 1: Switch to develop branch
git checkout develop
git pull origin develop

# STEP 2: Check current version and preview release
just version                # Current version
just release-preview        # Shows unreleased changes and recent commits

# STEP 3: Run release command (FROM DEVELOP BRANCH)
just release-patch          # Bump patch: 0.1.0 -> 0.1.1
just release-minor          # Bump minor: 0.1.0 -> 0.2.0
just release-major          # Bump major: 0.1.0 -> 1.0.0
just release-beta           # Beta release: 0.1.0 -> 0.1.0-beta.1

# Or release specific version
just release 0.2.0          # Full release process

# STEP 4: After release, merge main back to develop
git fetch origin main
git merge origin/main -m "chore: sync version updates from v[VERSION] release"
git push origin develop

# Other commands
just release-dry-run 0.2.0  # Test without making changes
just changelog-update       # Add new unreleased section after release
```

The release process automatically:
- Runs all tests and quality checks
- Updates version in Cargo.toml, VERSION, CHANGELOG.md
- Updates client library versions
- Creates git tag with changelog
- Pushes to main branch
- Triggers GitHub Actions for binaries, Docker images, crates.io, PyPI, and npm

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

## Important Files

### Versioning & Release
- `VERSION` - Current version number (plain text)
- `CHANGELOG.md` - Version history following Keep a Changelog format
- `scripts/release.sh` - Automated release script
- `scripts/version-bump.sh` - Version bumping utility
- `docs/RELEASE_PROCESS.md` - Complete release documentation
- `.github/workflows/release.yml` - GitHub Actions release automation

Always update CHANGELOG.md when making user-facing changes by adding entries to the `[Unreleased]` section.