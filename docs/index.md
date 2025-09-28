# KotaDB Documentation

KotaDB is a Rust code intelligence engine that turns repositories into searchable graphs backed by storage, trigram indices, and optional symbol relationships. This guide surfaces the runtime entry points you will touch most often.

## Step 1: Map the Storage Core
KotaDB's services rely on the unified `Database` abstraction (`src/database.rs:24-95`) which satisfies the `DatabaseAccess` trait all services consume (`src/services/search_service.rs:18-25`). `Database::new` sets up the file-backed storage, primary index, and binary trigram index under the chosen `db_path` (`src/database.rs:38-95`), wrapping storage with metering and caching via `create_wrapped_storage` (`src/database.rs:87-95`). Because the `path_cache` (an `Arc<RwLock<_>>`) is shared across services, cloning `Database` handles is cheap while maintaining consistent lookup semantics. Use `just dev` to boot the auto-reloading MCP server with these defaults.

| Handle | Source | Purpose |
| --- | --- | --- |
| `storage: Arc<Mutex<dyn Storage>>` | `src/database.rs:24-55` | Persists documents in `storage/` with read/write buffering. |
| `primary_index: Arc<Mutex<dyn Index>>` | `src/database.rs:24-63` | Maintains path-based lookups consumed by wildcard search. |
| `trigram_index: Arc<Mutex<dyn Index>>` | `src/database.rs:24-85` | Provides trigram-backed full text search; configured for binary mode when `--binary-index` is true. |
| `path_cache: Arc<RwLock<HashMap<..>>>` | `src/database.rs:28-30` | Lazily caches path-to-document-id resolutions for repeated queries. |

> **Note** SaaS deployments validate state before enabling local ingestion: `ServicesAppState::validate_saas_mode` ensures both API keys and Supabase pools exist (`src/services_http_server.rs:81-91`).

## Step 2: Index a Repository
The CLI `index-codebase` path (gated behind the `git-integration` feature) is wired in the `Commands` enum (`src/main.rs:145-180`) and forwards user input into `IndexingService::index_codebase` (`src/services/indexing_service.rs:149-333`). The options struct records toggles for commits, chunking, symbol extraction, and include filters (`src/services/indexing_service.rs:15-49`), and defaults to storing results under `repos/`.

| Option | Default | Where It Lands |
| --- | --- | --- |
| `prefix` | `"repos"` | Forwarded into `IngestionConfig::path_prefix` (`src/services/indexing_service.rs:239-244`). |
| `include_files` | `true` | Enables blob storage via `include_file_contents` (`src/services/indexing_service.rs:224-230`). |
| `include_commits` | `true` | Preserves git history during ingestion (`src/services/indexing_service.rs:224-236`). |
| `max_file_size_mb` | `10` | Converted to bytes for the ingester (`src/services/indexing_service.rs:224-229`). |
| `extract_symbols` | `Some(true)` | Requires the `tree-sitter-parsing` feature to activate symbol pipelines (`src/services/indexing_service.rs:174-198`). |
| `enable_chunking` | `true` | Controls memory caps in `MemoryLimitsConfig` (`src/services/indexing_service.rs:203-218`). |

Once options are prepared, `RepositoryIngester::ingest_with_binary_symbols_and_relationships` writes documents, binary symbols, and the dependency graph in one pass (`src/services/indexing_service.rs:264-276` delegating to `src/git/ingestion.rs:169-276`). The service always flushes pending writes (`src/services/indexing_service.rs:360-373`) and rebuilds the primary and trigram indices in batches to keep search consistent (`src/services/indexing_service.rs:379-476`).

> **Warning** If `tree-sitter-parsing` is disabled, `should_extract_symbols` falls back to `false`, so symbol and relationship outputs will be skipped without failing the ingestion (`src/services/indexing_service.rs:200-201`).

## Step 3: Query and Analyze
Searching and analysis share the same `DatabaseAccess` handles. `SearchService::search_content` dispatches to an LLM-optimized flow when the query is not a wildcard and the caller requests medium/full context (`src/services/search_service.rs:117-153`), otherwise falling back to `regular_search` over the trigram index (`src/services/search_service.rs:298-319`). Symbol lookups stream from the binary symbol store via `search_symbols` (`src/services/search_service.rs:175-252`), complete with wildcard matching. The private `try_llm_search` helper binds query options to `LLMSearchEngine::search_optimized` (`src/services/search_service.rs:255-296`), so tuning `context` directly informs token budgets.

Runtime relationship features are centralized in `AnalysisService`. The service lazily initializes a `BinaryRelationshipEngine` against `symbols.kota` and `dependency_graph.bin` (`src/services/analysis_service.rs:221-248`). `find_callers` and `analyze_impact` wrap relationship queries into ergonomic structs and markdown summaries (`src/services/analysis_service.rs:251-315`), while `generate_overview` joins document stats, symbol tallies, and dependency graph metrics for dashboards or AI prompts (`src/services/analysis_service.rs:317-400`). These flows mirror the CLI subcommands surfaced in `Commands` (`src/main.rs:182-255`), ensuring parity between terminal usage and HTTP APIs.

## Step 4: Serve and Automate
The Axum-based services server wires routes onto the same services layer. `ServicesAppState` carries cloned storage/index handles, optional auth, and job registries for SaaS mode (`src/services_http_server.rs:62-79`). `start_services_server` binds the router and logs available endpoints (`src/services_http_server.rs:628-682`), while `create_services_server` conditionally merges MCP tooling when the `mcp-server` feature is active (`src/services_http_server.rs:600-618`). Each HTTP handler reconstructs a lightweight `Database` wrapper before invoking the corresponding service—`index_codebase` is a direct thin wrapper over `IndexingService::index_codebase` (`src/services_http_server.rs:3044-3086`), and `search_code_v1_post` calls `SearchService::search_content` (`src/services_http_server.rs:1063-1104`).

Background ingestion for managed tenants is handled by `SupabaseJobWorker`, which polls Supabase queues, clones repositories, and feeds them into the same indexing service (`src/supabase_repository/job_worker.rs:27-179`). Webhook delivery ids are reconciled after each job, and failures propagate back to Supabase with detailed error payloads (`src/supabase_repository/job_worker.rs:180-199`). Managed deployments also require `KOTADB_WEBHOOK_BASE_URL` for webhook provisioning (`src/services_http_server.rs:710-714`) and can disable on-box ingestion via `ALLOW_LOCAL_PATH_INDEXING` (`src/services_http_server.rs:98-107`).

## Next Steps
- Run `just dev` to start the MCP server and verify endpoints with `curl http://localhost:8080/health`.
- Ingest a sample repository with `cargo run --bin kotadb -- index-codebase ./path/to/repo` and watch the flush/rebuild logs described above.
- Explore API routes and request payloads in `api/index.md` and the detailed service diagrams in `architecture/index.md`.
- Review the Supabase deployment flow in `SUPABASE_ARCHITECTURE.md` before enabling SaaS features.
