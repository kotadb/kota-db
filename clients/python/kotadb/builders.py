"""
Builder patterns for KotaDB Python client.

Provides safe, fluent construction of documents and queries
with validation at each step.
"""

from typing import Any, Dict, List, Optional, Union
import uuid

from .validated_types import (
    ValidatedPath,
    ValidatedDocumentId, 
    ValidatedTitle,
    NonZeroSize,
    ValidatedTimestamp,
)
from .validation import ValidationError, validate_tag
from .types import CreateDocumentRequest, Document
from datetime import datetime


class DocumentBuilder:
    """
    Builder for creating Document objects with validation.
    
    Provides a fluent interface for safe document construction
    mirroring the Rust builder patterns.
    
    Example:
        doc = (DocumentBuilder()
               .path("/notes/meeting.md")
               .title("Team Meeting Notes")
               .content("Meeting content here...")
               .add_tag("work")
               .add_tag("meeting")
               .build())
    """
    
    def __init__(self):
        """Initialize a new document builder."""
        self._path: Optional[ValidatedPath] = None
        self._title: Optional[ValidatedTitle] = None
        self._content: Optional[Union[str, bytes, List[int]]] = None
        self._tags: List[str] = []
        self._metadata: Dict[str, Any] = {}
        self._id: Optional[ValidatedDocumentId] = None
    
    def path(self, path: Union[str, ValidatedPath]) -> 'DocumentBuilder':
        """
        Set the document path.
        
        Args:
            path: Path for the document (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If path is invalid
        """
        if isinstance(path, str):
            self._path = ValidatedPath(path)
        else:
            self._path = path
        return self
    
    def title(self, title: Union[str, ValidatedTitle]) -> 'DocumentBuilder':
        """
        Set the document title.
        
        Args:
            title: Title for the document (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If title is invalid
        """
        if isinstance(title, str):
            self._title = ValidatedTitle(title)
        else:
            self._title = title
        return self
    
    def content(self, content: Union[str, bytes, List[int]]) -> 'DocumentBuilder':
        """
        Set the document content.
        
        Args:
            content: Content for the document
            
        Returns:
            Self for method chaining
        """
        self._content = content
        return self
    
    def add_tag(self, tag: str) -> 'DocumentBuilder':
        """
        Add a tag to the document.
        
        Args:
            tag: Tag to add (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If tag is invalid
        """
        validate_tag(tag)
        if tag not in self._tags:
            self._tags.append(tag)
        return self
    
    def tags(self, tags: List[str]) -> 'DocumentBuilder':
        """
        Set all tags for the document.
        
        Args:
            tags: List of tags (each will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If any tag is invalid
        """
        for tag in tags:
            validate_tag(tag)
        self._tags = list(tags)  # Create copy
        return self
    
    def add_metadata(self, key: str, value: Any) -> 'DocumentBuilder':
        """
        Add a metadata field.
        
        Args:
            key: Metadata key
            value: Metadata value
            
        Returns:
            Self for method chaining
        """
        self._metadata[key] = value
        return self
    
    def metadata(self, metadata: Dict[str, Any]) -> 'DocumentBuilder':
        """
        Set all metadata for the document.
        
        Args:
            metadata: Metadata dictionary
            
        Returns:
            Self for method chaining
        """
        self._metadata = dict(metadata)  # Create copy
        return self
    
    def id(self, doc_id: Union[str, uuid.UUID, ValidatedDocumentId]) -> 'DocumentBuilder':
        """
        Set the document ID.
        
        Args:
            doc_id: Document ID (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If ID is invalid
        """
        if isinstance(doc_id, ValidatedDocumentId):
            self._id = doc_id
        else:
            self._id = ValidatedDocumentId(doc_id)
        return self
    
    def auto_id(self) -> 'DocumentBuilder':
        """
        Generate a new random ID for the document.
        
        Returns:
            Self for method chaining
        """
        self._id = ValidatedDocumentId.new()
        return self
    
    def build(self) -> CreateDocumentRequest:
        """
        Build the CreateDocumentRequest.
        
        Returns:
            CreateDocumentRequest ready for insertion
            
        Raises:
            ValidationError: If required fields are missing
        """
        if self._path is None:
            raise ValidationError("Document path is required")
        
        if self._title is None:
            raise ValidationError("Document title is required")
        
        if self._content is None:
            raise ValidationError("Document content is required")
        
        return CreateDocumentRequest(
            path=self._path.as_str(),
            title=self._title.as_str(),
            content=self._content,
            tags=self._tags if self._tags else None,
            metadata=self._metadata if self._metadata else None,
        )
    
    def build_with_timestamps(self) -> Document:
        """
        Build a complete Document with timestamps.
        
        Returns:
            Document with current timestamps
            
        Raises:
            ValidationError: If required fields are missing
        """
        if self._path is None:
            raise ValidationError("Document path is required")
        
        if self._title is None:
            raise ValidationError("Document title is required")
        
        if self._content is None:
            raise ValidationError("Document content is required")
        
        doc_id = self._id.as_str() if self._id else str(ValidatedDocumentId.new())
        
        # Calculate content size
        if isinstance(self._content, str):
            size = len(self._content.encode('utf-8'))
            content_for_doc = self._content
        elif isinstance(self._content, bytes):
            size = len(self._content)
            content_for_doc = self._content.decode('utf-8', errors='replace')
        elif isinstance(self._content, list):
            size = len(self._content)
            content_for_doc = bytes(self._content).decode('utf-8', errors='replace')
        else:
            raise ValidationError("Invalid content type")
        
        now = datetime.now()
        
        return Document.from_dict({
            "id": doc_id,
            "path": self._path.as_str(),
            "title": self._title.as_str(),
            "content": content_for_doc,
            "tags": self._tags,
            "created_at": now.isoformat() + "Z",
            "modified_at": now.isoformat() + "Z",
            "size_bytes": size,
            "metadata": self._metadata,
        })


