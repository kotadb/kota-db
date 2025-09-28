# FileStorage Implementation

## Summary
KotaDB persists markdown documents by layering an in-memory metadata index over a file-system layout and wrapping the storage core with tracing, validation, retry, caching, and buffering wrappers. The `FileStorage` backend (`src/file_storage.rs:24`) is responsible for preparing the on-disk structure, maintaining a hot cache of document metadata, and serving CRUD operations that the higher-level services—HTTP APIs, hybrid storage, and semantic search—depend on.

## Step 1 – Prepare Directories and Write-Ahead Log
- `FileStorage::open` validates the target directory via `validation::path::validate_storage_directory_path` (`src/file_storage.rs:189`, `src/validation.rs:201`) before constructing the storage instance.
- Directory scaffolding is created by `ensure_directories` (`src/file_storage.rs:47`), which materializes `documents/`, `indices/`, `wal/`, and `meta/` subdirectories so later components can assume their presence.
- Crash recovery is primed by `init_wal` (`src/file_storage.rs:66`), which opens `wal/current.wal` in append mode and stores the handle in the `wal_writer` mutex for later `sync` calls.
- Existing metadata snapshots are loaded during `load_existing_documents` (`src/file_storage.rs:80`); it walks `documents/*.json`, deserializes `DocumentMetadata`, and seeds the `RwLock<HashMap<Uuid, DocumentMetadata>>` index so the process restarts hot.

## Step 2 – Maintain the Metadata Cache
- The in-memory index (`documents` field) and the WAL handle are the only mutable members of `FileStorage` (`src/file_storage.rs:24-30`), making concurrent reads cheap while write paths take exclusive locks only when necessary.
- `DocumentMetadata` encodes the essential bookkeeping that is cached in memory and mirrored on disk (`src/file_storage.rs:35-44`):
  | Field | Description | Location |
  | --- | --- | --- |
  | `id` | UUID backing the validated document id | `src/file_storage.rs:36` |
  | `file_path` | Absolute content path under `documents/` | `src/file_storage.rs:37` |
  | `original_path` / `title` | User-facing identifiers reconstructed into `Document` | `src/file_storage.rs:38-39` |
  | `size`, `created`, `updated`, `hash` | Integrity and ordering hints used for change detection | `src/file_storage.rs:40-43` |
  | `embedding` | Optional semantic vector carried into search | `src/file_storage.rs:44` |
- `metadata_to_document` (`src/file_storage.rs:147-183`) materializes cached entries into the public `Document` struct (`src/contracts/mod.rs:130-190`) by parsing YAML frontmatter via pure helpers (`src/pure/metadata.rs:8-30`) and rebuilding validated path, title, and tag types.

## Step 3 – Persist and Mutate Documents
- Writes flow through `insert`, `update`, and `delete`, each coordinating disk persistence with cache updates. The core routines are summarized below:
  | Routine | Signature | Location | Notes |
  | --- | --- | --- | --- |
  | `FileStorage::insert` | `async fn insert(&mut self, doc: Document)` | `src/file_storage.rs:215-300` | Guards against duplicate ids, regenerates YAML frontmatter when tags are present, writes content and JSON metadata, and updates the in-memory map. |
  | `FileStorage::get` | `async fn get(&self, id: &ValidatedDocumentId)` | `src/file_storage.rs:302-313` | Performs lock-free reads using cloned metadata before reconstructing the document bytes. |
  | `FileStorage::update` | `async fn update(&mut self, doc: Document)` | `src/file_storage.rs:316-353` | Rewrites the markdown file verbatim, recalculates hashes (`src/pure/metadata.rs:45-49`), and refreshes timestamps plus embeddings in metadata. |
  | `FileStorage::delete` | `async fn delete(&mut self, id: &ValidatedDocumentId)` | `src/file_storage.rs:356-385` | Removes the metadata entry, then best-effort deletes `.md` and `.json` files while tolerating missing paths. |
  | `FileStorage::list_all` | `async fn list_all(&self)` | `src/file_storage.rs:402-418` | Clones the metadata map to avoid holding locks during asynchronous disk reads. |
  | `FileStorage::flush` / `sync` / `close` | `src/file_storage.rs:421-449` | Flush cascades to `sync`, which calls `tokio::fs::File::sync_all` on the WAL; `close` drops the WAL handle after flushing buffered state. |

