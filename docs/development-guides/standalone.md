# Running KotaDB Standalone

KotaDB ships a `kotadb` CLI that bundles storage, indexing, search, and analysis into a single binary. This guide shows how to compile the tool, prepare its on-disk data stores, and exercise the same services the wider platform uses.

## Step 1 — Install prerequisites

- Install the stable Rust toolchain listed in `rust-toolchain.toml:1-5`; `rustup toolchain install stable` keeps you aligned with CI.
- Ensure the workspace utilities are available. The project relies on `cargo`, `just`, and `cargo-nextest` for the commands referenced throughout.

```bash
rustup show
just --list
```

## Step 2 — Build the CLI

The binary target is declared in `Cargo.toml:180-199`, and the Clap-powered interface is defined in `src/main.rs:31`. Build or run it directly to confirm the toolchain works:

```bash
cargo run --bin kotadb -- --help
```

The top-level `Cli` struct (`src/main.rs:31-124`) wires every subcommand, sets the default database directory (`--db-path`), and exposes verbosity controls that map to tracing setup (`src/main.rs:1568-1598`).

> **Note** Default features enable git ingestion, tree-sitter parsing, embeddings, and the MCP server (`Cargo.toml:154-171`). Only disable them if you are prepared to lose the corresponding commands documented below.

## Step 3 — Initialize the standalone data directory

When a command runs, `Database::new` provisions the required directories and in-memory handles (`src/main.rs:281-344`). It creates `storage/`, `primary_index/`, and `trigram_index/` folders beneath `--db-path` and opens the appropriate storage engines via `create_file_storage`, `create_primary_index`, and `create_binary_trigram_index`.

```bash
cargo run --bin kotadb -- stats --basic --db-path ./kota-db-data
```

Running `stats` (or any command) with a fresh path will initialize the structure if it does not already exist. Use `--binary-index=false` to fall back to the JSON trigram indexer if needed (`src/main.rs:82-84`), though the binary reader is the default for performance.

## Step 4 — Index a repository

The `index-codebase` subcommand (`src/main.rs:145-173`) is the entry point for ingestion. It constructs an `IndexingService` (`src/services/indexing_service.rs:149`) that forwards to `RepositoryIngester` with the options you provide. Symbol extraction and relationship graphs are produced when the `tree-sitter-parsing` feature is active; the service writes `symbols.kota` and `dependency_graph.bin` alongside document storage (`src/services/indexing_service.rs:214-309`).

```bash
cargo run --bin kotadb -- \
  index-codebase /path/to/repo \
  --prefix repos \
  --max-file-size-mb 5 \
  --db-path ./kota-db-data
```

After ingestion, the CLI flushes buffered storage and rebuilds indices so wildcard and trigram searches stay in sync (`src/main.rs:1838-1861`). The same batching logic lives in `Database::rebuild_indices` for reuse (`src/main.rs:365-416`).

> **Warning** Incremental updates and git-specific analytics are still stubs (`src/services/indexing_service.rs:581-642`); re-run a full `index-codebase` when repositories change.

## Step 5 — Query and inspect the database

`kotadb search-code` dispatches to `SearchService::search_content` (`src/services/search_service.rs:118-172`), which selects between the trigram and primary indices depending on whether the query contains wildcards (`src/services/search_service.rs:329-345`). Context levels control whether the LLM-optimized path is invoked, with budgets defined in `try_llm_search` (`src/services/search_service.rs:255-295`).

```bash
cargo run --bin kotadb -- search-code "storage::create" --context medium --limit 5 --db-path ./kota-db-data
```

Symbol lookups reuse the same service via `search_symbols` (`src/services/search_service.rs:174-252`), drawing from the binary symbol store created during indexing.

| Command | Service path | Implementation |
| --- | --- | --- |
| `kotadb search-code <pattern>` | `SearchService::search_content` → `regular_search` | `src/services/search_service.rs:118`, `src/services/search_service.rs:298` |
| `kotadb search-symbols <name>` | `SearchService::search_symbols` | `src/services/search_service.rs:174` |
| `kotadb stats [--basic|--symbols|--relationships]` | `StatsService::get_statistics` | `src/services/stats_service.rs:245` |

> **Note** Symbol-based commands require the default `tree-sitter-parsing` feature and an indexed repository; the CLI warns if `symbols.kota` is missing (`src/main.rs:1867-1893`).

## Step 6 — Run advanced analysis and services

The remaining subcommands reuse the service layer shared with the MCP server:

| Command | Service path | Implementation |
| --- | --- | --- |
| `kotadb find-callers <symbol>` | `AnalysisService::find_callers` | `src/services/analysis_service.rs:252` |
| `kotadb analyze-impact <symbol>` | `AnalysisService::analyze_impact` | `src/services/analysis_service.rs:284` |
| `kotadb codebase-overview` | `AnalysisService::generate_overview` | `src/services/analysis_service.rs:318` |
| `kotadb benchmark --operations 10000` | `BenchmarkService::run_benchmark` | `src/services/benchmark_service.rs:300` |
| `kotadb validate` | `ValidationService::validate_database` | `src/services/validation_service.rs:356` |
| `kotadb verify-docs` | `DocumentationVerifier::run_full_verification` | `src/main.rs:1708-1776` |
| `kotadb serve --port 8080` | `services_http_server::start_services_server` | `src/main.rs:1606-1631` |

Start the HTTP layer when you need REST access to the same services:

```bash
cargo run --bin kotadb -- serve --port 8080 --db-path ./kota-db-data
```

This call mounts the service endpoints listed inside `main` (`src/main.rs:1609-1622`) using the storage and indices already opened for the CLI session.

> **Note** All analysis commands expect that `index-codebase` has populated both documents and symbol data. If the binary symbol store is absent, the services emit actionable guidance before exiting (`src/services/analysis_service.rs:240-245`).

## Next Steps

- Run `just test` to exercise the same `cargo nextest` suite that backs CI.
- Use `just dev` for an auto-reloading MCP server while iterating on standalone workflows.
- Review the companion [CLI usage guide](./cli_usage.md) for detailed flag semantics and scripting patterns.
