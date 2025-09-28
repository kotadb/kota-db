---
title: "KOTA Database Data Model Specification"
tags: [database, data-model, specification]
related: ["IMPLEMENTATION_PLAN.md", "TECHNICAL_ARCHITECTURE.md"]
key_concepts: [document-structure, indexing, compression, relationships]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KOTA Database Data Model Specification

KotaDB persists repository artifacts as validated documents backed by filesystem storage, B+-tree indices, and optional graph/vector stores orchestrated by the `Database` facade. This specification maps each runtime structure to its implementation so storage, indexing, and query flows remain auditable.

## Data Flow Steps
1. Repository ingestion converts Git metadata and files into validated `Document` records via `DocumentBuilder`, tagging and summarizing each artifact before persistence (`src/git/ingestion.rs:1339`, `src/builders.rs:11`).
2. The `Database` facade resolves storage and index components, wrapping the file-backed store with tracing/validation wrappers before handing it to higher-level services (`src/database.rs:38`, `src/database.rs:87`, `src/wrappers.rs:12`).
3. Document writes land in `FileStorage::insert`, which materializes Markdown content with YAML frontmatter, JSON metadata, and a crash-safe WAL entry on disk (`src/file_storage.rs:215`).
4. Primary and secondary indices are updated in tandem—`PrimaryIndex::insert` stores path lookups while trigram/vector/graph components persist search data (`src/primary_index.rs:615`, `src/trigram_index.rs:956`, `src/vector_index.rs:117`, `src/native_graph_storage.rs:142`).
5. Runtime services rely on the `DatabaseAccess` trait to coordinate CRUD, indexing, and search operations across CLI, MCP, and HTTP entry points (`src/services/search_service.rs:19`, `src/services/management_service.rs:195`).

## Storage Pipeline Steps
1. `FileStorage::open` validates the target directory, creates the `documents/`, `indices/`, `wal/`, and `meta/` folders, and opens the append-only WAL handle (`src/file_storage.rs:193`, `src/file_storage.rs:47`, `src/file_storage.rs:66`).
2. When inserting, Markdown content is augmented with YAML frontmatter so tags round-trip cleanly, and the payload is written to `<uuid>.md` in the document directory (`src/file_storage.rs:225`).
3. Companion JSON metadata captures canonical path, title, timestamps, size, hashes, and embeddings for each document, persisted via `save_metadata` (`src/file_storage.rs:134`, `src/file_storage.rs:278`).
4. An in-memory index of `DocumentMetadata` is kept in a read/write lock for fast lookups, and deletions evict both on-disk and in-memory state (`src/file_storage.rs:28`, `src/file_storage.rs:356`).
5. Reads reconstruct `Document` instances by combining metadata with on-disk content and extracting tags from frontmatter (`src/file_storage.rs:148`, `src/file_storage.rs:160`).

## Index Maintenance Steps
1. The primary index keeps a B+-tree rooted in memory, logging every mutation to `PrimaryIndex`’s WAL before updating the tree and metadata counters (`src/primary_index.rs:75`, `src/primary_index.rs:615`).
2. Wildcard searches reuse the same tree, filtering results through `matches_wildcard_pattern` after lazy-loading the persisted tree state (`src/primary_index.rs:707`, `src/primary_index.rs:94`).
3. The text index extracts trigrams from combined title and content, stores unique trigram postings, and caches frequency maps for scoring (`src/trigram_index.rs:118`, `src/trigram_index.rs:956`).
4. The binary trigram variant optionally memory-maps compact postings and sparse frequency vectors for high-throughput search (`src/binary_trigram_index.rs:22`, `src/binary_trigram_index.rs:45`).
5. The vector index stores embeddings in an HNSW-inspired graph with configurable distance metrics, persisting nodes whenever new vectors are inserted (`src/vector_index.rs:16`, `src/vector_index.rs:117`).
6. Hybrid storage routes graph-aware documents to `NativeGraphStorage`, which maintains page-aligned node and edge stores with their own WAL for recovery (`src/hybrid_storage.rs:94`, `src/native_graph_storage.rs:142`).

## Query Execution Steps
1. `SearchService::regular_search` builds sanitized queries with `QueryBuilder`, optionally appending tag filters and limits (`src/services/search_service.rs:299`, `src/builders.rs:137`).
2. Wildcard queries are routed to the primary index while textual searches hit the trigram index; both paths return canonical document IDs (`src/services/search_service.rs:328`).
3. Result IDs are hydrated through the wrapped storage layer so callers receive full `Document` values, including reconstructed tags and embeddings (`src/services/search_service.rs:350`).
4. When LLM-optimized search is requested, the service coordinates the trigram index with the language model engine to synthesize responses without bypassing core storage semantics (`src/services/search_service.rs:280`).

## Storage Structures
`FileStorage::init_wal` opens `wal/current.wal` so inserts, updates, and deletes can be replayed after crashes (`src/file_storage.rs:66`). The combination of Markdown payloads and JSON metadata keeps human-readable content and machine metadata in sync.

