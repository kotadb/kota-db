# Fly.io Deployment Guide for KotaDB SaaS API

Deploy the `kotadb-api-server` binary to Fly.io using the Supabase-backed SaaS architecture that the runtime already enforces. This walkthrough shows how the server boots, which secrets it expects, and how Fly.io, Supabase, and GitHub webhooks coordinate during and after a deploy.

## Step 1 - Map the Runtime Architecture

- Command-line arguments hydrate the server from environment variables, covering storage paths, port binding, rate limits, and the Supabase pool DSN (`src/bin/kotadb-api-server.rs:16`).
- The executable initialises file, primary, and trigram index backends before touching the network, preventing half-configured startups (`src/bin/kotadb-api-server.rs:95`).
- Successful database connectivity is asserted prior to serving traffic; the same `ApiKeyConfig` is then passed into the HTTP stack (`src/bin/kotadb-api-server.rs:128`).
- `create_services_saas_server` builds the Axum router, enabling API key authentication, MCP tooling, and webhook routing only when SaaS mode is fully configured (`src/services_http_server.rs:694`).
- `ServicesAppState` keeps shared handles for indices, the `ApiKeyService`, the Supabase pool, job tracking, and repository metadata (`src/services_http_server.rs:62`).
- A Supabase job worker polls for queued repository indexing work and hands it to `IndexingService::index_codebase`, so webhook-triggered jobs reuse the same ingestion pipeline as the CLI (`src/services_http_server.rs:729`, `src/supabase_repository/job_worker.rs:123`, `src/services/indexing_service.rs:149`).
- GitHub webhook payloads are authenticated against the stored secret, deduplicated via Supabase, and translated into job rows before the worker sees them (`src/services_http_server.rs:1707`, `src/supabase_repository/mod.rs:92`).
- Health probes surface SaaS diagnostics, including Supabase latency and job queue saturation, which power smoke tests and Fly.io runtime health checks (`src/services_http_server.rs:168`, `src/services_http_server.rs:2575`).

## Step 2 - Prepare Fly.io Access and Local Tooling

- Install the Fly.io CLI (`flyctl`) from the official instructions and authenticate with `flyctl auth login`.
- Ensure the Rust toolchain matches the project (`rustup show`) and that local builds use the required features: `cargo build --release --no-default-features --features "git-integration,tree-sitter-parsing,mcp-server"` (`Dockerfile.production:31`).
- Run the same checks that the deployment script enforces: `cargo test --features tree-sitter-parsing,git-integration --bin kotadb-api-server` and `cargo clippy --features tree-sitter-parsing,git-integration --bin kotadb-api-server -- -D warnings` (`scripts/deploy-fly.sh:63`).
- Confirm you can reach Supabase from your network before deploying by running `scripts/check_pooler_connection.sh` with a populated `.env` (`scripts/check_pooler_connection.sh:5`).
- Keep `just` available so you can fall back to `just test` or `just ci-fast` when you need broader coverage during incident response.

## Step 3 - Configure Supabase and Secrets

> **Warning** Disabling SaaS environment validation via `DISABLE_SAAS_ENV_VALIDATION` skips every safety check performed at startup and should only be used for emergency debugging (`src/services_http_server.rs:2535`).

