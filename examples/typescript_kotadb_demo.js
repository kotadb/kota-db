#!/usr/bin/env node
"use strict";
/**
 * KotaDB TypeScript Client Demo
 * =============================
 *
 * This example demonstrates the full capabilities of the KotaDB TypeScript client,
 * including document management, search operations, and error handling.
 *
 * Prerequisites:
 * 1. Start the KotaDB server:
 *    cargo run --bin kotadb serve --port 18432
 *
 * 2. Install the TypeScript client:
 *    cd clients/typescript && npm install && npm run build
 *
 * 3. Run this example:
 *    npx ts-node examples/typescript_kotadb_demo.ts
 *    OR (if compiled):
 *    node examples/typescript_kotadb_demo.js
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.main = main;
const { KotaDB, connect } = require('../clients/typescript/dist/index.js');
const { NotFoundError, ConnectionError, ValidationError, KotaDBError } = require('../clients/typescript/dist/types.js');
// Test configuration
const TEST_SERVER_URL = process.env.KOTADB_TEST_URL || 'http://localhost:18432';
const INVALID_SERVER_URL = 'http://localhost:9999'; // Non-existent server for error testing
function printSection(title) {
    console.log('\n' + '='.repeat(60));
    console.log(`  ${title}`);
    console.log('='.repeat(60));
}
function printDocument(doc, indent = '') {
    console.log(`${indent}ID: ${doc.id}`);
    console.log(`${indent}Path: ${doc.path}`);
    console.log(`${indent}Title: ${doc.title}`);
    console.log(`${indent}Tags: ${doc.tags ? doc.tags.join(', ') : 'None'}`);
    console.log(`${indent}Size: ${doc.size_bytes || 0} bytes`);
    console.log(`${indent}Created: ${new Date(doc.created_at * 1000).toISOString()}`);
    if (doc.content) {
        const content = typeof doc.content === 'string' ? doc.content : 'Binary content';
        const preview = content.length > 100 ? content.substring(0, 100) + '...' : content;
        console.log(`${indent}Content: ${preview}`);
    }
}
async function demoConnectionManagement(dbUrl = TEST_SERVER_URL) {
    printSection('Connection Management');
    // Basic connection
    console.log('\n1. Basic connection:');
    const db = new KotaDB({ url: dbUrl });
    const health = await db.health();
    console.log(`   Server status: ${health.status || 'unknown'}`);
    // Context manager pattern (using try-finally)
    console.log('\n2. Resource management pattern:');
    const tempDb = new KotaDB({ url: dbUrl });
    try {
        const stats = await tempDb.stats();
        console.log(`   Documents in database: ${stats.document_count || 0}`);
        console.log(`   Total size: ${stats.total_size_bytes || 0} bytes`);
    }
    catch (error) {
        // Stats endpoint might not be implemented
        console.log('   Stats endpoint not available (optional feature)');
    }
    // Connection with custom timeout
    console.log('\n3. Custom timeout configuration:');
    const configuredDb = new KotaDB({
        url: dbUrl,
        timeout: 10000,
        retries: 3
    });
    console.log('   Configured with custom timeout and retry settings');
    // Convenience connect function
    console.log('\n4. Convenience connect function:');
    const connectedDb = connect({ url: dbUrl });
    console.log('   Connected using convenience function');
    return db;
}
async function demoDocumentOperations(db) {
    printSection('Document Operations');
    const documentsCreated = [];
    try {
        // Create documents
        console.log('\n1. Creating documents:');
        // Technical documentation
        const doc1Id = await db.insert({
            path: '/docs/typescript-guide.md',
            title: 'TypeScript Programming Guide',
            content: `# TypeScript Programming Guide

This guide covers TypeScript best practices, including:
- Type safety and interface definitions
- Generic programming patterns
- Advanced type manipulation
- Module system and namespaces
- Integration with existing JavaScript
- Build tooling and configuration
      `,
            tags: ['typescript', 'programming', 'guide', 'documentation'],
            metadata: {
                author: 'Demo Script',
                version: '1.0',
                difficulty: 'intermediate'
            }
        });
        documentsCreated.push(doc1Id);
        console.log(`   Created: TypeScript Guide (ID: ${doc1Id.substring(0, 8)}...)`);
        // Meeting notes
        const doc2Id = await db.insert({
            path: '/meetings/2024-q1-planning.md',
            title: 'Q1 2024 Planning Meeting',
            content: `# Q1 Planning Meeting Notes

**Date**: January 15, 2024
**Attendees**: Engineering Team

## Objectives
- Review Q4 2023 performance
- Set Q1 2024 goals
- Discuss resource allocation
- Plan feature roadmap

## Action Items
1. Complete migration to new infrastructure
2. Launch beta testing program
3. Hire 2 additional engineers
4. Implement automated testing pipeline
      `,
            tags: ['meeting', 'planning', 'q1-2024', 'roadmap'],
            metadata: {
                meeting_date: '2024-01-15',
                attendee_count: 12,
                follow_up_required: true
            }
        });
        documentsCreated.push(doc2Id);
        console.log(`   Created: Meeting Notes (ID: ${doc2Id.substring(0, 8)}...)`);
        // Code snippet
        const doc3Id = await db.insert({
            path: '/snippets/async-patterns.ts',
            title: 'TypeScript Async Patterns',
            content: `// Async/await patterns in TypeScript
interface ApiResponse<T> {
  data: T;
  status: number;
  message: string;
}

async function fetchWithRetry<T>(
  url: string, 
  retries: number = 3
): Promise<ApiResponse<T>> {
  for (let attempt = 1; attempt <= retries; attempt++) {
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(\`HTTP \${response.status}\`);
      }
      const data = await response.json();
      return { data, status: response.status, message: 'Success' };
    } catch (error) {
      if (attempt === retries) throw error;
      await new Promise(resolve => setTimeout(resolve, 1000 * attempt));
    }
  }
  throw new Error('Max retries exceeded');
}
      `,
            tags: ['typescript', 'async', 'patterns', 'snippet'],
            metadata: {
                language: 'typescript',
                framework: 'none',
                tested: true
            }
        });
        documentsCreated.push(doc3Id);
        console.log(`   Created: Code Snippet (ID: ${doc3Id.substring(0, 8)}...)`);
        // Retrieve a document
        console.log('\n2. Retrieving a document:');
        const doc = await db.get(doc1Id);
        printDocument(doc, '   ');
        // Update a document
        console.log('\n3. Updating a document:');
        const updatedDoc = await db.update(doc2Id, {
            content: doc.content + '\n\n## Update: All action items completed!',
            tags: ['meeting', 'planning', 'q1-2024', 'roadmap', 'completed']
        });
        console.log('   Updated meeting notes with completion status');
        console.log(`   New tags: ${updatedDoc.tags?.join(', ') || 'None'}`);
        // List all documents (if endpoint is available)
        console.log('\n4. Listing all documents:');
        try {
            const allDocs = await db.listAll({ limit: 10 });
            console.log(`   Found ${allDocs.length} documents:`);
            for (const doc of allDocs.slice(0, 5)) { // Show first 5
                console.log(`   - ${doc.path}: ${doc.title}`);
            }
        }
        catch (error) {
            console.log('   List endpoint not available (optional feature)');
        }
    }
    catch (error) {
        if (error instanceof KotaDBError) {
            console.log(`   Error during document operations: ${error.message}`);
        }
        else {
            throw error;
        }
    }
    return documentsCreated;
}
async function demoSearchOperations(db) {
    printSection('Search Operations');
    try {
        // Text search
        console.log('\n1. Text search for "typescript":');
        const results = await db.query('typescript', { limit: 5 });
        console.log(`   Found ${results.total_count} results in ${results.query_time_ms}ms`);
        for (const [i, result] of results.results.slice(0, 3).entries()) {
            console.log(`   ${i + 1}. ${result.document.title}`);
            // Score and preview are now available from our transformation
            console.log(`      Score: ${result.score}, Preview: ${result.content_preview.substring(0, 50)}...`);
        }
        // Search with specific terms
        console.log('\n2. Search for "planning meeting":');
        const results2 = await db.query('planning meeting', { limit: 5 });
        console.log(`   Found ${results2.total_count} results`);
        for (const result of results2.results) {
            console.log(`   - ${result.document.title}`);
            if (result.document.tags && result.document.tags.length > 0) {
                console.log(`     Tags: ${result.document.tags.join(', ')}`);
            }
        }
        // Pattern-based search
        console.log('\n3. Search for code patterns:');
        const results3 = await db.query('async patterns', { limit: 5 });
        console.log(`   Found ${results3.total_count} code-related results`);
        for (const result of results3.results) {
            console.log(`   - ${result.document.path}: ${result.document.title}`);
        }
        // Search with pagination
        console.log('\n4. Paginated search results:');
        const page1 = await db.query('typescript', { limit: 2, offset: 0 });
        const page2 = await db.query('typescript', { limit: 2, offset: 2 });
        console.log(`   Page 1: ${page1.results.length} results`);
        console.log(`   Page 2: ${page2.results.length} results`);
    }
    catch (error) {
        if (error instanceof KotaDBError) {
            console.log(`   Error during search: ${error.message}`);
        }
        else {
            throw error;
        }
    }
}
async function demoBulkOperations(db, docIds) {
    printSection('Bulk Operations');
    try {
        // Bulk insert
        console.log('\n1. Bulk document creation:');
        const bulkIds = [];
        const startTime = Date.now();
        const bulkPromises = Array.from({ length: 5 }, (_, i) => db.insert({
            path: `/bulk/document-${i}.md`,
            title: `Bulk Document #${i}`,
            content: `This is bulk document ${i} created for testing bulk operations using TypeScript.`,
            tags: ['bulk', 'test', `batch-${Math.floor(i / 2)}`]
        }));
        const bulkResults = await Promise.all(bulkPromises);
        bulkIds.push(...bulkResults);
        const elapsed = Date.now() - startTime;
        console.log(`   Created 5 documents in ${elapsed}ms`);
        console.log(`   Average: ${elapsed / 5}ms per document`);
        // Bulk retrieval with pagination (if available)
        console.log('\n2. Paginated retrieval:');
        try {
            const page1 = await db.listAll({ limit: 3, offset: 0 });
            const page2 = await db.listAll({ limit: 3, offset: 3 });
            console.log(`   Page 1: ${page1.length} documents`);
            console.log(`   Page 2: ${page2.length} documents`);
        }
        catch (error) {
            console.log('   Pagination endpoint not available (optional feature)');
        }
        // Cleanup bulk documents
        console.log('\n3. Bulk deletion:');
        let deletedCount = 0;
        const deletePromises = bulkIds.map(async (docId) => {
            try {
                await db.delete(docId);
                deletedCount++;
            }
            catch (error) {
                // Continue even if some deletions fail
            }
        });
        await Promise.all(deletePromises);
        console.log(`   Deleted ${deletedCount} bulk test documents`);
    }
    catch (error) {
        if (error instanceof KotaDBError) {
            console.log(`   Error during bulk operations: ${error.message}`);
        }
        else {
            throw error;
        }
    }
}
async function demoAdvancedSearch(db) {
    printSection('Advanced Search Features');
    try {
        // Semantic search (if available)
        console.log('\n1. Semantic search:');
        try {
            const semanticResults = await db.semanticSearch('programming concepts', { limit: 3 });
            console.log(`   Found ${semanticResults.total_count || 0} semantically related results`);
            // Note: semanticResults might not have the same structure as regular search
        }
        catch (error) {
            console.log('   Semantic search not available (requires embedding model)');
        }
        // Hybrid search (if available)
        console.log('\n2. Hybrid search:');
        try {
            const hybridResults = await db.hybridSearch('software development', {
                limit: 5,
                semantic_weight: 0.6 // 60% semantic, 40% text
            });
            console.log(`   Found ${hybridResults.total_count || 0} hybrid search results`);
        }
        catch (error) {
            console.log('   Hybrid search not available (requires embedding model)');
        }
        // Empty search results
        console.log('\n3. Empty search results:');
        const emptyResults = await db.query('nonexistentquerythatwillreturnnothing12345');
        console.log(`   Empty search returned ${emptyResults.results.length} results (expected: 0)`);
    }
    catch (error) {
        if (error instanceof KotaDBError) {
            console.log(`   Error during advanced search: ${error.message}`);
        }
        else {
            throw error;
        }
    }
}
async function demoErrorHandling(db) {
    printSection('Error Handling');
    console.log('\n1. Handling not found errors:');
    try {
        await db.get('00000000-0000-0000-0000-000000000000'); // Valid UUID format but doesn't exist
    }
    catch (error) {
        if (error instanceof NotFoundError) {
            console.log(`   ✓ Correctly caught NotFoundError: ${error.message}`);
        }
        else if (error instanceof KotaDBError) {
            console.log(`   ✓ Correctly caught KotaDBError: ${error.constructor.name}`);
        }
    }
    console.log('\n2. Handling invalid document creation:');
    try {
        // Missing required field (path)
        await db.insert({
            title: 'Invalid Document',
            content: 'This should fail'
        });
    }
    catch (error) {
        if (error instanceof ValidationError) {
            console.log(`   ✓ Correctly caught validation error: ${error.message}`);
        }
        else if (error instanceof KotaDBError) {
            console.log(`   ✓ Correctly caught error: ${error.constructor.name}`);
        }
    }
    console.log('\n3. Connection error simulation:');
    try {
        const badDb = new KotaDB({ url: INVALID_SERVER_URL, timeout: 1000 });
        await badDb.health();
    }
    catch (error) {
        if (error instanceof ConnectionError) {
            console.log('   ✓ Correctly caught connection error');
        }
        else if (error instanceof KotaDBError) {
            console.log('   ✓ Correctly caught connection-related error');
        }
    }
}
async function cleanupDemoDocuments(db, docIds) {
    printSection('Cleanup');
    console.log('\nRemoving demo documents...');
    let removed = 0;
    for (const docId of docIds) {
        try {
            await db.delete(docId);
            removed++;
            console.log(`   Deleted document ${docId.substring(0, 8)}...`);
        }
        catch (error) {
            if (!(error instanceof NotFoundError)) {
                console.log(`   Error deleting ${docId.substring(0, 8)}: ${error}`);
            }
        }
    }
    console.log(`\n✓ Cleanup complete. Removed ${removed} documents.`);
}
async function main() {
    console.log(`
╔════════════════════════════════════════════════════════════╗
║           KotaDB TypeScript Client Demo                    ║
║                                                            ║
║  This demo showcases the full capabilities of KotaDB's    ║
║  TypeScript client including CRUD operations, search,     ║
║  and error handling.                                      ║
╚════════════════════════════════════════════════════════════╝
  `);
    // Check server availability
    const dbUrl = TEST_SERVER_URL;
    console.log(`Connecting to KotaDB server at ${dbUrl}...`);
    try {
        // Initialize connection
        const db = await demoConnectionManagement(dbUrl);
        // Run demos
        const docIds = await demoDocumentOperations(db);
        await demoSearchOperations(db);
        await demoBulkOperations(db, docIds);
        await demoAdvancedSearch(db);
        await demoErrorHandling(db);
        // Cleanup
        await cleanupDemoDocuments(db, docIds);
        printSection('Demo Complete!');
        console.log('\n✅ All demonstrations completed successfully!');
        console.log('\nNext steps:');
        console.log('1. Check out the TypeScript client documentation');
        console.log('2. Explore the API reference at docs/api/');
        console.log('3. Build your own KotaDB application!');
    }
    catch (error) {
        if (error instanceof ConnectionError) {
            console.log('\n❌ Error: Could not connect to KotaDB server.');
            console.log('\nPlease ensure the server is running:');
            console.log('  cargo run --bin kotadb serve --port 18432');
            process.exit(1);
        }
        else {
            console.log(`\n❌ Unexpected error: ${error}`);
            console.error(error);
            process.exit(1);
        }
    }
}
// Run the demo
if (require.main === module) {
    main().catch(console.error);
}
