# KotaDB Development Tasks
# Run with: just <task-name>

# Default task - show available tasks
default:
  @just --list

# === Development ===

# Set up development environment
setup:
  ./scripts/dev/dev-setup.sh

# Run development server with auto-reload
dev:
  cargo watch -x 'run --bin mcp_server --features mcp-server -- --config kotadb-dev.toml'

# Start MCP server in development mode
mcp:
  RUST_LOG=debug cargo run --bin mcp_server --features mcp-server -- --config kotadb-dev.toml

# Watch for changes and run tests (fast)
watch:
  @echo "📝 Note: cargo-watch may not be available on all systems"
  @echo "🚀 Using cargo-nextest for 3-5x faster test execution"
  cargo watch -x 'nextest run --lib' -x 'clippy' || echo "❌ cargo-watch not available - use 'just test-fast' instead"

# === Testing ===

# Run all tests (FAST - uses cargo-nextest for 3-5x speedup)
test:
  cargo nextest run --all --no-fail-fast

# Fast test execution using cargo-nextest (recommended)
test-fast:
  cargo nextest run --all

# Run only unit tests (FAST)
test-unit:
  cargo nextest run --lib

# Run only integration tests (FAST)
test-integration:
  cargo nextest run --test '*'

# Run performance tests
test-perf:
  cargo test --release --features bench performance_regression_test

# Run property-based tests
test-property:
  cargo test --test property_tests

# Run infrastructure validation tests
test-infrastructure:
  ./scripts/infrastructure_test.sh

# Generate test coverage report
coverage:
  cargo llvm-cov --all-features --workspace --html
  @echo "Coverage report: target/llvm-cov/html/index.html"

# === Code Quality ===

# Format code
fmt:
  cargo fmt --all

# Check formatting without changing files
fmt-check:
  cargo fmt --all -- --check

# Run clippy linting
clippy:
  cargo clippy --all-targets --all-features -- -D warnings

# Run all quality checks (FAST - uses cargo-nextest)
check: fmt-check clippy test-unit
  @echo "✅ All quality checks passed!"

# Security audit
audit:
  cargo audit
  cargo deny check all

# Update dependencies
update:
  cargo update
  cargo outdated

# === Documentation ===

# Build and open documentation
docs:
  cargo doc --open --no-deps

# Build all documentation
docs-all:
  cargo doc --all --all-features --no-deps

# Serve documentation on http://localhost:8000
docs-serve:
  python3 -m http.server 8000 -d target/doc

# === Performance ===

# Run benchmarks
bench:
  cargo bench --features bench

# Profile the application
profile binary="kotadb":
  cargo build --release --bin {{binary}}
  perf record --call-graph=dwarf target/release/{{binary}} --help
  perf report

# === Database Operations ===

# Run the Stage 6 demo
demo:
  ./run_standalone.sh demo

# Run all examples
examples:
  @echo "🧠 Running KotaDB Examples"
  @echo "=========================="
  @echo
  @echo "📚 1. Personal Knowledge Base"
  RUST_LOG=warn cargo run --example 01_personal_knowledge_base
  @echo
  @echo "🔬 2. Research Project Manager"  
  RUST_LOG=warn cargo run --example 02_research_project_manager
  @echo
  @echo "📅 3. Meeting Notes System"
  RUST_LOG=warn cargo run --example 03_meeting_notes_system
  @echo
  @echo "✅ All examples completed successfully!"

# Initialize a test database
init-db path="./test-data":
  mkdir -p {{path}}
  KOTADB_DATA_DIR={{path}} cargo run --bin kotadb -- stats

# Benchmark codebase intelligence operations
db-bench:
  @echo "Running KotaDB Codebase Intelligence Benchmarks"
  @echo "Testing: Repository indexing, code search, symbol queries, relationship analysis"
  @echo ""
  cargo bench --features bench --bench codebase_intelligence_bench
  @echo ""
  @echo "Running resource usage and concurrent operation benchmarks"
  cargo bench --features bench --bench resource_usage_bench

# Dogfood the HTTP API end-to-end against this repo
dogfood:
  @echo "🍽️  Dogfooding KotaDB via HTTP API"
  @echo "   - Starts server, indexes current repo, runs queries"
  @echo "   - Override with: PORT=18080 KOTADB_DATA_DIR=./kotadb-data/dogfood-http"
  bash ./scripts/dogfood.sh

