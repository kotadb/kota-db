# API Reference

KotaDB's HTTP surface is a thin wrapper around the services layer. The public router in `src/services_http_server.rs:561` wires each REST endpoint directly to the `SearchService`, `AnalysisService`, `StatsService`, `IndexingService`, and related components, so everything you call over HTTP follows the same execution path as the CLI and MCP tooling.

## Step 1. Launch the Services HTTP API
- Local development uses `start_services_server` (`src/services_http_server.rs:627`), which brings up the router without authentication.
- Managed (SaaS) mode uses `create_services_saas_server` (`src/services_http_server.rs:700`) and `start_services_saas_server` (`src/services_http_server.rs:833`) to layer API-key auth, Supabase job tracking, and webhook handling on top of the same endpoints.
- Run the binaries directly:
  - Local services: `cargo run --bin kotadb -- serve --port 8080` (see `src/main.rs` for CLI wiring).
  - SaaS API: `cargo run --bin kotadb-api-server` (`src/bin/kotadb-api-server.rs:52`).
- Every request path flows through `with_trace_id` (`src/observability.rs:319`), so trace IDs will show up in logs and metrics without additional work.

| Mode | Entry Function | Extra Layers | Feature Flags |
| --- | --- | --- | --- |
| Local services | `create_services_server` (`src/services_http_server.rs:541`) | CORS + HTTP tracing | Optional MCP bridge behind `mcp-server` (and `tree-sitter-parsing` for symbol tools) |
| SaaS services | `create_services_saas_server` (`src/services_http_server.rs:700`) | API key auth, Supabase-backed job worker, GitHub webhooks | Requires database connectivity; MCP bridge still gated by `mcp-server` |

> **Note**: Enabling the MCP endpoints merges the bridge router from `create_mcp_bridge_router` when `--features mcp-server` is set (`src/services_http_server.rs:596`). Add `tree-sitter-parsing` to expose symbol tools via MCP.

## Step 2. Configure Authentication and State
- Application state lives in `ServicesAppState` (`src/services_http_server.rs:60`). The table below highlights the fields you typically configure.

| Field | Purpose | When Needed |
| --- | --- | --- |
| `storage`, `primary_index`, `trigram_index` | Backing stores for documents and trigram search | Always |
| `api_key_service` | Injected when running SaaS mode | Managed deployments |
| `supabase_pool` | Supabase connection for repository + job metadata | Managed deployments |
| `jobs` | In-memory job queue mirror used by local indexing | Local + SaaS |
| `repositories` | Cached copy of `repositories.json` under the data dir | Local + SaaS |

- `auth_middleware` (`src/auth_middleware.rs:100`) enforces API-key access. Successful requests get an `AuthContext` injected into handlers (`src/auth_middleware.rs:48`) with `key_id`, `user_email`, and quota data for rate limiting.
- SaaS mode spawns a `SupabaseJobWorker` (`src/services_http_server.rs:737`) that pulls indexing jobs from Supabase and executes them with the same services APIs used by HTTP handlers.

> **Warning**: When `ServicesAppState::is_saas_mode()` is true, local path ingestion is blocked unless `ALLOW_LOCAL_PATH_INDEXING=1` is set (`src/services_http_server.rs:98`). Git-only repositories stay allowed in SaaS mode.

## Step 3. Query Search and Intelligence Endpoints
The core read endpoints all share the same router slice (see `src/services_http_server.rs:563`). Each handler builds a `Database` façade (`src/database.rs:20`) and calls into the underlying service before formatting JSON.

