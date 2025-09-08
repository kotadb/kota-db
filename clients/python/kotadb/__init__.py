"""
KotaDB Python Client

A simple HTTP client for KotaDB that provides PostgreSQL-level ease of use.

Example usage:
    from kotadb import KotaDB

    db = KotaDB("http://localhost:8080")
    results = db.query("rust patterns")
    doc_id = db.insert({"title": "My Note", "content": "...", "tags": ["work"]})
"""

from .builders import DocumentBuilder, QueryBuilder, UpdateBuilder
from .client import KotaDB
from .exceptions import ConnectionError, KotaDBError, ValidationError
from .server import KotaDBServer, start_server, ensure_binary_installed
from .types import Document, QueryResult, SearchResult
from .validated_types import (
    NonZeroSize,
    ValidatedDirectoryPath,
    ValidatedDocumentId,
    ValidatedPath,
    ValidatedTimestamp,
    ValidatedTitle,
)
from .validation import ValidationError as ClientValidationError

__version__ = "0.5.0"
__all__ = [
    "ClientValidationError",
    "ConnectionError",
    "Document",
    "DocumentBuilder",
    "KotaDB",
    "KotaDBError",
    "KotaDBServer",
    "NonZeroSize",
    "QueryBuilder",
    "QueryResult",
    "SearchResult",
    "UpdateBuilder",
    "ValidatedDirectoryPath",
    "ValidatedDocumentId",
    "ValidatedPath",
    "ValidatedTimestamp",
    "ValidatedTitle",
    "ValidationError",
    "ensure_binary_installed",
    "start_server",
]
