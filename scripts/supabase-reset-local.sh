#!/bin/bash
# Reset the local Supabase stack and apply migrations + seed

set -euo pipefail

if ! command -v supabase >/dev/null 2>&1; then
  echo "supabase CLI is not installed" >&2
  exit 1
fi

echo "Resetting local Supabase containers with project migrations"
supabase db reset --local
