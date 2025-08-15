// Integration test for DocumentBuilder.id() functionality (Issue #146)
// Tests that user-specified UUIDs are preserved, not ignored

use anyhow::Result;
use kotadb::{create_file_storage, DocumentBuilder, Storage};
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_document_builder_id_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    let mut storage = create_file_storage(storage_path, Some(100)).await?;

    // Create a specific UUID to test with
    let custom_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;

    // Create document with custom ID using the fixed DocumentBuilder
    let doc = DocumentBuilder::new()
        .id_from_uuid(custom_uuid)?
        .path("test/integration.md")?
        .title("Integration Test Document")?
        .content(b"This document has a custom UUID")
        .build()?;

    // Verify the document has the correct ID before inserting
    assert_eq!(doc.id.as_uuid(), custom_uuid);

    // Insert into storage
    storage.insert(doc.clone()).await?;

    // Retrieve the document and verify the ID is preserved
    let retrieved = storage.get(&doc.id).await?.unwrap();
    assert_eq!(retrieved.id.as_uuid(), custom_uuid);
    assert_eq!(retrieved.path.as_str(), "test/integration.md");
    assert_eq!(retrieved.title.as_str(), "Integration Test Document");

    Ok(())
}

#[tokio::test]
async fn test_document_builder_auto_id_still_works() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    let mut storage = create_file_storage(storage_path, Some(100)).await?;

    // Create document without specifying ID (should auto-generate)
    let doc = DocumentBuilder::new()
        .path("test/auto-id.md")?
        .title("Auto ID Document")?
        .content(b"This document gets an auto-generated UUID")
        .build()?;

    // Verify the document has a valid, non-nil UUID
    assert_ne!(doc.id.as_uuid(), Uuid::nil());

    // Insert into storage
    storage.insert(doc.clone()).await?;

    // Retrieve the document and verify everything works
    let retrieved = storage.get(&doc.id).await?.unwrap();
    assert_eq!(retrieved.id.as_uuid(), doc.id.as_uuid());
    assert_eq!(retrieved.path.as_str(), "test/auto-id.md");

    Ok(())
}

#[tokio::test]
async fn test_multiple_documents_different_ids() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    let mut storage = create_file_storage(storage_path, Some(100)).await?;

    // Create two documents with different custom IDs
    let uuid1 = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001")?;
    let uuid2 = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002")?;

    let doc1 = DocumentBuilder::new()
        .id_from_uuid(uuid1)?
        .path("test/doc1.md")?
        .title("Document 1")?
        .content(b"First document")
        .build()?;

    let doc2 = DocumentBuilder::new()
        .id_from_uuid(uuid2)?
        .path("test/doc2.md")?
        .title("Document 2")?
        .content(b"Second document")
        .build()?;

    // Insert both documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;

    // Retrieve by their specific IDs
    let retrieved1 = storage.get(&doc1.id).await?.unwrap();
    let retrieved2 = storage.get(&doc2.id).await?.unwrap();

    // Verify each document kept its assigned ID
    assert_eq!(retrieved1.id.as_uuid(), uuid1);
    assert_eq!(retrieved2.id.as_uuid(), uuid2);
    assert_eq!(retrieved1.title.as_str(), "Document 1");
    assert_eq!(retrieved2.title.as_str(), "Document 2");

    Ok(())
}
