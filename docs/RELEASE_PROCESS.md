# KotaDB Release Process

This document outlines the release process for KotaDB, including versioning strategy, release procedures, and post-release tasks.

## Versioning Strategy

KotaDB follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** version (X.0.0): Incompatible API changes
- **MINOR** version (0.X.0): Backwards-compatible functionality additions
- **PATCH** version (0.0.X): Backwards-compatible bug fixes
- **PRERELEASE** versions: Alpha, beta, and release candidates (e.g., 1.0.0-beta.1)

## Quick Release Commands

```bash
# Check current version
just version

# Preview what would be in the next release
just release-preview

# Create releases with automatic version bump
just release-patch   # Bump patch version (0.1.0 -> 0.1.1)
just release-minor   # Bump minor version (0.1.0 -> 0.2.0)
just release-major   # Bump major version (0.1.0 -> 1.0.0)
just release-beta    # Create beta release (0.1.0 -> 0.1.0-beta.1)

# Create release with specific version
just release 0.2.0

# Dry run to test the process
just release-dry-run 0.2.0
```

## Release Checklist

### Pre-Release

- [ ] Ensure all PRs for the release are merged
- [ ] Update dependencies: `cargo update`
- [ ] Run security audit: `cargo audit`
- [ ] Update CHANGELOG.md with all changes
- [ ] Review and update documentation
- [ ] Test all client libraries (Python, TypeScript, Go, Rust)
- [ ] Run full test suite: `just ci`
- [ ] Verify Docker build: `just docker-build`

### Release Process

1. **Start the release**
   ```bash
   # For a specific version
   just release 0.2.0
   
   # Or with automatic version bump
   just release-minor
   ```

2. **The script will automatically:**
   - Verify clean working directory
   - Run all tests and quality checks
   - Update version in:
     - Cargo.toml
     - VERSION file
     - CHANGELOG.md
     - Client library versions
   - Commit changes
   - Create annotated git tag
   - Push to remote (with confirmation)

3. **GitHub Actions will then:**
   - Create GitHub Release with changelog
   - Build binaries for all platforms:
     - Linux x64 (glibc and musl)
     - macOS x64 and ARM64
     - Windows x64
   - Publish Docker images to GitHub Container Registry
   - Publish to crates.io (for non-prerelease versions)

### Post-Release

- [ ] Verify GitHub Release page
- [ ] Check binary downloads work
- [ ] Verify Docker images: `docker pull ghcr.io/jayminwest/kota-db:latest`
- [ ] Test crates.io package: `cargo install kotadb`
- [ ] Update documentation site (see Documentation Deployment section below)
- [ ] Announce release:
  - [ ] GitHub Discussions
  - [ ] Project Discord/Slack
  - [ ] Social media
- [ ] Create issues for next release cycle
- [ ] Update changelog with new Unreleased section: `just changelog-update`

## Manual Release Process

If the automated process fails, follow these manual steps:

1. **Update versions manually:**
   ```bash
   # Edit Cargo.toml
   vim Cargo.toml  # Update version = "X.Y.Z"
   
   # Update VERSION file
   echo "X.Y.Z" > VERSION
   
   # Update Cargo.lock
   cargo update --workspace
   ```

2. **Update CHANGELOG.md:**
   - Change `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`
   - Add new `## [Unreleased]` section at top
   - Update links at bottom

3. **Commit changes:**
   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md VERSION
   git commit -m "chore: release vX.Y.Z"
   ```

4. **Create and push tag:**
   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin main
   git push origin vX.Y.Z
   ```

## Rollback Procedure

If a release needs to be rolled back:

1. **Delete the tag locally and remotely:**
   ```bash
   git tag -d vX.Y.Z
   git push origin :refs/tags/vX.Y.Z
   ```

2. **Delete the GitHub Release:**
   - Go to GitHub Releases page
   - Click on the release
   - Click "Delete this release"

3. **Revert version changes if needed:**
   ```bash
   git revert <commit-hash>
   git push origin main
   ```

## Release Naming Conventions

- Production releases: `vX.Y.Z` (e.g., v1.0.0)
- Beta releases: `vX.Y.Z-beta.N` (e.g., v1.0.0-beta.1)
- Alpha releases: `vX.Y.Z-alpha.N` (e.g., v1.0.0-alpha.1)
- Release candidates: `vX.Y.Z-rc.N` (e.g., v1.0.0-rc.1)

## Documentation Deployment

