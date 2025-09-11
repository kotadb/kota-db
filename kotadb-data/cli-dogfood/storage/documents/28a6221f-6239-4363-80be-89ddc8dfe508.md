---
tags:
- file
- kota-db
- ext_py
---
"""
KotaDB validation module for Python client.

Mirrors the Rust validation patterns to provide consistent validation
across all client libraries.
"""

import re
import uuid
from pathlib import Path

# Constants for validation limits
MAX_TITLE_LENGTH = 1024
MAX_TAG_LENGTH = 128
MAX_QUERY_LENGTH = 1024
YEAR_3000_TIMESTAMP = 32503680000


class ValidationError(Exception):
    """Raised when validation fails."""

    pass


# Path validation constants
MAX_PATH_LENGTH = 4096
RESERVED_NAMES = {
    "CON",
    "PRN",
    "AUX",
    "NUL",
    "COM1",
    "COM2",
    "COM3",
    "COM4",
    "COM5",
    "COM6",
    "COM7",
    "COM8",
    "COM9",
    "LPT1",
    "LPT2",
    "LPT3",
    "LPT4",
    "LPT5",
    "LPT6",
    "LPT7",
    "LPT8",
    "LPT9",
}


def validate_file_path(path: str) -> None:
    """
    Validate a file path for storage.

    Ensures path is safe and follows platform conventions.

    Args:
        path: Path to validate

    Raises:
        ValidationError: If path is invalid
    """
    if not path:
        raise ValidationError("Path cannot be empty")

    if len(path) >= MAX_PATH_LENGTH:
        raise ValidationError(f"Path exceeds maximum length of {MAX_PATH_LENGTH}")

    if "\0" in path:
        raise ValidationError("Path contains null bytes")

    # Check for directory traversal attempts
    if ".." in path:
        # More thorough check for parent directory references
        path_parts = path.replace("\\", "/").split("/")
        if ".." in path_parts:
            raise ValidationError("Parent directory references (..) not allowed")

    # Check for reserved names (Windows compatibility)
    path_obj = Path(path)
    filename = path_obj.name
    if filename:
        stem = path_obj.stem.upper()
        if stem in RESERVED_NAMES:
            raise ValidationError(f"Reserved filename: {filename}")

    # Validate UTF-8 encoding
    try:
        path.encode("utf-8")
    except UnicodeEncodeError as e:
        raise ValidationError("Path is not valid UTF-8") from e


def validate_directory_path(path: str) -> None:
    """
    Validate a directory path.

    Args:
        path: Directory path to validate

    Raises:
        ValidationError: If path is invalid
    """
    validate_file_path(path)

    # Ensure it's not a file with extension
    if "." in Path(path).name:
        raise ValidationError("Directory path should not have file extension")


def validate_document_id(doc_id: str) -> None:
    """
    Validate a document ID.

    Args:
        doc_id: Document ID to validate

    Raises:
        ValidationError: If ID is invalid
    """
    if not doc_id:
        raise ValidationError("Document ID cannot be empty")

    try:
        parsed_uuid = uuid.UUID(doc_id)
        if parsed_uuid == uuid.UUID("00000000-0000-0000-0000-000000000000"):
            raise ValidationError("Document ID cannot be nil UUID")
    except ValueError as e:
        raise ValidationError(f"Invalid UUID format: {e}") from e


def validate_title(title: str) -> None:
    """
    Validate a document title.

    Args:
        title: Title to validate

    Raises:
        ValidationError: If title is invalid
    """
    if not title or not title.strip():
        raise ValidationError("Title cannot be empty")

    if len(title.strip()) > MAX_TITLE_LENGTH:
        raise ValidationError(f"Title exceeds maximum length of {MAX_TITLE_LENGTH} characters")


def validate_tag(tag: str) -> None:
    """
    Validate a tag.

    Args:
        tag: Tag to validate

    Raises:
        ValidationError: If tag is invalid
    """
    if not tag or not tag.strip():
        raise ValidationError("Tag cannot be empty")

    if len(tag) > MAX_TAG_LENGTH:
        raise ValidationError(f"Tag too long (max {MAX_TAG_LENGTH} chars)")

    # Check for valid characters (alphanumeric, dash, underscore, space)
    if not re.match(r"^[a-zA-Z0-9\-_ ]+$", tag):
        raise ValidationError("Tag contains invalid characters")


def validate_search_query(query: str) -> None:
    """
    Validate a search query.

    Args:
        query: Search query to validate

    Raises:
        ValidationError: If query is invalid
    """
    if not query or not query.strip():
        raise ValidationError("Search query cannot be empty")

    if len(query) > MAX_QUERY_LENGTH:
        raise ValidationError(f"Search query too long (max {MAX_QUERY_LENGTH} chars)")


def validate_timestamp(timestamp: int) -> None:
    """
    Validate a timestamp.

    Args:
        timestamp: Unix timestamp to validate

    Raises:
        ValidationError: If timestamp is invalid
    """
    if timestamp <= 0:
        raise ValidationError("Timestamp must be positive")

    # Check not too far in future (year 3000)
    if timestamp >= YEAR_3000_TIMESTAMP:
        raise ValidationError("Timestamp too far in future")


def validate_size(size: int) -> None:
    """
    Validate a size value.

    Args:
        size: Size to validate

    Raises:
        ValidationError: If size is invalid
    """
    if size <= 0:
        raise ValidationError("Size must be greater than zero")

    # Check for reasonable maximum (100MB)
    if size > 100 * 1024 * 1024:
        raise ValidationError("Size exceeds maximum (100MB)")
