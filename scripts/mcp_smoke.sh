#!/usr/bin/env bash
set -euo pipefail

# Simple MCP smoke test script for local KotaDB server
#
# Usage:
#   bash scripts/mcp_smoke.sh                # defaults to http://127.0.0.1:8080
#   bash scripts/mcp_smoke.sh -u http://127.0.0.1:8081
#   bash scripts/mcp_smoke.sh -h             # help

BASE_URL="http://127.0.0.1:8080"
REPO_PATH="."
SKIP_INDEX=0

while getopts ":u:r:Sh" opt; do
  case "$opt" in
    u)
      BASE_URL="$OPTARG"
      ;;
    r)
      REPO_PATH="$OPTARG"
      ;;
    S)
      SKIP_INDEX=1
      ;;
    h)
      echo "MCP smoke test for KotaDB"
      echo ""
      echo "Options:"
      echo "  -u <url>   Base URL (default: $BASE_URL)"
      echo "  -r <path>  Repo path to index (default: current dir)"
      echo "  -S         Skip indexing (use existing DB)"
      echo "  -h         Show help"
      exit 0
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
  esac
done

echo "=== MCP Smoke Test ==="
echo "Base URL: $BASE_URL"

curl_json() {
  local method="$1"; shift
  local url="$1"; shift
  local data="${1:-}"; shift || true
  if [[ -n "$data" ]]; then
    curl -sS -X "$method" "$url" -H 'Content-Type: application/json' -d "$data"
  else
    curl -sS -X "$method" "$url" -H 'Content-Type: application/json'
  fi
}

have_jq=0
if command -v jq >/dev/null 2>&1; then
  have_jq=1
fi

pretty() {
  if [[ $have_jq -eq 1 ]]; then
    jq . || true
  else
    cat
  fi
}

echo "\n-- Health Check --"
curl -sS "$BASE_URL/health" | pretty

if [[ $SKIP_INDEX -eq 0 ]]; then
  echo "\n-- Index Codebase --"
  echo "(This can take several minutes on larger repos)"
  INDEX_PAYLOAD=$(cat <<JSON
{"repo_path":"$REPO_PATH","prefix":"repos","include_files":true,"include_commits":false,"extract_symbols":true}
JSON
  )
  curl_json POST "$BASE_URL/api/index-codebase" "$INDEX_PAYLOAD" | pretty
else
  echo "\n-- Index Codebase --"
  echo "(skipped per -S)"
fi

echo "\n-- MCP: List Tools --"
curl_json POST "$BASE_URL/mcp/tools" '{}' | pretty

echo "\n-- MCP: Symbol Search (FileStorage*) --"
SYMBOL_SEARCH_PAYLOAD='{"pattern":"FileStorage*","limit":25}'
curl_json POST "$BASE_URL/mcp/tools/search_symbols" "$SYMBOL_SEARCH_PAYLOAD" | pretty

echo "\n-- MCP: Content Search (async fn) --"
CONTENT_SEARCH_PAYLOAD='{"query":"async fn","limit":10}'
curl_json POST "$BASE_URL/mcp/tools/search_code" "$CONTENT_SEARCH_PAYLOAD" | pretty

echo "\n=== Done ==="
