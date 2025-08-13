# KotaDB TypeScript Client Examples

This directory contains examples demonstrating the KotaDB TypeScript/JavaScript client capabilities.

## Quick Start

Make sure you have a KotaDB server running before running these examples:

```bash
# Start KotaDB server (from project root)
cargo run --bin kotadb -- serve

# Install dependencies
npm install

# Run TypeScript examples
npx ts-node examples/basic-usage.ts
```

## Examples Overview

### 📝 [basic-usage.ts](basic-usage.ts)
**Comprehensive demonstration of core functionality**
- Basic CRUD operations (create, read, update, delete)
- Text and semantic search capabilities
- Error handling patterns
- Database health and statistics
- Connection management

```bash
npx ts-node examples/basic-usage.ts
```

**Features demonstrated:**
- Document insertion with metadata and tags
- Multiple search types (text, semantic, hybrid)
- Document retrieval and updates
- Batch operations
- Error handling for non-existent documents
- Database statistics and health checking

### 🧪 [integration-test.ts](integration-test.ts)
**Integration testing against real KotaDB server**
- Comprehensive test suite for CI/CD pipelines
- CRUD operation validation
- Search capability testing
- Concurrent operation testing
- Error handling verification
- Database information endpoints

```bash
# Run against default localhost:8080
npx ts-node examples/integration-test.ts

# Run against custom server
npx ts-node examples/integration-test.ts --url http://your-server:8080
```

**Test categories:**
- Basic CRUD operations
- Search capabilities (text, semantic, hybrid)
- Error handling and edge cases
- Database information endpoints
- Concurrent operations
- Connection reliability

## Usage Patterns

### Basic Connection and Operations

```typescript
import { KotaDB } from 'kotadb-client';

// Connect to database
const db = new KotaDB({ url: 'http://localhost:8080' });

// Insert a document
const docId = await db.insert({
    path: '/notes/meeting.md',
    title: 'Team Meeting Notes',
    content: 'Important meeting details...',
    tags: ['work', 'meeting'],
    metadata: { priority: 'high' }
});

// Search for documents
const results = await db.query('meeting notes', { limit: 10 });
console.log(`Found ${results.totalCount} documents`);
```

### Advanced Search Operations

```typescript
// Text search with filters
const textResults = await db.query('typescript programming', {
    limit: 20,
    offset: 0
});

// Semantic search (if available)
try {
    const semanticResults = await db.semanticSearch('programming concepts', {
        limit: 10
    });
    console.log('Semantic search results:', semanticResults.results);
} catch (error) {
    console.log('Semantic search not available');
}

// Hybrid search combining text and semantic
try {
    const hybridResults = await db.hybridSearch('database optimization', {
        limit: 15,
        semanticWeight: 0.7  // 70% semantic, 30% text
    });
    console.log('Hybrid search results:', hybridResults.results);
} catch (error) {
    console.log('Hybrid search not available');
}
```

### Error Handling

```typescript
import { KotaDBError } from 'kotadb-client';

try {
    const doc = await db.get('non-existent-id');
} catch (error) {
    if (error instanceof KotaDBError) {
        console.log('KotaDB-specific error:', error.message);
    } else {
        console.log('Network or other error:', error);
    }
}
```

### Document Management

```typescript
// Create a document with full metadata
const doc = await db.insert({
    path: '/projects/kotadb/README.md',
    title: 'KotaDB Project README',
    content: '# KotaDB\n\nA custom database for distributed cognition...',
    tags: ['documentation', 'project', 'readme'],
    metadata: {
        author: 'developer@example.com',
        version: '1.0.0',
        lastReviewed: new Date().toISOString()
    }
});

// Update the document
const updatedDoc = await db.update(doc, {
    content: '# KotaDB\n\nUpdated content with new features...',
    tags: ['documentation', 'project', 'readme', 'updated'],
    metadata: {
        ...doc.metadata,
        lastModified: new Date().toISOString(),
        version: '1.1.0'
    }
});

// Retrieve with error handling
try {
    const retrieved = await db.get(doc);
    console.log('Document retrieved:', retrieved.title);
} catch (error) {
    console.error('Failed to retrieve document:', error);
}
```

## Type Safety

The TypeScript client provides full type safety with TypeScript interfaces:

```typescript
import type { Document, CreateDocumentRequest, QueryResult } from 'kotadb-client';

// Type-safe document creation
const docRequest: CreateDocumentRequest = {
    path: '/typed/document.md',
    title: 'Type-safe Document',
    content: 'Content here...',
    tags: ['typescript', 'types'],
    metadata: {
        stringField: 'value',
        numberField: 42,
        booleanField: true,
        arrayField: ['item1', 'item2']
    }
};

// Type-safe query results
const results: QueryResult = await db.query('search term');
results.results.forEach((doc: Document) => {
    console.log(`Title: ${doc.title}, Tags: ${doc.tags.join(', ')}`);
});
```

## Configuration Options

```typescript
import { KotaDB } from 'kotadb-client';

// Basic connection
const db1 = new KotaDB({ url: 'http://localhost:8080' });

// With custom timeout and retry settings
const db2 = new KotaDB({ 
    url: 'http://localhost:8080',
    timeout: 30000,  // 30 seconds
    retries: 5       // Retry failed requests up to 5 times
});

// Using environment variables
// Set KOTADB_URL environment variable
const db3 = new KotaDB();  // Will use KOTADB_URL
```

## Running Examples

### Prerequisites

1. **KotaDB Server**: Make sure you have a KotaDB server running:
   ```bash
   cargo run --bin kotadb -- serve
   ```

2. **Node.js & npm**: Install dependencies:
   ```bash
   npm install
   ```

3. **TypeScript**: Examples use ts-node for direct execution:
   ```bash
   npm install -g ts-node  # If not already installed
   ```

### Running Individual Examples

```bash
# Basic usage demonstration
npx ts-node examples/basic-usage.ts

# Integration test suite
npx ts-node examples/integration-test.ts

# Integration tests against custom server
npx ts-node examples/integration-test.ts --url http://remote-server:8080
```

### Compiling to JavaScript

If you prefer to compile and run JavaScript:

```bash
# Compile TypeScript to JavaScript
npx tsc examples/basic-usage.ts --outDir dist/examples

# Run the compiled JavaScript
node dist/examples/basic-usage.js
```

## Testing Your Setup

Run the integration test to verify everything is working:

```bash
npx ts-node examples/integration-test.ts
```

This will run a comprehensive test suite and report any issues with your setup.

## Common Issues

### Connection Errors
```
❌ Failed to connect: fetch failed
```
**Solution:** Make sure KotaDB server is running on the specified port.

### Module Not Found
```
❌ Cannot find module 'kotadb-client'
```
**Solution:** Install the client: `npm install kotadb-client`

### TypeScript Compilation Errors
```
❌ Cannot find name 'require'
```
**Solution:** Make sure you have proper TypeScript configuration and ts-node installed.

## Development

When developing with the TypeScript client:

1. **Use TypeScript**: Take advantage of full type safety
2. **Handle Errors**: Wrap operations in try-catch blocks
3. **Use Async/Await**: All operations are asynchronous
4. **Check Availability**: Not all features (semantic search) may be available
5. **Test Integration**: Use the integration test as a template for your own tests

## Next Steps

1. **Start with basic-usage.ts** to understand core concepts
2. **Run integration-test.ts** to verify your setup
3. **Adapt examples** to your specific use case
4. **Build your application** using the patterns shown

For more information, see the [main TypeScript client documentation](../README.md).