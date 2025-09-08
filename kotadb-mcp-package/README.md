# kotadb-mcp

[![npm version](https://badge.fury.io/js/kotadb-mcp.svg)](https://badge.fury.io/js/kotadb-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Model Context Protocol server for KotaDB - Enable Claude Desktop to understand and analyze your codebase**

## Quick Start (30 seconds)

```bash
# Install globally
npm install -g kotadb-mcp

# Configure Claude Desktop (see Configuration section)
# Then use in Claude Desktop immediately!
```

## Features

✅ **MCP Tools**: Complete codebase intelligence (symbol extraction, search, impact analysis)  
✅ **Zero Setup**: No Rust compilation or binary dependencies required  
✅ **Fast & Lightweight**: In-memory storage with file persistence  
✅ **Claude Desktop Ready**: Official MCP SDK with STDIO transport  
✅ **Cross-Platform**: Works on macOS, Linux, and Windows  

## Architecture Decision

### Standalone TypeScript Implementation

This package provides a **self-contained MCP server** that doesn't require the full KotaDB Rust binary. Here's why:

#### ✅ **Pros (Why We Chose This)**
- **🚀 Instant Setup**: No 10+ minute Rust compilation
- **📦 Easy Distribution**: Standard npm package workflow  
- **🔧 Zero Dependencies**: No external binaries to manage
- **🌐 Universal Access**: Works for non-developers immediately
- **⚡ Fast Startup**: Sub-second initialization

#### ⚠️ **Trade-offs (What You're Missing)**
- **Advanced Indexing**: No trigram or vector search indices (simple text matching instead)
- **Scalability**: Designed for personal projects (thousands of files, not millions)
- **Storage Format**: Independent from main KotaDB database files
- **Performance**: JavaScript vs Rust performance characteristics

#### 🎯 **When to Use Each**

| Use Case | This Package | Full KotaDB Rust |
|----------|-------------|------------------|
| **Personal Projects** | ✅ Perfect | Overkill |
| **Claude Desktop Integration** | ✅ Ideal | Complex setup |
| **Code Analysis** | ✅ Great | More features |
| **Enterprise Codebases** | Consider Rust | ✅ Recommended |
| **Advanced Symbol Search** | Basic only | ✅ Full-featured |
| **Large Repositories** | < 10K files | ✅ Unlimited |

## Installation & Setup

### Option 1: Global Installation (Recommended)

```bash
npm install -g kotadb-mcp
```

### Option 2: Local Development

```bash
git clone https://github.com/jayminwest/kota-db.git
cd kota-db/kotadb-mcp-package
npm install && npm run build
```

## Claude Desktop Configuration

### Step 1: Locate Configuration File

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
**Linux**: `~/.config/claude-desktop/config.json`

### Step 2: Add KotaDB MCP Server

```json
{
  "mcpServers": {
    "kotadb": {
      "command": "npx",
      "args": ["-y", "kotadb-mcp"],
      "env": {
        "KOTADB_DATA_DIR": "~/Documents/kotadb-data"
      }
    }
  }
}
```

### Step 3: Restart Claude Desktop

Completely quit and restart Claude Desktop to load the new MCP server.

## Available Tools

### Codebase Intelligence
- **`kotadb_index_codebase`** - Index a codebase with symbol extraction
- **`kotadb_search_code`** - Search for code patterns and symbols
- **`kotadb_search_symbols`** - Find functions, classes, and variables by name
- **`kotadb_find_callers`** - Find all references to a symbol
- **`kotadb_analyze_impact`** - Analyze what breaks if you change something

### Analysis & Statistics  
- **`kotadb_symbol_stats`** - View extracted symbol statistics
- **`kotadb_stats`** - Database statistics and information

### Tool Name Differences from Full KotaDB Rust Implementation

The TypeScript MCP package uses consistent `kotadb_` prefixes to clearly distinguish it from the full Rust implementation:

| This Package (TypeScript) | Full KotaDB Rust | Functionality |
|---------------------------|------------------|---------------|
| `kotadb_index_codebase` | `index-codebase` | Index repository with symbols |
| `kotadb_search_code` | `search-code` | Full-text code search |
| `kotadb_search_symbols` | `search-symbols` | Find symbols by pattern |
| `kotadb_find_callers` | `find-callers` | Find references to symbols |
| `kotadb_analyze_impact` | `analyze-impact` | Impact analysis |
| `kotadb_stats` | `stats` | Comprehensive database statistics (documents, symbols, relationships) |

**Why Different Names?**
- **Clear Origin**: Easy to identify which implementation provided the tool
- **Avoid Conflicts**: Prevents namespace collisions if both are installed
- **User Clarity**: Makes it obvious in Claude Desktop which database system is being used
- **Development Safety**: Reduces confusion during development and debugging

The functionality is intentionally similar to maintain consistency, but the implementation approaches differ significantly (self-contained TypeScript vs Rust binary integration).

## Usage Examples

### Indexing Your Codebase
```typescript
// In Claude Desktop, you can say:
"Index my TypeScript project at /path/to/project"

// This uses kotadb_index_codebase internally
```

### Searching Code
```typescript  
// In Claude Desktop:
"Search for all functions that handle authentication"

// Uses kotadb_search_code with symbol understanding
```

### Impact Analysis
```typescript
// In Claude Desktop:
"What would break if I change the DatabaseConnection class?"

// Uses kotadb_analyze_impact to trace dependencies
```

## Data Storage

### File Structure
```
~/Documents/kotadb-data/          # Your data directory
├── index.json                   # Document metadata index
├── doc-uuid-1.md               # Individual markdown files
├── doc-uuid-2.md               # Auto-generated from content
└── doc-uuid-3.md               # Human-readable format
```

### Backup & Migration
```bash
# Backup your data
cp -r ~/Documents/kotadb-data ~/Backups/kotadb-backup-$(date +%Y%m%d)

# Migrate to new machine
scp -r ~/Documents/kotadb-data user@newmachine:~/Documents/
```

## Development

### Building from Source
```bash
git clone https://github.com/jayminwest/kota-db.git
cd kota-db/kotadb-mcp-package
npm install
npm run build
npm test
```

### Running Tests
```bash
npm test              # Run all tests
npm run test:watch    # Watch mode  
npm run test:coverage # Coverage report
```

### Local Development
```bash
npm run dev           # Start in development mode
npm run build         # Build for production
```

## API Reference

### Search Functionality

```typescript
interface SearchResult {
  id: string;
  path: string; 
  title: string;
  content_preview: string; // First 200 chars with query highlighting
  score: number;          // Relevance score (higher = more relevant)
}
```

### Document Model

```typescript
interface Document {
  id: string;           // UUID
  path: string;         // Unique document path  
  title: string;        // Display title
  content: string;      // Full markdown content
  tags: string[];       // Categorization tags
  createdAt: string;    // ISO timestamp
  updatedAt: string;    // ISO timestamp  
}
```

## Troubleshooting

### "Module not found" errors
```bash
# Ensure global installation
npm install -g kotadb-mcp

# Or use full path in config
{
  "command": "/usr/local/bin/node",
  "args": ["/usr/local/lib/node_modules/kotadb-mcp/dist/index.js"]
}
```

### "Connection closed" errors on Windows
```json
{
  "command": "cmd",
  "args": ["/c", "npx", "-y", "kotadb-mcp"]
}
```

### Performance Issues
- Keep document collections under 1000 documents
- Use specific search terms for better performance
- Consider the full KotaDB Rust implementation for larger datasets

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit changes: `git commit -m 'Add amazing feature'`
4. Push to branch: `git push origin feature/amazing-feature`  
5. Open a Pull Request

## License

MIT License - see [LICENSE](../LICENSE) file for details.

## Related Projects

- **[KotaDB](https://github.com/jayminwest/kota-db)** - Full Rust implementation with advanced features
- **[Model Context Protocol](https://modelcontextprotocol.io/)** - Official MCP specification
- **[Claude Desktop](https://claude.ai/desktop)** - AI assistant with MCP support

---

**Made with ❤️ by the KotaDB Team**