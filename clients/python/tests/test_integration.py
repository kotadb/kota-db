"""
Integration tests for KotaDB Python client against a real server.

These tests require a running KotaDB server on localhost:8080.
"""

import os
import time
import uuid
from typing import Generator

import pytest
import requests

from kotadb import KotaDB
from kotadb.exceptions import NotFoundError
from kotadb.types import CreateDocumentRequest, Document


def server_is_running(url: str = "http://localhost:8080") -> bool:
    """Check if the KotaDB server is running."""
    try:
        response = requests.get(f"{url}/health", timeout=1)
        return response.status_code == 200
    except (requests.ConnectionError, requests.Timeout):
        return False


@pytest.fixture(scope="session")
def kotadb_server() -> Generator[str, None, None]:
    """Ensure KotaDB server is running for integration tests."""
    server_url = "http://localhost:8080"

    if not server_is_running(server_url):
        pytest.skip("KotaDB server not running. Start with: cargo run --bin kotadb -- serve")

    yield server_url


@pytest.fixture
def client(kotadb_server: str) -> Generator[KotaDB, None, None]:
    """Create a KotaDB client for testing."""
    db = KotaDB(kotadb_server)
    yield db
    db.close()


@pytest.fixture
def test_document() -> dict:
    """Create test document data."""
    return {
        "path": f"/test/doc_{uuid.uuid4()}.md",
        "title": f"Test Document {uuid.uuid4()}",
        "content": "This is test content for integration testing.",
        "tags": ["test", "integration", "automated"],
    }


@pytest.mark.integration
class TestServerConnection:
    """Test server connection and health checks."""

    def test_health_check(self, client: KotaDB) -> None:
        """Test that health check returns expected data."""
        health = client.health()

        assert health["status"] == "healthy"
        assert "version" in health
        assert "uptime_seconds" in health
        assert isinstance(health["uptime_seconds"], (int, float))

    def test_connection_with_wrong_url(self) -> None:
        """Test that connection fails with wrong URL."""
        with pytest.raises(Exception):
            KotaDB("http://localhost:99999")

    def test_connection_string_parsing(self, kotadb_server: str) -> None:
        """Test various connection string formats."""
        # These should all work
        clients = [
            KotaDB(kotadb_server),
            KotaDB("localhost:8080"),
            KotaDB("kotadb://localhost:8080/test"),
        ]

        for client in clients:
            health = client.health()
            assert health["status"] == "healthy"
            client.close()


@pytest.mark.integration
class TestDocumentCRUD:
    """Test document CRUD operations."""

    def test_create_document(self, client: KotaDB, test_document: dict) -> None:
        """Test creating a new document."""
        doc_id = client.insert(test_document)

        assert doc_id
        assert isinstance(doc_id, str)
        assert len(doc_id) > 0

        # Verify we can retrieve it
        doc = client.get(doc_id)
        assert doc.title == test_document["title"]
        assert doc.path == test_document["path"]
        assert test_document["content"] in doc.content
        assert set(doc.tags) == set(test_document["tags"])

        # Clean up
        client.delete(doc_id)

    def test_get_document(self, client: KotaDB, test_document: dict) -> None:
        """Test retrieving a document."""
        # Create a document
        doc_id = client.insert(test_document)

        # Retrieve it
        doc = client.get(doc_id)

        assert isinstance(doc, Document)
        assert doc.id == doc_id
        assert doc.title == test_document["title"]
        assert doc.path == test_document["path"]

        # Clean up
        client.delete(doc_id)

    def test_update_document(self, client: KotaDB, test_document: dict) -> None:
        """Test updating a document."""
        # Create a document
        doc_id = client.insert(test_document)

        # Update it
        updates = {
            "title": "Updated Title",
            "content": list("Updated content".encode("utf-8")),
            "tags": ["updated", "modified"],
        }
        updated_doc = client.update(doc_id, updates)

        assert updated_doc.title == "Updated Title"
        assert "Updated content" in updated_doc.content
        assert set(updated_doc.tags) == {"updated", "modified"}

        # Verify persistence
        doc = client.get(doc_id)
        assert doc.title == "Updated Title"

        # Clean up
        client.delete(doc_id)

    def test_delete_document(self, client: KotaDB, test_document: dict) -> None:
        """Test deleting a document."""
        # Create a document
        doc_id = client.insert(test_document)

        # Delete it
        result = client.delete(doc_id)
        assert result is True

        # Verify it's gone
        with pytest.raises(NotFoundError):
            client.get(doc_id)

    def test_get_nonexistent_document(self, client: KotaDB) -> None:
        """Test getting a document that doesn't exist."""
        fake_id = str(uuid.uuid4())

        with pytest.raises(NotFoundError):
            client.get(fake_id)

    def test_delete_nonexistent_document(self, client: KotaDB) -> None:
        """Test deleting a document that doesn't exist."""
        fake_id = str(uuid.uuid4())

        with pytest.raises(NotFoundError):
            client.delete(fake_id)


