---
title: "KOTA Query Language (KQL) Design"
tags: [database, query-language, design]
related: ["IMPLEMENTATION_PLAN.md", "TECHNICAL_ARCHITECTURE.md", "DATA_MODEL_SPECIFICATION.md"]
key_concepts: [query-language, natural-language, graph-queries, temporal-queries]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

## Summary
KotaDB search today is a pragmatic pipeline that unifies the CLI, HTTP API, and MCP tooling behind one sanitized `Query` object. Every request funnels through `SearchService`, which fans out to the trigram text index for content queries, the primary index for wildcard path scans, and optionally an LLM-backed summarizer. Natural-language or temporal query operators are not implemented yet; the current surface area is text search, path wildcards, vector-ready scaffolding, and symbol lookup.

## Step 1 - Capture Requests Across Interfaces
- The CLI dispatches `kotadb search-code` into `SearchService::search_content` (`src/main.rs:1644`), using the same database handle as long-lived servers.
- SaaS and self-hosted HTTP clients hit `POST /api/v1/search/code`, which deserializes into `SearchRequest` (`src/services_http_server.rs:316`) before calling the same service path (`src/services_http_server.rs:1063`).
- MCP integrations reuse the service through `SearchService::new` (`src/mcp/services_tools.rs:402`), keeping the contract identical to human-facing tooling.

| Field | Type | Source |
| --- | --- | --- |
| `query` | `String` | `src/services_http_server.rs:317` |
| `limit` | `Option<usize>` | `src/services_http_server.rs:318` |
| `search_type` | `Option<String>` | `src/services_http_server.rs:319` |
| `format` | `Option<String>` | `src/services_http_server.rs:320` |

Example CLI invocation that exercises the same path:

```bash
cargo run --bin kotadb -- search-code "SearchService" --limit 5 --context medium
```

## Step 2 - Normalize Query Data
- `SearchService` wraps raw inputs in `SearchOptions` before constructing a fluent `QueryBuilder` (`src/services/search_service.rs:118`, `src/services/search_service.rs:299`).
- `QueryBuilder::with_text` switches between `sanitize_search_query` and `sanitize_path_aware_query` to keep SQL or shell payloads out while preserving wildcards (`src/builders.rs:156`, `src/query_sanitization.rs:103`, `src/query_sanitization.rs:278`).
- Limits and tags are validated with strong types (`ValidatedLimit`, `ValidatedTag`) so downstream indexes never see unbounded requests (`src/builders.rs:221`).

| Field | Meaning | Source |
| --- | --- | --- |
| `search_terms` | Sanitized tokens for trigram lookup | `src/contracts/mod.rs:197` |
| `tags` | Lowercased tags (currently advisory) | `src/contracts/mod.rs:199` |
| `path_pattern` | Preserved wildcard/glob string | `src/contracts/mod.rs:200` |
| `limit` | Upper-bounded result window | `src/contracts/mod.rs:201` |
| `offset` | Reserved for paging (1-based) | `src/contracts/mod.rs:202` |

> **Note**: Feature flags such as `strict-sanitization` and `aggressive-trigram-thresholds` (see `Cargo.toml:154`) tighten these guardrails when you need stricter production policies.

## Step 3 - Select an Execution Path
- `SearchService::regular_search` checks for `*` in the final query string and routes wildcard requests to the primary B+ tree index (`src/services/search_service.rs:329`).
- Primary index lookups walk the persisted path tree and apply the same glob matcher used by the CLI (`src/primary_index.rs:707`). This path powers directory scans like `kotadb search "docs/*"` without touching document payloads.
- Text queries remain in the trigram engine, which extracts all three-character shingles, accumulates candidate hits, and enforces a match threshold (`src/trigram_index.rs:723`).

| Index | Trigger | Behavior | Source |
| --- | --- | --- | --- |
| Primary | Wildcard `*` in query/path | Glob match on stored paths | `src/primary_index.rs:729` |
| Trigram | Plain text | Trigram frequency ranking with guard rails | `src/trigram_index.rs:734` |

## Step 4 - Build Responses
- After index resolution, document IDs are fetched from storage inside the same task (`src/services/search_service.rs:351`). The service only retains the first `limit` items, so upper bounds stay enforced even if the index returned more.
- `SearchResult` captures raw documents alongside optional LLM output, and the `SearchType` enum lets clients know whether results are wildcard, regular, or LLM-optimized (`src/services/search_service.rs:71`, `src/services/search_service.rs:96`).
- When callers request `context` = `medium` or `full`, `try_llm_search` runs `LLMSearchEngine::search_optimized`, which reuses trigram hits, scores them, and generates token-budgeted snippets (`src/services/search_service.rs:255`, `src/llm_search.rs:252`). On failure, the code falls back to the plain trigram path without surfacing an error (`src/services/search_service.rs:142`).
- HTTP responses flow through `render_search_code_response`, but the payload ultimately mirrors `SearchResult`, keeping the wire format aligned with CLI output (`src/services_http_server.rs:1090`).

> **Note**: LLM search still depends on trigram candidates; there is no separate semantic index in the live pipeline. Logs tagged with `search_type == LLMOptimized` make this distinction explicit.

## Step 5 - Extend With Specialized Modules
### Symbol search
- `search_symbols` scans the symbol database emitted by the indexer, matching wildcards or substrings against Tree-sitter-derived symbol names (`src/services/search_service.rs:174`).
- The routine deduplicates by `(name, path, line)` and respects optional `symbol_type` filters so API and CLI callers see deterministic results (`src/services/search_service.rs:205`).

### Semantic infrastructure
- `SemanticSearchEngine` wires document storage, the vector index, and the embedding service (`src/semantic_search.rs:16`). It performs linear KNN search today (`src/vector_index.rs:146`) because the HNSW traversal is still stubbed out.
- No public route invokes `semantic_search` yet; integrating it requires provisioning embeddings (feature `embeddings-onnx`) and plumbed endpoints.

### Intent-based overlay
- The Intent MCP server parses conversational prompts into structured intents but ultimately calls the same REST endpoints through `QueryOrchestrator` (`src/intent_mcp_server.rs:15`, `src/intent_mcp_server.rs:105`). Natural-language parsing is therefore an integration concern, not part of the database query core.

## Next Steps
- Exercise both wildcard and text paths locally with `cargo run --bin kotadb -- search-code` and verify logs for `SearchType` transitions.
- Benchmark trigram thresholds with and without `--features aggressive-trigram-thresholds` to validate precision in your corpus.
- If you plan to light up semantic search, provision embeddings via `just dev` and wire the pending `SemanticSearchEngine` into a bespoke endpoint before exposing it publicly.
