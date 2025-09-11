#!/usr/bin/env bash
set -euo pipefail

# Dogfood KotaDB using the HTTP API.
# - Starts the services HTTP server
# - Indexes the current repo
# - Exercises search, symbols, callers, impact, and overview endpoints

PORT="${PORT:-8080}"
DB_DIR="${KOTADB_DATA_DIR:-./kotadb-data/dogfood-http}"
LOG_FILE="dogfood_http.log"

mkdir -p "${DB_DIR}"

echo "[dogfood] Using PORT=${PORT}"
echo "[dogfood] Using DB_DIR=${DB_DIR}"

# Resolve server binary or fallback to cargo
BIN=""
if [ -x target/debug/kotadb ]; then
  BIN="target/debug/kotadb"
elif [ -x target/release/kotadb ]; then
  BIN="target/release/kotadb"
else
  BIN=""
fi

start_server() {
  if [ -n "${BIN}" ]; then
    echo "[dogfood] Starting server binary: ${BIN} on port ${PORT}"
    RUST_LOG=info "${BIN}" --db-path "${DB_DIR}" serve --port "${PORT}" >"${LOG_FILE}" 2>&1 &
    SVPID=$!
  else
    echo "[dogfood] Building and starting via cargo run (first-time may take a bit) on port ${PORT}"
    RUST_LOG=info cargo run --bin kotadb -- --db-path "${DB_DIR}" serve --port "${PORT}" >"${LOG_FILE}" 2>&1 &
    SVPID=$!
  fi
  echo "[dogfood] Server PID=${SVPID}"
}

stop_server() {
  if [ -n "${SVPID:-}" ] && kill -0 "${SVPID}" 2>/dev/null; then
    echo "[dogfood] Stopping server PID=${SVPID}"
    kill "${SVPID}" 2>/dev/null || true
    wait "${SVPID}" 2>/dev/null || true
  fi
}

cleanup() {
  stop_server || true
}
trap cleanup EXIT INT TERM

MAX_PORT_ATTEMPTS=10
attempt_port_start() {
  start_server
  echo "[dogfood] Waiting for server to become healthy on port ${PORT}..."
  ATTEMPTS=120
  for i in $(seq 1 ${ATTEMPTS}); do
    if curl -sf "http://127.0.0.1:${PORT}/health" >/dev/null; then
      echo "[dogfood] Server healthy on port ${PORT}"
      return 0
    fi
    sleep 0.25
  done

  echo "[dogfood] Server failed to become healthy on port ${PORT}." >&2
  tail -n 40 "${LOG_FILE}" || true

  if rg -qi "(Address already in use|Failed to bind to port)" "${LOG_FILE}" >/dev/null 2>&1; then
    echo "[dogfood] Port ${PORT} appears to be in use. Trying next port..."
    return 2
  fi

  return 1
}

for try in $(seq 1 ${MAX_PORT_ATTEMPTS}); do
  if attempt_port_start; then
    break
  else
    rc=$?
    stop_server || true
    if [ ${rc} -eq 2 ]; then
      PORT=$((PORT+1))
      echo "[dogfood] Retrying on port ${PORT} (${try}/${MAX_PORT_ATTEMPTS})"
      continue
    else
      echo "[dogfood] Server did not start due to non-port issue. Aborting." >&2
      exit 1
    fi
  fi
done

if ! curl -sf "http://127.0.0.1:${PORT}/health" >/dev/null; then
  echo "[dogfood] Unable to start server after ${MAX_PORT_ATTEMPTS} attempts." >&2
  tail -n 200 "${LOG_FILE}" || true
  exit 1
fi

echo "[dogfood] Indexing current repository via HTTP API..."
cat > /tmp/kotadb_index_body.json <<'JSON'
{
  "repo_path": ".",
  "prefix": "self",
  "include_files": true,
  "include_commits": false,
  "extract_symbols": true
}
JSON

if ! curl -sf -X POST "http://127.0.0.1:${PORT}/api/index-codebase" \
  -H 'Content-Type: application/json' \
  --data @/tmp/kotadb_index_body.json \
  -o /tmp/kotadb_index_result.json; then
  echo "[dogfood] Indexing failed" >&2
  tail -n 200 "${LOG_FILE}" || true
  exit 1
fi

echo "[dogfood] Index result (summary):"
if command -v jq >/dev/null 2>&1; then
  jq '{success, files_processed, symbols_extracted, relationships_found}' /tmp/kotadb_index_result.json || true
else
  cat /tmp/kotadb_index_result.json | head -n 40
fi

echo "[dogfood] Search code (simple): query=IndexingService limit=5"
curl -sf "http://127.0.0.1:${PORT}/api/search-code?query=IndexingService&limit=5&format=simple" \
  -o /tmp/kotadb_search_code.json || true
