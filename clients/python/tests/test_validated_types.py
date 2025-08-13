"""
Tests for KotaDB validated types.
"""

import uuid

import pytest

from kotadb.validated_types import (
    NonZeroSize,
    ValidatedDirectoryPath,
    ValidatedDocumentId,
    ValidatedPath,
    ValidatedTimestamp,
    ValidatedTitle,
)
from kotadb.validation import ValidationError


class TestValidatedPath:
    """Test ValidatedPath class."""

    def test_valid_paths(self):
        """Test valid path creation."""
        valid_paths = [
            "/test/file.md",
            "relative/path.txt",
            "simple.txt",
            "/documents/notes/meeting-2024.md",
        ]

        for path in valid_paths:
            validated = ValidatedPath(path)
            assert validated.as_str() == path
            assert str(validated) == path

    def test_alternative_constructor(self):
        """Test the alternative .new() constructor."""
        path = "/test/file.md"
        validated = ValidatedPath.new(path)
        assert validated.as_str() == path

    def test_invalid_paths(self):
        """Test invalid path rejection."""
        invalid_paths = [
            "",  # Empty
            "../../../etc/passwd",  # Directory traversal
            "file\x00with\x00nulls",  # Null bytes
            "CON.txt",  # Windows reserved name
            "x" * 5000,  # Too long
        ]

        for path in invalid_paths:
            with pytest.raises(ValidationError):
                ValidatedPath(path)

    def test_equality(self):
        """Test path equality comparisons."""
        path1 = ValidatedPath("/test/file.md")
        path2 = ValidatedPath("/test/file.md")
        path3 = ValidatedPath("/different/file.md")

        assert path1 == path2
        assert path1 != path3
        assert path1 == "/test/file.md"
        assert path1 != "/different/file.md"

    def test_hash(self):
        """Test path hashing for use in sets/dicts."""
        path1 = ValidatedPath("/test/file.md")
        path2 = ValidatedPath("/test/file.md")
        path3 = ValidatedPath("/different/file.md")

        assert hash(path1) == hash(path2)
        assert hash(path1) != hash(path3)

        # Can be used in sets
        path_set = {path1, path2, path3}
        assert len(path_set) == 2


class TestValidatedDirectoryPath:
    """Test ValidatedDirectoryPath class."""

    def test_valid_directory_paths(self):
        """Test valid directory path creation."""
        valid_paths = [
            "/test/documents",
            "relative/path",
            "simple_dir",
        ]

        for path in valid_paths:
            validated = ValidatedDirectoryPath(path)
            assert validated.as_str() == path

    def test_invalid_directory_paths(self):
        """Test invalid directory path rejection."""
        # Regular invalid paths still fail
        with pytest.raises(ValidationError):
            ValidatedDirectoryPath("../../../etc")

        # Directory paths with extensions should fail
        with pytest.raises(ValidationError):
            ValidatedDirectoryPath("/path/to/file.txt")


class TestValidatedDocumentId:
    """Test ValidatedDocumentId class."""

    def test_new_id_generation(self):
        """Test new ID generation."""
        doc_id = ValidatedDocumentId.new()
        assert isinstance(doc_id.as_uuid(), uuid.UUID)
        assert len(doc_id.as_str()) == 36  # UUID string length

    def test_from_uuid(self):
        """Test creation from UUID."""
        test_uuid = uuid.uuid4()
        doc_id = ValidatedDocumentId.from_uuid(test_uuid)
        assert doc_id.as_uuid() == test_uuid
        assert doc_id.as_str() == str(test_uuid)

    def test_parse_from_string(self):
        """Test parsing from string."""
        test_uuid_str = str(uuid.uuid4())
        doc_id = ValidatedDocumentId.parse(test_uuid_str)
        assert doc_id.as_str() == test_uuid_str

    def test_invalid_ids(self):
        """Test invalid ID rejection."""
        invalid_ids = [
            "",  # Empty
            "not-a-uuid",  # Invalid format
            "00000000-0000-0000-0000-000000000000",  # Nil UUID
        ]

        for invalid_id in invalid_ids:
            with pytest.raises(ValidationError):
                ValidatedDocumentId.parse(invalid_id)

    def test_equality(self):
        """Test ID equality comparisons."""
        test_uuid = uuid.uuid4()
        id1 = ValidatedDocumentId.from_uuid(test_uuid)
        id2 = ValidatedDocumentId.from_uuid(test_uuid)
        id3 = ValidatedDocumentId.new()

        assert id1 == id2
        assert id1 != id3
        assert id1 == str(test_uuid)
        assert id1 == test_uuid