class QueryBuilder:
    """
    Builder for creating search queries with validation.
    
    Provides a fluent interface for building complex queries
    with proper validation and type safety.
    
    Example:
        query = (QueryBuilder()
                .text("rust patterns")
                .limit(10)
                .offset(20)
                .build())
    """
    
    def __init__(self):
        """Initialize a new query builder."""
        self._query_text: Optional[str] = None
        self._limit: Optional[int] = None
        self._offset: int = 0
        self._semantic_weight: Optional[float] = None
        self._filters: Dict[str, Any] = {}
    
    def text(self, query: str) -> 'QueryBuilder':
        """
        Set the query text.
        
        Args:
            query: Search query text
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If query is invalid
        """
        from .validation import validate_search_query
        validate_search_query(query)
        self._query_text = query
        return self
    
    def limit(self, limit: int) -> 'QueryBuilder':
        """
        Set the maximum number of results.
        
        Args:
            limit: Maximum number of results (must be positive)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If limit is invalid
        """
        if limit <= 0:
            raise ValidationError("Limit must be positive")
        if limit > 10000:
            raise ValidationError("Limit too large (max 10000)")
        self._limit = limit
        return self
    
    def offset(self, offset: int) -> 'QueryBuilder':
        """
        Set the number of results to skip.
        
        Args:
            offset: Number of results to skip (must be non-negative)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If offset is invalid
        """
        if offset < 0:
            raise ValidationError("Offset cannot be negative")
        self._offset = offset
        return self
    
    def semantic_weight(self, weight: float) -> 'QueryBuilder':
        """
        Set the semantic search weight for hybrid search.
        
        Args:
            weight: Weight between 0.0 and 1.0
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If weight is invalid
        """
        if not (0.0 <= weight <= 1.0):
            raise ValidationError("Semantic weight must be between 0.0 and 1.0")
        self._semantic_weight = weight
        return self
    
    def add_filter(self, key: str, value: Any) -> 'QueryBuilder':
        """
        Add a filter to the query.
        
        Args:
            key: Filter key
            value: Filter value
            
        Returns:
            Self for method chaining
        """
        self._filters[key] = value
        return self
    
    def tag_filter(self, tag: str) -> 'QueryBuilder':
        """
        Add a tag filter.
        
        Args:
            tag: Tag to filter by
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If tag is invalid
        """
        validate_tag(tag)
        return self.add_filter("tag", tag)
    
    def path_filter(self, path_pattern: str) -> 'QueryBuilder':
        """
        Add a path filter.
        
        Args:
            path_pattern: Path pattern to filter by
            
        Returns:
            Self for method chaining
        """
        return self.add_filter("path", path_pattern)
    
    def build(self) -> Dict[str, Any]:
        """
        Build the query parameters.
        
        Returns:
            Dictionary of query parameters
            
        Raises:
            ValidationError: If required fields are missing
        """
        if self._query_text is None:
            raise ValidationError("Query text is required")
        
        params = {"q": self._query_text}
        
        if self._limit is not None:
            params["limit"] = self._limit
        
        if self._offset > 0:
            params["offset"] = self._offset
        
        if self._semantic_weight is not None:
            params["semantic_weight"] = self._semantic_weight
        
        # Add filters
        params.update(self._filters)
        
        return params
    
    def build_for_semantic(self) -> Dict[str, Any]:
        """
        Build query data for semantic search endpoint.
        
        Returns:
            Dictionary suitable for semantic search POST body
            
        Raises:
            ValidationError: If required fields are missing
        """
        if self._query_text is None:
            raise ValidationError("Query text is required")
        
        data = {"query": self._query_text}
        
        if self._limit is not None:
            data["limit"] = self._limit
        
        if self._offset > 0:
            data["offset"] = self._offset
        
        # Add filters
        data.update(self._filters)
        
        return data
    
    def build_for_hybrid(self) -> Dict[str, Any]:
        """
        Build query data for hybrid search endpoint.
        
        Returns:
            Dictionary suitable for hybrid search POST body
        """
        data = self.build_for_semantic()
        
        if self._semantic_weight is not None:
            data["semantic_weight"] = self._semantic_weight
        
        return data