> **Note** Frontmatter re-write logic in `insert` (`src/file_storage.rs:228-268`) replaces malformed or partial headers with a normalized YAML block so downstream parsers always see structured `tags` arrays.

## Step 4 – Validate Inputs and Derived Data
- Path validation rejects traversal, URL schemes, and repeated separators before any disk access (`src/validation.rs:201-259`).
- Tags emitted from frontmatter are individually re-validated as `ValidatedTag` instances, preventing arbitrary YAML content from reaching higher layers (`src/file_storage.rs:165-167`).
- Document ids, titles, and paths are reconstructed using `ValidatedDocumentId`, `ValidatedTitle`, and `ValidatedPath` to enforce type-level invariants on every read path (`src/file_storage.rs:172-175`).
- Content integrity is tracked with SHA-256 hashes from `calculate_hash` (`src/pure/metadata.rs:45-49`), which allows future WAL or replication consumers to detect divergence without re-reading the markdown body.

## Step 5 – Wrap the Core for Production Use
- `create_file_storage` exposes the recommended constructor (`src/file_storage.rs:514-524`); after calling `FileStorage::open`, it passes the instance to the wrapper factory.
- Wrapper composition lives in `create_wrapped_storage` (`src/wrappers.rs:1005-1021`), which layers components in the following order:
  | Layer | Type | Purpose | Feature Flags |
  | --- | --- | --- | --- |
  | 1 | `BufferedStorage<S>` | Coalesces writes asynchronously to reduce fs churn | Always on |
  | 2 | `CachedStorage<…>` | LRU cache keyed by document id, sized by `cache_capacity` | Always on |
  | 3 | `RetryableStorage<…>` | Retries transient failures with exponential backoff | Always on |
  | 4 | `ValidatedStorage<…>` | Reapplies contract checks before delegating | Always on |
  | 5 | `TracedStorage<…>` | Emits structured tracing spans and metrics | Always on |
- The composite alias `FullyWrappedStorage` (`src/wrappers.rs:1005-1008`) ensures callers receive a single type that honours the Stage 6 risk controls without manual wiring.

## Step 6 – Integrate with Higher-Level Services
- The hybrid router instantiates `FileStorage` inside an `Arc<RwLock<…>>` so document operations can be routed independently from graph operations (`src/hybrid_storage.rs:94-122`). Routing decisions inspect sanitized paths before selecting document, graph, or dual writes (`src/hybrid_storage.rs:124-143`).
- Semantic search builds on top of `FileStorage::open` when constructing the `SemanticSearchEngine`, ensuring embeddings and vector indices stay consistent with the document store (`src/semantic_search.rs:482-495`).
- Buffered writes are tested directly to guarantee the wrapper pipeline behaves with `FileStorage` as the inner store (`src/wrappers/buffered_storage.rs:493-517`).
- HTTP-facing binaries use the same wrapper factory to expose CLI and API commands, for example `create_wrapped_storage` calls inside the server bootstrapper (`src/http_server.rs:1855-1896`).

## Step 7 – Verify with Targeted Tests
- Unit tests in `src/file_storage.rs` (e.g., initialization at `src/file_storage.rs:574-587`, persistence at `src/file_storage.rs:589-640`, and frontmatter handling at `src/file_storage.rs:701-741`) cover directory creation, CRUD flows, tag serialization, and metadata reloads across restarts.
- Integration scenarios that rely on symbol analysis and relation extraction exercise `FileStorage` through `HybridStorage` fixtures (`tests/api_relationship_endpoints_test.rs:427-472`), ensuring callers like `find-callers` operate on the same paths documented here.
- Run `just test` for the full `cargo nextest` suite or `just test-fast` to mirror CI gating; both commands execute the FileStorage tests alongside the higher-level wrappers.

## Next Steps
- Audit WAL replay requirements so `sync` can flush buffered operations in coordination with `BufferedStorage`.
- Extend the metadata schema with compression or encryption once requirements are finalized, updating `DocumentMetadata` serialization in sync.
- Capture traces from a representative workload (e.g., via `just dev`) to tune cache sizes and retry policies before deploying to new environments.
