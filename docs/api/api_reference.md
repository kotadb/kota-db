# KotaDB API Reference

KotaDB exposes a shared services layer across the Rust library, HTTP routers, MCP bridge, and CLI. This reference maps each interface to the exact structs, functions, and background workers that execute requests inside the current codebase.

## Step 1 – Review Core Library Surfaces

The root crate re-exports the building blocks you use from applications or integrations.

| Component | Purpose | Source |
| --- | --- | --- |
| `DocumentBuilder`, `create_file_storage`, validated ID types | Construct and persist documents with guarded storage and indices | `src/lib.rs:96-122`
| `create_services_server`, `start_services_server`, SaaS variants | Launch the Services HTTP router used by the CLI and binaries | `src/lib.rs:181-184`
| `Database::new` | Materialises wrapped storage, primary index, and trigram index, implementing `DatabaseAccess` for all services | `src/database.rs:20-114`
| `SearchService`, `IndexingService`, `AnalysisService`, `StatsService`, `BenchmarkService`, `ValidationService` | Unified business logic reused by HTTP handlers, MCP tools, and CLI commands | `src/services/mod.rs:20-53`

The `Database` abstraction hands out cloned `Arc<Mutex<dyn Storage>>` and index handles, so every interface constructs services the same way (`src/database.rs:82-111`). That parity is why the HTTP server simply instantiates a `Database` per request before delegating to the service layer.

> **Note** Feature-gated capabilities (symbols, relationship analysis, MCP tooling) require building with `--features "tree-sitter-parsing,mcp-server"`, as indicated by the `#[cfg]` blocks in `src/lib.rs:60-88` and throughout the services modules.

## Step 2 – Start the Services HTTP API

1. **Local development** – run `kotadb serve --port 8080` to build the router produced by `Commands::Serve` (`src/main.rs:1606-1631`). The CLI wires the `Database` from your `--db-path` into `start_services_server` so you get the same v1 routes the SaaS binary exposes.
2. **Managed/SaaS mode** – run `cargo run --bin kotadb-api-server -- --data-dir ./data --port 8080 --database-url ...` to execute the Clap-driven bootstrap in `src/bin/kotadb-api-server.rs:17-157`. After wiring storage and indices, it validates PostgreSQL connectivity through `test_database_connection` (`src/lib.rs:130-157`) and then calls `start_services_saas_server`.
3. **Programmatic embedding** – call `create_services_server` directly, passing the arcs you obtain from `Database::new`. The router declaration at `src/services_http_server.rs:542-586` shows every route and middleware applied to the state structure defined in `src/services_http_server.rs:45-109`.

The Services state tracks feature flags such as `allow_local_path_ingestion` (`src/services_http_server.rs:98-117`), which disallows absolute filesystem paths when `ALLOW_LOCAL_PATH_INDEXING=0` (important for SaaS).

> **Warning** SaaS deployments must provide `DATABASE_URL`, `DEFAULT_RATE_LIMIT`, and `KOTADB_WEBHOOK_BASE_URL`; the SaaS router refuses to start without them (`src/bin/kotadb-api-server.rs:28-90` and `src/services_http_server.rs:705-714`).

## Step 3 – Call REST v1 Endpoints

The HTTP layer is intentionally thin: each handler builds a service with the current state and defers to the shared logic. Use the tables below to map endpoints to runtime behaviour.

### Health, Metrics, and Validation

| Endpoint | Method | Handler → Service | Runtime behaviour |
| --- | --- | --- | --- |
| `/health` | GET | `health_check` → none | Reports enabled services and optional SaaS diagnostics (`src/services_http_server.rs:920-938`). |
| `/api/v1/health-check` | GET | `health_check_detailed` → `ValidationService::validate_database` | Executes an integrity and consistency scan with default options (`src/services_http_server.rs:2968-3039`, `src/services/validation_service.rs:356-420`). |
| `/api/v1/analysis/stats` | GET | `get_stats` → `StatsService::get_statistics` | Aggregates document, symbol, and relationship metrics, honouring `?basic=`, `?symbols=`, and `?relationships=` (`src/services_http_server.rs:943-994`, `src/services/stats_service.rs:245-280`). |
| `/api/v1/benchmark` | POST | `run_benchmark` → `BenchmarkService::run_benchmark` | Accepts JSON matching `BenchmarkRequest` and streams configuration into the benchmarking engine (`src/services_http_server.rs:996-1049`, `src/services/benchmark_service.rs:300-385`). |
| `/api/v1/validate` | POST | `validate_database` → `ValidationService::validate_database` | Runs the requested integrity/consistency checks; request schema is `ValidationRequest` (`src/services_http_server.rs:2869-3013`). |

### Search and Symbol Navigation