@pytest.mark.integration
class TestSearch:
    """Test search functionality."""

    @pytest.fixture(autouse=True)
    def setup_search_data(self, client: KotaDB) -> Generator[list, None, None]:
        """Create test documents for search tests."""
        docs = [
            {
                "path": "/search/rust.md",
                "title": "Rust Programming",
                "content": "Rust is a systems programming language focused on safety.",
                "tags": ["rust", "programming", "systems"],
            },
            {
                "path": "/search/python.md",
                "title": "Python Programming",
                "content": "Python is a high-level programming language.",
                "tags": ["python", "programming", "scripting"],
            },
            {
                "path": "/search/database.md",
                "title": "Database Design",
                "content": "Database design involves creating efficient data structures.",
                "tags": ["database", "design", "sql"],
            },
        ]

        doc_ids = []
        for doc in docs:
            doc_id = client.insert(doc)
            doc_ids.append(doc_id)

        yield doc_ids

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass

    def test_basic_search(self, client: KotaDB) -> None:
        """Test basic text search."""
        results = client.query("programming")

        assert results.total_count >= 2
        assert len(results.results) >= 2

        # Check that results contain the search term
        for doc in results.results:
            assert "programming" in doc.title.lower() or "programming" in doc.content.lower()

    def test_search_with_limit(self, client: KotaDB) -> None:
        """Test search with result limit."""
        results = client.query("programming", limit=1)

        assert len(results.results) <= 1

    def test_search_with_offset(self, client: KotaDB) -> None:
        """Test search with offset."""
        # Get all results
        all_results = client.query("programming")

        # Get results with offset
        offset_results = client.query("programming", offset=1)

        # Should have fewer results
        if all_results.total_count > 1:
            assert len(offset_results.results) < len(all_results.results)

    def test_empty_search_results(self, client: KotaDB) -> None:
        """Test search with no matches."""
        results = client.query("nonexistentterm12345")

        assert results.total_count == 0
        assert len(results.results) == 0


