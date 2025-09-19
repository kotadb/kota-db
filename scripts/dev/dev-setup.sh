#!/bin/bash
# Development environment setup script

set -e

echo "🚀 Setting up KotaDB development environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in container
if [ -f /.dockerenv ]; then
    print_status "Running in Docker container"
    IN_CONTAINER=true
else
    print_status "Running on host system"
    IN_CONTAINER=false
fi

# Install dependencies if not in container
if [ "$IN_CONTAINER" = false ]; then
    print_status "Installing system dependencies..."
    
    # Detect OS
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y build-essential pkg-config libssl-dev sqlite3
        elif command -v yum &> /dev/null; then
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y openssl-devel sqlite
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if ! command -v brew &> /dev/null; then
            print_error "Homebrew not found. Please install: https://brew.sh"
            exit 1
        fi
        brew install openssl sqlite
    fi
fi

# Set up Rust environment
print_status "Setting up Rust environment..."

# Install rustup if not present
if ! command -v rustup &> /dev/null; then
    print_status "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    print_success "Rust already installed"
fi

# Install required Rust toolchain and components
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy rust-src

# Install development tools
print_status "Installing Rust development tools..."
(
  set +e
  cargo install --quiet cargo-watch || true
  cargo install --quiet cargo-edit || true
  cargo install --quiet cargo-audit --locked || true
  cargo install --quiet cargo-deny --locked || true
  cargo install --quiet cargo-nextest --locked || true
  cargo install --quiet cargo-llvm-cov || true
  cargo install --quiet bacon || true
) || print_warning "Some tools may already be installed"

# Set up pre-commit hooks
print_status "Setting up pre-commit hooks..."
mkdir -p .git/hooks

cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Pre-commit hook for KotaDB

echo "Running pre-commit checks..."

# Format check
if ! cargo fmt --all -- --check; then
    echo "❌ Code formatting check failed. Run 'cargo fmt' to fix."
    exit 1
fi

# Clippy check
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "❌ Clippy check failed. Fix the warnings above."
    exit 1
fi

# Quick test
if ! cargo test --lib; then
    echo "❌ Library tests failed."
    exit 1
fi

echo "✅ Pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit

# Create development directories
print_status "Creating development directories..."
mkdir -p data logs cache temp

# Set up environment file
print_status "Creating development environment file..."
cat > .env.dev << 'EOF'
# KotaDB Development Environment
RUST_LOG=debug
RUST_BACKTRACE=1
KOTADB_DATA_DIR=./data
KOTADB_LOG_DIR=./logs
KOTADB_CACHE_DIR=./cache
KOTADB_CONFIG_FILE=./kotadb-dev.toml

# MCP Server Development
MCP_SERVER_PORT=8080
MCP_SERVER_HOST=localhost

# Performance Testing
BENCHMARK_DURATION=10s
PERFORMANCE_LOG_LEVEL=info
EOF

# Create development configuration
print_status "Creating development configuration..."
cat > kotadb-dev.toml << 'EOF'
# KotaDB Development Configuration

[database]
data_directory = "./data"
cache_size_mb = 256
enable_wal = true
sync_mode = "normal"

[logging]
level = "debug"
format = "pretty"
log_to_file = true
log_directory = "./logs"

[performance]
enable_metrics = true
metrics_port = 9090
benchmark_on_startup = false

[development]
auto_reload = true
enable_debug_endpoints = true
relaxed_validation = false

[mcp_server]
enabled = true
host = "localhost"
port = 8080
max_connections = 10
timeout_seconds = 30
EOF

# Build the project
print_status "Building KotaDB..."
if cargo build; then
    print_success "Build successful!"
else
    print_error "Build failed. Check the output above."
    exit 1
fi

# Run basic tests
print_status "Running basic tests..."
if cargo test --lib --quiet; then
    print_success "Basic tests passed!"
else
    print_warning "Some tests failed. This is expected if there are unimplemented TODOs."
fi

# Create helpful scripts
print_status "Creating development scripts..."

# Quick development command
cat > dev.sh << 'EOF'
#!/bin/bash
# Quick development commands

case "$1" in
    "setup")
        ./scripts/dev/dev-setup.sh
        ;;
    "test")
        cargo test --all
        ;;
    "watch")
        cargo watch -x 'test --lib' -x 'clippy'
        ;;
    "fmt")
        cargo fmt --all
        ;;
    "clean")
        cargo clean
        rm -rf data logs cache temp
        ;;
    "demo")
        ./run_standalone.sh demo
        ;;
    "docs")
        cargo doc --open --no-deps
        ;;
    "mcp")
        echo "Starting MCP server development mode..."
        RUST_LOG=debug cargo run -- mcp-server --config kotadb-dev.toml
        ;;
    *)
        echo "KotaDB Development Commands:"
        echo "  setup  - Run development environment setup"
        echo "  test   - Run all tests"
        echo "  watch  - Watch for changes and run tests"
        echo "  fmt    - Format code"
        echo "  clean  - Clean build artifacts and data"
        echo "  demo   - Run the Stage 6 demo"
        echo "  docs   - Build and open documentation"
        echo "  mcp    - Start MCP server in development mode"
        ;;
esac
EOF

chmod +x dev.sh

print_success "Development environment setup complete!"
print_status ""
print_status "Next steps:"
print_status "1. Run tests: ./dev.sh test"
print_status "2. Start development: ./dev.sh watch"
print_status "3. Run demo: ./dev.sh demo"
print_status "4. View docs: ./dev.sh docs"
print_status ""
print_status "For containerized development:"
print_status "1. docker-compose -f docker-compose.dev.yml up -d"
print_status "2. docker-compose -f docker-compose.dev.yml exec kotadb-dev bash"
print_status ""
print_success "Happy coding! 🦀"
