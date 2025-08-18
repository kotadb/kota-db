// Test suite for issue #198: Trigram search result differentiation
// This test ensures that the trigram index properly differentiates between documents
// and returns the most relevant document for a given query

use anyhow::Result;
use kotadb::{
    create_file_storage, create_trigram_index_for_tests, DocumentBuilder, Index, QueryBuilder,
    Storage,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_search_result_differentiation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&trigram_path)?;

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(10)).await?;
    let mut trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?;

    // Create documents with overlapping but different content
    // Doc1: Heavy focus on "rust programming"
    let doc1 = DocumentBuilder::new()
        .path("rust_guide.md")?
        .title("Rust Programming Guide")?
        .content(b"Rust programming is great. Rust is a systems programming language. Learn Rust programming today. Rust programming offers memory safety.")
        .build()?;

    // Doc2: Light mention of "rust programming"
    let doc2 = DocumentBuilder::new()
        .path("general_coding.md")?
        .title("General Coding Tips")?
        .content(b"There are many programming languages including Python, Java, and Rust. Each language has its benefits.")
        .build()?;

    // Doc3: No mention of "rust" but has "programming"
    let doc3 = DocumentBuilder::new()
        .path("python_tutorial.md")?
        .title("Python Tutorial")?
        .content(b"Python programming is beginner-friendly. Programming in Python is fun and productive.")
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

    // Test 1: Search for "rust programming" should return doc1 first
    let query = QueryBuilder::new().with_text("rust programming")?.build()?;
    let results = trigram_index.search(&query).await?;

    assert!(
        results.len() >= 2,
        "Search for 'rust programming' should return at least 2 documents, got {}",
        results.len()
    );

    // The document with more occurrences of "rust programming" should be first
    assert_eq!(
        results[0], doc1.id,
        "Document with most 'rust programming' mentions should be ranked first"
    );

    // Test 2: Search for just "rust" should still prioritize doc1
    let query = QueryBuilder::new().with_text("rust")?.build()?;
    let results = trigram_index.search(&query).await?;

    assert!(
        results.len() >= 2,
        "Search for 'rust' should return at least 2 documents, got {}",
        results.len()
    );

    assert_eq!(
        results[0], doc1.id,
        "Document with most 'rust' mentions should be ranked first"
    );

    // Test 3: Search for "python" should return docs with Python mentions
    let query = QueryBuilder::new().with_text("python")?.build()?;
    let results = trigram_index.search(&query).await?;

    assert!(
        !results.is_empty(),
        "Search for 'python' should return at least 1 document, got {}",
        results.len()
    );

    // Doc3 should be ranked first since it has more Python mentions
    assert_eq!(
        results[0], doc3.id,
        "The Python tutorial document should be ranked first for 'python' query"
    );

    Ok(())
}

#[tokio::test]
async fn test_document_length_consideration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&trigram_path)?;

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(10)).await?;
    let mut trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?;

    // Create two documents with same term frequency but different lengths
    // Doc1: Short and focused
    let doc1 = DocumentBuilder::new()
        .path("short.md")?
        .title("Short Doc")?
        .content(b"database indexing")
        .build()?;

    // Doc2: Long with same terms buried in text
    let doc2 = DocumentBuilder::new()
        .path("long.md")?
        .title("Long Doc")?
        .content(b"This is a very long document with lots of text. Somewhere in here we mention database stuff. And somewhere else we talk about indexing. But there's so much other content that these terms are diluted.")
        .build()?;

    // Insert documents
    storage.insert(doc1.clone()).await?;
    storage.insert(doc2.clone()).await?;

    // Update trigram index
    trigram_index
        .insert_with_content(doc1.id, doc1.path.clone(), &doc1.content)
        .await?;
    trigram_index
        .insert_with_content(doc2.id, doc2.path.clone(), &doc2.content)
        .await?;

    // Search for "database indexing" should prefer the shorter, more focused document
    let query = QueryBuilder::new()
        .with_text("database indexing")?
        .build()?;
    let results = trigram_index.search(&query).await?;

    assert!(
        !results.is_empty(),
        "Search should return at least 1 document"
    );

    // The shorter, more focused document should rank higher
    assert_eq!(
        results[0], doc1.id,
        "Shorter, more focused document should be ranked first"
    );

    Ok(())
}

#[tokio::test]
async fn test_exact_match_prioritization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&trigram_path)?;

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(10)).await?;
    let mut trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?;

    // Create documents with varying degrees of match
    // Doc1: Exact phrase match
    let doc1 = DocumentBuilder::new()
        .path("exact.md")?
        .title("Exact Match")?
        .content(b"The file storage module provides efficient data persistence.")
        .build()?;

    // Doc2: All words but separated
    let doc2 = DocumentBuilder::new()
        .path("separated.md")?
        .title("Separated Words")?
        .content(b"The file is here. Storage is over there. This module is different.")
        .build()?;

    // Doc3: Only some words
    let doc3 = DocumentBuilder::new()
        .path("partial.md")?
        .title("Partial Match")?
        .content(b"This file contains some information about modules.")
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

    // Search for "file storage module" should prioritize exact match
    let query = QueryBuilder::new()
        .with_text("file storage module")?
        .build()?;
    let results = trigram_index.search(&query).await?;

    assert!(
        !results.is_empty(),
        "Search should return at least 1 document"
    );

    // Document with exact phrase should rank first
    assert_eq!(
        results[0], doc1.id,
        "Document with exact phrase match should be ranked first"
    );

    Ok(())
}
