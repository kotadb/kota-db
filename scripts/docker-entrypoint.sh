#!/bin/sh
set -e

echo "Starting KotaDB API Server..."
echo "Environment check:"
echo "  PORT: ${PORT:-not set}"
echo "  DATABASE_URL: ${DATABASE_URL:+[REDACTED]}"
echo "  KOTADB_DATA_DIR: ${KOTADB_DATA_DIR:-not set}"
echo "  RUST_LOG: ${RUST_LOG:-not set}"

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL environment variable is not set"
    exit 1
fi

# Test database connection
echo "Testing database connection..."
if command -v pg_isready > /dev/null 2>&1; then
    # Extract connection details from DATABASE_URL
    DB_HOST=$(echo $DATABASE_URL | sed -n 's/.*@\([^:]*\):.*/\1/p')
    DB_PORT=$(echo $DATABASE_URL | sed -n 's/.*:\([0-9]*\)\/.*/\1/p')
    pg_isready -h "$DB_HOST" -p "$DB_PORT" || echo "Warning: Database might not be ready"
else
    echo "pg_isready not available, skipping connection test"
fi

# Create data directory if it doesn't exist
if [ ! -d "$KOTADB_DATA_DIR" ]; then
    echo "Creating data directory: $KOTADB_DATA_DIR"
    mkdir -p "$KOTADB_DATA_DIR"
fi

# Start the server
echo "Launching kotadb-api-server..."
exec kotadb-api-server