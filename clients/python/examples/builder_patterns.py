#!/usr/bin/env python3
"""
Builder patterns and validated types example for KotaDB Python client.

This example demonstrates the new type safety features and builder patterns
that provide runtime validation equivalent to the Rust implementation.
"""

from kotadb import (
    ClientValidationError,
    DocumentBuilder,
    KotaDB,
    NonZeroSize,
    QueryBuilder,
    UpdateBuilder,
    ValidatedDocumentId,
    ValidatedPath,
    ValidatedTimestamp,
    ValidatedTitle,
)
from kotadb.exceptions import KotaDBError


def demonstrate_validated_types():
    """Demonstrate validated types with safety guarantees."""
    print("🔒 Demonstrating Validated Types")
    print("=" * 50)

    # ValidatedPath - prevents directory traversal, null bytes, etc.
    try:
        safe_path = ValidatedPath("/docs/guide.md")
        print(f"✅ Valid path: {safe_path}")
    except ClientValidationError as e:
        print(f"❌ Path validation failed: {e}")

    try:
        # This will fail - directory traversal attempt
        dangerous_path = ValidatedPath("../../../etc/passwd")
        print(f"❌ This should not print: {dangerous_path}")
    except ClientValidationError as e:
        print(f"✅ Correctly blocked dangerous path: {e}")

    # ValidatedDocumentId - ensures proper UUID format
    try:
        doc_id = ValidatedDocumentId.new()
        print(f"✅ Generated ID: {doc_id}")

        # Parse existing ID
        parsed_id = ValidatedDocumentId.parse(str(doc_id))
        print(f"✅ Parsed ID: {parsed_id}")
    except ClientValidationError as e:
        print(f"❌ ID validation failed: {e}")

    try:
        # This will fail - invalid UUID
        bad_id = ValidatedDocumentId.parse("not-a-uuid")
        print(f"❌ This should not print: {bad_id}")
    except ClientValidationError as e:
        print(f"✅ Correctly blocked invalid UUID: {e}")

    # ValidatedTitle - ensures non-empty, length limits
    try:
        title = ValidatedTitle("My Document Title")
        print(f"✅ Valid title: {title}")
    except ClientValidationError as e:
        print(f"❌ Title validation failed: {e}")

    try:
        # This will fail - empty title
        empty_title = ValidatedTitle("   ")
        print(f"❌ This should not print: {empty_title}")
    except ClientValidationError as e:
        print(f"✅ Correctly blocked empty title: {e}")

    # ValidatedTimestamp - ensures reasonable time values
    try:
        now = ValidatedTimestamp.now()
        print(f"✅ Current timestamp: {now}")
    except ClientValidationError as e:
        print(f"❌ Timestamp validation failed: {e}")

    # NonZeroSize - ensures positive sizes
    try:
        size = NonZeroSize(1024)
        print(f"✅ Valid size: {size} bytes")
    except ClientValidationError as e:
        print(f"❌ Size validation failed: {e}")

    try:
        # This will fail - zero size
        zero_size = NonZeroSize(0)
        print(f"❌ This should not print: {zero_size}")
    except ClientValidationError as e:
        print(f"✅ Correctly blocked zero size: {e}")

    print()


def demonstrate_document_builder():
    """Demonstrate document builder pattern."""
    print("🏗️  Demonstrating Document Builder")
    print("=" * 50)

    try:
        # Build a document with validation at each step
        doc_request = (
            DocumentBuilder()
            .path("/knowledge/rust-patterns.md")
            .title("Advanced Rust Design Patterns")
            .content(
                "# Rust Design Patterns\n\nThis document covers advanced patterns in Rust programming..."
            )
            .add_tag("rust")
            .add_tag("programming")
            .add_tag("patterns")
            .add_metadata("author", "rust-expert@example.com")
            .add_metadata("difficulty", "advanced")
            .add_metadata("estimated_read_time", "15 minutes")
            .build()
        )

        print(f"✅ Built document request: {doc_request.title}")
        print(f"   Path: {doc_request.path}")
        print(f"   Tags: {doc_request.tags}")
        print(f"   Metadata: {doc_request.metadata}")

        return doc_request

    except ClientValidationError as e:
        print(f"❌ Document builder validation failed: {e}")
        return None

    except Exception as e:
        print(f"❌ Unexpected error in document builder: {e}")
        return None


