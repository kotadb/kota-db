# Installation Guide
KotaDB ships as a Rust workspace that exposes CLI, HTTP, and MCP binaries from the same build (`Cargo.toml:193-214`). The default feature set enables git ingestion, tree-sitter parsing, ONNX embeddings, and MCP tooling, so plan for those dependencies or compile with a reduced feature set when targeting constrained environments (`Cargo.toml:155-169`).

## Step 1: Prepare the Environment
1. Install the stable Rust toolchain with the pinned components (`rust-toolchain.toml:1-4`):
   ```bash
   rustup toolchain install stable --component rustfmt --component clippy --component rust-src
   ```
2. Install system libraries required by the dev bootstrap script—OpenSSL headers, SQLite, and build tooling on Linux/macOS (`scripts/dev/dev-setup.sh:42-62`). Windows users should provision these inside WSL2 so the Linux packages resolve correctly.
3. Install the Rust developer utilities used by the project. The bootstrap script installs `cargo-watch`, `cargo-edit`, `cargo-audit`, `cargo-deny`, `cargo-nextest`, `cargo-llvm-cov`, and `bacon` (`scripts/dev/dev-setup.sh:82-93`).
4. Install `just` so you can run the curated tasks such as `just dev`, `just test-fast`, and `just mcp` defined in the recipe file (`justfile:11-42`).

> **Note** Run `just setup` to execute the full bootstrap script whenever you need a fresh workstation (`justfile:11-17`).

## Step 2: Fetch KotaDB
1. Clone the repository and enter it:
   ```bash
   git clone https://github.com/jayminwest/kota-db.git
   cd kota-db
   ```
2. If you are working from an existing checkout, ensure it is clean before compiling so `cargo` does not reuse stale build artifacts.

## Step 3: Build the Binaries
1. Build the optimized artifacts:
   ```bash
   cargo build --release
   ```
   This produces `kotadb`, `kotadb-api-server`, `mcp_server`, and `intent_mcp_server` under `target/release/` (`Cargo.toml:193-214`).
2. To slim dependencies, disable defaults and opt back into the features you need. CI exercises `git-integration`, `tree-sitter-parsing`, and `mcp-server`, which is the same combo exposed by `just test-fast` (`Cargo.toml:155-169`, `justfile:40-42`).

> **Note** Embeddings rely on the ONNX runtime shipped through the `embeddings-onnx` feature; keep it enabled if you plan to run semantic or vector search workloads (`Cargo.toml:165-169`).

## Step 4: Prime Local Storage
1. Choose a data directory (default `./kota-db-data`). The CLI exposes it as `--db-path` and a global flag in the `Cli` definition (`src/main.rs:78-86`).
2. Create the directory before launching the server so permissions are correct:
   ```bash
   mkdir -p ./kota-db-data
   ```
3. When KotaDB boots it calls `Database::new`, which ensures the `storage/`, `primary_index/`, and `trigram_index/` subdirectories exist and wires them into the storage and index implementations (`src/database.rs:32-95`). If you pass `--binary-index=false`, the trigram index falls back to the text implementation in the same constructor (`src/database.rs:64-85`).

## Step 5: Smoke-Test the CLI
1. Confirm the binary starts and reports the embedded version metadata:
   ```bash
   ./target/release/kotadb --version
   ```
   The version is provided by `clap` using the crate metadata embedded at compile time (`src/main.rs:31-57`).
2. Inspect the available subcommands:
   ```bash
   ./target/release/kotadb --help
   ```
   Subcommand definitions live alongside their arguments in the `Commands` enum (`src/main.rs:90-199`).
3. Run a stats query against the empty database:
   ```bash
   ./target/release/kotadb stats --basic --db-path ./kota-db-data
   ```
   This exercises `StatsService::get_statistics`, which is invoked when the CLI dispatches the `Stats` branch (`src/main.rs:1667-1684`).
4. Run the fast pre-flight test suite:
   ```bash
   just test-fast
   ```
   The recipe executes `cargo nextest` with the feature set used in CI and follows up with doctests (`justfile:40-42`).

> **Note** The global `--verbosity` and `--binary-index` flags are parsed in the CLI root struct and control logging and index selection before the command dispatch (`src/main.rs:59-85`).

## Step 6: Expose the Services HTTP Server
1. Launch the services API from the CLI:
   ```bash
   ./target/release/kotadb serve --db-path ./kota-db-data --port 8080
   ```
   The `serve` branch opens the storage via `Database::new` and delegates to `start_services_server` to mount Axum routes (`src/main.rs:1603-1631`).
2. The HTTP layer builds a `ServicesAppState` that shares the storage, primary index, trigram index, and optional API key service across handlers (`src/services_http_server.rs:60-79`).
3. Verify the basic health endpoint:
   ```bash
   curl http://localhost:8080/health
   ```
   The handler responds with enabled service names and optional SaaS status (`src/services_http_server.rs:919-939`).
4. Review the standard endpoints exposed for indexing, search, and analysis. They are logged when the server boots and handled through the corresponding service modules (`src/main.rs:1608-1621`, `src/services_http_server.rs:680-683`).

## Step 7: Enable SaaS Postgres Features (Optional)
1. Provision a PostgreSQL database and collect the connection string and quotas required by `ApiKeyConfig` (`src/api_keys.rs:24-46`).
2. Run the SaaS server binary with the necessary environment:
   ```bash
   ./target/release/kotadb-api-server \
     --data-dir ./kota-db-data \
     --database-url postgresql://USER:PASSWORD@HOST:5432/kotadb \
     --port 8080
   ```
   The binary initializes storage, indices, and the API key service, then verifies connectivity through `kotadb::test_database_connection` before starting `start_services_saas_server` (`src/bin/kotadb-api-server.rs:16-160`, `src/lib.rs:130-157`).
3. When SaaS mode is active, the HTTP stack validates that `KOTADB_WEBHOOK_BASE_URL` and database URLs are set, spawns the Supabase-backed job worker, and shares the API key pool with request handlers (`src/services_http_server.rs:694-743`, `src/services_http_server.rs:2535-2569`).

> **Warning** Missing or empty `DATABASE_URL` and webhook environment variables cause `validate_saas_environment` to abort startup unless you explicitly set `DISABLE_SAAS_ENV_VALIDATION` (`src/services_http_server.rs:2535-2569`).

## Step 8: Container Workflows (Optional)
1. Bring up the development stack:
   ```bash
   docker compose -f docker-compose.dev.yml up --build
   ```
   The `kotadb-dev` service mounts the workspace, exposes ports 8080/8000/9090, and wires the same environment variables expected by the CLI (`docker-compose.dev.yml:7-52`).
2. The `kotadb-mcp` service runs the MCP server binary on port 8484 with a mounted data directory and health check hitting the JSON-RPC endpoint (`docker-compose.dev.yml:54-82`).
3. Optional companions—`docs-server`, `redis-dev`, and `postgres-dev`—provide documentation previews, caching, and a Postgres instance seeded from `scripts/sql/` (`docker-compose.dev.yml:84-138`).

> **Note** Container volumes cache Cargo registries and build artifacts to avoid recompiling dependencies on each restart (`docker-compose.dev.yml:17-33`).

## Next Steps
- [Services Architecture](./SERVICES_ARCHITECTURE.md)
- [Supabase Architecture](./SUPABASE_ARCHITECTURE.md)
- [Developer Onboarding](developer/index.md)
- [MCP Implementations](./MCP_IMPLEMENTATIONS.md)
