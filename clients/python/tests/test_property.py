"""
Property-based tests for KotaDB Python client using Hypothesis.

These tests ensure the client behaves correctly across a wide range of inputs.
"""

import string
from datetime import datetime
from typing import Any, Dict, List

import pytest
from hypothesis import given
from hypothesis import strategies as st
from hypothesis.stateful import Bundle, RuleBasedStateMachine, rule

from kotadb.client import KotaDB
from kotadb.types import CreateDocumentRequest


# Custom strategies for generating test data
@st.composite
def valid_path(draw: Any) -> str:
    """Generate valid document paths."""
    parts = draw(
        st.lists(
            st.text(alphabet=string.ascii_letters + string.digits + "-_", min_size=1, max_size=20),
            min_size=1,
            max_size=5,
        )
    )
    extension = draw(st.sampled_from([".md", ".txt", ".json", ".yaml", ".html"]))
    return "/" + "/".join(parts) + extension


@st.composite
def valid_title(draw: Any) -> str:
    """Generate valid document titles."""
    return draw(st.text(alphabet=string.printable, min_size=1, max_size=200)).strip()


@st.composite
def valid_content(draw: Any) -> str:
    """Generate valid document content."""
    return draw(st.text(min_size=0, max_size=10000))


@st.composite
def valid_tags(draw: Any) -> List[str]:
    """Generate valid tag lists."""
    return draw(
        st.lists(
            st.text(alphabet=string.ascii_letters + string.digits + "-_", min_size=1, max_size=50),
            min_size=0,
            max_size=10,
            unique=True,
        )
    )


@st.composite
def document_dict(draw: Any) -> Dict[str, Any]:
    """Generate valid document dictionaries."""
    return {
        "path": draw(valid_path()),
        "title": draw(valid_title()),
        "content": draw(valid_content()),
        "tags": draw(valid_tags()),
    }


class TestDocumentProperties:
    """Property-based tests for Document type."""

    @given(path=valid_path(), title=valid_title(), content=valid_content(), tags=valid_tags())
    def test_document_creation_preserves_data(
        self, path: str, title: str, content: str, tags: List[str]
    ) -> None:
        """Test that document creation preserves all input data."""
        request = CreateDocumentRequest(path=path, title=title, content=content, tags=tags)

        # Convert to dict and back
        data = request.to_dict()

        # Content should be converted to byte array
        assert isinstance(data["content"], list)
        assert data["path"] == path
        assert data["title"] == title
        if tags:
            assert data["tags"] == tags

    @given(content=valid_content())
    def test_content_encoding_roundtrip(self, content: str) -> None:
        """Test that content can be encoded and decoded without loss."""
        # Encode to byte array
        encoded = list(content.encode("utf-8"))

        # Decode back
        decoded = bytes(encoded).decode("utf-8", errors="replace")

        # Should match original (unless there were encoding errors)
        if content.isascii():
            assert decoded == content

    @given(st.lists(st.integers(0, 255), min_size=0, max_size=1000))
    def test_byte_array_handling(self, byte_array: List[int]) -> None:
        """Test that any valid byte array can be handled."""
        try:
            # Try to decode as UTF-8
            content = bytes(byte_array).decode("utf-8", errors="replace")
            # Should not raise an exception
            assert isinstance(content, str)
        except Exception as e:
            pytest.fail(f"Failed to handle byte array: {e}")

    @given(timestamp=st.integers(min_value=0, max_value=2**31 - 1))
    def test_unix_timestamp_conversion(self, timestamp: int) -> None:
        """Test that Unix timestamps convert to datetime correctly."""
        dt = datetime.fromtimestamp(timestamp)
        assert isinstance(dt, datetime)
        assert dt.timestamp() == timestamp


class TestClientURLParsing:
    """Property-based tests for URL parsing."""

    @given(
        host=st.text(
            alphabet=string.ascii_letters + string.digits + ".-", min_size=1, max_size=100
        ),
        port=st.integers(min_value=1, max_value=65535),
    )
    def test_url_parsing_http(self, host: str, port: int) -> None:
        """Test that various HTTP URLs are parsed correctly."""
        from unittest.mock import patch

        from kotadb.client import KotaDB

        url = f"http://{host}:{port}"

        with patch.object(KotaDB, "_test_connection"):
            client = KotaDB(url)
            assert client.base_url == url

    @given(
        host=st.text(
            alphabet=string.ascii_letters + string.digits + ".-", min_size=1, max_size=100
        ),
        port=st.integers(min_value=1, max_value=65535),
        database=st.text(
            alphabet=string.ascii_letters + string.digits + "_", min_size=1, max_size=50
        ),
    )
    def test_url_parsing_kotadb_scheme(self, host: str, port: int, database: str) -> None:
        """Test that kotadb:// URLs are parsed correctly."""
        from unittest.mock import patch

        from kotadb.client import KotaDB

        url = f"kotadb://{host}:{port}/{database}"
        expected = f"http://{host}:{port}"

        with patch.object(KotaDB, "_test_connection"):
            client = KotaDB(url)
            assert client.base_url == expected


