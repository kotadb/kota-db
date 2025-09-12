#!/usr/bin/env bash
set -euo pipefail

# Local CI runner that mirrors GitHub CI and Fly.io container build
# - Builds production image with Dockerfile.production for linux/amd64
# - Builds a CI runner image (Rust 1.85) and runs the same steps as CI
# - Optionally runs Docker-backed integration tests (requires Docker socket)

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

PLATFORM="linux/amd64"
CI_IMAGE="kotadb:ci-runner"
PROD_IMAGE="kotadb:ci-prod"

echo "[local-ci] Ensuring binfmt/qemu and buildx are available..."
docker run --privileged --rm tonistiigi/binfmt --install all >/dev/null 2>&1 || true
docker buildx inspect >/dev/null 2>&1 || docker buildx create --use

echo "[local-ci] Building production image ($PLATFORM) from Dockerfile.production..."
if docker buildx build --help >/dev/null 2>&1; then
  set +e
  docker buildx build \
    --platform "$PLATFORM" \
    -f Dockerfile.production \
    -t "$PROD_IMAGE" \
    --load \
    .
  BX=$?
  set -e
  if [ "$BX" -ne 0 ]; then
    echo "[local-ci] buildx failed; falling back to native docker build (arch may differ)."
    docker build -f Dockerfile.production -t "$PROD_IMAGE" .
  fi
else
  echo "[local-ci] buildx not available; using native docker build."
  docker build -f Dockerfile.production -t "$PROD_IMAGE" .
fi

echo "[local-ci] Building CI runner image ($PLATFORM) from Dockerfile.ci..."
if docker buildx build --help >/dev/null 2>&1; then
  set +e
  docker buildx build \
    --platform "$PLATFORM" \
    -f Dockerfile.ci \
    -t "$CI_IMAGE" \
    --load \
    .
  BX=$?
  set -e
  if [ "$BX" -ne 0 ]; then
    echo "[local-ci] buildx failed; falling back to native docker build (arch may differ)."
    docker build -f Dockerfile.ci -t "$CI_IMAGE" .
  fi
else
  echo "[local-ci] buildx not available; using native docker build."
  docker build -f Dockerfile.ci -t "$CI_IMAGE" .
fi

# Reusable docker run wrapper
run_ci() {
  local cmd="$*"
  echo "[local-ci] >>> $cmd"
  docker run --rm \
    --platform "$PLATFORM" \
    -v "$ROOT_DIR":/workspace \
    -w /workspace \
    -e RUST_LOG=error \
    -e CI=true \
    -e CARGO_HOME=/cargo-home \
    -e CARGO_TARGET_DIR=/workspace/target \
    -v "$HOME/.cargo/registry":/cargo-home/registry \
    -v "$HOME/.cargo/git":/cargo-home/git \
    "$CI_IMAGE" \
    bash -lc "$cmd"
}

echo "[local-ci] Running Lint (fmt, clippy)"
run_ci "command -v rustup >/dev/null 2>&1 && rustup show || true; cargo fmt --all -- --check"
run_ci "cargo clippy --all-targets --all-features -- -D warnings"

echo "[local-ci] Running Unit & Doc Tests"
run_ci "cargo build --no-default-features --features 'git-integration,tree-sitter-parsing'"
run_ci "cargo nextest run --no-default-features --features 'git-integration,tree-sitter-parsing' --no-fail-fast"
run_ci "cargo test --doc --no-default-features --features 'git-integration,tree-sitter-parsing'"
run_ci "RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --no-default-features --features 'git-integration,tree-sitter-parsing'"

echo "[local-ci] Running Docker-backed Integration (requires Docker socket)"
docker run --rm \
  --platform "$PLATFORM" \
  -v "$ROOT_DIR":/workspace \
  -w /workspace \
  -e RUST_LOG=warn \
  -e CI=true \
  -e CARGO_HOME=/cargo-home \
  -e CARGO_TARGET_DIR=/workspace/target \
  -v "$HOME/.cargo/registry":/cargo-home/registry \
  -v "$HOME/.cargo/git":/cargo-home/git \
  -v /var/run/docker.sock:/var/run/docker.sock \
  "$CI_IMAGE" \
  bash -lc "cargo test --no-default-features --features 'git-integration,tree-sitter-parsing' --test mcp_auth_middleware_test -- --ignored --nocapture" || echo "[local-ci] Integration docker test failed or skipped; investigate if required."

echo "[local-ci] Building coverage artifacts (optional)"
run_ci "cargo llvm-cov --no-default-features --features 'git-integration,tree-sitter-parsing' --workspace --lcov --output-path lcov.info"
run_ci "cargo llvm-cov --no-default-features --features 'git-integration,tree-sitter-parsing' --workspace --html"

echo "[local-ci] Success. Images built: $PROD_IMAGE and $CI_IMAGE"
