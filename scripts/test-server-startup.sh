#!/bin/bash

echo "Testing kotadb-api-server startup..."

# Set minimal environment variables
export DATABASE_URL="postgresql://test:test@localhost:5432/test"
export PORT="8080"
export KOTADB_DATA_DIR="/tmp/kotadb-test"
export RUST_LOG="debug"

# Create data directory
mkdir -p "$KOTADB_DATA_DIR"

# Try to start the server (it will fail to connect to DB but we'll see startup messages)
echo "Starting server (expecting DB connection failure)..."
timeout 5 cargo run --bin kotadb-api-server 2>&1 | head -50

echo ""
echo "If you see startup messages before the DB error, the binary is working."