#!/bin/bash
# KotaDB MCP Server Startup Script
# Ensures the MCP server is built and starts with proper configuration

set -e

# Build the MCP server if needed
echo "Building KotaDB MCP server..."
cargo build --bin mcp_server_stdio --features mcp-server --quiet

# Start the server
echo "Starting KotaDB MCP server..."
exec ./target/debug/mcp_server_stdio --config kotadb-mcp-dev.toml
