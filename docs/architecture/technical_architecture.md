---
title: "KOTA Custom Database Technical Architecture"
tags: [database, architecture, technical-design]
related: ["IMPLEMENTATION_PLAN.md"]
key_concepts: [storage-engine, query-engine, indexing, compression]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KOTA Custom Database Technical Architecture

**Summary**: KotaDB centres every interface around a shared `Database` handle that wires file-backed storage, B+ tree and trigram indices, and semantic overlays while the services layer reuses those abstractions for HTTP, CLI, Supabase, and MCP entry points. Repository ingestion, symbol intelligence, and search flows all share the same validation, caching, and observability wrappers so deployments stay consistent regardless of feature flags or transport.

## Step 1 — Compose the Storage and Index Primitives
- `Database::new` allocates the storage directory hierarchy, opens the file store, B+ tree primary index, and binary or text trigram index depending on the flag provided at runtime (`src/database.rs:33`). The struct retains a per-process path cache so repeated lookups avoid index hits (`src/database.rs:24`).
- The file store persists Markdown documents and JSON metadata side-by-side, performing WAL initialisation and metadata hydration during startup (`src/file_storage.rs:48`, `src/file_storage.rs:66`, `src/file_storage.rs:80`). Inserts rewrite or inject YAML frontmatter for tags, recompute hashes, and drive the metadata cache (`src/file_storage.rs:215`).
- The primary index wraps a file-backed B+ tree with WAL durability, lazy load protection, and pattern matching helpers for wildcard queries (`src/primary_index.rs:22`, `src/primary_index.rs:80`, `src/primary_index.rs:93`).
- The trigram index extracts lowercase trigrams, keeps per-document frequency maps, and stores previews to accelerate scoring and snippet assembly during search (`src/trigram_index.rs:22`, `src/trigram_index.rs:90`).
- Every storage handle is wrapped through buffering, caching, retries, validation, and tracing via `create_wrapped_storage`, giving production defaults for correctness and metrics (`src/wrappers.rs:1005`).

| Signature | Purpose | Source |
| --- | --- | --- |
| `async fn insert(&mut self, document: Document) -> Result<()>` | Persist a new document and enforce invariants | `src/contracts/mod.rs:45` |
| `async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>>` | Recover a document for subsequent search/result hydration | `src/contracts/mod.rs:49` |
| `async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>>` | Shared index lookup used by primary and trigram engines | `src/contracts/mod.rs:115` |
| `async fn sync(&mut self) -> Result<()>` | Flush pending writes and propagate WAL durability | `src/contracts/mod.rs:118` |

> **Note** The `tree-sitter-parsing` feature gate extends this foundation with symbol storage, binary relationship engines, and factory helpers declared in `src/lib.rs:32`; the core storage path remains valid without those extras.

## Step 2 — Expose Services to Interface Layers
- `ServicesAppState` carries the shared storage and index mutexes, deployment flags, webhook base URLs, and job registries that back every HTTP handler (`src/services_http_server.rs:62`). Helper methods validate SaaS requirements and guard local path indexing when managed hosting is enabled (`src/services_http_server.rs:82`, `src/services_http_server.rs:98`).
- `create_services_server` wires the canonical Axum router to health, search, indexing, analytics, validation, and benchmarking endpoints while seeding repository state from disk (`src/services_http_server.rs:542`). Each handler snapshots a lightweight `Database` struct that satisfies the shared `DatabaseAccess` trait so services can reuse caching and tracing (`src/services/search_service.rs:20`, `src/services_http_server.rs:3044`).
- Content and symbol search endpoints build a `SearchService`, call the same logic that powers the CLI commands, and return structured results (`src/services_http_server.rs:1095`, `src/services_http_server.rs:3196`). Indexing and analysis routes similarly construct `IndexingService` and `AnalysisService` instances using the shared handles (`src/services_http_server.rs:3068`, `src/services_http_server.rs:1518`).
- When compiled with `mcp-server`, the router registers MCP toolkits over the same storage and index mutexes so automation clients hit the identical code paths (`src/services_http_server.rs:598`).

