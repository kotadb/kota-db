# KotaDB TypeScript/JavaScript Client

A simple, PostgreSQL-level easy-to-use TypeScript/JavaScript client for KotaDB.

## Installation

```bash
npm install kotadb-client
```

## Quick Start

### TypeScript
```typescript
import { KotaDB } from 'kotadb-client';

// Connect to KotaDB
const db = new KotaDB({ url: 'http://localhost:8080' });

// Insert a document
const docId = await db.insert({
  path: '/notes/meeting.md',
  title: 'Team Meeting Notes',
  content: 'Discussed project roadmap and next steps...',
  tags: ['work', 'meeting', 'planning']
});

// Search for documents
const results = await db.query('project roadmap');
for (const result of results.results) {
  console.log(`Found: ${result.document.title} (score: ${result.score})`);
}

// Get a specific document
const doc = await db.get(docId);
console.log(`Document: ${doc.title}`);

// Update a document
const updatedDoc = await db.update(docId, {
  content: 'Updated meeting notes with action items...'
});

// Delete a document
await db.delete(docId);
```

### JavaScript (CommonJS)
```javascript
const { KotaDB } = require('kotadb-client');

// Connect to KotaDB
const db = new KotaDB({ url: 'http://localhost:8080' });

// Use with async/await or promises
db.query('search term')
  .then(results => {
    console.log(`Found ${results.total_count} results`);
  })
  .catch(error => {
    console.error('Search failed:', error);
  });
```

### JavaScript (ES Modules)
```javascript
import { KotaDB } from 'kotadb-client';

const db = new KotaDB({ url: 'http://localhost:8080' });
const results = await db.query('search term');
```

## Connection Options

### Environment Variable
```bash
export KOTADB_URL="http://localhost:8080"
```

```typescript
// Will use KOTADB_URL automatically
const db = new KotaDB();
```

### Connection String
```typescript
// PostgreSQL-style connection string
const db = new KotaDB({ url: 'kotadb://localhost:8080/myapp' });

// Direct HTTP URL
const db = new KotaDB({ url: 'http://localhost:8080' });
```

### Advanced Configuration
```typescript
const db = new KotaDB({
  url: 'http://localhost:8080',
  timeout: 30000,  // 30 second timeout
  retries: 3,      // 3 retry attempts
  headers: {       // Custom headers
    'Authorization': 'Bearer token',
    'X-Custom-Header': 'value'
  }
});
```

## Search Options

### Text Search
```typescript
const results = await db.query('rust programming patterns', {
  limit: 10,
  offset: 0
});
```

### Semantic Search
```typescript
const results = await db.semanticSearch('machine learning concepts', {
  limit: 5,
  model: 'all-MiniLM-L6-v2'
});
```

### Hybrid Search
```typescript
const results = await db.hybridSearch('database optimization', {
  limit: 10,
  semantic_weight: 0.7  // 70% semantic, 30% text
});
```

## Document Operations

### Create Document
```typescript
const docId = await db.insert({
  path: '/docs/guide.md',
  title: 'User Guide',
  content: 'How to use the system...',
  tags: ['documentation', 'guide'],
  metadata: { author: 'jane@example.com' }
});
```

### List Documents
```typescript
// Get all documents
const allDocs = await db.listAll();

// With pagination
const docs = await db.listAll({ limit: 50, offset: 100 });
```

### Database Health
```typescript
// Check health
const health = await db.health();
console.log(`Status: ${health.status}`);

// Get statistics
const stats = await db.stats();
console.log(`Document count: ${stats.document_count}`);
```

## Error Handling

```typescript
import { KotaDBError, NotFoundError, ConnectionError } from 'kotadb-client';

try {
  const doc = await db.get('non-existent-id');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log('Document not found');
  } else if (error instanceof ConnectionError) {
    console.log('Failed to connect to database');
  } else if (error instanceof KotaDBError) {
    console.log(`Database error: ${error.message}`);
  } else {
    console.log(`Unexpected error: ${error}`);
  }
}
```

## Type Definitions

### Document
```typescript
interface Document {
  id: string;
  path: string;
  title: string;
  content: string;
  tags: string[];
  created_at: string;
  updated_at: string;
  size: number;
  metadata?: Record<string, any>;
}
```

### SearchResult
```typescript
interface SearchResult {
  document: Document;
  score: number;
  content_preview: string;
}
```

### QueryResult
```typescript
interface QueryResult {
  results: SearchResult[];
  total_count: number;
  query_time_ms: number;
}
```

## Browser Support

This client works in both Node.js and modern browsers. For browser usage:

```html
<!-- Using a CDN (once published) -->
<script src="https://unpkg.com/kotadb-client@latest/dist/index.js"></script>
<script>
  const db = new KotaDB.default({ url: 'http://localhost:8080' });
  // Use the client...
</script>
```

## Development

### Building
```bash
npm run build
```

### Testing
```bash
npm test
```

### Linting
```bash
npm run lint
npm run lint:fix
```

### Formatting
```bash
npm run format
```

## License

MIT License - see LICENSE file for details.

## Contributing

See CONTRIBUTING.md for contribution guidelines.

## Support

- GitHub Issues: https://github.com/jayminwest/kota-db/issues
- Documentation: https://github.com/jayminwest/kota-db/docs
