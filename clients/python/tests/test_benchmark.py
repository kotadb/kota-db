"""
Benchmark tests for KotaDB Python client.

These tests measure and track performance metrics.
"""

import random
import string
import time
import uuid
from typing import Any, List

import pytest


def generate_document(size_bytes: int = 1024) -> dict:
    """Generate a test document of specified size."""
    content = "".join(random.choices(string.ascii_letters, k=size_bytes))
    return {
        "path": f"/bench/doc_{uuid.uuid4()}.md",
        "title": f"Benchmark Doc {uuid.uuid4()}",
        "content": content,
        "tags": ["benchmark", "performance"],
    }


@pytest.mark.benchmark
class TestInsertPerformance:
    """Benchmark document insertion operations."""

    def test_single_insert(self, benchmark: Any, client: Any) -> None:
        """Benchmark single document insertion."""
        doc = generate_document(size_bytes=1024)

        def insert():
            doc_id = client.insert(doc)
            client.delete(doc_id)  # Clean up immediately
            return doc_id

        result = benchmark(insert)
        assert result is not None

    def test_bulk_insert_small(self, benchmark: Any, client: Any) -> None:
        """Benchmark bulk insertion of small documents."""
        docs = [generate_document(size_bytes=100) for _ in range(10)]

        def bulk_insert():
            doc_ids = []
            for doc in docs:
                doc_ids.append(client.insert(doc))
            # Clean up
            for doc_id in doc_ids:
                client.delete(doc_id)
            return doc_ids

        result = benchmark(bulk_insert)
        assert len(result) == 10

    def test_bulk_insert_large(self, benchmark: Any, client: Any) -> None:
        """Benchmark bulk insertion of large documents."""
        docs = [generate_document(size_bytes=10000) for _ in range(5)]

        def bulk_insert():
            doc_ids = []
            for doc in docs:
                doc_ids.append(client.insert(doc))
            # Clean up
            for doc_id in doc_ids:
                client.delete(doc_id)
            return doc_ids

        result = benchmark(bulk_insert)
        assert len(result) == 5


@pytest.mark.benchmark
class TestRetrievalPerformance:
    """Benchmark document retrieval operations."""

    @pytest.fixture(scope="class")
    def test_docs(self, client: Any) -> List[str]:
        """Create test documents for retrieval benchmarks."""
        doc_ids = []
        for i in range(20):
            doc = generate_document(size_bytes=1024)
            doc_ids.append(client.insert(doc))

        yield doc_ids

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass

    def test_single_get(self, benchmark: Any, client: Any, test_docs: List[str]) -> None:
        """Benchmark single document retrieval."""
        doc_id = test_docs[0]

        def get_doc():
            return client.get(doc_id)

        result = benchmark(get_doc)
        assert result.id == doc_id

    def test_sequential_gets(self, benchmark: Any, client: Any, test_docs: List[str]) -> None:
        """Benchmark sequential document retrievals."""
        doc_ids = test_docs[:10]

        def get_docs():
            docs = []
            for doc_id in doc_ids:
                docs.append(client.get(doc_id))
            return docs

        result = benchmark(get_docs)
        assert len(result) == 10

    def test_random_gets(self, benchmark: Any, client: Any, test_docs: List[str]) -> None:
        """Benchmark random document retrievals."""

        def get_random_docs():
            docs = []
            for _ in range(10):
                doc_id = random.choice(test_docs)
                docs.append(client.get(doc_id))
            return docs

        result = benchmark(get_random_docs)
        assert len(result) == 10


@pytest.mark.benchmark
class TestSearchPerformance:
    """Benchmark search operations."""

    @pytest.fixture(scope="class", autouse=True)
    def search_data(self, client: Any) -> List[str]:
        """Create documents for search benchmarks."""
        doc_ids = []
        terms = ["alpha", "beta", "gamma", "delta", "epsilon"]

        for i in range(50):
            term = terms[i % len(terms)]
            doc = {
                "path": f"/search/{term}/doc_{i}.md",
                "title": f"{term.capitalize()} Document {i}",
                "content": f"This document contains the term {term} multiple times. {term} is important.",
                "tags": [term, "search", f"group{i // 10}"],
            }
            doc_ids.append(client.insert(doc))

        yield doc_ids

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass

    def test_simple_search(self, benchmark: Any, client: Any) -> None:
        """Benchmark simple text search."""

        def search():
            return client.query("alpha")

        result = benchmark(search)
        assert result.total_count >= 10

    def test_search_with_limit(self, benchmark: Any, client: Any) -> None:
        """Benchmark search with result limit."""

        def search():
            return client.query("document", limit=10)

        result = benchmark(search)
        assert len(result.results) <= 10

    def test_search_with_pagination(self, benchmark: Any, client: Any) -> None:
        """Benchmark paginated search."""

        def paginated_search():
            results = []
            for offset in range(0, 30, 10):
                page = client.query("document", limit=10, offset=offset)
                results.extend(page.results)
            return results

        result = benchmark(paginated_search)
        assert len(result) <= 30

    def test_complex_search(self, benchmark: Any, client: Any) -> None:
        """Benchmark complex search queries."""
        queries = ["alpha beta", "gamma delta", "epsilon document"]

        def complex_search():
            all_results = []
            for query in queries:
                results = client.query(query, limit=5)
                all_results.append(results)
            return all_results

        result = benchmark(complex_search)
        assert len(result) == len(queries)


