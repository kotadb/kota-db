# KotaDB Relationship Queries - The Killer Feature

This document demonstrates KotaDB's unique relationship query capabilities that differentiate it from text search tools by enabling LLMs to understand code architecture and perform impact analysis.

## What Makes This Special?

Unlike grep or text search tools, KotaDB builds a semantic understanding of code relationships through dependency graph analysis. This enables questions that are impossible to answer with traditional search:

- **What calls this function?** (reverse dependency analysis)
- **What would break if I change this?** (impact analysis)  
- **Show me the call chain from A to B** (execution flow tracing)
- **Find circular dependencies** (architectural problem detection)

## MCP Tool Examples

### 1. Find Callers - "What calls this function?"

```bash
# MCP Tool: kotadb://find_callers
{
  "target": "FileStorage::insert"
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "summary": "Found 3 symbols that call/use 'FileStorage::insert'",
    "direct_relationships": [
      {
        "symbol_name": "create_document", 
        "file_path": "src/api/handlers.rs",
        "location": {"line_number": 45, "column_number": 12},
        "relation_type": "Calls",
        "context": "storage.insert(document).await?"
      }
    ]
  }
}
```

### 2. Impact Analysis - "What would break if I change this?"

```bash
# MCP Tool: kotadb://impact_analysis  
{
  "target": "StorageError"
}
```

This finds all code that would be affected by changing the `StorageError` type, including direct usage and transitive dependencies through the call graph.

### 3. Call Chain Analysis - "How does data flow from A to B?"

```bash
# MCP Tool: kotadb://call_chain
{
  "from": "main",
  "to": "handle_error"  
}
```

Shows the execution path: `main → run_server → handle_request → validate_input → handle_error`

### 4. Natural Language Queries

```bash
# MCP Tool: kotadb://relationship_query
{
  "query": "what calls FileStorage?"
}
```

```bash
# MCP Tool: kotadb://relationship_query  
{
  "query": "what would break if I change StorageError?"
}
```

```bash
# MCP Tool: kotadb://relationship_query
{
  "query": "find unused functions"
}
```

## Architecture Benefits

### For LLMs
- **Safe Refactoring**: Know exactly what will break before making changes
- **Code Understanding**: Trace execution flows and data dependencies  
- **Architecture Analysis**: Identify design problems like circular dependencies
- **Dead Code Detection**: Find unused symbols for cleanup

### For Developers
- **Impact Assessment**: "If I change this API, what breaks?"
- **Code Navigation**: "How does this error bubble up to the user?"
- **Architectural Insights**: "What are the core components everything depends on?"
- **Maintenance**: "What code can I safely delete?"

### Differentiation from Text Search

| Capability | Grep/Text Search | KotaDB Relationship Queries |
|------------|------------------|----------------------------|
| "What calls function X?" | ❌ Can't determine callers | ✅ Precise caller analysis |
| "What breaks if I change Y?" | ❌ No impact analysis | ✅ Full impact assessment |
| "Show call path A→B" | ❌ Can't trace execution | ✅ Call chain analysis |  
| "Find circular dependencies" | ❌ No architectural analysis | ✅ Structural problem detection |
| "What's unused?" | ❌ Can't determine usage | ✅ Dead code identification |

## Implementation Architecture

The relationship query system builds on KotaDB's existing infrastructure:

1. **Dependency Extractor** (`src/dependency_extractor.rs`) - Uses tree-sitter to build dependency graphs
2. **Relationship Query Engine** (`src/relationship_query.rs`) - Graph traversal algorithms  
3. **Natural Language Processing** - Converts English queries to graph operations
4. **MCP Integration** - Exposes capabilities through Model Context Protocol

## Key Algorithms

- **Reverse Dependencies**: Graph traversal with incoming edges
- **Impact Analysis**: BFS traversal with depth limiting to find transitive effects
- **Call Chain Finding**: Dijkstra's shortest path algorithm
- **Circular Dependencies**: Kosaraju's strongly connected components algorithm
- **Hot Path Analysis**: In-degree ranking to find most-called symbols

This combination of semantic code understanding + graph algorithms + natural language interface creates a unique capability that no text search tool can provide.