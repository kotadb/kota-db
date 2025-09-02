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

# Extract host from DATABASE_URL for connection testing
DB_HOST=$(echo "$DATABASE_URL" | sed -n 's/.*@\([^:/]*\).*/\1/p')
DB_PORT=$(echo "$DATABASE_URL" | sed -n 's/.*:\([0-9]*\)\/.*/\1/p')

# Default port if not found
if [ -z "$DB_PORT" ]; then
    DB_PORT=5432
fi

echo "Waiting for PostgreSQL at $DB_HOST:$DB_PORT..."

# Wait for database to be available (max 30 seconds)
MAX_RETRIES=30
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    # Try to connect to the database port
    if nc -z "$DB_HOST" "$DB_PORT" 2>/dev/null; then
        echo "PostgreSQL is reachable!"
        break
    fi
    
    RETRY_COUNT=$((RETRY_COUNT + 1))
    echo "Waiting for database... ($RETRY_COUNT/$MAX_RETRIES)"
    sleep 1
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo "WARNING: Could not verify database connectivity after $MAX_RETRIES seconds"
    echo "Attempting to start server anyway..."
fi

# Create data directory
mkdir -p "${KOTADB_DATA_DIR:-/data}" || true

# Start the server directly to see all output
echo "Starting server on port ${PORT:-8080}..."
exec kotadb-api-server 2>&1