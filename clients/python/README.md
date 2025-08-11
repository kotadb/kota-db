# KotaDB Python Client

A simple, PostgreSQL-level easy-to-use Python client for KotaDB.

## Installation

```bash
pip install kotadb-client
```

## Quick Start

```python
from kotadb import KotaDB

# Connect to KotaDB
db = KotaDB("http://localhost:8080")

# Insert a document
doc_id = db.insert({
    "path": "/notes/meeting.md",
    "title": "Team Meeting Notes",
    "content": "Discussed project roadmap and next steps...",
    "tags": ["work", "meeting", "planning"]
})

# Search for documents
results = db.query("project roadmap")
for result in results.results:
    print(f"Found: {result.document.title} (score: {result.score})")

# Get a specific document
doc = db.get(doc_id)
print(f"Document: {doc.title}")

# Update a document
updated_doc = db.update(doc_id, {
    "content": "Updated meeting notes with action items..."
})

# Delete a document
db.delete(doc_id)
```

## Connection Options

### Environment Variable
```bash
export KOTADB_URL="http://localhost:8080"
```

```python
# Will use KOTADB_URL automatically
db = KotaDB()
```

### Connection String
```python
# PostgreSQL-style connection string
db = KotaDB("kotadb://localhost:8080/myapp")

# Direct HTTP URL
db = KotaDB("http://localhost:8080")
```

### Context Manager
```python
with KotaDB("http://localhost:8080") as db:
    results = db.query("search term")
    # Connection automatically closed
```

## Search Options

### Text Search
```python
results = db.query("rust programming patterns", limit=10)
```

### Semantic Search
```python
results = db.semantic_search("machine learning concepts", limit=5)
```

### Hybrid Search
```python
results = db.hybrid_search(
    "database optimization",
    limit=10,
    semantic_weight=0.7  # 70% semantic, 30% text
)
```

## Document Operations

### Create Document
```python
# Using dictionary
doc_id = db.insert({
    "path": "/docs/guide.md",
    "title": "User Guide",
    "content": "How to use the system...",
    "tags": ["documentation", "guide"],
    "metadata": {"author": "jane@example.com"}
})

# Using CreateDocumentRequest
from kotadb.types import CreateDocumentRequest

doc_request = CreateDocumentRequest(
    path="/docs/api.md",
    title="API Documentation",
    content="API endpoints and usage...",
    tags=["api", "docs"]
)
doc_id = db.insert(doc_request)
```

### List Documents
```python
# Get all documents
all_docs = db.list_all()

# With pagination
docs = db.list_all(limit=50, offset=100)
```

### Database Health
```python
# Check health
health = db.health()
print(f"Status: {health['status']}")

# Get statistics
stats = db.stats()
print(f"Document count: {stats['document_count']}")
```

## Error Handling

```python
from kotadb.exceptions import KotaDBError, NotFoundError, ConnectionError

try:
    doc = db.get("non-existent-id")
except NotFoundError:
    print("Document not found")
except ConnectionError:
    print("Failed to connect to database")
except KotaDBError as e:
    print(f"Database error: {e}")
```

## Configuration

```python
db = KotaDB(
    url="http://localhost:8080",
    timeout=30,  # Request timeout in seconds
    retries=3    # Number of retry attempts
)
```

## Data Types

### Document
```python
@dataclass
class Document:
    id: str
    path: str
    title: str
    content: str
    tags: List[str]
    created_at: datetime
    updated_at: datetime
    size: int
    metadata: Optional[Dict[str, Any]]
```

### SearchResult
```python
@dataclass
class SearchResult:
    document: Document
    score: float
    content_preview: str
```

### QueryResult
```python
@dataclass
class QueryResult:
    results: List[SearchResult]
    total_count: int
    query_time_ms: int
```

## Development

Install development dependencies:
```bash
pip install -e ".[dev]"
```

Run tests:
```bash
pytest
```

Format code:
```bash
black kotadb/
```

Type checking:
```bash
mypy kotadb/
```

## License

MIT License - see LICENSE file for details.

## Contributing

See CONTRIBUTING.md for contribution guidelines.

## Support

- GitHub Issues: https://github.com/jayminwest/kota-db/issues
- Documentation: https://github.com/jayminwest/kota-db/docs
