// Primary Index Tests - Stage 1: Test-Driven Development
// These tests define the expected behavior before implementation

use anyhow::Result;
use kotadb::{Index, Query, ValidatedDocumentId, ValidatedPath};
use uuid::Uuid;

// Note: PrimaryIndex will be implemented to satisfy these tests
// For now, this file establishes the test-driven development approach

// Helper function to create test primary index
async fn create_test_index() -> Result<impl Index> {
    let test_dir = format!("test_data/primary_test_{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&test_dir).await?;
    let index_path = format!("{}/primary_index", test_dir);

    let result = kotadb::create_primary_index_for_tests(&index_path).await;

    // Clean up on test completion (best effort)
    tokio::spawn(async move {
        let _ = tokio::fs::remove_dir_all(&test_dir).await;
    });

    result
}

#[cfg(test)]
mod primary_index_tests {
    use super::*;

    #[tokio::test]
    async fn test_primary_index_insert_and_search() -> Result<()> {
        let mut index = create_test_index().await?;

        // Create test data
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/document.md")?;

        // Insert key-value pair
        index.insert(doc_id, doc_path.clone()).await?;

        // Search for the document by ID
        let query = Query::new(
            Some("*".to_string()), // Wildcard search for primary index
            None,                  // No tags
            None,                  // No date range
            10,                    // Limit
        )?;

        // For primary index, we'll need a specific query type for ID lookup
        // This test defines the expected interface
        let results = index.search(&query).await?;

        // Should find exactly one result
        assert_eq!(results.len(), 1);
        // Primary index returns document IDs, not paths
        assert_eq!(results[0], doc_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_insert_duplicate_key() -> Result<()> {
        let mut index = create_test_index().await?;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path1 = ValidatedPath::new("test/doc1.md")?;
        let path2 = ValidatedPath::new("test/doc2.md")?;

        // First insert
        index.insert(doc_id, path1).await?;

        // Second insert with same key should overwrite
        index.insert(doc_id, path2.clone()).await?;

        // Should find only the second value
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let results = index.search(&query).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], doc_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_delete() -> Result<()> {
        let mut index = create_test_index().await?;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/document.md")?;

        // Insert then delete
        index.insert(doc_id, doc_path).await?;
        index.delete(&doc_id).await?;

        // Should find no results
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let results = index.search(&query).await?;

        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_delete_nonexistent() -> Result<()> {
        let mut index = create_test_index().await?;

        let nonexistent_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

        // Delete should succeed even if key doesn't exist
        index.delete(&nonexistent_id).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_multiple_documents() -> Result<()> {
        let mut index = create_test_index().await?;

        // Create multiple documents
        let doc1_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc1_path = ValidatedPath::new("test/doc1.md")?;

        let doc2_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc2_path = ValidatedPath::new("test/doc2.md")?;

        let doc3_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc3_path = ValidatedPath::new("test/doc3.md")?;

        // Insert all documents
        index.insert(doc1_id, doc1_path.clone()).await?;
        index.insert(doc2_id, doc2_path.clone()).await?;
        index.insert(doc3_id, doc3_path.clone()).await?;

        // Search should return all documents
        let query = Query::new(Some("*".to_string()), None, None, 100)?;
        let results = index.search(&query).await?;

        assert_eq!(results.len(), 3);
        assert!(results.contains(&doc1_id));
        assert!(results.contains(&doc2_id));
        assert!(results.contains(&doc3_id));

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_persistence() -> Result<()> {
        let test_dir = format!("test_data/persistence_test_{}", uuid::Uuid::new_v4());
        tokio::fs::create_dir_all(&test_dir).await?;
        let index_path = format!("{}/persistent_index", test_dir);
        let index_path_str = &index_path;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/persistent.md")?;

        // Create index, insert data, and flush
        {
            let mut index = kotadb::create_primary_index_for_tests(index_path_str).await?;
            index.insert(doc_id, doc_path.clone()).await?;
            index.flush().await?;
        }

        // Create new index instance from same path
        {
            let index = kotadb::create_primary_index_for_tests(index_path_str).await?;

            // Should find the previously inserted document
            let query = Query::new(Some("*".to_string()), None, None, 10)?;
            let results = index.search(&query).await?;

            assert_eq!(results.len(), 1);
            assert_eq!(results[0], doc_id);
        }

        // Clean up test directory
        let _ = tokio::fs::remove_dir_all(&test_dir).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_empty_operations() -> Result<()> {
        let index = create_test_index().await?;

        // Search on empty index
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_large_dataset() -> Result<()> {
        let mut index = create_test_index().await?;

        // Insert 1000 documents
        let mut expected_paths = Vec::new();
        for i in 0..1000 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("test/doc_{i}.md"))?;

            index.insert(doc_id, doc_path.clone()).await?;
            expected_paths.push(doc_path);
        }

        // Search should return all documents
        let query = Query::new(Some("*".to_string()), None, None, 1000)?; // Maximum allowed limit
        let results = index.search(&query).await?;

        assert_eq!(results.len(), 1000);

        // We can't easily match paths to IDs in this test since they're randomized
        // Just verify we got the right count
        assert_eq!(results.len(), expected_paths.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_concurrent_access() -> Result<()> {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let index = Arc::new(Mutex::new(create_test_index().await?));
        let mut handles = Vec::new();

        // Spawn 10 concurrent insert operations
        for i in 0..10 {
            let index_clone = Arc::clone(&index);

            let handle = tokio::spawn(async move {
                let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
                let doc_path = ValidatedPath::new(format!("test/concurrent_{i}.md")).unwrap();

                let mut index_guard = index_clone.lock().await;
                index_guard.insert(doc_id, doc_path).await.unwrap();
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await?;
        }

        // Verify all documents were inserted
        let index_guard = index.lock().await;
        let query = Query::new(Some("*".to_string()), None, None, 20)?;
        let results = index_guard.search(&query).await?;

        assert_eq!(results.len(), 10);

        Ok(())
    }
}

#[cfg(test)]
mod primary_index_performance_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn test_primary_index_insert_performance() -> Result<()> {
        let mut index = create_test_index().await?;

        let start = Instant::now();

        // Insert 100 documents and measure time
        for i in 0..100 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("test/perf_{i}.md"))?;

            index.insert(doc_id, doc_path).await?;
        }

        let duration = start.elapsed();
        let avg_per_insert = duration / 100;
        println!(
            "Primary index insert benchmark: total {:?}, avg {:?}",
            duration, avg_per_insert
        );

        // Allow some headroom for CI variance but guard against major regressions.
        let total_limit = Duration::from_secs(3);
        assert!(
            duration <= total_limit,
            "Insert too slow: {:?} total for 100 operations (limit {:?})",
            duration,
            total_limit
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_search_performance() -> Result<()> {
        let mut index = create_test_index().await?;

        // Pre-populate with 1000 documents
        for i in 0..1000 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("test/search_perf_{i}.md"))?;

            index.insert(doc_id, doc_path).await?;
        }

        let start = Instant::now();

        // Perform 100 searches
        for _ in 0..100 {
            let query = Query::new(Some("*".to_string()), None, None, 10)?;
            let results = index.search(&query).await?;
        }

        let duration = start.elapsed();
        let avg_per_search = duration / 100;
        println!(
            "Primary index search benchmark: total {:?}, avg {:?}",
            duration, avg_per_search
        );

        let total_limit = Duration::from_secs(2);
        assert!(
            duration <= total_limit,
            "Search too slow: {:?} total for 100 queries (limit {:?})",
            duration,
            total_limit
        );

        Ok(())
    }
}

#[cfg(test)]
mod primary_index_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_primary_index_invalid_path() -> Result<()> {
        // Test that the index creation fails with invalid paths
        // This will be implemented when we have the actual PrimaryIndex::open method

        // Should fail for non-existent directory
        // Should fail for path without write permissions
        // Should fail for invalid path characters

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_disk_full_simulation() -> Result<()> {
        // Test behavior when disk is full
        // This will require implementation-specific error handling

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_corruption_recovery() -> Result<()> {
        // Test recovery from corrupted index files
        // This will be implemented with the actual file format

        Ok(())
    }
}
