# Services Architecture

KotaDB centralizes search, analysis, ingestion, and validation logic inside `src/services`, and every interface (CLI, MCP, HTTP) calls those services through small adapter layers. This guide walks the runtime flow, highlights the feature flags that gate advanced capabilities, and calls out the areas that still return stubbed data so you can align docs with the code as it exists today.

## Step 1 — Compose the Services Layer
The `src/services/mod.rs` hub re-exports each service so callers can depend on a single module and share type definitions across interfaces (`src/services/mod.rs:12`–`60`).

| Service | Responsibilities | Location |
| --- | --- | --- |
| `SearchService` | Full-text, wildcard, and LLM-assisted searches over the document store | `src/services/search_service.rs:102` |
| `AnalysisService` | Relationship queries and codebase intelligence backed by binary graphs | `src/services/analysis_service.rs:104` |
| `IndexingService` | Repository ingestion, symbol extraction, and index rebuilds | `src/services/indexing_service.rs:132` |
| `StatsService` | Aggregated document, symbol, and relationship metrics (feature-gated) | `src/services/stats_service.rs:229` |
| `BenchmarkService` | Synthetic load tests exercising storage and search paths | `src/services/benchmark_service.rs:123` |
| `ValidationService` | Post-index validation, integrity checks, and repair scaffolding | `src/services/validation_service.rs:339` |
| `ManagementService` | Transitional façade kept for compatibility during the service split | `src/services/management_service.rs:195` |

> **Note** Feature-heavy services such as analysis, stats, and validation only compile when the `tree-sitter-parsing` feature is enabled, mirroring the conditional blocks in the service implementations and CLI commands.

## Step 2 — Provide Shared Database Handles
Services depend on thin traits so callers can share existing `Database` instances without copying storage internals:

- `DatabaseAccess` exposes `storage`, `primary_index`, `trigram_index`, and a `path_cache` for path lookups (`src/services/search_service.rs:20`–`25`).
- `AnalysisServiceDatabase` narrows the dependency to `storage` for read-heavy analytics workloads (`src/services/analysis_service.rs:23`–`25`).
- The CLI `Database` struct implements both traits by returning its `Arc<Mutex<_>>` handles, keeping concurrency semantics consistent with legacy code (`src/main.rs:494`–`518`).
- The HTTP server reconstructs a `Database` on demand per request before calling into services (`src/services_http_server.rs:1088`–`1105`).
- MCP tools implement `DatabaseAccess` directly on their handler structs so cached handles can be reused inside tool invocations (`src/mcp/tools/symbol_tools.rs:51`–`64`).

This trait-based design lets each interface inject the same storage indices while still honoring async locking guarantees around `tokio::sync::Mutex` and `RwLock` wrappers.

## Step 3 — Execute Searches with `SearchService`
`SearchService` encapsulates the entire search stack, from input sanitization to index selection.

| Method | Signature | Location | Notes |
| --- | --- | --- | --- |
| `search_content` | `pub async fn search_content(&self, options: SearchOptions) -> Result<SearchResult>` | `src/services/search_service.rs:117` | Tries LLM-optimized search when a context tier other than `none` is requested, then falls back to regular search. |
| `search_symbols` | `pub async fn search_symbols(&self, options: SymbolSearchOptions) -> Result<SymbolResult>` | `src/services/search_service.rs:175` | Streams the binary symbols file, applies wildcard filtering, and deduplicates matches. |
| `try_llm_search` | `async fn try_llm_search(&self, options: &SearchOptions) -> Result<LLMSearchResponse>` | `src/services/search_service.rs:255` | Configures `LLMSearchEngine` with per-context token budgets before querying the trigram index. |
| `regular_search` | `async fn regular_search(&self, query: &str, tags: &Option<Vec<String>>, limit: usize) -> Result<(Vec<Document>, usize)>` | `src/services/search_service.rs:298` | Builds queries with `QueryBuilder`, routes wildcard searches to the primary index, and hydrates documents from storage. |

Flow overview:

1. The service short-circuits empty queries to avoid unnecessary lock acquisition (`src/services/search_service.rs:120`–`127`).
2. For non-wildcard inputs it attempts an LLM-assisted search, returning the optimizer payload when available (`src/services/search_service.rs:132`–`140`).
3. On errors or wildcard queries, it calls `regular_search`, which builds a `QueryBuilder`, selects the correct index, and retrieves documents (`src/services/search_service.rs:313`–`362`).
4. Symbol search ensures the binary cache exists before scanning and deduplicating matches (`src/services/search_service.rs:176`–`252`).

Interfaces reuse the same implementation: the CLI wraps the output to maintain prior formatting (`src/main.rs:1880`–`1904`), MCP tools expose it as `kotadb://symbol_search` (`src/mcp/services_tools.rs:360`–`412`), and the HTTP server maps `/v1/search/code` requests to `SearchService::search_content` (`src/services_http_server.rs:1088`–`1115`).

> **Note** LLM-powered queries require the `llm_search` module to be configured; when the engine fails, the service automatically falls back to trigram search so interfaces remain stable (`src/services/search_service.rs:142`–`153`).

## Step 4 — Run Relationship Analysis
`AnalysisService` layers binary relationship data and document metadata to answer higher-level questions about the codebase.

| Method | Signature | Location | Notes |
| --- | --- | --- | --- |
| `find_callers` | `pub async fn find_callers(&mut self, options: CallersOptions) -> Result<CallersResult>` | `src/services/analysis_service.rs:251` | Builds a `FindCallers` query, limits results, and renders markdown summaries. |
| `analyze_impact` | `pub async fn analyze_impact(&mut self, options: ImpactOptions) -> Result<ImpactResult>` | `src/services/analysis_service.rs:284` | Reuses the relationship engine to project downstream impacts. |
| `generate_overview` | `pub async fn generate_overview(&self, options: OverviewOptions) -> Result<OverviewResult>` | `src/services/analysis_service.rs:317` | Aggregates document counts, symbol stats, dependency graph metrics, and file organization details. |

Key runtime steps:

- Relationships are lazy-loaded: the first analysis call builds a `BinaryRelationshipEngine` and errors with actionable indexing instructions when no symbols are present (`src/services/analysis_service.rs:221`–`248`).
- Caller and impact conversions include human-readable context, using helper methods to map relation types to verbs and impact labels (`src/services/analysis_service.rs:125`–`200`).
- `generate_overview` stitches together storage metrics, symbol histograms, dependency graph stats (via `SerializableDependencyGraph`), and heuristics that mark potential entry points (`src/services/analysis_service.rs:321`–`488`).
- File organization statistics rely on helper utilities to classify tests and documentation (`src/services/analysis_service.rs:390`–`535`).

CLI commands such as `find-callers`, `analyze-impact`, and `codebase-overview` simply construct the service and print its markdown (`src/main.rs:1906`–`1999`). Because those subcommands are guarded by `#[cfg(feature = "tree-sitter-parsing")]`, you must enable that feature to expose analysis endpoints.

## Step 5 — Ingest and Rebuild Indexes
`IndexingService` owns the codebase ingestion pipeline, from validating inputs to rebuilding secondary indexes.

| Method | Signature | Location | Notes |
| --- | --- | --- | --- |
| `index_codebase` | `pub async fn index_codebase(&self, options: IndexCodebaseOptions) -> Result<IndexResult>` | `src/services/indexing_service.rs:149` | Performs repository ingestion and index rebuilds. |
| `index_git_repository` | `pub async fn index_git_repository(&self, options: IndexGitOptions) -> Result<GitIndexResult>` | `src/services/indexing_service.rs:530` | Delegates to `index_codebase` until git-specific metrics ship. |
| `incremental_update` | `pub async fn incremental_update(&self, options: IncrementalUpdateOptions) -> Result<UpdateResult>` | `src/services/indexing_service.rs:577` | Stub describing the intended incremental pipeline. |
| `reindex_scope` | `pub async fn reindex_scope(&self, scope_path: &Path, extract_symbols: bool) -> Result<IndexResult>` | `src/services/indexing_service.rs:621` | Placeholder for selective reindexing. |

`index_codebase` workflow:

