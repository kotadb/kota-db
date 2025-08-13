#!/usr/bin/env python3
"""
Integration test example for KotaDB Python client.

This example demonstrates how to write integration tests that work
against a real KotaDB server, useful for CI/CD pipelines.
"""

import sys
import time
import uuid

from kotadb import (
    ClientValidationError,
    DocumentBuilder,
    KotaDB,
    QueryBuilder,
    ValidatedPath,
    ValidatedTitle,
)
from kotadb.exceptions import KotaDBError, NotFoundError


class IntegrationTestSuite:
    """Integration test suite for KotaDB client."""

    def __init__(self, db_url="http://localhost:8080"):
        """Initialize test suite with database connection."""
        self.db_url = db_url
        self.db = None
        self.test_docs = []
        self.passed = 0
        self.failed = 0
        self.test_prefix = f"test_{int(time.time())}_{uuid.uuid4().hex[:8]}"

    def setup(self):
        """Set up test environment."""
        print("🔧 Setting up integration tests...")
        print(f"Database URL: {self.db_url}")
        print(f"Test prefix: {self.test_prefix}")

        try:
            self.db = KotaDB(self.db_url)
            health = self.db.health()
            print(f"✅ Connected to KotaDB (status: {health.get('status', 'unknown')})")
            return True
        except Exception as e:
            print(f"❌ Failed to connect to database: {e}")
            return False

    def teardown(self):
        """Clean up test environment."""
        print("\n🧹 Cleaning up test data...")

        # Delete all test documents
        for doc_id in self.test_docs:
            try:
                self.db.delete(doc_id)
                print(f"✅ Deleted test document: {doc_id}")
            except NotFoundError:
                print(f"⚠️  Test document already deleted: {doc_id}")
            except Exception as e:
                print(f"❌ Failed to delete {doc_id}: {e}")

        if self.db:
            self.db.close()

        print(f"\n📊 Test Results: {self.passed} passed, {self.failed} failed")
        return self.failed == 0

    def assert_test(self, condition, test_name, error_msg=""):
        """Assert a test condition and track results."""
        if condition:
            print(f"✅ {test_name}")
            self.passed += 1
        else:
            print(f"❌ {test_name}: {error_msg}")
            self.failed += 1
        return condition

    def test_basic_crud_operations(self):
        """Test basic CRUD operations."""
        print("\n📝 Testing Basic CRUD Operations")
        print("-" * 40)

        # Test document insertion
        try:
            doc_data = {
                "path": f"/{self.test_prefix}/crud_test.md",
                "title": "CRUD Test Document",
                "content": "This is a test document for CRUD operations.",
                "tags": ["test", "crud"],
                "metadata": {"test_type": "crud", "created_by": "integration_test"},
            }

            doc_id = self.db.insert(doc_data)
            self.test_docs.append(doc_id)
            self.assert_test(
                doc_id is not None and len(doc_id) > 0,
                "Document insertion",
                "Failed to get valid document ID",
            )

            # Test document retrieval
            doc = self.db.get(doc_id)
            self.assert_test(
                doc.title == doc_data["title"],
                "Document retrieval",
                f"Title mismatch: expected {doc_data['title']}, got {doc.title}",
            )

            # Test document update
            updated_doc = self.db.update(
                doc_id,
                {"content": "Updated content for CRUD test.", "tags": ["test", "crud", "updated"]},
            )
            self.assert_test(
                "updated" in updated_doc.tags,
                "Document update",
                "Updated tag not found in document",
            )

            # Test document deletion
            self.db.delete(doc_id)
            self.test_docs.remove(doc_id)  # Don't try to delete again in cleanup

            try:
                self.db.get(doc_id)
                self.assert_test(False, "Document deletion", "Document still exists after deletion")
            except NotFoundError:
                self.assert_test(True, "Document deletion")

        except Exception as e:
            self.assert_test(False, "Basic CRUD operations", str(e))

    def test_builder_patterns(self):
        """Test builder pattern functionality."""
        print("\n🏗️  Testing Builder Patterns")
        print("-" * 40)

        try:
            # Test DocumentBuilder
            doc_id = self.db.insert_with_builder(
                DocumentBuilder()
                .path(f"/{self.test_prefix}/builder_test.md")
                .title("Builder Pattern Test")
                .content("Testing the builder pattern for type safety.")
                .add_tag("test")
                .add_tag("builder")
                .add_metadata("pattern", "builder")
            )
            self.test_docs.append(doc_id)
            self.assert_test(doc_id is not None, "DocumentBuilder insertion")

            # Test QueryBuilder
            results = self.db.query_with_builder(
                QueryBuilder().text("builder pattern").limit(5).tag_filter("builder")
            )
            self.assert_test(
                results.total_count > 0,
                "QueryBuilder search",
                f"Expected results, got {results.total_count}",
            )

            # Test UpdateBuilder
            from kotadb import UpdateBuilder

            updated_doc = self.db.update_with_builder(
                doc_id,
                UpdateBuilder().add_tag("updated").add_metadata("last_test", "builder_update"),
            )
            self.assert_test("updated" in updated_doc.tags, "UpdateBuilder update")

        except Exception as e:
            self.assert_test(False, "Builder patterns", str(e))

    def test_validated_types(self):
        """Test validated type functionality."""
        print("\n🔒 Testing Validated Types")
        print("-" * 40)

        # Test ValidatedPath
        try:
            valid_path = ValidatedPath(f"/{self.test_prefix}/valid_path.md")
            self.assert_test(True, "ValidatedPath creation")
        except ClientValidationError:
            self.assert_test(False, "ValidatedPath creation", "Valid path rejected")

        # Test invalid path rejection
        try:
            ValidatedPath("../../../etc/passwd")
            self.assert_test(False, "ValidatedPath security", "Dangerous path accepted")
        except ClientValidationError:
            self.assert_test(True, "ValidatedPath security")

        # Test ValidatedTitle
        try:
            valid_title = ValidatedTitle("Valid Test Title")
            self.assert_test(True, "ValidatedTitle creation")
        except ClientValidationError:
            self.assert_test(False, "ValidatedTitle creation", "Valid title rejected")

        # Test invalid title rejection
        try:
            ValidatedTitle("")
            self.assert_test(False, "ValidatedTitle validation", "Empty title accepted")
        except ClientValidationError:
            self.assert_test(True, "ValidatedTitle validation")

    def test_search_capabilities(self):
        """Test various search capabilities."""
        print("\n🔍 Testing Search Capabilities")
        print("-" * 40)

        # Insert test documents for searching
        test_docs = [
            {
                "path": f"/{self.test_prefix}/search_1.md",
                "title": "Python Programming Guide",
                "content": "Python is a versatile programming language used for web development, data science, and automation.",
                "tags": ["python", "programming", "guide"],
            },
            {
                "path": f"/{self.test_prefix}/search_2.md",
                "title": "Rust Systems Programming",
                "content": "Rust provides memory safety without garbage collection, making it ideal for systems programming.",
                "tags": ["rust", "systems", "programming"],
            },
        ]

        search_doc_ids = []
        for doc in test_docs:
            try:
                doc_id = self.db.insert(doc)
                search_doc_ids.append(doc_id)
                self.test_docs.append(doc_id)
            except Exception as e:
                print(f"⚠️  Failed to insert search test document: {e}")

        # Test text search
        try:
            results = self.db.query("programming", limit=10)
            self.assert_test(
                results.total_count >= len(search_doc_ids),
                "Text search",
                f"Expected at least {len(search_doc_ids)} results, got {results.total_count}",
            )
        except Exception as e:
            self.assert_test(False, "Text search", str(e))

        # Test search with builder
        try:
            results = self.db.query_with_builder(
                QueryBuilder().text("python").tag_filter("programming").limit(5)
            )
            self.assert_test(results.total_count > 0, "Search with QueryBuilder")
        except Exception as e:
            self.assert_test(False, "Search with QueryBuilder", str(e))

        # Test semantic search (if available)
        try:
            results = self.db.semantic_search("programming languages", limit=5)
            self.assert_test(True, "Semantic search availability")
        except Exception:
            print("ℹ️  Semantic search not available (expected in some configurations)")

    def test_error_handling(self):
        """Test proper error handling."""
        print("\n⚠️  Testing Error Handling")
        print("-" * 40)

        # Test getting non-existent document
        try:
            fake_id = str(uuid.uuid4())
            self.db.get(fake_id)
            self.assert_test(False, "NotFoundError handling", "Should have raised NotFoundError")
        except NotFoundError:
            self.assert_test(True, "NotFoundError handling")
        except Exception as e:
            self.assert_test(False, "NotFoundError handling", f"Wrong exception type: {e}")

        # Test updating non-existent document
        try:
            fake_id = str(uuid.uuid4())
            self.db.update(fake_id, {"title": "Should not work"})
            self.assert_test(False, "Update error handling", "Should have raised error")
        except (NotFoundError, KotaDBError):
            self.assert_test(True, "Update error handling")
        except Exception as e:
            self.assert_test(False, "Update error handling", f"Wrong exception type: {e}")

    def test_database_info(self):
        """Test database information endpoints."""
        print("\n📊 Testing Database Information")
        print("-" * 40)

        # Test health endpoint
        try:
            health = self.db.health()
            self.assert_test(isinstance(health, dict) and "status" in health, "Health endpoint")
        except Exception as e:
            self.assert_test(False, "Health endpoint", str(e))

        # Test stats endpoint
        try:
            stats = self.db.stats()
            self.assert_test(isinstance(stats, dict), "Stats endpoint")
        except Exception as e:
            self.assert_test(False, "Stats endpoint", str(e))

    def run_all_tests(self):
        """Run all integration tests."""
        print("🧪 KotaDB Python Client - Integration Test Suite")
        print("=" * 60)

        if not self.setup():
            return False

        try:
            # Run all test methods
            self.test_basic_crud_operations()
            self.test_builder_patterns()
            self.test_validated_types()
            self.test_search_capabilities()
            self.test_error_handling()
            self.test_database_info()

        finally:
            return self.teardown()


def main():
    """Run integration tests."""
    import argparse

    parser = argparse.ArgumentParser(description="KotaDB Python Client Integration Tests")
    parser.add_argument(
        "--url",
        default="http://localhost:8080",
        help="KotaDB server URL (default: http://localhost:8080)",
    )
    parser.add_argument(
        "--timeout", type=int, default=30, help="Request timeout in seconds (default: 30)"
    )

    args = parser.parse_args()

    # Run integration tests
    test_suite = IntegrationTestSuite(args.url)
    success = test_suite.run_all_tests()

    if success:
        print("\n🎉 All integration tests passed!")
        sys.exit(0)
    else:
        print("\n💥 Some integration tests failed!")
        sys.exit(1)


if __name__ == "__main__":
    main()
