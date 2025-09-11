# MCP Implementations Guide

This document covers KotaDB's Model Context Protocol (MCP) implementations for AI assistant integration.

## Overview

KotaDB provides two MCP implementations to support different integration patterns:

1. **MCP-over-HTTP Bridge** (Issue #541) - HTTP endpoints that mirror MCP functionality
2. **Intent-Based MCP Server** (Issue #645) - Natural language interface for AI assistants

## MCP-over-HTTP Bridge

### Purpose
Enables Claude Code integration without requiring local Rust compilation by providing HTTP endpoints that translate to MCP protocol calls.

### Architecture
- HTTP POST endpoints at `/mcp/*`
- API key authentication using existing system
- Protocol translation layer (HTTP → MCP JSON-RPC)
- Feature-gated for compatibility

### Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /mcp/tools` | List available MCP tools (POST also supported for compatibility) |
| `POST /mcp/tools/:tool_name` | Execute specific MCP tool by name |
| `POST /mcp/tools/search_code` | Search code content |
| `POST /mcp/tools/search_symbols` | Search symbols |
| `POST /mcp/tools/find_callers` | Find function callers |
| `POST /mcp/tools/analyze_impact` | Analyze change impact |
| `GET /mcp/tools/stats` | Bridge help and discovery for stats (POST also supported) |

### Usage
```bash
# Start HTTP server with MCP bridge
cargo run --bin kotadb-api-server --features mcp-server

# List tools (preferred)
curl -sS http://localhost:8080/mcp/tools \
  -H "Authorization: Bearer $API_KEY"

# Call a tool (example: text search)
curl -sS -X POST http://localhost:8080/mcp/tools/search_code \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "storage", "limit": 10}'

# Bridge stats/discovery
curl -sS http://localhost:8080/mcp/tools/stats \
  -H "Authorization: Bearer $API_KEY"
```

### Error Codes

Bridge errors use a stable schema `{ success: false, error: { code, message } }`:
- `feature_disabled` – MCP feature or relationship tools not enabled
- `tool_not_found` – unknown tool name
- `registry_unavailable` – bridge is enabled but no tool registry configured
- `internal_error` – unexpected runtime error when invoking a tool

### Implementation Files
- `src/mcp_http_bridge.rs` - Core bridge implementation
- `src/http_server.rs` - Integration with HTTP server

## Intent-Based MCP Server

### Purpose
Transforms natural language queries into orchestrated API calls, providing a conversational interface for AI assistants.

### Architecture
- **Intent Parser**: Natural language → structured intents
- **Query Orchestrator**: Intents → HTTP API calls
- **Context Manager**: Session state and conversation memory
- **Response Generator**: Technical results → AI-friendly format

### Supported Intents

#### Search Intent
- **Patterns**: "find", "search", "look for", "locate"
- **Scopes**: functions, classes, variables, symbols, files, code
- **Example**: "Find all async functions in the storage module"

#### Analysis Intent  
- **Patterns**: "impact", "who calls", "dependencies", "usage"
- **Types**: callers, callees, impact analysis, dependency tracking
- **Example**: "Who calls validate_path?"

#### Navigation Intent
- **Patterns**: "show implementation", "definition", "usage"
- **Contexts**: implementation, definition, usage examples
- **Example**: "Show me the implementation of FileStorage"

#### Overview Intent
- **Patterns**: "overview", "summary", "architecture"
- **Levels**: summary, detailed, comprehensive
- **Example**: "Give me an overview of the codebase"

#### Debugging Intent
- **Patterns**: "debug", "error", "problem", "issue"
- **Context**: Error messages and debugging scenarios
- **Example**: "Debug authentication error in login flow"

### Usage

#### Interactive Mode (Development)
```bash
cargo run --bin intent_mcp_server -- --interactive
```

#### MCP Protocol Mode (Production)
```bash
cargo run --bin intent_mcp_server -- \
  --api-url http://localhost:8080 \
  --api-key $API_KEY
```

#### Configuration Options
```bash
--api-url <URL>          # Base URL for KotaDB HTTP API
--api-key <KEY>          # API key for authentication  
--max-results <NUM>      # Maximum results per query
--timeout <MS>           # Request timeout in milliseconds
-i, --interactive        # Interactive mode for testing
-v, --verbose            # Increase verbosity
```

### Natural Language Examples

| Query | Intent | API Calls |
|-------|--------|-----------|
| "Find async functions in storage" | Search(Functions) | `/api/symbols/search?q=async storage` |
| "Who calls validate_path?" | Analysis(Callers) | `/api/relationships/callers/validate_path` |
| "Impact of changing FileStorage" | Analysis(Impact) | `/api/analysis/impact/FileStorage` |
| "Show codebase overview" | Overview(Summary) | `/stats` + contextual searches |

### Implementation Files
- `src/intent_mcp_server.rs` - Core intent processing
- `src/bin/intent_mcp_server.rs` - Standalone binary

## Integration Patterns

### Claude Code Integration
1. **Development**: Use HTTP bridge endpoints for rapid testing
2. **Production**: Deploy intent-based server for natural language queries

### Custom AI Assistants
1. **Structured**: Use HTTP bridge for predictable API calls
2. **Conversational**: Use intent server for natural language interaction

### Hybrid Approach
- HTTP bridge for deterministic operations
- Intent server for exploratory and conversational queries

## Testing

### Unit Tests
```bash
# Test MCP bridge
cargo nextest run mcp_http_bridge::tests --lib

# Test intent server  
cargo nextest run intent_mcp_server::tests --lib
```

### Integration Testing
```bash
# Test intent server interactively
cargo run --bin intent_mcp_server -- --interactive

# Test HTTP bridge with curl
curl -X POST http://localhost:8080/mcp/tools \
  -H "Authorization: Bearer $API_KEY"
```

## Development Notes

### Feature Gates
Both implementations respect the `mcp-server` feature flag:
```toml
# Enable full MCP functionality
cargo run --features mcp-server

# Basic HTTP server without MCP bridge/tools
cargo run

# When `mcp-server` is disabled, /mcp/* endpoints return 501 with
# error.code = "feature_disabled".
```

### Extension Points
- **Intent Patterns**: Add new regex patterns in `IntentParser`
- **Tool Mappings**: Extend tool registry in HTTP bridge
- **Response Formatting**: Customize AI assistant responses
- **Context Persistence**: Add database backing for conversation state

### Security Considerations
- All endpoints require API key authentication
- Rate limiting should be configured at reverse proxy level
- Input validation prevents injection attacks
- Error messages avoid leaking sensitive information

## Deployment

### Docker (Recommended)
```dockerfile
FROM rust:1.75 as builder
COPY . .
RUN cargo build --release --bin intent_mcp_server

FROM debian:bookworm-slim
COPY --from=builder /target/release/intent_mcp_server /usr/local/bin/
CMD ["intent_mcp_server", "--api-url", "http://kotadb:8080"]
```

### Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: intent-mcp-server
spec:
  replicas: 2
  template:
    spec:
      containers:
      - name: intent-mcp
        image: kotadb/intent-mcp-server:latest
        env:
        - name: API_KEY
          valueFrom:
            secretKeyRef:
              name: kotadb-secrets
              key: api-key
```

### Monitoring
- Structured logging with tracing
- Metrics available via `/stats` endpoint
- Health checks via server status
- Error rates and response times tracked

## Future Enhancements

### Planned Features
- Vector similarity search integration
- Multi-turn conversation support  
- Custom domain vocabulary training
- GraphQL API bridge option
- WebSocket streaming responses

### Community Contributions
- Additional language patterns
- Domain-specific intent recognition
- Custom response formatters
- Integration examples

## Support

- GitHub Issues: [kotadb/issues](https://github.com/jayminwest/kota-db/issues)
- Discussions: Use `mcp-integration` label
- Documentation: See `/docs` directory
- Examples: See `/examples` directory (coming soon)
