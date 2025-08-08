"""
KotaDB data types and models.
"""

from typing import Dict, List, Any, Optional, Union
from dataclasses import dataclass
from datetime import datetime


@dataclass
class Document:
    """Represents a document in KotaDB."""
    id: str
    path: str
    title: str
    content: str
    tags: List[str]
    created_at: datetime
    updated_at: datetime
    size: int
    metadata: Optional[Dict[str, Any]] = None
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Document':
        """Create a Document from a dictionary response."""
        return cls(
            id=data['id'],
            path=data['path'],
            title=data['title'],
            content=data['content'],
            tags=data.get('tags', []),
            created_at=datetime.fromisoformat(data['created_at'].replace('Z', '+00:00')),
            updated_at=datetime.fromisoformat(data['updated_at'].replace('Z', '+00:00')),
            size=data['size'],
            metadata=data.get('metadata')
        )
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert Document to dictionary for API requests."""
        return {
            'id': self.id,
            'path': self.path,
            'title': self.title,
            'content': self.content,
            'tags': self.tags,
            'metadata': self.metadata
        }


@dataclass
class SearchResult:
    """Represents a search result with relevance score."""
    document: Document
    score: float
    content_preview: str
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SearchResult':
        """Create a SearchResult from a dictionary response."""
        return cls(
            document=Document.from_dict(data['document']),
            score=data['score'],
            content_preview=data.get('content_preview', '')
        )


@dataclass
class QueryResult:
    """Represents the result of a query operation."""
    results: List[SearchResult]
    total_count: int
    query_time_ms: int
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'QueryResult':
        """Create a QueryResult from a dictionary response."""
        return cls(
            results=[SearchResult.from_dict(r) for r in data['results']],
            total_count=data['total_count'],
            query_time_ms=data['query_time_ms']
        )


@dataclass
class CreateDocumentRequest:
    """Request payload for creating a document."""
    path: str
    title: str
    content: str
    tags: Optional[List[str]] = None
    metadata: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for API request."""
        data = {
            'path': self.path,
            'title': self.title,
            'content': self.content
        }
        if self.tags:
            data['tags'] = self.tags
        if self.metadata:
            data['metadata'] = self.metadata
        return data


# Type aliases for convenience
DocumentDict = Dict[str, Union[str, List[str], Dict[str, Any]]]
ConnectionString = str
