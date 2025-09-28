# Stage 6: Component Library

## Summary
Stage 6 hardens KotaDB by routing every storage-facing call through validated types, fluent builders, and composable wrappers so invalid inputs never reach disk and cross-cutting guarantees (tracing, retries, caching) are opt-in by default. The components documented here reflect the current implementation; follow the referenced modules to trace runtime behavior.

## Step 1 – Harden Domain Primitives
- Validated types eliminate ad-hoc checks and centralize sanitization. They defer to the Stage 2 validation layer where appropriate, so you always get the same invariants whether the value came from a builder, an HTTP request, or a test harness.
- Reference the table below when you need to understand which invariant makes a constructor fail and which downstream services depend on it.

| Type | Purpose & Invariants | Reference |
| --- | --- | --- |
| `ValidatedPath` | Rejects empty, absolute, URL-encoded, or traversal-prone paths before they enter storage APIs; delegates to `validation::path::validate_file_path` for shared logic. | `src/types.rs:34`, `src/validation.rs:63`
| `ValidatedDocumentId` | Refuses nil UUIDs and provides ergonomic parsing/generation so IDs are unique before persistence. | `src/types.rs:80`
| `ValidatedTitle` | Trims whitespace, enforces `<=1024` chars, and pairs with the Stage 2 validator that re-checks title length on inserts. | `src/types.rs:124`, `src/validation.rs:325`
| `ValidatedTimestamp` / `TimestampPair` | Ensures timestamps are post-epoch, not absurdly far in the future, and that updates never precede creation. | `src/types.rs:185`, `src/types.rs:224`
| `ValidatedSearchQuery` | Pipes text through `query_sanitization::sanitize_search_query`, blocking control characters, injection payloads, and overlong inputs, with optional strict filtering when the `strict-sanitization` feature is enabled. | `src/types.rs:303`, `src/query_sanitization.rs:103`
| `ValidatedTag`, `ValidatedLimit`, `ValidatedPageId` | Normalize tag inputs, bound result sets (`<=100_000`), and keep pagination IDs positive to avoid undefined backend behavior. | `src/types.rs:270`, `src/types.rs:372`

> **Note** Stage 1 contracts still re-run key checks (for example `validation::document::validate_for_insert`) so even manually constructed `Document` values are re-validated on the way into storage (`src/validation.rs:289`).

## Step 2 – Guard Document Lifecycle
- `state::TypedDocument` enforces compile-time state transitions between draft, persisted, and modified documents so callers cannot skip intermediate steps (`src/types.rs:399`). `TimestampPair::touch` keeps modification times monotonic as part of the transition logic (`src/types.rs:264`).
- The runtime validators mirror those guarantees in persistence layers: `ValidatedStorage::insert` reuses `validation::document::validate_for_insert`, while updates compare timestamps and IDs via `validate_for_update` (`src/wrappers.rs:287`, `src/validation.rs:340`).
- When you need to reason about lifecycle bugs, search for `TypedDocument` transitions and the places where `TimestampPair` is read—any deviation from these patterns generally means a precondition is missing.

## Step 3 – Assemble Domain Objects via Builders
- `DocumentBuilder` turns raw bytes into `Document` contracts by wiring validated primitives, hashing content with the pure metadata helper, and deriving default timestamps if you omit them (`src/builders.rs:22`, `src/builders.rs:88`, `src/pure/metadata.rs:45`).
- `QueryBuilder` separates wildcard path patterns from full-text terms, applies path-aware sanitization when slashes or globbing characters appear, and rehydrates `ValidatedTag` values so contract-level filtering works identically across CLI and API entry points (`src/builders.rs:145`, `src/builders.rs:249`).
- Config builders (`StorageConfigBuilder`, `IndexConfigBuilder`, `MetricsBuilder`) provide fluent, validated assembly for operational settings and telemetry snapshots (`src/builders.rs:297`, `src/builders.rs:385`, `src/builders.rs:471`). Required fields (like storage paths) deliberately error at `build()` time so misconfigured environments fail fast.

```rust
use kotadb::{DocumentBuilder, QueryBuilder};

let doc = DocumentBuilder::new()
    .path("docs/howto.md")?
    .title("How KotaDB defends itself")?
    .content(b"# Risk reduction\n")
    .build()?; // hashes content and timestamps automatically

let query = QueryBuilder::new()
    .with_text("*.rs")?        // becomes a path pattern
    .with_tag("risk")?         // sanitized and re-validated
    .with_limit(25)?
    .build()?;
```

