# MCP Implementations Guide

## Summary
KotaDB ships two complementary MCP entry points: the spec-compliant streamable HTTP transport that fronts the JSON-RPC tool registry, and a compatibility bridge that keeps the older `/mcp/tools/*` REST surface alive. Both layers share the same configuration, tool registry, and storage plumbing that are assembled by `MCPServer::new` before any route is exposed (`src/mcp/server.rs:94`).

## Step 1 – Load the MCP configuration
KotaDB centralises MCP server settings in `MCPConfig` (`src/mcp/config.rs:5`). Populate or override these fields before starting any transport.

| Field | Purpose | Source |
| --- | --- | --- |
| `server.host`, `server.port`, `max_connections` | Socket binding and connection limits for the streamable HTTP listener | `src/mcp/config.rs:15` |
| `mcp.enable_search_tools`, `mcp.enable_relationship_tools` | Feature switches that decide which tool handlers are registered | `src/mcp/config.rs:34` |
| `security.allowed_origins` | Allowed CORS origins; enforced for both POST and SSE handshakes | `src/mcp/config.rs:58`, `src/mcp/streamable_http.rs:69` |

`MCPConfig::default()` already disables document tools per the codebase-intelligence migration and pins protocol version `2025-06-18` (`src/mcp/config.rs:83`). Environment variables such as `MCP_SERVER_HOST`, `MCP_SERVER_PORT`, and `KOTADB_DATA_DIR` transparently override the TOML values (`src/mcp/config.rs:121`).

## Step 2 – Build the tool registry and feature gates
`MCPServer::new` provisions shared storage, primary, and trigram indexes, then wires handlers into an `MCPToolRegistry` instance (`src/mcp/server.rs:101` through `src/mcp/server.rs:188`). The registry itself dispatches calls based on URI prefixes and feature flags (`src/mcp/tools/mod.rs:35`).

| Method prefix | Handler & Behaviour | Notes |
| --- | --- | --- |
| `kotadb://text_search` | Uses `TextSearchTools::text_search` to walk the trigram index and hydrate previews from storage (`src/mcp/tools/text_search_tools.rs:35`, `src/mcp/tools/text_search_tools.rs:70`) | Enabled when `enable_search_tools` is true |
| `kotadb://symbol_search` | Wraps `SearchService::search_symbols` via `SymbolTools` to provide wildcard-aware symbol lookups (`src/mcp/tools/symbol_tools.rs:29`) | Requires `tree-sitter-parsing` and `enable_relationship_tools` (`src/mcp/server.rs:177`) |
| `kotadb://find_callers` / `kotadb://impact_analysis` | Call into `AnalysisService` to compute relationship graphs (`src/mcp/tools/relationship_tools.rs:223`, `src/mcp/tools/relationship_tools.rs:246`) | Also gated by `tree-sitter-parsing`; returns structured markdown payloads |

> **Note** Some relationship methods are placeholders until later phases (`src/mcp/tools/relationship_tools.rs:254`). Expect explicit `TODO` errors for `find_callees`, `call_chain`, and additional analytics endpoints when the backing `AnalysisService` functions are absent.

## Step 3 – Serve the MCP Streamable HTTP transport
The primary MCP surface mounts at `/mcp` using `create_streamable_http_router` (`src/mcp/streamable_http.rs:40`). Requests traverse these guardrails:

1. **Handshake validation** – Every POST must advertise the negotiated `Accept` header and protocol version, enforced by `validate_accept_for_post` and `extract_protocol_version` (`src/mcp/streamable_http.rs:143`, `src/mcp/streamable_http.rs:80`). The server falls back to its configured protocol but logs a warning when the header is missing.
2. **Session negotiation** – `process_method_call` creates a session and returns server capabilities on `initialize`, optionally echoing a `mcp-session-id` header used by follow-up calls and SSE subscriptions (`src/mcp/streamable_http.rs:380`).
3. **Tool execution** – Subsequent `tools/list` and `tools/call` invocations require the session header and delegate into the registry via `handle_tool_call`, wrapping JSON-RPC results into MCP-friendly `content` envelopes (`src/mcp/streamable_http.rs:394`, `src/mcp/streamable_http.rs:452`).
4. **Event streaming** – Clients upgrade to SSE by calling `GET /mcp` with the same protocol and session headers; `handle_streamable_get` replays backlog entries and keeps the channel alive with periodic keep-alives (`src/mcp/streamable_http.rs:283`). The backing `SessionManager` keeps a bounded event queue and enforces session lookups (`src/mcp/streamable_http.rs:560`).

