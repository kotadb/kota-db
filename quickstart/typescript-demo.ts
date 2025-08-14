#!/usr/bin/env ts-node
/**
 * KotaDB TypeScript Client Demo - 60 Second Quick Start
 * Demonstrates all core features with full type safety.
 */

import { KotaDB, DocumentBuilder, QueryBuilder, type Document, type QueryResult } from 'kotadb-client';

interface Stats {
    document_count?: number;
    [key: string]: any;
}

async function main(): Promise<void> {
    console.log('🚀 KotaDB TypeScript Demo - All Core Features');
    console.log('='.repeat(52));
    
    // Connect to KotaDB server
    const kotadbUrl = process.env.KOTADB_URL || 'http://localhost:8080';
    console.log(`📡 Connecting to KotaDB at ${kotadbUrl}`);
    
    const db = new KotaDB({ url: kotadbUrl });
    
    // Test connection
    try {
        const stats: Stats = await db.stats();
        console.log(`✅ Connected! Database has ${stats.document_count || 0} documents`);
    } catch (error) {
        console.log(`❌ Connection failed: ${error}`);
        return;
    }
    
    console.log('\n1️⃣ DOCUMENT CREATION (Builder Pattern)');
    console.log('-'.repeat(40));
    
    // Create sample documents with builder pattern (type-safe)
    const sampleDocs: string[] = [];
    
    // Document 1: Programming guide
    const doc1Id = await db.insertWithBuilder(
        new DocumentBuilder()
            .path('/guides/typescript-async.md')
            .title('TypeScript Async Programming')
            .content(`# TypeScript Async Programming

Modern JavaScript and TypeScript provide powerful async patterns.

## Key Concepts:
- Promises for single async operations
- Async/await for readable async code
- Proper error handling with try/catch

## Examples:
\`\`\`typescript
async function fetchData(): Promise<Data> {
    try {
        const response = await fetch('/api/data');
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch data:', error);
        throw error;
    }
}
\`\`\`
`)
            .addTag('typescript')
            .addTag('programming')
            .addTag('async')
    );
    sampleDocs.push(doc1Id);
    
    // Document 2: Meeting notes
    const doc2Id = await db.insertWithBuilder(
        new DocumentBuilder()
            .path('/meetings/2024-08-14-planning.md')
            .title('Sprint Planning - Aug 14')
            .content(`# Sprint Planning - August 14, 2024

## Team
- Alice (Product Manager)
- Bob (Backend Engineer)
- Carol (Frontend Engineer)
- David (DevOps)

## Sprint Goals
- Complete KotaDB TypeScript client
- Implement user authentication
- Deploy staging environment

## Stories
- [TASK-001] KotaDB client library integration
- [TASK-002] User login/logout flows  
- [TASK-003] Docker deployment pipeline

## Definition of Done
- All tests passing
- Code review completed
- Documentation updated
`)
            .addTag('meeting')
            .addTag('planning')
            .addTag('sprint')
    );
    sampleDocs.push(doc2Id);
    
    // Document 3: Technical note
    const doc3Id = await db.insertWithBuilder(
        new DocumentBuilder()
            .path('/technical/kotadb-integration.md')
            .title('KotaDB Integration Notes')
            .content(`# KotaDB Integration Notes

## Architecture Overview
KotaDB provides a unified interface for document storage and search.

## Client Libraries
- **Rust**: Full feature access, compile-time safety
- **Python**: Runtime validation, builder patterns  
- **TypeScript**: Full type safety, modern async/await
- **Go**: Basic operations (in development)

## Performance Characteristics
- Document insertion: <1ms average
- Text search: <10ms for most queries
- Concurrent access: 1000+ ops/sec
- Memory usage: Efficient with caching

## Integration Patterns
- Use builders for type safety
- Handle errors gracefully
- Leverage connection pooling
- Monitor performance metrics
`)
            .addTag('technical')
            .addTag('integration')
            .addTag('kotadb')
    );
    sampleDocs.push(doc3Id);
    
    console.log(`✅ Created ${sampleDocs.length} documents with builder pattern`);
    
    console.log('\n2️⃣ DOCUMENT RETRIEVAL');
    console.log('-'.repeat(40));
    
    // Get documents back
    for (let i = 0; i < sampleDocs.length; i++) {
        try {
            const doc: Document = await db.get(sampleDocs[i]);
            console.log(`📄 Doc ${i + 1}: '${doc.title}' - ${doc.content.length} chars`);
        } catch (error) {
            console.log(`❌ Failed to retrieve doc ${i + 1}: ${error}`);
        }
    }
    
    console.log('\n3️⃣ FULL-TEXT SEARCH');
    console.log('-'.repeat(40));
    
    // Test different search queries
    const searchQueries = [
        'typescript programming',
        'kotadb integration', 
        'meeting planning',
        'async patterns'
    ];
    
    for (const query of searchQueries) {
        try {
            const results: QueryResult = await db.query(query, { limit: 3 });
            console.log(`🔍 '${query}': ${results.results.length} results`);
            
            for (const doc of results.results.slice(0, 2)) {
                console.log(`   - ${doc.title || 'No title'}`);
            }
        } catch (error) {
            console.log(`❌ Search '${query}' failed: ${error}`);
        }
    }
    
    console.log('\n4️⃣ STRUCTURED QUERIES (Builder Pattern)');
    console.log('-'.repeat(40));
    
    // Use QueryBuilder for type-safe queries
    try {
        const results: QueryResult = await db.queryWithBuilder(
            new QueryBuilder()
                .text('integration')
                .tagFilter('technical')
                .limit(5)
        );
        console.log(`🎯 Structured query: ${results.results.length} results`);
        
        for (const doc of results.results) {
            const tags = doc.tags.join(', ');
            console.log(`   - ${doc.title || 'No title'} [tags: ${tags}]`);
        }
    } catch (error) {
        console.log(`❌ Structured query failed: ${error}`);
    }
    
    console.log('\n5️⃣ DOCUMENT UPDATES');
    console.log('-'.repeat(40));
    
    // Update first document
    try {
        const originalDoc = await db.get(sampleDocs[0]);
        const updatedDoc = await db.update(sampleDocs[0], {
            content: originalDoc.content + '\n\n## Updated Content\nThis document was updated via the TypeScript client demo!',
            tags: [...originalDoc.tags, 'updated', 'demo']
        });
        console.log('✅ Document updated successfully');
        
        // Verify update
        const retrieved = await db.get(sampleDocs[0]);
        console.log(`📄 Updated doc has ${retrieved.tags.length} tags`);
    } catch (error) {
        console.log(`❌ Update failed: ${error}`);
    }
    
    console.log('\n6️⃣ PERFORMANCE TEST');
    console.log('-'.repeat(40));
    
    // Quick performance test
    const startTime = Date.now();
    const perfDocs: string[] = [];
    
    for (let i = 0; i < 10; i++) {
        const docId = await db.insert({
            path: `/perf-test/doc-${i.toString().padStart(3, '0')}.md`,
            title: `Performance Test Document ${i}`,
            content: `This is performance test document number ${i}. `.repeat(10),
            tags: ['performance', 'test', `batch-${Math.floor(i / 5)}`]
        });
        perfDocs.push(docId);
    }
    
    const insertTime = (Date.now() - startTime) / 1000;
    
    // Test query performance
    const searchStart = Date.now();
    const results = await db.query('performance test', { limit: 20 });
    const searchTime = (Date.now() - searchStart) / 1000;
    
    console.log('⚡ Performance:');
    console.log(`   - 10 inserts: ${insertTime.toFixed(3)}s (${(10 / insertTime).toFixed(1)} ops/sec)`);
    console.log(`   - 1 search: ${searchTime.toFixed(3)}s (${(1000 * searchTime).toFixed(1)}ms)`);
    console.log(`   - Found: ${results.results.length} documents`);
    
    console.log('\n7️⃣ DATABASE STATISTICS');
    console.log('-'.repeat(40));
    
    try {
        const finalStats: Stats = await db.stats();
        console.log('📊 Final Statistics:');
        for (const [key, value] of Object.entries(finalStats)) {
            console.log(`   - ${key}: ${value}`);
        }
    } catch (error) {
        console.log(`❌ Stats failed: ${error}`);
    }
    
    console.log('\n🎉 DEMO COMPLETE!');
    console.log('='.repeat(52));
    console.log('✅ All KotaDB core features demonstrated:');
    console.log('   - Document CRUD with builder patterns');
    console.log('   - Full-text search with trigram index');
    console.log('   - Structured queries with filters');
    console.log('   - Full TypeScript type safety');
    console.log('   - High-performance operations');
    console.log('\n📚 Next steps:');
    console.log('   - Install client: npm install kotadb-client');
    console.log('   - Try examples: see examples/ directory');
    console.log('   - Read docs: visit documentation');
    console.log('   - Build your app!');
}

// Run the demo
if (require.main === module) {
    main().catch(console.error);
}