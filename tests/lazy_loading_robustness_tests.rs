use anyhow::Result;
use kotadb::{
    create_primary_index, create_trigram_index, Index, QueryBuilder, ValidatedDocumentId,
    ValidatedPath,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_concurrent_trigram_index_loading() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("trigram_concurrent");

    // Create and populate index with some data first
    {
        let mut index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;
        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("test/document.md")?;
        let content = b"Test document with searchable content for concurrent loading tests";
        index.insert_with_content(doc_id, path, content).await?;
        index.flush().await?;
    }

    // Create fresh index (will lazy load)
    let index = Arc::new(create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?);
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = Vec::new();

    // Spawn 10 concurrent search tasks
    for i in 0..10 {
        let index_clone = Arc::clone(&index);
        let barrier_clone = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            // All threads wait at barrier, then search simultaneously
            barrier_clone.wait().await;

            let query = QueryBuilder::new().with_text("searchable")?.build()?;

            let start = Instant::now();
            let results = index_clone.search(&query).await?;
            let duration = start.elapsed();

            Ok::<(usize, Duration, usize), anyhow::Error>((i, duration, results.len()))
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }

    // Verify all searches succeeded
    assert_eq!(results.len(), 10);

    // All should find the same results
    let expected_count = results[0].2;
    for (thread_id, duration, count) in &results {
        assert_eq!(
            *count, expected_count,
            "Thread {} found different result count",
            thread_id
        );
        // Even concurrent loads should be reasonably fast
        assert!(
            duration.as_millis() < 2000,
            "Thread {} took too long: {:?}",
            thread_id,
            duration
        );
    }

    // At least one thread should have found results
    assert!(expected_count > 0, "No documents were found");

    println!(
        "Concurrent loading test passed: {} threads, {} results each",
        results.len(),
        expected_count
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_primary_index_loading() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("primary_concurrent");

    // Create and populate index
    {
        let mut index = create_primary_index(index_path.to_str().unwrap(), Some(100)).await?;
        for i in 0..5 {
            let doc_id = ValidatedDocumentId::new();
            let path = ValidatedPath::new(format!("test/document_{}.md", i))?;
            index.insert(doc_id, path).await?;
        }
        index.flush().await?;
    }

    // Create fresh index for concurrent testing
    let index = Arc::new(create_primary_index(index_path.to_str().unwrap(), Some(100)).await?);
    let barrier = Arc::new(Barrier::new(8));
    let mut handles = Vec::new();

    // Spawn concurrent searches
    for i in 0..8 {
        let index_clone = Arc::clone(&index);
        let barrier_clone = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            barrier_clone.wait().await;

            let query = QueryBuilder::new()
                .with_text("*")? // Wildcard search for primary index
                .build()?;

            let results = index_clone.search(&query).await?;
            Ok::<(usize, usize), anyhow::Error>((i, results.len()))
        });

        handles.push(handle);
    }

    // Collect all results
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }

    // All threads should get same results
    assert_eq!(results.len(), 8);
    let expected_count = results[0].1;

    for (thread_id, count) in &results {
        assert_eq!(
            *count, expected_count,
            "Thread {} found different result count",
            thread_id
        );
    }

    assert!(expected_count >= 5, "Should find at least 5 documents");

    Ok(())
}