class UpdateBuilder:
    """
    Builder for creating document updates with validation.
    
    Provides a fluent interface for safely updating documents
    without overwriting fields unintentionally.
    
    Example:
        updates = (UpdateBuilder()
                  .title("Updated Title")
                  .add_tag("updated")
                  .add_metadata("last_modified_by", "user123")
                  .build())
    """
    
    def __init__(self):
        """Initialize a new update builder."""
        self._updates: Dict[str, Any] = {}
        self._tags_to_add: List[str] = []
        self._tags_to_remove: List[str] = []
        self._metadata_updates: Dict[str, Any] = {}
    
    def title(self, title: Union[str, ValidatedTitle]) -> 'UpdateBuilder':
        """
        Update the document title.
        
        Args:
            title: New title (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If title is invalid
        """
        if isinstance(title, str):
            validated_title = ValidatedTitle(title)
            self._updates["title"] = validated_title.as_str()
        else:
            self._updates["title"] = title.as_str()
        return self
    
    def content(self, content: Union[str, bytes, List[int]]) -> 'UpdateBuilder':
        """
        Update the document content.
        
        Args:
            content: New content
            
        Returns:
            Self for method chaining
        """
        self._updates["content"] = content
        return self
    
    def add_tag(self, tag: str) -> 'UpdateBuilder':
        """
        Add a tag (will be merged with existing tags).
        
        Args:
            tag: Tag to add (will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If tag is invalid
        """
        validate_tag(tag)
        if tag not in self._tags_to_add:
            self._tags_to_add.append(tag)
        return self
    
    def remove_tag(self, tag: str) -> 'UpdateBuilder':
        """
        Remove a tag.
        
        Args:
            tag: Tag to remove
            
        Returns:
            Self for method chaining
        """
        if tag not in self._tags_to_remove:
            self._tags_to_remove.append(tag)
        return self
    
    def replace_tags(self, tags: List[str]) -> 'UpdateBuilder':
        """
        Replace all tags.
        
        Args:
            tags: New tags list (each will be validated)
            
        Returns:
            Self for method chaining
            
        Raises:
            ValidationError: If any tag is invalid
        """
        for tag in tags:
            validate_tag(tag)
        self._updates["tags"] = list(tags)
        # Clear tag modifications since we're replacing
        self._tags_to_add.clear()
        self._tags_to_remove.clear()
        return self
    
    def add_metadata(self, key: str, value: Any) -> 'UpdateBuilder':
        """
        Add or update a metadata field.
        
        Args:
            key: Metadata key
            value: Metadata value
            
        Returns:
            Self for method chaining
        """
        self._metadata_updates[key] = value
        return self
    
    def remove_metadata(self, key: str) -> 'UpdateBuilder':
        """
        Remove a metadata field.
        
        Args:
            key: Metadata key to remove
            
        Returns:
            Self for method chaining
        """
        self._metadata_updates[key] = None  # Use None to indicate removal
        return self
    
    def build(self) -> Dict[str, Any]:
        """
        Build the update dictionary.
        
        Returns:
            Dictionary of updates to apply
        """
        updates = dict(self._updates)
        
        # Handle tag updates
        if self._tags_to_add or self._tags_to_remove:
            if "tags" not in updates:
                # Need current tags to modify them
                # This will require the caller to handle merging
                tag_ops = {}
                if self._tags_to_add:
                    tag_ops["add"] = self._tags_to_add
                if self._tags_to_remove:
                    tag_ops["remove"] = self._tags_to_remove
                updates["_tag_operations"] = tag_ops
        
        # Handle metadata updates
        if self._metadata_updates:
            updates["_metadata_operations"] = self._metadata_updates
        
        return updates