class TestValidatedTitle:
    """Test ValidatedTitle class."""

    def test_valid_titles(self):
        """Test valid title creation."""
        valid_titles = [
            "Simple Title",
            "Title with Numbers 123",
            "   Title with whitespace   ",  # Should be trimmed
            "A" * 1000,  # Max length
        ]

        for title in valid_titles:
            validated = ValidatedTitle(title)
            assert validated.as_str() == title.strip()

    def test_alternative_constructor(self):
        """Test the alternative .new() constructor."""
        title = "Test Title"
        validated = ValidatedTitle.new(title)
        assert validated.as_str() == title

    def test_invalid_titles(self):
        """Test invalid title rejection."""
        invalid_titles = [
            "",  # Empty
            "   ",  # Only whitespace
            "A" * 2000,  # Too long
        ]

        for title in invalid_titles:
            with pytest.raises(ValidationError):
                ValidatedTitle(title)

    def test_whitespace_trimming(self):
        """Test that whitespace is properly trimmed."""
        title = ValidatedTitle("  Test Title  ")
        assert title.as_str() == "Test Title"


class TestNonZeroSize:
    """Test NonZeroSize class."""

    def test_valid_sizes(self):
        """Test valid size creation."""
        valid_sizes = [1, 100, 1024, 1000000]

        for size in valid_sizes:
            validated = NonZeroSize(size)
            assert validated.get() == size
            assert int(validated) == size

    def test_alternative_constructor(self):
        """Test the alternative .new() constructor."""
        size = 1024
        validated = NonZeroSize.new(size)
        assert validated.get() == size

    def test_invalid_sizes(self):
        """Test invalid size rejection."""
        invalid_sizes = [0, -1, -100]

        for size in invalid_sizes:
            with pytest.raises(ValidationError):
                NonZeroSize(size)

    def test_size_limit(self):
        """Test size limit enforcement."""
        # Very large size should fail
        with pytest.raises(ValidationError):
            NonZeroSize(200 * 1024 * 1024)  # 200MB


class TestValidatedTimestamp:
    """Test ValidatedTimestamp class."""

    def test_valid_timestamps(self):
        """Test valid timestamp creation."""
        valid_timestamps = [
            1609459200,  # 2021-01-01
            1640995200,  # 2022-01-01
            1234567890,  # Random valid timestamp
        ]

        for timestamp in valid_timestamps:
            validated = ValidatedTimestamp(timestamp)
            assert validated.as_secs() == timestamp
            assert int(validated) == timestamp

    def test_alternative_constructor(self):
        """Test the alternative .new() constructor."""
        timestamp = 1609459200
        validated = ValidatedTimestamp.new(timestamp)
        assert validated.as_secs() == timestamp

    def test_now_constructor(self):
        """Test the .now() constructor."""
        timestamp = ValidatedTimestamp.now()
        assert isinstance(timestamp.as_secs(), int)
        assert timestamp.as_secs() > 0

    def test_invalid_timestamps(self):
        """Test invalid timestamp rejection."""
        invalid_timestamps = [
            0,  # Zero
            -1,  # Negative
            40000000000,  # Too far in future (year 3000+)
        ]

        for timestamp in invalid_timestamps:
            with pytest.raises(ValidationError):
                ValidatedTimestamp(timestamp)

    def test_equality(self):
        """Test timestamp equality comparisons."""
        timestamp = 1609459200
        ts1 = ValidatedTimestamp(timestamp)
        ts2 = ValidatedTimestamp(timestamp)
        ts3 = ValidatedTimestamp(timestamp + 1)

        assert ts1 == ts2
        assert ts1 != ts3
        assert ts1 == timestamp
        assert ts1 != timestamp + 1


if __name__ == "__main__":
    pytest.main([__file__])
