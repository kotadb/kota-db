# Getting Started with KotaDB

KotaDB boots a local knowledge base by constructing a `Database` that wires file-backed storage and dual indices through `Database::new` (`src/database.rs:32`). The `serve` subcommand then exposes those components over HTTP via `start_services_server` (`src/services_http_server.rs:628`), giving you CLI and API access to search, indexing, and analysis services out of the box.

## Step 1: Install Prerequisites
- Install the Rust toolchain with `rustup`; the project tracks stable and enables `rustfmt`/`clippy` via `rust-toolchain.toml`.
- Add `just` for repeatable workflows—targets such as `just dev` and `just test-fast` are defined in the repository `justfile:15` and `justfile:40`.
- Confirm `git`, `cargo`, and `docker` (optional) are on your PATH if you plan to mirror production packaging.

```bash
rustup component add rustfmt clippy
brew install just            # macOS
apt-get install just -y      # Debian/Ubuntu
```

> **Note** Ensure `cargo nextest` is installed (`cargo install cargo-nextest`) before running `just test` (`justfile:35`), otherwise the task will exit early.

## Step 2: Bootstrap the Workspace
1. Clone the repository and enter the workspace directory.
2. Run the fast gating checks so your local environment mirrors CI.

```bash
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
just test-fast
```

`just test-fast` executes `cargo nextest` with the required feature flags `git-integration`, `tree-sitter-parsing`, and `mcp-server` (`justfile:40`), matching the defaults declared in `Cargo.toml`.

## Step 3: Initialize a Data Directory
The CLI stores all data beneath the `--db-path` argument (default `./kota-db-data`) defined on the root `Cli` struct (`src/main.rs:78`). When you launch any command, `Database::new` creates the `storage`, `primary_index`, and `trigram_index` trees in that directory (`src/database.rs:39-94`).

```bash
export KOTADB_DATA_DIR="$PWD/.kota-local"
mkdir -p "$KOTADB_DATA_DIR"
cargo run --bin kotadb -- --db-path "$KOTADB_DATA_DIR" stats
```

The `stats` subcommand exercises the same storage pipeline the services use by invoking `StatsService::get_statistics` through the CLI dispatcher (`src/main.rs:1668-1684`).

## Step 4: Start the Services Server
Run the HTTP server from the CLI. The `Serve` subcommand is implemented in `src/main.rs:93-97` and ultimately calls `start_services_server` (`src/services_http_server.rs:628`) with the storage and index handles you prepared in Step 3.

```bash
cargo run --bin kotadb -- --db-path "$KOTADB_DATA_DIR" serve --port 8080
```

The router created in `create_services_server` exposes health checks and versioned APIs (`src/services_http_server.rs:563-588`). Verify the process by hitting the health endpoint:

```bash
curl http://localhost:8080/health
```

> **Warning** If the port is already taken, the bind helper surfaces remediation tips before exiting (`src/services_http_server.rs:638-660`).

## Step 5: Index Your First Repository
With the default feature set (`Cargo.toml`), the CLI includes the `index-codebase` command guarded by `#[cfg(feature = "git-integration")]` (`src/main.rs:145-159`). It delegates to `IndexingService::index_codebase` (`src/services/indexing_service.rs:145-198`) to traverse repositories, extract symbols, and persist them.

```bash
cargo run --bin kotadb -- --db-path "$KOTADB_DATA_DIR" index-codebase ./my-project --prefix repos/my-project
```

For remote automation or UI backends, call the HTTP endpoint wired to the same service (`src/services_http_server.rs:3044-3088`):

```bash
curl -X POST http://localhost:8080/api/v1/index-codebase \
  -H 'Content-Type: application/json' \
  -d '{"repo_path":"/absolute/path/to/my-project","prefix":"repos/my-project"}'
```

> **Note** Managed deployments can disable local path ingestion; the handler returns `403` when `allow_local_path_ingestion` is false (`src/services_http_server.rs:3048-3056`).

## Step 6: Search and Inspect Results
Content search is served by `SearchService::search_content`, which chooses between regular and LLM-optimized retrieval based on query context (`src/services/search_service.rs:117-172`). The CLI `search-code` subcommand shares this path.

```bash
cargo run --bin kotadb -- --db-path "$KOTADB_DATA_DIR" search-code 'DocumentBuilder' --limit 5 --context medium
```

All HTTP callers hit the same logic through `GET /api/v1/search/code`, which validates input and renders responses in `search_code_enhanced` (`src/services_http_server.rs:3174-3219`).

```bash
curl 'http://localhost:8080/api/v1/search/code?query=DocumentBuilder&limit=5&search_type=medium'
```

For symbol-aware workflows, query `GET /api/v1/search/symbols`, which uses `SearchService::search_symbols` to read the binary symbol store (`src/services/search_service.rs:174-195`).

## Step 7: Monitor Health and Metrics
KotaDB surfaces quick diagnostics via the services API. `GET /api/v1/analysis/stats` drives `StatsService::get_statistics` just like the CLI (`src/services_http_server.rs:565` and `src/services_http_server.rs:943-979`). For richer context, `GET /api/v1/codebase-overview` invokes `AnalysisService::generate_overview` (`src/services_http_server.rs:3116-3165`).

```bash
curl http://localhost:8080/api/v1/analysis/stats | jq
curl 'http://localhost:8080/api/v1/codebase-overview?format=json&top_symbols_limit=10' | jq
```

These endpoints reuse the same `Database` adapter that wraps your storage and index handles (`src/services_http_server.rs:949-954` for stats and `src/services_http_server.rs:3123-3194` for analysis), ensuring CLI and HTTP behavior stay consistent.

## Next Steps
- Explore CLI deep dives in `docs/development-guides/cli_usage.md`.
- Review architecture internals starting at `../architecture/index.md`.
- Learn how services stitch together for production in `../SERVICES_ARCHITECTURE.md`.
- If you plan to deploy the SaaS binary with API keys, read `../FLY_DEPLOYMENT.md` for infrastructure guidance.
