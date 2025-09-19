// Test suite for issue #221: Trigram index returning incorrect results for non-matching queries
// This test ensures that the trigram index doesn't return false positives

use anyhow::Result;
use kotadb::{
    create_file_storage, create_trigram_index_for_tests, DocumentBuilder, Index, QueryBuilder,
    Storage,
};
use tempfile::TempDir;

#[cfg_attr(
    feature = "aggressive-trigram-thresholds",
    ignore = "Aggressive trigram fallback relaxes strict zero-result expectations"
)]
#[tokio::test]
async fn test_nonexistent_search_returns_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let _primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&trigram_path)?;

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(10)).await?;
    let mut trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?;

    // Insert test documents with specific content
    let doc1 = DocumentBuilder::new()
        .path("test1.md")?
        .title("Test Document 1")?
        .content(b"The file storage module implements storage functionality")
        .build()?;

    let doc2 = DocumentBuilder::new()
        .path("test2.md")?
        .title("Test Document 2")?
        .content(b"This is about symbols and functions in programming")
        .build()?;

    let doc3 = DocumentBuilder::new()
        .path("test3.md")?
        .title("Test Document 3")?
        .content(b"Library entry point with no special keywords at all")
        .build()?;

    // Insert documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;
    storage.insert(doc3.clone()).await?;

    // Update trigram index with content
    trigram_index
        .insert_with_content(doc1.id, doc1.path.clone(), &doc1.content)
        .await?;
    trigram_index
        .insert_with_content(doc2.id, doc2.path.clone(), &doc2.content)
        .await?;
    trigram_index
        .insert_with_content(doc3.id, doc3.path.clone(), &doc3.content)
        .await?;

    // Test 1: Search for completely nonexistent term should return 0 results
    let query = QueryBuilder::new()
        .with_text("ZZZNONEXISTENT999")?
        .build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Search for nonexistent term should return 0 documents, got {}",
        results.len()
    );

    // Test 2: Search for another nonexistent term
    let query = QueryBuilder::new().with_text("XYZABC123NOTHERE")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Search for another nonexistent term should return 0 documents, got {}",
        results.len()
    );

    // Test 3: Search for existing term should return correct documents
    let query = QueryBuilder::new().with_text("storage")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Search for 'storage' should return 1 document, got {}",
        results.len()
    );
    assert_eq!(results[0], doc1.id);

    // Test 4: Search for another existing term
    let query = QueryBuilder::new().with_text("symbols")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Search for 'symbols' should return 1 document, got {}",
        results.len()
    );
    assert_eq!(results[0], doc2.id);

    // Test 5: Empty search should return empty results (not all documents)
    let query = QueryBuilder::new().build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Empty search should return 0 documents in trigram index, got {}",
        results.len()
    );

    // Test 6: Search with short non-matching string (less than 3 chars)
    let query = QueryBuilder::new().with_text("XY")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Search for short non-matching string should return 0 documents, got {}",
        results.len()
    );

    // Test 7: Search with special characters that don't exist
    // Note: Special characters like "@#$%^&*()" get sanitized to empty,
    // so we use alphanumeric characters with special chars that will remain after sanitization
    let query = QueryBuilder::new().with_text("xyz@#$%")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Search for text with special characters should return 0 documents, got {}",
        results.len()
    );

    // Test 8: Search with unicode that doesn't exist
    let query = QueryBuilder::new().with_text("你好世界")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "Search for non-existent unicode should return 0 documents, got {}",
        results.len()
    );

    // Test 9: Search with mixed case for existing term (should match due to case-insensitive)
    let query = QueryBuilder::new().with_text("STORAGE")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Case-insensitive search for 'STORAGE' should return 1 document, got {}",
        results.len()
    );

    // Test 10: Partial word match should work
    let query = QueryBuilder::new().with_text("stor")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert!(
        !results.is_empty(),
        "Partial match 'stor' should return at least 1 document"
    );

    Ok(())
}

#[tokio::test]
async fn test_trigram_threshold_filtering() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&trigram_path)?;

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(10)).await?;
    let mut trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?;

    // Create documents with very different content to avoid false matches
    let doc1 = DocumentBuilder::new()
        .path("doc1.md")?
        .title("Document Alpha")?
        .content(b"alpha beta gamma delta epsilon zeta eta theta")
        .build()?;

    let doc2 = DocumentBuilder::new()
        .path("doc2.md")?
        .title("Document Numbers")?
        .content(b"one two three four five six seven eight nine ten")
        .build()?;

    let doc3 = DocumentBuilder::new()
        .path("doc3.md")?
        .title("Document Code")?
        .content(b"function variable constant parameter argument return value type")
        .build()?;

    // Insert documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;
    storage.insert(doc3.clone()).await?;

    // Update trigram index
    trigram_index
        .insert_with_content(doc1.id, doc1.path.clone(), &doc1.content)
        .await?;
    trigram_index
        .insert_with_content(doc2.id, doc2.path.clone(), &doc2.content)
        .await?;
    trigram_index
        .insert_with_content(doc3.id, doc3.path.clone(), &doc3.content)
        .await?;

    // Test that searches for terms from one document don't return others
    let query = QueryBuilder::new().with_text("alpha beta")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Should only match document with 'alpha beta'"
    );
    assert_eq!(results[0], doc1.id);

    let query = QueryBuilder::new()
        .with_text("function variable")?
        .build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Should only match document with 'function variable'"
    );
    assert_eq!(results[0], doc3.id);

    // Test that gibberish doesn't match anything
    let query = QueryBuilder::new().with_text("xqz wpy vbn")?.build()?;
    let results = trigram_index.search(&query).await?;
    assert_eq!(results.len(), 0, "Gibberish should not match any documents");

    Ok(())
}
