# Documentation Versioning

This document explains how versioned documentation works for KotaDB.

## Overview

KotaDB uses [Mike](https://github.com/jimporter/mike) to manage versioned documentation with MkDocs. This allows users to view documentation for specific versions of KotaDB while keeping development docs separate.

## Version Structure

Documentation is organized as follows:

- **`latest`** - Points to the most recent stable release
- **`stable`** - Alias for the latest stable version
- **`dev`** - Development documentation from the main branch
- **`X.Y.Z`** - Specific version documentation (e.g., `0.2.0`, `0.3.0`)

## Automatic Deployment

### On Release

When a new version is tagged and released:

1. GitHub Actions triggers the release workflow
2. Documentation is built for that specific version
3. Mike deploys the versioned docs to GitHub Pages
4. Version aliases are updated (`latest`, `stable` for non-prerelease)

### On Main Branch Push

When changes are pushed to the main branch:

1. Documentation is built from the current state
2. Mike deploys it as the `dev` version
3. Users can preview upcoming documentation changes

## Manual Deployment

### Deploy a Specific Version

```bash
# Via GitHub Actions (recommended)
gh workflow run "Deploy Versioned Documentation" \
  --field version=0.2.1 \
  --field alias=stable

# Locally (requires gh-pages access)
mike deploy --push --update-aliases 0.2.1 latest
```

### Initialize Documentation Locally

```bash
# Run the initialization script
./scripts/init-mike-docs.sh

# Or manually
pip install mike mkdocs-material
mike deploy --update-aliases $(cat VERSION) latest
mike serve
```

## Version Selector

The Material for MkDocs theme provides a built-in version selector that:

- Shows all available versions
- Indicates the current version
- Allows switching between versions
- Preserves the current page when switching (when possible)

## Configuration

### mkdocs.yml

```yaml
extra:
  version:
    provider: mike
    default: latest
    alias: true
```

### GitHub Actions

Three workflows handle documentation:

1. **`.github/workflows/docs.yml`** - Deploys dev docs on main branch push
2. **`.github/workflows/docs-versioned.yml`** - Manual versioned deployment
3. **`.github/workflows/release.yml`** - Includes docs deployment on release

## Viewing Documentation

- **Latest stable**: https://jayminwest.github.io/kota-db/
- **Specific version**: https://jayminwest.github.io/kota-db/0.2.0/
- **Development**: https://jayminwest.github.io/kota-db/dev/

## Local Development

### Serve Documentation Locally

```bash
# Serve with live reload
mkdocs serve

# Serve with Mike (includes version selector)
mike serve
```

### Build Documentation

```bash
# Build static site
mkdocs build

# Build and deploy with Mike
mike deploy 0.2.1-dev
```

## Troubleshooting

### Missing Version Selector

If the version selector doesn't appear:

1. Ensure Mike is installed: `pip install mike`
2. Check that gh-pages branch exists
3. Verify `extra.version.provider: mike` in mkdocs.yml

### Deployment Fails

If deployment fails:

1. Check GitHub Actions permissions
2. Ensure gh-pages branch is not protected
3. Verify Mike configuration in mkdocs.yml

### Wrong Default Version

To fix the default version:

```bash
# Set a specific version as default
mike set-default --push latest

# Or specify exact version
mike set-default --push 0.2.0
```

## Best Practices

1. **Always tag releases** - Use semantic versioning (e.g., v0.2.0)
2. **Update CHANGELOG.md** - Document changes for each version
3. **Test locally** - Use `mike serve` before deploying
4. **Keep dev separate** - Development docs should reflect main branch
5. **Use aliases** - Maintain `latest` and `stable` for user convenience

## Related Files

- `mkdocs.yml` - Main MkDocs configuration
- `.github/workflows/docs.yml` - Development documentation workflow
- `.github/workflows/docs-versioned.yml` - Versioned deployment workflow
- `.github/workflows/release.yml` - Release workflow with docs deployment
- `scripts/init-mike-docs.sh` - Local initialization script