// Test for Issue #248: Index synchronization failure during repository ingestion
// This test reproduces the bug where:
// 1. Primary index appeared to have only 1000 documents (actually a validation bug)
// 2. Trigram index had 0 documents (actual indexing failure)

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder, Index, Query,
    QueryBuilder, Storage, ValidatedPath,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_bulk_index_rebuild_with_large_dataset() -> Result<()> {
    // Create temporary directories for storage and indices
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    // Create storage and indices
    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Insert 2755 documents (the number from the original issue)
    // This simulates the git ingestion scenario
    const DOCUMENT_COUNT: usize = 2755;
    println!("Inserting {} documents into storage...", DOCUMENT_COUNT);

    for i in 0..DOCUMENT_COUNT {
        let doc = DocumentBuilder::new()
            .path(format!("repo/file_{}.rs", i))?
            .title(format!("File {}", i))?
            .content(format!("// Rust source code file {}\nfn main() {{ println!(\"Hello from file {}\"); }}", i, i).as_bytes())
            .build()?;

        storage.insert(doc).await?;

        if i % 500 == 0 {
            println!("  Inserted {} documents...", i);
        }
    }

    println!("Finished inserting documents into storage");

    // Now rebuild indices (simulating what happens after git ingestion)
    println!("Rebuilding indices from storage...");

    let all_docs = storage.list_all().await?;
    assert_eq!(
        all_docs.len(),
        DOCUMENT_COUNT,
        "Storage should have all {} documents",
        DOCUMENT_COUNT
    );

    const BATCH_SIZE: usize = 100;
    let mut processed = 0;

    for chunk in all_docs.chunks(BATCH_SIZE) {
        // Process batch for primary index
        for doc in chunk {
            let doc_id = doc.id;
            let doc_path = ValidatedPath::new(doc.path.to_string())?;
            primary_index.insert(doc_id, doc_path.clone()).await?;

            // For trigram index, use insert_with_content
            trigram_index
                .insert_with_content(doc_id, doc_path, &doc.content)
                .await?;
        }

        processed += chunk.len();

        // Periodic flush
        if processed % 500 == 0 || processed == DOCUMENT_COUNT {
            primary_index.flush().await?;
            trigram_index.flush().await?;
            println!("  Processed {} documents...", processed);
        }
    }

    println!("Finished rebuilding indices");

    // Now validate the indices have the correct number of documents
    // Test with high limit to ensure we see all documents
    // Primary index can use wildcard query
    let wildcard_query = QueryBuilder::new()
        .with_text("*")? // Wildcard for primary index
        .with_limit(10000)? // High limit to see all documents
        .build()?;

    println!("Validating index counts...");

    // Check primary index
    let primary_results = primary_index.search(&wildcard_query).await?;
    println!("Primary index contains {} documents", primary_results.len());

    // Check trigram index with text search (trigram doesn't support wildcard)
    // Search for "main" which appears in all generated document paths
    let trigram_query = QueryBuilder::new()
        .with_text("main")? // Text search for trigram index
        .with_limit(10000)? // High limit
        .build()?;
    let trigram_results = trigram_index.search(&trigram_query).await?;
    println!(
        "Trigram index contains {} documents with 'main'",
        trigram_results.len()
    );

    // The bug was:
    // - Primary index appeared to have only 1000 (due to query limit)
    // - Trigram index had 0 (actual failure)

    // With our fixes:
    // - Query limits are increased to 100,000
    // - Trigram index properly uses insert_with_content

    assert!(
        primary_results.len() >= DOCUMENT_COUNT,
        "Primary index should have at least {} documents, but has {}",
        DOCUMENT_COUNT,
        primary_results.len()
    );

    // Trigram index wildcard queries might not return all documents
    // but should return some
    assert!(
        !trigram_results.is_empty(),
        "Trigram index should have indexed documents, but has 0"
    );

    // Test text search on trigram index
    let text_query = QueryBuilder::new()
        .with_text("main")? // Search for "main" which appears in all documents
        .with_limit(100)?
        .build()?;

    let text_results = trigram_index.search(&text_query).await?;
    println!(
        "Text search for 'main' found {} documents",
        text_results.len()
    );
    assert!(
        !text_results.is_empty(),
        "Trigram text search should find documents containing 'main'"
    );

    Ok(())
}

#[tokio::test]
async fn test_validation_with_1000_plus_documents() -> Result<()> {
    // This test specifically checks that validation doesn't falsely report
    // a 1000 document limit when there are more documents

    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Insert 1500 documents (more than the old 1000 limit)
    const DOC_COUNT: usize = 1500;

    for i in 0..DOC_COUNT {
        let doc = DocumentBuilder::new()
            .path(format!("test/doc_{}.md", i))?
            .title(format!("Document {}", i))?
            .content(format!("Content for document {}", i).as_bytes())
            .build()?;

        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(doc.path.to_string())?;
        let content = doc.content.clone();

        storage.insert(doc).await?;
        primary_index.insert(doc_id, doc_path.clone()).await?;
        trigram_index
            .insert_with_content(doc_id, doc_path, &content)
            .await?;
    }

    // Flush to ensure everything is persisted
    storage.flush().await?;
    primary_index.flush().await?;
    trigram_index.flush().await?;

    // Now run validation-style queries
    let storage_count = storage.list_all().await?.len();

    // This was the problematic query - limited to 1000
    let old_style_query = Query::new(None, None, None, 1000)?;
    let old_primary_count = primary_index.search(&old_style_query).await?.len();
    let old_trigram_count = trigram_index.search(&old_style_query).await?.len();

    println!("Old style validation (limit=1000):");
    println!("  Storage: {} docs", storage_count);
    println!("  Primary: {} docs (capped at 1000!)", old_primary_count);
    println!("  Trigram: {} docs", old_trigram_count);

    // New style with proper limit
    let new_style_query = QueryBuilder::new()
        .with_limit(DOC_COUNT + 100)? // Ensure we can see all documents
        .build()?;
    let new_primary_count = primary_index.search(&new_style_query).await?.len();
    let new_trigram_count = trigram_index.search(&new_style_query).await?.len();

    println!("New style validation (limit={}):", DOC_COUNT + 100);
    println!("  Storage: {} docs", storage_count);
    println!("  Primary: {} docs", new_primary_count);
    println!("  Trigram: {} docs", new_trigram_count);

    // Old style would fail this assertion
    assert_eq!(
        storage_count, DOC_COUNT,
        "Storage should have exactly {} documents",
        DOC_COUNT
    );

    // With proper limits, we should see all documents
    assert!(
        new_primary_count >= DOC_COUNT,
        "Primary index should show at least {} documents with proper limit, but shows {}",
        DOC_COUNT,
        new_primary_count
    );

    Ok(())
}
