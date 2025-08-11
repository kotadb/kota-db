# Contributing to KotaDB

Thank you for your interest in contributing to KotaDB! This project is developed 100% by LLM coding tools, with agents communicating through GitHub issues, PRs, and commits.

## Quick Links

- [Full Contributing Guide](https://github.com/jayminwest/kota-db/blob/main/CONTRIBUTING.md) - Detailed contribution guidelines
- [Agent Guide](https://github.com/jayminwest/kota-db/blob/main/AGENT.md) - Essential guide for LLM agents
- [Developer Guide](developer/index.md) - Technical development documentation

## How to Contribute

### For Human Contributors

1. **Report Issues**: [Create an issue](https://github.com/jayminwest/kota-db/issues/new) for bugs or feature requests
2. **Submit PRs**: Fork the repo, create a branch, and submit a pull request
3. **Improve Docs**: Help improve documentation or add examples
4. **Test & Review**: Test new features and review pull requests

### For LLM Agents

Follow the [Agent Communication Protocol](https://github.com/jayminwest/kota-db/blob/main/AGENT.md):

```bash
# Always comment on issues when working
gh issue comment <issue-number> --body "Starting work on this issue..."

# Create detailed PR descriptions
gh pr create --title "feat: Add feature X" --body "Detailed description..."

# Document all changes
gh api repos/:owner/:repo/commits/<sha>/comments --method POST --field body="..."
```

## Development Workflow

### 1. Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/jayminwest/kota-db.git
cd kota-db

# Install development tools
cargo install just
cargo install cargo-watch

# Run development server
just dev
```

### 2. Before Making Changes

```bash
# Create a new branch
git checkout -b feature/your-feature-name

# Run tests to ensure clean state
just test

# Check code quality
just check
```

### 3. Make Your Changes

Follow the [6-Stage Risk Reduction Methodology](architecture/risk-reduction.md):
1. Write tests first (TDD)
2. Define contracts
3. Implement pure functions
4. Add observability
5. Include adversarial tests
6. Use the component library

### 4. Test Your Changes

```bash
# Run all tests
just test

# Run specific test
cargo test test_name

# Run with coverage
just coverage

# Performance tests
just test-perf
```

### 5. Code Quality Checks

```bash
# Format code
just fmt

# Run clippy (must pass with no warnings)
just clippy

# Security audit
just audit

# Run all checks
just ci
```

### 6. Submit Your Changes

```bash
# Commit with conventional commit message
git commit -m "feat(component): add new feature"

# Push to your fork
git push origin feature/your-feature-name

# Create PR via GitHub CLI
gh pr create --title "feat: Add feature" --body "Description..."
```

## Code Style Guidelines

### Rust Conventions

- Use descriptive names
- Prefer immutability
- Use the type system for safety
- Handle all errors explicitly
- Add comprehensive documentation

### Example Code Style

```rust
/// Validates and creates a new document path
/// 
/// # Arguments
/// * `path` - The path to validate
/// 
/// # Returns
/// * `Result<ValidatedPath>` - The validated path or error
/// 
/// # Example
/// ```
/// let path = validate_document_path("/docs/example.md")?;
/// ```
pub fn validate_document_path(path: &str) -> Result<ValidatedPath> {
    // Implementation with proper error handling
}
```

## Testing Requirements

### Test Coverage Goals
- Unit tests: >90% coverage
- Integration tests: All major workflows
- Property tests: Core algorithms
- Performance tests: Sub-10ms latency

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_feature() -> Result<()> {
        // Arrange
        let storage = create_test_storage().await?;
        
        // Act
        let result = storage.operation().await?;
        
        // Assert
        assert_eq!(result, expected);
        Ok(())
    }
}
```

## Documentation

### Code Documentation
- Document all public APIs
- Include examples in doc comments
- Explain complex algorithms
- Add architecture decision records

### Documentation Types
- **API Docs**: Generated from code comments
- **User Guides**: In the docs/ directory
- **Examples**: Working code in examples/
- **Architecture**: Design documents in docs/

## Getting Help

- **GitHub Issues**: Search existing issues or create new ones
- **Discussions**: Ask questions in GitHub Discussions
- **Documentation**: Read the comprehensive docs
- **Examples**: Check the examples directory

## Recognition

Contributors are recognized in:
- GitHub contributors page
- Release notes
- Documentation credits

Thank you for contributing to KotaDB! 🚀