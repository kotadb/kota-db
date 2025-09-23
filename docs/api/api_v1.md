---
title: API v1 (Services)
summary: Versioned HTTP API for code intelligence and repository management
---

This document describes the KotaDB `/api/v1/*` endpoints exposed by the services HTTP servers.

Highlights
- Versioned, minimal wrappers over existing services.
- Standardized error contract (`StandardApiError`).
- Local/dev server exposes all v1 routes without auth. SaaS server protects them with API keys.

Security
- The non-SaaS server (`create_services_server`) exposes repository registration and indexing for arbitrary local paths. It is intended for local/dev, single-tenant usage. Do not expose it publicly in multi-tenant environments.
- The SaaS server (`create_services_saas_server`) puts v1 routes behind API key auth and expects every `/internal/*` request to include the secret `INTERNAL_API_KEY` header value.
- Managed deployments reject filesystem paths outright; onboarding requires `git_url`. Self-hosted installs can opt back into local path ingestion via `ALLOW_LOCAL_PATH_INDEXING=1` if they trust the environment.

Error Contract
`StandardApiError` (JSON):
{
  "error_type": "string",
  "message": "string",
  "details": "string|null",
  "suggestions": ["string"],
  "error_code": 400|404|500
}

Endpoints
- POST `/api/v1/search/code`
  - Body: { "query": "string", "limit?": number, "format?": "rich"|"simple"|"cli" }
  - 200 OK: rich JSON result or simple/cli formats
  - 400: validation error on empty query

- POST `/api/v1/search/symbols`
  - Body: { "pattern": "string", "limit?": number, "symbol_type?": "string", "format?": "rich"|"simple"|"cli" }
  - 200 OK: rich JSON result or simple/cli formats
  - 400: validation error on empty pattern

- GET `/api/v1/symbols/:symbol/callers`
  - Query: { "limit?": number }
  - 200 OK: callers
  - 500: when symbols DB is missing
  - 400: if path parameter `symbol` is empty (routing usually prevents this)

- GET `/api/v1/symbols/:symbol/impact`
  - Query: { "limit?": number }
  - 200 OK: impact
  - 500: when symbols DB is missing
  - 400: if path parameter `symbol` is empty

- GET `/api/v1/symbols`
  - Query: { "pattern?": string, "limit?": number, "symbol_type?": string }
  - 200 OK: symbol list

- GET `/api/v1/files/symbols/*path`
  - 200 OK: { "file": string, "symbols": [ { name, kind, start_line, end_line } ] }
  - 404: symbols DB missing
  - Implementation detail: optimized lookup via a cached file→symbols index.

- POST `/api/v1/repositories`
  - Body: { "path"?: string, "git_url"?: string, "branch"?: string,
            "include_files?": bool, "include_commits?": bool,
            "max_file_size_mb?": number, "max_memory_mb?": number,
            "max_parallel_files?": number, "enable_chunking?": bool,
            "extract_symbols?": bool }
  - Managed (SaaS) deployments require `git_url` and ignore/forbid `path`. Self-hosted mode still supports local `path` ingestion.
  - The response includes `webhook_secret` when a repository is provisioned for the first time so you can configure the GitHub webhook signature. Re-registering an existing repository omits the secret.
  - 400: when neither `path` nor `git_url` provided; when `path` does not exist or is not a directory
  - 200: { job_id, repository_id, status: "queued", webhook_secret? }

- GET `/api/v1/repositories`
  - 200 OK: { repositories: [ { id, name, path, url, last_indexed } ] }

- GET `/api/v1/index/status?job_id=...`
  - 200 OK: { job: { id, status, progress?, started_at?, updated_at?, error? } }
  - 404 Not Found: unknown `job_id` (returns `StandardApiError`)

- POST `/webhooks/github/:repository_id`
  - Headers: `X-Hub-Signature-256` (HMAC SHA-256), `X-GitHub-Event`, `X-GitHub-Delivery`
  - Body: raw GitHub webhook payload
  - 202: { status: "queued", job_id? } when the push/pull request event enqueues an indexing job
  - 200: { status: "pong" } for `ping` events; ignored events report `{ status: "ignored:<event>" }`
  - Use the per-repository `webhook_secret` returned from registration to compute the HMAC signature GitHub expects.
  - Push payloads aggregate commit `added`/`modified`/`removed` file paths into the queued job’s payload so the worker (and future incremental pipeline) can scope reindexing work.

Operational Notes
- Job tracking uses pruning to prevent unbounded growth (TTL=1h, cap=100 completed/failed jobs).
- Timestamps are RFC3339.
- Server startup banners and endpoint listings are logged at `debug` level to avoid noisy `info` logs; use `RUST_LOG=debug` or `--verbose` to see them.
