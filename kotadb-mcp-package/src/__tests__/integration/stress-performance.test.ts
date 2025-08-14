import { createTestClient, MCPTestClient, PerformanceTimer, ErrorInjectionClient } from './test-helpers';
import * as os from 'os';

describe('MCP Stress Testing and Performance', () => {
  let client: MCPTestClient;
  let perfTimer: PerformanceTimer;

  beforeAll(async () => {
    client = await createTestClient();
    perfTimer = new PerformanceTimer();
  }, 15000);

  afterAll(async () => {
    await client.cleanup();
  });

  describe('High Volume Operations', () => {
    test('should handle large batch document creation', async () => {
      const batchSize = 100;
      const startTime = Date.now();
      
      // Create documents in batches to avoid overwhelming the server
      const batchPromises = [];
      for (let batch = 0; batch < 5; batch++) {
        const batchOps = Array.from({ length: batchSize / 5 }, (_, i) => {
          const docNum = batch * (batchSize / 5) + i;
          return client.createDocument({
            path: `/batch/doc-${docNum}.md`,
            title: `Batch Document ${docNum}`,
            content: `Content for batch document ${docNum}. This contains various keywords for testing: batch, performance, stress, document-${docNum}`,
            tags: ['batch', 'stress', `batch-${batch}`],
          });
        });
        batchPromises.push(Promise.all(batchOps));
      }

      const results = await Promise.all(batchPromises);
      const allDocs = results.flat();
      const endTime = Date.now();
      
      expect(allDocs.length).toBe(batchSize);
      
      const totalTime = endTime - startTime;
      const avgTimePerDoc = totalTime / batchSize;
      
      console.log(`Created ${batchSize} documents in ${totalTime}ms (${avgTimePerDoc.toFixed(2)}ms per document)`);
      
      // Performance expectations
      expect(totalTime).toBeLessThan(30000); // Less than 30 seconds for 100 docs
      expect(avgTimePerDoc).toBeLessThan(300); // Less than 300ms per document on average
      
      // Verify all documents were created correctly
      allDocs.forEach((doc, i) => {
        expect(doc.id).toBeDefined();
        expect(doc.title).toBe(`Batch Document ${i}`);
      });
    });

    test('should handle high-frequency search operations', async () => {
      // First, ensure we have documents to search
      await Promise.all([
        client.createDocument({
          path: '/search-test/rust-guide.md',
          content: 'Comprehensive Rust programming guide with examples',
          tags: ['rust', 'programming']
        }),
        client.createDocument({
          path: '/search-test/javascript-tips.md',
          content: 'JavaScript tips and tricks for web development',
          tags: ['javascript', 'web']
        }),
        client.createDocument({
          path: '/search-test/python-tutorial.md',
          content: 'Python tutorial for beginners and advanced users',
          tags: ['python', 'tutorial']
        }),
      ]);

      // Perform many searches rapidly
      const searchQueries = [
        'rust', 'javascript', 'python', 'programming', 'web', 'tutorial',
        'guide', 'tips', 'examples', 'development'
      ];

      const numIterations = 50;
      const searchPromises = [];
      
      perfTimer.start();
      
      for (let i = 0; i < numIterations; i++) {
        const query = searchQueries[i % searchQueries.length];
        searchPromises.push(client.searchDocuments(query, 10));
      }

      const searchResults = await Promise.all(searchPromises);
      const totalSearchTime = perfTimer.end();

      // Verify all searches completed
      expect(searchResults.length).toBe(numIterations);
      
      const avgSearchTime = totalSearchTime / numIterations;
      console.log(`Completed ${numIterations} searches in ${totalSearchTime}ms (${avgSearchTime.toFixed(2)}ms per search)`);
      
      // Performance expectations
      expect(totalSearchTime).toBeLessThan(10000); // Less than 10 seconds total
      expect(avgSearchTime).toBeLessThan(200); // Less than 200ms per search on average
      
      // Verify search quality
      const rustSearches = searchResults.filter((_, i) => searchQueries[i % searchQueries.length] === 'rust');
      rustSearches.forEach(result => {
        expect(Array.isArray(result)).toBe(true);
        if (result.length > 0) {
          expect(result[0].title || result[0].content).toMatch(/rust/i);
        }
      });
    });

    test('should handle concurrent mixed operations', async () => {
      // Mix of different operation types running concurrently
      const operations = [];
      
      // Document operations
      for (let i = 0; i < 15; i++) {
        operations.push(
          client.createDocument({
            path: `/concurrent/mixed-${i}.md`,
            content: `Concurrent mixed operation test ${i}`,
            tags: ['concurrent', 'mixed']
          })
        );
      }
      
      // Search operations
      for (let i = 0; i < 10; i++) {
        operations.push(client.searchDocuments('concurrent', 5));
      }
      
      // Stats operations
      for (let i = 0; i < 5; i++) {
        operations.push(client.getStats());
      }
      
      // List operations
      for (let i = 0; i < 5; i++) {
        operations.push(client.listDocuments(10, i * 10));
      }

      perfTimer.start();
      const results = await Promise.allSettled(operations);
      const mixedOpTime = perfTimer.end();

      const successful = results.filter(r => r.status === 'fulfilled');
      const failed = results.filter(r => r.status === 'rejected');

      console.log(`Mixed operations: ${successful.length} succeeded, ${failed.length} failed in ${mixedOpTime}ms`);
      
      // Most operations should succeed
      expect(successful.length).toBeGreaterThan(operations.length * 0.8); // 80% success rate
      expect(mixedOpTime).toBeLessThan(15000); // Complete within 15 seconds
    });
  });

  describe('Memory and Resource Management', () => {
    test('should manage memory efficiently under load', async () => {
      const initialMemory = process.memoryUsage();
      console.log(`Initial memory: ${Math.round(initialMemory.heapUsed / 1024 / 1024)}MB`);
      
      // Create a substantial number of documents
      const numDocs = 75;
      const largeContent = 'Large document content. '.repeat(500); // ~10KB per document
      
      const creationPromises = Array.from({ length: numDocs }, (_, i) =>
        client.createDocument({
          path: `/memory-stress/large-doc-${i}.md`,
          title: `Large Document ${i}`,
          content: largeContent + ` Document ID: ${i}`,
          tags: ['memory', 'stress', 'large']
        })
      );

      perfTimer.start();
      const docs = await Promise.all(creationPromises);
      const creationTime = perfTimer.end();
      
      const afterCreationMemory = process.memoryUsage();
      console.log(`After creation: ${Math.round(afterCreationMemory.heapUsed / 1024 / 1024)}MB`);
      
      // Perform operations on all documents
      const operationPromises = docs.map(doc => 
        Promise.all([
          client.getDocument(doc.id),
          client.searchDocuments(`Document ID: ${docs.indexOf(doc)}`, 1),
          client.updateDocument(doc.id, doc.content + ' - Updated')
        ])
      );
      
      await Promise.all(operationPromises);
      
      const finalMemory = process.memoryUsage();
      console.log(`Final memory: ${Math.round(finalMemory.heapUsed / 1024 / 1024)}MB`);
      
      // Verify performance
      console.log(`Created and processed ${numDocs} large documents in ${creationTime}ms`);
      
      // Memory growth should be reasonable
      const memoryGrowthMB = (finalMemory.heapUsed - initialMemory.heapUsed) / 1024 / 1024;
      console.log(`Memory growth: ${memoryGrowthMB.toFixed(2)}MB`);
      
      expect(memoryGrowthMB).toBeLessThan(100); // Less than 100MB growth for the test
    });

    test('should handle file descriptor limits', async () => {
      // Test rapid file operations to stress file descriptor usage
      const numOps = 50;
      const rapidOps = [];
      
      for (let i = 0; i < numOps; i++) {
        rapidOps.push(
          client.createDocument({
            path: `/fd-test/file-${i}.md`,
            content: `File descriptor test ${i}`,
          }).then(doc => 
            client.getDocument(doc.id)
          ).then(retrieved => 
            client.deleteDocument(retrieved.id)
          )
        );
      }

      const results = await Promise.allSettled(rapidOps);
      const successful = results.filter(r => r.status === 'fulfilled');
      
      console.log(`File descriptor test: ${successful.length}/${numOps} operations succeeded`);
      
      // Most operations should succeed (some failures acceptable under stress)
      expect(successful.length).toBeGreaterThan(numOps * 0.7); // 70% success rate
      
      // Server should still be responsive
      const healthCheck = await client.getStats();
      expect(healthCheck.total_documents).toBeDefined();
    });
  });

  describe('Network and Communication Stress', () => {
    test('should handle request backpressure', async () => {
      // Send many requests rapidly without waiting
      const numRequests = 30;
      const requests = [];
      
      perfTimer.start();
      
      for (let i = 0; i < numRequests; i++) {
        requests.push(
          client.sendRequest('tools/call', {
            name: 'kotadb_stats',
            arguments: {}
          })
        );
      }
      
      const responses = await Promise.allSettled(requests);
      const backpressureTime = perfTimer.end();
      
      const successful = responses.filter(r => r.status === 'fulfilled');
      const failed = responses.filter(r => r.status === 'rejected');
      
      console.log(`Backpressure test: ${successful.length} succeeded, ${failed.length} failed in ${backpressureTime}ms`);
      
      // Most requests should succeed
      expect(successful.length).toBeGreaterThan(numRequests * 0.8);
      
      // Should handle backpressure reasonably
      expect(backpressureTime).toBeLessThan(10000); // Within 10 seconds
    });

    test('should recover from network simulation failures', async () => {
      // Use ErrorInjectionClient to simulate network issues
      const failureClient = new ErrorInjectionClient(0.3, 50); // 30% failure rate, 50ms delay
      await failureClient.initialize();
      
      try {
        // Attempt multiple operations with simulated failures
        const attempts = [];
        for (let i = 0; i < 20; i++) {
          attempts.push(
            failureClient.createDocument({
              path: `/failure-sim/doc-${i}.md`,
              content: `Failure simulation test ${i}`,
            }).catch(error => ({ error: error.message }))
          );
        }
        
        const results = await Promise.all(attempts);
        
        const successes = results.filter(r => r.id && !r.error);
        const failures = results.filter(r => r.error);
        
        console.log(`Network failure simulation: ${successes.length} succeeded, ${failures.length} failed`);
        
        // Some should succeed despite failures
        expect(successes.length).toBeGreaterThan(0);
        
        // Original client should still work fine
        const healthDoc = await client.createDocument({
          path: '/health-check-after-failures.md',
          content: 'Health check after failure simulation',
        });
        expect(healthDoc.id).toBeDefined();
        
      } finally {
        await failureClient.cleanup();
      }
    });
  });

  describe('Long-Running Operations', () => {
    test('should maintain performance over time', async () => {
      const measurements: { operation: string; time: number; memory: number }[] = [];
      const iterations = 10;
      
      for (let i = 0; i < iterations; i++) {
        const beforeMemory = process.memoryUsage().heapUsed;
        
        // Perform a standardized set of operations
        perfTimer.start();
        
        const doc = await client.createDocument({
          path: `/longevity/iteration-${i}.md`,
          title: `Longevity Test ${i}`,
          content: `Long-running test iteration ${i} with content`,
          tags: ['longevity', `iteration-${i}`]
        });
        
        await client.getDocument(doc.id);
        await client.searchDocuments('longevity', 5);
        await client.updateDocument(doc.id, doc.content + ' - updated');
        
        const opTime = perfTimer.end();
        const afterMemory = process.memoryUsage().heapUsed;
        
        measurements.push({
          operation: `iteration-${i}`,
          time: opTime,
          memory: afterMemory - beforeMemory
        });
        
        // Small delay between iterations
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      
      // Analyze performance degradation
      const avgTimeFirst3 = measurements.slice(0, 3).reduce((a, b) => a + b.time, 0) / 3;
      const avgTimeLast3 = measurements.slice(-3).reduce((a, b) => a + b.time, 0) / 3;
      
      console.log(`Longevity test - First 3 iterations: ${avgTimeFirst3.toFixed(2)}ms avg`);
      console.log(`Longevity test - Last 3 iterations: ${avgTimeLast3.toFixed(2)}ms avg`);
      
      // Performance should not degrade significantly over time
      expect(avgTimeLast3).toBeLessThan(avgTimeFirst3 * 2); // Less than 2x degradation
      
      // Memory should not grow unbounded
      const totalMemoryGrowth = measurements.reduce((sum, m) => sum + m.memory, 0);
      const avgMemoryPerOp = totalMemoryGrowth / iterations;
      
      console.log(`Average memory per operation: ${Math.round(avgMemoryPerOp / 1024)}KB`);
      expect(avgMemoryPerOp).toBeLessThan(5 * 1024 * 1024); // Less than 5MB per operation
    });

    test('should handle connection timeouts gracefully', async () => {
      // Test with longer operations and shorter timeouts
      const shortTimeoutPromises = Array.from({ length: 5 }, (_, i) =>
        client.sendRequest('tools/call', {
          name: 'kotadb_search',
          arguments: { query: 'test', limit: 100 }
        }, 100) // Very short timeout
        .catch(error => ({ timeout: true, error: error.message }))
      );
      
      const results = await Promise.all(shortTimeoutPromises);
      
      // Some might timeout, but system should remain stable
      const timeouts = results.filter((r: any) => r.timeout);
      const successes = results.filter((r: any) => !r.timeout);
      
      console.log(`Timeout test: ${successes.length} succeeded, ${timeouts.length} timed out`);
      
      // System should still be responsive after timeouts
      const healthCheck = await client.getStats();
      expect(healthCheck.total_documents).toBeDefined();
    });
  });

  describe('Resource Exhaustion Recovery', () => {
    test('should recover from simulated resource exhaustion', async () => {
      // Create many operations simultaneously to simulate resource pressure
      const pressure = Array.from({ length: 50 }, (_, i) =>
        client.createDocument({
          path: `/pressure/doc-${i}.md`,
          content: `Pressure test document ${i}`,
        }).catch(error => ({ failed: true, error: error.message }))
      );
      
      const results = await Promise.allSettled(pressure);
      const resolved = results
        .filter(r => r.status === 'fulfilled')
        .map(r => r.value);
      
      const successful = resolved.filter(r => r.id);
      const failed = resolved.filter(r => r.failed);
      
      console.log(`Resource pressure: ${successful.length} succeeded, ${failed.length} failed`);
      
      // Some operations should succeed
      expect(successful.length).toBeGreaterThan(0);
      
      // After pressure, system should recover
      await new Promise(resolve => setTimeout(resolve, 1000)); // Brief recovery time
      
      const recoveryDoc = await client.createDocument({
        path: '/recovery-test.md',
        content: 'Testing recovery after resource pressure',
      });
      
      expect(recoveryDoc.id).toBeDefined();
      
      // Should be able to perform normal operations
      const retrieved = await client.getDocument(recoveryDoc.id);
      expect(retrieved.id).toBe(recoveryDoc.id);
    });
  });
});