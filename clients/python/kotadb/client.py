"""
KotaDB Python Client

Main client class for interacting with KotaDB HTTP API.
"""

import os
import urllib.parse
from typing import Any, Dict, List, Optional, Union

import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

from .exceptions import ConnectionError, NotFoundError, ServerError, ValidationError
from .types import CreateDocumentRequest, Document, DocumentDict, QueryResult


class KotaDB:
    """
    KotaDB client for easy database operations.

    Provides a simple, PostgreSQL-like interface for document operations.

    Example:
        # Connect using URL
        db = KotaDB("http://localhost:8080")

        # Connect using environment variable
        db = KotaDB()  # Uses KOTADB_URL

        # Connect with connection string
        db = KotaDB("kotadb://localhost:8080/myapp")

        # Basic operations
        results = db.query("rust patterns")
        doc_id = db.insert({"title": "My Note", "content": "...", "tags": ["work"]})
        doc = db.get(doc_id)
        db.delete(doc_id)
    """

    def __init__(self, url: Optional[str] = None, timeout: int = 30, retries: int = 3):
        """
        Initialize KotaDB client.

        Args:
            url: Database URL. Can be HTTP URL or kotadb:// connection string.
                 If None, uses KOTADB_URL environment variable.
            timeout: Request timeout in seconds.
            retries: Number of retry attempts for failed requests.
        """
        self.base_url = self._parse_url(url)
        self.timeout = timeout

        # Configure session with retries
        self.session = requests.Session()
        retry_strategy = Retry(
            total=retries,
            status_forcelist=[429, 500, 502, 503, 504],
            allowed_methods=["HEAD", "GET", "PUT", "DELETE", "OPTIONS", "TRACE", "POST"],
        )
        adapter = HTTPAdapter(max_retries=retry_strategy)
        self.session.mount("http://", adapter)
        self.session.mount("https://", adapter)

        # Test connection
        self._test_connection()

    def test_connection(self) -> Dict[str, Any]:
        """
        Test connection to the database.

        Returns:
            Health status information
        """
        return self.health()

    def _parse_url(self, url: Optional[str]) -> str:
        """Parse and normalize the database URL."""
        if url is None:
            url = os.getenv("KOTADB_URL")
            if not url:
                raise ConnectionError("No URL provided and KOTADB_URL environment variable not set")

        # Handle kotadb:// connection strings
        if url.startswith("kotadb://"):
            # Convert kotadb://host:port/database to http://host:port
            parsed = urllib.parse.urlparse(url)
            return f"http://{parsed.netloc}"

        # Ensure URL has protocol
        if not url.startswith(("http://", "https://")):
            url = f"http://{url}"

        # Remove trailing slash
        return url.rstrip("/")

    def _test_connection(self):
        """Test connection to the database."""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=self.timeout)
            if response.status_code != 200:
                raise ConnectionError(f"Health check failed with status {response.status_code}")
        except requests.RequestException as e:
            raise ConnectionError(f"Failed to connect to KotaDB at {self.base_url}: {e}") from e

    def _make_request(self, method: str, endpoint: str, **kwargs) -> requests.Response:
        """Make an HTTP request with error handling."""
        url = f"{self.base_url}{endpoint}"

        try:
            response = self.session.request(method, url, timeout=self.timeout, **kwargs)

            if response.status_code == 404:
                raise NotFoundError("Resource not found")
            elif response.status_code >= 400:
                try:
                    error_data = response.json()
                    error_msg = error_data.get("error", f"HTTP {response.status_code}")
                except (ValueError, KeyError, AttributeError):
                    error_msg = f"HTTP {response.status_code}: {response.text}"
                raise ServerError(error_msg, response.status_code, response.text)

            return response

        except requests.RequestException as e:
            raise ConnectionError(f"Request failed: {e}") from e

    def query(self, query: str, limit: Optional[int] = None, offset: int = 0) -> QueryResult:
        """
        Search documents using text query.

        Args:
            query: Search query string
            limit: Maximum number of results to return
            offset: Number of results to skip

        Returns:
            QueryResult with matching documents and metadata
        """
        params = {"q": query}
        if limit:
            params["limit"] = limit
        if offset:
            params["offset"] = offset

        response = self._make_request("GET", "/documents/search", params=params)
        return QueryResult.from_dict(response.json())

    def semantic_search(
        self, query: str, limit: Optional[int] = None, offset: int = 0
    ) -> QueryResult:
        """
        Perform semantic search using embeddings.

        Args:
            query: Semantic search query
            limit: Maximum number of results to return
            offset: Number of results to skip

        Returns:
            QueryResult with semantically similar documents
        """
        data = {"query": query}
        if limit:
            data["limit"] = limit
        if offset:
            data["offset"] = offset

        response = self._make_request("POST", "/search/semantic", json=data)
        return QueryResult.from_dict(response.json())

    def hybrid_search(
        self, query: str, limit: Optional[int] = None, offset: int = 0, semantic_weight: float = 0.7
    ) -> QueryResult:
        """
        Perform hybrid search combining text and semantic search.

        Args:
            query: Search query
            limit: Maximum number of results to return
            offset: Number of results to skip
            semantic_weight: Weight for semantic vs text search (0.0-1.0)

        Returns:
            QueryResult with hybrid search results
        """
        data = {"query": query, "semantic_weight": semantic_weight}
        if limit:
            data["limit"] = limit
        if offset:
            data["offset"] = offset

        response = self._make_request("POST", "/search/hybrid", json=data)
        return QueryResult.from_dict(response.json())

    def get(self, doc_id: str) -> Document:
        """
        Get a document by ID.

        Args:
            doc_id: Document identifier

        Returns:
            Document object
        """
        response = self._make_request("GET", f"/documents/{doc_id}")
        return Document.from_dict(response.json())

    def insert(self, document: Union[DocumentDict, CreateDocumentRequest]) -> str:
        """
        Insert a new document.

        Args:
            document: Document data as dict or CreateDocumentRequest

        Returns:
            ID of the created document
        """
        if isinstance(document, dict):
            # Validate required fields
            required_fields = ["path", "title", "content"]
            for field in required_fields:
                if field not in document:
                    raise ValidationError(f"Required field '{field}' missing")

            # Convert dict to CreateDocumentRequest
            document = CreateDocumentRequest(
                path=document["path"],
                title=document["title"],
                content=document["content"],
                tags=document.get("tags"),
                metadata=document.get("metadata"),
            )

        response = self._make_request("POST", "/documents", json=document.to_dict())
        result = response.json()
        return result["id"]

    def update(self, doc_id: str, updates: DocumentDict) -> Document:
        """
        Update an existing document.

        Args:
            doc_id: Document identifier
            updates: Fields to update

        Returns:
            Updated document
        """
        # Convert content to byte array if present
        if "content" in updates:
            content = updates["content"]
            if isinstance(content, str):
                updates["content"] = list(content.encode("utf-8"))
            elif isinstance(content, bytes):
                updates["content"] = list(content)

        response = self._make_request("PUT", f"/documents/{doc_id}", json=updates)
        return Document.from_dict(response.json())

    def delete(self, doc_id: str) -> bool:
        """
        Delete a document.

        Args:
            doc_id: Document identifier

        Returns:
            True if deletion was successful
        """
        self._make_request("DELETE", f"/documents/{doc_id}")
        return True

    def list_all(self, limit: Optional[int] = None, offset: int = 0) -> List[Document]:
        """
        List all documents.

        Args:
            limit: Maximum number of documents to return
            offset: Number of documents to skip

        Returns:
            List of documents
        """
        params = {"offset": offset}
        if limit:
            params["limit"] = limit

        response = self._make_request("GET", "/documents", params=params)
        data = response.json()
        return [Document.from_dict(doc) for doc in data["documents"]]

    def health(self) -> Dict[str, Any]:
        """
        Check database health status.

        Returns:
            Health status information
        """
        response = self._make_request("GET", "/health")
        return response.json()

    def stats(self) -> Dict[str, Any]:
        """
        Get database statistics.

        Returns:
            Database statistics
        """
        response = self._make_request("GET", "/stats")
        return response.json()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.session.close()

    def close(self):
        """Close the client session."""
        self.session.close()

    # Builder pattern methods
    
    def insert_with_builder(self, builder) -> str:
        """
        Insert a document using DocumentBuilder.
        
        Args:
            builder: DocumentBuilder instance
            
        Returns:
            ID of the created document
        """
        from .builders import DocumentBuilder
        if not isinstance(builder, DocumentBuilder):
            raise ValidationError("Expected DocumentBuilder instance")
        
        document = builder.build()
        return self.insert(document)
    
    def query_with_builder(self, builder) -> QueryResult:
        """
        Query documents using QueryBuilder.
        
        Args:
            builder: QueryBuilder instance
            
        Returns:
            QueryResult with matching documents
        """
        from .builders import QueryBuilder
        if not isinstance(builder, QueryBuilder):
            raise ValidationError("Expected QueryBuilder instance")
        
        params = builder.build()
        query_text = params.pop("q")
        return self.query(query_text, **params)
    
    def semantic_search_with_builder(self, builder) -> QueryResult:
        """
        Perform semantic search using QueryBuilder.
        
        Args:
            builder: QueryBuilder instance
            
        Returns:
            QueryResult with semantically similar documents
        """
        from .builders import QueryBuilder
        if not isinstance(builder, QueryBuilder):
            raise ValidationError("Expected QueryBuilder instance")
        
        data = builder.build_for_semantic()
        query_text = data.pop("query")
        return self.semantic_search(query_text, **data)
    
    def hybrid_search_with_builder(self, builder) -> QueryResult:
        """
        Perform hybrid search using QueryBuilder.
        
        Args:
            builder: QueryBuilder instance
            
        Returns:
            QueryResult with hybrid search results
        """
        from .builders import QueryBuilder
        if not isinstance(builder, QueryBuilder):
            raise ValidationError("Expected QueryBuilder instance")
        
        data = builder.build_for_hybrid()
        query_text = data.pop("query")
        semantic_weight = data.pop("semantic_weight", 0.7)
        return self.hybrid_search(query_text, semantic_weight=semantic_weight, **data)
    
    def update_with_builder(self, doc_id: str, builder) -> Document:
        """
        Update a document using UpdateBuilder.
        
        Args:
            doc_id: Document identifier
            builder: UpdateBuilder instance
            
        Returns:
            Updated document
        """
        from .builders import UpdateBuilder
        if not isinstance(builder, UpdateBuilder):
            raise ValidationError("Expected UpdateBuilder instance")
        
        updates = builder.build()
        
        # Handle special operations
        if "_tag_operations" in updates:
            # For now, we'll need to get current document and merge tags
            # This is a limitation of the current API
            tag_ops = updates.pop("_tag_operations")
            current_doc = self.get(doc_id)
            current_tags = set(current_doc.tags)
            
            if "add" in tag_ops:
                current_tags.update(tag_ops["add"])
            if "remove" in tag_ops:
                current_tags.difference_update(tag_ops["remove"])
            
            updates["tags"] = list(current_tags)
        
        if "_metadata_operations" in updates:
            # Handle metadata operations
            meta_ops = updates.pop("_metadata_operations")
            current_doc = self.get(doc_id)
            current_metadata = dict(current_doc.metadata) if current_doc.metadata else {}
            
            for key, value in meta_ops.items():
                if value is None:
                    current_metadata.pop(key, None)  # Remove key
                else:
                    current_metadata[key] = value
            
            updates["metadata"] = current_metadata
        
        return self.update(doc_id, updates)


# Convenience function for simple usage
def connect(url: Optional[str] = None, **kwargs) -> KotaDB:
    """
    Create a KotaDB client connection.

    Args:
        url: Database URL or connection string
        **kwargs: Additional arguments for KotaDB constructor

    Returns:
        KotaDB client instance
    """
    return KotaDB(url, **kwargs)