1. Validates the repository path, returning an early failure result if the path is missing (`src/services/indexing_service.rs:154`–`167`).
2. Configures symbol extraction based on the `tree-sitter-parsing` feature, CLI flags, and `--no-symbols` overrides (`src/services/indexing_service.rs:175`–`198`).
3. Builds memory limits and ingestion options for `RepositoryIngester` (`src/services/indexing_service.rs:203`–`244`).
4. Calls the appropriate ingestion method: full symbol extraction when enabled, otherwise storage-only ingestion (`src/services/indexing_service.rs:263`–`289`).
5. Flushes storage buffers to ensure durability (`src/services/indexing_service.rs:360`–`373`).
6. Rebuilds the primary and trigram indexes in batches, flushing periodically to control memory pressure (`src/services/indexing_service.rs:379`–`505`).

> **Warning** Git history metrics, incremental updates, and scope reindexing are documented but not yet implemented—they return warnings and success=false placeholders so consumers can detect the gap (`src/services/indexing_service.rs:596`–`618`, `src/services/indexing_service.rs:632`–`651`).

`index_codebase` also prints progress snippets unless `quiet` is set, mirroring CLI expectations. The CLI `index-codebase` command simply forwards parsed flags to this method, so documentation should point users here for the authoritative behavior.

## Step 6 — Operate and Validate the Database
Several services expose operational insights; most ship scaffolding with partial implementations that will evolve alongside the code.

- **StatsService**: `get_statistics` toggles basic, symbol, and relationship stats depending on CLI flags (`src/services/stats_service.rs:245`–`296`). Basic metrics come from `storage.list_all()` (document counts and average sizes), while symbol and relationship stats require `tree-sitter-parsing`. Health and performance checks currently return hard-coded values with TODOs (`src/services/stats_service.rs:802`–`835`), so downstream docs should flag that actual validation is pending.
- **BenchmarkService**: `run_benchmark` orchestrates warmups and delegates to storage/index/search benchmarks (`src/services/benchmark_service.rs:300`–`356`). The search benchmark reuses `SearchService` to run alternating content and symbol searches, ensuring the same code paths are profiled (`src/services/benchmark_service.rs:907`–`959`). Output formats include human-readable, JSON, and CSV renderings (`src/services/benchmark_service.rs:367`–`438`).
- **ValidationService**: `validate_database` chains `validate_post_ingestion_search`, optional integrity and consistency checks, and summarizes the outcome (`src/services/validation_service.rs:352`–`443`). Integrity checks probe storage, index, and relationship layers but still rely on stubbed helpers (e.g., `check_storage_integrity`) that must be implemented to provide actionable reports (`src/services/validation_service.rs:445`–`548`).

Whenever you describe these services, make sure to call out the current limitations so users understand that some reports remain aspirational until the TODO blocks are completed.

## Step 7 — Wire Services into Interfaces
Each interface is a thin adapter that transforms user input into service calls:

- **CLI**: Command handlers invoke services and print their formatted output; for example, `search-symbols` and `find-callers` instantiate the appropriate service and render markdown (`src/main.rs:1880`–`1999`).
- **HTTP API**: `services_http_server` rebuilds a temporary `Database` from shared state on each request and routes operations like `/v1/search/code` or `/v1/search/symbols` straight to service methods, wrapping errors in API-friendly responses (`src/services_http_server.rs:1088`–`1160`).
- **MCP Tools**: Tool definitions expose service capabilities to LLM clients; `kotadb://symbol_search` passes directly through to `SearchService::search_symbols`, while other tools map to indexing and management stubs (`src/mcp/services_tools.rs:360`–`440`).

Because all interfaces share the same service methods, behavior stays aligned as long as the services stay the source of truth. When adding a new interface (for example, a GraphQL server), implement `DatabaseAccess`, construct the necessary services, and reuse the existing option structs to guarantee parity.

## Next Steps
- Enable the `tree-sitter-parsing` feature locally before documenting analysis, stats, or validation flows so the referenced code paths compile. 
- Fill in the TODO sections for stats, validation, and incremental indexing before promising those capabilities externally (`src/services/stats_service.rs:802`–`835`, `src/services/validation_service.rs:445`–`618`).
- Exercise the service calls through `just test` or targeted `cargo nextest` runs whenever you update documentation to confirm code references remain valid.
- When introducing new interfaces, mirror the pattern used by `services_http_server` and MCP tools to keep the service layer as the single source of truth.
