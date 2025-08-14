# KotaDB MCP Server

[![npm version](https://badge.fury.io/js/kotadb-mcp.svg)](https://badge.fury.io/js/kotadb-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Model Context Protocol (MCP) server that enables Claude Desktop to interact with your KotaDB database. Search, create, and manage documents using natural language through Claude.

## 🚀 Quick Start

### 30-Second Setup

```bash
# Install globally
npm install -g kotadb-mcp

# Auto-configure Claude Desktop
kotadb-mcp-setup

# That's it! Restart Claude Desktop and start using KotaDB
```

### Usage in Claude Desktop

Once configured, you can interact with your KotaDB through Claude:

- **"Search my KotaDB for rust programming concepts"**
- **"Create a document about AI safety in my KotaDB"**
- **"Show me statistics about my knowledge base"**
- **"Find documents related to machine learning"**

## 📋 Prerequisites

- **Node.js 18+**
- **KotaDB binary** installed and available in PATH
- **Claude Desktop** (for GUI usage)

## 🛠 Installation

### Global Installation (Recommended)

```bash
npm install -g kotadb-mcp
```

### Local Installation

```bash
npm install kotadb-mcp
```

## ⚙️ Configuration

### Automatic Setup

The easiest way to configure Claude Desktop:

```bash
# Use default data directory (~/.kotadb/data)
kotadb-mcp-setup

# Use custom data directory
kotadb-mcp-setup --data-dir /path/to/your/kotadb/data
```

### Manual Setup

Add this to your Claude Desktop config file (`~/.config/claude-desktop/config.json` on Linux, `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "kotadb": {
      "command": "npx",
      "args": ["kotadb-mcp"],
      "env": {
        "KOTADB_DATA_DIR": "/path/to/your/kotadb/data"
      }
    }
  }
}
```

### Configuration Management

```bash
# Check current configuration
kotadb-mcp-setup status

# Remove KotaDB from Claude Desktop
kotadb-mcp-setup remove

# Reconfigure with different data directory
kotadb-mcp-setup --data-dir /new/path/to/data
```

## 🔧 Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `KOTADB_DATA_DIR` | Path to KotaDB data directory | `~/.kotadb/data` |

## 🛠 Available Tools

The MCP server provides these tools to Claude:

### `kotadb_search`
Search documents using natural language queries.

**Parameters:**
- `query` (string): Search query text
- `limit` (number, optional): Maximum results (default: 10)

### `kotadb_create_document`
Create a new document in KotaDB.

**Parameters:**
- `path` (string): Unique document path
- `title` (string, optional): Document title
- `content` (string): Document content
- `tags` (array, optional): Document tags

### `kotadb_stats`
Get database statistics and information.

**Parameters:** None

## 📚 Resources

The server also exposes resources for browsing:

- `kotadb://documents` - Browse all documents in the database

## 🏗 Development

### Setup Development Environment

```bash
git clone https://github.com/jayminwest/kota-db.git
cd kota-db/kotadb-mcp-package
npm install
```

### Build

```bash
npm run build
```

### Development Mode

```bash
npm run dev
```

### Testing

```bash
npm test
```

## 🐛 Troubleshooting

### "KotaDB binary not found"

Make sure KotaDB is installed and available in your PATH:

```bash
# Check if KotaDB is installed
kotadb --version

# If not installed, install from source or binary release
# See: https://github.com/jayminwest/kota-db#installation
```

### "Claude Desktop not connecting"

1. Check your configuration:
   ```bash
   kotadb-mcp-setup status
   ```

2. Verify the data directory exists and is accessible:
   ```bash
   ls -la ~/.kotadb/data
   ```

3. Restart Claude Desktop completely

4. Check Claude Desktop logs for MCP connection errors

### "Permission denied" errors

Make sure the KotaDB data directory is writable:

```bash
chmod -R 755 ~/.kotadb
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](https://github.com/jayminwest/kota-db/blob/main/CONTRIBUTING.md).

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Add tests if applicable
5. Run tests: `npm test`
6. Commit changes: `git commit -m 'Add amazing feature'`
7. Push to branch: `git push origin feature/amazing-feature`
8. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- [KotaDB](https://github.com/jayminwest/kota-db) - The main KotaDB database
- [Model Context Protocol](https://github.com/modelcontextprotocol) - The MCP specification
- [Claude Desktop](https://claude.ai/download) - AI assistant with MCP support

## 📞 Support

- **Documentation**: [KotaDB Docs](https://github.com/jayminwest/kota-db/docs)
- **Issues**: [GitHub Issues](https://github.com/jayminwest/kota-db/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jayminwest/kota-db/discussions)

## 🚀 Roadmap

- [ ] Enhanced search capabilities (semantic, hybrid)
- [ ] Document browsing and editing
- [ ] Real-time synchronization
- [ ] Graph traversal tools
- [ ] Advanced analytics
- [ ] Multi-database support
- [ ] Binary download automation

---

**Made with ❤️ by the KotaDB team**