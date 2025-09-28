# Migration Guide – KotaDB v0.5.1

## Summary
KotaDB v0.5.1 moves CLI searches to the fast trigram path by setting the default context to `minimal` (`src/main.rs:119`), so LLM augmentation only runs when callers explicitly request `medium` or `full` contexts (`src/services/search_service.rs:131`). This guide covers the updates required across wrappers, services, and validation workflows to preserve the 151× speedup while keeping enhanced analysis available on demand.

## Step 1: Audit CLI entry points
Existing aliases and scripts that relied on implicit LLM analysis now receive trigram-only results unless they pass `--context medium` or `--context full`. Verify every `kotadb search` invocation and set the context flag deliberately so the runtime behavior matches user expectations (`src/main.rs:123`).

### CLI SearchCode signature
| Item | Source | Details |
| `--context` flag | `src/main.rs:116` | Accepts `none`, `minimal`, `medium`, `full` and routes into the shared search service. |
| Default value | `src/main.rs:119` | CLI now defaults to `minimal`, enabling the trigram index path for ordinary queries. |
| Library default | `src/services/search_service.rs:43` | `SearchOptions::default()` still yields `"medium"`, so programmatic callers must override `context` to benefit from the faster mode. |

> **Warning** Libraries or plugins that call `SearchOptions::default()` without overriding `context` will continue triggering the LLM path and pay its latency cost (`src/services/search_service.rs:43`).

```bash
# Force medium analysis when LLM output is required
kotadb search -c medium "async function"
```

## Step 2: Update automation and API clients
Ensure CI scripts, MCP integrations, and other automation explicitly set the desired context. The versioned HTTP endpoint constructs a `SearchRequest` with `search_type: Some("medium")` before dispatching to the service, so API callers must opt into `minimal` if they want the faster behavior (`src/services_http_server.rs:1075`).

> **Note** `SearchRequest.search_type` is forwarded directly into `SearchOptions.context`, so missing or empty values re-enable the legacy LLM pathway (`src/services_http_server.rs:1100`).

### HTTP SearchRequest fields
| Field | Source | Description |
| `search_type` | `src/services_http_server.rs:1075` | Send `"minimal"` for trigram-only responses or `"medium"/"full"` for LLM output. |
| `limit` | `src/services_http_server.rs:1098` | Defaults to `10` if omitted; raise it explicitly for larger result windows. |
| `format` | `src/services_http_server.rs:1076` | Controls response rendering (rich/simple/cli) without affecting search cost. |

```json
{
  "query": "services::search_service",
  "search_type": "minimal",
  "limit": 25,
  "format": "cli"
}
```

## Step 3: Validate search performance characteristics
Trigram filtering now enforces stricter match thresholds—short queries require 100% trigram overlap (`src/trigram_index.rs:773`), mid-length queries demand at least 80% matches (`src/trigram_index.rs:777`), and longer queries must satisfy a 60% floor (`src/trigram_index.rs:783`). When the `aggressive-trigram-thresholds` feature is enabled, the engine relaxes those values progressively if every candidate is filtered out (`src/trigram_index.rs:805`; `Cargo.toml:160`).

Use representative queries to compare fast and LLM-enhanced paths:

```bash
time kotadb search "database::SearchService"
kotadb search -c medium "database::SearchService"
```

The unit suite exercises the same code paths to ensure context-driven behavior stays correct (`tests/search_service_context_modes_test.rs:208`), so keep these commands aligned with your automation.

## Step 4: Re-run regression checks
Run the fast CI target to confirm the documentation and automation changes leave behavior intact:

```bash
just test-fast
```

The CLI behavior validation test asserts that each context level still produces distinct output (`tests/cli_interface_behavior_validation_test.rs:180`), so a green run verifies the migration is wired correctly.

## Troubleshooting
- Medium or full searches returning plain trigram results indicate an LLM failure that triggered the built-in fallback (`src/services/search_service.rs:143`); enable `RUST_LOG=debug` to capture the underlying error before retrying.
- Zero matches after the upgrade usually mean the stricter thresholds filtered everything; consider enabling the fallback feature or refining the query for longer phrases (`src/trigram_index.rs:773`, `src/trigram_index.rs:805`).
- If automation still reports legacy output formatting, confirm it passes the new context flag and that the context differentiation test remains green (`tests/cli_interface_behavior_validation_test.rs:180`).

## Next Steps
- Review every CLI or MCP script and set the context flag explicitly where LLM enrichment is required.
- Update HTTP clients to pass `search_type: "minimal"` when you want parity with the new CLI default.
- Run `just test-fast` before release to confirm context-dependent tests still succeed.
