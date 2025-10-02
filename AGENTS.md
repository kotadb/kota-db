# Repository Guidelines

## Project Structure & Module Organization
- Source code in `src/` (library in `src/lib.rs`; binaries in `src/bin/*.rs`).
- Integration tests in `tests/`; unit tests alongside modules; benches in `benches/`.
- Helper scripts in `scripts/`; docs in `docs/`; example data in `examples-data/` and `test-data/`.
- Config and dev presets: `kotadb-dev.toml`, `.env.example`, `docker-compose.*.yml`.

## Build, Test, and Development Commands
- `just dev` — run MCP server with auto-reload for local dev.
- `just test` — run all tests fast via `cargo nextest`.
- `just test-fast` — mirrors CI gating: nextest lib + doctests with `--no-default-features --features "git-integration,tree-sitter-parsing,mcp-server"`.
- `just fmt` / `just clippy` — format and lint; CI fails on warnings.
- `just coverage` — HTML coverage at `target/llvm-cov/html/index.html`.
- `just ci-fast` — format, clippy, unit tests, audit.
- Examples: `cargo run --example 01_personal_knowledge_base`.
- Binaries: `cargo run --bin kotadb`, `cargo run --bin kotadb-api-server`, `cargo run --bin mcp_server`.

## Coding Style & Naming Conventions
- Rust 2021; use `rustfmt` (`just fmt`) and `clippy` with `-D warnings`.
- Naming: `snake_case` for modules/files, `CamelCase` types/traits, `SCREAMING_SNAKE_CASE` constants.
- Errors: prefer `thiserror` for typed errors and `anyhow` in binaries; avoid `unwrap()` in hot paths.
- Logging via `tracing`; gate heavy features behind Cargo features (see `Cargo.toml`).

## Testing Guidelines
- Runner: `cargo nextest`; property tests via `proptest`; perf via `criterion` (use `--features bench`).
- Layout: integration tests in `tests/*.rs`; unit tests in-module under `mod tests { ... }`.
- Naming: end files with `_test.rs` or descriptive `*_tests.rs`.
- Run: `just test`, focus: `cargo nextest run --test name_pattern`.

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat:`, `fix(scope):`, `docs:`, `chore:`, etc. Link issues (e.g., `#642`).
- PRs must include: clear description, rationale, test coverage for new logic, and updates to docs/config when applicable.
- Ensure `just ci-fast` passes locally; attach logs or screenshots for API/CLI changes when helpful.

## Security & Configuration Tips
- Never commit secrets; use `.env.dev`/`.env.example` as references.
- Local data directory via `KOTADB_DATA_DIR`; sample configs: `kotadb-dev.toml`.
- Optional features: `advanced-search`, `tree-sitter-parsing` — enable explicitly when needed.
