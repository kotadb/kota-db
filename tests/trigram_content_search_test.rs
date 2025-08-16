// Test for trigram content indexing fix (Issue #196)
// Validates that trigram index now properly indexes document content, not just paths

use anyhow::Result;
use kotadb::contracts::{Index, Storage};
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder, QueryBuilder,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

/// Simulate the Database structure for testing
struct TestDatabase {
    storage: Arc<Mutex<Box<dyn kotadb::contracts::Storage>>>,
    primary_index: Arc<Mutex<Box<dyn kotadb::contracts::Index>>>,
    trigram_index: Arc<Mutex<Box<dyn kotadb::contracts::Index>>>,
    #[allow(dead_code)]
    path_cache: Arc<RwLock<HashMap<String, kotadb::types::ValidatedDocumentId>>>,
}

impl TestDatabase {
    /// Rebuild all indices from current storage
    async fn rebuild_indices(&self) -> Result<()> {
        // Get all documents from storage
        let all_docs = self.storage.lock().await.list_all().await?;
        let total_docs = all_docs.len();

        if total_docs == 0 {
            return Ok(());
        }

        // Process documents in batches for better performance
        const BATCH_SIZE: usize = 100;
        let mut processed = 0;

        // Process in chunks to reduce lock contention and prevent OOM
        for chunk in all_docs.chunks(BATCH_SIZE) {
            // Collect document data for this batch (including content for trigram indexing)
            let mut batch_entries = Vec::with_capacity(chunk.len());
            for doc in chunk {
                let doc_id = doc.id;
                let doc_path = kotadb::types::ValidatedPath::new(doc.path.to_string())?;
                batch_entries.push((doc_id, doc_path, doc.content.clone()));
            }

            // Insert batch into primary index (path-based)
            {
                let mut primary_index = self.primary_index.lock().await;
                for (doc_id, doc_path, _) in &batch_entries {
                    primary_index.insert(*doc_id, doc_path.clone()).await?;
                }
            }

            // Insert batch into trigram index with content for proper full-text search
            {
                let mut trigram_index = self.trigram_index.lock().await;
                for (doc_id, doc_path, content) in &batch_entries {
                    // Use the new content-aware method for proper trigram indexing
                    trigram_index
                        .insert_with_content(*doc_id, doc_path.clone(), content)
                        .await?;
                }
            }

            processed += chunk.len();

            // Force flush after every batch to ensure all data is persisted
            self.primary_index.lock().await.flush().await?;
            self.trigram_index.lock().await.flush().await?;
        }

        Ok(())
    }
}

