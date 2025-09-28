# KotaDB Services HTTP API

KotaDB exposes the same indexing, search, and analysis services that power the CLI and MCP integrations through an Axum HTTP server. Each handler wires the shared `ServicesAppState` into the underlying services layer, so every request reuses the real storage, index, and Supabase stacks defined in `src/services_http_server.rs:60`.

## Step 1: Start the Services HTTP server

Spin up the router that suits your deployment before calling any endpoint.

| Mode | How to run | Router entry point |
| --- | --- | --- |
| Local workspace | `kotadb serve --port 8080` (CLI) | `start_services_server` (`src/services_http_server.rs:628`) |
| Managed/SaaS | `cargo run --bin kotadb-api-server` | `start_services_saas_server` (`src/services_http_server.rs:833`) |

Both paths construct the shared `ServicesAppState` (`src/services_http_server.rs:60`) so handlers can lock the file-backed `Storage`, open the B-tree/Trigram indices, and look up Supabase resources when SaaS mode is enabled. `ServicesAppState::allow_local_path_ingestion` (`src/services_http_server.rs:98`) blocks local path ingestion automatically unless `ALLOW_LOCAL_PATH_INDEXING=1` is set.

> **Note** The router automatically mounts MCP tool endpoints whenever you compile with the `mcp-server` feature (see `create_services_server`, `src/services_http_server.rs:561`). Tree-sitter driven symbol features are available only when the `tree-sitter-parsing` feature is enabled.

## Step 2: Verify health

Begin with the health probes to confirm storage, indices, and (optionally) Supabase connectivity.

| Path | Method | Handler | Output |
| --- | --- | --- | --- |
| `/health` | GET | `health_check` (`src/services_http_server.rs:920`) | Returns `HealthResponse` (`src/services_http_server.rs:167`) listing enabled services and optional SaaS job-queue metrics. |
| `/api/v1/health-check` | GET | `health_check_detailed` (`src/services_http_server.rs:2998`) | Runs `ValidationService::validate_database` (`src/services/validation_service.rs:356`) to surface integrity and consistency findings. |