| Endpoint | Method(s) | Handler → Service | Key request fields |
| --- | --- | --- | --- |
| `/api/v1/search/code` | GET/POST | `search_code_enhanced` / `search_code_v1_post` → `SearchService::search_content` | `query`, optional `limit`, `search_type`, `format` (`src/services_http_server.rs:3174-3220`, `src/services/search_service.rs:117-171`). |
| `/api/v1/search/symbols` | GET/POST | `search_symbols_enhanced` / `search_symbols_v1_post` → `SearchService::search_symbols` | `pattern`, `limit`, optional `symbol_type`, `format` (`src/services_http_server.rs:3225-3306`, `src/services/search_service.rs:175-210`). |
| `/api/v1/symbols/:symbol/callers` | GET | `find_callers_v1_get` → `AnalysisService::find_callers` | Optional `?limit=` for caller count (`src/services_http_server.rs:1170-1207`, `src/services/analysis_service.rs:252-282`). |
| `/api/v1/symbols/:symbol/impact` | GET | `analyze_impact_v1_get` → `AnalysisService::analyze_impact` | Optional `?limit=` for impact rows (`src/services_http_server.rs:1210-1248`, `src/services/analysis_service.rs:284-314`). |
| `/api/v1/symbols` | GET | `list_symbols_v1` → `SearchService::search_symbols` | Supports `?pattern=*`, `?limit=50`, `?symbol_type=` (`src/services_http_server.rs:1269-1293`). |
| `/api/v1/files/symbols/*path` | GET | `file_symbols_v1` → binary symbol reader | Streams symbol metadata for a file when `symbols.kota` exists (`src/services_http_server.rs:1296-1327`). |
| `/api/v1/codebase-overview` | GET | `codebase_overview` → `AnalysisService::generate_overview` | `format=json|human`, `top_symbols_limit`, `entry_points_limit` (`src/services_http_server.rs:3116-3165`, `src/services/analysis_service.rs:317-396`). |

Responses honour the `format` parameter via `render_search_code_response` and `render_symbol_search_response`, while the analysis handlers reuse `format_callers_as_cli`, `format_impact_as_cli`, and their simple extractors to mirror CLI output when requested (`src/services_http_server.rs:3445-3626`).

### Repository Lifecycle and Jobs

| Endpoint | Method | Handler → Service | Behaviour |
| --- | --- | --- | --- |
| `/api/v1/repositories` | POST | `register_repository_v1` → background `IndexingService::index_codebase` | Accepts `RegisterRepositoryRequest` for local path ingestion, queues a job, and persists a repository record (`src/services_http_server.rs:1331-1499`, `src/services/indexing_service.rs:149-245`). |
| `/api/v1/repositories` | GET | `list_repositories_v1` | Returns cached repository metadata maintained under `db_path/repositories.json` (`src/services_http_server.rs:1511-1568`). |
| `/api/v1/index/status` | GET | `index_status_v1` | Resolves job state from in-memory queues or Supabase via `index_status_saas` (`src/services_http_server.rs:2717-2787`). |
| `/api/v1/index-codebase` | POST | `index_codebase` → `IndexingService::index_codebase` | Direct indexing for an already-mounted repository; respects `include_files`, `include_commits`, and `extract_symbols` (`src/services_http_server.rs:3044-3113`). |
| `/api/v1/find-callers` | POST | `find_callers_enhanced` → `AnalysisService::find_callers` | Body matches `CallersRequest` with richer format options (`src/services_http_server.rs:3308-3385`). |
| `/api/v1/analyze-impact` | POST | `analyze_impact_enhanced` → `AnalysisService::analyze_impact` | Accepts `ImpactAnalysisRequest`; returns CLI, simple, or rich payloads (`src/services_http_server.rs:3387-3438`). |

> **Note** Setting `ALLOW_LOCAL_PATH_INDEXING=0` disables direct filesystem ingestion for `/api/v1/repositories` and `/api/v1/index-codebase`, returning `local_path_indexing_disabled` (`src/services_http_server.rs:3048-3056`).

### SaaS-Only Webhooks and API Keys

| Endpoint | Method | Handler → Service | Behaviour |
| --- | --- | --- | --- |
| `/webhooks/github/:repository_id` | POST | `handle_github_webhook` → `SupabaseRepositoryStore` job orchestration | Validates GitHub signatures and enqueues incremental indexing via Supabase (`src/services_http_server.rs:1707-1787`). |
| `/internal/create-api-key` | POST | `create_api_key_handler` → `ApiKeyService::create_api_key` | Requires internal auth middleware; persists hashed keys in PostgreSQL (`src/services_http_server.rs:3630-3667`, `src/api_keys.rs:124-205`). |

When SaaS mode is enabled, `create_services_saas_server` spins up the Supabase worker that drains repository jobs (`src/services_http_server.rs:728-743`) and enforces API key auth on every v1 route via `auth_middleware` (`src/services_http_server.rs:745-774`).

## Step 4 – Use MCP Tools

The MCP-over-HTTP bridge mounts alongside the services router when the `mcp-server` feature is active. `create_mcp_bridge_router` defines tool discovery and invocation routes (`src/mcp_http_bridge.rs:53-83`).

