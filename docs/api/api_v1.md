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
- The SaaS server (`create_services_saas_server`) puts v1 routes behind API key auth.

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
  - Note: `git_url` is not supported yet. Return: `git_url_not_supported` (400). Clone locally and use `path`.
  - 400: when neither `path` nor `git_url` provided; when `path` does not exist or is not a directory
  - 200: { job_id, repository_id, status: "accepted" }
  - Details: repository_id is stable (hash of canonical path).

- GET `/api/v1/repositories`
  - 200 OK: { repositories: [ { id, name, path, url, last_indexed } ] }

- GET `/api/v1/index/status?job_id=...`
  - 200 OK: { job: { id, status, progress?, started_at?, updated_at?, error? } }
  - 404 Not Found: unknown `job_id` (returns `StandardApiError`)

Operational Notes
- Job tracking uses pruning to prevent unbounded growth (TTL=1h, cap=100 completed/failed jobs).
- Timestamps are RFC3339.
- Server startup banners and endpoint listings are logged at `debug` level to avoid noisy `info` logs; use `RUST_LOG=debug` or `--verbose` to see them.

