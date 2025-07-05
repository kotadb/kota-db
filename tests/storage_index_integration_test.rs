// Storage-Index Integration Tests - Stage 1: Test-Driven Development
// These tests define how FileStorage and PrimaryIndex work together

use anyhow::Result;
use kotadb::{create_file_storage, DocumentBuilder, Index, Query, Storage};
use kotadb::{ValidatedDocumentId, ValidatedPath, ValidatedTitle};
use tempfile::TempDir;
use uuid::Uuid;

// Helper to create coordinated storage and index
async fn create_coordinated_storage_index() -> Result<(
    impl Storage,
    impl Index,
    TempDir, // Keep the TempDir alive
)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Ensure the parent directories exist first
    let storage_path = format!("{}/storage", db_path);
    let index_path = format!("{}/index", db_path);

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&index_path)?;

    // Also ensure the subdirectories that FileStorage expects
    std::fs::create_dir_all(&format!("{}/documents", storage_path))?;
    std::fs::create_dir_all(&format!("{}/indices", storage_path))?;
    std::fs::create_dir_all(&format!("{}/wal", storage_path))?;
    std::fs::create_dir_all(&format!("{}/meta", storage_path))?;

    // Create storage and index in same directory structure
    let storage = create_file_storage(&storage_path, Some(100)).await?;

    let index = kotadb::create_primary_index_for_tests(&index_path).await?;

    Ok((storage, index, temp_dir))
}

