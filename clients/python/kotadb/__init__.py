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
from .types import Document, SearchResult, QueryResult
from .exceptions import KotaDBError, ConnectionError, ValidationError

__version__ = "0.1.0"
__all__ = ["KotaDB", "Document", "SearchResult", "QueryResult", "KotaDBError", "ConnectionError", "ValidationError"]