def demonstrate_query_builder():
    """Demonstrate query builder pattern."""
    print("🔍 Demonstrating Query Builder")
    print("=" * 50)

    try:
        # Build a text search query
        text_query = (
            QueryBuilder()
            .text("rust design patterns")
            .limit(10)
            .offset(0)
            .tag_filter("programming")
            .path_filter("/knowledge/*")
            .build()
        )

        print(f"✅ Built text query: {text_query}")

        # Build a semantic search query
        semantic_query = (
            QueryBuilder()
            .text("object-oriented programming concepts")
            .limit(5)
            .build_for_semantic()
        )

        print(f"✅ Built semantic query: {semantic_query}")

        # Build a hybrid search query
        hybrid_query = (
            QueryBuilder()
            .text("database optimization techniques")
            .semantic_weight(0.7)
            .limit(15)
            .add_filter("category", "performance")
            .build_for_hybrid()
        )

        print(f"✅ Built hybrid query: {hybrid_query}")

        return text_query, semantic_query, hybrid_query

    except ClientValidationError as e:
        print(f"❌ Query builder validation failed: {e}")
        return None, None, None

    except Exception as e:
        print(f"❌ Unexpected error in query builder: {e}")
        return None, None, None


def demonstrate_update_builder():
    """Demonstrate update builder pattern."""
    print("✏️  Demonstrating Update Builder")
    print("=" * 50)

    try:
        # Build an update with various operations
        updates = (
            UpdateBuilder()
            .title("Updated: Advanced Rust Design Patterns")
            .add_tag("updated")
            .add_tag("2024")
            .remove_tag("draft")
            .add_metadata("last_modified_by", "editor@example.com")
            .add_metadata("version", "2.0")
            .add_metadata("review_status", "approved")
            .build()
        )

        print(f"✅ Built update operations: {updates}")

        return updates

    except ClientValidationError as e:
        print(f"❌ Update builder validation failed: {e}")
        return None

    except Exception as e:
        print(f"❌ Unexpected error in update builder: {e}")
        return None


def demonstrate_with_database():
    """Demonstrate builders with actual database operations."""
    print("🗄️  Demonstrating Builders with Database")
    print("=" * 50)

    try:
        # Connect to database
        db = KotaDB("http://localhost:8080")
        print("✅ Connected to KotaDB")

        # Insert document using builder
        doc_id = db.insert_with_builder(
            DocumentBuilder()
            .path("/examples/builder-demo.md")
            .title("Builder Pattern Demo")
            .content("This document was created using the DocumentBuilder pattern for type safety.")
            .add_tag("demo")
            .add_tag("builder-pattern")
            .add_metadata("created_by", "builder_example")
        )
        print(f"✅ Inserted document with ID: {doc_id}")

        # Query using builder
        results = db.query_with_builder(
            QueryBuilder().text("builder pattern").limit(5).tag_filter("demo")
        )
        print(f"✅ Found {results.total_count} results using QueryBuilder")

        # Update using builder
        updated_doc = db.update_with_builder(
            doc_id,
            UpdateBuilder()
            .content("This document was created AND UPDATED using builder patterns!")
            .add_tag("updated")
            .add_metadata("last_update", "builder_example"),
        )
        print(f"✅ Updated document: {updated_doc.title}")

        # Semantic search using builder (if available)
        try:
            semantic_results = db.semantic_search_with_builder(
                QueryBuilder().text("software design patterns").limit(3)
            )
            print(f"✅ Found {semantic_results.total_count} semantic results")
        except Exception as e:
            print(f"ℹ️  Semantic search not available: {e}")

        # Clean up
        db.delete(doc_id)
        print(f"✅ Cleaned up document: {doc_id}")

        db.close()

    except KotaDBError as e:
        print(f"❌ Database error: {e}")
    except Exception as e:
        print(f"❌ Connection error: {e}")
        print("Make sure KotaDB server is running on localhost:8080")


def main():
    """Run all builder pattern demonstrations."""
    print("🎯 KotaDB Python Client - Builder Patterns & Type Safety Demo")
    print("=" * 70)
    print()

    # Demonstrate validated types
    demonstrate_validated_types()

    # Demonstrate document builder
    doc_request = demonstrate_document_builder()
    print()

    # Demonstrate query builder
    text_query, semantic_query, hybrid_query = demonstrate_query_builder()
    print()

    # Demonstrate update builder
    updates = demonstrate_update_builder()
    print()

    # Demonstrate with actual database (if available)
    demonstrate_with_database()
    print()

    print("🎉 Builder patterns demonstration complete!")
    print(
        "These patterns provide runtime type safety equivalent to Rust's compile-time guarantees."
    )


if __name__ == "__main__":
    main()
