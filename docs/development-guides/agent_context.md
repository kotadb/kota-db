# Agent Context: KotaDB Standalone Development

## Summary
KotaDB ships a single services layer that powers the CLI, HTTP API, and MCP server; agents work against the same abstractions that production code uses. This guide maps those runtime boundaries so you can safely script database operations, extend indexing, or expose new interfaces without diverging from `src/`.

## Step 1 — Align With Runtime Entry Points
The CLI in `src/main.rs:31` drives every developer workflow. Commands such as `index-codebase`, `search-code`, and `serve` are conditionally compiled behind feature flags (`git-integration`, `tree-sitter-parsing`, `mcp-server`) and all share a lazily constructed `Database` handle (`src/main.rs:1177`). Inspect the subcommand wiring at `src/main.rs:146` to confirm which surfaces are enabled for your target build.
> **Note**: `Commands::IndexCodebase` only exists when `--features "git-integration"` is present; `just test-fast` enables it automatically.
The same command definitions are reused by the `run_standalone.sh` helper, which invokes `cargo` subcommands directly (`run_standalone.sh:90`). Use `./run_standalone.sh status` when you need an environment sanity check without touching the broader workspace.

## Step 2 — Compose the Database Adapter
All services expect an implementation of `DatabaseAccess` (`src/services/search_service.rs:20`). The production adapter lives in `src/database.rs:24`; it wires file-backed storage, a primary path index, and a trigram content index, and exposes shared caches for path lookups. `Database::new` ensures directory scaffolding and index selection (`src/database.rs:38`) before wrapping storage with safety middleware via `create_wrapped_storage` (`src/database.rs:88`).
When the CLI boots, it mirrors the same construction flow (`src/main.rs:300`), so any agent code should call into the existing constructor rather than rebuilding storage primitives. After wide writes—such as git ingestion—trigger a rebuild using `Database::rebuild_indices` (`src/main.rs:365`) to rescan storage and repopulate both indices in batches of 100.

## Step 3 — Run the Indexing Pipeline
`IndexingService<'_>` is the canonical entry for repository ingestion (`src/services/indexing_service.rs:134`). The `index_codebase` method validates paths, configures memory limits, spawns a `RepositoryIngester` (`src/services/indexing_service.rs:239`), and delegates to binary symbol and relationship extraction when the `tree-sitter-parsing` feature is compiled (`src/services/indexing_service.rs:264`). Once ingestion completes, it flushes storage buffers and rebuilds indices so search remains consistent (`src/services/indexing_service.rs:360`).

| Field | Source | Runtime Notes |
| --- | --- | --- |
| `repo_path: PathBuf` | `src/services/indexing_service.rs:16` | Absolute repository root that must exist before ingestion.
| `max_memory_mb: Option<u64>` | `src/services/indexing_service.rs:39` | Passed to `crate::memory::MemoryLimitsConfig` to enforce adaptive chunking.
| `extract_symbols: Option<bool>` | `src/services/indexing_service.rs:24` | Honored only when `tree-sitter-parsing` is enabled; otherwise ignored.
| `create_index: bool` | `src/services/indexing_service.rs:28` | Controls whether `RepositoryIngester` writes repository metadata documents.

Successful runs return `IndexResult` with document counts and formatted output (`src/services/indexing_service.rs:97`). When you need lower-level hooks, reuse `RepositoryIngester::ingest_with_binary_symbols_and_relationships` to guarantee relationship graph hydration (`src/services/indexing_service.rs:269`) and inspect the ingestion internals in `src/git/ingestion.rs:62`.

## Step 4 — Orchestrate Search and Analysis
`SearchService` holds all query flows (`src/services/search_service.rs:102`). `SearchService::search_content` chooses between regular trigram search and LLM-optimized search depending on the context level and query pattern (`src/services/search_service.rs:117`). Content reads come from the trigram index populated during indexing; wildcard queries fall back to the primary index (`src/services/search_service.rs:167`).

| Field | Source | Runtime Notes |
| --- | --- | --- |
| `query: String` | `src/services/search_service.rs:30` | Empty strings short-circuit to no-op results to keep CLI parity.
| `context: String` | `src/services/search_service.rs:33` | `medium`/`full` trigger LLM optimization (`src/services/search_service.rs:131`).
| `tags: Option<Vec<String>>` | `src/services/search_service.rs:32` | Forwarded to `regular_search` for tag-filtered lookups (`src/services/search_service.rs:145`).

Symbol search reuses the same service but requires the `symbols.kota` binary database (`src/services/search_service.rs:176`). Analysis endpoints such as impact reports and caller graphs use the `AnalysisServiceDatabase` slice of the same adapter (`src/database.rs:126`), so you can safely reuse the database handle across search, analysis, and indexing tasks without additional locks.

## Step 5 — Surface Services Over HTTP and MCP
The services-only HTTP server binds the same adapters through `ServicesAppState` (`src/services_http_server.rs:61`), layering optional SaaS helpers like API keys and Supabase job tracking (`src/services_http_server.rs:68`). Routes mount `SearchService`, `IndexingService`, and validation hooks via Axum routers (`src/services_http_server.rs:48`), ensuring interface parity with the CLI.
For MCP integrations, `just dev` launches `mcp_server` with hot reload (`justfile:15`), which in turn registers tools discovered in `src/mcp/tools` and shares the `DatabaseAccess` implementation through the bridge (`src/services_http_server.rs:51`). Keep feature flags aligned—`cargo run --bin mcp_server --features mcp-server` mirrors the same configuration the justfile uses.

## Step 6 — Validate and Observe Your Changes
Tests default to `cargo nextest`, so use `just test-fast` to mirror CI with the necessary features toggled (`justfile:40`). When you need full coverage, `just coverage` emits HTML reports under `target/llvm-cov/html/index.html` (`justfile:65`). All binaries should call `init_logging_with_level` to respect `--verbosity` semantics and quiet mode (`src/observability.rs:27`); reuse this helper when crafting standalone scripts.
> **Warning**: Skipping the storage flush that `IndexingService::index_codebase` performs can leave small repositories unindexed (`src/services/indexing_service.rs:360`). Always flush before tearing down your adapter in tests or demos.

## Next Steps
- Spin up the MCP development server with `just dev` and trace index requests through `src/services/indexing_service.rs:149` to confirm feature-flag coverage.
- Extend `IndexResult` in `src/services/indexing_service.rs:97` if your agent needs richer telemetry, then validate the output format via `just test-fast`.
- Cross-reference this guide with `docs/architecture/technical_architecture.md` before introducing new pipelines to ensure consistency with system-level data flows.