| Endpoint | Handler | Downstream Service |
| --- | --- | --- |
| `GET /api/v1/search/code` | `search_code_enhanced` (`src/services_http_server.rs:3174`) | `SearchService::search_content` (`src/services/search_service.rs:118`) |
| `POST /api/v1/search/code` | `search_code_v1_post` (`src/services_http_server.rs:1063`) | Same as above, using a JSON body |
| `GET /api/v1/search/symbols` | `search_symbols_enhanced` (`src/services_http_server.rs:3225`) | `SearchService::search_symbols` (`src/services/search_service.rs:175`) |
| `GET /api/v1/symbols/:symbol/callers` | `find_callers_v1_get` (`src/services_http_server.rs:1170`) | `AnalysisService::find_callers` (`src/services/analysis_service.rs:252`) |
| `GET /api/v1/symbols/:symbol/impact` | `analyze_impact_v1_get` (`src/services_http_server.rs:1211`) | `AnalysisService::analyze_impact` (`src/services/analysis_service.rs:284`) |
| `GET /api/v1/analysis/stats` | `get_stats` (`src/services_http_server.rs:943`) | `StatsService::get_statistics` (`src/services/stats_service.rs:245`) |
| `POST /api/v1/benchmark` | `run_benchmark` (`src/services_http_server.rs:997`) | `BenchmarkService::run_benchmark` (`src/services/benchmark_service.rs:300`) |
| `POST /api/v1/validate` | `validate_database` (`src/services_http_server.rs:2938`) | `ValidationService::validate_database` (`src/services/validation_service.rs:356`) |
| `GET /api/v1/codebase-overview` | `codebase_overview` (`src/services_http_server.rs:3117`) | `AnalysisService::generate_overview` (`src/services/analysis_service.rs:318`) |

### Content Search
- Query parameters are parsed through `SearchRequest` (`src/services_http_server.rs:316`) for GET and `V1SearchCodeBody` (`src/services_http_server.rs:1056`) for POST.

| Field | Type | Notes |
| --- | --- | --- |
| `query` | string | Required; empty strings are rejected with `handle_validation_error` (`src/services_http_server.rs:3180`). |
| `limit` | integer | Defaults to 10; forwarded into `SearchOptions::limit` (`src/services/search_service.rs:96`). |
| `search_type` | string | Controls the context size (`medium` or `full`) before LLM optimization kicks in (`src/services/search_service.rs:131`). |
| `format` | string | `rich` (default), `simple`, or `cli`; used by `render_search_code_response` to structure the payload (`src/services_http_server.rs:3212`). |

- `SearchService::search_content` switches between LLM-optimized and regular trigram search (`src/services/search_service.rs:118-172`). Failures fall back automatically, so the HTTP response surface stays stable.

> **Note**: The search handlers call `with_trace_id("api_enhanced_search_code", ...)` (`src/services_http_server.rs:3187`), which emits per-request spans you can join with CLI traces.

### Symbol Search and Listing
- POST bodies use `V1SearchSymbolsBody` (`src/services_http_server.rs:1119`). Empty patterns are rejected early (line 1132).
- Symbol matching happens against `BinarySymbolReader` (`src/services/search_service.rs:174`) with wildcard support (`matches_wildcard_pattern`).

| Field | Type | Description |
| --- | --- | --- |
| `pattern` | string | Required; supports `*` wildcards. |
| `limit` | integer | Defaults to 25; caps the number of `SymbolMatch` entries (`src/services/search_service.rs:200`). |
| `symbol_type` | string | Optional substring filter against the symbol kind. |
| `format` | string | Controls presentation in the formatter before JSON serialization (`src/services_http_server.rs:1160`). |

- Directory-wide symbol listings reuse the same service via `list_symbols_v1` (`src/services_http_server.rs:1269`).

### Call Graph and Impact Analysis
- `CallersOptions` and `ImpactOptions` live in `src/services/analysis_service.rs:29` and `src/services/analysis_service.rs:36`. They wrap caller/impact limit parameters and forward them to the binary relationship engine.
- Both endpoints share validation logic (lines 1175–1180 and 1216–1221) and return the serialized `CallersResult`/`ImpactResult` structures from the analysis service.
- Enhanced POST variants (`find_callers_enhanced` at `src/services_http_server.rs:3275` and `analyze_impact_enhanced` at `src/services_http_server.rs:3358`) accept JSON bodies for parity with CLI flags.

### Stats, Benchmarks, and Validation
- `StatsOptions` toggles symbol and relationship counters (`src/services_http_server.rs:958`). Use query flags like `?symbols=false` to skip heavy aggregations.
- Benchmarks accept `BenchmarkRequest` (`src/services_http_server.rs:232`) to switch between scenarios (`benchmark_type`) before invoking `BenchmarkService`.
- `validate_database` lifts `ValidationOptions` defaults aligned with the CLI (`src/services_http_server.rs:3004`), running integrity and consistency passes without repair by default.
- `health_check` (`src/services_http_server.rs:919`) surfaces feature flags and SaaS job stats in one lightweight probe.

