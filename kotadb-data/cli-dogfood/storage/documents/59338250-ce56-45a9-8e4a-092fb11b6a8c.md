---
tags:
- file
- kota-db
- ext_md
---
# KotaDB Development Guide

## 🚀 Quick Start

### Option 1: Native Development (Recommended for macOS/Linux)
```bash
# Clone the repository
git clone https://github.com/jayminwest/kota-db.git
cd kota-db

# Run development setup
./scripts/dev/dev-setup.sh

# Start development with watch mode
./dev.sh watch
```

### Option 2: Containerized Development
```bash
# Start development environment
./scripts/dev/docker-dev.sh up

# Connect to development container
./scripts/dev/docker-dev.sh shell

# Inside container, run setup
./scripts/dev/dev-setup.sh
```

## 📋 Development Commands

### Native Development
```bash
./dev.sh setup   # Run development environment setup
./dev.sh test    # Run all tests
./dev.sh watch   # Watch for changes and run tests
./dev.sh fmt     # Format code
./dev.sh demo    # Run the Stage 6 demo
./dev.sh docs    # Build and open documentation
./dev.sh mcp     # Start MCP server in development mode
```

### Containerized Development
```bash
./scripts/dev/docker-dev.sh up      # Start environment
./scripts/dev/docker-dev.sh shell   # Connect to container
./scripts/dev/docker-dev.sh test    # Run tests in container
./scripts/dev/docker-dev.sh watch   # Start watch mode
./scripts/dev/docker-dev.sh docs    # Build docs (available at http://localhost:8001)
./scripts/dev/docker-dev.sh mcp     # Start MCP server
./scripts/dev/docker-dev.sh down    # Stop environment
```

## 🏗️ Project Architecture

KotaDB follows a **6-stage risk reduction methodology**:

1. **Test-Driven Development** (-5.0 risk)
2. **Contract-First Design** (-5.0 risk)
3. **Pure Function Modularization** (-3.5 risk)
4. **Comprehensive Observability** (-4.5 risk)
5. **Adversarial Testing** (-0.5 risk)
6. **Component Library** (-1.0 risk)

**Total Risk Reduction**: -19.5 points (99% success rate)

### Key Design Patterns
- **Validated Types**: Invalid states are unrepresentable
- **Builder Patterns**: Fluent APIs with sensible defaults
- **Wrapper Components**: Automatic cross-cutting concerns
- **Pure Functions**: Predictable, testable business logic

## 🧪 Testing Strategy

### Test Types
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Property-based tests
cargo test --test property_tests

# Performance tests
cargo test --release --features bench performance_regression_test

# All tests
cargo test --all
```

### Coverage
```bash
# Generate coverage report
cargo llvm-cov --all-features --workspace --html
# Report available in target/llvm-cov/html/index.html
```

## 🔧 Code Quality

### Pre-commit Checks
```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Security audit
cargo audit

# Dependency check
cargo outdated
```

### Automated Quality Gates
All PRs must pass:
- ✅ Code formatting (`cargo fmt`)
- ✅ Linting (`cargo clippy`) 
- ✅ All tests (`cargo test`)
- ✅ Security audit (`cargo audit`)
- ✅ Documentation builds (`cargo doc`)

## 📊 Performance Monitoring

### Benchmarks
```bash
# Run benchmarks
cargo bench --features bench

# Performance regression tests
cargo test --release performance_regression_test
```

### Metrics
- Query latency target: <10ms
- Bulk operation speedup: 10x
- Memory overhead: <2.5x raw data
- Test coverage: >90%

## 🐳 Container Development

### Services Available
- **kotadb-dev**: Main development environment (port 8080)
- **docs-server**: Documentation server (port 8001)
- **redis-dev**: Development cache (port 6379)
- **postgres-dev**: Test database (port 5432)

### Development Workflow
```bash
# Start full environment
docker-compose -f docker-compose.dev.yml up -d