@pytest.mark.benchmark
class TestUpdatePerformance:
    """Benchmark document update operations."""

    @pytest.fixture
    def update_doc(self, client: Any) -> str:
        """Create a document for update benchmarks."""
        doc = generate_document(size_bytes=1024)
        doc_id = client.insert(doc)

        yield doc_id

        # Clean up
        try:
            client.delete(doc_id)
        except:
            pass

    def test_single_update(self, benchmark: Any, client: Any, update_doc: str) -> None:
        """Benchmark single document update."""
        updates = {
            "title": f"Updated at {time.time()}",
            "content": list(b"Updated content"),
            "tags": ["updated", "benchmark"],
        }

        def update():
            return client.update(update_doc, updates)

        result = benchmark(update)
        assert result.id == update_doc

    def test_rapid_updates(self, benchmark: Any, client: Any) -> None:
        """Benchmark rapid successive updates."""
        # Create a fresh document for this test
        doc = generate_document(size_bytes=100)
        doc_id = client.insert(doc)

        def rapid_update():
            for i in range(10):
                updates = {"title": f"Update {i}", "content": list(f"Content {i}".encode())}
                client.update(doc_id, updates)

        benchmark(rapid_update)

        # Clean up
        client.delete(doc_id)


@pytest.mark.benchmark
class TestDeletePerformance:
    """Benchmark document deletion operations."""

    def test_single_delete(self, benchmark: Any, client: Any) -> None:
        """Benchmark single document deletion."""

        def delete_operation():
            # Create and delete a document
            doc = generate_document(size_bytes=1024)
            doc_id = client.insert(doc)
            result = client.delete(doc_id)
            return result

        result = benchmark(delete_operation)
        assert result is True

    def test_bulk_delete(self, benchmark: Any, client: Any) -> None:
        """Benchmark bulk document deletion."""

        def bulk_delete():
            # Create documents
            doc_ids = []
            for _ in range(10):
                doc = generate_document(size_bytes=100)
                doc_ids.append(client.insert(doc))

            # Delete them all
            results = []
            for doc_id in doc_ids:
                results.append(client.delete(doc_id))

            return results

        results = benchmark(bulk_delete)
        assert all(results)


@pytest.mark.benchmark
class TestConnectionPerformance:
    """Benchmark connection and health check operations."""

    def test_health_check(self, benchmark: Any, client: Any) -> None:
        """Benchmark health check operation."""

        def health_check():
            return client.health()

        result = benchmark(health_check)
        assert result["status"] == "healthy"

    def test_connection_establishment(self, benchmark: Any) -> None:
        """Benchmark new connection establishment."""
        from kotadb import KotaDB

        def create_connection():
            client = KotaDB("http://localhost:8080")
            health = client.health()
            client.close()
            return health

        result = benchmark(create_connection)
        assert result["status"] == "healthy"


@pytest.mark.benchmark
class TestMemoryUsage:
    """Benchmark memory usage patterns."""

    def test_large_result_set_memory(self, benchmark: Any, client: Any) -> None:
        """Benchmark memory usage with large result sets."""
        # Create many documents
        doc_ids = []
        for i in range(100):
            doc = {
                "path": f"/memory/doc_{i}.md",
                "title": f"Memory Test {i}",
                "content": "x" * 1000,  # 1KB each
                "tags": ["memory"],
            }
            doc_ids.append(client.insert(doc))

        def search_large_set():
            return client.query("memory", limit=100)

        result = benchmark(search_large_set)

        # Clean up
        for doc_id in doc_ids:
            try:
                client.delete(doc_id)
            except:
                pass

        assert result.total_count >= 100


@pytest.fixture
def client() -> Any:
    """Create a client for benchmarking."""
    import requests

    from kotadb import KotaDB

    # Check if server is running
    try:
        response = requests.get("http://localhost:8080/health", timeout=1)
        if response.status_code != 200:
            pytest.skip("KotaDB server not running")
    except (requests.ConnectionError, requests.Timeout):
        pytest.skip("KotaDB server not running")

    return KotaDB("http://localhost:8080")
