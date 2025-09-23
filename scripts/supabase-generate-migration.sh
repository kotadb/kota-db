#!/bin/bash
# Generate a Supabase migration using the local dev stack
# Usage: ./scripts/supabase-generate-migration.sh descriptive_name

set -euo pipefail

if ! command -v supabase >/dev/null 2>&1; then
  echo "supabase CLI is not installed" >&2
  echo "Install from https://supabase.com/docs/guides/cli" >&2
  exit 1
fi

if [ $# -ne 1 ]; then
  echo "Usage: $0 descriptive_name" >&2
  exit 1
fi

NAME=$1
TIMESTAMP=$(date -u +%Y%m%d%H%M%S)
FILE="supabase/migrations/${TIMESTAMP}_${NAME}.sql"

supabase db diff --use-migra -f "$FILE"

echo "Generated migration: $FILE"