The builders surface the same errors you would hit deeper in the stack: `ValidatedPath::new` rejects traversal paths, `sanitize_path_aware_query` strips encoded `..` segments, and `ValidatedLimit::new` clamps excessive requests. That symmetry keeps regression tests focused on a single surface.

## Step 4 – Apply Storage Wrappers in Production
Stage 6’s storage wrappers are layered deliberately; follow the order below to understand what happens to every CRUD call:

1. **Write buffering** (`BufferedStorage` queues inserts/updates/deletes, flushing on size, memory, or interval thresholds; background workers disable themselves automatically in CI via the environment gates at `src/wrappers/buffered_storage.rs:78`).
2. **In-memory caching** (`CachedStorage` adds an LRU keyed by document ID, tracks hit/miss counters, and invalidates on writes so stale data never leaks (`src/wrappers.rs:697`)).
3. **Retry envelope** (`RetryableStorage` wraps `Storage` trait calls in exponential backoff with jitter, logging every retry attempt via `tracing` (`src/wrappers.rs:354`)).
4. **Validation firewall** (`ValidatedStorage` re-checks inserts/updates and guards against duplicate IDs, referencing the shared validator set (`src/wrappers.rs:258`)).
5. **Tracing and metrics** (`TracedStorage` binds operations to a UUID, records histograms via the observability layer, and publishes structured logs for each method (`src/wrappers.rs:23`, `src/observability.rs:89`)).

`create_wrapped_storage` wires those layers, returning a `TracedStorage<…BufferedStorage<S>>` stack (`src/wrappers.rs:1010`). Higher-level factories call it directly: `create_file_storage` wraps the on-disk engine before returning a handle (`src/file_storage.rs:514`), and `Database::new` replaces the raw storage handle with the fully wrapped variant to guarantee the same behavior for HTTP, MCP, and CLI front ends (`src/database.rs:88`).

> **Warning** The `SafeTransaction` RAII guard is intentionally commented out until the storage layer exposes a concrete `Transaction` implementation (`src/wrappers.rs:958`). Do not rely on it for cleanup semantics yet.

## Step 5 – Instrument Index and Optimization Layers
- `MeteredIndex` records per-operation latency, emitting histogram metrics each time an index call finishes and providing aggregated min/avg/max summaries when the wrapper is dropped (`src/wrappers.rs:820`). It implements the full `Index` trait, so swapping it in requires no changes upstream.
- `OptimizedIndex` adds concurrency control, bulk-operation scaffolding, and cached tree metrics for feature-flagged index engines (`src/wrappers/optimization.rs:25`). Its `OptimizationConfig` exposes knobs for batching thresholds, adaptive caching, and rebalancing triggers. When the tree or contention metrics indicate drift, the wrapper’s `analyze_and_optimize` path highlights recommended actions (`src/wrappers/optimization.rs:180`).
- Both wrappers integrate with the shared `OptimizationMetricsCollector`, which is where long-term telemetry accumulates for `just coverage` and performance regression dashboards.

## Step 6 – Validate with Tests and Runtime Usage
- Unit tests inside the component modules cover the most critical invariants: path traversal rejection, wildcard query routing, and wrapper stacking behavior (`src/types.rs:505`, `src/builders.rs:555`, `src/wrappers.rs:1087`). Use them as references when extending invariants.
- Integration tests exercise the same paths end-to-end. For example, `tests/security_path_traversal_test.rs:1` proves `ValidatedPath` and `DocumentBuilder` reject hostile paths, while the services layer spins up wrapped storage in the MCP and HTTP servers via `create_wrapped_storage` to ensure middleware observes traced, validated mutations (`tests/connectinfo_middleware_test.rs:44`, `src/http_server.rs:1855`).
- Because every builder and wrapper shares the same constructors, running `just test-fast` gives quick assurance that new feature flags or validation rules haven’t broken the staging pipeline.

## Next Steps
- Run `just test-fast` before and after editing validated types or wrappers to catch regressions quickly.
- When adding new storage backends, integrate them through `create_wrapped_storage` so they inherit buffering, caching, validation, and tracing automatically.
- Extend `OptimizedIndex` once the underlying index exposes richer metrics—its configuration hooks are ready for new rebalancing or caching strategies.
- Mirror any new invariants or wrappers back into this guide with file/line references so implementation and documentation stay synchronized.
