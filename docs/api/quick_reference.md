# KotaDB Quick Reference

**Summary**  
Use KotaDB’s Stage 6 stack to compose validated content builders, resilient storage, and tracing-first observability exactly as implemented in the crate.

## Step 1: Provision the storage stack
- Instantiate the disk-backed engine with `create_file_storage` and let it compose buffering, caching, retries, validation, and tracing in the documented order.
- Prefer supplying a cache budget that mirrors your working set while leaving buffering defaults enabled for WAL-backed durability.
- Initialize storage once per process and share the resulting wrapper via `Arc<Mutex<_>>` or service constructors to reuse the buffered worker.

```rust
use kotadb::create_file_storage;

let mut storage = create_file_storage("/var/lib/kotadb", Some(2_000)).await?;
```

| API | Signature | Location | Details |
| --- | --- | --- | --- |
| `create_file_storage` | `pub async fn create_file_storage(path: &str, cache_capacity: Option<usize>) -> Result<impl Storage>` | `src/file_storage.rs:514` | Opens `FileStorage`, then wraps it with buffering, caching, retries, validation, and tracing using `create_wrapped_storage`. |
| `create_wrapped_storage` | `pub async fn create_wrapped_storage<S: Storage>(inner: S, cache_capacity: usize) -> FullyWrappedStorage<S>` | `src/wrappers.rs:1010` | Applies `BufferedStorage → CachedStorage → RetryableStorage → ValidatedStorage → TracedStorage` so every `Storage` call is traced and validated. |
| `BufferedStorage::new` | `pub fn new(inner: S) -> Self` | `src/wrappers/buffered_storage.rs:65` | Batches writes with WAL-backed flush logic; CI and test runs disable background tasks automatically. |

> **Note** WAL-backed buffering flips to a synchronous path when `KOTADB_DISABLE_BUFFER_TASKS=1`, matching the CI safeguards coded in `src/wrappers/buffered_storage.rs:85`.

## Step 2: Build validated documents
- Use `DocumentBuilder` to assemble validated documents; path, title, and content validation all flow through the `types` module before persistence.
- Attach tags, timestamps, or deterministic IDs as needed; the builder computes hashes and word counts via `crate::pure::metadata::calculate_hash`.
- Inspect the resulting `Document` struct for persisted field meanings before storing.

```rust
use kotadb::DocumentBuilder;

let doc = DocumentBuilder::new()
    .path("knowledge/design.md")?
    .title("Design Notes")?
    .content(b"# Design\n\nDecisions…")
    .tag("architecture")?
    .build()?; // Hash, timestamps, and word count resolved here.
```

| Builder API | Signature | Location | Purpose |
| --- | --- | --- | --- |
| `DocumentBuilder::new` | `pub fn new() -> Self` | `src/builders.rs:22` | Starts a builder with optional ID, timestamps, and empty tag set. |
| `DocumentBuilder::path` | `pub fn path(self, path: impl AsRef<Path>) -> Result<Self>` | `src/builders.rs:50` | Validates file-system safety through `ValidatedPath::new`. |
| `DocumentBuilder::title` | `pub fn title(self, title: impl Into<String>) -> Result<Self>` | `src/builders.rs:56` | Enforces trimmed, ≤1024 character titles via `ValidatedTitle`. |
| `DocumentBuilder::timestamps` | `pub fn timestamps(self, created: i64, updated: i64) -> Result<Self>` | `src/builders.rs:80` | Ensures ordered timestamps with `TimestampPair::new`. |
| `DocumentBuilder::build` | `pub fn build(self) -> Result<Document>` | `src/builders.rs:88` | Finalizes validation, auto-hashing content, and defaults missing timestamps/IDs. |

