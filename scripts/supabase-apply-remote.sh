#!/bin/bash
# Apply migrations to a remote Supabase Postgres instance.
# Usage: SUPABASE_DB_URL=postgres://... ./scripts/supabase-apply-remote.sh
#    or: ./scripts/supabase-apply-remote.sh postgres://...

set -euo pipefail

DB_URL=${SUPABASE_DB_URL:-${1:-}}
if [ -z "$DB_URL" ]; then
  echo "Missing SUPABASE_DB_URL" >&2
  echo "Usage: SUPABASE_DB_URL=postgres://... $0" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql client is not available on PATH" >&2
  exit 1
fi

REDACTED_URL=$(echo "$DB_URL" | sed -E 's#(postgres://)([^:@]+)(:[^@]*)?@#\1****@#')
echo "Applying Supabase migrations via psql -> $REDACTED_URL"

# Ensure metadata table exists for tracking applied migrations
psql "$DB_URL" -v ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA IF NOT EXISTS supabase_migrations;
CREATE TABLE IF NOT EXISTS supabase_migrations.schema_migrations (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
SQL

shopt -s nullglob
MIGRATIONS=(supabase/migrations/*.sql)
if [ ${#MIGRATIONS[@]} -eq 0 ]; then
  echo "No migrations found under supabase/migrations"
else
  APPLIED=$(psql "$DB_URL" -Atqc "SELECT name FROM supabase_migrations.schema_migrations")
  for migration in "${MIGRATIONS[@]}"; do
    name=$(basename "$migration")
    if echo "$APPLIED" | grep -Fxq "$name"; then
      echo "Skipping already applied migration: $name"
      continue
    fi

    echo "Running migration: $migration"
    psql "$DB_URL" -v ON_ERROR_STOP=1 -f "$migration"
    psql "$DB_URL" -v ON_ERROR_STOP=1 -c "INSERT INTO supabase_migrations.schema_migrations (name) VALUES ('$name') ON CONFLICT DO NOTHING;"
  done
fi

if [ -f supabase/seed.sql ]; then
  echo "Applying seed data from supabase/seed.sql"
  psql "$DB_URL" -v ON_ERROR_STOP=1 -f supabase/seed.sql
fi
