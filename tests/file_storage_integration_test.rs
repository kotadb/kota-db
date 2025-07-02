// Integration test for FileStorage implementation
use kotadb::{create_file_storage, DocumentBuilder, Storage};
use anyhow::Result;
use tempfile::TempDir;

#[tokio::test]
async fn test_file_storage_basic_operations() -> Result<()> {
    // Create temporary directory
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Create wrapped FileStorage
    let mut storage = create_file_storage(db_path, Some(10)).await?;
    
    // Create test document
    let doc = DocumentBuilder::new()
        .path("/test/document.md")?
        .title("Test Document")?
        .content(b"This is a test document with some content for testing.")?
        .build()?;
    
    let doc_id = doc.id;
    
    // Test insert
    storage.insert(doc.clone()).await?;
    
    // Test get
    let retrieved = storage.get(&doc_id).await?;
    assert!(retrieved.is_some());
    let retrieved_doc = retrieved.unwrap();
    assert_eq!(retrieved_doc.id, doc_id);
    assert_eq!(retrieved_doc.title, "Test Document");
    
    // Test update
    let mut updated_doc = doc;
    updated_doc.title = "Updated Test Document".to_string();
    updated_doc.updated = chrono::Utc::now().timestamp();
    
    storage.update(updated_doc.clone()).await?;
    
    // Verify update
    let updated_retrieved = storage.get(&doc_id).await?;
    assert!(updated_retrieved.is_some());
    assert_eq!(updated_retrieved.unwrap().title, "Updated Test Document");
    
    // Test delete
    storage.delete(&doc_id).await?;
    
    // Verify deletion
    let deleted_check = storage.get(&doc_id).await?;
    assert!(deleted_check.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_file_storage_multiple_documents() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    let mut storage = create_file_storage(db_path, Some(10)).await?;
    
    // Create multiple documents
    let doc1 = DocumentBuilder::new()
        .path("/test/doc1.md")?
        .title("Document 1")?
        .content(b"Content for document 1")?
        .build()?;
    
    let doc2 = DocumentBuilder::new()
        .path("/test/doc2.md")?
        .title("Document 2")?
        .content(b"Content for document 2")?
        .build()?;
    
    // Insert both documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;
    
    // Verify both can be retrieved
    let retrieved1 = storage.get(&doc1.id).await?;
    let retrieved2 = storage.get(&doc2.id).await?;
    
    assert!(retrieved1.is_some());
    assert!(retrieved2.is_some());
    assert_eq!(retrieved1.unwrap().title, "Document 1");
    assert_eq!(retrieved2.unwrap().title, "Document 2");
    
    Ok(())
}

#[tokio::test]
async fn test_file_storage_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    let doc = DocumentBuilder::new()
        .path("/test/persistent.md")?
        .title("Persistent Document")?
        .content(b"This document should persist across storage instances")?
        .build()?;
    
    let doc_id = doc.id;
    
    // Create storage, insert document, then close
    {
        let mut storage = create_file_storage(db_path, Some(10)).await?;
        storage.insert(doc.clone()).await?;
        storage.sync().await?;
        // Storage goes out of scope here
    }
    
    // Create new storage instance and verify document persists
    {
        let storage = create_file_storage(db_path, Some(10)).await?;
        let retrieved = storage.get(&doc_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Persistent Document");
    }
    
    Ok(())
}