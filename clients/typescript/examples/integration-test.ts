#!/usr/bin/env npx ts-node
/**
 * Integration test example for KotaDB TypeScript client.
 * 
 * This example demonstrates how to write integration tests that work
 * against a real KotaDB server, useful for CI/CD pipelines.
 */

import { KotaDB } from '../src';
import type { Document, CreateDocumentRequest } from '../src/types';

interface TestResult {
    name: string;
    passed: boolean;
    error?: string;
}

class IntegrationTestSuite {
    private db: KotaDB;
    private testDocs: string[] = [];
    private results: TestResult[] = [];
    private testPrefix: string;

    constructor(private dbUrl: string = 'http://localhost:8080') {
        this.db = new KotaDB({ url: dbUrl });
        this.testPrefix = `test_${Date.now()}_${Math.random().toString(36).substr(2, 8)}`;
    }

    private async assert(condition: boolean, testName: string, errorMsg?: string): Promise<boolean> {
        if (condition) {
            console.log(`✅ ${testName}`);
            this.results.push({ name: testName, passed: true });
            return true;
        } else {
            console.log(`❌ ${testName}: ${errorMsg || 'Assertion failed'}`);
            this.results.push({ name: testName, passed: false, error: errorMsg });
            return false;
        }
    }

    async setup(): Promise<boolean> {
        console.log(`🔧 Setting up integration tests...`);
        console.log(`Database URL: ${this.dbUrl}`);
        console.log(`Test prefix: ${this.testPrefix}`);

        try {
            const health = await this.db.health();
            console.log(`✅ Connected to KotaDB (status: ${health.status || 'unknown'})`);
            return true;
        } catch (error) {
            console.error(`❌ Failed to connect to database: ${error}`);
            return false;
        }
    }

    async teardown(): Promise<boolean> {
        console.log(`\n🧹 Cleaning up test data...`);

        // Delete all test documents
        for (const docId of this.testDocs) {
            try {
                await this.db.delete(docId);
                console.log(`✅ Deleted test document: ${docId}`);
            } catch (error) {
                console.log(`⚠️  Test document already deleted: ${docId}`);
            }
        }

        // Print test summary
        const passed = this.results.filter(r => r.passed).length;
        const failed = this.results.filter(r => !r.passed).length;
        console.log(`\n📊 Test Results: ${passed} passed, ${failed} failed`);

        if (failed > 0) {
            console.log('\nFailed tests:');
            this.results.filter(r => !r.passed).forEach(r => {
                console.log(`  - ${r.name}: ${r.error}`);
            });
        }

        return failed === 0;
    }

    async testBasicCrudOperations(): Promise<void> {
        console.log('\n📝 Testing Basic CRUD Operations');
        console.log('-'.repeat(40));

        try {
            // Test document insertion
            const docData: CreateDocumentRequest = {
                path: `/${this.testPrefix}/crud_test.md`,
                title: 'CRUD Test Document',
                content: 'This is a test document for CRUD operations.',
                tags: ['test', 'crud'],
                metadata: { testType: 'crud', createdBy: 'integration_test' }
            };

            const docId = await this.db.insert(docData);
            this.testDocs.push(docId);
            await this.assert(
                docId != null && docId.length > 0,
                'Document insertion',
                'Failed to get valid document ID'
            );

            // Test document retrieval
            const doc = await this.db.get(docId);
            await this.assert(
                doc.title === docData.title,
                'Document retrieval',
                `Title mismatch: expected ${docData.title}, got ${doc.title}`
            );

            // Test document update
            const updatedDoc = await this.db.update(docId, {
                content: 'Updated content for CRUD test.',
                tags: ['test', 'crud', 'updated']
            });
            await this.assert(
                updatedDoc.tags.includes('updated'),
                'Document update',
                'Updated tag not found in document'
            );

            // Test document deletion
            await this.db.delete(docId);
            this.testDocs = this.testDocs.filter(id => id !== docId); // Don't try to delete again

            try {
                await this.db.get(docId);
                await this.assert(false, 'Document deletion', 'Document still exists after deletion');
            } catch (error) {
                // Expected - document should not be found
                await this.assert(true, 'Document deletion');
            }

        } catch (error) {
            await this.assert(false, 'Basic CRUD operations', String(error));
        }
    }

    async testSearchCapabilities(): Promise<void> {
        console.log('\n🔍 Testing Search Capabilities');
        console.log('-'.repeat(40));

        // Insert test documents for searching
        const testDocs: CreateDocumentRequest[] = [
            {
                path: `/${this.testPrefix}/search_1.md`,
                title: 'TypeScript Programming Guide',
                content: 'TypeScript is a powerful programming language that builds on JavaScript by adding static type definitions.',
                tags: ['typescript', 'programming', 'guide']
            },
            {
                path: `/${this.testPrefix}/search_2.md`,
                title: 'JavaScript Fundamentals',
                content: 'JavaScript is a versatile programming language that powers the web and many server applications.',
                tags: ['javascript', 'programming', 'fundamentals']
            }
        ];

        const searchDocIds: string[] = [];
        for (const doc of testDocs) {
            try {
                const docId = await this.db.insert(doc);
                searchDocIds.push(docId);
                this.testDocs.push(docId);
            } catch (error) {
                console.log(`⚠️  Failed to insert search test document: ${error}`);
            }
        }

        // Test text search
        try {
            const results = await this.db.query('programming', { limit: 10 });
            await this.assert(
                results.totalCount >= searchDocIds.length,
                'Text search',
                `Expected at least ${searchDocIds.length} results, got ${results.totalCount}`
            );
        } catch (error) {
            await this.assert(false, 'Text search', String(error));
        }

        // Test semantic search (if available)
        try {
            const results = await this.db.semanticSearch('programming languages', { limit: 5 });
            await this.assert(true, 'Semantic search availability');
        } catch (error) {
            console.log('ℹ️  Semantic search not available (expected in some configurations)');
        }

        // Test hybrid search (if available)
        try {
            const results = await this.db.hybridSearch('typescript javascript', { 
                limit: 5, 
                semanticWeight: 0.7 
            });
            await this.assert(true, 'Hybrid search availability');
        } catch (error) {
            console.log('ℹ️  Hybrid search not available (expected in some configurations)');
        }
    }

