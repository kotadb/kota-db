---
tags:
- file
- kota-db
- ext_md
---
# Branching Strategy & Workflow

## Overview

KotaDB follows a **Git Flow (Simplified)** branching model optimized for open-source development with AI agents.

```
feature/* ──┐
            ├──> develop ──> release/* ──> main
hotfix/*  ──────────────────────────────┘
```

## Branch Types

### 🔐 Protected Branches

#### `main` (Production)
- **Purpose**: Stable, production-ready code only
- **Protected**: Yes (strict)
- **Direct commits**: Forbidden
- **Merge requirements**:
  - PR with 1 approval
  - All CI checks passing (Build, Test, Clippy, Format)
  - Up-to-date with main (strict mode)
  - Conversation resolution required
- **Deploys**: Automatically publishes packages to PyPI/npm

#### `develop` (Integration)
- **Purpose**: Integration branch for completed features
- **Protected**: Yes (relaxed)
- **Direct commits**: Allowed for maintainers
- **Merge requirements**:
  - CI checks passing (Build, Test, Clippy)
  - No review required (but recommended)
- **Deploys**: None (testing only)

### 🚀 Working Branches

#### `feature/*` (Feature Development)
- **Purpose**: Individual feature implementation
- **Naming**: `feature/description-of-feature`
- **Created from**: `develop`
- **Merges to**: `develop`
- **Lifetime**: Delete after merge
- **Example**: `feature/add-vector-search`

#### `release/*` (Release Preparation)
- **Purpose**: Prepare and test releases
- **Naming**: `release/v0.3.0`
- **Created from**: `develop`
- **Merges to**: `main` AND `develop`
- **Lifetime**: Delete after merge
- **Activities**:
  - Version bumping
  - Changelog updates
  - Final testing
  - Documentation updates

#### `hotfix/*` (Emergency Fixes)
- **Purpose**: Critical production fixes
- **Naming**: `hotfix/fix-description`
- **Created from**: `main`
- **Merges to**: `main` AND `develop`
- **Lifetime**: Delete after merge
- **Example**: `hotfix/security-vulnerability`

## Workflow Examples

### Feature Development
```bash
# 1. Create feature branch from develop
git checkout develop
git pull origin develop
git checkout -b feature/my-feature

# 2. Work on feature
git add .
git commit -m "feat: implement my feature"

# 3. Push and create PR
git push -u origin feature/my-feature
gh pr create --base develop --title "feat: my feature"

# 4. After PR approval and merge
git checkout develop
git pull origin develop
git branch -d feature/my-feature
```

### Release Process
```bash
# 1. Create release branch from develop
git checkout develop
git pull origin develop
git checkout -b release/v0.3.0

# 2. Prepare release
just release-preview  # Check what's in the release
# Update VERSION, CHANGELOG.md, etc.
git commit -m "chore: prepare release v0.3.0"

# 3. Create PR to main
gh pr create --base main --title "Release v0.3.0"

# 4. After merge to main, back-merge to develop
git checkout main
git pull origin main
git tag v0.3.0
git push --tags

git checkout develop
git merge main
git push origin develop
```

### Hotfix Process
```bash
# 1. Create hotfix from main
git checkout main
git pull origin main
git checkout -b hotfix/critical-bug

# 2. Fix the issue
git add .
git commit -m "fix: resolve critical bug"

# 3. Create PR to main
gh pr create --base main --title "Hotfix: critical bug"

# 4. After merge, back-merge to develop
git checkout develop
git merge main
git push origin develop
```

## Automation & CI/CD

### Continuous Integration
- **Triggers**: All pushes and PRs to `main`, `develop`, `release/*`, `hotfix/*`
- **Checks**:
  - Build and Test (required)
  - Clippy linting (required)
  - Format check (required for main)
  - Security audit
  - Coverage reporting

### Continuous Deployment
- **Production (main)**:
  - Publishes to PyPI and npm
  - Creates GitHub release
  - Builds Docker images
  - Updates documentation
  
- **Development (develop)**:
  - Runs extended test suite
  - No deployment

## Branch Protection Rules

### Main Branch
```json
{
  "required_status_checks": ["Build and Test", "Clippy", "Format"],
  "require_pr_reviews": true,
  "dismiss_stale_reviews": true,
  "require_conversation_resolution": true,
  "no_force_pushes": true,
  "no_deletions": true
}
```

### Develop Branch
```json
{
  "required_status_checks": ["Build and Test", "Clippy"],
  "require_pr_reviews": false,
  "no_force_pushes": true,
  "no_deletions": true
}
```

## Best Practices

### For AI Agents
1. **Always create feature branches** for new work
2. **Comment on issues** when starting work
3. **Update PR descriptions** with detailed changes
4. **Run `just check`** before pushing
5. **Keep branches up-to-date** with their base branch

### Commit Messages
Follow conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `chore:` Maintenance tasks
- `perf:` Performance improvements

### Pull Request Guidelines
1. **Title**: Use conventional commit format
2. **Description**: Include:
   - What changed and why
   - Testing performed
   - Breaking changes (if any)
   - Related issues
3. **Size**: Keep PRs focused and small
4. **Reviews**: Request reviews from maintainers

## Migration Guide

For existing work on `main`:
```bash
# Ensure main is up-to-date
git checkout main
git pull origin main

# Switch to develop for new work
git checkout develop
git merge main  # If needed

# Create feature branch
git checkout -b feature/your-feature
```

## Quick Reference

| Branch | Creates From | Merges To | Protected | Auto-Deploy |
|--------|-------------|-----------|-----------|-------------|
| main | - | - | ✅ Strict | ✅ PyPI/npm |
| develop | main | main | ✅ Relaxed | ❌ |
| feature/* | develop | develop | ❌ | ❌ |
| release/* | develop | main, develop | ❌ | ❌ |
| hotfix/* | main | main, develop | ❌ | ❌ |

## Troubleshooting

### "Branch is behind main"
```bash
git checkout your-branch
git fetch origin
git rebase origin/main
# Resolve conflicts if any
git push --force-with-lease
```

### "PR checks failing"
```bash
# Run local checks
just check
just test
just fmt
just clippy
```

### "Can't push to protected branch"
Protected branches require PRs. Create a feature branch instead:
```bash
git checkout -b feature/your-changes
git push -u origin feature/your-changes
gh pr create
```