# KotaDB Development Guide

## Summary
KotaDB's development workflow centers on the shared `Database` aggregator (`src/database.rs:24`) and service layer exports (`src/services/mod.rs:14`), letting the CLI, HTTP server, and MCP server reuse the same storage, indexing, and analysis pipelines. This guide walks through the concrete commands and runtime paths you will exercise during day-to-day development.

## Step 1 — Map the Runtime
- `Database::new` bootstraps storage plus primary/trigram indices and wraps them for reuse across interfaces (`src/database.rs:32`).
- The services layer exposes indexing, search, analysis, validation, and stats APIs that all front ends consume (`src/services/mod.rs:14`).
- `ServicesAppState` wires those services into the clean HTTP surface; routes under `/api/v1/*` invoke the same structs the CLI uses (`src/services_http_server.rs:62`).
- The CLI `serve` subcommand spins up that router via `start_services_server` (`src/main.rs:1605`, `src/services_http_server.rs:627`).
- The standalone MCP runtime builds on the same storage/index handles inside `MCPServer::new` (`src/mcp/server.rs:94`).
- Default features (`Cargo.toml:154`) enable `embeddings-onnx`, `git-integration`, `tree-sitter-parsing`, and `mcp-server`; disable them only when you intentionally narrow functionality.

## Step 2 — Prepare Tooling & Features
```bash
just setup
source .env.dev
```
- `just setup` runs `scripts/dev/dev-setup.sh` to install platform dependencies, rustup components, and helper binaries (`justfile:11`, `scripts/dev/dev-setup.sh:45`).
- The script provisions a pre-commit hook that enforces `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --lib` before every commit (`scripts/dev/dev-setup.sh:99`).
- Sourcing `.env.dev` loads `RUST_LOG`, `RUST_BACKTRACE`, and `KOTADB_DATA_DIR` defaults created by the setup script (`scripts/dev/dev-setup.sh:134`).

> **Note**: `cargo-watch` is installed during setup but may be unavailable on minimal environments; the `just watch` task falls back gracefully (`justfile:27`).

## Step 3 — Initialize Workspace Data
- The setup script creates `data/`, `logs/`, `cache/`, and `temp/` directories that back the file storage and indices (`scripts/dev/dev-setup.sh:128`).
- `Database::new` expects those directories and will lazily create sub-folders under `storage/`, `primary_index/`, and `trigram_index/` during startup (`src/database.rs:39`).
- Override the storage root per session with:
  ```bash
  export KOTADB_DATA_DIR=$PWD/.kota-local
  mkdir -p "$KOTADB_DATA_DIR"
  ```
  The CLI automatically passes this path into `Database::new` and child services (`src/main.rs:1602`).

## Step 4 — Run Development Surfaces
```bash
just dev
just mcp
cargo run --bin kotadb serve -- --port 8080
```
- `just dev` watches the tree and re-runs the MCP server with hot reload (`justfile:15`).
- `just mcp` launches the same binary once with verbose logging (`justfile:19`).
- `cargo run --bin kotadb serve` binds the services HTTP server to the requested port, invoking `start_services_server` with shared state (`src/main.rs:1605`).
- Want a cheaper validation loop? Use `just watch` to run `cargo nextest run --lib` and `clippy` on every save (`justfile:27`).

> **Warning**: MCP routes are only compiled when the `mcp-server` feature is active (on by default). If you disable it via `--no-default-features`, `just dev` and `just mcp` will fail.