| `Document` Field | Type | Location | Notes |
| --- | --- | --- | --- |
| `id` | `ValidatedDocumentId` | `src/contracts/mod.rs:130` | Nil UUIDs rejected (`src/types.rs:92`). |
| `path` | `ValidatedPath` | `src/contracts/mod.rs:131` | Guards traversal via `validation::path::validate_file_path` (`src/types.rs:40`). |
| `title` | `ValidatedTitle` | `src/contracts/mod.rs:132` | Enforces trimmed bounds (`src/types.rs:124`). |
| `tags` | `Vec<ValidatedTag>` | `src/contracts/mod.rs:135` | Sanitized through `query_sanitization::sanitize_tag` (`src/builders.rs:267`). |
| `created_at` / `updated_at` | `DateTime<Utc>` | `src/contracts/mod.rs:136` | Derived from `TimestampPair` ensuring monotonic updates (`src/types.rs:224`). |
| `embedding` | `Option<Vec<f32>>` | `src/contracts/mod.rs:138` | Optional semantic vector attached on ingestion. |

> **Warning** The typed document state machine in `src/types.rs:399` enforces compile-time transitions (`Draft → Persisted → Modified`); skip it only if you must interop with legacy `Document` payloads.

## Step 3: Persist, fetch, and mutate data
- The fully wrapped storage implements the `Storage` contract; every method runs input validation and emits trace metrics before touching disk.
- Use the async trait methods directly, keeping mutations behind a single mutable handle to respect the buffered writer.

```rust
use kotadb::{create_file_storage, DocumentBuilder, Storage};

let mut storage = create_file_storage("/var/lib/kotadb", Some(1_000)).await?;
let doc = DocumentBuilder::new()
    .path("notes/quickref.md")?
    .title("Quick Reference")?
    .content(b"KotaDB quick reference")
    .build()?;

storage.insert(doc.clone()).await?;
if let Some(found) = storage.get(&doc.id).await? {
    println!("Found {} ({} bytes)", found.title, found.size);
}

storage.update(doc).await?;
storage.delete(&doc.id).await?;
```

| Storage API | Signature | Location | Behavior |
| --- | --- | --- | --- |
| `Storage::insert` | `async fn insert(&mut self, document: Document) -> Result<()>` | `src/contracts/mod.rs:45` | Validated writes add IDs to the in-memory index (`src/wrappers.rs:287`). |
| `Storage::get` | `async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>>` | `src/contracts/mod.rs:48` | Cached lookups hit LRU before disk (`src/wrappers.rs:742`). |
| `Storage::update` | `async fn update(&mut self, document: Document) -> Result<()>` | `src/contracts/mod.rs:51` | Checks existing metadata before applying updates (`src/wrappers.rs:313`). |
| `Storage::delete` | `async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool>` | `src/contracts/mod.rs:54` | Evicts cache entries and buffered ops on success (`src/wrappers.rs:775`). |
| `Storage::list_all` | `async fn list_all(&self) -> Result<Vec<Document>>` | `src/contracts/mod.rs:57` | Delegates to underlying engine while emitting metrics (`src/wrappers.rs:784`). |
| `Storage::sync` / `flush` / `close` | `async fn …` | `src/contracts/mod.rs:60` | Forwarded to `FileStorage` with additional trace logging (`src/wrappers.rs:210`). |

## Step 4: Shape search requests and index configuration
- Build sanitized text, tag, and wildcard queries with `QueryBuilder`; the builder automatically detects glob patterns and routes them to `Query::path_pattern`.
- Compose index and storage configuration builders when provisioning long-running services or integrating alternate backends.

```rust
use kotadb::{IndexConfigBuilder, QueryBuilder};

let query = QueryBuilder::new()
    .with_text("docs/*.md")?
    .with_tag("architecture")?
    .with_limit(500)?
    .build()?;

let index_config = IndexConfigBuilder::new()
    .name("semantic_index")
    .max_memory(256 * 1024 * 1024)
    .persistence(false)
    .build()?;
```