| MCP tool | HTTP path | Handler | Downstream service |
| --- | --- | --- | --- |
| `kotadb://text_search` | `POST /mcp/tools/text_search` (dynamic `:tool_name` route) | `TextSearchTools::handle_call` streams trigram queries and loads docs from storage (`src/mcp/tools/text_search_tools.rs:27-102`). | Direct `Index::search` followed by document hydration. |
| `kotadb://symbol_search` | `POST /mcp/tools/search_symbols` | `SymbolTools::handle_call` builds a transient `SearchService` to return structured symbol matches (`src/mcp/tools/symbol_tools.rs:41-90`). | `SearchService::search_symbols`. |
| `kotadb://semantic_search`, `kotadb://hybrid_search`, `kotadb://find_similar` | `POST /mcp/tools/:tool_name` (`tool_name` = `semantic_search`, `hybrid_search`, `find_similar`) | `SearchTools::handle_call` orchestrates trigram, vector, and LLM search paths (`src/mcp/tools/search_tools.rs:18-164`). | Combines `SemanticSearchEngine` with trigram results for richer context. |
| `kotadb://find_callers`, `kotadb://impact_analysis`, `kotadb://codebase_overview` | `POST /mcp/tools/find_callers`, `POST /mcp/tools/analyze_impact`, `POST /mcp/tools/codebase_overview` | `RelationshipTools::handle_call` maps payloads into `AnalysisService` queries (`src/mcp/tools/relationship_tools.rs:1-220`). | Executes the same relationship queries exposed over HTTP.

Tool metadata returned by `/mcp/tools` is either generated from a live `MCPToolRegistry` or falls back to the static list defined in `src/mcp_http_bridge.rs:89-123`.

## Step 5 – Automate via CLI Commands

CLI subcommands call the identical services used by HTTP handlers, so scripting around `kotadb` mirrors API behaviour.

| Command | Implementation | Downstream service |
| --- | --- | --- |
| `kotadb serve` | `Commands::Serve` dispatches to `start_services_server` (`src/main.rs:1606-1631`). | Exposes the HTTP API locally. |
| `kotadb search-code` | Builds `SearchService` with `SearchOptions` and prints results (`src/main.rs:1635-1707`). | `SearchService::search_content` (`src/services/search_service.rs:117-171`). |
| `kotadb search-symbols` *(tree-sitter only)* | Uses `SearchService::search_symbols` with pattern, limit, and type filters (`src/main.rs:1867-1904`). | `SearchService::search_symbols` (`src/services/search_service.rs:175-210`). |
| `kotadb find-callers` / `kotadb analyze-impact` *(tree-sitter only)* | Instantiate `AnalysisService` and call `find_callers` or `analyze_impact` (`src/main.rs:1906-2017`). | `AnalysisService::find_callers` / `analyze_impact` (`src/services/analysis_service.rs:252-314`). |
| `kotadb benchmark` | Wraps `BenchmarkService::run_benchmark` with CLI flags (`src/main.rs:229-305`, `src/main.rs:2019-2103`). | `BenchmarkService` (`src/services/benchmark_service.rs:300-385`). |
| `kotadb index-codebase` *(git-integration only)* | Delegates to `IndexingService::index_codebase` with CLI-controlled options (`src/main.rs:328-452`, `src/main.rs:2105-2254`). | `IndexingService::index_codebase` (`src/services/indexing_service.rs:149-245`). |
| `kotadb codebase-overview` *(tree-sitter only)* | Calls `AnalysisService::generate_overview` and prints formatted output (`src/main.rs:226-327`, `src/main.rs:2256-2342`). | `AnalysisService::generate_overview` (`src/services/analysis_service.rs:317-396`). |

Because the CLI mirrors the HTTP surface, you can develop contract tests locally by invoking `kotadb` commands and then calling the HTTP routes to verify parity.

## Step 6 – Manage SaaS Integrations

SaaS deployments extend the base services state with API key validation and Supabase-backed job orchestration.

- `ApiKeyService::new` establishes the PostgreSQL pool used for key storage and quota enforcement (`src/api_keys.rs:124-140`).
- `create_services_saas_server` attaches authentication middleware and starts the Supabase job worker to drain repository ingestion tasks (`src/services_http_server.rs:705-743`).
- Repository jobs are stored in-memory locally (`state.jobs`) and mirrored to Supabase via `SupabaseRepositoryStore` when `saas_mode` is true (`src/services_http_server.rs:2717-2787`).
- GitHub webhook processing validates HMAC signatures before queueing incremental updates (`src/services_http_server.rs:1707-1787`).

> **Note** The SaaS worker expects Supabase tables matching `SupabaseRepositoryStore`; ensure migrations are applied before starting the binary (see `src/supabase_repository/job_worker.rs`).

## Next Steps

- Run `just dev` to hot-reload the MCP server against the HTTP API for local testing.
- Exercise critical routes with `just test` or targeted `cargo nextest run` suites to confirm behaviour when you modify services.
- Capture example requests/responses for your API consumers and link them back to the handler references above.
