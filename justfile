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
  cargo watch -x 'run -- --config kotadb-dev.toml'

# Start MCP server in development mode
mcp:
  RUST_LOG=debug cargo run -- mcp-server --config kotadb-dev.toml

# Watch for changes and run tests
watch:
  cargo watch -x 'test --lib' -x 'clippy'

# === Testing ===

# Run all tests
test:
  cargo test --all

# Run only unit tests
test-unit:
  cargo test --lib

# Run only integration tests  
test-integration:
  cargo test --test '*'

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

# Run all quality checks
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

# Initialize a test database
init-db path="./test-data":
  mkdir -p {{path}}
  KOTADB_DATA_DIR={{path}} cargo run -- init

# Benchmark database operations
db-bench:
  cargo run --release -- benchmark --operations 10000

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

# Run the same checks as CI
ci: fmt-check clippy test audit
  @echo "🚀 CI checks completed successfully!"

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

# Prepare for release
pre-release version:
  @echo "Preparing release {{version}}"
  # Update version in Cargo.toml
  sed -i 's/version = ".*"/version = "{{version}}"/' Cargo.toml
  # Run all checks
  just ci
  # Build release
  just build-release
  @echo "✅ Ready for release {{version}}"

# Create release tag
release version: (pre-release version)
  git add Cargo.toml Cargo.lock
  git commit -m "chore: bump version to {{version}}"
  git tag -a v{{version}} -m "Release v{{version}}"
  @echo "🏷️  Created tag v{{version}}"
  @echo "Push with: git push origin main && git push origin v{{version}}"
