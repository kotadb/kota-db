# Changelog

All notable changes to KotaDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed
- **BREAKING CHANGE**: Removed natural language query parser (#445)
  - Removed `RelationshipQuery` CLI command that supported natural language queries
  - Removed `natural_language_query` module and all NL parsing functions
  - Removed `parse_natural_language_relationship_query` function from relationship_query module
  - Natural language patterns like "what calls X?" are no longer supported
  - **Migration**: Use direct commands instead:
    - Replace "what calls X?" with `find-callers X`
    - Replace "what would break if I change X?" with `analyze-impact X`
    - Replace "find unused functions" with direct symbol queries
  - **Rationale**: The NL parser was limited (only 2 patterns), created false expectations, and added complexity for minimal benefit
  - Direct commands are clearer, more predictable, and have better error messages

### Added
- Made `--quiet` mode the default for CLI to optimize LLM context usage (#429)
  - Reduces token consumption by 70% for AI assistants
  - Use `--quiet=false` to show detailed output
  - Aligns with project goal of reducing LLM context requirements
- Enhanced natural language query parser for improved AI agent experience (#431)
  - Fixed rigid parsing patterns that failed on documented query variations
  - Added support for "what would break if I change X?" and alternative phrasings
  - Enhanced "creates" pattern recognition: "what creates storage?" maps to caller queries
  - Added flexible impact analysis patterns: "what depends on X?", "what would be affected by X?"
  - Improved symbol extraction with proper punctuation handling (removes trailing "?", "!", etc.)
  - **Production-ready robustness improvements**: Enhanced symbol validation for qualified names (std::vec::Vec)
  - **Extensible constructor mapping**: Configurable generic term mapping ("storage" → "create_storage") with fallback patterns
  - **Comprehensive edge case handling**: Unicode support, performance validation, malformed query resilience
  - Added comprehensive test coverage for all new natural language patterns (14 total tests)
  - Enables truly natural queries for LLMs, reducing context usage requirements
- LLM-optimized search output with progressive disclosure (#370)
  - New `--context` flag for search command with levels: none, minimal, medium, full
  - "medium" context (default) implements dream workflow with relevance scores and line numbers
  - Smart context extraction shows complete semantic code blocks
  - Progressive disclosure prevents overwhelming LLM context windows
  - Integrated existing LLMSearchEngine for sophisticated relevance scoring
- CI failure investigation and improvements to ensure all checks pass reliably
- `--verbose` flag for CLI operations to enable detailed logging output (#335)
  - Default logging level set to WARN for clean agent-friendly output
  - Use `--verbose` or `-v` to enable INFO level logging for progress visibility
  - RUST_LOG environment variable still overrides all settings for debugging
- `--limit` flag for find-callers and impact-analysis commands (#335)
  - Allows limiting the number of results returned for better readability
  - Prevents overwhelming output when analyzing symbols with many dependencies
- Binary trigram index implementation for high-performance search (#311)
  - New `--binary-index` flag enables 10x faster index operations
  - Uses bincode serialization instead of JSON for compact storage
  - Implements memory-mapped file access for zero-copy operations
  - Reduces index size by 3-5x compared to JSON format
  - Target: Achieve <10ms query latency (previously 580ms)

### Fixed
- Improved relationship type context display in find-callers command (#418)
  - Fixed context strings to correctly show "References" for type usage (not just "Calls")
  - Clarified command help text to indicate it finds all references, not just function calls
  - Includes constructor calls (Type::new), type annotations, and parameter types
- Edge metadata update and removal operations for multiple edge support (#332)
  - Restored `update_edge_metadata` and `remove_edge` methods disabled during #331 fix
  - Added new `update_edge_metadata_by_type` and `remove_edge_by_type` methods
  - Methods now properly handle multiple edges between same nodes with different RelationTypes
  - Added comprehensive test coverage for edge metadata operations with multiple edges
  - Fixed WAL (Write-Ahead Log) operations to support new edge update/delete variants
- CLI text search functionality for programming-related terms (#345)
  - Fixed over-aggressive query sanitization that blocked legitimate search terms
  - Words like "script", "javascript", "select", "insert", "create" now work correctly
  - Made SQL injection protection more precise to target actual injection patterns
  - Preserved security against real SQL injection and XSS attacks
- CLI logging verbosity issues that impacted agent/LLM workflows (#335)
  - Changed default logging level from DEBUG to WARN for cleaner output
  - Eliminated excessive trace output that buried useful information
  - CLI now produces clean, parseable output by default without RUST_LOG=warn
- Wildcard pattern matching in search functionality (#335)
  - Fixed routing of wildcard queries (e.g., "*.rs", "*Controller.rs") to primary index
  - Implemented proper glob pattern matching for path-based searches
  - Wildcard patterns now correctly filter results based on file paths
- Symbol storage directory not being created automatically (#272)
  - Added automatic directory creation when accessing symbol storage
  - Fixed issue where symbol-stats and relationship commands would fail on first use
  - Symbol storage paths are now created lazily when needed
- Missing CLI commands for symbol analysis (#271)
  - Enabled tree-sitter-parsing feature by default
  - Made symbol-stats, find-callers, impact-analysis commands available by default
  - Commands were previously hidden behind feature flag
- Improved documentation for ingest-repo command flags (#273)
  - Added clearer documentation for --extract-symbols and --no-symbols flags
  - Made symbol extraction enabled by default with tree-sitter feature
  - Improved help text to explain flag usage
- Over-aggressive query sanitization breaking path searches (#275)
  - Added path-aware query sanitization that preserves forward slashes
  - Fixed issue where path-based searches were failing due to slash removal
  - Automatically detects path queries and applies appropriate sanitization
- Critical index synchronization failure during repository ingestion (#248)
  - Fixed validation false positive showing primary index limited to 1000 documents
    - Increased query limits from 1000 to 100,000 throughout the codebase
    - Updated ValidationConfig defaults to handle larger repositories
  - Fixed trigram index returning 0 documents for wildcard queries
    - Added support for wildcard queries (empty search terms) in trigram index
    - Trigram index now returns all indexed documents for validation queries
  - Added comprehensive test suite to reproduce and validate the fixes
  - This resolves issues where search validation failed for repositories with >1000 documents
- Critical trigram index bug where insert() method was not indexing actual document content (#249)
  - Changed insert() to return error directing users to insert_with_content()
  - Fixed unit test to use insert_with_content() for proper content indexing
  - Added comprehensive test to validate the error behavior
  - This fixes full-text search which was completely broken due to placeholder indexing

### Security
- Enhanced input sanitization for search queries (#202)
  - Added comprehensive query sanitization module to prevent injection attacks
  - Protection against SQL injection, command injection, XSS, and path traversal
  - Validated all search input through QueryBuilder and Query constructors
  - Added ValidatedSearchQuery type with built-in sanitization
  - Comprehensive test suite for security validation

## [0.5.0] - 2025-08-15

### Added
- Comprehensive MCP package integration testing suite (#124)
  - Protocol compliance tests for JSON-RPC 2.0 and MCP standards
  - Real-world user workflow scenarios and new user onboarding flows
  - Cross-platform compatibility testing (macOS, Linux, Windows)
  - Stress testing and performance validation with sub-10ms query targets
  - Anti-mock testing philosophy using real MCP server processes
  - CI/CD integration for automated MCP functionality validation

## [0.4.0] - 2025-08-14

## [0.3.1] - 2025-08-14

### Added
- Comprehensive getting started guide and examples documentation (#111)
- Recovery and preservation of all Claude agent configurations

### Changed
- Client library improvements including linting standards and test coverage (#87, #94, #97, #100)
- Dropped Python 3.8 support in CI/CD pipeline

### Fixed
- Python client CI/CD to run all unit tests properly
- Python client version mismatch between local and PyPI (#87)
- TypeScript package publish failures in CI due to test server requirement (#100)
- Release workflow and MkDocs validation issues (#98, #99)
- Various CI/CD pipeline issues affecting v0.3.0 release

### Documentation
- Updated README to properly reflect v0.3.0 TypeScript/Python type safety features
- Improved documentation for v0.3.0 release features

## [0.3.0] - 2025-08-13

### Added
- Comprehensive TypeScript client type safety and builder patterns (#93)
- Comprehensive Python client improvements with validated types and builders (#91)

### Changed
- Improved client library discoverability and documentation (#90)

### Fixed
- GitHub Pages documentation deployment issues (#76)

## [0.2.1] - 2025-08-12

### Added
- Git Flow branching strategy with branch protection rules (#64)
- Automated GitHub Pages versioning with Mike (#65)
- Performance benchmarks in Docker builds

### Changed
- Replaced std::sync::RwLock with parking_lot::RwLock for 3-5x faster lock operations (#72)
- Increased bulk operation threshold from 50 to 500 for better batch performance (#72)
- Added Vec::with_capacity() pre-allocation in hot paths to reduce memory allocations (#72)
- Optimized CI workflow for better efficiency and reliability

### Fixed
- Docker build now includes storage_stress benchmark
- Code coverage job resilience improvements
- Package publishing workflow robustness

### Security
- Updated slab crate to resolve security vulnerability

## [0.2.0] - 2025-08-11

### Added
- Phase 1 client libraries for PostgreSQL-level ease of use (#50)
  - Python client library with full async support
  - TypeScript/JavaScript client library for Node.js and browsers
  - Go client library with native performance
  - Rust client library as a lightweight wrapper
- Comprehensive client documentation and examples
- Client library CI/CD pipelines

### Changed
- Updated README with stunning minimal design
- Enhanced documentation structure for better navigation

### Security
- Bumped rust from 1.70-bullseye to 1.89-bullseye (#53)

### Infrastructure
- Upgraded actions/upload-artifact from 3 to 4 (#52)

## [0.1.0] - 2024-01-01

### Added
- Initial release of KotaDB
- Core storage engine with Write-Ahead Log (WAL)
- B+ tree primary index for path-based lookups
- Trigram index for full-text search
- HNSW vector index for semantic search
- 6-stage risk reduction architecture
- Component library with validated types and wrappers
- Model Context Protocol (MCP) server implementation
- Comprehensive test suite with property-based testing
- Docker support and Kubernetes manifests
- GitHub Actions CI/CD pipeline

### Performance
- Sub-10ms query latency for most operations
- Bulk operations with 10x speedup
- Memory overhead less than 2.5x raw data size

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jayminwest/kota-db/releases/tag/v0.1.0
[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jayminwest/kota-db/compare/v0.1.0...v0.2.0

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/jayminwest/kota-db/compare/v0.2.0...v0.2.1

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/jayminwest/kota-db/compare/v0.2.1...v0.3.0

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/jayminwest/kota-db/compare/v0.3.0...v0.3.1

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/jayminwest/kota-db/compare/v0.3.1...v0.4.0

[Unreleased]: https://github.com/jayminwest/kota-db/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/jayminwest/kota-db/compare/v0.4.0...v0.5.0