## Step 4. Manage Repositories and Indexing
Repository lifecycle endpoints sit next to the read APIs, but they layer in job management and (optionally) Supabase persistence.

| Endpoint | Handler | Purpose |
| --- | --- | --- |
| `POST /api/v1/repositories` | `register_repository_v1` (`src/services_http_server.rs:1296`) | Kick off indexing jobs (local or SaaS) |
| `GET /api/v1/repositories` | `list_repositories_v1` (`src/services_http_server.rs:2638`) | Enumerate tracked repositories |
| `GET /api/v1/index/status` | `index_status_v1` (`src/services_http_server.rs:2717`) | Poll job progress |
| `POST /api/v1/index-codebase` | `index_codebase` (`src/services_http_server.rs:3044`) | Synchronously index a local path |
| `POST /webhooks/github/:repository_id` | `handle_github_webhook` (`src/services_http_server.rs:1707`) | Enqueue Supabase jobs from GitHub events |

### Register Repositories
- JSON payloads follow `RegisterRepositoryRequest` (`src/services_http_server.rs:259`).

| Field | Type | Details |
| --- | --- | --- |
| `path` | string | Required in local mode; must exist and be a directory (`src/services_http_server.rs:1378-1392`). |
| `git_url` | string | SaaS-only today. Local mode returns `git_url_not_supported` (`src/services_http_server.rs:1411-1421`). |
| `branch` | string | Optional; normalized via `normalize_git_ref` when provisioning webhooks. |
| `include_*`, `max_*`, `enable_chunking`, `extract_symbols` | overrides | Passed through to `IndexingService` job options (lines 1435–1442).

- Successful submissions allocate a `JobStatus` entry and, in SaaS mode, persist metadata to Supabase before optionally provisioning a GitHub webhook (`src/services_http_server.rs:1680`).

### Poll Job Status
- Local mode inspects the in-memory `jobs` map (`src/services_http_server.rs:2726`).
- SaaS mode requires an authenticated user ID and fetches status from Supabase (`src/services_http_server.rs:2749`). Unknown IDs trigger a structured 404 via `handle_not_found_error`.

### Direct Indexing
- `index_codebase` creates an `IndexingService` and executes `index_codebase` (`src/services/indexing_service.rs:149`). Key options include `enable_chunking` and `extract_symbols`, which toggle tree-sitter parsing when the feature is built in.

| Option | Source | Description |
| --- | --- | --- |
| `repo_path` | `IndexCodebaseRequest.repo_path` (`src/services_http_server.rs:249`) | Absolute or relative path to index. |
| `prefix` | defaults to `"repos"` (`src/services_http_server.rs:3072`) | Namespaced storage prefix used by the file backend. |
| `include_files`/`include_commits` | propagate to ingestion (`src/services/indexing_service.rs:18`) | Control document and commit capture. |
| `extract_symbols` | optional boolean | Enables symbol extraction; defaults to feature flag behavior (`src/services/indexing_service.rs:175`). |

> **Warning**: Direct indexing respects `allow_local_path_ingestion`; SaaS deployments will reject this endpoint unless explicitly allowed (`src/services_http_server.rs:3048`).

### GitHub Webhooks
- Incoming deliveries are authenticated with HMAC SHA-256 verification in `verify_github_signature` (`src/services_http_server.rs:2484`) before being recorded.
- Deduplication relies on the `X-GitHub-Delivery` header (`src/services_http_server.rs:1812`) so retries do not spawn duplicate jobs.
- Processed events are turned into Supabase jobs via `SupabaseRepositoryStore` and linked back to indexing results so `index_status_v1` can surface them.

> **Note**: Set `KOTADB_WEBHOOK_BASE_URL` before starting the SaaS server to ensure webhook provisioning succeeds (`src/services_http_server.rs:710`).

## Next Steps
- Run `just test-fast` to confirm service integrations and doctests remain green.
- Add project-specific automation by extending the router in `src/services_http_server.rs` if you need bespoke endpoints.
- Tail the logs with `RUST_LOG=info just dev` or your own launch command to correlate trace IDs from `with_trace_id` with client requests.
