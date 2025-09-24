#!/usr/bin/env bash

set -euo pipefail

DEFAULT_URL="https://kotadb-api-staging.fly.dev"
MCP_CHECK=0

usage() {
  cat <<'HELP'
KotaDB SaaS smoke test

Checks the public /health endpoint for Supabase connectivity and optionally
hits the authenticated repository listing when an API key is supplied.

Usage:
  scripts/saas_smoke.sh [-u https://your-app.fly.dev] [-k api_key] [--mcp]

Env vars:
  KOTADB_SAAS_URL     Base URL when -u is not provided (default staging URL)
  KOTADB_SAAS_API_KEY API key to test authenticated endpoints (or pass -k)
HELP
}

BASE_URL="${KOTADB_SAAS_URL:-$DEFAULT_URL}"
API_KEY="${KOTADB_SAAS_API_KEY:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -u)
      BASE_URL="$2"
      shift 2
      ;;
    -k)
      API_KEY="$2"
      shift 2
      ;;
    --mcp)
      MCP_CHECK=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

echo "ℹ️  Checking $BASE_URL/health"
HEALTH_JSON=$(curl -fsSL "$BASE_URL/health")

python3 - "$HEALTH_JSON" <<'PY'
import json
import sys

raw = sys.argv[1]
try:
    data = json.loads(raw)
except json.JSONDecodeError as exc:
    print(f"Health endpoint returned invalid JSON: {exc}", file=sys.stderr)
    sys.exit(1)

saas = data.get("saas") or {}
status = saas.get("supabase_status", "unknown")
latency = saas.get("supabase_latency_ms")
job_queue = saas.get("job_queue") or {}
queued = job_queue.get("queued") or 0
failed_recent = job_queue.get("failed_recent") or 0
oldest_age = job_queue.get("oldest_queued_seconds")

print(f"Supabase status: {status}")
if latency is not None:
    print(f"Supabase latency: {latency} ms")
print(f"Queued jobs: {queued}")
print(f"Failed jobs (last hour): {failed_recent}")
if oldest_age is not None:
    print(f"Oldest queued job age: {oldest_age} seconds")

if status != "ok":
    print("Supabase connectivity check failed", file=sys.stderr)
    sys.exit(1)

if failed_recent:
    print("Recent Supabase jobs failed", file=sys.stderr)
    sys.exit(1)
PY

if [[ -n "$API_KEY" ]]; then
  echo "ℹ️  Verifying authenticated repository listing"
  curl -fsSL \
    -H "X-API-Key: $API_KEY" \
    "$BASE_URL/api/v1/repositories" >/dev/null
else
  echo "⚠️  Skipping authenticated checks (no API key provided)"
fi

if [[ $MCP_CHECK -eq 1 ]]; then
  if [[ -z "$API_KEY" ]]; then
    echo "❌ Cannot run MCP smoke checks without API key" >&2
    exit 1
  fi

  echo "ℹ️  MCP tools smoke"
  curl -fsSL \
    -H "X-API-Key: $API_KEY" \
    -H 'Content-Type: application/json' \
    -d '{}' \
    "$BASE_URL/mcp/tools" >/dev/null

  curl -fsSL \
    -H "X-API-Key: $API_KEY" \
    -H 'Content-Type: application/json' \
    -d '{"query":"async fn","limit":5}' \
    "$BASE_URL/mcp/tools/search_code" >/dev/null
fi

echo "✅ SaaS smoke test passed"
