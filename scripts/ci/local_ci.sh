#!/usr/bin/env bash
# Local CI runner to mirror GitHub Actions pipeline
# Runs formatting, clippy, build, tests, docs, optional coverage & container build

set -euo pipefail

# -----------------------------
# Environment parity with CI
# -----------------------------
export CARGO_TERM_COLOR=always
export RUST_BACKTRACE=1
export CARGO_INCREMENTAL=0
export CARGO_NET_RETRY=10
export RUSTUP_MAX_RETRIES=10
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
export RUST_TEST_THREADS="4"
export CARGO_BUILD_JOBS="4"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()   { echo -e "${BLUE}[CI]${NC} $*"; }
warn()   { echo -e "${YELLOW}[CI]${NC} $*"; }
error()  { echo -e "${RED}[CI]${NC} $*"; }
success(){ echo -e "${GREEN}[CI]${NC} $*"; }

start_timer() { date +%s; }
end_timer()   { local start=$1; echo $(( $(date +%s) - start )); }

# Track critical and noncritical failures
CRITICAL_FAILED=0
NONCRITICAL_WARNINGS=()

run_step() {
  local name="$1"; shift
  local cmd=("$@")
  info "▶ ${name}"
  local start; start=$(start_timer)
  if "${cmd[@]}"; then
    local secs; secs=$(end_timer "$start")
    success "✓ ${name} (${secs}s)"
    return 0
  else
    local code=$?
    error "✗ ${name} failed (exit ${code})"
    return $code
  fi
}

nonblocking_step() {
  local name="$1"; shift
  local cmd=("$@")
  if ! run_step "$name" "${cmd[@]}"; then
    NONCRITICAL_WARNINGS+=("${name}")
  fi
}

# -----------------------------
# Pre-flight checks
# -----------------------------
if [ ! -f Cargo.toml ]; then
  error "Run from repository root (Cargo.toml not found)"
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  error "cargo not found. Install Rust from https://rustup.rs/"
  exit 2
fi

# Helpful tools (optional but recommended)
if ! command -v cargo-nextest >/dev/null 2>&1 && ! command -v cargo nextest >/dev/null 2>&1; then
  warn "cargo-nextest not found; using 'cargo test' fallback (slower)"
  USE_NEXTEST=0
else
  USE_NEXTEST=1
fi

# -----------------------------
# CI Steps (critical first)
# -----------------------------

if ! run_step "Format check" bash -lc "cargo fmt --all -- --check"; then CRITICAL_FAILED=1; fi
if ! run_step "Clippy (all targets, all features)" bash -lc "cargo clippy --all-targets --all-features -- -D warnings"; then CRITICAL_FAILED=1; fi
if ! run_step "Build (debug, all features)" bash -lc "cargo build --all-features"; then CRITICAL_FAILED=1; fi
if ! run_step "Build release binary" bash -lc "cargo build --release --bin kotadb"; then CRITICAL_FAILED=1; fi

if [ "$USE_NEXTEST" -eq 1 ]; then
  if ! run_step "Unit tests (nextest)" bash -lc "cargo nextest run --lib --all-features"; then CRITICAL_FAILED=1; fi
else
  if ! run_step "Unit tests" bash -lc "cargo test --lib --all-features"; then CRITICAL_FAILED=1; fi
fi

if ! run_step "Doc tests" bash -lc "cargo test --doc --all-features"; then CRITICAL_FAILED=1; fi

# Integration + e2e: locally run full test matrix with nextest if available
if [ "$USE_NEXTEST" -eq 1 ]; then
  if ! run_step "Full test matrix (nextest)" bash -lc "cargo nextest run --all-features --no-fail-fast"; then CRITICAL_FAILED=1; fi
else
  if ! run_step "Full test matrix" bash -lc "cargo test --all-features -- --nocapture"; then CRITICAL_FAILED=1; fi
fi

# -----------------------------
# Non-blocking informational steps
# -----------------------------

# Security audits (informational locally)
if command -v cargo-audit >/dev/null 2>&1; then
  nonblocking_step "Security audit (cargo-audit)" bash -lc "cargo audit"
else
  warn "cargo-audit not installed; skipping security audit"
fi

if command -v cargo-deny >/dev/null 2>&1; then
  nonblocking_step "License/Advisory checks (cargo-deny)" bash -lc "cargo deny check all"
else
  warn "cargo-deny not installed; skipping cargo-deny checks"
fi

# Documentation build
nonblocking_step "Build docs" bash -lc "RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features"

# Container build (optional)
if command -v docker >/dev/null 2>&1; then
  nonblocking_step "Docker build (test)" bash -lc "docker build -t kotadb:ci-test -f Dockerfile ."
else
  warn "Docker not found; skipping container build"
fi

# MCP package tests (if present and npm available)
if [ -d "kotadb-mcp-package" ] && [ -f "kotadb-mcp-package/package.json" ] && command -v npm >/dev/null 2>&1; then
  info "▶ MCP package detected; running Node tests"
  ( set -e
    pushd kotadb-mcp-package >/dev/null
    npm pkg delete scripts.postinstall 2>/dev/null || true
    npm ci
    npm run build
    KOTADB_BINARY_PATH=../target/release/kotadb npm run test:unit
    KOTADB_BINARY_PATH=../target/release/kotadb npm run test:integration
    popd >/dev/null
  ) || NONCRITICAL_WARNINGS+=("MCP package tests")
else
  warn "MCP package or npm not available; skipping"
fi

# Coverage (optional)
if command -v cargo-llvm-cov >/dev/null 2>&1; then
  nonblocking_step "Coverage (llvm-cov)" bash -lc "cargo llvm-cov clean --workspace && cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info"
else
  warn "cargo-llvm-cov not installed; skipping coverage"
fi

# -----------------------------
# Summary and exit
# -----------------------------
echo
info "CI summary (local)"
if [ "$CRITICAL_FAILED" -ne 0 ]; then
  error "Critical CI steps failed. See logs above."
  exit 1
fi

if [ ${#NONCRITICAL_WARNINGS[@]} -gt 0 ]; then
  warn "Non-blocking steps with issues: ${NONCRITICAL_WARNINGS[*]}"
fi

success "All critical CI steps passed locally."
exit 0