```bash
curl http://localhost:8080/health | jq
```

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "services_enabled": [
    "StatsService",
    "BenchmarkService"
  ]
}
```

In SaaS mode the handler augments the response with Supabase latency and job-queue details collected by `fetch_saas_health` (`src/services_http_server.rs:2575`).

## Step 3: Collect stats and diagnostics

The stats, benchmark, and validation endpoints reuse the Rust services that the CLI invokes, so responses mirror CLI output.

| Path | Method | Handler | Delegated service |
| --- | --- | --- | --- |
| `/api/v1/analysis/stats` | GET | `get_stats` (`src/services_http_server.rs:943`) | `StatsService::get_statistics` (`src/services/stats_service.rs:245`). Optional query flags: `basic`, `symbols`, `relationships` (`StatsQuery`, `src/services_http_server.rs:225`). |
| `/api/v1/benchmark` | POST | `run_benchmark` (`src/services_http_server.rs:1002`) | `BenchmarkService::run_benchmark` (`src/services/benchmark_service.rs:285`). Body schema `BenchmarkRequest` (`src/services_http_server.rs:236`). |
| `/api/v1/validate` | POST | `validate_database` (`src/services_http_server.rs:2926`) | `ValidationService::validate_database` (`src/services/validation_service.rs:356`). Body schema `ValidationRequest` (`src/services_http_server.rs:244`). |

```bash
curl "http://localhost:8080/api/v1/analysis/stats?symbols=true" | jq '.basic_stats'
```

`StatsService` gates symbol-derived metrics behind the `tree-sitter-parsing` feature (`src/services/stats_service.rs:259`), so compile with that feature to see symbol counts in the response.

## Step 4: Search the indexed content

All search endpoints funnel into `SearchService` (`src/services/search_service.rs:102`), which performs trigram lookups and optional LLM summarisation.

| Path | Method(s) | Handler | Request schema |
| --- | --- | --- | --- |
| `/api/v1/search/code` | GET+POST | `search_code_enhanced` (`src/services_http_server.rs:3174`) and `search_code_v1_post` (`src/services_http_server.rs:1063`) | `SearchRequest` (`src/services_http_server.rs:316`) controls `query`, `limit`, `search_type`, and `format`. |
| `/api/v1/search/symbols` | GET+POST | `search_symbols_enhanced` (`src/services_http_server.rs:3212`) and `search_symbols_v1_post` (`src/services_http_server.rs:1126`) | `SymbolSearchRequest` (`src/services_http_server.rs:323`) with optional `symbol_type` filter. |
| `/api/v1/symbols` | GET | `list_symbols_v1` (`src/services_http_server.rs:1269`) | Query params `pattern`, `limit`, `symbol_type` (`ListSymbolsQuery`, `src/services_http_server.rs:1252`). |
| `/api/v1/files/symbols/*path` | GET | `file_symbols_v1` (`src/services_http_server.rs:1296`) | Streams symbol records directly from `symbols.kota`. |

`SearchService::search_content` (`src/services/search_service.rs:117`) upsells to LLM-backed summaries when `search_type` is `medium` or `full`, falling back to trigram search on failure (`src/services/search_service.rs:131`). Toggle response formats with `format=rich|simple|cli`; the handlers transform results via `render_search_code_response` (`src/services_http_server.rs:3387`).

```bash
curl "http://localhost:8080/api/v1/search/code?query=Database::new&format=cli" | jq -r '.output'
```

> **Note** Symbol search and `/api/v1/files/symbols/*path` require the `tree-sitter-parsing` feature because the handlers read `symbols.kota` generated by symbol extraction (`SearchService::search_symbols`, `src/services/search_service.rs:197`).

## Step 5: Analyze relationships and codebase context

Relationship endpoints delegate to `AnalysisService` so that HTTP consumers receive the same markdown and relationship graphs the CLI renders.

| Path | Method | Handler | Delegated call |
| --- | --- | --- | --- |
| `/api/v1/find-callers` | POST | `find_callers_enhanced` (`src/services_http_server.rs:3286`) | `AnalysisService::find_callers` (`src/services/analysis_service.rs:252`). Body schema `CallersRequest` (`src/services_http_server.rs:332`). |
| `/api/v1/symbols/:symbol/callers` | GET | `find_callers_v1_get` (`src/services_http_server.rs:1170`) | Same service method using query `limit`. |
| `/api/v1/analyze-impact` | POST | `analyze_impact_enhanced` (`src/services_http_server.rs:3328`) | `AnalysisService::analyze_impact` (`src/services/analysis_service.rs:274`). Body schema `ImpactAnalysisRequest` (`src/services_http_server.rs:342`). |
| `/api/v1/symbols/:symbol/impact` | GET | `analyze_impact_v1_get` (`src/services_http_server.rs:1211`) | GET façade for the same analysis. |
| `/api/v1/codebase-overview` | GET | `codebase_overview` (`src/services_http_server.rs:3078`) | `AnalysisService::generate_overview` (`src/services/analysis_service.rs:296`). |

Use the `format` field (`rich`, `simple`, `cli`) to collapse responses to filenames, produce CLI-formatted markdown, or keep the full structured payload. Helper functions such as `format_callers_as_cli` (`src/services_http_server.rs:3475`) ensure parity with the terminal output.

## Step 6: Manage repositories and indexing

Repository operations orchestrate background jobs and persist state either locally or via Supabase.

| Path | Method | Handler | Behaviour |
| --- | --- | --- | --- |
| `/api/v1/repositories` | POST | `register_repository_v1` (`src/services_http_server.rs:1331`) | Dispatches to `register_repository_local` (`src/services_http_server.rs:1348`) or `register_repository_saas` (`src/services_http_server.rs:1554`). Request body `RegisterRepositoryRequest` (`src/services_http_server.rs:253`). |
| `/api/v1/repositories` | GET | `list_repositories_v1` (`src/services_http_server.rs:2638`) | Returns in-memory registry or Supabase rows. |
| `/api/v1/index/status` | GET | `index_status_v1` (`src/services_http_server.rs:2717`) | Looks up job status from in-memory map or Supabase. Requires `job_id` query parameter. |
| `/api/v1/index-codebase` | POST | `index_codebase` (`src/services_http_server.rs:3034`) | Directly invokes `IndexingService::index_codebase` (`src/services/indexing_service.rs:141`). |

`register_repository_local` normalises filesystem paths, assigns stable IDs, and enqueues background indexing tasks that update `JobStatus` (`src/services_http_server.rs:1400`). The spawned task constructs `IndexingService` with the same `Storage` and indices and mirrors CLI progress handling (`src/services_http_server.rs:1488`). In SaaS mode the handler records jobs through `SupabaseRepositoryStore` and provisions optional GitHub webhooks via `ensure_github_webhook` (`src/services_http_server.rs:1668`).

> **Warning** Managed deployments reject local filesystem ingestion; callers must supply `git_url` and authenticate with an API key (`register_repository_saas`, `src/services_http_server.rs:1554`).

To monitor progress, poll `/api/v1/index/status?job_id={uuid}` until the `status` field transitions to `completed` or `failed`.

## Step 7: Enable SaaS authentication and webhooks

API key enforcement is provided by `auth_middleware` (`src/auth_middleware.rs:54`), which expects an `X-API-Key` or `Authorization: Bearer` header and attaches `AuthContext` to the request extensions. SaaS routers wrap every authenticated route with this middleware (`src/services_http_server.rs:745`).

- Issue new API keys through the internal-only `/internal/create-api-key` endpoint handled by `create_api_key_handler` (`src/services_http_server.rs:3630`).
- GitHub webhook deliveries should target `/webhooks/github/:repository_id`; the handler `handle_github_webhook` (`src/services_http_server.rs:1707`) verifies HMAC signatures using the stored secret before enqueuing refresh jobs.
- Rate limiting and quota checks happen inside `ApiKeyService::validate_api_key` (see `src/auth_middleware.rs:114`), so clients should cache responses or use the stats endpoints instead of polling aggressively.

## Step 8: Interpret errors and response formats

Every JSON handler returns `ApiResult<T>` (`src/services_http_server.rs:393`), which serialises success payloads and maps failures to `StandardApiError` (`src/services_http_server.rs:344`).

```json
{
  "error_type": "validation_error",
  "message": "Query cannot be empty",
  "details": "search-code",
  "suggestions": ["Provide a non-empty query"],
  "error_code": 400
}
```

Enhanced endpoints share formatter helpers:

- `render_search_code_response` (`src/services_http_server.rs:3387`) chooses between rich, simple (`SimpleSearchResponse`, `src/services_http_server.rs:350`), and CLI (`CliFormatResponse`, `src/services_http_server.rs:356`) outputs.
- Relationship handlers convert `AnalysisService` results to the requested format using helpers such as `format_impact_as_cli` (`src/services_http_server.rs:3539`).

Always expect ISO-8601 timestamps (e.g., `JobStatus.updated_at`, `src/services_http_server.rs:203`) and stable UUID/job IDs generated with `Uuid::new_v4` (`src/services_http_server.rs:139`) or Supabase IDs.

## Next Steps

- Run `just test` to ensure your codebase is indexed before exercising analysis endpoints.
- Capture example payloads for your clients and pin expected fields in contract tests.
- Enable `tree-sitter-parsing` and `mcp-server` features when you need symbol search or MCP tooling; rebuild the server afterward.
- Configure Supabase credentials and `KOTADB_WEBHOOK_BASE_URL` before switching production traffic to the SaaS router.
