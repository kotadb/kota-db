// Tests for Primary Index persistence functionality
// Covers edge cases like corrupted files, version migration, and recovery

use anyhow::Result;
use kotadb::{
    create_primary_index_for_tests, Index, QueryBuilder, ValidatedDocumentId,
    ValidatedPath,
};
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper to create a test index with a unique temporary directory
async fn create_test_index() -> Result<(Box<dyn Index>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");
    let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
    Ok((Box::new(index), temp_dir))
}

#[tokio::test]
async fn test_persistence_basic_save_and_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("persistence_test");

    // Create and populate index
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        // Insert multiple documents
        let doc1_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc1_path = ValidatedPath::new("/docs/first.md")?;
        index.insert(doc1_id.clone(), doc1_path.clone()).await?;

        let doc2_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc2_path = ValidatedPath::new("/docs/second.md")?;
        index.insert(doc2_id.clone(), doc2_path.clone()).await?;

        let doc3_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc3_path = ValidatedPath::new("/docs/third.md")?;
        index.insert(doc3_id.clone(), doc3_path.clone()).await?;

        // Explicitly flush to disk
        index.flush().await?;
    }

    // Verify files were created
    assert!(index_path.join("meta").join("metadata.json").exists());
    assert!(index_path.join("data").join("btree_data.json").exists());

    // Load index from disk
    {
        let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        // Verify all documents are present
        let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 3, "Should have all 3 documents after reload");
    }

    Ok(())
}

#[tokio::test]
async fn test_persistence_empty_index() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("empty_test");

    // Create empty index and flush
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        index.flush().await?;
    }

    // Load empty index
    {
        let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 0, "Empty index should have no documents");
    }

    Ok(())
}

#[tokio::test]
async fn test_persistence_corrupted_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("corrupted_meta_test");

    // Create index with data
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("/test.md")?;
        index.insert(doc_id, doc_path).await?;
        index.flush().await?;
    }

    // Corrupt metadata file
    let metadata_path = index_path.join("meta").join("metadata.json");
    fs::write(&metadata_path, "{ invalid json")?;

    // Try to load - should fail gracefully
    let result = create_primary_index_for_tests(index_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Should fail to load with corrupted metadata");
    let error_message = result.err().unwrap().to_string();
    assert!(
        error_message.contains("Failed to deserialize index metadata"),
        "Error should mention metadata deserialization: {}",
        error_message
    );

    Ok(())
}

#[tokio::test]
async fn test_persistence_corrupted_btree_data() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("corrupted_btree_test");

    // Create index with data
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("/test.md")?;
        index.insert(doc_id, doc_path).await?;
        index.flush().await?;
    }

    // Corrupt B+ tree data file
    let btree_path = index_path.join("data").join("btree_data.json");
    fs::write(&btree_path, "not valid json at all")?;

    // Try to load - should fail gracefully
    let result = create_primary_index_for_tests(index_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Should fail to load with corrupted B+ tree data");
    let error_message = result.err().unwrap().to_string();
    assert!(
        error_message.contains("Failed to deserialize B+ tree data"),
        "Error should mention B+ tree deserialization: {}",
        error_message
    );

    Ok(())
}

#[tokio::test]
async fn test_persistence_invalid_uuid_in_data() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("invalid_uuid_test");

    // Create index structure
    fs::create_dir_all(index_path.join("meta"))?;
    fs::create_dir_all(index_path.join("data"))?;
    fs::create_dir_all(index_path.join("wal"))?;

    // Write valid metadata
    let metadata = r#"{
        "version": 1,
        "document_count": 1,
        "created": 1000,
        "updated": 2000
    }"#;
    fs::write(index_path.join("meta").join("metadata.json"), metadata)?;

    // Write B+ tree data with invalid UUID
    let btree_data = r#"{
        "not-a-valid-uuid": "/test.md"
    }"#;
    fs::write(index_path.join("data").join("btree_data.json"), btree_data)?;

    // Try to load - should fail with UUID error
    let result = create_primary_index_for_tests(index_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Should fail with invalid UUID");
    let error_message = result.err().unwrap().to_string();
    assert!(
        error_message.contains("Invalid UUID"),
        "Error should mention invalid UUID: {}",
        error_message
    );

    Ok(())
}

