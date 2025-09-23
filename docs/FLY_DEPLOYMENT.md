# Fly.io Deployment Guide for KotaDB SaaS API

> **Migration Status**: Migrated from Railway to Fly.io (Issue #510)
> **Last Updated**: September 2025

## Overview

KotaDB SaaS API is deployed on Fly.io for both staging and production environments. This guide covers deployment procedures, configuration management, troubleshooting, and operational tasks.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Setup](#environment-setup)
3. [Deployment Process](#deployment-process)
4. [Configuration Files](#configuration-files)
5. [Secrets Management](#secrets-management)
6. [CI/CD Pipeline](#cicd-pipeline)
7. [Monitoring & Debugging](#monitoring--debugging)
8. [Troubleshooting](#troubleshooting)
9. [Rollback Procedures](#rollback-procedures)
10. [Migration from Railway](#migration-from-railway)

## Prerequisites

### Required Tools

1. **Fly.io CLI (flyctl)**:
   ```bash
   # macOS
   brew install flyctl
   
   # Linux
   curl -L https://fly.io/install.sh | sh
   
   # Windows
   powershell -Command "iwr https://fly.io/install.ps1 -useb | iex"
   ```

2. **Authentication**:
   ```bash
   flyctl auth login
   ```

3. **Rust Toolchain** (for local testing):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Environment Setup

### Staging Environment
- **App Name**: `kotadb-api-staging`
- **URL**: https://kotadb-api-staging.fly.dev
- **Region**: IAD (Ashburn, Virginia)
- **Config**: `fly.staging.toml`

### Production Environment
- **App Name**: `kotadb-api`
- **URL**: https://kotadb-api.fly.dev
- **Region**: IAD (Ashburn, Virginia)
- **Config**: `fly.toml`

## Deployment Process

### Quick Deploy

Use the provided deployment script for easy deployments:

```bash
# Deploy to staging
./scripts/deploy-fly.sh staging

# Deploy to production (requires confirmation)
./scripts/deploy-fly.sh production
```

### Manual Deployment

#### Staging Deployment
```bash
flyctl deploy \
  --config fly.staging.toml \
  --app kotadb-api-staging \
  --ha=false \
  --strategy immediate
```

#### Production Deployment
```bash
flyctl deploy \
  --config fly.toml \
  --app kotadb-api \
  --ha=true \
  --strategy rolling
```

### First-Time Setup

If deploying for the first time:

1. **Create the app**:
   ```bash
   # Staging
   flyctl apps create kotadb-api-staging --org personal
   
   # Production
   flyctl apps create kotadb-api --org personal
   ```

2. **Create persistent volumes**:
   ```bash
   # Staging (5GB)
   flyctl volumes create kotadb_staging_data \
     --size 5 \
     --app kotadb-api-staging \
     --region iad
   
   # Production (10GB)
   flyctl volumes create kotadb_data \
     --size 10 \
     --app kotadb-api \
     --region iad
   ```

3. **Set required secrets** (see [Secrets Management](#secrets-management))

4. **Deploy the application**

## Configuration Files

### fly.toml (Production)
Main configuration for production deployment:
- High availability enabled
- Rolling deployment strategy
- 512MB RAM, 1 shared CPU
- Health checks every 30s
- Auto-rollback enabled

### fly.staging.toml (Staging)
Configuration for staging environment:
- Single instance (no HA)
- Immediate deployment strategy
- 256MB RAM, 1 shared CPU
- Debug endpoints enabled
- More verbose logging

### Key Configuration Options

```toml
# Deployment strategy
[deploy]
  strategy = "rolling"        # or "immediate" for staging
  max_unavailable = 0.33      # Max 33% unavailable during deploy
  wait_timeout = "10m"        # Max deployment time

# Health checks
[[services.http_checks]]
  interval = "30s"
  timeout = "10s"
  grace_period = "5s"
  method = "GET"
  path = "/health"

# Scaling
[[vm]]
  cpu_kind = "shared"         # or "dedicated" for production
  cpus = 1
  memory_mb = 512
```

## Secrets Management

### Architecture Note: Supabase Integration

KotaDB uses **Supabase for all persistent data storage**:
- **API Keys**: Stored and managed in Supabase
- **Documents**: All content stored in Supabase
- **User Data**: Managed by Supabase Auth
- **Usage Metrics**: Tracked in Supabase

The Fly.io deployment is **stateless** and only processes requests. See `docs/SUPABASE_ARCHITECTURE.md` for detailed architecture.

## Supabase Migrations

- Generate new SQL from local changes with `just supabase-generate <short_name>`; this wraps `supabase db diff` and writes into `supabase/migrations/`.
- Rebuild the local Supabase containers and apply migrations via `just supabase-reset` before sending a PR. This helper only touches the Dockerised dev stack—it never talks to staging or production.
- Apply the migrations to a remote database with `just supabase-apply <postgres_url>`; in CI the URL is supplied through secrets (see deployment workflow). The helper delegates to `supabase db push`, so the official migration tracking table (`supabase_migrations.schema_migrations`) stays fully compatible with the Supabase CLI.

### Using the Secrets Script

```bash
# Set secrets for staging
./scripts/fly-secrets.sh staging set

# List current secrets
./scripts/fly-secrets.sh production list

# Remove a secret
./scripts/fly-secrets.sh staging unset API_KEY
```

### Manual Secret Management

```bash
# Set Supabase connection (most important)
flyctl secrets set \
  DATABASE_URL="postgresql://postgres.[PROJECT_REF]:[PASSWORD]@aws-0-[REGION].pooler.supabase.com:6543/postgres" \
  --app kotadb-api

# Set additional Supabase credentials
flyctl secrets set \
  SUPABASE_URL="https://[PROJECT_REF].supabase.co" \
  SUPABASE_ANON_KEY="[YOUR_ANON_KEY]" \
  SUPABASE_SERVICE_KEY="[YOUR_SERVICE_KEY]" \
  --app kotadb-api

# List secrets (shows only names, not values)
flyctl secrets list --app kotadb-api

# Remove a secret
flyctl secrets unset API_KEY --app kotadb-api
```

### Required Secrets

| Secret | Description | Required | Example |
|--------|-------------|----------|---------|
| DATABASE_URL | Supabase PostgreSQL connection (pooler endpoint) | Yes | `postgresql://postgres.[ref]:[pass]@aws-0-region.pooler.supabase.com:6543/postgres` |
| SUPABASE_URL | Supabase project URL | Yes | `https://[ref].supabase.co` |
| SUPABASE_ANON_KEY | Public anonymous key | Yes | Your project's anon key |
| SUPABASE_SERVICE_KEY | Service role key (admin) | Yes | Your project's service key |
| SUPABASE_DB_URL_STAGING | Direct Postgres URL for staging | Yes | Passed to deploy workflow for migrations |
| SUPABASE_DB_URL_PRODUCTION | Direct Postgres URL for production | Yes | Only used in manual production deploys |
| JWT_SECRET | Secret for JWT token validation | No | Auto-handled by Supabase |
| REDIS_URL | Redis connection for caching | No | `redis://host:6379` |
| SENTRY_DSN | Error tracking with Sentry | No | Sentry project DSN |

## CI/CD Pipeline

### GitHub Actions Workflow

The deployment is automated via GitHub Actions (`.github/workflows/saas-api-deploy.yml`):

1. **Triggers**:
   - Push to `develop` → Deploy to staging
   - Push to `main` → Deploy to production
   - Manual workflow dispatch

2. **Deployment Flow**:
   ```
   Tests → Build → Deploy → Health Check → Smoke Tests
   ```

3. **Required GitHub Secrets**:
   - `FLY_API_TOKEN`: Fly.io authentication token
   - `SUPABASE_DB_URL_STAGING`: direct connection string used during staging deploys
   - `SUPABASE_DB_URL_PRODUCTION`: direct connection string used during production deploys
   
   Get your token:
   ```bash
   flyctl auth token
   ```
   
   Add to GitHub:
   ```bash
   gh secret set FLY_API_TOKEN --body "YOUR_TOKEN_HERE"
   ```

### Manual CI/CD Trigger

```bash
# Trigger deployment manually
gh workflow run saas-api-deploy.yml \
  --field environment=staging

# Check workflow status
gh run list --workflow=saas-api-deploy.yml
```

## Monitoring & Debugging

### View Logs

```bash
# Real-time logs
flyctl logs --app kotadb-api

# Last 100 lines
flyctl logs --app kotadb-api -n 100

# Filter by instance
flyctl logs --app kotadb-api --instance=abcd1234
```

### SSH Access

```bash
# Connect to running instance
flyctl ssh console --app kotadb-api

# Run commands in the container
flyctl ssh console --app kotadb-api --command "ls -la /data"
```

### Application Status

```bash
# Overall status
flyctl status --app kotadb-api

# Detailed instance info
flyctl status --app kotadb-api --verbose

# List all instances
flyctl scale show --app kotadb-api
```

### Metrics

```bash
# Open Fly.io dashboard
flyctl dashboard --app kotadb-api

# View metrics in terminal
flyctl monitor --app kotadb-api
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Container Restart Loops
**Symptom**: App keeps restarting
**Solution**:
```bash
# Check logs for errors
flyctl logs --app kotadb-api -n 200

# Verify secrets are set
flyctl secrets list --app kotadb-api

# Check health endpoint locally
curl https://kotadb-api.fly.dev/health
```

#### 2. Database Connection Issues
**Symptom**: "DATABASE_URL is not set" or connection timeouts
**Solution**:
```bash
# Verify DATABASE_URL is set
flyctl secrets list --app kotadb-api | grep DATABASE_URL

# Test connection from container
flyctl ssh console --app kotadb-api
> apt-get update && apt-get install -y postgresql-client
> psql $DATABASE_URL -c "SELECT 1"
```

#### 3. Deployment Failures
**Symptom**: Deploy command fails
**Solution**:
```bash
# Check build logs
flyctl deploy --verbose

# Try with local Docker build
flyctl deploy --local-only

# Clear builder cache
flyctl deploy --no-cache
```

#### 4. Out of Memory
**Symptom**: App crashes with OOM errors
**Solution**:
```bash
# Scale up memory
flyctl scale memory 1024 --app kotadb-api

# Check current usage
flyctl scale show --app kotadb-api
```

### Debug Commands

```bash
# Get detailed app info
flyctl info --app kotadb-api

# List releases
flyctl releases list --app kotadb-api

# Check certificates
flyctl certs list --app kotadb-api

# View current configuration
flyctl config show --app kotadb-api
```

## Rollback Procedures

### Automatic Rollback

Fly.io automatically rolls back if health checks fail during deployment.

### Manual Rollback

```bash
# List recent releases
flyctl releases list --app kotadb-api

# Rollback to specific version
flyctl deploy --image registry.fly.io/kotadb-api:deployment-01J6ABCD

# Or use the GitHub Actions workflow
gh workflow run saas-api-deploy.yml \
  --field environment=production \
  --field action=rollback
```

## Migration from Railway

### What Changed

1. **Configuration Format**: 
   - Railway: `railway.toml`
   - Fly.io: `fly.toml` and `fly.staging.toml`

2. **Deployment Command**:
   - Railway: `railway up`
   - Fly.io: `flyctl deploy`

3. **Environment Variables**:
   - Railway: Set in dashboard
   - Fly.io: Set via `flyctl secrets`

4. **Persistent Storage**:
   - Railway: Automatic
   - Fly.io: Explicit volume mounts

5. **Health Checks**:
   - Railway: Basic HTTP checks
   - Fly.io: Comprehensive TCP and HTTP checks

### Benefits of Fly.io

- ✅ Better debugging with SSH access
- ✅ Clear error messages during deployment
- ✅ Native Docker support
- ✅ CLI-first approach
- ✅ Better GLIBC compatibility
- ✅ Predictable container behavior
- ✅ Superior monitoring and metrics

## Best Practices

1. **Always test in staging first**
2. **Monitor logs during deployment**
3. **Keep secrets in environment-specific files**
4. **Use health checks to validate deployments**
5. **Document any manual changes in GitHub issues**
6. **Use the provided scripts for consistency**
7. **Tag releases in git after successful production deployments**

## Support and Resources

- [Fly.io Documentation](https://fly.io/docs/)
- [Fly.io Status Page](https://status.fly.io/)
- [KotaDB GitHub Issues](https://github.com/jayminwest/kota-db/issues)
- [Deployment Script](./scripts/deploy-fly.sh)
- [Secrets Management Script](./scripts/fly-secrets.sh)

## Emergency Contacts

For critical production issues:
1. Check Fly.io status page
2. Review recent deployments in GitHub Actions
3. Create high-priority GitHub issue with `production-blocker` label
4. Use `flyctl ssh console` for immediate debugging
