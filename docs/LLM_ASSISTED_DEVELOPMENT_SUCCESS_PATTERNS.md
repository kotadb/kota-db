# LLM-Assisted Development Success Patterns

## Summary
KotaDB's agent workflow anchors on the same contract-first storage and indexing components used by the CLI, then feeds natural-language intents into the HTTP services via the Model Context Protocol. `MCPServer::new` wires fully wrapped storage, primary, and trigram indexes into one tool registry so that every tool invocation shares the same caches and WAL (`src/mcp/server.rs:94`, `src/file_storage.rs:514`, `src/primary_index.rs:856`, `src/trigram_index.rs:1079`). `IntentMcpServer::process_query` converts typed intents into HTTP calls and conversational summaries while preserving session context for follow-up prompts (`src/intent_mcp_server.rs:198`, `src/intent_mcp_server.rs:780`). Stage 6 wrappers add tracing, validation, retries, and buffering around every storage operation, which is why agents consistently fall into the pit of success (`src/wrappers.rs:23`, `src/wrappers.rs:1005`).

## Step-by-Step: Construct Agent-Safe Runtime Contracts
1. Anchor every workflow on the Stage 2 contracts exported in `src/contracts/mod.rs:20` so agents pass data through the `Storage` and `Index` traits instead of raw filesystem paths.
2. Instantiate storage and indexes through the Stage 6 factories `create_file_storage`, `create_primary_index`, and `create_trigram_index`; each factory wraps buffering, caching, retries, validation, and metrics around the raw engines before handing them to callers (`src/file_storage.rs:514`, `src/primary_index.rs:856`, `src/trigram_index.rs:1079`, `src/wrappers.rs:1005`).
3. Let `MCPServer::new` bind those shared instances into the tool registry and deletion service so every tool call sees the same `Arc<Mutex<_>>` handles and cache state (`src/mcp/server.rs:94`, `src/mcp/server.rs:125`).
4. Rely on `CoordinatedDeletionService::delete_document` to remove documents from storage and both indexes with automatic rollback when any stage fails, preventing orphaned entries that would confuse agents (`src/coordinated_deletion.rs:21`, `src/coordinated_deletion.rs:41`).
5. Register only the capabilities your session needs by composing `MCPToolRegistry::with_text_tools` and, when relationship graphing is required, the `with_relationship_tools`/`with_symbol_tools` extensions guarded by feature flags (`src/mcp/tools/mod.rs:43`, `src/mcp/tools/mod.rs:62`).
6. Expose the runtime through `MCPServer::start_sync` or the `kotadb-intent-mcp` binary so human and automated clients reuse the same transport surface during handoffs (`src/mcp/server.rs:208`, `src/bin/intent_mcp_server.rs:110`).

## Step-by-Step: Route Natural Language Through Verified Services
1. Classify every prompt with `IntentParser::parse`, which detects search, analysis, navigation, overview, and debugging intents using the regex patterns baked into the parser (`src/intent_mcp_server.rs:321`, `src/intent_mcp_server.rs:407`).
2. Handle each request through `IntentMcpServer::process_query`; it fetches the prior session context, executes the orchestrator, and logs the resulting `IntentResponse` so conversations stay stateful (`src/intent_mcp_server.rs:198`, `src/intent_mcp_server.rs:220`).
3. Allow `QueryOrchestrator::execute_intent` to map intents onto the HTTP endpoints that power the CLI and API, including `/api/code/search`, `/api/symbols/search`, and `/stats` (`src/intent_mcp_server.rs:638`, `src/intent_mcp_server.rs:668`).
4. Downstream, `SearchService::search_content` and its LLM optimizations decide whether to execute regular trigram queries or context-aware reranking, guaranteeing the MCP flow matches the CLI behaviour (`src/services/search_service.rs:39`, `src/services/search_service.rs:61`).
5. Return helpful follow-ups by letting `generate_suggestions` and `ContextManager::update_context` learn from the latest results before responding (`src/intent_mcp_server.rs:265`, `src/intent_mcp_server.rs:780`).
6. Validate the full path locally with `cargo run --bin kotadb-intent-mcp -- --interactive` or `just mcp` so agents and humans can sample the exact MCP contract exposed in production (`src/bin/intent_mcp_server.rs:10`, `justfile:19`).

## Step-by-Step: Reinforce Observability and Risk Controls
1. Wrap every storage call with `TracedStorage`, which records operation counts plus structured metrics for open/read/write timings (`src/wrappers.rs:23`, `src/wrappers.rs:68`).
2. Keep operations resilient under transient faults by composing `RetryableStorage` and `ValidatedStorage`; retries add exponential backoff while validation guards against inconsistent updates (`src/wrappers.rs:353`, `src/wrappers.rs:274`).
3. Smooth write bursts with `BufferedStorage`, whose background flusher intentionally disables itself inside CI to avoid hanging tests while still batching writes in production (`src/wrappers/buffered_storage.rs:15`, `src/wrappers/buffered_storage.rs:70`).
4. Exercise the real indexes under concurrency using the mixed query stress test that routes hundreds of wildcard and trigram searches through the same optimized handles agents call (`tests/query_routing_stress.rs:147`, `tests/query_routing_stress.rs:261`).
5. Mirror CI locally with `just test-fast`, which runs `cargo nextest` and doctests under the `git-integration`, `tree-sitter-parsing`, and `mcp-server` feature set that production requires (`justfile:40`, `justfile:41`).
6. Smoke-test the MCP bootstrap itself via the asynchronous unit tests that instantiate the server, ensuring tool registration stays healthy before agents attach (`src/mcp/server.rs:473`, `src/mcp/server.rs:486`).

> **Note** Relationship and symbol MCP tools are compiled only when the `tree-sitter-parsing` feature is enabled; without it the registry purposely exposes search-only capabilities (`src/mcp/server.rs:149`, `src/mcp/tools/mod.rs:35`).

## Key Interfaces
| Symbol | Description | Path |
| --- | --- | --- |
| `Intent` | Categorizes prompts into search, analysis, navigation, overview, and debugging tasks for orchestration. | `src/intent_mcp_server.rs:18` |
| `IntentResponse` | Returns structured results, summaries, and suggestions to agents after each query. | `src/intent_mcp_server.rs:166` |
| `MCPToolRegistry::handle_tool_call` | Dispatches `kotadb://` tool invocations to enabled handlers while enforcing feature gates. | `src/mcp/tools/mod.rs:84` |
| `TextSearchTools::handle_call` | Runs trigram-backed search through shared storage/index handles when embeddings are disabled. | `src/mcp/tools/text_search_tools.rs:36` |

## Feature Flags
- `tree-sitter-parsing` enables relationship and symbol tools in the MCP registry and related analysis services (`src/mcp/server.rs:149`, `src/mcp/tools/mod.rs:35`).
- `mcp-server` compiles the MCP transport, tool registry, and CLI entrypoints; commands like `just mcp` and `just test-fast` build with this flag set (`justfile:19`, `justfile:41`).
- `git-integration` unlocks repository ingestion and Supabase jobs that provide commit context to LLM flows, and is part of the default feature set for CI and release builds (`src/git/repository.rs:12`, `Cargo.toml:155`).

## Next Steps
- Run `just mcp` and attach your agent to verify tool registration and search flows against a real repository.
- Execute `just test-fast` before introducing new automation instructions so MCP, git, and tree-sitter features stay green locally.
- Capture any new agent workflows in the MCP-focused tests (e.g., extend `tests/query_routing_stress.rs`) to preserve the pit of success for the next handoff.
