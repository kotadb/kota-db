#!/usr/bin/env npx ts-node
/**
 * Basic usage example for KotaDB TypeScript client.
 * 
 * This example demonstrates the core functionality of the KotaDB client
 * including document insertion, searching, and management.
 */

import { KotaDB } from '../src';
import type { Document, CreateDocumentRequest } from '../src/types';

async function main() {
    console.log('🎯 KotaDB TypeScript Client - Basic Usage Example');
    console.log('='.repeat(60));

    // Connect to KotaDB (make sure server is running on localhost:8080)
    let db: KotaDB;
    try {
        db = new KotaDB({ url: 'http://localhost:8080' });
        console.log('✅ Connected to KotaDB');
    } catch (error) {
        console.error('❌ Failed to connect:', error);
        console.error('Make sure KotaDB server is running on localhost:8080');
        return;
    }

    const createdDocIds: string[] = [];

    try {
        // Check database health
        const health = await db.health();
        console.log(`Database status: ${health.status || 'unknown'}`);

        // Insert some sample documents
        console.log('\n📝 Inserting sample documents...');

        const docs: CreateDocumentRequest[] = [
            {
                path: '/docs/typescript-guide.md',
                title: 'TypeScript Programming Guide',
                content: 'TypeScript is a strongly typed programming language that builds on JavaScript, giving you better tooling at any scale.',
                tags: ['typescript', 'programming', 'guide', 'javascript'],
                metadata: { author: 'typescript-expert', difficulty: 'intermediate' }
            },
            {
                path: '/docs/web-development.md',
                title: 'Modern Web Development',
                content: 'Modern web development combines TypeScript, React, Node.js, and other tools to create robust applications.',
                tags: ['web', 'development', 'typescript', 'react'],
                metadata: { author: 'web-dev', difficulty: 'advanced' }
            },
            {
                path: '/notes/project-meeting.md',
                title: 'Project Planning Meeting',
                content: 'Discussed the roadmap for Q2 2024. Key focus areas include TypeScript client improvements and performance optimization.',
                tags: ['meeting', 'planning', 'roadmap', '2024'],
                metadata: { meeting_type: 'planning', duration_minutes: 60 }
            }
        ];

        for (const doc of docs) {
            try {
                const docId = await db.insert(doc);
                createdDocIds.push(docId);
                console.log(`  ✅ Created document: ${doc.title} (ID: ${docId})`);
            } catch (error) {
                console.error(`  ❌ Failed to create ${doc.title}:`, error);
            }
        }

        // Perform text search
        console.log('\n🔍 Performing text search...');
        try {
            const results = await db.query('typescript programming', { limit: 5 });
            console.log(`Found ${results.totalCount} results:`);
            for (const doc of results.results) {
                console.log(`  - ${doc.title}`);
                console.log(`    Tags: ${doc.tags.join(', ')}`);
            }
        } catch (error) {
            console.error('❌ Text search failed:', error);
        }

        // Perform semantic search (if enabled)
        console.log('\n🧠 Attempting semantic search...');
        try {
            const semanticResults = await db.semanticSearch('programming languages and type safety', { limit: 3 });
            console.log(`Found ${semanticResults.totalCount} semantic results:`);
            for (const doc of semanticResults.results) {
                console.log(`  - ${doc.title}`);
            }
        } catch (error) {
            console.log(`  ℹ️  Semantic search not available: ${error}`);
        }

        // Get a specific document
        console.log('\n📄 Retrieving specific document...');
        if (createdDocIds.length > 0) {
            try {
                const doc = await db.get(createdDocIds[0]);
                console.log(`Retrieved: ${doc.title}`);
                console.log(`Tags: ${doc.tags.join(', ')}`);
                console.log(`Size: ${doc.size} bytes`);
                console.log(`Created: ${new Date(doc.createdAt).toISOString()}`);
            } catch (error) {
                console.error('❌ Failed to retrieve document:', error);
            }
        }

        // Update a document
        console.log('\n✏️  Updating document...');
        if (createdDocIds.length > 0) {
            try {
                const doc = await db.get(createdDocIds[0]);
                const updatedDoc = await db.update(createdDocIds[0], {
                    content: doc.content + '\n\nUPDATE: Added TypeScript examples and additional resources.',
                    tags: [...doc.tags, 'updated']
                });
                console.log(`Updated document size: ${updatedDoc.size} bytes`);
            } catch (error) {
                console.error('❌ Failed to update document:', error);
            }
        }

        // List all documents
        console.log('\n📋 Listing documents...');
        try {
            const allDocs = await db.listAll({ limit: 10 });
            console.log(`Total documents: ${allDocs.length}`);
            for (const doc of allDocs.slice(0, 5)) {
                console.log(`  - ${doc.title} (${doc.path})`);
            }
            if (allDocs.length > 5) {
                console.log(`  ... and ${allDocs.length - 5} more`);
            }
        } catch (error) {
            console.error('❌ Failed to list documents:', error);
        }

        // Get database statistics
        console.log('\n📊 Database statistics...');
        try {
            const stats = await db.stats();
            console.log(`Document count: ${stats.documentCount || 'unknown'}`);
            console.log(`Total size: ${stats.totalSizeBytes || 'unknown'} bytes`);
        } catch (error) {
            console.log(`Stats not available: ${error}`);
        }

        // Demonstrate error handling
        console.log('\n⚠️  Testing error handling...');
        try {
            await db.get('non-existent-id');
            console.log('❌ Should have thrown an error');
        } catch (error) {
            console.log('✅ Correctly handled non-existent document error');
        }

        console.log('\n✅ Basic usage example completed successfully!');

    } catch (error) {
        console.error('❌ Unexpected error:', error);
    } finally {
        // Clean up test documents
        if (createdDocIds.length > 0) {
            console.log('\n🗑️  Cleaning up test documents...');
            for (const docId of createdDocIds) {
                try {
                    await db.delete(docId);
                    console.log(`  ✅ Deleted document: ${docId}`);
                } catch (error) {
                    console.log(`  ⚠️  Failed to delete ${docId}: ${error}`);
                }
            }
        }
    }
}

// Run the example
if (require.main === module) {
    main().catch(console.error);
}