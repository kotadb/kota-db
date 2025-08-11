#!/usr/bin/env python3
"""Test script for KotaDB Python client."""

import sys
import os
sys.path.insert(0, 'clients/python')

from kotadb import KotaDB
import json

def test_client():
    """Test basic client operations."""
    try:
        # Connect to the database
        print("Connecting to KotaDB...")
        db = KotaDB('http://localhost:8080')
        print("✓ Connection successful")
        
        # Check health
        health = db.health()
        print(f"✓ Health check: {health}")
        
        # Test insert
        print("\nTesting document insertion...")
        doc_id = db.insert({
            'path': '/test/document.md',
            'title': 'Test Document',
            'content': 'This is a test document with some content.',
            'tags': ['test', 'demo']
        })
        print(f"✓ Document created with ID: {doc_id}")
        
        # Test get
        print("\nTesting document retrieval...")
        doc = db.get(doc_id)
        print(f"✓ Retrieved document: {doc.title}")
        print(f"  Path: {doc.path}")
        print(f"  Content: {doc.content[:50]}...")
        print(f"  Tags: {doc.tags}")
        
        # Test update
        print("\nTesting document update...")
        updated_doc = db.update(doc_id, {
            'title': 'Updated Test Document',
            'content': list('Updated content for the test document.'.encode('utf-8')),
            'tags': ['test', 'demo', 'updated']
        })
        print(f"✓ Document updated: {updated_doc.title}")
        
        # Test search
        print("\nTesting search...")
        results = db.query('test', limit=10)
        print(f"✓ Search found {results.total_count} results")
        for result in results.results:
            print(f"  - {result.title} ({result.path})")
        
        # Test delete
        print("\nTesting document deletion...")
        success = db.delete(doc_id)
        print(f"✓ Document deleted: {success}")
        
        # Verify deletion
        try:
            db.get(doc_id)
            print("✗ Document still exists after deletion!")
        except Exception:
            print("✓ Document successfully deleted (get raises exception)")
        
        print("\n✅ All tests passed!")
        return True
        
    except Exception as e:
        print(f"\n✗ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == '__main__':
    success = test_client()
    sys.exit(0 if success else 1)