# KotaDB 60-Second Quick Start

🚀 **From zero to first query in under 60 seconds**

## One-Command Start

```bash
# Start KotaDB with demo data
docker-compose -f docker-compose.quickstart.yml up -d

# Wait for server to be ready (10-15 seconds)
docker-compose -f docker-compose.quickstart.yml logs -f kotadb-server

# Run Python demo (shows all core features)
docker-compose -f docker-compose.quickstart.yml --profile demo up python-demo

# Or run TypeScript demo
docker-compose -f docker-compose.quickstart.yml --profile demo up typescript-demo
```

## What You Get

- ✅ KotaDB server running on http://localhost:8080
- ✅ Pre-loaded sample data (notes, documents, code examples)
- ✅ Working Python and TypeScript client examples
- ✅ All core features demonstrated: CRUD, search, indexing
- ✅ Optional web UI on http://localhost:3000

## Manual Testing

```bash
# Check server health
curl http://localhost:8080/health

# Search documents
curl "http://localhost:8080/search?q=rust&limit=5"

# Get database stats
curl http://localhost:8080/stats
```

## What's Demonstrated

### Document Operations
- Creating documents with metadata and tags
- Retrieving documents by ID
- Updating document content
- Deleting documents

### Search Features
- Full-text search with trigram index
- Wildcard path-based queries
- Tag filtering
- Relevance ranking

### Performance
- Sub-10ms query latency
- 1000+ operations per second
- Concurrent access patterns

### Type Safety
- Builder patterns in all clients
- Runtime validation
- Error handling

## Next Steps

1. **Install client library**: `pip install kotadb-client` or `npm install kotadb-client`
2. **Try the examples**: See `examples/` directory for comprehensive demos
3. **Read the docs**: Visit the [full documentation](../docs/)
4. **Build your app**: Use the demo code as starting points

## Clean Up

```bash
# Stop everything
docker-compose -f docker-compose.quickstart.yml down -v

# Remove demo data
rm -rf quickstart-data/
```

**Total setup time: ~30 seconds • First query: ~60 seconds** ⚡