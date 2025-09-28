# Supabase + Fly.io Architecture Guide
KotaDB's SaaS control plane stores tenants, repositories, and indexing jobs in Supabase while Fly.io hosts stateless API servers that pull work from Supabase and push results back. This guide walks through provisioning, runtime flows, and health checks using the current Rust implementation.

## Step 1 — Provision the Supabase control plane
1. Rebuild the local Supabase stack with `just supabase-reset` (`justfile:164`), which shells to `scripts/supabase-reset-local.sh:11` to replay every SQL migration and seed file against the Dockerised dev database.
2. Capture schema changes with `just supabase-generate <name>` (`justfile:161`), invoking the migra-based diff helper in `scripts/supabase-generate-migration.sh:22` so new DDL lands under `supabase/migrations/`.
3. Apply migrations to remote environments via `just supabase-apply postgres://…` (`justfile:167`), which sanitises the URL, normalises Supabase metadata, and runs `supabase db push` inside `scripts/supabase-apply-remote.sh:27`.
4. Validate row-level security and indexes: `repositories` governs multi-tenant repository state with service-role overrides (`supabase/migrations/20250922_saas_repositories_jobs.sql:8` and `supabase/migrations/20250922_saas_repositories_jobs.sql:28`), while `indexing_jobs`, `webhook_deliveries`, `token_usage`, and `repository_secrets` compose the job queue and secret escrow layers (`supabase/migrations/20250922_saas_repositories_jobs.sql:57`, `supabase/migrations/20250922_saas_repositories_jobs.sql:143`, `supabase/migrations/20250922_saas_repositories_jobs.sql:182`, `supabase/migrations/20250922_saas_repositories_jobs.sql:215`).
5. Expose operational views such as `repository_status_view` for dashboards (`supabase/migrations/20250922_saas_repositories_jobs.sql:251`) before promoting changes.

```bash
just supabase-reset
just supabase-generate saas_hotfix
SUPABASE_DB_URL=postgres://user:pass@db just supabase-apply postgres://user:pass@db
```

> **Warning** Keep the Supabase service role key confined to the API tier; it is required by deployment checks but must never leak to browser clients (`src/services_http_server.rs:2503`).

## Step 2 — Boot the Fly API with Supabase credentials
1. `create_services_saas_server` instantiates `ApiKeyService` and reuses its `PgPool` as the Supabase connection (`src/services_http_server.rs:705`, `src/services_http_server.rs:709`).
2. Environment validation fails fast unless `DISABLE_SAAS_ENV_VALIDATION` is set, ensuring a database URL and all critical Supabase/GitHub secrets exist before serving traffic (`src/services_http_server.rs:2535`, `src/services_http_server.rs:2502`).
3. The resulting `ServicesAppState` is stateless—no Fly volume is mounted—relying exclusively on Supabase for persistence as documented in `fly.toml:67`.

## Step 3 — Authenticate every request via Supabase-backed keys
1. API routes are wrapped by `auth_middleware`, which extracts API keys, shortcuts `/health`, and injects an `AuthContext` for downstream handlers (`src/auth_middleware.rs:103`).
2. `ApiKeyService::validate_api_key` hashes the presented key, loads the row from `kotadb_api_keys`, evaluates expiry/IP restrictions, and updates the last-used timestamp (`src/api_keys.rs:289`).
3. Quotas are enforced with `check_rate_limit`, which maintains a sliding window in `api_key_rate_limits` (`src/api_keys.rs:398`).
4. Regardless of outcome, the middleware logs usage metrics and increments counters via `record_usage` so Supabase holds a full audit trail (`src/api_keys.rs:465`).

## Step 4 — Register repositories through the SaaS API
1. `register_repository_v1` dispatches to the SaaS path whenever the server booted in SaaS mode (`src/services_http_server.rs:1331`).
2. `register_repository_saas` rejects filesystem ingestion, resolves the caller's Supabase `user_id`, and normalises provider metadata before touching the database (`src/services_http_server.rs:1554`).
3. The handler retrieves the user's primary API key (if any) through `SupabaseRepositoryStore::lookup_primary_api_key` to preserve key provenance on queued jobs (`src/supabase_repository/mod.rs:764`).
4. `SupabaseRepositoryStore::register_repository_and_enqueue_job` upserts the repository, manages webhook secrets, enqueues a `full_index` job, and commits the transaction atomically (`src/supabase_repository/mod.rs:93`).
5. GitHub repositories trigger webhook provisioning—`ensure_github_webhook` stores metadata and hashes secrets so future deliveries can be validated (`src/services_http_server.rs:1638`).

## Step 5 — Process GitHub webhooks and deduplicate deliveries
1. `/webhooks/github/:repository_id` resolves the tenant repository, fetches the encrypted secret, and validates `X-Hub-Signature-256` with `verify_github_signature` (`src/services_http_server.rs:1707`, `src/services_http_server.rs:2484`).
2. Deliveries are deduped through `SupabaseRepositoryStore::find_webhook_delivery`, which tracks prior status and either refreshes, resets, or short-circuits the request (`src/supabase_repository/mod.rs:539`).
3. New or retried deliveries are recorded via `record_webhook_delivery`, capturing payloads, headers, and signature for later debugging (`src/supabase_repository/mod.rs:648`).
4. The handler enqueues follow-up jobs using the same store API so the background worker processes them in order (`src/supabase_repository/mod.rs:718`).