    async testErrorHandling(): Promise<void> {
        console.log('\n⚠️  Testing Error Handling');
        console.log('-'.repeat(40));

        // Test getting non-existent document
        try {
            const fakeId = 'a'.repeat(36); // Fake UUID-like string
            await this.db.get(fakeId);
            await this.assert(false, 'NotFound error handling', 'Should have thrown an error');
        } catch (error) {
            await this.assert(true, 'NotFound error handling');
        }

        // Test updating non-existent document
        try {
            const fakeId = 'b'.repeat(36);
            await this.db.update(fakeId, { title: 'Should not work' });
            await this.assert(false, 'Update error handling', 'Should have thrown an error');
        } catch (error) {
            await this.assert(true, 'Update error handling');
        }

        // Test deleting non-existent document
        try {
            const fakeId = 'c'.repeat(36);
            await this.db.delete(fakeId);
            await this.assert(false, 'Delete error handling', 'Should have thrown an error');
        } catch (error) {
            await this.assert(true, 'Delete error handling');
        }
    }

    async testDatabaseInfo(): Promise<void> {
        console.log('\n📊 Testing Database Information');
        console.log('-'.repeat(40));

        // Test health endpoint
        try {
            const health = await this.db.health();
            await this.assert(
                typeof health === 'object' && 'status' in health,
                'Health endpoint'
            );
        } catch (error) {
            await this.assert(false, 'Health endpoint', String(error));
        }

        // Test stats endpoint
        try {
            const stats = await this.db.stats();
            await this.assert(
                typeof stats === 'object',
                'Stats endpoint'
            );
        } catch (error) {
            await this.assert(false, 'Stats endpoint', String(error));
        }
    }

    async testConcurrentOperations(): Promise<void> {
        console.log('\n🔄 Testing Concurrent Operations');
        console.log('-'.repeat(40));

        // Test concurrent inserts
        try {
            const concurrentDocs: CreateDocumentRequest[] = [];
            for (let i = 0; i < 5; i++) {
                concurrentDocs.push({
                    path: `/${this.testPrefix}/concurrent_${i}.md`,
                    title: `Concurrent Document ${i}`,
                    content: `Content for concurrent document ${i}`,
                    tags: ['concurrent', 'test'],
                    metadata: { index: i }
                });
            }

            const insertPromises = concurrentDocs.map(doc => this.db.insert(doc));
            const docIds = await Promise.all(insertPromises);
            this.testDocs.push(...docIds);

            await this.assert(
                docIds.length === concurrentDocs.length && docIds.every(id => id && id.length > 0),
                'Concurrent insertions',
                `Expected ${concurrentDocs.length} valid IDs, got ${docIds.filter(id => id && id.length > 0).length}`
            );

            // Test concurrent retrieval
            const retrievePromises = docIds.map(id => this.db.get(id));
            const retrievedDocs = await Promise.all(retrievePromises);

            await this.assert(
                retrievedDocs.length === docIds.length,
                'Concurrent retrievals',
                `Expected ${docIds.length} documents, got ${retrievedDocs.length}`
            );

        } catch (error) {
            await this.assert(false, 'Concurrent operations', String(error));
        }
    }

    async runAllTests(): Promise<boolean> {
        console.log('🧪 KotaDB TypeScript Client - Integration Test Suite');
        console.log('='.repeat(70));

        if (!await this.setup()) {
            return false;
        }

        try {
            await this.testBasicCrudOperations();
            await this.testSearchCapabilities();
            await this.testErrorHandling();
            await this.testDatabaseInfo();
            await this.testConcurrentOperations();
        } finally {
            return await this.teardown();
        }
    }
}

async function main() {
    const args = process.argv.slice(2);
    const urlIndex = args.indexOf('--url');
    const dbUrl = urlIndex !== -1 && args[urlIndex + 1] ? args[urlIndex + 1] : 'http://localhost:8080';

    console.log(`Running integration tests against: ${dbUrl}`);

    const testSuite = new IntegrationTestSuite(dbUrl);
    const success = await testSuite.runAllTests();

    if (success) {
        console.log('\n🎉 All integration tests passed!');
        process.exit(0);
    } else {
        console.log('\n💥 Some integration tests failed!');
        process.exit(1);
    }
}

// Run the tests
if (require.main === module) {
    main().catch(error => {
        console.error('❌ Test suite failed:', error);
        process.exit(1);
    });
}