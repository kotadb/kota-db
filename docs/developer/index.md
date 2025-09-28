# Developer Guide

KotaDB's developer home base orients you to the shared services layer that unifies the CLI, HTTP API, and MCP server flows defined in `src/services/mod.rs:14`.

## Quick Start Workflow
1. Run `just setup` to install toolchain dependencies and optional helpers; it executes `scripts/dev/dev-setup.sh` (`justfile:11`).
2. Start the auto-reloading MCP server with `just dev`, which wraps `cargo watch -x 'run --bin mcp_server --features mcp-server'` so you exercise the binary defined in `src/bin/mcp_server.rs:15` under the feature gate from `Cargo.toml:155` (`justfile:16`).
3. Prime the database by indexing the current repository via `kotadb index-codebase .`—this CLI subcommand forwards arguments into `Commands::IndexCodebase` (`src/main.rs:145`).
4. Validate code health before committing by running `just test-fast` for feature-gated nextest runs and doctests, then `just fmt`/`just clippy` to match CI expectations (`justfile:41`, `justfile:72`, `justfile:80`).

> **Note** The default feature set enables `mcp-server`, `git-integration`, and `tree-sitter-parsing`, which power the indexing and search flows below (`Cargo.toml:155`).

## Index-to-Query Flow
1. The CLI builds `IndexCodebaseOptions` from user flags and injects them into `IndexingService::index_codebase` so all interfaces reuse the same ingestion logic (`src/main.rs:145`, `src/services/indexing_service.rs:149`).
2. `IndexingService` prepares `IngestionConfig` and opens a `RepositoryIngester`, selecting binary symbol extraction when the tree-sitter feature is present (`src/services/indexing_service.rs:239`, `src/services/indexing_service.rs:263`).
3. `RepositoryIngester::ingest_with_binary_symbols` streams files, commits, symbols, and relationships into storage while reporting progress via callbacks (`src/git/ingestion.rs:169`).
4. The `SearchService::search_content` path later retrieves documents through the trigram or primary index and optionally escalates to LLM-backed results via `try_llm_search` (`src/services/search_service.rs:117`, `src/services/search_service.rs:255`).
5. HTTP requests use the same storage handles through the shared `AppState`, and the router exposes CRUD, search, and stats endpoints via `create_server` (`src/http_server.rs:62`, `src/http_server.rs:260`).

## Testing Checklist
1. Execute `just test` for the full nextest matrix when validating larger changes (`justfile:35`).
2. Generate coverage with `just coverage` and inspect `target/llvm-cov/html/index.html` if you change query or ingestion logic (`justfile:65`).
3. Run `just audit` whenever dependencies shift; it wraps `cargo audit` and `cargo deny check all` for supply-chain safety (`justfile:88`).
4. For performance-sensitive work, profile with `just bench` or `just db-bench` to exercise the Criterion suites compiled behind the `bench` feature (`justfile:114`, `justfile:151`).

## Next Steps
- Deep-dive into service responsibilities in [Services Architecture](../SERVICES_ARCHITECTURE.md).
- Review request routing details in [Architecture Overview](../architecture/index.md).
- Follow the branching and release expectations in [Branching Strategy](../BRANCHING_STRATEGY.md) and [Release Process](../RELEASE_PROCESS.md).
