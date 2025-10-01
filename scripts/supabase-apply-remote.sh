#!/bin/bash
# Apply migrations to a remote Supabase Postgres instance using the Supabase CLI.
# Usage: SUPABASE_DB_URL=postgres://... ./scripts/supabase-apply-remote.sh
#    or: ./scripts/supabase-apply-remote.sh postgres://...

set -euo pipefail

DB_URL=${SUPABASE_DB_URL:-${1:-}}
if [ -z "$DB_URL" ]; then
  echo "Missing SUPABASE_DB_URL" >&2
  echo "Usage: SUPABASE_DB_URL=postgres://... $0" >&2
  exit 1
fi

if ! command -v supabase >/dev/null 2>&1; then
  echo "Supabase CLI is not available on PATH" >&2
  echo "Install it via https://supabase.com/docs/guides/cli" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql client is not available on PATH" >&2
  exit 1
fi

REDACTED_URL=$(echo "$DB_URL" | sed -E 's#(postgres://)([^:@]+)(:[^@]*)?@#\1****@#')
echo "Applying Supabase migrations via supabase db push -> $REDACTED_URL"

HAS_VERSION=$(psql "$DB_URL" -Atqc "SELECT 1 FROM information_schema.columns WHERE table_schema = 'supabase_migrations' AND table_name = 'schema_migrations' AND column_name = 'version' LIMIT 1" || true)
HAS_ID=$(psql "$DB_URL" -Atqc "SELECT 1 FROM information_schema.columns WHERE table_schema = 'supabase_migrations' AND table_name = 'schema_migrations' AND column_name = 'id' LIMIT 1" || true)

if [ "$HAS_VERSION" != "1" ]; then
  echo "Reinitialising supabase_migrations.schema_migrations to Supabase CLI schema"
  psql "$DB_URL" <<'SQL'
CREATE SCHEMA IF NOT EXISTS supabase_migrations;
DROP TABLE IF EXISTS supabase_migrations.schema_migrations;
DROP TABLE IF EXISTS supabase_migrations.seed_files;
SQL
elif [ "$HAS_ID" = "1" ]; then
  echo "Cleaning legacy rows in supabase_migrations.schema_migrations"
  psql "$DB_URL" -c "DELETE FROM supabase_migrations.schema_migrations WHERE version IS NULL"
fi

export SUPABASE_DB_PUSH_USE_MIGRA="${SUPABASE_DB_PUSH_USE_MIGRA:-false}"

supabase db push --db-url "$DB_URL"

if [ -s supabase/seed.sql ]; then
  echo "Applying seed data from supabase/seed.sql"
  psql "$DB_URL" -v ON_ERROR_STOP=1 -f supabase/seed.sql
fi
