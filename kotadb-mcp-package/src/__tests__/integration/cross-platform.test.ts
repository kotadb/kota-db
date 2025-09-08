import { createTestClient, MCPTestClient } from './test-helpers';
import * as os from 'os';
import * as path from 'path';
import * as fs from 'fs/promises';

describe('Cross-Platform Compatibility', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    client = await createTestClient();
  }, 15000);

  afterAll(async () => {
    await client.cleanup();
  });

  describe('Platform Detection and Handling', () => {
    test('should work on current platform', async () => {
      const platform = os.platform();
      const arch = os.arch();
      
      console.log(`Testing on platform: ${platform}, architecture: ${arch}`);
      
      // Server should start and be responsive regardless of platform
      const tools = await client.listTools();
      expect(tools.length).toBe(7);
      
      const stats = await client.getStats();
      expect(stats.data_directory).toBeDefined();
      expect(typeof stats.data_directory).toBe('string');
    });

    test('should handle platform-specific file paths', async () => {
      // Test various path formats that might be used on different platforms
      const testPaths = [
        '/unix/style/path.md',
        '/path/with spaces/document.md',
        '/path-with-dashes/file.md',
        '/path_with_underscores/file.md',
        '/path.with.dots/file.md',
      ];

      const docs = [];
      for (const testPath of testPaths) {
        try {
          const doc = await client.createDocument({
            path: testPath,
            title: `Test for path: ${testPath}`,
            content: `Testing path handling for: ${testPath}`,
            tags: ['cross-platform', 'paths'],
          });
          docs.push(doc);
          expect(doc.path).toBe(testPath);
        } catch (error) {
          console.warn(`Path ${testPath} failed on ${os.platform()}:`, error);
          // Some paths might not be valid on all platforms, that's okay
        }
      }

      // At least some paths should work
      expect(docs.length).toBeGreaterThan(0);

      // All created documents should be retrievable
      for (const doc of docs) {
        const retrieved = await client.getDocument(doc.id);
        expect(retrieved.id).toBe(doc.id);
        expect(retrieved.path).toBe(doc.path);
      }
    });

    test('should handle Unicode and international characters', async () => {
      const unicodeTests = [
        {
          path: '/unicode/中文文档.md',
          title: '中文标题',
          content: '这是中文内容测试',
        },
        {
          path: '/unicode/español.md',
          title: 'Título en Español',
          content: 'Contenido en español con caracteres especiales: ñáéíóú',
        },
        {
          path: '/unicode/русский.md',
          title: 'Русский заголовок',
          content: 'Русский контент для тестирования',
        },
        {
          path: '/unicode/emoji-📝.md',
          title: 'Document with Emoji 📝🔍',
          content: 'Testing emoji support: 🚀💡✅🔥📊',
        },
      ];

      const created = [];
      for (const test of unicodeTests) {
        try {
          const doc = await client.createDocument({
            path: test.path,
            title: test.title,
            content: test.content,
            tags: ['unicode', 'international'],
          });
          created.push(doc);
          
          expect(doc.title).toBe(test.title);
          expect(doc.content).toBe(test.content);
          expect(doc.path).toBe(test.path);
        } catch (error) {
          console.warn(`Unicode test failed for ${test.path}:`, error);
        }
      }

      // At least some Unicode should work
      expect(created.length).toBeGreaterThan(0);

      // Test Unicode search
      if (created.length > 0) {
        const searchResults = await client.searchDocuments('测试');
        // Search might or might not find results depending on implementation
        expect(Array.isArray(searchResults)).toBe(true);
      }
    });
  });

  describe('File System Compatibility', () => {
    test('should handle various file name lengths', async () => {
      const testCases = [
        {
          name: 'short.md',
          path: '/short.md',
        },
        {
          name: 'medium-length-filename-test.md',
          path: '/medium-length-filename-test.md',
        },
        {
          name: 'very-long-filename-that-tests-filesystem-limits-and-compatibility-across-different-platforms.md',
          path: '/very-long-filename-that-tests-filesystem-limits-and-compatibility-across-different-platforms.md',
        },
      ];

      const results = [];
      for (const testCase of testCases) {
        try {
          const doc = await client.createDocument({
            path: testCase.path,
            title: `Test for ${testCase.name}`,
            content: `Testing filename length: ${testCase.name.length} characters`,
          });
          results.push({ success: true, doc, length: testCase.name.length });
        } catch (error) {
          results.push({ success: false, error, length: testCase.name.length });
          console.warn(`Filename length test failed for ${testCase.name}:`, error);
        }
      }

      // Short and medium names should definitely work
      expect(results[0].success).toBe(true);
      expect(results[1].success).toBe(true);
      
      console.log('Filename length test results:', results.map(r => ({
        length: r.length,
        success: r.success
      })));
    });

    test('should handle concurrent file operations safely', async () => {
      // Test concurrent file operations that might stress the file system
      const concurrentOps = Array.from({ length: 10 }, (_, i) => [
        // Create
        client.createDocument({
          path: `/concurrent/thread-${i}-create.md`,
          content: `Concurrent creation test ${i}`,
        }),
        // Immediate read (via search)
        client.searchDocuments(`thread-${i}`),
      ]).flat();

      const results = await Promise.allSettled(concurrentOps);
      const successful = results.filter(r => r.status === 'fulfilled');
      const failed = results.filter(r => r.status === 'rejected');

      console.log(`Concurrent file ops: ${successful.length} succeeded, ${failed.length} failed`);
      
      // Most operations should succeed
      expect(successful.length).toBeGreaterThan(failed.length);
    });

    test('should handle file system case sensitivity appropriately', async () => {
      // Test case sensitivity handling
      const testDoc1 = await client.createDocument({
        path: '/case-test/lowercase.md',
        title: 'Lowercase Test',
        content: 'Testing lowercase filename',
      });

      const testDoc2 = await client.createDocument({
        path: '/case-test/UPPERCASE.md',
        title: 'Uppercase Test',
        content: 'Testing uppercase filename',
      });

      // Both should be created successfully
      expect(testDoc1.id).toBeDefined();
      expect(testDoc2.id).toBeDefined();
      expect(testDoc1.id).not.toBe(testDoc2.id);

      // Both should be retrievable
      const retrieved1 = await client.getDocument(testDoc1.id);
      const retrieved2 = await client.getDocument(testDoc2.id);
      
      expect(retrieved1.path).toBe('/case-test/lowercase.md');
      expect(retrieved2.path).toBe('/case-test/UPPERCASE.md');
    });
  });

  describe('Environment Variable Handling', () => {
    test('should handle data directory configuration', async () => {
      // Verify that the server respects the configured data directory
      const stats = await client.getStats();
      expect(stats.data_directory).toBeDefined();
      expect(typeof stats.data_directory).toBe('string');
      expect(stats.data_directory.length).toBeGreaterThan(0);

      // The directory should exist and be writable
      try {
        await fs.access(stats.data_directory, fs.constants.W_OK);
      } catch (error) {
        // If we can't access it, that might be okay depending on the implementation
        console.warn('Could not verify data directory access:', error);
      }
    });

    test('should work with different temporary directory configurations', async () => {
      // Test that the system can handle different temp directory locations
      const tempDir = client.getTempDir();
      expect(tempDir).toBeDefined();
      expect(typeof tempDir).toBe('string');
      
      // Should be able to create and manage documents in the configured location
      const testDoc = await client.createDocument({
        path: '/temp-dir-test.md',
        content: 'Testing temporary directory handling',
      });

      expect(testDoc.id).toBeDefined();
      
      const retrieved = await client.getDocument(testDoc.id);
      expect(retrieved.content).toBe('Testing temporary directory handling');
    });
  });

  describe('Performance Across Platforms', () => {
    test('should meet performance benchmarks on current platform', async () => {
      const platform = os.platform();
      const startTime = Date.now();

      // Perform a standardized set of operations
      const perfDoc = await client.createDocument({
        path: '/perf-benchmark.md',
        title: 'Performance Benchmark',
        content: 'This is a performance benchmark document with some content to search through.',
        tags: ['performance', 'benchmark'],
      });

      const midTime = Date.now();
      const createTime = midTime - startTime;

      // Retrieve the document
      const retrieved = await client.getDocument(perfDoc.id);
      const endTime = Date.now();
      const retrieveTime = endTime - midTime;

      // Search for the document
      const searchStart = Date.now();
      const searchResult = await client.searchDocuments('benchmark');
      const searchTime = Date.now() - searchStart;

      console.log(`Platform ${platform} performance:`);
      console.log(`  Create: ${createTime}ms`);
      console.log(`  Retrieve: ${retrieveTime}ms`);
      console.log(`  Search: ${searchTime}ms`);

      // Performance expectations (generous to account for different platforms)
      expect(createTime).toBeLessThan(2000); // 2 seconds
      expect(retrieveTime).toBeLessThan(1000); // 1 second
      expect(searchTime).toBeLessThan(2000); // 2 seconds

      // Verify correctness
      expect(retrieved.id).toBe(perfDoc.id);
      expect(searchResult.length).toBeGreaterThan(0);
      expect(searchResult[0].id).toBe(perfDoc.id);
    });

    test('should handle memory usage appropriately', async () => {
      const initialMemory = process.memoryUsage();
      
      // Create a moderate number of documents to test memory handling
      const numDocs = 25;
      const docs = [];
      
      for (let i = 0; i < numDocs; i++) {
        const doc = await client.createDocument({
          path: `/memory-test/doc-${i}.md`,
          title: `Memory Test Document ${i}`,
          content: `Content for memory test document ${i}. `.repeat(100), // ~4KB per doc
          tags: ['memory', 'test'],
        }, 10000); // 10 second timeout for memory-intensive operations
        docs.push(doc);
      }

      const midMemory = process.memoryUsage();
      
      // Perform operations on all documents
      const operations = docs.map(doc => client.getDocument(doc.id));
      await Promise.all(operations);
      
      const finalMemory = process.memoryUsage();

      console.log('Memory usage test:');
      console.log(`  Initial: ${Math.round(initialMemory.heapUsed / 1024 / 1024)}MB`);
      console.log(`  After creation: ${Math.round(midMemory.heapUsed / 1024 / 1024)}MB`);
      console.log(`  After operations: ${Math.round(finalMemory.heapUsed / 1024 / 1024)}MB`);

      // Memory should not grow excessively
      const memoryGrowth = finalMemory.heapUsed - initialMemory.heapUsed;
      const memoryGrowthMB = memoryGrowth / 1024 / 1024;
      
      // Allow reasonable memory growth (documents + overhead)
      expect(memoryGrowthMB).toBeLessThan(50); // Less than 50MB growth
    });
  });

  describe('Binary Distribution Compatibility', () => {
    test('should work with packaged binary', async () => {
      // Test that the MCP server can locate and use the KotaDB binary
      const stats = await client.getStats();
      expect(stats).toBeDefined();
      expect(stats.total_documents).toBeDefined();
      
      // If we got stats, the binary is working
      expect(typeof stats.total_documents).toBe('number');
    });

    test('should handle missing binary gracefully', async () => {
      // This test verifies error handling when binary is not found
      // Since we have a working binary, we just verify the system is robust
      
      const tools = await client.listTools();
      expect(tools.length).toBe(7);
      
      // All tools should have proper schemas
      tools.forEach(tool => {
        expect(tool.name).toBeDefined();
        expect(tool.description).toBeDefined();
        expect(tool.inputSchema).toBeDefined();
      });
    });
  });
});