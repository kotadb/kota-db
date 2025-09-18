// Test to reproduce issue #222: Search results inconsistent
// This test attempts to reproduce the non-deterministic behavior where
// searches sometimes return all documents regardless of matching

use anyhow::Result;
use kotadb::contracts::{Index, Storage};
use kotadb::{create_file_storage, create_trigram_index, DocumentBuilder, QueryBuilder};
use tempfile::TempDir;

#[cfg_attr(
    feature = "aggressive-trigram-thresholds",
    ignore = "Aggressive trigram fallback relaxes strict zero-result expectations"
)]
#[tokio::test]
async fn test_search_returns_all_documents_incorrectly() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    // Create 3 documents, only 1 contains "storage"
    let doc1 = DocumentBuilder::new()
        .path("doc1.md")?
        .title("Document 1")?
        .content(b"This document contains the word storage in its content")
        .build()?;

    let doc2 = DocumentBuilder::new()
        .path("doc2.md")?
        .title("Document 2")?
        .content(b"This document is about something else entirely")
        .build()?;

    let doc3 = DocumentBuilder::new()
        .path("doc3.md")?
        .title("Document 3")?
        .content(b"Another document with different content")
        .build()?;

    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;
    storage.insert(doc3.clone()).await?;

    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Index documents with content
    trigram_index
        .insert_with_content(
            doc1.id,
            kotadb::types::ValidatedPath::new(doc1.path.to_string())?,
            &doc1.content,
        )
        .await?;

    trigram_index
        .insert_with_content(
            doc2.id,
            kotadb::types::ValidatedPath::new(doc2.path.to_string())?,
            &doc2.content,
        )
        .await?;

    trigram_index
        .insert_with_content(
            doc3.id,
            kotadb::types::ValidatedPath::new(doc3.path.to_string())?,
            &doc3.content,
        )
        .await?;

    // Flush to ensure persistence
    trigram_index.flush().await?;

    // Test 1: Search for "storage" - should return only 1 document
    let storage_query = QueryBuilder::new().with_text("storage")?.build()?;
    let storage_results = trigram_index.search(&storage_query).await?;

    println!(
        "Search for 'storage' returned {} documents",
        storage_results.len()
    );
    assert_eq!(
        storage_results.len(),
        1,
        "Search for 'storage' should return exactly 1 document, but returned {}",
        storage_results.len()
    );

    // Test 2: Search for non-existent text - should return 0 documents
    let nonexistent_query = QueryBuilder::new()
        .with_text("nonexistent-text-that-should-not-match")?
        .build()?;

    // Debug: Check what's in the query
    println!(
        "Query search_terms count: {}",
        nonexistent_query.search_terms.len()
    );
    for (i, term) in nonexistent_query.search_terms.iter().enumerate() {
        println!("  Term {}: '{}'", i, term.as_str());
    }

    let nonexistent_results = trigram_index.search(&nonexistent_query).await?;

    println!(
        "Search for 'nonexistent-text-that-should-not-match' returned {} documents",
        nonexistent_results.len()
    );
    assert_eq!(
        nonexistent_results.len(),
        0,
        "Search for non-existent text should return 0 documents, but returned {}",
        nonexistent_results.len()
    );

    // Test 3: Multiple searches for the same term should return consistent results
    let mut results_consistency = Vec::new();
    for i in 0..5 {
        let query = QueryBuilder::new().with_text("storage")?.build()?;
        let results = trigram_index.search(&query).await?;
        results_consistency.push(results.len());
        println!(
            "Search iteration {} returned {} documents",
            i, results_consistency[i]
        );
    }

    // All searches should return the same number of results
    let first_result = results_consistency[0];
    for (i, &count) in results_consistency.iter().enumerate() {
        assert_eq!(
            count, first_result,
            "Search iteration {} returned {} documents, but first search returned {}",
            i, count, first_result
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_empty_search_terms_behavior() -> Result<()> {
    // Test what happens when search_terms might be empty
    let temp_dir = TempDir::new()?;
    let trigram_path = temp_dir.path().join("trigram");

    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Add some documents
    for i in 0..3 {
        let path = format!("doc{}.md", i);
        let content = format!("Document {} content", i);
        let doc = DocumentBuilder::new()
            .path(&path)?
            .title(format!("Document {}", i))?
            .content(content.as_bytes())
            .build()?;
        trigram_index
            .insert_with_content(
                doc.id,
                kotadb::types::ValidatedPath::new(&path)?,
                content.as_bytes(),
            )
            .await?;
    }

    // Create a query that might result in empty search_terms
    // This simulates what might happen if ValidatedSearchQuery::new fails
    let query = kotadb::contracts::Query::empty();
    let results = trigram_index.search(&query).await?;

    println!("Empty query returned {} documents", results.len());

    // An empty query should return 0 documents, not all documents
    assert_eq!(
        results.len(),
        0,
        "Empty query should return 0 documents, but returned {}",
        results.len()
    );

    Ok(())
}
