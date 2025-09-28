# Search Service Validation Report

## Summary
SearchService unifies KotaDB content and symbol lookups across the CLI, HTTP server, and MCP tools by sharing the `DatabaseAccess` trait implementation in `src/main.rs:494` with the async orchestration in `src/services/search_service.rs:117`. Validation confirms that CLI queries route through the same `regular_search` and LLM fallback paths (`src/services/search_service.rs:299` and `src/services/search_service.rs:255`) that power the REST handlers in `src/services_http_server.rs:1063`, while UX regressions are covered by targeted CLI tests (`tests/cli_interface_behavior_validation_test.rs:118`). Symbol queries flow through the binary reader pipeline (`src/services/search_service.rs:175` and `src/binary_symbols.rs:239`), and integration suites keep HTTP behaviors in lockstep (`tests/api_v1_services_server_test.rs:165`).

## Step 1 – Prepare a Searchable Database
- Index a repository with the CLI so Storage, Primary, and Trigram indices are created:
  ```bash
  cargo run --bin kotadb -- -d ./data/analysis index-codebase /path/to/repo
  ```
- `Database::new` wires the shared mutex-backed storage and index handles (`src/main.rs:272`) and exposes them through `DatabaseAccess` for SearchService (`src/main.rs:494`).
- After indexing, the CLI rebuilds path and trigram indices (`src/main.rs:1854`) to guarantee wildcard support before you invoke any search entry point.

## Step 2 – Validate CLI Content Search
- Execute a representative query:
  ```bash
  cargo run --bin kotadb -- -d ./data/analysis search-code "SearchService" --context medium --limit 5
  ```
- CLI command routing instantiates SearchService with the current database handle (`src/main.rs:1605`) and forwards sanitized options parsed by `SearchOptions` (`src/services/search_service.rs:129`).
- `search_content` first filters empty queries, then decides between LLM-enhanced and fast-path searches based on the `context` flag (`src/services/search_service.rs:131`). When LLM mode is chosen, `try_llm_search` builds a context budget and executes `LLMSearchEngine::search_optimized` (`src/services/search_service.rs:255` and `src/llm_search.rs:252`).
- For regular and wildcard queries, `regular_search` constructs an indexed query via `QueryBuilder::with_text` (`src/services/search_service.rs:299` and `src/builders.rs:156`) and routes to the trigram or primary index (`src/services/search_service.rs:329`). Documents are fetched from storage with deterministic ID lookups (`src/services/search_service.rs:350`).
- The CLI formatter surfaces snippets, IDs, and guidance so output never degenerates into bare paths (`src/main.rs:520`). Regression coverage in `tests/cli_interface_behavior_validation_test.rs:118` verifies that medium context yields context-rich details, while `tests/cli_interface_behavior_validation_test.rs:176` asserts each context level produces distinct UX.

| Context flag | Execution path | UX cues |
| --- | --- | --- |
| `none` | Regular search only (`src/services/search_service.rs:157`) | Paths-only output validated by `tests/search_service_context_modes_test.rs:127` |
| `minimal` | Regular search with counters (`src/services/search_service.rs:157`) | Result counts and scores, enforced by `tests/cli_interface_behavior_validation_test.rs:176` |
| `medium` / `full` | LLM fallback with graceful downgrade (`src/services/search_service.rs:131`) | Rich snippets and truncation hints from `format_search_result` (`src/main.rs:573`) |

> **Note** `try_llm_search` respects token budgets defined per context (`src/services/search_service.rs:258`), so large repositories stay within model limits without extra configuration.

## Step 3 – Validate Symbol Search Path
- Ensure the repository was indexed with symbol extraction (`--features tree-sitter-parsing`), then run:
  ```bash
  cargo run --bin kotadb --features "tree-sitter-parsing" -- \
    -d ./data/analysis search-symbols "*Service" --limit 10
  ```
- The CLI guards symbol commands behind the `tree-sitter-parsing` feature and checks for a materialized `symbols.kota` database before dispatch (`src/main.rs:1866`).
- `search_symbols` opens the memory-mapped binary store via `BinarySymbolReader::open` (`src/services/search_service.rs:186` and `src/binary_symbols.rs:239`), iterates packed symbols, applies wildcard-aware matching (`src/services/search_service.rs:206`), and deduplicates hits with a hash set (`src/services/search_service.rs:228`).
- Output formatting reports symbol kinds, file paths, and helpful hints even when no symbols match (`src/main.rs:684`). Automated coverage in `tests/cli_interface_behavior_validation_test.rs:219` ensures the enriched output appears, while `tests/cli_interface_behavior_validation_test.rs:382` asserts the missing-database guidance remains accurate.

## Step 4 – Validate HTTP API Parity
- Start the parity server:
  ```bash
  cargo run --bin kotadb -- serve --port 8077
  ```
- The Axum handlers build a lightweight `Database` shim, instantiate SearchService, and reuse the same search paths as the CLI (`src/services_http_server.rs:1063` for `/api/v1/search/code` and `src/services_http_server.rs:1126` for `/api/v1/search/symbols`).
- Responses are serialized through the shared renderer so the JSON structure mirrors CLI semantics (`src/services_http_server.rs:1110`). Integration test `tests/api_v1_services_server_test.rs:165` indexes a throwaway repo and exercises the `search/code` endpoint end-to-end.
- Symbol routes gracefully handle absent databases by returning helpful errors (`src/services_http_server.rs:1320`), and `tests/api_v1_services_server_test.rs:229` confirms the behavior without symbol assets.

## Step 5 – Inspect LLM Context Handling
- Context presets (`none`, `minimal`, `medium`, `full`) map to `ContextConfig` values with increasing token budgets and snippet lengths (`src/services/search_service.rs:258`).
- `LLMSearchEngine::search_optimized` ranks documents by relevance and prunes them to fit the token budget while logging diagnostics (`src/llm_search.rs:252`).
- The SearchService fallback path maintains resilience; `tests/search_service_context_modes_test.rs:127` checks that `context = "none"` stays on the fast path, while the same suite tracks that `minimal` avoids LLM usage unless explicitly requested (`tests/search_service_context_modes_test.rs:170`).

## Step 6 – Confirm Automated Coverage
- CLI happy-path, limit handling, and quiet-mode behaviors are covered in `tests/cli_defaults_validation_test.rs:127` and `tests/cli_interface_behavior_validation_test.rs:261`.
- End-to-end journeys (`tests/e2e/test_codebase_analysis_journey.rs:47`) validate that search results feed downstream analysis workflows without manual wiring.
- Performance regression safeguards keep query latency in check (`tests/performance_regression_test_596.rs:340`), ensuring future refactors preserve the current <50 ms fast-path target validated in `tests/search_service_context_modes_test.rs:150`.

## Next Steps
- Re-run `just test-fast` before release to execute the focused CLI, HTTP, and performance suites.
- Add HTTP contract tests for the LLM-rich format once the MCP server surfaces that payload externally.
- Extend symbol search docs with concrete examples once additional language grammars land behind the `tree-sitter-parsing` feature flag.
