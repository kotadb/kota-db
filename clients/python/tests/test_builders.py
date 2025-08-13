"""
Tests for KotaDB builder patterns.
"""

import pytest

from kotadb.builders import DocumentBuilder, QueryBuilder, UpdateBuilder
from kotadb.validated_types import ValidatedDocumentId, ValidatedPath, ValidatedTitle
from kotadb.validation import ValidationError


class TestDocumentBuilder:
    """Test DocumentBuilder class."""

    def test_basic_document_building(self):
        """Test basic document construction."""
        builder = DocumentBuilder()
        doc_request = (
            builder.path("/notes/test.md")
            .title("Test Document")
            .content("This is test content")
            .build()
        )

        assert doc_request.path == "/notes/test.md"
        assert doc_request.title == "Test Document"
        assert doc_request.content == "This is test content"
        assert doc_request.tags is None
        assert doc_request.metadata is None

    def test_document_with_tags_and_metadata(self):
        """Test document building with tags and metadata."""
        doc_request = (
            DocumentBuilder()
            .path("/notes/test.md")
            .title("Test Document")
            .content("This is test content")
            .add_tag("work")
            .add_tag("meeting")
            .add_metadata("priority", "high")
            .add_metadata("author", "user123")
            .build()
        )

        assert doc_request.tags == ["work", "meeting"]
        assert doc_request.metadata == {"priority": "high", "author": "user123"}

    def test_document_with_validated_types(self):
        """Test document building with validated types."""
        path = ValidatedPath("/notes/test.md")
        title = ValidatedTitle("Test Document")
        doc_id = ValidatedDocumentId.new()

        doc_request = (
            DocumentBuilder().path(path).title(title).content("Test content").id(doc_id).build()
        )

        assert doc_request.path == path.as_str()
        assert doc_request.title == title.as_str()

    def test_tag_deduplication(self):
        """Test that duplicate tags are not added."""
        doc_request = (
            DocumentBuilder()
            .path("/notes/test.md")
            .title("Test Document")
            .content("Test content")
            .add_tag("work")
            .add_tag("work")  # Duplicate
            .add_tag("meeting")
            .build()
        )

        assert doc_request.tags == ["work", "meeting"]

    def test_tags_replacement(self):
        """Test replacing all tags."""
        doc_request = (
            DocumentBuilder()
            .path("/notes/test.md")
            .title("Test Document")
            .content("Test content")
            .add_tag("old_tag")
            .tags(["new_tag1", "new_tag2"])
            .build()
        )

        assert doc_request.tags == ["new_tag1", "new_tag2"]

    def test_auto_id_generation(self):
        """Test automatic ID generation."""
        builder = DocumentBuilder().auto_id()
        assert builder._id is not None
        assert isinstance(builder._id, ValidatedDocumentId)

    def test_missing_required_fields(self):
        """Test validation of required fields."""
        # Missing path
        with pytest.raises(ValidationError, match="path is required"):
            DocumentBuilder().title("Test").content("Content").build()

        # Missing title
        with pytest.raises(ValidationError, match="title is required"):
            DocumentBuilder().path("/test.md").content("Content").build()

        # Missing content
        with pytest.raises(ValidationError, match="content is required"):
            DocumentBuilder().path("/test.md").title("Test").build()

    def test_invalid_path(self):
        """Test invalid path rejection."""
        with pytest.raises(ValidationError):
            DocumentBuilder().path("../../../etc/passwd")

    def test_invalid_title(self):
        """Test invalid title rejection."""
        with pytest.raises(ValidationError):
            DocumentBuilder().title("")

    def test_invalid_tag(self):
        """Test invalid tag rejection."""
        with pytest.raises(ValidationError):
            DocumentBuilder().add_tag("invalid@tag")

    def test_build_with_timestamps(self):
        """Test building a complete document with timestamps."""
        doc = (
            DocumentBuilder()
            .path("/notes/test.md")
            .title("Test Document")
            .content("Test content")
            .build_with_timestamps()
        )

        assert doc.id is not None
        assert doc.path == "/notes/test.md"
        assert doc.title == "Test Document"
        assert doc.content == "Test content"
        assert doc.created_at is not None
        assert doc.updated_at is not None
        assert doc.size > 0


