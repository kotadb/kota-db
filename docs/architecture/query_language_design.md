---
title: "KOTA Query Language (KQL) Design"
tags: [database, query-language, design]
related: ["IMPLEMENTATION_PLAN.md", "TECHNICAL_ARCHITECTURE.md", "DATA_MODEL_SPECIFICATION.md"]
key_concepts: [query-language, natural-language, graph-queries, temporal-queries]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KOTA Query Language (KQL) Design

> ⚠️ **IMPORTANT**: This document describes the **planned** query language for KotaDB. Most features described here are **not yet implemented**.
>
> **Currently Implemented:**
> - ✅ Text search via trigram index
> - ✅ Semantic search via HNSW vector index
> - ✅ Basic path-based queries with wildcards
>
> **Not Yet Implemented:**
> - ⏳ Natural language processing
> - ⏳ Temporal queries and aggregations
> - ⏳ Graph traversal queries
> - ⏳ Advanced structured queries
> - ⏳ Pattern matching and analysis
>
> See the [Current API](#current-api) section at the end for what's actually available today.

## Overview

KQL is designed to be a natural, intuitive query language that bridges human thought patterns and AI cognitive processes. Unlike SQL, which was designed for tabular data, KQL natively understands documents, relationships, time, and meaning.

## Design Philosophy

1. **Natural Language First**: Queries should read like thoughts
2. **Context-Aware**: Implicit understanding of current context
3. **Temporal by Default**: Time is always a consideration
4. **Relationship-Centric**: Everything connects to everything
5. **AI-Native**: Designed for LLM generation and interpretation

## Query Types

### 1. Natural Language Queries (🚧 PLANNED - Not Yet Implemented)

The primary interface will be natural language, processed by an LLM-powered parser:

```
# These queries are PLANNED features, not currently available:
"What did I learn about rust last week?"
"Show me all meetings with Greg from Cogzia"
"Find documents similar to distributed cognition"
"What are my productivity patterns?"
"When was the last time I felt energized after a meeting?"
```

### 2. Structured Queries

For precise control and programmatic access:

```javascript
// Find related documents
{
  type: "graph",
  start: "projects/kota-ai/README.md",
  follow: ["related", "references"],
  depth: 2,
  filter: {
    tags: { $contains: "architecture" }
  }
}

// Semantic search with filters
{
  type: "semantic",
  query: "consciousness implementation",
  threshold: 0.7,
  filter: {
    created: { $gte: "2025-01-01" },
    path: { $match: "*/consciousness/*" }
  },
  limit: 10
}

// Temporal aggregation (PLANNED - Not Yet Implemented)
{
  type: "temporal",
  aggregate: "count",
  groupBy: "day",
  filter: {
    tags: { $contains: "meeting" }
  },
  range: "last_month"
}
```

### 3. Hybrid Queries (🚧 PLANNED - Not Yet Implemented)

Combining natural language with structured precision:

```
# This syntax is PLANNED, not currently available:
"meetings with Greg" WHERE {
  participants: { $contains: "Greg" },
  duration: { $gte: "30m" }
} ORDER BY created DESC
```

## Query Syntax

### Basic Structure

```
[NATURAL_LANGUAGE] [WHERE CONDITIONS] [ORDER BY fields] [LIMIT n]
```

### Natural Language Processing

The NLP parser extracts:
- **Intent**: search, analyze, summarize, etc.
- **Entities**: people, projects, topics, dates
- **Modifiers**: recent, important, related to
- **Context**: current document, time, previous queries

### Structured Conditions

#### Comparison Operators
- `$eq`: Equals
- `$ne`: Not equals
- `$gt`, `$gte`: Greater than (or equal)
- `$lt`, `$lte`: Less than (or equal)
- `$in`: In array
- `$contains`: Contains substring/element
- `$match`: Regex/glob pattern match

#### Logical Operators
- `$and`: All conditions must match
- `$or`: Any condition must match
- `$not`: Negation
- `$exists`: Field exists

#### Special Operators
- `$similar`: Semantic similarity
- `$near`: Temporal/spatial proximity
- `$related`: Graph relationship exists
- `$matches_pattern`: Behavioral pattern matching

### Field References

Standard fields:
- `path`: File path
- `title`: Document title
- `content`: Full text content
- `tags`: Tag array
- `created`, `updated`: Timestamps
- `frontmatter.*`: Any frontmatter field

Computed fields:
- `relevance`: Relevance score
- `distance`: Semantic distance
- `depth`: Graph traversal depth
- `age`: Time since creation

## Query Examples

### 1. Content Discovery

```
# Natural language
"rust programming tutorials"

# Structured equivalent
{
  type: "text",
  query: "rust programming tutorials",
  boost: {
    title: 2.0,
    tags: 1.5,
    content: 1.0
  }
}

# With filters
"rust tutorials" WHERE {
  created: { $gte: "2024-01-01" },
  tags: { $contains: ["programming", "rust"] }
}
```

### 2. Relationship Navigation

```
# Find all documents connected to a project
GRAPH {
  start: "projects/kota-ai",
  follow: ["related", "implements", "references"],
  depth: 3,
  return: ["path", "title", "relationship_type"]
}

# Find collaboration patterns
"documents edited with Charlie" GRAPH {
  edge_filter: {
    type: "co-edited",
    participant: "Charlie"
  }
}
```

### 3. Temporal Analysis

```
# Activity timeline
TIMELINE {
  range: "last_month",
  events: ["created", "updated"],
  groupBy: "day",
  include: ["meetings", "code_changes", "notes"]
}

# Productivity patterns
"When am I most productive?" ANALYZE {
  metric: "documents_created",
  correlate_with: ["time_of_day", "recovery_score", "previous_activity"],
  period: "last_3_months"
}
```

### 4. Semantic Exploration

```
# Find similar concepts
SIMILAR TO "distributed cognition" {
  threshold: 0.7,
  expand: true,  // Include related concepts
  limit: 20
}

# Concept clustering
CLUSTER {
  algorithm: "semantic",
  min_similarity: 0.6,
  max_clusters: 10
}
```

### 5. Complex Queries

```
# Multi-step analysis
PIPELINE [
  // Step 1: Find all meetings
  { 
    type: "text",
    query: "meeting",
    filter: { tags: { $contains: "meeting" } }
  },
  
  // Step 2: Extract participants
  {
    type: "extract",
    field: "participants",
    unique: true
  },
  
  // Step 3: Analyze collaboration frequency
  {
    type: "aggregate",
    groupBy: "participant",
    count: "meetings",
    average: "duration"
  }
]

# Pattern detection
DETECT PATTERN {
  name: "breakthrough_after_struggle",
  sequence: [
    { tags: { $contains: "challenge" }, sentiment: "negative" },
    { tags: { $contains: "solution" }, sentiment: "positive" },
  ],
  within: "1 week",
  min_occurrences: 3
}
```

## Query Processing Pipeline

### 1. Natural Language Understanding

```rust
pub struct NLUParser {
    // LLM for intent extraction
    llm: Box<dyn LanguageModel>,
    
    // Entity recognition
    entity_extractor: EntityExtractor,
    
    // Temporal expression parser
    temporal_parser: TemporalParser,
    
    // Context manager
    context: QueryContext,
}

impl NLUParser {
    pub async fn parse(&self, query: &str) -> Result<ParsedQuery> {
        // 1. Extract intent and entities
        let intent = self.extract_intent(query).await?;
        let entities = self.extract_entities(query)?;
        
        // 2. Resolve temporal expressions
        let temporal = self.parse_temporal(query)?;
        
        // 3. Build structured query
        self.build_query(intent, entities, temporal)
    }
}
```

### 2. Query Optimization

```rust
pub struct QueryOptimizer {
    // Statistics for cost estimation
    stats: DatabaseStatistics,
    
    // Index availability
    indices: IndexCatalog,
    
    // Rewrite rules
    rules: Vec<RewriteRule>,
}

impl QueryOptimizer {
    pub fn optimize(&self, query: Query) -> OptimizedQuery {
        // 1. Apply rewrite rules
        let rewritten = self.apply_rules(query);
        
        // 2. Choose optimal indices
        let index_plan = self.select_indices(&rewritten);
        
        // 3. Generate execution plan
        self.generate_plan(rewritten, index_plan)
    }
}
```

### 3. Query Execution

```rust
pub struct QueryExecutor {
    // Storage engine
    storage: StorageEngine,
    
    // Index manager
    indices: IndexManager,
    
    // Cache for repeated queries
    cache: QueryCache,
}

impl QueryExecutor {
    pub async fn execute(&self, plan: ExecutionPlan) -> QueryResult {
        // Check cache first
        if let Some(cached) = self.cache.get(&plan) {
            return cached;
        }
        
        // Execute plan steps
        let result = self.execute_plan(plan).await?;
        
        // Cache results
        self.cache.put(&plan, &result);
        
        result
    }
}
```

## Context-Aware Features

### 1. Pronoun Resolution

```
"What did we discuss?" 
// Resolves 'we' based on current document participants

"Show me more like this"
// 'this' refers to currently viewed document
```

### 2. Temporal Context

```
"What happened next?"
// Continues from previous query time range

"Earlier meetings"
// Relative to last query results
```

### 3. Implicit Filters

```
// In consciousness session context
"recent insights"
// Automatically filters to consciousness-generated content

// In project context
"related issues"
// Scoped to current project
```

## Query Result Types

### 1. Document Results

```rust
pub struct DocumentResult {
    // Core document data
    pub id: DocumentId,
    pub path: String,
    pub title: String,
    
    // Relevance and scoring
    pub score: f32,
    pub highlights: Vec<Highlight>,
    
    // Context
    pub breadcrumbs: Vec<String>,
    pub related: Vec<DocumentId>,
}
```

### 2. Graph Results

```rust
pub struct GraphResult {
    // Nodes
    pub nodes: Vec<Node>,
    
    // Edges
    pub edges: Vec<Edge>,
    
    // Traversal metadata
    pub paths: Vec<Path>,
    pub depths: HashMap<NodeId, u32>,
}
```

### 3. Analytical Results

```rust
pub struct AnalyticalResult {
    // Aggregations
    pub aggregates: HashMap<String, Value>,
    
    // Time series
    pub series: Option<TimeSeries>,
    
    // Statistics
    pub stats: Statistics,
    
    // Insights (LLM-generated)
    pub insights: Vec<Insight>,
}
```

## Advanced Features

### 1. Query Macros

Define reusable query patterns:

```
DEFINE MACRO weekly_review AS {
  PIPELINE [
    { type: "temporal", range: "last_week" },
    { type: "aggregate", by: "day", count: "activities" },
    { type: "analyze", generate: "insights" }
  ]
}

// Use macro
EXECUTE weekly_review WHERE { tags: { $contains: "work" } }
```

### 2. Continuous Queries

Subscribe to ongoing results:

```
SUBSCRIBE TO "new insights" {
  filter: {
    type: "consciousness_session",
    created: { $gte: "now" }
  },
  notify: "webhook://localhost:8080/insights"
}
```

### 3. Query Learning

System learns from usage patterns:

```rust
pub struct QueryLearner {
    // Track query patterns
    query_history: Vec<QueryRecord>,
    
    // Learn common refinements
    refinement_patterns: HashMap<QueryPattern, Vec<Refinement>>,
    
    // Suggest improvements
    suggestion_engine: SuggestionEngine,
}
```

## Integration with KOTA

### 1. Consciousness Queries

```
# Find patterns in consciousness sessions
CONSCIOUSNESS {
  analyze: "themes",
  period: "last_month",
  min_frequency: 3
}

# Track insight evolution
CONSCIOUSNESS EVOLUTION {
  concept: "distributed cognition",
  show: ["first_mention", "developments", "current_understanding"]
}
```

### 2. Health Correlations

```
# Correlate productivity with health
CORRELATE {
  metric1: "documents_created",
  metric2: "whoop.recovery_score",
  period: "last_3_months",
  lag: [0, 1, 2]  // days
}
```

### 3. Project Intelligence

```
# Project health check
PROJECT "kota-ai" ANALYZE {
  metrics: ["velocity", "complexity", "technical_debt"],
  compare_to: "baseline",
  suggest: "improvements"
}
```

## Error Handling

### Query Errors

```javascript
{
  error: {
    type: "PARSE_ERROR",
    message: "Unexpected token 'WHER' - did you mean 'WHERE'?",
    position: 45,
    suggestion: "WHERE"
  }
}
```

### Graceful Degradation

```javascript
{
  warning: "Semantic index unavailable, falling back to text search",
  results: [...],  // Still returns results
  suggestions: ["Try again later for semantic results"]
}
```

## Performance Considerations

### 1. Query Complexity Limits

```toml
[limits]
max_depth = 5           # Graph traversal
max_results = 10000     # Result set size
max_duration = 5000     # Query timeout (ms)
max_memory = 100        # Memory limit (MB)
```

### 2. Query Hints

```
"complex analysis" HINTS {
  use_index: "semantic",
  parallel: true,
  cache: false
}
```

## Future Extensions

### 1. Multi-Modal Queries

```
"Find screenshots similar to [image]"
"Documents discussed in [audio_file]"
```

### 2. Federated Queries

```
FEDERATE {
  sources: ["local", "github", "google_drive"],
  query: "project documentation",
  merge_by: "similarity"
}
```

### 3. Predictive Queries

```
PREDICT {
  what: "next_document_needed",
  based_on: "current_context",
  confidence: 0.8
}
```

## Current API (What's Actually Available Today)

### Text Search
```python
# Python client
from kotadb import KotaDB
db = KotaDB("http://localhost:8080")

# Simple text search using trigram index
results = db.query("rust programming")

# With limit
results = db.query("design patterns", limit=10)
```

```typescript
// TypeScript client
import { KotaDB } from 'kotadb-client';
const db = new KotaDB({ url: 'http://localhost:8080' });

// Simple text search
const results = await db.query("rust programming");

// With options
const results = await db.query("design patterns", { limit: 10 });
```

### Semantic Search (If Embeddings Configured)
```bash
# Via REST API
curl -X POST http://localhost:8080/search/semantic \
  -H "Content-Type: application/json" \
  -d '{"query": "distributed systems concepts", "limit": 10}'
```

### Path-Based Queries
```bash
# CLI wildcard search
kotadb search "*"              # List all documents
kotadb search "/projects/*"    # Documents in projects folder
```

### What's NOT Available
- ❌ Natural language queries ("what did I learn last week")
- ❌ Temporal aggregations (groupBy day/week/month)
- ❌ Graph traversal (follow relationships)
- ❌ Complex filters (participants, duration, etc.)
- ❌ Pattern analysis (productivity patterns)
- ❌ Hybrid queries (natural language + structured)

## Conclusion

KQL is designed to grow with KOTA's cognitive capabilities. It bridges natural human expression with precise data operations, enabling true distributed cognition. The language will evolve based on usage patterns, becoming more intuitive and powerful over time.

**Current Status**: Basic text and semantic search are implemented. The full KQL vision remains a roadmap item for future development.

The key innovation is treating queries not as database operations, but as cognitive requests - allowing KOTA to understand not just what you're looking for, but why you're looking for it.