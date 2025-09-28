# Architecture Overview
KotaDB layers file-backed storage, index services, and API surfaces into a composable runtime so each interface (CLI, HTTP, MCP) exercises the same code paths. This guide walks through the boot sequence, service wiring, and background jobs that keep repositories searchable and synchronized.

## Step 1: Initialize Observability and Runtime Configuration
Startup binaries such as the SaaS server first configure logging, parse CLI flags, and probe external dependencies. The API entry point sets up tracing with `tracing_subscriber` and environment-aware filters (`src/bin/kotadb-api-server.rs:52`) before building storage and index handles. Shared helpers like `init_logging` (`src/observability.rs:22`) and `test_database_connection` (`src/lib.rs:131`) enforce Stage 4 observability and verify PostgreSQL connectivity for API-key enforcement.

> **Note** Feature flags influence which subsystems boot: enabling `mcp-server` adds MCP tool routing (`src/services_http_server.rs:776`), while `tree-sitter-parsing` unlocks symbol extraction during indexing (`src/services/indexing_service.rs:175`).

## Step 2: Assemble Storage and Index Foundations
KotaDB consolidates persistence concerns inside the `Database` facade (`src/database.rs:24`). `Database::new` (`src/database.rs:38`) creates the filesystem layout, wires `create_file_storage`, `create_primary_index`, and either `create_binary_trigram_index` or `create_trigram_index` depending on binary-search configuration. The struct holds `Arc<Mutex<dyn Storage>>` and `Arc<Mutex<dyn Index>>` handles plus a `path_cache` for fast lookups (`src/database.rs:24-30`), exposing them through the `DatabaseAccess` trait (`src/database.rs:107`). Wrappers from `create_wrapped_storage` apply caching and flushing guarantees so higher layers can assume durability without micromanaging syncs.

## Step 3: Wire the Services Layer
The services module reuses the same database contract for every interface (`src/services/mod.rs:1`). Each service encapsulates a slice of business logic and accepts a `DatabaseAccess` implementor so callers can swap in the unified `Database` or test doubles. Key services include:

| Service | Source | Primary responsibilities |
| --- | --- | --- |
| `IndexingService` | `src/services/indexing_service.rs:134` | Coordinates repository ingestion, symbol extraction, and index rebuilds |
| `SearchService` | `src/services/search_service.rs:103` | Executes LLM-assisted, trigram, and wildcard searches against shared storage |
| `StatsService` | `src/services/stats_service.rs:230` | Aggregates health metrics and index status for monitoring endpoints |
| `ValidationService` | `src/services/validation_service.rs:341` | Runs consistency checks and repair routines on stored documents |

All services hang off the same `DatabaseAccess` trait (`src/services/search_service.rs:20`), guaranteeing consistent access to storage, trigram, and primary indexes regardless of caller.

## Step 4: Serve Requests Through HTTP and Tooling Bridges
`ServicesAppState` encapsulates the runtime wiring for HTTP handlers, including optional SaaS integrations (`src/services_http_server.rs:62`). It exposes mutex-protected storage and indices, API-key services, Supabase pools, and job registries. The state layout is summarized below:

| Field | Source | Purpose |
| --- | --- | --- |
| `storage` | `src/services_http_server.rs:63` | Shared document store handle used by every service endpoint |
| `primary_index` | `src/services_http_server.rs:64` | Path-based index for wildcard queries and repository listings |
| `trigram_index` | `src/services_http_server.rs:65` | Content index backing full-text search and LLM contexts |
| `api_key_service` | `src/services_http_server.rs:68` | Optional SaaS authentication and quota enforcement |
| `supabase_pool` | `src/services_http_server.rs:70` | Connection pool for Supabase-backed job orchestration |
| `jobs` | `src/services_http_server.rs:76` | In-memory tracker for long-running indexing tasks |

`create_services_saas_server` wires authenticated and public routers with Axum, enforcing API keys on service routes and wiring MCP tool registries when enabled (`src/services_http_server.rs:694-801`). `start_services_saas_server` then binds the listener, logs available endpoints, and serves the composed router (`src/services_http_server.rs:834`). CLI flows call the same services directly, while `just dev` launches the MCP server so IDE tooling can reuse the HTTP service layer.

## Step 5: Execute Repository Indexing Pipelines
`IndexingService::index_codebase` orchestrates end-to-end ingestion (`src/services/indexing_service.rs:149`). It validates repository paths, configures memory limits, and builds a `RepositoryIngester` with `IngestionConfig` tuned by CLI flags (`src/services/indexing_service.rs:222` and `src/git/ingestion.rs:36`). During ingestion the service locks storage, streams git contents, and optionally extracts symbols into binary stores when `tree-sitter-parsing` is enabled (`src/services/indexing_service.rs:262-276`). After documents land in storage, it flushes buffered writes (`src/services/indexing_service.rs:360-369`) and rebuilds both primary and trigram indexes in batches to limit lock contention (`src/services/indexing_service.rs:385-476`). Result structs capture counts and durations so callers can surface progress to users (`src/services/indexing_service.rs:95-130`).

## Step 6: Answer Search and Analysis Queries
`SearchService::search_content` applies the same decision tree for every interface (`src/services/search_service.rs:117`). It falls back to regular trigram or primary-index searches when LLM-enhanced search fails or when wildcard queries are detected, using `regular_search` to route queries to the correct index and hydrate documents (`src/services/search_service.rs:299-360`). When symbols are available, `search_symbols` loads the binary symbol database and filters matches with de-duplication logic (`src/services/search_service.rs:174-200`). The service also exposes LLM context generation via `LLMSearchEngine::search_optimized`, using shared storage and trigram handles (`src/services/search_service.rs:280-295`), ensuring search results and generated summaries align across CLI, MCP, and HTTP consumers.

## Step 7: Orchestrate Background Jobs and Supabase Integrations
SaaS mode spawns a `SupabaseJobWorker` that polls for indexing jobs and drives the same service methods as interactive users (`src/services_http_server.rs:728-743`). The worker reconciles payloads, merges repository settings, and invokes `IndexingService` with incremental plans when webhooks provide change sets (`src/supabase_repository/job_worker.rs:217-282`). Failures are recorded back into Supabase and webhook delivery tables (`src/supabase_repository/job_worker.rs:160-205`), keeping job status endpoints in sync with actual work. By reusing `DatabaseAccess`, background ingestion, webhook responses, and API-triggered indexing share the same batching and flush logic described earlier.

## Next Steps
- Run `just dev` to exercise the MCP server against the services layer during local development.
- Review storage and persistence details in [File Storage Implementation](filestorage_implementation.md) to complement this overview.
- Explore feature-specific docs (e.g., symbol parsing) before enabling `--features tree-sitter-parsing` in production builds.