## Step 6 — Run background indexing workers from Supabase jobs
1. SaaS boot spawns `SupabaseJobWorker::run`, which continually polls Supabase, recovering stale jobs before claiming a new one (`src/services_http_server.rs:728`, `src/supabase_repository/job_worker.rs:124`).
2. Claimed jobs transition the repository to `syncing`, receive detailed job events, and branch based on `job_type` (`src/supabase_repository/job_worker.rs:140`, `src/supabase_repository/job_worker.rs:217`).
3. Incremental webhook payloads are normalised by `SupabaseJobPayload::parse` and `merge_settings`, which blend runtime overrides with stored repository settings (`src/supabase_repository/task.rs:5`, `src/supabase_repository/task.rs:24`).
4. The worker computes incremental work, purges removed files, and only clones when necessary while still recording “no changes” events (`src/supabase_repository/job_worker.rs:328`).
5. Repository preparation and cloning require the optional `git-integration` feature; the worker aborts early without it (`src/supabase_repository/job_worker.rs:417`).
6. After indexing—delegated to `IndexingService`—the worker updates repository metadata, completes the job record, and marks any webhook delivery as processed (`src/supabase_repository/job_worker.rs:362`, `src/supabase_repository/job_worker.rs:383`, `src/supabase_repository/job_worker.rs:162`).
7. Failures propagate through `fail_job`, flipping repository state to `error` and annotating events for observability (`src/supabase_repository/mod.rs:571`).

## Step 7 — Monitor Supabase health and job queues
1. `/health` surfaces SaaS metrics via `fetch_saas_health`, timing a Supabase query that counts queued, in-progress, and failed jobs (`src/services_http_server.rs:2575`).
2. The helper also records the oldest queued age and recent failures for Fly smoke tests to consume (`src/services_http_server.rs:2594`).
3. `scripts/saas_smoke.sh` calls the same endpoint, fails deployments when Supabase is unreachable, and optionally hits authenticated repository listings (`scripts/saas_smoke.sh:12`, `scripts/saas_smoke.sh:92`).

## Supabase Schema Snapshot
| Table/View | Purpose | Source |
| --- | --- | --- |
| `repositories` | Tenant repository registry with webhook hashes and metadata | `supabase/migrations/20250922_saas_repositories_jobs.sql:8` |
| `indexing_jobs` | Priority queue for background workers with status/attempt tracking | `supabase/migrations/20250922_saas_repositories_jobs.sql:57` |
| `indexing_job_events` | Append-only event log for worker state transitions | `supabase/migrations/20250922_saas_repositories_jobs.sql:110` |
| `webhook_deliveries` | Delivery deduplication and payload archive | `supabase/migrations/20250922_saas_repositories_jobs.sql:143` |
| `repository_secrets` | Service-role-only storage for webhook secrets | `supabase/migrations/20250922_saas_repositories_jobs.sql:215` |
| `token_usage` | API token accounting by key, repository, and job | `supabase/migrations/20250922_saas_repositories_jobs.sql:182` |
| `repository_status_view` | Aggregated job counts for dashboards and health checks | `supabase/migrations/20250922_saas_repositories_jobs.sql:251` |

## Key Rust APIs
| Component | Responsibility | Source |
| --- | --- | --- |
| `auth_middleware` | Enforce API key auth, rate limiting, and usage logging | `src/auth_middleware.rs:103` |
| `register_repository_saas` | Map SaaS API requests to Supabase rows and GitHub provisioning | `src/services_http_server.rs:1554` |
| `SupabaseRepositoryStore::register_repository_and_enqueue_job` | Transactionally upsert repositories, secrets, and indexing jobs | `src/supabase_repository/mod.rs:93` |
| `SupabaseRepositoryStore::record_job_event` | Persist job lifecycle messages with optional JSON context | `src/supabase_repository/mod.rs:622` |
| `SupabaseJobWorker::run` | Poll Supabase, execute indexing, and close out jobs | `src/supabase_repository/job_worker.rs:124` |
| `handle_github_webhook` | Verify signatures, deduplicate deliveries, and enqueue jobs | `src/services_http_server.rs:1707` |
| `SupabaseJobPayload` | Deserialize webhook/generic job payloads and merge settings | `src/supabase_repository/task.rs:5` |

## Operational Utilities
- `scripts/saas_smoke.sh` verifies Supabase connectivity, queue depth, and optional MCP endpoints before or after deploys (`scripts/saas_smoke.sh:12`).
- `scripts/supabase-apply-remote.sh` performs guarded remote migrations while scrubbing connection strings from logs (`scripts/supabase-apply-remote.sh:27`).
- `validate_saas_environment` warns when optional GitHub or Supabase knobs are missing, making it the first place to augment when adding new secrets (`src/services_http_server.rs:2535`).

## Next Steps
- Run `just supabase-apply` against staging before cutting a Fly release to guarantee schema parity.
- Tail `scripts/saas_smoke.sh` in CI/CD so Supabase latency and queue depth regressions block deploys.
- Extend `repository_status_view` with per-feature counters if additional job types land.
- Add alerting on `/health` fields (queued depth, failed_recent) once metrics ingestion is wired up.
