// Test for issue #368 - Trigram index not populated during repository ingestion
// This test currently FAILS, demonstrating the issue
use anyhow::Result;
use kotadb::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_trigram_index_population_after_rebuild() -> Result<()> {
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    // Create storage
    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    // Create indices
    let mut primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Create test documents
    for i in 0..5 {
        let doc = DocumentBuilder::new()
            .path(format!("test{}.md", i))?
            .title(format!("Test Document {}", i))?
            .content(format!("This is test content number {}. It contains various words like function, struct, impl, and let.", i).as_bytes())
            .build()?;

        storage.insert(doc.clone()).await?;
    }

    // Test trigram index before rebuild - should be empty
    let text_query = QueryBuilder::new()
        .with_text("test")?
        .with_limit(10)?
        .build()?;
    let trigram_results_before = trigram_index.search(&text_query).await?;
    assert_eq!(
        trigram_results_before.len(),
        0,
        "Trigram index should be empty before rebuild"
    );

    // Rebuild indices with content
    let all_docs = storage.list_all().await?;
    assert_eq!(all_docs.len(), 5, "Should have 5 documents in storage");

    // Rebuild primary index
    for doc in &all_docs {
        primary_index
            .insert(doc.id, ValidatedPath::new(doc.path.to_string())?)
            .await?;
    }

    // Rebuild trigram index WITH CONTENT (this is the critical part)
    for doc in &all_docs {
        trigram_index
            .insert_with_content(
                doc.id,
                ValidatedPath::new(doc.path.to_string())?,
                &doc.content,
            )
            .await?;
    }

    // Flush indices
    primary_index.flush().await?;
    trigram_index.flush().await?;

    // Test primary index after rebuild
    let wildcard_query = QueryBuilder::new().with_limit(10)?.build()?;
    let primary_results = primary_index.search(&wildcard_query).await?;
    assert_eq!(
        primary_results.len(),
        5,
        "Primary index should have 5 documents"
    );

    // Test trigram index after rebuild - should now have results
    let trigram_results_after = trigram_index.search(&text_query).await?;
    assert!(
        !trigram_results_after.is_empty(),
        "Trigram index should have results after rebuild with content"
    );
    assert_eq!(
        trigram_results_after.len(),
        5,
        "All 5 documents should match 'test'"
    );

    // Test other search terms
    for term in &["function", "struct", "impl", "content"] {
        let query = QueryBuilder::new()
            .with_text(*term)?
            .with_limit(10)?
            .build()?;
        let results = trigram_index.search(&query).await?;
        assert!(
            !results.is_empty(),
            "Should find results for term '{}'",
            term
        );
    }

    Ok(())
}
