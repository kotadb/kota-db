# KotaDB CLI Usage Guide

KotaDB's `kotadb` binary composes the shared service layer in `src/services` to index repositories, answer code intelligence queries, validate data, and expose an HTTP surface. This guide shows how to build the binary, prepare storage, run every subcommand, and interpret the runtime responsibilities wired into the code.

## Step 1 – Build the `kotadb` Binary
- Build with default features (which include `git-integration`, `tree-sitter-parsing`, and `mcp-server`) so every subcommand is compiled:
  ```bash
  cargo build --bin kotadb
  ```
- Run the binary directly for iterative work:
  ```bash
  cargo run --bin kotadb -- --help
  ```
- Global flags such as `--verbosity` and `--binary-index` live on the `Cli` struct in `src/main.rs:58` and are consumed inside `main` at `src/main.rs:1568-1603` to configure logging through `init_logging_with_level`.
- `just test` keeps parity with CI via `cargo nextest` before you rely on a new build.

## Step 2 – Configure the Database Directory
| Flag | Description | Implementation |
| --- | --- | --- |
| `--db-path <dir>` | Sets the root directory for storage, indices, symbols, and graphs. Defaults to `./kota-db-data`. | `src/main.rs:78` and `Database::new` in `src/main.rs:281` create `storage/`, `primary_index/`, and `trigram_index/` folders on demand. |
| `--binary-index=<bool>` | Chooses between the binary trigram index (`true`, default) and the legacy text index (`false`). | `src/main.rs:82` branches to `create_binary_trigram_index` vs `create_trigram_index` inside `Database::new` at `src/main.rs:306`. |
| `--verbosity <quiet|normal|verbose|debug>` | Controls CLI logging levels and toggles `RUST_LOG` in debug mode. | `src/main.rs:59` with handling in `src/main.rs:1572-1598`. |

- `Database::new` wires together file storage, primary path index, and trigram index in one place (`src/main.rs:281-344`). Every subcommand reuses this handle, so the directory must be writable before you proceed.
- `Database::rebuild_indices` is the canonical way to repopulate both indices after writes (`src/main.rs:365`). The CLI calls it automatically after ingestion but you can invoke it in custom workflows via the service layer.

## Step 3 – Ingest a Repository with `index-codebase`
- The `index-codebase` subcommand (protected by the `git-integration` feature) is defined at `src/main.rs:1785-1863` and fans into `IndexingService::index_codebase` (`src/services/indexing_service.rs:149`).
- Key options:
  | Flag | Purpose | Implementation |
  | --- | --- | --- |
  | `<repo_path>` | Filesystem path to a Git repository. | Required positional arg at `src/main.rs:1787` validated in `IndexingService` at `src/services/indexing_service.rs:155`. |
  | `--prefix <name>` | Prefix added to stored document paths. | `src/main.rs:1803` forwarded to `IngestionConfig` at `src/services/indexing_service.rs:239`. |
  | `--include-files/--include-commits` | Toggle blob content and commit history ingestion. | `src/main.rs:1806-1808` → ingestion options at `src/services/indexing_service.rs:223-236`. |
  | `--max-file-size-mb`, `--max-memory-mb`, `--max-parallel-files`, `--enable-chunking` | Resource guards for large repos. | `src/main.rs:1809-1811` and memory limits wiring at `src/services/indexing_service.rs:203-218`. |
  | `--extract-symbols/--no-symbols` | Control tree-sitter extraction (requires `tree-sitter-parsing`). | `src/main.rs:1795-1798` with branching at `src/services/indexing_service.rs:174-199`.
- When symbol extraction is active, `IndexingService` emits `symbols.kota` and `dependency_graph.bin` alongside document storage via `ingest_with_binary_symbols_and_relationships` (`src/services/indexing_service.rs:264-276`).
- After ingestion, the CLI flushes storage and rebuilds indices to ensure immediate searchability (`src/main.rs:1840-1859`).
  ```bash
  cargo run --bin kotadb -- index-codebase ~/code/my-service --prefix repos/my-service
  ```

> **Note** The `tree-sitter-parsing` feature is enabled by default; disable it with `cargo run --no-default-features --features git-integration --bin kotadb -- index-codebase ...` if your toolchain lacks tree-sitter grammars.

## Step 4 – Search Documents and Symbols
### `search-code`
- Declared in `src/main.rs:1635-1663` and implemented through `SearchService::search_content` (`src/services/search_service.rs:118`).
- Pipeline:
  1. The CLI normalises tags and verbosity flags.
  2. `SearchService` evaluates whether to use the LLM-optimised engine when `--context medium|full` (`src/services/search_service.rs:129-151`) and otherwise falls back to the index-backed search path (`src/services/search_service.rs:157-171`).
  3. `regular_search` routes wildcard queries to the primary index and other queries to the trigram index (`src/services/search_service.rs:298-337`), fetching documents via `Storage::get`.