#[tokio::test]
async fn test_persistence_invalid_path_in_data() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("invalid_path_test");

    // Create index structure
    fs::create_dir_all(index_path.join("meta"))?;
    fs::create_dir_all(index_path.join("data"))?;
    fs::create_dir_all(index_path.join("wal"))?;

    // Write valid metadata
    let metadata = r#"{
        "version": 1,
        "document_count": 1,
        "created": 1000,
        "updated": 2000
    }"#;
    fs::write(index_path.join("meta").join("metadata.json"), metadata)?;

    // Write B+ tree data with invalid path (contains ..)
    let uuid = Uuid::new_v4().to_string();
    let btree_data = format!(
        r#"{{
        "{}": "../../../etc/passwd"
    }}"#,
        uuid
    );
    fs::write(index_path.join("data").join("btree_data.json"), btree_data)?;

    // Try to load - should fail with path validation error
    let result = create_primary_index_for_tests(index_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Should fail with invalid path");
    let error_message = result.err().unwrap().to_string();
    assert!(
        error_message.contains("Invalid path"),
        "Error should mention invalid path: {}",
        error_message
    );

    Ok(())
}

#[tokio::test]
async fn test_persistence_missing_data_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("missing_data_test");

    // Create only metadata, no data directory
    fs::create_dir_all(index_path.join("meta"))?;
    let metadata = r#"{
        "version": 1,
        "document_count": 0,
        "created": 1000,
        "updated": 2000
    }"#;
    fs::write(index_path.join("meta").join("metadata.json"), metadata)?;

    // Should load successfully with empty data
    let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
    let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 0, "Should have no documents");

    Ok(())
}

#[tokio::test]
async fn test_persistence_incremental_updates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("incremental_test");

    let doc1_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let doc1_path = ValidatedPath::new("/first.md")?;

    let doc2_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let doc2_path = ValidatedPath::new("/second.md")?;

    // First session: add one document
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        index.insert(doc1_id.clone(), doc1_path.clone()).await?;
        index.flush().await?;
    }

    // Second session: verify first doc exists, add second
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 1, "Should have first document");

        index.insert(doc2_id.clone(), doc2_path.clone()).await?;
        index.flush().await?;
    }

    // Third session: verify both documents exist
    {
        let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 2, "Should have both documents");
    }

    Ok(())
}

#[tokio::test]
async fn test_persistence_large_dataset() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("large_dataset_test");

    const NUM_DOCS: usize = 1000;
    let mut doc_ids = Vec::new();

    // Create index with many documents
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        for i in 0..NUM_DOCS {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(&format!("/docs/doc_{:04}.md", i))?;
            doc_ids.push(doc_id.clone());
            index.insert(doc_id, doc_path).await?;
        }

        index.flush().await?;
    }

    // Verify all documents persist
    {
        let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let query = QueryBuilder::new()
            .with_text("*")?
            .with_limit(1000)?  // Use max allowed limit
            .build()?;
        let results = index.search(&query).await?;
        assert_eq!(
            results.len(),
            NUM_DOCS,
            "Should have all {} documents after reload",
            NUM_DOCS
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_persistence_concurrent_modifications() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("concurrent_test");

    // Create initial index
    {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("/initial.md")?;
        index.insert(doc_id, doc_path).await?;
        index.flush().await?;
    }

    // Simulate concurrent modifications by rapidly opening, modifying, and closing
    for i in 0..5 {
        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new(&format!("/concurrent_{}.md", i))?;
        index.insert(doc_id, doc_path).await?;
        index.flush().await?;
        // Index dropped here, simulating process exit
    }

    // Verify all documents are present
    {
        let index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;
        let query = QueryBuilder::new().with_text("*")?.with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(
            results.len(),
            6,
            "Should have initial + 5 concurrent documents"
        );
    }

    Ok(())
}