#[cfg(test)]
mod storage_index_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_document_insert_updates_both_storage_and_index() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        // Create a test document
        let doc = DocumentBuilder::new()
            .path("/integration/test.md")?
            .title("Integration Test Document")?
            .content(b"This tests storage and index coordination")
            .build()?;

        let _doc_id = doc.id.clone();
        let doc_path = doc.path.clone();
        let validated_id = doc.id.clone();

        // Insert into both storage and index (simulating coordinated operation)
        storage.insert(doc.clone()).await?;
        index.insert(validated_id.clone(), doc_path.clone()).await?;

        // Verify storage has the document
        let stored_doc = storage.get(&validated_id).await?;
        assert!(stored_doc.is_some());
        assert_eq!(
            stored_doc.unwrap().title,
            ValidatedTitle::new("Integration Test Document")?
        );

        // Verify index can find the document
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 1);
        assert_eq!(index_results[0], validated_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_document_delete_removes_from_both_storage_and_index() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        // Create and insert document
        let doc = DocumentBuilder::new()
            .path("/integration/delete_test.md")?
            .title("Delete Test Document")?
            .content(b"This document will be deleted")
            .build()?;

        let _doc_id = doc.id.clone();
        let doc_path = doc.path.clone();
        let validated_id = doc.id.clone();

        // Insert into both
        storage.insert(doc).await?;
        index.insert(validated_id.clone(), doc_path).await?;

        // Delete from both (simulating coordinated operation)
        storage.delete(&validated_id).await?;
        index.delete(&validated_id).await?;

        // Verify removal from storage
        let stored_doc = storage.get(&validated_id).await?;
        assert!(stored_doc.is_none());

        // Verify removal from index
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_document_update_maintains_consistency() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        // Create original document
        let doc = DocumentBuilder::new()
            .path("/integration/update_test.md")?
            .title("Original Title")?
            .content(b"Original content")
            .build()?;

        let doc_id = doc.id.clone();
        let original_path = doc.path.clone();
        let validated_id = doc.id.clone();

        // Insert original
        storage.insert(doc.clone()).await?;
        index.insert(validated_id.clone(), original_path).await?;

        // Update document (same ID, different content)
        let updated_doc = DocumentBuilder::new()
            .path("/integration/updated_test.md")? // New path
            .title("Updated Title")?
            .content(b"Updated content")
            .build()?;

        // Manually set same ID to simulate update
        let mut updated_doc = updated_doc;
        updated_doc.id = validated_id.clone();
        updated_doc.updated_at = chrono::Utc::now();

        let new_path = updated_doc.path.clone();

        // Update both storage and index
        storage.update(updated_doc.clone()).await?;
        index.insert(validated_id.clone(), new_path.clone()).await?; // Overwrite in index

        // Verify storage has updated document
        let stored_doc = storage.get(&validated_id).await?;
        assert!(stored_doc.is_some());
        assert_eq!(
            stored_doc.unwrap().title,
            ValidatedTitle::new("Updated Title")?
        );

        // Verify index has updated document
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 1);
        assert_eq!(index_results[0], validated_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_documents_coordination() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        let mut doc_ids = Vec::new();
        let mut _doc_paths = Vec::new();

        // Create and insert multiple documents
        for i in 0..5 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/multi_{}.md", i))?
                .title(&format!("Multi Document {}", i))?
                .content(format!("Content for document {}", i).as_bytes())
                .build()?;

            let _doc_id = doc.id.clone();
            let doc_path = doc.path.clone();
            let validated_id = doc.id.clone();

            // Coordinate storage and index insertion
            storage.insert(doc).await?;
            index.insert(validated_id.clone(), doc_path.clone()).await?;

            doc_ids.push(validated_id);
            _doc_paths.push(doc_path);
        }

        // Verify all documents in storage
        for validated_id in &doc_ids {
            let stored_doc = storage.get(validated_id).await?;
            assert!(stored_doc.is_some());
        }

        // Verify all documents in index
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_storage_index_operations() -> Result<()> {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let (storage, index, _temp_dir) = create_coordinated_storage_index().await?;
        let storage = Arc::new(Mutex::new(storage));
        let index = Arc::new(Mutex::new(index));

        let mut handles = Vec::new();

        // Spawn concurrent operations
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let index_clone = Arc::clone(&index);

            let handle = tokio::spawn(async move {
                let doc = DocumentBuilder::new()
                    .path(&format!("/integration/concurrent_{}.md", i))
                    .unwrap()
                    .title(&format!("Concurrent Document {}", i))
                    .unwrap()
                    .content(format!("Concurrent content {}", i).as_bytes())
                    .build()
                    .unwrap();

                let _doc_id = doc.id.clone();
                let doc_path = doc.path.clone();
                let validated_id = doc.id.clone();

                // Coordinate insertion
                {
                    let mut storage_guard = storage_clone.lock().await;
                    storage_guard.insert(doc).await.unwrap();
                }

                {
                    let mut index_guard = index_clone.lock().await;
                    index_guard.insert(validated_id, doc_path).await.unwrap();
                }
            });

            handles.push(handle);
        }

        // Wait for all operations
        for handle in handles {
            handle.await?;
        }

        // Verify coordination worked
        let _storage_guard = storage.lock().await;
        let index_guard = index.lock().await;

        let query = Query::new(Some("*".to_string()), None, None, 20)?;
        let index_results = index_guard.search(&query).await?;
        assert_eq!(index_results.len(), 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_persistence_coordination() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().to_str().unwrap();
        let storage_path = format!("{}/storage", db_path);
        let index_path = format!("{}/index", db_path);

        // Ensure directories exist
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&index_path)?;
        std::fs::create_dir_all(&format!("{}/documents", storage_path))?;
        std::fs::create_dir_all(&format!("{}/indices", storage_path))?;
        std::fs::create_dir_all(&format!("{}/wal", storage_path))?;
        std::fs::create_dir_all(&format!("{}/meta", storage_path))?;

        let doc_id = Uuid::new_v4();
        let doc_path = "/integration/persistent.md";

        // Create and populate storage and index
        {
            let mut storage = create_file_storage(&storage_path, Some(100)).await?;
            let mut index = kotadb::create_primary_index_for_tests(&index_path).await?;

            let doc = DocumentBuilder::new()
                .path(doc_path)?
                .title("Persistent Integration Test")?
                .content(b"This should persist across restarts")
                .build()?;

            // Manually set the ID for predictable testing
            let mut doc = doc;
            doc.id = ValidatedDocumentId::from_uuid(doc_id)?;

            let doc_path_validated = ValidatedPath::new(doc_path)?;
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;

            println!("Document title before insert: {:?}", doc.title);
            println!(
                "Document content before insert: {:?}",
                String::from_utf8_lossy(&doc.content)
            );

            storage.insert(doc).await?;
            index.insert(validated_id, doc_path_validated).await?;

            // Ensure persistence
            storage.sync().await?;
            index.flush().await?;
        }

        // Recreate storage and index from same paths
        {
            let storage = create_file_storage(&storage_path, Some(100)).await?;
            let index = kotadb::create_primary_index_for_tests(&index_path).await?;

            // Verify document persisted in storage
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
            let stored_doc = storage.get(&validated_id).await?;
            assert!(stored_doc.is_some());
            let stored_doc = stored_doc.unwrap();
            println!("Stored document title: {:?}", stored_doc.title);
            println!(
                "Stored document content: {:?}",
                String::from_utf8_lossy(&stored_doc.content)
            );
            assert_eq!(
                stored_doc.title,
                ValidatedTitle::new("Persistent Integration Test")?
            );

            // Verify document persisted in index
            let query = Query::new(Some("*".to_string()), None, None, 10)?;
            let index_results = index.search(&query).await?;
            assert_eq!(index_results.len(), 1);
        }

        // Keep temp_dir alive
        drop(temp_dir);
        Ok(())
    }
}