| Secret | Purpose in runtime | Where it is consumed |
| --- | --- | --- |
| `DATABASE_URL` | Connects `ApiKeyService::new` to the Supabase Postgres pool for API key validation (`src/bin/kotadb-api-server.rs:28`, `src/api_keys.rs:125`) | Fly secret, local `.env` |
| `SUPABASE_DB_URL_STAGING` / `SUPABASE_DB_URL_PRODUCTION` | Direct (non-pooler) Postgres URLs used for migrations during deploys (`.github/workflows/saas-api-deploy.yml:88`, `.github/workflows/saas-api-deploy.yml:134`) | GitHub Secrets & Fly |
| `SUPABASE_URL`, `SUPABASE_ANON_KEY`, `SUPABASE_SERVICE_KEY` | Required for webhook provisioning and repository metadata sync (`src/services_http_server.rs:2502`) | Fly secrets |
| `KOTADB_WEBHOOK_BASE_URL` | Public base URL baked into webhook registrations (`src/services_http_server.rs:710`) | Fly secrets |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_WEBHOOK_TOKEN` | Enables GitHub OAuth handshakes and webhook verification (`src/services_http_server.rs:2515`) | Fly secrets |
| `SAAS_STAGING_API_KEY`, `SAAS_PRODUCTION_API_KEY` | API keys used by smoke tests during CI deploys (`.github/workflows/saas-api-deploy.yml:109`, `.github/workflows/saas-api-deploy.yml:156`) | GitHub Secrets |
| `REDIS_URL`, `SENTRY_DSN`, `JWT_SECRET` | Optional diagnostics and caching toggles surfaced as best-effort warnings (`src/services_http_server.rs:2553`) | Fly secrets |

- Use `./scripts/fly-secrets.sh staging set` or `./scripts/fly-secrets.sh production set` to update secrets interactively; the helper writes a temporary script so the final `flyctl secrets set` invocation is auditable (`scripts/fly-secrets.sh:50`).
- After staging secrets change, rerun `scripts/check_pooler_connection.sh` against the new `.env` and redeploy only after it reports success (`scripts/check_pooler_connection.sh:31`).
- Keep Supabase migrations current with `just supabase-generate` and `just supabase-reset`, then apply them to remote environments using `SUPABASE_DB_URL=... ./scripts/supabase-apply-remote.sh` (`scripts/supabase-apply-remote.sh:8`).

## Step 4 - Build the Release Image

- The container image is produced by `Dockerfile.production`, which compiles `kotadb`, `kotadb-api-server`, and the MCP bridge with the Fly-enabled feature set and copies them into a minimal Alpine runtime (`Dockerfile.production:31`, `Dockerfile.production:53`).
- Runtime defaults such as `PORT=8080`, tracing configuration, and `KOTADB_DATA_DIR=/app/data` are baked into the image and mirrored in the Fly manifests (`Dockerfile.production:66`, `fly.toml:16`).
- Production deployments inherit concurrency and health check policies from `fly.toml`, including HTTP health probes on `/health`, a 30-second timeout, and automatic rollbacks (`fly.toml:24`, `fly.toml:49`).
- Staging lowers resource limits and switches to the immediate deployment strategy while enabling extra diagnostics (`fly.staging.toml:14`, `fly.staging.toml:74`).
- Because all durable data lives in Supabase, the Fly manifests intentionally avoid attaching volumes; `/app/data` is only a scratch directory for indexing (`fly.toml:67`).

## Step 5 - Deploy the SaaS API

1. Log in to Fly (`flyctl auth login`) and pick the target environment (`staging` or `production`).
2. Run `./scripts/deploy-fly.sh staging` for standard deployments; the script gates the rollout behind tests, Clippy, and a confirmation prompt for production environments (`scripts/deploy-fly.sh:33`).
3. For manual control, call `flyctl deploy --config fly.staging.toml --app kotadb-api-staging --strategy immediate --ha=false --wait-timeout 300` in staging, or swap in the production manifest and rolling strategy when targeting `kotadb-api` (`scripts/deploy-fly.sh:101`).
4. First-time setups should invoke `flyctl apps create` for each environment, then set required secrets before deploying; skip volume creation because the runtime is stateless (`scripts/deploy-fly.sh:78`).
5. After the deploy waits for health checks, the script sleeps briefly and curls `/health`; use that output alongside `flyctl status --app <app>` to confirm readiness (`scripts/deploy-fly.sh:126`).

## Step 6 - Integrate With CI/CD

- The `saas-api-deploy.yml` workflow rebuilds and tests the API server for pushes to `develop` (staging) and `main` (production), mirroring the local deployment guardrails (`.github/workflows/saas-api-deploy.yml:1`).
- Supabase migrations run automatically when `SUPABASE_DB_URL_STAGING` or `SUPABASE_DB_URL_PRODUCTION` are present, using the same helper script you can run locally (`.github/workflows/saas-api-deploy.yml:88`).
- Deployments reuse Fly.io's official action to install `flyctl`, then apply the environment-specific manifest with the matching HA flag (`.github/workflows/saas-api-deploy.yml:149`).
- Post-deploy smoke tests call `scripts/saas_smoke.sh --mcp` so MCP endpoints and authenticated repository listings are exercised with environment-specific API keys (`.github/workflows/saas-api-deploy.yml:109`).
- Manual rollbacks are exposed through a workflow dispatch input that skips the deploy jobs and instead reruns `flyctl deploy` against a prior image (`.github/workflows/saas-api-deploy.yml:187`).

## Step 7 - Validate and Monitor the Service

- Fly.io health checks hit `/health`, which wraps `fetch_saas_health` and reports Supabase availability, latency, and job queue depth; an unhealthy Supabase connection flips the status to `unhealthy` (`src/services_http_server.rs:920`, `src/services_http_server.rs:2575`).
- Run `scripts/saas_smoke.sh -u https://kotadb-api-staging.fly.dev --mcp` after each deploy to parse the JSON health payload and verify authenticated endpoints (`scripts/saas_smoke.sh:53`, `scripts/saas_smoke.sh:92`).
- Tail runtime logs with `flyctl logs --app kotadb-api` to watch webhook intake, worker activity, and indexing progress messages emitted from the job loop (`src/supabase_repository/job_worker.rs:164`).
- Spot-check instances and scaling with `flyctl status --app kotadb-api` and `flyctl scale show --app kotadb-api` to confirm that concurrency, memory, and region match expectations (`fly.toml:38`).
- Keep Supabase-side metrics and job history in view via `supabase_migrations.schema_migrations`, plus telemetry forwarded by optional `SENTRY_DSN` if configured (`scripts/supabase-apply-remote.sh:32`).

## Step 8 - Troubleshoot and Roll Back

- If the server exits during startup, check for missing secrets; `validate_saas_environment` logs warnings and errors before returning a fatal error when core variables are absent (`src/services_http_server.rs:2545`).
- Repository webhooks failing with signature errors indicate mismatched secrets between Supabase and Fly; re-run registration or reset secrets using `SupabaseRepositoryStore::register_repository_and_enqueue_job` paths (`src/supabase_repository/mod.rs:136`).
- Stalled jobs will surface under `saas.job_queue.failed_recent` in the health payload; inspect the worker logs for `Job failed` messages and requeue from Supabase after correcting the repository (`src/supabase_repository/job_worker.rs:181`).
- Roll back quickly with `flyctl releases list --app kotadb-api` followed by `flyctl deploy --image <previous image>`; GitHub Actions exposes the same rollback flow through the workflow dispatch input (`scripts/deploy-fly.sh:111`, `.github/workflows/saas-api-deploy.yml:199`).
- When necessary, open an SSH console (`flyctl ssh console --app kotadb-api`) and run `curl localhost:8080/health` to bypass edge caching and confirm port readiness before promoting traffic.

## Next Steps

- Run `just ci-fast` to preflight your branch before triggering the Fly.io workflow.
- Keep staging and production migrations aligned with `./scripts/supabase-apply-remote.sh` prior to high-risk deploys.
- Schedule regular smoke tests with `scripts/saas_smoke.sh --mcp` using rotating API keys.
- Tag releases after successful production deploys so `flyctl releases` history maps cleanly to source commits.
