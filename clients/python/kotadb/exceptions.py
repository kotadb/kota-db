"""
KotaDB client exceptions.
"""


class KotaDBError(Exception):
    """Base exception for all KotaDB client errors."""
    pass


class ConnectionError(KotaDBError):
    """Raised when connection to KotaDB server fails."""
    pass


class ValidationError(KotaDBError):
    """Raised when input validation fails."""
    pass


class NotFoundError(KotaDBError):
    """Raised when a requested document is not found."""
    pass


class ServerError(KotaDBError):
    """Raised when the server returns an error response."""
    
    def __init__(self, message, status_code=None, response_body=None):
        super().__init__(message)
        self.status_code = status_code
        self.response_body = response_body
