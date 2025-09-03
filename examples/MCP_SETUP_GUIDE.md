# KotaDB MCP Setup for Claude Code

This guide shows you how to connect KotaDB to Claude Code instances in other repositories on your system via the Model Context Protocol (MCP).

## Quick Setup

### 1. Prerequisites

- Rust toolchain installed
- KotaDB repository cloned locally
- Claude Code with MCP support

### 2. Copy Configuration

Copy the `.mcp.json` file from the KotaDB repository to your project:

```bash
cp /path/to/kota-db/.mcp.json ~/.config/claude-code/mcp.json
# OR copy to your project's .mcp.json
```

### 3. Update Paths

Edit the copied `.mcp.json` file and update the `cwd` path:

```json
{
  "mcpServers": {
    "kotadb": {
      "command": "cargo",
      "args": [
        "run",
        "--release",
        "--bin",
        "mcp_server_stdio",
        "--features",
        "mcp-server",
        "--",
        "--config",
        "kotadb-mcp-dev.toml"
      ],
      "cwd": "/path/to/your/kota-db",
      "env": {
        "RUST_LOG": "warn"
      }
    }
  }
}
```

### 4. Build KotaDB MCP Server

In your KotaDB directory:

```bash
cd /path/to/your/kota-db
cargo build --release --bin mcp_server_stdio --features mcp-server
```

### 5. Test the Connection

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"roots": {"listChanged": true}}}}' | cargo run --release --bin mcp_server_stdio --features mcp-server -- --config kotadb-mcp-dev.toml
```

You should see a JSON response with server capabilities.

## Configuration Options

### Environment Variables

- `RUST_LOG`: Controls logging level (`warn`, `info`, `debug`)
  - `warn`: Minimal output (recommended for Claude Code)
  - `info`: Standard operational logs
  - `debug`: Detailed debugging information

### Server Configuration

The MCP server uses `kotadb-mcp-dev.toml` for detailed configuration:

```toml
[server]
host = "127.0.0.1"
port = 9876
max_connections = 100
request_timeout = "30s"

[database]
data_dir = "./kotadb-data"
max_cache_size = 1000
enable_wal = true

[mcp]
protocol_version = "2024-11-05"
server_name = "kotadb"
server_version = "0.5.0"
enable_search_tools = true

[performance]
max_query_latency_ms = 10
max_semantic_search_latency_ms = 100
```

## Available MCP Tools

Once connected, Claude Code can use these KotaDB tools:

### Search Tools
- **kotadb_search**: Full-text search across codebase
- **kotadb_document_get**: Retrieve specific documents
- **kotadb_document_list**: List documents with pagination
- **kotadb_stats**: Database statistics and information

### Code Intelligence Tools  
- **search-code**: Fast content search (<3ms)
- **search-symbols**: Symbol and function search
- **find-callers**: Find who calls a specific function
- **analyze-impact**: Analyze change impact across codebase

## Troubleshooting

### Common Issues

1. **"Permission denied" or build fails**
   - Ensure Rust toolchain is properly installed
   - Run `cargo clean` then rebuild

2. **"Configuration file not found"**
   - Verify `kotadb-mcp-dev.toml` exists in the KotaDB directory
   - Check the `cwd` path in your `.mcp.json`

3. **MCP connection fails**
   - Test the server manually using the test command above
   - Check `RUST_LOG` is set appropriately
   - Verify the binary built successfully

4. **Slow responses**
   - Use `--release` flag for optimized builds
   - Set `RUST_LOG=warn` to reduce logging overhead
   - Ensure data directory has proper permissions

### Debug Mode

For debugging issues, temporarily change the logging level:

```json
{
  "env": {
    "RUST_LOG": "debug,kotadb=trace"
  }
}
```

### Performance Optimization

- Always use `--release` builds for production usage
- Set `RUST_LOG=warn` or `RUST_LOG=error` for minimal logging
- Consider using a dedicated data directory per project

## Integration with Claude Code

Once configured, Claude Code instances in other repositories can:

1. **Index their codebase**: KotaDB will extract symbols, relationships, and content
2. **Perform intelligent searches**: Find functions, classes, and dependencies
3. **Analyze code relationships**: Understand call graphs and impact analysis
4. **Get contextual suggestions**: Leverage codebase intelligence for better AI assistance

The integration provides sub-10ms query performance for most operations, dramatically improving AI assistant effectiveness while reducing token usage.

## Recent Updates

- ✅ Fixed critical server startup issues (PR #518)
- ✅ Completed Fly.io deployment for SaaS version
- ✅ Optimized Docker builds and production readiness
- ✅ Enhanced CLI UX with proper quiet mode support

## Support

For issues or questions:
1. Check the [GitHub issues](https://github.com/jayminwest/kota-db/issues)
2. Review recent [PRs and discussions](https://github.com/jayminwest/kota-db/pulls)
3. Follow the dogfooding protocol in AGENT.md for testing changes