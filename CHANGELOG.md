# Changelog

All notable changes to KotaDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
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