class TestQueryBuilder:
    """Test QueryBuilder class."""

    def test_basic_query_building(self):
        """Test basic query construction."""
        params = QueryBuilder().text("rust patterns").limit(10).offset(5).build()

        assert params["q"] == "rust patterns"
        assert params["limit"] == 10
        assert params["offset"] == 5

    def test_semantic_query_building(self):
        """Test semantic query construction."""
        data = QueryBuilder().text("machine learning algorithms").limit(20).build_for_semantic()

        assert data["query"] == "machine learning algorithms"
        assert data["limit"] == 20

    def test_hybrid_query_building(self):
        """Test hybrid query construction."""
        data = (
            QueryBuilder()
            .text("database optimization")
            .semantic_weight(0.8)
            .limit(15)
            .build_for_hybrid()
        )

        assert data["query"] == "database optimization"
        assert data["semantic_weight"] == 0.8
        assert data["limit"] == 15

    def test_query_with_filters(self):
        """Test query with filters."""
        params = (
            QueryBuilder()
            .text("search term")
            .tag_filter("work")
            .path_filter("/notes/*")
            .add_filter("custom_field", "custom_value")
            .build()
        )

        assert params["tag"] == "work"
        assert params["path"] == "/notes/*"
        assert params["custom_field"] == "custom_value"

    def test_query_without_offset(self):
        """Test that offset is not included when zero."""
        params = QueryBuilder().text("test query").build()

        assert "offset" not in params

    def test_missing_query_text(self):
        """Test validation of required query text."""
        with pytest.raises(ValidationError, match="Query text is required"):
            QueryBuilder().limit(10).build()

    def test_invalid_query_text(self):
        """Test invalid query text rejection."""
        with pytest.raises(ValidationError):
            QueryBuilder().text("")

    def test_invalid_limit(self):
        """Test invalid limit rejection."""
        with pytest.raises(ValidationError, match="Limit must be positive"):
            QueryBuilder().limit(0)

        with pytest.raises(ValidationError, match="Limit too large"):
            QueryBuilder().limit(20000)

    def test_invalid_offset(self):
        """Test invalid offset rejection."""
        with pytest.raises(ValidationError, match="Offset cannot be negative"):
            QueryBuilder().offset(-1)

    def test_invalid_semantic_weight(self):
        """Test invalid semantic weight rejection."""
        with pytest.raises(ValidationError, match="Semantic weight must be between"):
            QueryBuilder().semantic_weight(-0.1)

        with pytest.raises(ValidationError, match="Semantic weight must be between"):
            QueryBuilder().semantic_weight(1.1)

    def test_invalid_tag_filter(self):
        """Test invalid tag filter rejection."""
        with pytest.raises(ValidationError):
            QueryBuilder().tag_filter("invalid@tag")


class TestUpdateBuilder:
    """Test UpdateBuilder class."""

    def test_basic_update_building(self):
        """Test basic update construction."""
        updates = UpdateBuilder().title("Updated Title").content("Updated content").build()

        assert updates["title"] == "Updated Title"
        assert updates["content"] == "Updated content"

    def test_tag_operations(self):
        """Test tag addition and removal."""
        updates = (
            UpdateBuilder().add_tag("new_tag").add_tag("another_tag").remove_tag("old_tag").build()
        )

        tag_ops = updates["_tag_operations"]
        assert "new_tag" in tag_ops["add"]
        assert "another_tag" in tag_ops["add"]
        assert "old_tag" in tag_ops["remove"]

    def test_tag_replacement(self):
        """Test tag replacement."""
        updates = (
            UpdateBuilder().add_tag("to_be_replaced").replace_tags(["new_tag1", "new_tag2"]).build()
        )

        assert updates["tags"] == ["new_tag1", "new_tag2"]
        assert "_tag_operations" not in updates

    def test_metadata_operations(self):
        """Test metadata operations."""
        updates = (
            UpdateBuilder()
            .add_metadata("new_field", "new_value")
            .add_metadata("updated_field", "updated_value")
            .remove_metadata("old_field")
            .build()
        )

        meta_ops = updates["_metadata_operations"]
        assert meta_ops["new_field"] == "new_value"
        assert meta_ops["updated_field"] == "updated_value"
        assert meta_ops["old_field"] is None

    def test_validated_title_update(self):
        """Test update with validated title."""
        title = ValidatedTitle("Validated Title")
        updates = UpdateBuilder().title(title).build()

        assert updates["title"] == title.as_str()

    def test_invalid_title_update(self):
        """Test invalid title rejection in updates."""
        with pytest.raises(ValidationError):
            UpdateBuilder().title("")

    def test_invalid_tag_operations(self):
        """Test invalid tag rejection in operations."""
        with pytest.raises(ValidationError):
            UpdateBuilder().add_tag("invalid@tag")

        with pytest.raises(ValidationError):
            UpdateBuilder().replace_tags(["valid_tag", "invalid@tag"])

    def test_tag_deduplication_in_operations(self):
        """Test that duplicate tags are not added in operations."""
        updates = UpdateBuilder().add_tag("duplicate").add_tag("duplicate").build()

        tag_ops = updates["_tag_operations"]
        assert tag_ops["add"].count("duplicate") == 1


if __name__ == "__main__":
    pytest.main([__file__])