#[tokio::test]
async fn test_trigram_content_indexing_basic() -> Result<()> {
    // Test that trigram index now properly indexes document content, not just paths
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    // Add documents with specific content that should be searchable
    let rust_doc = DocumentBuilder::new()
        .path("code/rust_example.rs")?
        .title("Rust Code Example")?
        .content(b"async fn process_data() { println!(\"Processing data\"); }")
        .build()?;

    let python_doc = DocumentBuilder::new()
        .path("code/python_example.py")?
        .title("Python Code Example")?
        .content(b"def process_data(): print(\"Processing data\")")
        .build()?;

    let readme_doc = DocumentBuilder::new()
        .path("README.md")?
        .title("Project README")?
        .content(b"This project demonstrates async programming patterns in Rust.")
        .build()?;

    storage.insert(rust_doc).await?;
    storage.insert(python_doc).await?;
    storage.insert(readme_doc).await?;

    let primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    let db = TestDatabase {
        storage: Arc::new(Mutex::new(Box::new(storage))),
        primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
        trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    // Rebuild indices with content-aware indexing
    db.rebuild_indices().await?;

    // Test 1: Search for content that exists in document bodies (not paths)
    let content_query = QueryBuilder::new().with_text("async")?.build()?;
    let content_results = db.trigram_index.lock().await.search(&content_query).await?;
    assert!(
        !content_results.is_empty(),
        "Should find 'async' in Rust document content"
    );

    // Test 2: Search for function names in code
    let function_query = QueryBuilder::new().with_text("process_data")?.build()?;
    let function_results = db
        .trigram_index
        .lock()
        .await
        .search(&function_query)
        .await?;
    assert!(
        function_results.len() >= 2,
        "Should find 'process_data' in both Rust and Python documents, got {} results",
        function_results.len()
    );

    // Test 3: Search for programming language keywords
    let print_query = QueryBuilder::new().with_text("println")?.build()?;
    let print_results = db.trigram_index.lock().await.search(&print_query).await?;
    assert!(
        !print_results.is_empty(),
        "Should find 'println' in Rust document content"
    );

    // Test 4: Search for documentation content
    let patterns_query = QueryBuilder::new().with_text("programming")?.build()?;
    let patterns_results = db
        .trigram_index
        .lock()
        .await
        .search(&patterns_query)
        .await?;
    assert!(
        !patterns_results.is_empty(),
        "Should find 'programming' in README content"
    );

    // Test 5: Verify that path-based searches still work (backward compatibility)
    let path_query = QueryBuilder::new().with_text("README")?.build()?;
    let path_results = db.trigram_index.lock().await.search(&path_query).await?;
    assert!(
        !path_results.is_empty(),
        "Should still find documents by path names"
    );

    Ok(())
}

#[tokio::test]
async fn test_trigram_content_vs_path_search() -> Result<()> {
    // Test that demonstrates the difference between content and path-based search
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    // Create a document where the filename and content are completely different
    let misleading_doc = DocumentBuilder::new()
        .path("database/users.sql")?  // Path suggests SQL
        .title("User Management SQL")?
        .content(b"# This is actually a Rust configuration file\nuse std::collections::HashMap;\nlet config = HashMap::new();")  // Content is Rust
        .build()?;

    let doc_id = misleading_doc.id;
    let doc_content = misleading_doc.content.clone();
    storage.insert(misleading_doc).await?;

    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    // Use content-aware indexing
    let doc_path = kotadb::types::ValidatedPath::new("database/users.sql")?;
    trigram_index
        .insert_with_content(doc_id, doc_path, &doc_content)
        .await?;

    // Test 1: Search for content that exists in the file but not in the path
    let rust_query = QueryBuilder::new().with_text("HashMap")?.build()?;
    let rust_results = trigram_index.search(&rust_query).await?;
    assert!(
        !rust_results.is_empty(),
        "Should find 'HashMap' in document content even though path suggests SQL"
    );

    // Test 2: Search for content keywords
    let use_query = QueryBuilder::new().with_text("collections")?.build()?;
    let use_results = trigram_index.search(&use_query).await?;
    assert!(
        !use_results.is_empty(),
        "Should find 'collections' in Rust code content"
    );

    // Test 3: Path-based search should still work
    let sql_query = QueryBuilder::new().with_text("users")?.build()?;
    let sql_results = trigram_index.search(&sql_query).await?;
    assert!(
        !sql_results.is_empty(),
        "Should still find document by its filename"
    );

    Ok(())
}

#[tokio::test]
async fn test_large_content_memory_safety() -> Result<()> {
    // Test that the content indexing doesn't cause OOM with large documents
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(500)).await?;

    // Create documents with substantial content
    for i in 0..50 {
        let large_content = format!(
            "This is document number {} with substantial content. {}\n{}\n{}\n{}",
            i,
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(20),
            "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(20),
            "Ut enim ad minim veniam, quis nostrud exercitation ullamco. ".repeat(20),
            "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum. ".repeat(20)
        );

        let doc = DocumentBuilder::new()
            .path(format!("large/doc_{}.txt", i))?
            .title(format!("Large Document {}", i))?
            .content(large_content.as_bytes())
            .build()?;

        storage.insert(doc).await?;
    }

    let primary_index =
        create_primary_index(temp_dir.path().join("primary").to_str().unwrap(), Some(500)).await?;
    let trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(500)).await?;

    let db = TestDatabase {
        storage: Arc::new(Mutex::new(Box::new(storage))),
        primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
        trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    // This should complete without OOM errors
    let start = std::time::Instant::now();
    db.rebuild_indices().await?;
    let duration = start.elapsed();

    // Test that content search works with large documents
    // Use a higher limit to ensure we can find all 50 documents
    let lorem_query = QueryBuilder::new()
        .with_text("Lorem")?
        .with_limit(100)?
        .build()?;
    let lorem_results = db.trigram_index.lock().await.search(&lorem_query).await?;

    assert!(
        lorem_results.len() >= 50,
        "Should find 'Lorem ipsum' in all large documents, got {} results",
        lorem_results.len()
    );

    // Performance check: should complete in reasonable time even with content processing
    assert!(
        duration.as_secs() < 10,
        "Large content rebuild took too long: {:?}",
        duration
    );

    Ok(())
}