## Step 5 — Index Repositories and Jobs
- The CLI `index-codebase` subcommand delegates to `IndexingService::index_codebase`, which configures `RepositoryIngester` and orchestrates symbol extraction, storage flushes, and index rebuilds (`src/main.rs:145`, `src/services/indexing_service.rs:149`).
- During ingestion the service applies optional memory caps, toggles symbol extraction, then flushes storage and repopulates indices to keep search consistent (`src/services/indexing_service.rs:203`).
- SaaS flows enqueue the same work via Supabase; `SupabaseJobWorker::process_indexing_job` clones or updates repos, purges removed paths, and ultimately calls the indexing service (`src/supabase_repository/job_worker.rs:277`).
- Run a local ingest against this repository:
  ```bash
  cargo run --bin kotadb index-codebase . --max-file-size-mb 15 --prefix repos/kotadb
  ```
  Watch for the flush and rebuild log lines defined in `IndexingService::index_codebase` (`src/services/indexing_service.rs:360`).

> **Note**: Symbol extraction and relationship graphs require `tree-sitter-parsing`; disable it and `IndexingService` silently skips symbol work while keeping document ingestion healthy (`src/services/indexing_service.rs:175`).

## Step 6 — Search and Analyze the Corpus
- `SearchService::search_content` routes queries to either the trigram or primary index, falling back to LLM-enhanced search when context depth demands it (`src/services/search_service.rs:117`).
- `regular_search` builds a validated query, chooses the correct index, and hydrates documents from storage (`src/services/search_service.rs:298`).
- Analysis endpoints (`find-callers`, `analyze-impact`, `codebase-overview`) run through `AnalysisService`, which loads binary relationship data and formats results (`src/services/analysis_service.rs:104`).
- The HTTP server exposes `/api/v1/search/code`, `/api/v1/symbols`, and related routes that wrap those services (`src/services_http_server.rs:565`).
- Example curl to verify content search parity:
  ```bash
  curl -sS "http://localhost:8080/api/v1/search/code?q=index" | jq '.total_count'
  ```
  Compare against `cargo run --bin kotadb search-code index` for the same dataset.
- For more CLI usage patterns, see [CLI usage](cli_usage.md).

## Step 7 — Validate, Test, and Package
- `just test` runs the full nextest suite with `--no-fail-fast` (`justfile:35`).
- `just test-fast` mirrors the CI gating matrix by pinning features and running doctests afterwards (`justfile:40`).
- Quality gates live behind `just ci-fast`, combining format, lint, unit tests, and `cargo audit` (`justfile:199`).
- Generate coverage with `just coverage` and open `target/llvm-cov/html/index.html` (`justfile:65`).
- Container workflows:
  ```bash
  just docker-up
  just docker-shell
  just docker-test
  ```
  These wrap `scripts/dev/docker-dev.sh` to start the compose stack and execute commands inside `kotadb-dev` (`justfile:180`, `scripts/dev/docker-dev.sh:22`). Ports and dependencies are defined in `docker-compose.dev.yml` (`docker-compose.dev.yml:15`).

## Step 8 — Observe and Debug
- Runtime logs honor `RUST_LOG` and `RUST_BACKTRACE` from `.env.dev`; bump to `trace` when diagnosing ingestion or MCP issues (`scripts/dev/dev-setup.sh:134`).
- The CLI and HTTP paths wrap work in `with_trace_id` so related logs share a trace identifier (`src/main.rs:1601`).
- `start_services_server` prints endpoint summaries on boot and suggests remediation when ports are in use (`src/services_http_server.rs:664`).
- The services HTTP server optionally exposes an MCP bridge when the feature is compiled, so you can test tool calls via HTTP during local runs (`src/services_http_server.rs:596`).
- The Docker compose file exposes port 9090 for metrics and 8000/8001 for documentation previews (`docker-compose.dev.yml:21`).

## Next Steps
- Run `just dev` and exercise `/api/v1/search/code` to confirm your environment streams logs at the expected verbosity.
- Index a fresh repository with `cargo run --bin kotadb index-codebase` and inspect the regenerated indices under `data/`.
- Capture coverage with `just coverage` before large refactors to baseline your diff.
- If you need Supabase parity, start the compose stack via `just docker-up` and monitor `SupabaseJobWorker` logs for job progression.
- Explore [CLI usage](cli_usage.md) for deeper command examples once your environment is live.