Legacy `/mcp/tools/*` routes remain mounted inside the same router so older agents still hit the shared registry through `call_tool_legacy` (`src/mcp/streamable_http.rs:534`).

## Step 4 – Maintain the MCP-over-HTTP bridge
The bridge router exposes convenience REST endpoints for hosted environments (`src/mcp_http_bridge.rs:84`). Each handler resolves the active `MCPToolRegistry`, maps human-friendly tool names to protocol URIs, and returns stable error payloads.

| HTTP endpoint | Invoked MCP method | Source |
| --- | --- | --- |
| `GET/POST /mcp/tools` | Enumerates registered tools for discovery | `src/mcp_http_bridge.rs:98` |
| `POST /mcp/tools/:tool_name` | Resolves to `kotadb://text_search`, `kotadb://symbol_search`, etc. | `src/mcp_http_bridge.rs:188`, `src/mcp_http_bridge.rs:485` |
| `POST /mcp/tools/search_code` | Short-circuits to `kotadb://text_search` with raw JSON arguments | `src/mcp_http_bridge.rs:210` |
| `POST /mcp/tools/find_callers` | Delegates to relationship tools when the feature flag is on | `src/mcp_http_bridge.rs:363` |

Fallback behaviour is explicit: a missing registry returns `registry_unavailable`, disabled features raise `feature_disabled`, and tool handlers bubble up `internal_error` (`src/mcp_http_bridge.rs:324`, `src/mcp_http_bridge.rs:347`). The same code path powers both unauthenticated dev servers and authenticated SaaS deployments because the state object simply toggles which registry is passed in (`src/services_http_server.rs:603`, `src/http_server.rs:675`).

## Step 5 – Run the servers
During development, `just dev` hot-reloads the streamable HTTP server with the required feature flag (`justfile:15`). For a single-shot run with verbose logging, use `just mcp` (`justfile:19`). These tasks invoke the `kotadb-mcp` binary defined in `src/bin/mcp_server.rs`, which loads configuration, applies CLI overrides, and starts the blocking listener (`src/bin/mcp_server.rs:68`, `src/bin/mcp_server.rs:138`).

For environments that need STDIO transport instead of HTTP, launch `cargo run --bin mcp_server_stdio --features mcp-server -- --config kotadb-mcp-dev.toml`, which uses the simplified loop in `run_stdio_server` to exchange JSON-RPC messages over stdin/stdout (`src/bin/mcp_server_stdio.rs:63`).

Hosted API servers embed the bridge automatically: `create_services_server` merges `/mcp/tools/*` without auth for local runs, while `create_services_saas_server` wraps the same router with API key middleware (`src/services_http_server.rs:596`, `src/services_http_server.rs:776`). The SaaS HTTP entrypoint logs the MCP routes at startup so operators know which URLs are live (`src/http_server.rs:768`).

## Step 6 – Exercise and test the endpoints
Run the fast MCP-aware test suite with `just test-fast` to cover unit tests and doctests under the `mcp-server` and `tree-sitter-parsing` features (`justfile:40`). The bridge module also ships direct coverage for tool routing—`cargo nextest run mcp_http_bridge::tests --lib` exercises the static list and error handling (`src/mcp_http_bridge.rs:535`).

To smoke test the streamable transport, ensure the negotiated headers are honoured:

```bash
curl -sS http://localhost:8484/mcp \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

> **Warning** If the client omits the `MCP-Protocol-Version` header or the dual `Accept` media types, the server will terminate the handshake with a `406` or `400` generated by `validate_accept_for_post` and `extract_protocol_version` (`src/mcp/streamable_http.rs:143`, `src/mcp/streamable_http.rs:80`).

## Next Steps
- Validate your configuration with `cargo run --bin mcp_server --features mcp-server -- --health-check` before deploying (`src/bin/mcp_server.rs:81`).
- Enable `tree-sitter-parsing` and confirm relationship tooling via `POST /mcp/tools/find_callers` once symbol indexing is available (`src/mcp_http_bridge.rs:363`).
- Extend the registry with new tool handlers by implementing `MCPToolHandler` and registering them in `MCPServer::new` (`src/mcp/tools/mod.rs:24`).
- Record curl transcripts when shipping changes so downstream assistant integrations can trace MCP negotiations end to end.