KotaDB uses [Mike](https://github.com/jimporter/mike) for versioned documentation on GitHub Pages. Documentation is built with MkDocs and deployed to the `gh-pages` branch.

### Prerequisites

```bash
# Install required tools
pip install mkdocs mkdocs-material mike
```

### Deployment Process

1. **Deploy a new version:**
   ```bash
   # Deploy specific version
   mike deploy 0.2.0 --push
   
   # Deploy with alias (e.g., latest)
   mike deploy 0.2.0 latest --push
   
   # Deploy as stable (recommended for production releases)
   mike deploy 0.2.0 stable --push
   ```

2. **Set default version:**
   ```bash
   # Make a version the default when users visit the root URL
   mike set-default stable --push
   ```

3. **List deployed versions:**
   ```bash
   mike list
   ```

4. **Delete a version:**
   ```bash
   mike delete 0.1.0 --push
   ```

### Best Practices

1. **Version Naming:**
   - Use semantic version numbers (e.g., `0.2.0`, `1.0.0`)
   - Use `stable` alias for the current stable release
   - Use `latest` alias for the most recent release (including betas)
   - Use `dev` for development/unreleased documentation

2. **Release Documentation Updates:**
   ```bash
   # When releasing a new stable version
   mike deploy <version> stable --push --update-aliases
   
   # For beta/prerelease versions
   mike deploy <version>-beta.1 --push
   ```

3. **Local Testing:**
   ```bash
   # Build and serve documentation locally
   mkdocs serve
   
   # Test Mike deployment locally (without pushing)
   mike deploy <version> --no-push
   mike serve  # View the versioned site locally
   ```

### Structure

The `gh-pages` branch should maintain this structure:
```
gh-pages/
├── index.html          # Redirect to default version
├── versions.json       # Mike version metadata
├── stable/            # Stable version (alias)
│   └── [docs]
├── 0.2.0/             # Specific version
│   └── [docs]
└── site/              # Legacy structure (can be removed)
```

### Troubleshooting

1. **Documentation not updating:**
   ```bash
   # Force push to update
   mike deploy <version> --push --force
   ```

2. **Broken redirect:**
   - Ensure `index.html` at root redirects to correct version
   - Check with: `mike set-default stable --push`

3. **Version selector not working:**
   - Verify `versions.json` exists in gh-pages root
   - Check multiple versions are deployed: `mike list`

### GitHub Pages Protection

To prevent accidental commits to the `gh-pages` branch:
1. Use branch protection rules in GitHub settings
2. Always use Mike for deployments (never commit directly)
3. Use the GitHub Action workflow for automated deployments

## Platform-Specific Notes

### Docker Images

Docker images are automatically built and pushed to GitHub Container Registry:
- Latest stable: `ghcr.io/jayminwest/kota-db:latest`
- Specific version: `ghcr.io/jayminwest/kota-db:0.2.0`
- Major version: `ghcr.io/jayminwest/kota-db:0`
- Major.Minor: `ghcr.io/jayminwest/kota-db:0.2`

### Crates.io

Publishing to crates.io requires:
- `CRATES_IO_TOKEN` secret configured in GitHub
- Non-prerelease version (no alpha/beta/rc)
- All dependencies must be published on crates.io

### Binary Artifacts

Binaries are built for:
- `x86_64-unknown-linux-gnu`: Standard Linux (Ubuntu, Debian, etc.)
- `x86_64-unknown-linux-musl`: Alpine Linux and static linking
- `x86_64-apple-darwin`: macOS Intel
- `aarch64-apple-darwin`: macOS Apple Silicon
- `x86_64-pc-windows-msvc`: Windows 64-bit

## Troubleshooting

### Release workflow fails

1. Check GitHub Actions logs for specific error
2. Common issues:
   - Missing `CRATES_IO_TOKEN` secret
   - Version already exists on crates.io
   - Tests failing on specific platform
   - Docker build issues

### Tag already exists

```bash
# Delete local tag
git tag -d vX.Y.Z

# Delete remote tag
git push origin :refs/tags/vX.Y.Z

# Recreate tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

### Version mismatch

Ensure all version references are updated:
```bash
grep -r "0\.1\.0" --include="*.toml" --include="*.json" --include="*.go"
```

## Security Considerations

- Never commit sensitive data in releases
- Run `cargo audit` before each release
- Review dependencies for known vulnerabilities
- Sign releases with GPG when possible:
  ```bash
  git tag -s vX.Y.Z -m "Release vX.Y.Z"
  ```

## Contact

For release-related questions or issues:
- Create an issue on GitHub
- Contact the maintainers
- Check the release documentation in `/docs`