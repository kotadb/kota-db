# 🤖 Agent Context: KotaDB Standalone Project

## ⚠️ IMPORTANT: This is a Standalone Project

**KotaDB is a complete, independent project within the broader kota_md workspace.**

When working on KotaDB:
- **Treat this as a separate repository** with its own lifecycle
- **All work should be contained within this directory**
- **This project has its own documentation, tests, and deployment**
- **Use the standalone execution tools**: `./run_standalone.sh`

## 🎯 Project Status: Storage Engine Complete

✅ **All 6 Risk Reduction Stages Complete**
- Stage 1: Test-Driven Development (-5.0 risk)
- Stage 2: Contract-First Design (-5.0 risk) 
- Stage 3: Pure Function Modularization (-3.5 risk)
- Stage 4: Comprehensive Observability (-4.5 risk)
- Stage 5: Adversarial Testing (-0.5 risk)
- Stage 6: Component Library (-1.0 risk)

✅ **FileStorage Implementation Complete**
- Production-ready file-based storage engine
- Full Stage 6 wrapper composition applied
- Integration tests and documentation complete

**Total Risk Reduction**: -19.5 points (99% success rate)
**Current Phase**: Ready for index implementation

## 📁 Project Structure

```
kota-db/
├── AGENT_CONTEXT.md     ← You are here
├── README.md            ← Project overview
├── STANDALONE.md        ← Standalone usage guide
├── run_standalone.sh    ← Primary execution tool
├── Cargo.toml          ← Rust project configuration
├── .gitignore          ← Git ignore rules
├── src/                ← Source code
├── tests/              ← Test suites
├── docs/               ← Comprehensive documentation
├── examples/           ← Usage examples
├── benches/            ← Performance benchmarks
└── handoffs/           ← Development history
```

## 🚀 Quick Start for Agents

```bash
# Get project status
./run_standalone.sh status

# Run tests
./run_standalone.sh test

# See Stage 6 demo
./run_standalone.sh demo

# Build project
./run_standalone.sh build
```

## 🏗️ Architecture Principles

### 1. Component Library Approach
- **Validated Types**: Compile-time safety
- **Builder Patterns**: Fluent APIs
- **Wrapper Components**: Automatic best practices

### 2. Risk Reduction First
- Every component designed to prevent failures
- Comprehensive testing at all levels
- Observable, debuggable, maintainable

### 3. Pure Functions + Contracts
- Clear interfaces with pre/post conditions
- Immutable data structures where possible
- Predictable, testable behavior

## 📋 Current Implementation Status

✅ **Foundation Complete**
- All core traits and contracts defined
- Validation layer implemented
- Observability infrastructure ready
- Component library functional

✅ **FileStorage Implementation Complete**
- `src/file_storage.rs` - Production-ready storage engine
- `create_file_storage()` - Factory with all Stage 6 wrappers
- `tests/file_storage_integration_test.rs` - Comprehensive tests
- `examples/file_storage_demo.rs` - Usage demonstration

🔄 **Ready for Next Phase**
- Index implementations (using Stage 6 metered wrappers)
- Query engine (leveraging pure functions)
- CLI integration (builder patterns)

## 🎯 For New Agents: Essential Reading

1. **Read `handoffs/README.md`** - Understand project history
2. **Read `docs/architecture/stage6_component_library.md`** - Core architecture
3. **Run `./run_standalone.sh demo`** - See components in action
4. **Check `docs/api/quick_reference.md`** - Development patterns

## 🚨 Critical Guidelines

### DO:
- Use the component library (builders, wrappers, validated types)
- Follow the 6-stage methodology principles
- Add comprehensive tests for new features
- Use the standalone execution tools
- Maintain observability and validation

### DON'T:
- Break the risk reduction achievements
- Bypass validation or safety mechanisms
- Add dependencies without careful consideration
- Ignore the existing architectural patterns
- Work outside this directory structure

## 💡 Development Philosophy

> "Prevention is better than detection. The component library approach means bugs are caught at compile time, not runtime."

This project prioritizes:
1. **Safety** - Prevent invalid states
2. **Reliability** - 99% success rate through risk reduction
3. **Maintainability** - Clear contracts and pure functions
4. **Performance** - When safety is ensured
5. **Usability** - Builder patterns and fluent APIs

---

**Remember**: KotaDB is designed to be a production-ready database for distributed human-AI cognition. Every design decision prioritizes safety, reliability, and maintainability.