## Step 3 — Index Source Repositories
- `IndexingService::index_codebase` orchestrates validation, progress logging, and symbol extraction decisions before delegating to the git ingestion pipeline (`src/services/indexing_service.rs:149`). Feature gates ensure symbol extraction toggles align with build-time capabilities (`src/services/indexing_service.rs:175`).
- Repository ingestion sanitises repository identifiers, coordinates file organisation, and streams documents into storage using builders and memory reservations (`src/git/ingestion.rs:72`, `src/git/ingestion.rs:97`). The binary symbol pipeline records relationships via the relationship bridge and symbol writers when enabled (`src/git/ingestion.rs:169`).
- The ingestion flow enforces memory ceilings and adaptive chunking through `MemoryManager`, guarding large repositories from exhausting host resources (`src/memory.rs:23`, `src/memory.rs:67`).
- Incremental updates produced by Supabase webhooks merge path-level diffs so only changed files are reindexed, while removed paths are purged before re-running ingestion (`src/supabase_repository/job_worker.rs:242`, `src/supabase_repository/job_worker.rs:340`).

> **Warning** Building with `git-integration` is required for remote clone support inside the Supabase worker; without it, `prepare_repository` returns an error early (`src/supabase_repository/job_worker.rs:417`).

## Step 4 — Execute Queries and Search
- `SearchService::search_content` routes wildcard queries to the primary index and full-text queries to the trigram index, falling back to the LLM-optimised path when the caller requests broader context (`src/services/search_service.rs:117`). Regular search builds a validated query, selects an index, and hydrates documents from storage in a single place that all interfaces share (`src/services/search_service.rs:299`).
- The trigram index normalises content, filters punctuation, and maintains per-document frequency maps that power efficient coverage and relevance scoring (`src/trigram_index.rs:90`, `src/trigram_index.rs:187`).
- Semantic search layers `VectorIndex::search_knn` on top, providing cosine, Euclidean, or dot-product similarity with persisted HNSW state (`src/vector_index.rs:145`).
- Symbol-aware flows use `AnalysisService` to query the binary relationship engine and translate matches into callers or impact records while preserving line numbers and relationship verbs (`src/services/analysis_service.rs:105`, `src/services/analysis_service.rs:126`).

## Step 5 — Coordinate Background Jobs and SaaS Integrations
- `SupabaseJobWorker::run` polls the job queue, recovers stale work, and hands each job to `process_job` for type-specific handling (`src/supabase_repository/job_worker.rs:124`, `src/supabase_repository/job_worker.rs:217`).
- Indexing jobs merge repository metadata, infer safe repository names, apply incremental diffs, and call the same ingestion routines used locally via `index_repository` (`src/supabase_repository/job_worker.rs:324`, `src/supabase_repository/job_worker.rs:362`).
- Repository metadata and webhook deliveries are updated atomically so SaaS dashboards reflect job status, success metrics, and failures in real time (`src/supabase_repository/job_worker.rs:383`, `src/supabase_repository/job_worker.rs:303`).
- `ServicesAppState` maintains an in-memory job map to expose live progress to HTTP clients while persisting the canonical history through Supabase (`src/services_http_server.rs:76`).

## Step 6 — Safeguards, Observability, and Optimizations
- The wrapper stack layers buffered writes, LRU caching, retry backoff, schema validation, and structured tracing over every storage call, emitting metrics via `log_operation` and `record_metric` hooks (`src/wrappers.rs:23`, `src/wrappers.rs:258`, `src/wrappers.rs:353`, `src/wrappers.rs:696`).
- Logging initialisation honours verbose and quiet modes, integrates with `RUST_LOG`, and raises histograms, counters, and timers through the central observability module (`src/observability.rs:20`, `src/observability.rs:89`, `src/observability.rs:175`).
- Operation contexts carry trace IDs so API handlers (`with_trace_id`) and storage/index wrappers report unified spans across ingestion, search, and Supabase job execution (`src/observability.rs:197`, `src/services_http_server.rs:3059`).
- Validation utilities keep document IDs, paths, and tags consistent across retries and background tasks, which prevents stale metadata from leaking into cache layers (`src/validation.rs` via `src/wrappers.rs:287`).

## Next Steps
- Run `just ci-fast` before shipping architectural changes to confirm wrappers, indices, and services stay aligned.
- Capture representative repository workloads and measure trigram/vector index timings through the existing metrics endpoints to size cache capacities.
- Document any custom feature-flag combinations you deploy so MCP, Supabase, and CLI clients agree on available services.