- Options:
  | Flag | Purpose | Implementation |
  | --- | --- | --- |
  | `<query>` | Defaults to `*`. Empty strings short-circuit with a warning. | `src/main.rs:1635-1641` and guard in `search_content` at `src/services/search_service.rs:118-127`. |
  | `-l/--limit <n>` | Limits hits (default 10). | `src/main.rs:105-111` → `SearchOptions::limit` at `src/services/search_service.rs:135-170`. |
  | `-t/--tags <csv>` | Adds tag filters to the query builder. | `src/main.rs:112-114` and `QueryBuilder::with_tag` via `regular_search` at `src/services/search_service.rs:318-330`. |
  | `-c/--context <level>` | Switches between minimal output and LLM-enhanced answers. | `src/main.rs:115-123` and context mapping at `src/services/search_service.rs:255-279`.
  ```bash
  cargo run --bin kotadb -- search-code "search_service.rs" -l 5 -c medium
  ```

### `search-symbols`
- Available when `tree-sitter-parsing` is compiled; the CLI entry is `src/main.rs:1866-1903` and it calls `SearchService::search_symbols` (`src/services/search_service.rs:174`).
- The service reads `symbols.kota` via `BinarySymbolReader::open` (`src/services/search_service.rs:187`) and applies wildcard-aware matching plus optional type filters (`src/services/search_service.rs:206-220`).
  ```bash
  cargo run --bin kotadb -- search-symbols Storage -l 20 --symbol-type struct
  ```

## Step 5 – Explore Relationships and Overviews
- **`find-callers`** (requires `tree-sitter-parsing`): The CLI wiring at `src/main.rs:1906-1927` creates a fresh database handle (binary indices enforced) and delegates to `AnalysisService::find_callers` (`src/services/analysis_service.rs:252`). Internally, the service initialises a `BinaryRelationshipEngine` (`src/services/analysis_service.rs:222`) backed by `dependency_graph.bin`, converts raw relationships into call sites, and returns Markdown listing callers, files, and line numbers.
- **`analyze-impact`**: Similar to callers, but maps to `AnalysisService::analyze_impact` (`src/services/analysis_service.rs:284`) to enumerate symbols affected by changes along with contextual impact types.
- **`codebase-overview`**: `src/main.rs:1982-1999` triggers `AnalysisService::generate_overview` (`src/services/analysis_service.rs:317`) which aggregates storage stats, symbol distributions, entry points, and language breakdowns into human or JSON formats.
  ```bash
  cargo run --bin kotadb -- find-callers storage::FileStorage
  cargo run --bin kotadb -- analyze-impact storage::StorageService --limit 25
  cargo run --bin kotadb -- codebase-overview --format json
  ```

> **Warning** Relationship-driven commands rely on having both `symbols.kota` and `dependency_graph.bin` present. If indexing was run without symbol extraction, `create_relationship_engine` (`src/services/analysis_service.rs:233`) will error with remediation steps.

## Step 6 – Monitor Data Quality and Performance
- **`stats`**: `src/main.rs:1667-1684` calls `StatsService::get_statistics` (`src/services/stats_service.rs:245`). Flag combinations decide whether basic document counts, symbol metrics, and relationship analytics are returned; outputs stay informative even in quiet mode.
- **`validate`**: `src/main.rs:1687-1705` maps to `ValidationService::validate_database` (`src/services/validation_service.rs:356`). The service runs functional search checks, optional integrity scans of storage and indices, and summarises pass/fail status while still emitting critical issues when `--verbosity quiet`.
- **`benchmark`**: `src/main.rs:1955-1979` instantiates `BenchmarkService::run_benchmark` (`src/services/benchmark_service.rs:300`). It optionally warms up, then times storage, index, query, and search loops, reporting aggregated throughput in human, JSON, or CSV formats.
- **`verify-docs`**: `src/main.rs:1708-1782` invokes `DocumentationVerifier::run_full_verification` (`src/documentation_verification.rs:644`) to cross-check documented endpoints, clients, and features against the actual code, failing the command if critical mismatches remain.
  ```bash
  cargo run --bin kotadb -- stats --symbols --relationships
  cargo run --bin kotadb -- validate
  cargo run --bin kotadb -- benchmark -t search -f json --max-search-queries 50
  cargo run --bin kotadb -- verify-docs
  ```

## Step 7 – Serve the HTTP API
- The `serve` subcommand at `src/main.rs:1605-1632` promotes the local database into an HTTP service by calling `start_services_server` (`src/services_http_server.rs:628`).
- `start_services_server` binds an Axum router with endpoints such as `/api/v1/search/code`, `/api/v1/analysis/stats`, and `/api/v1/index-codebase` (`src/services_http_server.rs:664-681`). It reuses the same storage, index, and trigram handles constructed in `Database::new`, so CLI and HTTP queries stay consistent.
  ```bash
  cargo run --bin kotadb -- serve --port 8484 --db-path ./kota-db-data
  ```

> **Note** If the port is in use, the helper hints emitted from `start_services_server` at `src/services_http_server.rs:637-655` explain how to inspect and free it on Unix or Windows.

## Next Steps
- Re-run `just test` or `just ci-fast` after extending CLI logic to keep service behaviour in sync.
- Capture benchmark snapshots (`kotadb benchmark`) before large refactors to spot performance regressions.
- Export HTTP traces while `kotadb serve` is running to document any new endpoints in `docs/api/`.
