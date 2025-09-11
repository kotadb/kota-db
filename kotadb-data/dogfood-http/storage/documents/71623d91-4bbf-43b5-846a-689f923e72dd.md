---
tags:
- file
- kota-db
- ext_py
---
#!/usr/bin/env python3
"""
Basic usage example for KotaDB Python client.

This example demonstrates the core functionality of the KotaDB client
including document insertion, searching, and management.
"""

from kotadb import KotaDB
from kotadb.exceptions import KotaDBError, NotFoundError


def main():
    """Demonstrate basic KotaDB operations."""

    # Connect to KotaDB (make sure server is running on localhost:8080)
    try:
        db = KotaDB("http://localhost:8080")
        print("✅ Connected to KotaDB")
    except Exception as e:
        print(f"❌ Failed to connect: {e}")
        print("Make sure KotaDB server is running on localhost:8080")
        return

    try:
        # Check database health
        health = db.health()
        print(f"Database status: {health.get('status', 'unknown')}")

        # Insert some sample documents
        print("\n📝 Inserting sample documents...")

        docs = [
            {
                "path": "/docs/rust-guide.md",
                "title": "Rust Programming Guide",
                "content": "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection.",
                "tags": ["rust", "programming", "guide", "systems"],
            },
            {
                "path": "/docs/database-design.md",
                "title": "Database Design Patterns",
                "content": "Database design patterns for modern applications. Covers indexing strategies, query optimization, and data modeling best practices.",
                "tags": ["database", "design", "patterns", "optimization"],
            },
            {
                "path": "/notes/meeting-2024.md",
                "title": "Project Meeting Notes",
                "content": "Discussed the roadmap for Q1 2024. Key focus areas include performance optimization and new client libraries.",
                "tags": ["meeting", "planning", "roadmap", "2024"],
            },
        ]

        doc_ids = []
        for doc in docs:
            doc_id = db.insert(doc)
            doc_ids.append(doc_id)
            print(f"  Created document: {doc['title']} (ID: {doc_id})")

        # Perform text search
        print("\n🔍 Performing text search...")
        results = db.query("rust programming", limit=5)
        print(f"Found {results.total_count} results in {results.query_time_ms}ms:")
        for result in results.results:
            print(f"  - {result.document.title} (score: {result.score:.2f})")
            print(f"    Preview: {result.content_preview[:100]}...")

        # Perform semantic search (if enabled)
        print("\n🧠 Attempting semantic search...")
        try:
            semantic_results = db.semantic_search("programming languages and safety", limit=3)
            print(f"Found {semantic_results.total_count} semantic results:")
            for result in semantic_results.results:
                print(f"  - {result.document.title} (score: {result.score:.2f})")
        except Exception as e:
            print(f"  Semantic search not available: {e}")

        # Get a specific document
        print("\n📄 Retrieving specific document...")
        if doc_ids:
            doc = db.get(doc_ids[0])
            print(f"Retrieved: {doc.title}")
            print(f"Tags: {doc.tags}")
            print(f"Size: {doc.size} bytes")
            print(f"Created: {doc.created_at}")

        # Update a document
        print("\n✏️  Updating document...")
        if doc_ids:
            updated_doc = db.update(
                doc_ids[0],
                {
                    "content": doc.content
                    + "\n\nUPDATE: Added example code and additional resources."
                },
            )
            print(f"Updated document size: {updated_doc.size} bytes")

        # List all documents
        print("\n📋 Listing all documents...")
        all_docs = db.list_all(limit=10)
        print(f"Total documents: {len(all_docs)}")
        for doc in all_docs:
            print(f"  - {doc.title} ({doc.path})")

        # Get database statistics
        print("\n📊 Database statistics...")
        try:
            stats = db.stats()
            print(f"Document count: {stats.get('document_count', 'unknown')}")
            print(f"Total size: {stats.get('total_size_bytes', 'unknown')} bytes")
        except Exception as e:
            print(f"Stats not available: {e}")

        # Clean up (delete test documents)
        print("\n🗑️  Cleaning up test documents...")
        for doc_id in doc_ids:
            try:
                db.delete(doc_id)
                print(f"  Deleted document: {doc_id}")
            except NotFoundError:
                print(f"  Document {doc_id} already deleted")

        print("\n✅ Example completed successfully!")

    except KotaDBError as e:
        print(f"❌ KotaDB error: {e}")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
    finally:
        db.close()


if __name__ == "__main__":
    main()
