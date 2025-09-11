---
tags:
- file
- kota-db
- ext_py
---
#!/usr/bin/env python3
"""
Performance test example for KotaDB Python client.

This example demonstrates how to measure and benchmark KotaDB operations
to ensure they meet performance requirements.
"""

import statistics
import time
import uuid

from kotadb import DocumentBuilder, KotaDB


class PerformanceTestSuite:
    """Performance test suite for KotaDB client."""

    def __init__(self, db_url="http://localhost:8080"):
        """Initialize performance test suite."""
        self.db_url = db_url
        self.db = None
        self.test_docs = []
        self.test_prefix = f"perf_{int(time.time())}_{uuid.uuid4().hex[:8]}"

    def setup(self):
        """Set up performance test environment."""
        print("🔧 Setting up performance tests...")
        print(f"Database URL: {self.db_url}")

        try:
            self.db = KotaDB(self.db_url)
            health = self.db.health()
            print(f"✅ Connected to KotaDB (status: {health.get('status', 'unknown')})")
            return True
        except Exception as e:
            print(f"❌ Failed to connect to database: {e}")
            return False

    def teardown(self):
        """Clean up performance test environment."""
        print("\n🧹 Cleaning up performance test data...")

        # Delete test documents in batches for efficiency
        deleted_count = 0
        for doc_id in self.test_docs:
            try:
                self.db.delete(doc_id)
                deleted_count += 1
                if deleted_count % 100 == 0:
                    print(f"  Deleted {deleted_count}/{len(self.test_docs)} documents...")
            except Exception:
                pass  # Ignore errors during cleanup

        print(f"✅ Deleted {deleted_count} test documents")

        if self.db:
            self.db.close()

    def time_operation(self, operation, *args, **kwargs):
        """Time a single operation and return (result, duration_ms)."""
        start_time = time.perf_counter()
        try:
            result = operation(*args, **kwargs)
            end_time = time.perf_counter()
            duration_ms = (end_time - start_time) * 1000
            return result, duration_ms
        except Exception as e:
            end_time = time.perf_counter()
            duration_ms = (end_time - start_time) * 1000
            raise RuntimeError(f"Operation failed after {duration_ms:.2f}ms: {e}")

    def time_multiple_operations(self, operation, count, *args, **kwargs):
        """Time multiple operations and return statistics."""
        print(f"  Running {count} operations...")
        durations = []
        results = []

        for i in range(count):
            try:
                result, duration = self.time_operation(operation, *args, **kwargs)
                durations.append(duration)
                results.append(result)

                if (i + 1) % 100 == 0:
                    avg_ms = statistics.mean(durations[-100:])
                    print(f"    Progress: {i+1}/{count} (avg last 100: {avg_ms:.2f}ms)")

            except Exception as e:
                print(f"    ❌ Operation {i+1} failed: {e}")
                continue

        if not durations:
            raise RuntimeError("All operations failed")

        return {
            "count": len(durations),
            "total_time_ms": sum(durations),
            "avg_ms": statistics.mean(durations),
            "median_ms": statistics.median(durations),
            "min_ms": min(durations),
            "max_ms": max(durations),
            "std_dev_ms": statistics.stdev(durations) if len(durations) > 1 else 0,
            "ops_per_sec": len(durations) / (sum(durations) / 1000),
            "results": results,
        }

    def print_stats(self, name, stats):
        """Print performance statistics."""
        print(f"\n📊 {name} Performance:")
        print(f"  Operations: {stats['count']}")
        print(f"  Total time: {stats['total_time_ms']:.1f}ms")
        print(f"  Average: {stats['avg_ms']:.2f}ms")
        print(f"  Median: {stats['median_ms']:.2f}ms")
        print(f"  Min: {stats['min_ms']:.2f}ms")
        print(f"  Max: {stats['max_ms']:.2f}ms")
        print(f"  Std Dev: {stats['std_dev_ms']:.2f}ms")
        print(f"  Throughput: {stats['ops_per_sec']:.1f} ops/sec")

    def test_document_insertion_performance(self, count=100):
        """Test document insertion performance."""
        print(f"\n📝 Testing Document Insertion Performance ({count} documents)")
        print("-" * 60)

        def insert_document():
            doc_data = {
                "path": f"/{self.test_prefix}/perf_{uuid.uuid4().hex[:8]}.md",
                "title": f"Performance Test Document {uuid.uuid4().hex[:8]}",
                "content": f"This is a performance test document created at {time.time()}. " * 10,
                "tags": ["performance", "test", "benchmark"],
                "metadata": {"test_type": "performance", "created_at": time.time()},
            }
            return self.db.insert(doc_data)

        try:
            stats = self.time_multiple_operations(insert_document, count)
            self.test_docs.extend(stats["results"])
            self.print_stats("Document Insertion", stats)

            # Check if we meet performance targets
            if stats["avg_ms"] < 50:  # Target: <50ms average
                print("✅ Insertion performance meets target (<50ms avg)")
            else:
                print("⚠️  Insertion performance above target (>50ms avg)")

        except Exception as e:
            print(f"❌ Document insertion performance test failed: {e}")

    def test_document_retrieval_performance(self, count=100):
        """Test document retrieval performance."""
        print(f"\n📖 Testing Document Retrieval Performance ({count} retrievals)")
        print("-" * 60)

        if len(self.test_docs) < count:
            print(f"⚠️  Only {len(self.test_docs)} documents available for retrieval test")
            count = min(count, len(self.test_docs))

        if count == 0:
            print("❌ No documents available for retrieval test")
            return

        # Select random document IDs for retrieval
        import random

        doc_ids_to_retrieve = random.sample(self.test_docs, count)

        def retrieve_document(doc_id):
            return self.db.get(doc_id)

        try:
            durations = []
            for doc_id in doc_ids_to_retrieve:
                _, duration = self.time_operation(retrieve_document, doc_id)
                durations.append(duration)

            stats = {
                "count": len(durations),
                "avg_ms": statistics.mean(durations),
                "median_ms": statistics.median(durations),
                "min_ms": min(durations),
                "max_ms": max(durations),
                "std_dev_ms": statistics.stdev(durations) if len(durations) > 1 else 0,
                "ops_per_sec": len(durations) / (sum(durations) / 1000),
                "total_time_ms": sum(durations),
            }

            self.print_stats("Document Retrieval", stats)

            # Check if we meet performance targets
            if stats["avg_ms"] < 10:  # Target: <10ms average
                print("✅ Retrieval performance meets target (<10ms avg)")
            else:
                print("⚠️  Retrieval performance above target (>10ms avg)")

        except Exception as e:
            print(f"❌ Document retrieval performance test failed: {e}")

    def test_search_performance(self, count=50):
        """Test search performance."""
        print(f"\n🔍 Testing Search Performance ({count} searches)")
        print("-" * 60)

        search_terms = [
            "performance test",
            "benchmark document",
            "test data",
            "performance",
            "document",
            "test",
        ]

        def perform_search():
            import random

            term = random.choice(search_terms)
            return self.db.query(term, limit=10)

        try:
            stats = self.time_multiple_operations(perform_search, count)
            self.print_stats("Text Search", stats)

            # Check if we meet performance targets
            if stats["avg_ms"] < 100:  # Target: <100ms average
                print("✅ Search performance meets target (<100ms avg)")
            else:
                print("⚠️  Search performance above target (>100ms avg)")

        except Exception as e:
            print(f"❌ Search performance test failed: {e}")

    def test_builder_pattern_performance(self, count=50):
        """Test builder pattern performance vs traditional approach."""
        print(f"\n🏗️  Testing Builder Pattern Performance ({count} operations)")
        print("-" * 60)

        # Test traditional approach
        def traditional_insert():
            doc_data = {
                "path": f"/{self.test_prefix}/trad_{uuid.uuid4().hex[:8]}.md",
                "title": f"Traditional Document {uuid.uuid4().hex[:8]}",
                "content": "Traditional approach document content.",
                "tags": ["traditional", "performance"],
                "metadata": {"approach": "traditional"},
            }
            return self.db.insert(doc_data)

        # Test builder approach
        def builder_insert():
            return self.db.insert_with_builder(
                DocumentBuilder()
                .path(f"/{self.test_prefix}/builder_{uuid.uuid4().hex[:8]}.md")
                .title(f"Builder Document {uuid.uuid4().hex[:8]}")
                .content("Builder pattern document content.")
                .add_tag("builder")
                .add_tag("performance")
                .add_metadata("approach", "builder")
            )

        try:
            # Test traditional approach
            trad_stats = self.time_multiple_operations(traditional_insert, count)
            self.test_docs.extend(trad_stats["results"])
            self.print_stats("Traditional Insert", trad_stats)

            # Test builder approach
            builder_stats = self.time_multiple_operations(builder_insert, count)
            self.test_docs.extend(builder_stats["results"])
            self.print_stats("Builder Insert", builder_stats)

            # Compare approaches
            print("\n📈 Builder vs Traditional Comparison:")
            overhead = (
                (builder_stats["avg_ms"] - trad_stats["avg_ms"]) / trad_stats["avg_ms"]
            ) * 100
            print(f"  Traditional avg: {trad_stats['avg_ms']:.2f}ms")
            print(f"  Builder avg: {builder_stats['avg_ms']:.2f}ms")
            print(f"  Builder overhead: {overhead:+.1f}%")

            if overhead < 20:  # Accept up to 20% overhead for type safety
                print("✅ Builder pattern overhead acceptable (<20%)")
            else:
                print("⚠️  Builder pattern overhead high (>20%)")

        except Exception as e:
            print(f"❌ Builder pattern performance test failed: {e}")

    def test_bulk_operations_performance(self, batch_size=10):
        """Test bulk operations performance."""
        print(f"\n📦 Testing Bulk Operations Performance (batch size: {batch_size})")
        print("-" * 60)

        def bulk_insert():
            docs = []
            for i in range(batch_size):
                docs.append(
                    {
                        "path": f"/{self.test_prefix}/bulk_{i}_{uuid.uuid4().hex[:8]}.md",
                        "title": f"Bulk Document {i}",
                        "content": f"Bulk document {i} content.",
                        "tags": ["bulk", "performance"],
                        "metadata": {"batch": "true", "index": i},
                    }
                )

            # Insert documents one by one (simulating bulk)
            doc_ids = []
            for doc in docs:
                doc_id = self.db.insert(doc)
                doc_ids.append(doc_id)
            return doc_ids

        try:
            stats = self.time_multiple_operations(bulk_insert, 10)  # 10 batches

            # Flatten the results
            all_doc_ids = []
            for batch in stats["results"]:
                all_doc_ids.extend(batch)
            self.test_docs.extend(all_doc_ids)

            # Calculate per-document statistics
            docs_per_batch = batch_size
            total_docs = stats["count"] * docs_per_batch
            total_time_sec = stats["total_time_ms"] / 1000
            docs_per_sec = total_docs / total_time_sec
            ms_per_doc = stats["avg_ms"] / docs_per_batch

            print("\n📊 Bulk Operations Performance:")
            print(f"  Batches: {stats['count']}")
            print(f"  Documents per batch: {docs_per_batch}")
            print(f"  Total documents: {total_docs}")
            print(f"  Avg batch time: {stats['avg_ms']:.2f}ms")
            print(f"  Avg per document: {ms_per_doc:.2f}ms")
            print(f"  Throughput: {docs_per_sec:.1f} docs/sec")

            if docs_per_sec > 100:  # Target: >100 docs/sec
                print("✅ Bulk performance meets target (>100 docs/sec)")
            else:
                print("⚠️  Bulk performance below target (<100 docs/sec)")

        except Exception as e:
            print(f"❌ Bulk operations performance test failed: {e}")

    def run_all_tests(self):
        """Run all performance tests."""
        print("⚡ KotaDB Python Client - Performance Test Suite")
        print("=" * 70)

        if not self.setup():
            return False

        try:
            # Run performance tests
            self.test_document_insertion_performance(100)
            self.test_document_retrieval_performance(100)
            self.test_search_performance(50)
            self.test_builder_pattern_performance(50)
            self.test_bulk_operations_performance(10)

            print("\n🎯 Performance Test Summary:")
            print("=" * 50)
            print("Target Performance Metrics:")
            print("  - Document insertion: <50ms average")
            print("  - Document retrieval: <10ms average")
            print("  - Text search: <100ms average")
            print("  - Builder overhead: <20% vs traditional")
            print("  - Bulk operations: >100 docs/sec")
            print("\nCheck individual test results above for actual performance.")

        finally:
            self.teardown()

        return True


def main():
    """Run performance tests."""
    import argparse

    parser = argparse.ArgumentParser(description="KotaDB Python Client Performance Tests")
    parser.add_argument(
        "--url",
        default="http://localhost:8080",
        help="KotaDB server URL (default: http://localhost:8080)",
    )
    parser.add_argument(
        "--insert-count",
        type=int,
        default=100,
        help="Number of documents to insert for testing (default: 100)",
    )
    parser.add_argument(
        "--search-count",
        type=int,
        default=50,
        help="Number of search operations to perform (default: 50)",
    )

    args = parser.parse_args()

    # Run performance tests
    test_suite = PerformanceTestSuite(args.url)
    success = test_suite.run_all_tests()

    if success:
        print("\n🎉 Performance tests completed!")
    else:
        print("\n💥 Performance tests failed!")


if __name__ == "__main__":
    main()
