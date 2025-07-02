// Storage-Index Integration Tests - Stage 1: Test-Driven Development
// These tests define how FileStorage and PrimaryIndex work together

use kotadb::{create_file_storage, DocumentBuilder, Storage, Index, Query};
use kotadb::{ValidatedDocumentId, ValidatedPath};
use anyhow::Result;
use tempfile::TempDir;
use uuid::Uuid;

#[cfg(test)]
mod storage_index_integration_tests {
    use super::*;
    
    // Helper to create coordinated storage and index
    async fn create_coordinated_storage_index() -> Result<(
        impl Storage,
        impl Index<Key = ValidatedDocumentId, Value = ValidatedPath>
    )> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().to_str().unwrap();
        
        // Create storage and index in same directory structure
        let storage = create_file_storage(&format!("{}/storage", db_path), Some(100)).await?;
        
        let index = kotadb::create_primary_index_for_tests(&format!("{}/index", db_path)).await?;
        
        Ok((storage, index))
    }
    
    #[tokio::test]
    async fn test_document_insert_updates_both_storage_and_index() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        // Create a test document
        let doc = DocumentBuilder::new()
            .path("/integration/test.md")?
            .title("Integration Test Document")?
            .content(b"This tests storage and index coordination")?
            .build()?;
        
        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(&doc.path)?;
        let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
        
        // Insert into both storage and index (simulating coordinated operation)
        storage.insert(doc.clone()).await?;
        index.insert(validated_id.clone(), doc_path.clone()).await?;
        
        // Verify storage has the document
        let stored_doc = storage.get(&doc_id).await?;
        assert!(stored_doc.is_some());
        assert_eq!(stored_doc.unwrap().title, "Integration Test Document");
        
        // Verify index can find the document
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 1);
        assert_eq!(index_results[0], doc_path);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_document_delete_removes_from_both_storage_and_index() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        // Create and insert document
        let doc = DocumentBuilder::new()
            .path("/integration/delete_test.md")?
            .title("Delete Test Document")?
            .content(b"This document will be deleted")?
            .build()?;
        
        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(&doc.path)?;
        let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
        
        // Insert into both
        storage.insert(doc).await?;
        index.insert(validated_id.clone(), doc_path).await?;
        
        // Delete from both (simulating coordinated operation)
        storage.delete(&doc_id).await?;
        index.delete(&validated_id).await?;
        
        // Verify removal from storage
        let stored_doc = storage.get(&doc_id).await?;
        assert!(stored_doc.is_none());
        
        // Verify removal from index
        let query = Query::new(Some("*".to_string()), None, None, 10)?);
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_document_update_maintains_consistency() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        // Create original document
        let doc = DocumentBuilder::new()
            .path("/integration/update_test.md")?
            .title("Original Title")?
            .content(b"Original content")?
            .build()?;
        
        let doc_id = doc.id;
        let original_path = ValidatedPath::new(&doc.path)?;
        let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
        
        // Insert original
        storage.insert(doc.clone()).await?;
        index.insert(validated_id.clone(), original_path).await?;
        
        // Update document (same ID, different content)
        let updated_doc = DocumentBuilder::new()
            .path("/integration/updated_test.md")? // New path
            .title("Updated Title")?
            .content(b"Updated content")?
            .build()?;
        
        // Manually set same ID to simulate update
        let mut updated_doc = updated_doc;
        updated_doc.id = doc_id;
        updated_doc.updated = chrono::Utc::now().timestamp();
        
        let new_path = ValidatedPath::new(&updated_doc.path)?;
        
        // Update both storage and index
        storage.update(updated_doc.clone()).await?;
        index.insert(validated_id.clone(), new_path.clone()).await?; // Overwrite in index
        
        // Verify storage has updated document
        let stored_doc = storage.get(&doc_id).await?;
        assert!(stored_doc.is_some());
        assert_eq!(stored_doc.unwrap().title, "Updated Title");
        
        // Verify index has updated path
        let query = Query::new(Some("*".to_string()), None, None, 10)?);
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 1);
        assert_eq!(index_results[0], new_path);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_multiple_documents_coordination() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        let mut doc_ids = Vec::new();
        let mut doc_paths = Vec::new();
        
        // Create and insert multiple documents
        for i in 0..5 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/multi_{}.md", i))?
                .title(&format!("Multi Document {}", i))?
                .content(format!("Content for document {}", i).as_bytes())?
                .build()?;
            
            let doc_id = doc.id;
            let doc_path = ValidatedPath::new(&doc.path)?;
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
            
            // Coordinate storage and index insertion
            storage.insert(doc).await?;
            index.insert(validated_id, doc_path.clone()).await?;
            
            doc_ids.push(doc_id);
            doc_paths.push(doc_path);
        }
        
        // Verify all documents in storage
        for doc_id in &doc_ids {
            let stored_doc = storage.get(doc_id).await?;
            assert!(stored_doc.is_some());
        }
        
        // Verify all documents in index
        let query = Query::new(Some("*".to_string()), None, None, 10)?);
        let index_results = index.search(&query).await?;
        assert_eq!(index_results.len(), 5);
        
        for expected_path in &doc_paths {
            assert!(index_results.contains(expected_path));
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_concurrent_storage_index_operations() -> Result<()> {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        let (storage, index) = create_coordinated_storage_index().await?;
        let storage = Arc::new(Mutex::new(storage));
        let index = Arc::new(Mutex::new(index));
        
        let mut handles = Vec::new();
        
        // Spawn concurrent operations
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let index_clone = Arc::clone(&index);
            
            let handle = tokio::spawn(async move {
                let doc = DocumentBuilder::new()
                    .path(&format!("/integration/concurrent_{}.md", i)).unwrap()
                    .title(&format!("Concurrent Document {}", i)).unwrap()
                    .content(format!("Concurrent content {}", i).as_bytes()).unwrap()
                    .build().unwrap();
                
                let doc_id = doc.id;
                let doc_path = ValidatedPath::new(&doc.path).unwrap();
                let validated_id = ValidatedDocumentId::from_uuid(doc_id).unwrap();
                
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
        let storage_guard = storage.lock().await;
        let index_guard = index.lock().await;
        
        let query = Query::new(Some("*".to_string()), None, None, 20)?);
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
        
        let doc_id = Uuid::new_v4();
        let doc_path = "/integration/persistent.md";
        
        // Create and populate storage and index
        {
            let mut storage = create_file_storage(&storage_path, Some(100)).await?;
            let mut index = kotadb::create_primary_index_for_tests(&index_path).await?;
            
            let doc = DocumentBuilder::new()
                .path(doc_path)?
                .title("Persistent Integration Test")?
                .content(b"This should persist across restarts")?
                .build()?;
            
            // Manually set the ID for predictable testing
            let mut doc = doc;
            doc.id = doc_id;
            
            let doc_path_validated = ValidatedPath::new(doc_path)?;
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
            
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
            let stored_doc = storage.get(&doc_id).await?;
            assert!(stored_doc.is_some());
            assert_eq!(stored_doc.unwrap().title, "Persistent Integration Test");
            
            // Verify document persisted in index
            let query = Query::new(Some("*".to_string()), None, None, 10)?);
            let index_results = index.search(&query).await?;
            assert_eq!(index_results.len(), 1);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod storage_index_performance_integration_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_coordinated_insert_performance() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        let start = Instant::now();
        
        // Insert 100 documents to both storage and index
        for i in 0..100 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/perf_{}.md", i))?
                .title(&format!("Performance Document {}", i))?
                .content(format!("Performance test content {}", i).as_bytes())?
                .build()?;
            
            let doc_id = doc.id;
            let doc_path = ValidatedPath::new(&doc.path)?;
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
            
            // Coordinate insertion (simulates real coordination overhead)
            storage.insert(doc).await?;
            index.insert(validated_id, doc_path).await?;
        }
        
        let duration = start.elapsed();
        let avg_per_coordinated_insert = duration / 100;
        
        // Should be faster than 10ms per coordinated operation
        assert!(avg_per_coordinated_insert.as_millis() < 10, 
                "Coordinated insert too slow: {:?} per operation", avg_per_coordinated_insert);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_coordinated_search_performance() -> Result<()> {
        let (mut storage, mut index) = create_coordinated_storage_index().await?;
        
        // Pre-populate with 1000 documents
        for i in 0..1000 {
            let doc = DocumentBuilder::new()
                .path(&format!("/integration/search_perf_{}.md", i))?
                .title(&format!("Search Performance Document {}", i))?
                .content(format!("Search performance test content {}", i).as_bytes())?
                .build()?;
            
            let doc_id = doc.id;
            let doc_path = ValidatedPath::new(&doc.path)?;
            let validated_id = ValidatedDocumentId::from_uuid(doc_id)?;
            
            storage.insert(doc).await?;
            index.insert(validated_id, doc_path).await?;
        }
        
        let start = Instant::now();
        
        // Perform coordinated searches (index lookup + storage retrieval)
        for i in 0..100 {
            // Use index to find paths, then retrieve from storage
            let query = Query::new(Some("*".to_string()), None, None, 10)?);
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
        assert!(avg_per_coordinated_search.as_millis() < 5,
                "Coordinated search too slow: {:?} per operation", avg_per_coordinated_search);
        
        Ok(())
    }
}