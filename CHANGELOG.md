# Changelog

All notable changes to KotaDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
