"""
KotaDB Python Client

A simple HTTP client for KotaDB that provides PostgreSQL-level ease of use.

Example usage:
    from kotadb import KotaDB

    db = KotaDB("http://localhost:8080")
    results = db.query("rust patterns")
    doc_id = db.insert({"title": "My Note", "content": "...", "tags": ["work"]})
"""

from .client import KotaDB
from .exceptions import ConnectionError, KotaDBError, ValidationError
from .types import Document, QueryResult, SearchResult

__version__ = "0.2.0"
__all__ = [
    "ConnectionError",
    "Document",
    "KotaDB",
    "KotaDBError",
    "QueryResult",
    "SearchResult",
    "ValidationError",
]