# Connect to main container
docker-compose -f docker-compose.dev.yml exec kotadb-dev bash

# Inside container
./dev.sh watch    # Start development mode
./dev.sh mcp      # Start MCP server
```

## 🔍 Debugging

### Logging
```bash
# Enable debug logging
export RUST_LOG=debug

# Specific module logging
export RUST_LOG=kotadb::storage=debug,kotadb::index=info

# Run with full backtrace
export RUST_BACKTRACE=full
```

### Development Tools
- **bacon**: Continuous checking (`bacon`)
- **cargo-watch**: Watch for changes (`cargo watch -x test`)
- **cargo-expand**: Expand macros (`cargo expand`)
- **cargo-tree**: Dependency tree (`cargo tree`)

## 🌐 MCP Server Development

### Starting MCP Server
```bash
# Development mode
RUST_LOG=debug cargo run -- mcp-server --config kotadb-dev.toml

# Or using dev script
./dev.sh mcp
```

### Testing MCP Integration
```bash
# Test JSON-RPC endpoint
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

### MCP Development Ports
- **8080**: MCP server
- **9090**: Metrics endpoint

## 📚 Documentation

### Building Docs
```bash
# API documentation
cargo doc --no-deps --open

# Serve documentation
# Available at http://localhost:8001 in container mode
```

### Documentation Types
- **API Docs**: Generated from rustdoc comments
- **User Guide**: `/docs` directory
- **Architecture**: `AGENT_CONTEXT.md`, `MCP_INTEGRATION_PLAN.md`
- **Development**: This guide

## 🐛 Troubleshooting

### Common Issues

**Build fails with linking errors**:
```bash
# Install system dependencies
./scripts/dev/dev-setup.sh
```

**Tests fail with file permission errors**:
```bash
# Fix permissions
chmod -R 755 data logs cache
```

**Container fails to start**:
```bash
# Clean and rebuild
./scripts/dev/docker-dev.sh clean
./scripts/dev/docker-dev.sh build
```

**MCP server connection refused**:
```bash
# Check if port is available
lsof -i :8080

# Restart with debug logging
RUST_LOG=debug ./dev.sh mcp
```

### Getting Help
- 🐛 **Bugs**: Open issue with bug report template
- 💡 **Features**: Open issue with feature request template
- 🤔 **Questions**: Start a GitHub Discussion
- 📖 **Docs**: Check `/docs` directory

## 🚀 Contributing

### Development Flow
1. **Fork & Clone**: Fork repository and clone locally
2. **Setup**: Run `./scripts/dev/dev-setup.sh`
3. **Branch**: Create feature branch (`git checkout -b feature/name`)
4. **Develop**: Write code following project patterns
5. **Test**: Ensure all tests pass (`./dev.sh test`)
6. **Format**: Format code (`./dev.sh fmt`)
7. **Commit**: Use conventional commits
8. **Push**: Push to your fork
9. **PR**: Open pull request with template

### Code Style
- Follow Rust standard formatting
- Use meaningful names
- Add rustdoc for public APIs
- Include examples in documentation
- Never use `unwrap()` in production code

### Commit Messages
```bash
# Format: type(scope): description
feat(mcp): add semantic search tool
fix(storage): resolve memory leak in bulk operations
docs(api): add examples for document builder
test(index): add property tests for B+ tree
```

## 📈 Project Status

### Completed ✅
- Storage engine with Stage 6 safety wrappers
- Primary and trigram indices
- Comprehensive CI/CD pipeline
- Development environment setup
- Production containerization

### In Progress 🔄
- MCP server implementation
- Semantic search integration
- Performance optimization

### Planned 📋
- Advanced analytics tools
- Multi-tenant support
- Distributed indexing
- Machine learning integration

---

**Ready to contribute?** Start with the [Contributing Guide](CONTRIBUTING.md) and check [Outstanding Issues](OUTSTANDING_ISSUES.md) for current priorities.