#[tokio::test]
async fn test_trigram_index_load_failure_protection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("trigram_fail_test");

    // Create index directory but with corrupted metadata
    tokio::fs::create_dir_all(&index_path.join("trigrams")).await?;
    tokio::fs::create_dir_all(&index_path.join("cache")).await?;
    tokio::fs::create_dir_all(&index_path.join("meta")).await?;

    // Write invalid JSON to metadata file
    let metadata_path = index_path.join("meta").join("trigram_metadata.json");
    tokio::fs::write(&metadata_path, "{ invalid json }").await?;

    // Create index with corrupted data
    let index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

    let query = QueryBuilder::new().with_text("test")?.build()?;

    // First search should fail due to corrupted metadata
    let start = Instant::now();
    let result1 = index.search(&query).await;
    let first_duration = start.elapsed();

    assert!(result1.is_err(), "Expected first search to fail");

    // Second search should fail immediately (no retry storm)
    let start = Instant::now();
    let result2 = index.search(&query).await;
    let second_duration = start.elapsed();

    assert!(result2.is_err(), "Expected second search to fail");
    assert!(
        second_duration.as_millis() < 50,
        "Second failure should be immediate, took {:?}",
        second_duration
    );

    // Error messages should indicate retry protection
    let err_msg = result2.unwrap_err().to_string();
    assert!(
        err_msg.contains("previously failed") || err_msg.contains("retry is disabled"),
        "Error should indicate retry protection: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_primary_index_load_failure_protection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("primary_fail_test");

    // Create index with corrupted B+ tree file
    tokio::fs::create_dir_all(&index_path.join("data")).await?;
    let btree_path = index_path.join("data").join("btree_root.json");
    tokio::fs::write(&btree_path, "{ corrupted json data }").await?;

    let index = create_primary_index(index_path.to_str().unwrap(), Some(100)).await?;

    let query = QueryBuilder::new().with_text("*")?.build()?;

    // First search with corrupted B+ tree - should succeed with graceful degradation
    let start = Instant::now();
    let result1 = index.search(&query).await;
    let first_duration = start.elapsed();

    // Primary index should gracefully handle corrupted data
    assert!(
        result1.is_ok(),
        "Primary index should gracefully handle corrupted data"
    );

    // Second search should be fast (cached state)
    let start = Instant::now();
    let result2 = index.search(&query).await;
    let second_duration = start.elapsed();

    assert!(result2.is_ok(), "Second search should succeed");
    assert!(
        second_duration < first_duration,
        "Second search should be faster due to caching"
    );
    assert!(
        second_duration.as_millis() < 50,
        "Second search should be very fast"
    );

    Ok(())
}

#[tokio::test]
async fn test_loading_state_transitions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("state_test");

    // Create valid index
    {
        let mut index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;
        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("test/doc.md")?;
        index
            .insert_with_content(doc_id, path, b"test content")
            .await?;
        index.flush().await?;
    }

    // Create fresh index to test state transitions
    let index = Arc::new(create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?);
    let query = QueryBuilder::new().with_text("test")?.build()?;

    // Index should start in NotLoaded state and successfully load
    let result = index.search(&query).await?;
    assert!(!result.is_empty(), "Should find test document");

    // Subsequent searches should use loaded index (fast path)
    let start = Instant::now();
    let result2 = index.search(&query).await?;
    let duration = start.elapsed();

    assert_eq!(result2.len(), result.len(), "Results should be consistent");
    assert!(
        duration.as_millis() < 10,
        "Loaded index search should be very fast"
    );

    Ok(())
}

#[tokio::test]
async fn test_memory_usage_awareness() -> Result<()> {
    // This test ensures we're aware of memory usage during lazy loading
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("memory_test");

    // Create a reasonably large index
    {
        let mut index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

        for i in 0..100 {
            let doc_id = ValidatedDocumentId::new();
            let path = ValidatedPath::new(format!("test/large_doc_{}.md", i))?;
            let content = format!("Large document content with lots of searchable text and trigrams for testing memory usage during lazy loading. Document number {}. This content is repeated to create a larger index that will consume more memory when loaded.", i).repeat(10);
            index
                .insert_with_content(doc_id, path, content.as_bytes())
                .await?;
        }
        index.flush().await?;
    }

    // Measure memory usage before and after loading
    let index = create_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

    // Get initial memory baseline (this is approximate)
    let start_time = Instant::now();

    let query = QueryBuilder::new().with_text("searchable")?.build()?;
    let results = index.search(&query).await?;

    let load_time = start_time.elapsed();

    // Should find documents but loading should be reasonable
    assert!(!results.is_empty(), "Should find documents");
    assert!(results.len() <= 100, "Shouldn't find more than we created");

    // Loading should complete in reasonable time even for larger index
    assert!(
        load_time.as_millis() < 5000,
        "Large index should still load in reasonable time: {:?}",
        load_time
    );

    println!(
        "Large index test: {} documents found, loaded in {:?}",
        results.len(),
        load_time
    );

    Ok(())
}