@pytest.mark.integration
class TestBulkOperations:
    """Test bulk operations and performance."""

    def test_bulk_insert(self, client: KotaDB) -> None:
        """Test inserting multiple documents."""
        doc_ids = []
        num_docs = 10

        start_time = time.time()

        for i in range(num_docs):
            doc = {
                "path": f"/bulk/doc_{i}.md",
                "title": f"Bulk Document {i}",
                "content": f"Content for document {i}",
                "tags": ["bulk", f"doc{i}"],
            }
            doc_id = client.insert(doc)
            doc_ids.append(doc_id)

        elapsed = time.time() - start_time

        assert len(doc_ids) == num_docs
        assert elapsed < 5.0  # Should complete within 5 seconds

        # Clean up
        for doc_id in doc_ids:
            client.delete(doc_id)

    def test_concurrent_operations(self, client: KotaDB) -> None:
        """Test concurrent read/write operations."""
        import threading

        doc_ids = []
        errors = []

        def create_doc(index: int) -> None:
            try:
                doc = {
                    "path": f"/concurrent/doc_{index}.md",
                    "title": f"Concurrent Doc {index}",
                    "content": f"Content {index}",
                    "tags": ["concurrent"],
                }
                doc_id = client.insert(doc)
                doc_ids.append(doc_id)
            except Exception as e:
                errors.append(e)

        # Create documents concurrently
        threads = []
        for i in range(5):
            thread = threading.Thread(target=create_doc, args=(i,))
            threads.append(thread)
            thread.start()

        for thread in threads:
            thread.join()

        assert len(errors) == 0
        assert len(doc_ids) == 5

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling and edge cases."""

    def test_invalid_document_data(self, client: KotaDB) -> None:
        """Test handling of invalid document data."""
        # Missing required fields
        with pytest.raises(Exception):
            client.insert({"title": "No path"})

        with pytest.raises(Exception):
            client.insert({"path": "/test.md"})

    def test_large_document(self, client: KotaDB) -> None:
        """Test handling of large documents."""
        large_content = "x" * 100000  # 100KB of content

        doc = {
            "path": "/large/doc.md",
            "title": "Large Document",
            "content": large_content,
            "tags": ["large"],
        }

        doc_id = client.insert(doc)
        assert doc_id

        # Retrieve and verify
        retrieved = client.get(doc_id)
        assert len(retrieved.content) >= 100000

        # Clean up
        client.delete(doc_id)

    def test_special_characters(self, client: KotaDB) -> None:
        """Test handling of special characters."""
        doc = {
            "path": "/special/doc.md",
            "title": "Special: 特殊文字 🚀 & <tags>",
            "content": "Content with special chars: é, ñ, 中文, 🎉",
            "tags": ["special", "unicode"],
        }

        doc_id = client.insert(doc)

        # Retrieve and verify
        retrieved = client.get(doc_id)
        assert "特殊文字" in retrieved.title
        assert "🚀" in retrieved.title
        assert "中文" in retrieved.content

        # Clean up
        client.delete(doc_id)

    def test_connection_resilience(self, client: KotaDB) -> None:
        """Test that client handles temporary connection issues."""
        # This would test retry logic, but we can't easily simulate
        # connection issues in integration tests

        # At least verify the client has retry configuration
        assert client.session.adapters["http://"].max_retries.total > 0


@pytest.mark.integration
@pytest.mark.slow
class TestPerformance:
    """Performance tests for the client."""

    def test_query_performance(self, client: KotaDB) -> None:
        """Test query performance meets requirements."""
        # Create some test data
        doc_ids = []
        for i in range(50):
            doc = {
                "path": f"/perf/doc_{i}.md",
                "title": f"Performance Test {i}",
                "content": f"Content for performance testing document {i}",
                "tags": ["performance", f"test{i}"],
            }
            doc_ids.append(client.insert(doc))

        # Measure query time
        start = time.time()
        results = client.query("performance")
        elapsed = time.time() - start

        # Should complete within 100ms (accounting for network)
        assert elapsed < 0.1
        assert results.total_count >= 50

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass

    def test_insert_performance(self, client: KotaDB) -> None:
        """Test insert performance."""
        doc = {
            "path": "/perf/insert.md",
            "title": "Insert Performance Test",
            "content": "Test content",
            "tags": ["performance"],
        }

        times = []
        doc_ids = []

        for _ in range(10):
            start = time.time()
            doc_id = client.insert(doc)
            elapsed = time.time() - start
            times.append(elapsed)
            doc_ids.append(doc_id)

        # Average insert time should be under 50ms
        avg_time = sum(times) / len(times)
        assert avg_time < 0.05

        # Clean up
        for doc_id in doc_ids:
            client.delete(doc_id)