| Query Builder API | Signature | Location | Highlights |
| --- | --- | --- | --- |
| `QueryBuilder::with_text` | `pub fn with_text(self, text: impl Into<String>) -> Result<Self>` | `src/builders.rs:156` | Chooses path-aware or standard sanitization and preserves `*` wildcards. |
| `QueryBuilder::with_tag` | `pub fn with_tag(self, tag: impl Into<String>) -> Result<Self>` | `src/builders.rs:192` | Sanitizes via `query_sanitization::sanitize_tag` before wrapping in `ValidatedTag`. |
| `QueryBuilder::with_date_range` | `pub fn with_date_range(self, start: i64, end: i64) -> Result<Self>` | `src/builders.rs:210` | Ensures ordered timestamps using `ValidatedTimestamp`. |
| `QueryBuilder::build` | `pub fn build(self) -> Result<Query>` | `src/builders.rs:231` | Populates `Query::path_pattern` when wildcards appear and upgrades tags to validated types.

| `Query` Field | Type | Location | Details |
| --- | --- | --- | --- |
| `search_terms` | `Vec<ValidatedSearchQuery>` | `src/contracts/mod.rs:198` | Sanitized text tokens with min-length guarantees (`src/types.rs:301`). |
| `tags` | `Vec<ValidatedTag>` | `src/contracts/mod.rs:199` | Filled post-build to retain sanitized tag list (`src/builders.rs:267`). |
| `path_pattern` | `Option<String>` | `src/contracts/mod.rs:200` | Holds glob expressions like `docs/*.md`. |
| `limit` | `ValidatedLimit` | `src/contracts/mod.rs:201` | Supports high fan-out queries (≤100 000) (`src/builders.rs:222`). |
| `offset` | `ValidatedPageId` | `src/contracts/mod.rs:202` | Defaults to page 1 with non-zero enforcement (`src/types.rs:349`). |

| Config Builder | Key Methods | Location | Feature Notes |
| --- | --- | --- | --- |
| `StorageConfigBuilder` | `.path()`, `.cache_size()`, `.compression()`, `.encryption_key()` | `src/builders.rs:297` | Produces `StorageConfig` for custom engines; no feature flags required. |
| `IndexConfigBuilder` | `.name()`, `.fuzzy_search(bool)`, `.similarity_threshold(f32)` | `src/builders.rs:385` | Pair with `vector_index` or `trigram_index`; optional semantic features gated by `embeddings-onnx`. |

## Step 5: Observe and tune runtime behavior
- Pull metrics and traces from the wrapper stack—`TracedStorage` instruments every operation, while `MeteredIndex` captures per-op timing histograms.
- Use `RetryableStorage::with_retry_config` to adapt backoff windows to your deployment latency profile.
- Inspect cache efficiency through `CachedStorage::cache_stats` before increasing capacity.

| Wrapper | Key Method | Location | Runtime Insight |
| --- | --- | --- | --- |
| `TracedStorage` | `pub fn trace_id(&self) -> Uuid` / `pub async fn operation_count(&self) -> u64` | `src/wrappers.rs:41` | Emits `storage.*` metrics and tags with a stable trace ID per handle. |
| `ValidatedStorage` | `pub fn new(inner: S) -> Self` | `src/wrappers.rs:266` | Ensures documents pass `validation::document` checks before persistence. |
| `RetryableStorage::with_retry_config` | `pub fn with_retry_config(self, max_retries, base_delay, max_delay) -> Self` | `src/wrappers.rs:372` | Configures exponential backoff with jitter for flaky IO. |
| `CachedStorage::cache_stats` | `pub async fn cache_stats(&self) -> (u64, u64)` | `src/wrappers.rs:715` | Reports hit/miss counters to guide sizing. |
| `MeteredIndex::timing_stats` | `pub async fn timing_stats(&self) -> HashMap<String, (Duration, Duration, Duration)>` | `src/wrappers.rs:850` | Aggregates min/avg/max latency per index operation. |

> **Note** Enable feature-gated modules (`--features "tree-sitter-parsing"` or `--features "mcp-server"`) only when you depend on symbol pipelines or MCP adapters; the quick reference APIs above operate on the default feature set.

## Next Steps
- Run `just test` to exercise the full storage stack under `cargo nextest`.
- Explore `docs/api/api_reference.md` for HTTP surface details that build on these primitives.
- Inspect `src/services/` implementations to see how service layers call into the same builders and wrappers.
- Capture runtime traces with `just dev` and verify `storage.*` metrics appear in your connected `tracing` subscriber.
