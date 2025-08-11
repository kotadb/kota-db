"""
KotaDB data types and models.
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Any, Dict, List, Optional, Union


@dataclass
class Document:
    """Represents a document in KotaDB."""

    id: str
    path: str
    title: str
    content: Union[str, bytes, List[int]]  # Can be string, bytes, or byte array
    tags: List[str]
    created_at: datetime
    updated_at: datetime
    size: int
    metadata: Optional[Dict[str, Any]] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Document":
        """Create a Document from a dictionary response."""
        # Handle content which comes as byte array from server
        content = data["content"]
        if isinstance(content, list):
            # Convert byte array to string
            content = bytes(content).decode("utf-8", errors="replace")

        # Handle timestamp fields - they may be Unix timestamps or ISO strings
        created_at = data.get("created_at", data.get("created_at_unix"))
        updated_at = data.get("modified_at", data.get("updated_at", data.get("modified_at_unix")))

        if isinstance(created_at, (int, float)):
            created_at = datetime.fromtimestamp(created_at)
        else:
            created_at = datetime.fromisoformat(created_at.replace("Z", "+00:00"))

        if isinstance(updated_at, (int, float)):
            updated_at = datetime.fromtimestamp(updated_at)
        else:
            updated_at = datetime.fromisoformat(updated_at.replace("Z", "+00:00"))

        return cls(
            id=data["id"],
            path=data["path"],
            title=data["title"],
            content=content,
            tags=data.get("tags", []),
            created_at=created_at,
            updated_at=updated_at,
            size=data.get("size_bytes", data.get("size", 0)),
            metadata=data.get("metadata"),
        )

    def to_dict(self) -> Dict[str, Any]:
        """Convert Document to dictionary for API requests."""
        # Convert content to byte array if it's a string
        content = self.content
        if isinstance(content, str):
            content = list(content.encode("utf-8"))
        elif isinstance(content, bytes):
            content = list(content)

        return {
            "id": self.id,
            "path": self.path,
            "title": self.title,
            "content": content,
            "tags": self.tags,
            "metadata": self.metadata,
        }


@dataclass
class SearchResult:
    """Represents a search result with relevance score."""

    document: Document
    score: float
    content_preview: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "SearchResult":
        """Create a SearchResult from a dictionary response."""
        return cls(
            document=Document.from_dict(data["document"]),
            score=data["score"],
            content_preview=data.get("content_preview", ""),
        )


@dataclass
class QueryResult:
    """Represents the result of a query operation."""

    results: List[Document]  # Changed from SearchResult to Document
    total_count: int
    query_time_ms: Optional[int] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "QueryResult":
        """Create a QueryResult from a dictionary response."""
        # Handle different response formats
        documents = data.get("documents", data.get("results", []))
        if documents and not isinstance(documents[0], dict):
            # If we get a list of IDs or other format, wrap them
            documents = [
                {"document": doc} if not isinstance(doc, dict) else doc for doc in documents
            ]

        return cls(
            results=[Document.from_dict(doc) for doc in documents],
            total_count=data.get("total_count", len(documents)),
            query_time_ms=data.get("query_time_ms"),
        )


@dataclass
class CreateDocumentRequest:
    """Request payload for creating a document."""

    path: str
    title: str
    content: Union[str, bytes, List[int]]  # Can be string, bytes, or byte array
    tags: Optional[List[str]] = None
    metadata: Optional[Dict[str, Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for API request."""
        # Convert content to byte array if it's a string
        content = self.content
        if isinstance(content, str):
            content = list(content.encode("utf-8"))
        elif isinstance(content, bytes):
            content = list(content)

        data = {"path": self.path, "title": self.title, "content": content}
        if self.tags:
            data["tags"] = self.tags
        if self.metadata:
            data["metadata"] = self.metadata
        return data


# Type aliases for convenience
DocumentDict = Dict[str, Union[str, List[str], Dict[str, Any]]]
ConnectionString = str
