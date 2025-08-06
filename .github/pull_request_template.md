# Pull Request

## Summary
<!-- Provide a brief description of the changes in this PR -->

## Type of Change
<!-- Mark the type of change with an `x` -->
- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📚 Documentation update
- [ ] 🧪 Test improvements
- [ ] ⚡ Performance improvements
- [ ] 🔧 Refactoring (no functional changes)
- [ ] 🚀 MCP server integration
- [ ] 🏗️ CI/CD improvements

## Related Issues
<!-- Link to related issues using "Fixes #123" or "Relates to #123" -->
- Fixes #
- Relates to #

## Changes Made
<!-- Describe the changes made in this PR -->
- 
- 
- 

## Testing
<!-- Describe the testing that has been done -->
### Test Coverage
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Property-based tests added/updated (if applicable)
- [ ] Performance tests added/updated (if applicable)
- [ ] Manual testing performed

### Test Results
- [ ] All existing tests pass
- [ ] New tests pass
- [ ] No performance regressions
- [ ] Memory usage within acceptable limits

### Testing Commands Run
```bash
# List the commands you ran to test your changes
cargo test --all
cargo clippy --all-targets --all-features
cargo fmt --all -- --check
./run_standalone.sh test
./run_standalone.sh demo
```

## Performance Impact
<!-- Assess the performance impact of your changes -->
- [ ] No performance impact
- [ ] Performance improvement (describe below)
- [ ] Performance regression (justified below)
- [ ] Performance impact unknown (needs benchmarking)

**Performance Details:**
<!-- If there's a performance impact, describe it here -->

## MCP Integration Impact
<!-- If this affects MCP server integration -->
- [ ] No impact on MCP server
- [ ] Improves MCP server functionality
- [ ] Adds new MCP tools/resources
- [ ] Changes MCP API (breaking/non-breaking)
- [ ] Requires MCP client updates

## Documentation
<!-- Check all that apply -->
- [ ] Code is self-documenting
- [ ] Inline comments added for complex logic
- [ ] API documentation updated (rustdoc)
- [ ] User documentation updated
- [ ] Examples added/updated
- [ ] CHANGELOG.md updated (for releases)

## Checklist
<!-- Ensure all items are checked before requesting review -->
### Code Quality
- [ ] Code follows the project's style guidelines
- [ ] Self-review of code completed
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] No new clippy warnings (`cargo clippy`)
- [ ] No new compiler warnings
- [ ] Error handling is comprehensive
- [ ] No `unwrap()` calls in production code (or justified)

### Risk Assessment
- [ ] Changes follow the 6-stage risk reduction methodology
- [ ] Contracts and validation maintained
- [ ] Pure functions preserved where applicable
- [ ] Observability/tracing maintained
- [ ] Component library patterns followed

### Deployment
- [ ] Changes are backward compatible
- [ ] Database migration scripts provided (if needed)
- [ ] Configuration changes documented
- [ ] Docker image builds successfully
- [ ] CI/CD pipeline passes

## Reviewer Notes
<!-- Any specific areas you'd like reviewers to focus on -->

## Screenshots (if applicable)
<!-- Add screenshots for UI changes or visual improvements -->

---

## For Maintainers
<!-- This section is for maintainer use -->
### Release Notes Category
- [ ] Breaking Changes
- [ ] New Features
- [ ] Bug Fixes
- [ ] Performance Improvements
- [ ] Documentation
- [ ] Internal/DevX

### Merge Strategy
- [ ] Squash and merge (default)
- [ ] Merge commit (preserve history)
- [ ] Rebase and merge (clean history)
