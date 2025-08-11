#!/usr/bin/env python3
"""
KotaDB Python Client Demo
=========================

This example demonstrates the full capabilities of the KotaDB Python client,
including document management, search operations, and error handling.

Prerequisites:
1. Start the KotaDB server:
   cargo run -- --config kotadb-dev.toml
   
2. Install the Python client:
   pip install -e ./clients/python

3. Run this example:
   python examples/python_kotadb_demo.py
"""

import json
import sys
import time
from datetime import datetime
from pathlib import Path

# Add the client to path if running from examples directory
sys.path.insert(0, str(Path(__file__).parent.parent / "clients" / "python"))

try:
    from kotadb import KotaDB
    from kotadb.exceptions import NotFoundError, ConnectionError, KotaDBError
except ImportError:
    print("Error: KotaDB client not found. Please install it first:")
    print("  pip install -e ./clients/python")
    sys.exit(1)


def print_section(title):
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print('=' * 60)


def print_document(doc, indent=""):
    """Pretty print a document."""
    print(f"{indent}ID: {doc.id}")
    print(f"{indent}Path: {doc.path}")
    print(f"{indent}Title: {doc.title}")
    print(f"{indent}Tags: {', '.join(doc.tags) if doc.tags else 'None'}")
    print(f"{indent}Size: {doc.size} bytes")
    print(f"{indent}Created: {doc.created_at}")
    if hasattr(doc, 'content'):
        preview = doc.content[:100] + "..." if len(doc.content) > 100 else doc.content
        print(f"{indent}Content: {preview}")


def demo_connection_management(db_url="http://localhost:8080"):
    """Demonstrate different connection patterns."""
    print_section("Connection Management")
    
    # Basic connection
    print("\n1. Basic connection:")
    db = KotaDB(db_url)
    health = db.health()
    print(f"   Server status: {health.get('status', 'unknown')}")
    
    # Context manager (auto-closes connection)
    print("\n2. Context manager pattern:")
    with KotaDB(db_url) as db:
        try:
            stats = db.stats()
            print(f"   Documents in database: {stats.get('document_count', 0)}")
            print(f"   Total size: {stats.get('total_size', 0)} bytes")
        except (NotFoundError, KotaDBError):
            # Stats endpoint might not be implemented
            print("   Stats endpoint not available (optional feature)")
    
    # Connection with custom timeout
    print("\n3. Custom timeout configuration:")
    db = KotaDB(db_url, timeout=10)
    print(f"   Configured with custom timeout settings")
    
    return db