# === Container Development ===

# Start development containers
docker-up:
  ./scripts/dev/docker-dev.sh up

# Stop development containers
docker-down:
  ./scripts/dev/docker-dev.sh down

# Connect to development container
docker-shell:
  ./scripts/dev/docker-dev.sh shell

# === CI/CD ===

# Install common CI/dev tools locally
install-ci-tools:
  @echo "📦 Installing CI/dev tools (may already be installed)..."
  cargo install cargo-nextest --locked || true
  cargo install cargo-audit --locked || true
  cargo install cargo-deny --locked || true
  cargo install cargo-llvm-cov || true
  @echo "✅ Tools ready: cargo-nextest, cargo-audit, cargo-deny, cargo-llvm-cov"

# Fast local CI (format, clippy, unit tests, audit)
ci-fast: fmt-check clippy test audit
  @echo "🚀 Fast CI checks completed successfully!"

# Full local CI mirroring GitHub pipeline
ci:
  bash ./scripts/ci/local_ci.sh

# Build release binaries
build-release:
  cargo build --release

# Build Docker image
docker-build tag="kotadb:dev":
  docker build -t {{tag}} .

# === Deployment ===

# Deploy to Kubernetes (development)
k8s-deploy-dev:
  kubectl apply -k k8s/overlays/development

# Deploy to Kubernetes (production)
k8s-deploy-prod:
  kubectl apply -k k8s/overlays/production

# Generate Kubernetes manifests
k8s-generate env="development":
  kubectl kustomize k8s/overlays/{{env}}

# === Cleanup ===

# Clean build artifacts
clean:
  cargo clean
  rm -rf data logs cache temp

# Deep clean (including Docker)
clean-all: clean
  docker system prune -f
  ./scripts/dev/docker-dev.sh clean

# === Release ===

# Show current version
version:
  @grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2

# Bump version (major, minor, patch, or prerelease)
bump type="patch":
  ./scripts/version-bump.sh {{type}} --preview

# Create a new release (runs full release process)
release version:
  ./scripts/release.sh {{version}}

# Create a release with automatic version bump
release-patch:
  ./scripts/version-bump.sh patch

release-minor:
  ./scripts/version-bump.sh minor

release-major:
  ./scripts/version-bump.sh major

release-beta:
  ./scripts/version-bump.sh prerelease

# Dry run of release process
release-dry-run version:
  ./scripts/release.sh {{version}} --dry-run

# Update changelog (add new unreleased section)
changelog-update:
  @echo "## [Unreleased]" > CHANGELOG.tmp
  @echo "" >> CHANGELOG.tmp
  @echo "### Added" >> CHANGELOG.tmp
  @echo "" >> CHANGELOG.tmp
  @echo "### Changed" >> CHANGELOG.tmp
  @echo "" >> CHANGELOG.tmp
  @echo "### Fixed" >> CHANGELOG.tmp
  @echo "" >> CHANGELOG.tmp
  @echo "### Security" >> CHANGELOG.tmp
  @echo "" >> CHANGELOG.tmp
  @tail -n +2 CHANGELOG.md >> CHANGELOG.tmp
  @mv CHANGELOG.tmp CHANGELOG.md
  @echo "✅ CHANGELOG.md updated with new unreleased section"

# Check what would be included in next release
release-preview:
  @echo "📦 Next Release Preview"
  @echo "======================="
  @echo
  @echo "Current version: $(just version)"
  @echo
  @echo "Unreleased changes:"
  @echo "-------------------"
  @awk '/^## \[Unreleased\]/{flag=1; next} /^## \[/{flag=0} flag' CHANGELOG.md
  @echo
  @echo "Recent commits since last tag:"
  @echo "------------------------------"
  @git log --oneline $(git describe --tags --abbrev=0 2>/dev/null || echo HEAD~10)..HEAD 2>/dev/null || echo "No tags found"

# Tag current commit without full release process
tag-version version:
  git tag -a v{{version}} -m "Version {{version}}"
  @echo "Tagged as v{{version}}"
  @echo "Push with: git push origin v{{version}}"
