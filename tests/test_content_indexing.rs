// Integration test for content indexing functionality
use anyhow::Result;
use kotadb::{
    create_file_storage, create_trigram_index, DocumentBuilder, Index, MeteredIndex, QueryBuilder,
    Storage, TrigramIndex,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_trigram_content_indexing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let index_path = temp_dir.path().join("trigram_index");

    // Create storage and index
    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

    // Create test documents with different content
    let doc1 = DocumentBuilder::new()
        .path("/docs/rust-tutorial.md")?
        .title("Rust Programming Tutorial")?
        .content(b"Learn Rust programming language with this comprehensive tutorial covering ownership, borrowing, and lifetimes.".to_vec())
        .build()?;

    let doc2 = DocumentBuilder::new()
        .path("/docs/golang-guide.md")?
        .title("Go Programming Guide")?
        .content(b"Master Go programming with goroutines, channels, and concurrent programming patterns.".to_vec())
        .build()?;

    let doc3 = DocumentBuilder::new()
        .path("/docs/rust-async.md")?
        .title("Async Rust Programming")?
        .content(
            b"Advanced Rust async programming with tokio, futures, and async/await patterns."
                .to_vec(),
        )
        .build()?;

    // Insert documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;
    storage.insert(doc3.clone()).await?;

    // Index documents with content
    index.insert(doc1.id.clone(), doc1.path.clone()).await?;
    index.insert(doc2.id.clone(), doc2.path.clone()).await?;
    index.insert(doc3.id.clone(), doc3.path.clone()).await?;

    // Search for "rust" - should find doc1 and doc3
    let query = QueryBuilder::new()
        .with_text("rust")?
        .with_limit(10)?
        .build()?;

    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        2,
        "Should find 2 documents containing 'rust'"
    );

    // Verify the correct documents were found
    let result_ids: Vec<_> = results.iter().map(|id| id.as_uuid()).collect();
    assert!(result_ids.contains(&doc1.id.as_uuid()));
    assert!(result_ids.contains(&doc3.id.as_uuid()));
    assert!(!result_ids.contains(&doc2.id.as_uuid()));

    // Search for "programming" - should find all 3
    let query = QueryBuilder::new()
        .with_text("programming")?
        .with_limit(10)?
        .build()?;

    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        3,
        "Should find all 3 documents containing 'programming'"
    );

    // Search for "goroutines" - should only find doc2
    let query = QueryBuilder::new()
        .with_text("goroutines")?
        .with_limit(10)?
        .build()?;

    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Should find 1 document containing 'goroutines'"
    );
    assert_eq!(results[0].as_uuid(), doc2.id.as_uuid());

    // Test update with new content
    let mut updated_doc2 = doc2.clone();
    updated_doc2.content =
        b"Master Go programming and Rust interoperability with FFI bindings.".to_vec();
    updated_doc2.updated_at = chrono::Utc::now(); // Ensure timestamp is updated
    updated_doc2.size = updated_doc2.content.len(); // Update size too

    storage.update(updated_doc2.clone()).await?;
    index
        .update(updated_doc2.id.clone(), updated_doc2.path.clone())
        .await?;

    // Now searching for "rust" should find all 3 documents
    let query = QueryBuilder::new()
        .with_text("rust")?
        .with_limit(10)?
        .build()?;

    let results = index.search(&query).await?;
    assert_eq!(results.len(), 3, "Should find 3 documents after update");

    Ok(())
}

#[tokio::test]
async fn test_trigram_case_insensitive_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("trigram_index");
    let mut index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

    let doc = DocumentBuilder::new()
        .path("/test.md")?
        .title("Test Document")?
        .content(b"This is a TEST document with UPPERCASE and lowercase words.".to_vec())
        .build()?;

    index.insert(doc.id.clone(), doc.path.clone()).await?;

    // Search with different cases
    for search_term in &["test", "TEST", "Test", "TeSt"] {
        let query = QueryBuilder::new()
            .with_text(*search_term)?
            .with_limit(10)?
            .build()?;

        let results = index.search(&query).await?;
        assert_eq!(
            results.len(),
            1,
            "Case-insensitive search should find the document"
        );
        assert_eq!(results[0].as_uuid(), doc.id.as_uuid());
    }

    Ok(())
}