if command -v jq >/dev/null 2>&1; then
  jq '.results' /tmp/kotadb_search_code.json | head -n 20
else
  sed -n '1,20p' /tmp/kotadb_search_code.json
fi

echo "[dogfood] Search symbols (cli): pattern=SearchService limit=10"
curl -sf "http://127.0.0.1:${PORT}/api/search-symbols?pattern=SearchService&limit=10&format=cli" \
  -o /tmp/kotadb_search_symbols.json || true
if command -v jq >/dev/null 2>&1; then
  jq -r '.output' /tmp/kotadb_search_symbols.json | sed -n '1,40p'
else
  sed -n '1,60p' /tmp/kotadb_search_symbols.json
fi

echo "[dogfood] Find callers (cli): symbol=IndexingService limit=5"
cat > /tmp/kotadb_callers_body.json <<'JSON'
{
  "symbol": "IndexingService",
  "limit": 5,
  "format": "cli",
  "include_indirect": false
}
JSON
curl -sf -X POST "http://127.0.0.1:${PORT}/api/find-callers" \
  -H 'Content-Type: application/json' \
  --data @/tmp/kotadb_callers_body.json \
  -o /tmp/kotadb_callers.json || true
if command -v jq >/dev/null 2>&1; then
  jq -r '.output' /tmp/kotadb_callers.json | sed -n '1,60p'
else
  sed -n '1,80p' /tmp/kotadb_callers.json
fi

echo "[dogfood] Analyze impact (cli): symbol=SearchService limit=5"
cat > /tmp/kotadb_impact_body.json <<'JSON'
{
  "symbol": "SearchService",
  "limit": 5,
  "format": "cli",
  "max_depth": 2
}
JSON
curl -sf -X POST "http://127.0.0.1:${PORT}/api/analyze-impact" \
  -H 'Content-Type: application/json' \
  --data @/tmp/kotadb_impact_body.json \
  -o /tmp/kotadb_impact.json || true
if command -v jq >/dev/null 2>&1; then
  jq -r '.output // .markdown // .impacts' /tmp/kotadb_impact.json | sed -n '1,60p'
else
  sed -n '1,120p' /tmp/kotadb_impact.json
fi

echo "[dogfood] Codebase overview (json): top_symbols_limit=5 entry_points_limit=5"
curl -sf "http://127.0.0.1:${PORT}/api/codebase-overview?format=json&top_symbols_limit=5&entry_points_limit=5" \
  -o /tmp/kotadb_overview.json || true
if command -v jq >/dev/null 2>&1; then
  jq '.overview_data | {total_files, total_symbols, total_relationships, file_organization, symbols_by_language}' /tmp/kotadb_overview.json || true
else
  sed -n '1,80p' /tmp/kotadb_overview.json
fi

echo "[dogfood] Stats (json): basic=true symbols=true relationships=true"
curl -sf "http://127.0.0.1:${PORT}/api/stats?basic=true&symbols=true&relationships=true" \
  -o /tmp/kotadb_stats.json || true
if command -v jq >/dev/null 2>&1; then
  jq '{
        documents: (.basic_stats.document_count // 0),
        total_size_bytes: (.basic_stats.total_size_bytes // 0),
        total_symbols: (.symbol_stats.total_symbols // 0),
        total_relationships: (.relationship_stats.total_relationships // 0),
        top_languages: ((.symbol_stats.symbols_by_language // {})
          | to_entries | sort_by(-.value) | .[0:3])
      }' /tmp/kotadb_stats.json || true
else
  sed -n '1,120p' /tmp/kotadb_stats.json
fi

echo "[dogfood] Benchmark (json): operations=300 type=search"
cat > /tmp/kotadb_bench_body.json <<'JSON'
{
  "operations": 300,
  "benchmark_type": "search",
  "format": "json"
}
JSON
curl -sf -X POST "http://127.0.0.1:${PORT}/api/benchmark" \
  -H 'Content-Type: application/json' \
  --data @/tmp/kotadb_bench_body.json \
  -o /tmp/kotadb_bench.json || true
if command -v jq >/dev/null 2>&1; then
  jq '{
        overall_ops_per_sec: (.operations_per_second // 0),
        search_ops_per_sec: (.results_by_type.search.operations_per_second // 0),
        avg_search_ms: (.results_by_type.search.average_time_ms // 0)
      }' /tmp/kotadb_bench.json || true
else
  sed -n '1,160p' /tmp/kotadb_bench.json
fi

echo
echo "[dogfood] ✅ Dogfood run complete"
echo "           - Logs: ${LOG_FILE}"
echo "           - Artifacts: /tmp/kotadb_*.{json,txt}"