#[cfg(test)]
mod storage_index_performance_integration_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_coordinated_insert_performance() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        let start = Instant::now();

        // Insert 100 documents to both storage and index
        for i in 0..100 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/perf_{}.md", i))?
                .title(&format!("Performance Document {}", i))?
                .content(format!("Performance test content {}", i).as_bytes())
                .build()?;

            let _doc_id = doc.id.clone();
            let doc_path = doc.path.clone();
            let validated_id = doc.id.clone();

            // Coordinate insertion (simulates real coordination overhead)
            storage.insert(doc).await?;
            index.insert(validated_id, doc_path).await?;
        }

        let duration = start.elapsed();
        let avg_per_coordinated_insert = duration / 100;

        // Should be faster than 10ms per coordinated operation
        assert!(
            avg_per_coordinated_insert.as_millis() < 10,
            "Coordinated insert too slow: {:?} per operation",
            avg_per_coordinated_insert
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_coordinated_search_performance() -> Result<()> {
        let (mut storage, mut index, _temp_dir) = create_coordinated_storage_index().await?;

        // Pre-populate with 1000 documents
        for i in 0..1000 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/search_perf_{}.md", i))?
                .title(&format!("Search Performance Document {}", i))?
                .content(format!("Search performance test content {}", i).as_bytes())
                .build()?;

            let _doc_id = doc.id.clone();
            let doc_path = doc.path.clone();
            let validated_id = doc.id.clone();

            storage.insert(doc).await?;
            index.insert(validated_id, doc_path).await?;
        }

        let start = Instant::now();

        // Perform coordinated searches (index lookup + storage retrieval)
        for i in 0..100 {
            // Use index to find paths, then retrieve from storage
            let query = Query::new(Some("*".to_string()), None, None, 10)?;
            let index_results = index.search(&query).await?;

            // For each result, we'd typically retrieve from storage
            // This simulates the coordinated operation
            if !index_results.is_empty() {
                // In real implementation, we'd use the path to find document ID
                // then retrieve from storage
            }
        }

        let duration = start.elapsed();
        let avg_per_coordinated_search = duration / 100;

        // Should be faster than 5ms per coordinated search
        assert!(
            avg_per_coordinated_search.as_millis() < 5,
            "Coordinated search too slow: {:?} per operation",
            avg_per_coordinated_search
        );

        Ok(())
    }
}