def demo_document_operations(db):
    """Demonstrate CRUD operations on documents."""
    print_section("Document Operations")
    
    documents_created = []
    
    try:
        # Create documents
        print("\n1. Creating documents:")
        
        # Technical documentation
        doc1_id = db.insert({
            "path": "/docs/python-guide.md",
            "title": "Python Programming Guide",
            "content": """# Python Programming Guide
            
This guide covers Python best practices, including:
- Code organization and structure
- Error handling patterns
- Testing strategies
- Performance optimization
- Async programming with asyncio
- Type hints and static analysis
            """,
            "tags": ["python", "programming", "guide", "documentation"],
            "metadata": {
                "author": "Demo Script",
                "version": "1.0",
                "difficulty": "intermediate"
            }
        })
        documents_created.append(doc1_id)
        print(f"   Created: Python Guide (ID: {doc1_id[:8]}...)")
        
        # Meeting notes
        doc2_id = db.insert({
            "path": "/meetings/2024-q1-planning.md",
            "title": "Q1 2024 Planning Meeting",
            "content": """# Q1 Planning Meeting Notes

**Date**: January 15, 2024
**Attendees**: Engineering Team

## Objectives
- Review Q4 2023 performance
- Set Q1 2024 goals
- Discuss resource allocation
- Plan feature roadmap

## Action Items
1. Complete migration to new infrastructure
2. Launch beta testing program
3. Hire 2 additional engineers
4. Implement automated testing pipeline
            """,
            "tags": ["meeting", "planning", "q1-2024", "roadmap"],
            "metadata": {
                "meeting_date": "2024-01-15",
                "attendee_count": 12,
                "follow_up_required": True
            }
        })
        documents_created.append(doc2_id)
        print(f"   Created: Meeting Notes (ID: {doc2_id[:8]}...)")
        
        # Code snippet
        doc3_id = db.insert({
            "path": "/snippets/database-connection.py",
            "title": "Database Connection Helper",
            "content": """import psycopg2
from contextlib import contextmanager

@contextmanager
def get_db_connection(host, database, user, password):
    '''Context manager for database connections'''
    conn = None
    try:
        conn = psycopg2.connect(
            host=host,
            database=database,
            user=user,
            password=password
        )
        yield conn
    finally:
        if conn:
            conn.close()
            """,
            "tags": ["python", "database", "postgresql", "snippet"],
            "metadata": {
                "language": "python",
                "framework": "psycopg2",
                "tested": True
            }
        })
        documents_created.append(doc3_id)
        print(f"   Created: Code Snippet (ID: {doc3_id[:8]}...)")
        
        # Retrieve a document
        print("\n2. Retrieving a document:")
        doc = db.get(doc1_id)
        print_document(doc, "   ")
        
        # Update a document
        print("\n3. Updating a document:")
        updated_doc = db.update(doc2_id, {
            "content": doc.content + "\n\n## Update: All action items completed!",
            "tags": ["meeting", "planning", "q1-2024", "roadmap", "completed"]
        })
        print(f"   Updated meeting notes with completion status")
        print(f"   New tags: {', '.join(updated_doc.tags)}")
        
        # List all documents (if endpoint is available)
        print("\n4. Listing all documents:")
        try:
            all_docs = db.list_all(limit=10)
            print(f"   Found {len(all_docs)} documents:")
            for doc in all_docs[:5]:  # Show first 5
                print(f"   - {doc.path}: {doc.title}")
        except (NotFoundError, KotaDBError):
            print("   List endpoint not available (optional feature)")
        
    except KotaDBError as e:
        print(f"   Error during document operations: {e}")
    
    return documents_created


def demo_search_operations(db):
    """Demonstrate different search capabilities."""
    print_section("Search Operations")
    
    try:
        # Text search
        print("\n1. Text search for 'python':")
        results = db.query("python", limit=5)
        print(f"   Found {results.total_count} results in {results.query_time_ms}ms")
        for i, result in enumerate(results.results[:3], 1):
            print(f"   {i}. {result.title}")
            # Score and preview not available in current API
        
        # Search with specific terms
        print("\n2. Search for 'planning meeting':")
        results = db.query("planning meeting", limit=5)
        print(f"   Found {results.total_count} results")
        for result in results.results:
            print(f"   - {result.title}")
            if result.tags:
                print(f"     Tags: {', '.join(result.tags)}")
        
        # Pattern-based search
        print("\n3. Search for code patterns:")
        results = db.query("database connection", limit=5)
        print(f"   Found {results.total_count} code-related results")
        for result in results.results:
            print(f"   - {result.path}: {result.title}")
        
    except KotaDBError as e:
        print(f"   Error during search: {e}")


