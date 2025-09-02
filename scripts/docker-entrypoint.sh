#!/bin/sh

echo "=== KotaDB API Server Starting ==="
echo "PORT: ${PORT:-8080}"
echo "DATABASE_URL: ${DATABASE_URL:+SET}"
echo "KOTADB_DATA_DIR: ${KOTADB_DATA_DIR:-/data}"
echo "RUST_LOG: ${RUST_LOG:-info}"

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL is not set!"
    exit 1
fi

echo "DATABASE_URL is configured"

# Create data directory
mkdir -p "${KOTADB_DATA_DIR:-/data}" || true

# Simple delay to let Railway's internal networking stabilize
echo "Waiting 10 seconds for network initialization..."
sleep 10

# Start the server directly to see all output
echo "Starting server on port ${PORT:-8080}..."
exec kotadb-api-server 2>&1