### Document Fields
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `id` | `ValidatedDocumentId` | Primary key shared across storage and indices. | `src/contracts/mod.rs:131` |
| `path` | `ValidatedPath` | Normalized logical path used for indexing and display. | `src/contracts/mod.rs:132` |
| `title` | `ValidatedTitle` | Human-readable label surfaced in search results. | `src/contracts/mod.rs:133` |
| `content` | `Vec<u8>` | Markdown or binary payload written to disk. | `src/contracts/mod.rs:134` |
| `tags` | `Vec<ValidatedTag>` | Semantic tags serialized in frontmatter. | `src/contracts/mod.rs:135` |
| `created_at / updated_at` | `DateTime<Utc>` | MVCC-style timestamps persisted in metadata. | `src/contracts/mod.rs:136` |
| `embedding` | `Option<Vec<f32>>` | Optional semantic vector used by the vector index. | `src/contracts/mod.rs:140` |

### File Metadata Fields
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `id` | `Uuid` | Mirror of the validated document identifier. | `src/file_storage.rs:35` |
| `file_path` | `PathBuf` | Absolute path to the Markdown payload. | `src/file_storage.rs:37` |
| `original_path` | `String` | Logical repository path retained for queries. | `src/file_storage.rs:38` |
| `title` | `String` | Stored title rebuilt into `ValidatedTitle` on read. | `src/file_storage.rs:39` |
| `hash` | `[u8; 32]` | Content checksum for change detection. | `src/file_storage.rs:43` |
| `embedding` | `Option<Vec<f32>>` | Cached semantic vector for fast reload. | `src/file_storage.rs:44` |

## Index Structures
The indexing layer combines logical metadata, text search signals, and embeddings so queries can mix wildcard, lexical, and semantic retrieval.

### Primary Index Metadata
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `version` | `u32` | On-disk schema version of the index. | `src/primary_index.rs:42` |
| `document_count` | `usize` | Total entries tracked by the B+-tree. | `src/primary_index.rs:43` |
| `created` | `i64` | Epoch seconds when the index was first created. | `src/primary_index.rs:44` |
| `updated` | `i64` | Last successful mutation timestamp. | `src/primary_index.rs:45` |

### Trigram Document Cache
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `title` | `String` | Derived title for snippet headers. | `src/trigram_index.rs:40` |
| `content_preview` | `String` | Truncated body cached for snippets. | `src/trigram_index.rs:41` |
| `full_trigrams` | `Vec<String>` | Raw trigram list for scoring. | `src/trigram_index.rs:42` |
| `trigram_freq` | `HashMap<String, usize>` | Pre-computed frequency map for relevance. | `src/trigram_index.rs:49` |
| `word_count` | `usize` | Document length hint for ranking. | `src/trigram_index.rs:43` |

### Binary Trigram Compact Meta
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `title_hash` | `u64` | Compact identifier for deduplicating titles. | `src/binary_trigram_index.rs:47` |
| `trigram_freqs` | `Vec<(u16, u8)>` | Sparse trigram frequency table. | `src/binary_trigram_index.rs:49` |
| `packed_stats` | `u32` | Encodes word count and unique trigram totals. | `src/binary_trigram_index.rs:51` |

### Vector Index Node
| Field | Type | Description | Source |
| --- | --- | --- | --- |
| `id` | `ValidatedDocumentId` | Document associated with the vector. | `src/vector_index.rs:27` |
| `vector` | `Vec<f32>` | Embedding coordinates stored on disk. | `src/vector_index.rs:28` |
| `levels` | `Vec<HashSet<ValidatedDocumentId>>` | HNSW adjacency lists per layer. | `src/vector_index.rs:29` |

> **Note** Vector search currently performs a linear scan after distance calculations, so enabling the HNSW traversal optimizations in `search_knn` remains an open performance improvement (`src/vector_index.rs:171`).

### Graph Entities
| Entity | Key Fields | Description | Source |
| --- | --- | --- | --- |
| `GraphNode` | `id`, `node_type`, `qualified_name`, `metadata` | Symbol or asset nodes tracked for dependency analysis. | `src/graph_storage.rs:83` |
| `GraphEdge` | `relation_type`, `location`, `metadata` | Directional relationships such as references or similarities. | `src/graph_storage.rs:110` |
| `NodeLocation` | `start_line`, `end_line` | Source offsets for both nodes and edges. | `src/graph_storage.rs:101` |
| `GraphStats` | degree metrics, counts | Aggregated sizing and topology statistics. | `src/graph_storage.rs:160` |

## Configuration and Feature Flags
- `Database::new` selects between text and binary trigram indices through the `use_binary_index` flag passed by clients (`src/database.rs:65`).
- `HybridStorageConfig::default` enables graph storage and routes `/symbols/*`, `/relationships/*`, and `/dependencies/*` into the graph backend (`src/hybrid_storage.rs:33`, `src/hybrid_storage.rs:46`).
- Feature gates such as `tree-sitter-parsing` install symbol extraction, binary relationship bridges, and related graph tooling when enabled (`src/lib.rs:52`).
- Indexing options expose flags for symbol extraction, chunking, and repository filters so ingestion can be tuned per deployment (`src/services/indexing_service.rs:15`).

## Next Steps
- `just test` — run the nextest-powered suite to verify storage, indexing, and service contracts.
- `just clippy` — enforce the `-D warnings` policy before landing documentation-driven refactors.
- `cargo nextest run --test search_service` — exercise the query pipeline end-to-end after modifying search structures.