def demo_bulk_operations(db, doc_ids):
    """Demonstrate bulk operations and performance."""
    print_section("Bulk Operations")
    
    try:
        # Bulk insert
        print("\n1. Bulk document creation:")
        bulk_ids = []
        start_time = time.time()
        
        for i in range(5):
            doc_id = db.insert({
                "path": f"/bulk/document-{i}.md",
                "title": f"Bulk Document #{i}",
                "content": f"This is bulk document {i} created for testing bulk operations.",
                "tags": ["bulk", "test", f"batch-{i//2}"]
            })
            bulk_ids.append(doc_id)
        
        elapsed = time.time() - start_time
        print(f"   Created 5 documents in {elapsed:.3f} seconds")
        print(f"   Average: {elapsed/5:.3f} seconds per document")
        
        # Bulk retrieval with pagination (if available)
        print("\n2. Paginated retrieval:")
        try:
            page1 = db.list_all(limit=3, offset=0)
            page2 = db.list_all(limit=3, offset=3)
            print(f"   Page 1: {len(page1)} documents")
            print(f"   Page 2: {len(page2)} documents")
        except (NotFoundError, KotaDBError):
            print("   Pagination endpoint not available (optional feature)")
        
        # Cleanup bulk documents
        print("\n3. Bulk deletion:")
        deleted_count = 0
        for doc_id in bulk_ids:
            try:
                db.delete(doc_id)
                deleted_count += 1
            except NotFoundError:
                pass
        print(f"   Deleted {deleted_count} bulk test documents")
        
    except KotaDBError as e:
        print(f"   Error during bulk operations: {e}")


def demo_error_handling(db):
    """Demonstrate error handling patterns."""
    print_section("Error Handling")
    
    print("\n1. Handling not found errors:")
    try:
        doc = db.get("00000000-0000-0000-0000-000000000000")  # Valid UUID format but doesn't exist
    except (NotFoundError, KotaDBError) as e:
        print(f"   ✓ Correctly caught error: {type(e).__name__}")
    
    print("\n2. Handling invalid document creation:")
    try:
        # Missing required field (path)
        doc_id = db.insert({
            "title": "Invalid Document",
            "content": "This should fail"
        })
    except KotaDBError as e:
        print(f"   ✓ Correctly caught validation error: {e}")
    
    print("\n3. Connection error simulation:")
    try:
        bad_db = KotaDB("http://localhost:9999")  # Wrong port
        bad_db.health()
    except (ConnectionError, KotaDBError) as e:
        print(f"   ✓ Correctly caught connection error")


def cleanup_demo_documents(db, doc_ids):
    """Clean up documents created during the demo."""
    print_section("Cleanup")
    
    print("\nRemoving demo documents...")
    removed = 0
    for doc_id in doc_ids:
        try:
            db.delete(doc_id)
            removed += 1
            print(f"   Deleted document {doc_id[:8]}...")
        except NotFoundError:
            pass  # Already deleted
        except KotaDBError as e:
            print(f"   Error deleting {doc_id[:8]}: {e}")
    
    print(f"\n✓ Cleanup complete. Removed {removed} documents.")


def main():
    """Run the complete KotaDB demo."""
    print("""
╔════════════════════════════════════════════════════════════╗
║           KotaDB Python Client Demo                        ║
║                                                            ║
║  This demo showcases the full capabilities of KotaDB's    ║
║  Python client including CRUD operations, search, and     ║
║  error handling.                                          ║
╚════════════════════════════════════════════════════════════╝
    """)
    
    # Check server availability
    db_url = "http://localhost:8080"
    print(f"Connecting to KotaDB server at {db_url}...")
    
    try:
        # Initialize connection
        db = demo_connection_management(db_url)
        
        # Run demos
        doc_ids = demo_document_operations(db)
        demo_search_operations(db)
        demo_bulk_operations(db, doc_ids)
        demo_error_handling(db)
        
        # Cleanup
        cleanup_demo_documents(db, doc_ids)
        
        print_section("Demo Complete!")
        print("\n✅ All demonstrations completed successfully!")
        print("\nNext steps:")
        print("1. Check out the Python client documentation")
        print("2. Explore the API reference at docs/api/")
        print("3. Build your own KotaDB application!")
        
    except ConnectionError:
        print("\n❌ Error: Could not connect to KotaDB server.")
        print("\nPlease ensure the server is running:")
        print("  cargo run -- --config kotadb-dev.toml")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()