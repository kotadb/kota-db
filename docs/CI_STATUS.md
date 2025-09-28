# CI/CD Pipeline Status
KotaDB's gatekeeping pipeline ties workflow triggers, staged quality checks, and selective feature flags to the runtime behaviour validated in tests and services. This guide maps each job to the code it executes so you can reproduce CI locally and understand what a passing merge requires.

## Step 1: Track Workflow Entry Points
The main `CI` workflow runs on pushes and pull requests targeting `main` or `develop`, plus a nightly schedule for deeper coverage `.github/workflows/ci.yml:3` `.github/workflows/ci.yml:4` `.github/workflows/ci.yml:9`. Concurrency keeps only the latest run alive per ref to avoid redundant capacity use during review loops `.github/workflows/ci.yml:12`. Shared environment defaults, such as sparse Cargo registry usage and retry tuning, apply uniformly across jobs to stabilise dependency downloads `.github/workflows/ci.yml:16`.

## Step 2: Enforce Baseline Quality Gates
CI starts with `Lint (fmt, clippy)`, which installs the stable toolchain and enforces formatting and linting with warnings-as-errors `.github/workflows/ci.yml:25` `.github/workflows/ci.yml:32` `.github/workflows/ci.yml:41` `.github/workflows/ci.yml:43`. Running `cargo clippy --all-targets --all-features -- -D warnings` guarantees every binary, test, and optional module compiles cleanly; you can reproduce this gate locally with `just fmt-check` and `just clippy` from the `justfile` `.github/workflows/ci.yml:43` `justfile:72` `justfile:80`. Passing this stage is required for downstream jobs because `tests` declares `needs: lint` `.github/workflows/ci.yml:49`.

## Step 3: Run Feature-Aware Tests
The `Unit & Doc Tests` job installs `cargo-nextest` and toggles feature sets depending on branch to align with the feature matrix declared in `Cargo.toml` `.github/workflows/ci.yml:59` `.github/workflows/ci.yml:63` `Cargo.toml:154`. On merges to `main`, it exercises stricter flags, including `strict-sanitization` and `aggressive-trigram-thresholds`, mirroring the production profile `.github/workflows/ci.yml:74` `Cargo.toml:159` `Cargo.toml:160`. For other branches it mirrors the developer-friendly subset used by `just test-fast`, combining `cargo nextest run --lib` with doctests under `git-integration`, `tree-sitter-parsing`, and `mcp-server` `.github/workflows/ci.yml:79` `justfile:40`. Representative coverage comes from:
- `tests/file_storage_integration_test.rs:6` exercising `create_file_storage` in `src/file_storage.rs:514`, ensuring the storage wrapper stack expected by higher-level services remains stable.
- `tests/http_server_integration_test.rs:98` hitting live HTTP routes backed by `create_server` in `src/http_server.rs:260`, confirming REST endpoints accept and mutate documents exactly as the API binaries do.
Testing with `cargo nextest run --no-fail-fast` keeps failing cases visible without halting the suite `.github/workflows/ci.yml:77`.

## Step 4: Exercise Integration Surfaces and Documentation
After unit coverage, CI runs Docker-backed tests behind the `docker-tests` feature flag to validate the MCP authentication middleware against a disposable Postgres instance `.github/workflows/ci.yml:83` `Cargo.toml:161`. The targeted suite invokes `tests/mcp_auth_middleware_test.rs:1`, which expects real sockets and the `AuthMiddleware` stack shipped in `src/mcp/server.rs:6`. Documentation builds reuse the linted toolchain and fail on any rustdoc warning, confirming the API surface stays coherent for consumers `.github/workflows/ci.yml:105` `.github/workflows/ci.yml:121`.

## Step 5: Produce Coverage and Deployment Artifacts
Coverage is gated behind scheduled runs, pushes to `main`, or a PR label named `run-coverage`, balancing runtime cost with reviewer intent `.github/workflows/ci.yml:151`. When it fires, the job installs `cargo-llvm-cov` and publishes LCOV plus HTML bundles via artifacts for inspection `.github/workflows/ci.yml:169` `.github/workflows/ci.yml:172` `.github/workflows/ci.yml:181`. Parallel jobs build release-mode binaries for `kotadb` and `kotadb-api-server` so QA can smoke-test pull requests without compiling locally `.github/workflows/ci.yml:202`. A Buildx-based container check confirms the production Dockerfile stays compatible with Fly.io deployments `.github/workflows/ci.yml:226`. 

| Job | Triggered After | Key Command(s) |
|-----|-----------------|----------------|
| Lint | Workflow start | `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings` `.github/workflows/ci.yml:41` `.github/workflows/ci.yml:43` |
| Tests | Lint | `cargo nextest run --lib --no-default-features --features "git-integration,tree-sitter-parsing"` (branch) or `--features "embeddings-onnx,…"` (main) `.github/workflows/ci.yml:74` `.github/workflows/ci.yml:79` |
| Integration | Tests | `cargo test --no-default-features --features "git-integration,tree-sitter-parsing,mcp-server,docker-tests" --test mcp_auth_middleware_test -- --ignored --nocapture` `.github/workflows/ci.yml:102` |
| Docs | Lint | `cargo doc --no-deps --no-default-features --features "git-integration,tree-sitter-parsing"` `.github/workflows/ci.yml:122` |
| Coverage | Tests + label/schedule | `cargo llvm-cov --workspace --lcov`, `cargo llvm-cov --workspace --html` `.github/workflows/ci.yml:172` `.github/workflows/ci.yml:175` |
| Artifacts | Tests | `cargo build --release --no-default-features --features "git-integration,tree-sitter-parsing" --bin …` `.github/workflows/ci.yml:204` |
| Container Check | Tests | `docker/build-push-action@v5` against `Dockerfile.prod` `.github/workflows/ci.yml:226` |

> **Note** Optional security scans run without blocking merges; failing audits print advisories and rely on maintainers to open follow-up issues `.github/workflows/ci.yml:238` `.github/workflows/ci.yml:251`.

## Step 6: Use Fast CI for Development Loops
For rapid feedback, `Fast CI (Development)` targets feature branches and PRs against `develop`, cancelling superseded runs immediately `.github/workflows/fast-ci.yml:5` `.github/workflows/fast-ci.yml:12`. The `Fast Essential Checks` job mirrors the same lint and build commands but limits integration coverage to `file_storage_integration_test` and `http_server_integration_test`, matching the baseline behaviour validated above `.github/workflows/fast-ci.yml:45` `.github/workflows/fast-ci.yml:64`. Developers can reproduce the exact test set with `cargo nextest run --lib --no-default-features --features "git-integration,tree-sitter-parsing,mcp-server"` followed by the two targeted integration invocations `.github/workflows/fast-ci.yml:56` `.github/workflows/fast-ci.yml:64`. A parallel, non-blocking `Security Scan (Optional)` surfaces `cargo audit` advisories early without breaking iteration `.github/workflows/fast-ci.yml:81`.

## Next Steps
- Run `just test-fast` before pushing to align with the `Unit & Doc Tests` feature set `justfile:40`.
- Reproduce integration gates locally by executing `cargo test --no-default-features --features "git-integration,tree-sitter-parsing,mcp-server,docker-tests" --test mcp_auth_middleware_test -- --ignored` when Docker is available `.github/workflows/ci.yml:102`.
- Attach coverage artifacts to reviews by labeling PRs with `run-coverage` whenever changes touch indexing, storage, or API surfaces `.github/workflows/ci.yml:151`.
