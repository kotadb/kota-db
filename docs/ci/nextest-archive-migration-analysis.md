# Cargo Nextest Archive Migration for KotaDB CI

## Summary
GitHub Actions currently rebuilds the entire KotaDB workspace in every test job because each workflow executes in a fresh VM and re-runs `cargo build` or `cargo test` from scratch (`.github/workflows/ci.yml:45-215`). The feature-gated layout in `Cargo.toml:154-177` is already proven by `scripts/test-nextest-archive.sh:39-156`, so we can package one `cargo nextest archive` bundle and reuse it across the matrix without sacrificing doc tests or Docker-guarded suites.

## Step-by-Step Migration

### Step 1. Profile the Current Pipeline
- `tests` performs a branch-sensitive debug build and executes unit plus doc tests (`.github/workflows/ci.yml:61-80`). Because the job does not publish its `target` directory, every downstream job recompiles identical crates.
- `integration-docker` reruns `cargo test` with Docker features in a clean environment (`.github/workflows/ci.yml:83-103`), while `artifacts` rebuilds release binaries (`.github/workflows/ci.yml:188-215`). Both repeat the compilation work already completed upstream.
- The cache wired in at `.github/workflows/ci.yml:55-58` accelerates dependency downloads but cannot transport Cargo fingerprints between machines, so it does not prevent redundant builds.
- Capture a local baseline with `scripts/ci/local_ci.sh`; when `cargo-nextest` is detected it mirrors the same `cargo build` → `cargo nextest run` ordering we plan to collapse (`scripts/ci/local_ci.sh:86-104`).

### Step 2. Produce a Nextest Archive During the Build Stage
- Extract the branch-aware feature list shown in `cargo build` (`.github/workflows/ci.yml:63-76`) so the archive matches the default and main-branch profiles defined in `Cargo.toml:154-177`.
- Introduce a `build-tests` job that stops after building once and emitting an archive. The command sequence already validated locally in `scripts/test-nextest-archive.sh:39-55` is the template to copy.
```yaml
build-tests:
  name: Build and Archive Tests
  runs-on: ubuntu-latest
  timeout-minutes: 15
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
      with:
        cache-on-failure: true
        shared-key: nextest-archive
    - name: Build and package test binaries
      run: |
        FEATURES="git-integration,tree-sitter-parsing"
        if [[ "${{ github.ref }}" == "refs/heads/main" || "${{ github.base_ref }}" == "main" ]]; then
          FEATURES="embeddings-onnx,git-integration,tree-sitter-parsing,mcp-server,strict-sanitization,aggressive-trigram-thresholds"
        fi
        cargo build --no-default-features --features "$FEATURES"
        cargo nextest archive \
          --archive-file nextest-archive.tar.zst \
          --no-default-features \
          --features "$FEATURES"
    - uses: actions/upload-artifact@v4
      with:
        name: nextest-archive
        path: nextest-archive.tar.zst
        retention-days: 1
```
- Keep the release binaries in `artifacts`; they already rely on the same feature gate and can optionally reuse the archive if you also upload `target/release` from this job.

### Step 3. Consume the Archive in Unit, Doc, and Integration Jobs
- Split the current `tests` job into a `unit-tests` job that depends on `build-tests`, downloads the archive, and runs the unit suite with zero compilation:
```yaml
unit-tests:
  needs: [build-tests]
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: taiki-e/install-action@nextest
    - uses: actions/download-artifact@v4
      with:
        name: nextest-archive
    - name: Unit suite from archive
      run: cargo nextest run \
        --archive-file nextest-archive.tar.zst \
        --lib \
        --no-fail-fast
    - name: Rust doc tests
      run: cargo test --doc --no-default-features --features "$FEATURES"
```
- Convert `integration-docker` into an archive consumer so Docker-only suites run against prebuilt binaries. The ignored tests currently executed with `cargo test` (`.github/workflows/ci.yml:101-103`) map to `cargo nextest run --run-ignored ignored-only --test mcp_auth_middleware_test`.
```yaml
integration-docker:
  needs: [build-tests]
  strategy:
    fail-fast: false
    matrix:
      partition: [1, 2, 3, 4]
  steps:
    - uses: actions/checkout@v4
    - uses: taiki-e/install-action@nextest
    - uses: actions/download-artifact@v4
      with:
        name: nextest-archive
    - name: Integration partition ${{ matrix.partition }}
      run: cargo nextest run \
        --archive-file nextest-archive.tar.zst \
        --test '*' \
        --partition count:${{ matrix.partition }}/4 \
        --no-fail-fast
    - name: Docker-only ignored suite
      run: cargo nextest run \
        --archive-file nextest-archive.tar.zst \
        --test mcp_auth_middleware_test \
        --run-ignored ignored-only \
        --no-fail-fast
```
> **Note** Doc tests remain on `cargo test --doc` because Nextest ignores doctests by design; keep them colocated with the unit job so they still reuse the downloaded source checkout.

### Step 4. Partition Integration Suites and Align Fast Pipelines
- Keep the four-way partition used in CI by relying on Nextest’s native partitioning, as exercised in `scripts/test-nextest-archive.sh:77-93`. This keeps runtime parity with the existing matrix while eliminating rebuilds.
- Update the critical subsets in `fast-ci.yml:53-66` to optionally download `nextest-archive` when the workflow is triggered together with the main CI, or leave `fast-ci` untouched if you prefer independent runs.
- For local smoke tests, document that `just test-fast` already matches the feature flags the archive expects (`justfile:39-42`); developers should run it before pushing to ensure the archive build will succeed.

### Step 5. Validate Archives Locally and Monitor in CI
- Run `scripts/test-nextest-archive.sh` before opening the PR; it verifies archive creation, extraction, and multi-partition execution without re-triggering compilation (`scripts/test-nextest-archive.sh:63-155`).
- After wiring the workflow, confirm that GitHub Actions reports cache hits by watching for the absence of `Compiling` lines in unit and integration logs.
- Track artifact size and retention in the `build-tests` job; the script logs compression details (`scripts/test-nextest-archive.sh:50-54`) that match what you should see in CI.
> **Warning** Coverage still needs a fresh instrumentation build (`.github/workflows/ci.yml:142-185`), so do not swap `cargo llvm-cov` to archive mode; keep that job isolated.

## Reference: Archive Layout & Tooling
- `cargo nextest archive` bundles every binary, dynamic dependency, and fingerprint necessary for the unit and integration suites (`scripts/test-nextest-archive.sh:39-55`).
- Extraction does not populate `target/debug`, so Nextest runs directly from the archive and proves no recompilation occurs (`scripts/test-nextest-archive.sh:95-114`).
- The feature toggles that influence coverage (`strict-sanitization`, `aggressive-trigram-thresholds`, `docker-tests`) are all defined in `Cargo.toml:159-177`; reuse them verbatim when setting the `FEATURES` environment block.
- Local automation can continue to call `scripts/ci/local_ci.sh` to mirror the order of operations but will skip archive logic, ensuring developers without the artifact still have deterministic checks.

## Next Steps
- Open a branch that introduces the `build-tests` archive job and refactors downstream jobs to depend on it.
- Verify the rewritten pipeline with `scripts/test-nextest-archive.sh` and `just test-fast`.
- Capture CI run metrics (duration, artifact size) after the change and compare to the previous baseline.
- Roll the update into `main` once the matrix confirms runtime savings, then monitor for one sprint before deleting redundant fallback logic.