class TestQueryParameters:
    """Property-based tests for query parameters."""

    @given(
        query=st.text(min_size=0, max_size=1000),
        limit=st.one_of(st.none(), st.integers(min_value=1, max_value=1000)),
        offset=st.integers(min_value=0, max_value=10000),
    )
    def test_query_parameters_valid(self, query: str, limit: int, offset: int) -> None:
        """Test that various query parameters are handled correctly."""
        from unittest.mock import Mock, patch

        from kotadb.client import KotaDB

        with patch.object(KotaDB, "_test_connection"):
            client = KotaDB("http://localhost:8080")

            with patch.object(client, "_make_request") as mock_request:
                mock_response = Mock()
                mock_response.json.return_value = {"documents": [], "total_count": 0}
                mock_request.return_value = mock_response

                # Should not raise an exception
                result = client.query(query, limit=limit, offset=offset)

                # Verify parameters were passed correctly
                call_args = mock_request.call_args
                params = call_args[1]["params"]
                assert params["q"] == query
                if limit:
                    assert params["limit"] == limit
                if offset:
                    assert params["offset"] == offset


# Removed mock-based stateful testing per user guidance
# Real integration tests should be used instead of mocked state machines

# class DocumentDatabaseStateMachine(RuleBasedStateMachine):
    """
    Stateful property testing for KotaDB client.

    This ensures that sequences of operations maintain consistency.
    """

    documents = Bundle("documents")

    def __init__(self):
        super().__init__()
        from unittest.mock import patch

        # Mock the client to avoid network calls
        with patch("kotadb.client.KotaDB._test_connection"):
            self.client = KotaDB("http://localhost:8080")

        # Track state
        self.stored_documents = {}
        self.next_id = 1

    @rule(path=valid_path(), title=valid_title(), content=valid_content(), tags=valid_tags())
    def create_document(self, path: str, title: str, content: str, tags: List[str]) -> None:
        """Create a new document."""
        from unittest.mock import Mock, patch

        doc_id = f"doc-{self.next_id}"
        self.next_id += 1

        with patch.object(self.client, "_make_request") as mock:
            mock_response = Mock()
            mock_response.json.return_value = {"id": doc_id}
            mock.return_value = mock_response

            result_id = self.client.insert(
                {"path": path, "title": title, "content": content, "tags": tags}
            )

            assert result_id == doc_id
            self.stored_documents[doc_id] = {
                "path": path,
                "title": title,
                "content": content,
                "tags": tags,
            }

    @rule(doc_id=st.sampled_from(["doc-1", "doc-2", "doc-3", "doc-999"]))
    def get_document(self, doc_id: str) -> None:
        """Get a document by ID."""
        from unittest.mock import Mock, patch

        from kotadb.exceptions import NotFoundError

        if doc_id in self.stored_documents:
            doc_data = self.stored_documents[doc_id]

            with patch.object(self.client, "_make_request") as mock:
                mock_response = Mock()
                mock_response.json.return_value = {
                    "id": doc_id,
                    "path": doc_data["path"],
                    "title": doc_data["title"],
                    "content": list(doc_data["content"].encode("utf-8")),
                    "tags": doc_data["tags"],
                    "created_at_unix": 1704067200,
                    "modified_at_unix": 1704067200,
                    "size_bytes": len(doc_data["content"]),
                }
                mock.return_value = mock_response

                doc = self.client.get(doc_id)
                assert doc.id == doc_id
                assert doc.title == doc_data["title"]
        else:
            with patch.object(self.client, "_make_request") as mock:
                mock_response = Mock()
                mock_response.status_code = 404
                mock.return_value = mock_response

                with pytest.raises(NotFoundError):
                    self.client.get(doc_id)

    @rule(doc_id=st.sampled_from(["doc-1", "doc-2", "doc-3"]))
    def delete_document(self, doc_id: str) -> None:
        """Delete a document."""
        from unittest.mock import Mock, patch

        with patch.object(self.client, "_make_request") as mock:
            mock_response = Mock()
            mock_response.status_code = 200 if doc_id in self.stored_documents else 404
            mock.return_value = mock_response

            if doc_id in self.stored_documents:
                result = self.client.delete(doc_id)
                assert result is True
                del self.stored_documents[doc_id]
            else:
                from kotadb.exceptions import NotFoundError

                with pytest.raises(NotFoundError):
                    self.client.delete(doc_id)

    def invariant_document_count(self) -> None:
        """Invariant: document count should match our tracking."""
        # This would normally check against the actual database
        assert len(self.stored_documents) >= 0

    def invariant_unique_ids(self) -> None:
        """Invariant: all document IDs should be unique."""
        ids = list(self.stored_documents.keys())
        assert len(ids) == len(set(ids))


# Test the state machine
# TestDocumentDatabase = DocumentDatabaseStateMachine.TestCase


@pytest.mark.property
class TestFuzzingInputs:
    """Fuzz testing with potentially malicious inputs."""

    @given(path=st.text(min_size=0, max_size=1000))
    def test_path_validation(self, path: str) -> None:
        """Test that any path input is handled safely."""
        from kotadb.types import CreateDocumentRequest

        # Should either work or raise a validation error
        try:
            request = CreateDocumentRequest(path=path, title="Test", content="Test")
            # If it succeeds, the path should be preserved
            assert request.path == path
        except Exception as e:
            # Should be a validation error, not a crash
            assert "validation" in str(e).lower() or "invalid" in str(e).lower()

    @given(content=st.binary(min_size=0, max_size=10000))
    def test_binary_content_handling(self, content: bytes) -> None:
        """Test that binary content is handled correctly."""
        # Convert to list of integers (byte array)
        byte_array = list(content)

        # Should be valid integers
        assert all(0 <= b <= 255 for b in byte_array)

        # Should be able to decode with error handling
        text = content.decode("utf-8", errors="replace")
        assert isinstance(text, str)
