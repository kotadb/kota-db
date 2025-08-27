//! Integration tests for Database::search method
//! This tests the full search pipeline including routing logic between indices

use anyhow::Result;
use kotadb::{create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

// Import the Database struct from main.rs (note: this requires making it public or using a test helper)
// For now, we'll create a minimal test harness that mimics the Database behavior

struct TestDatabase {
    storage: Arc<Mutex<Box<dyn kotadb::Storage>>>,
    primary_index: Arc<Mutex<Box<dyn kotadb::Index>>>,
    trigram_index: Arc<Mutex<Box<dyn kotadb::Index>>>,
}

impl TestDatabase {
    async fn new(temp_dir: &TempDir) -> Result<Self> {
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&primary_path)?;
        std::fs::create_dir_all(&trigram_path)?;

        let storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
        let primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
        let trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

        Ok(Self {
            storage: Arc::new(Mutex::new(Box::new(storage) as Box<dyn kotadb::Storage>)),
            primary_index: Arc::new(Mutex::new(Box::new(primary_index) as Box<dyn kotadb::Index>)),
            trigram_index: Arc::new(Mutex::new(Box::new(trigram_index) as Box<dyn kotadb::Index>)),
        })
    }

    async fn insert(&self, path: &str, title: &str, content: &str) -> Result<()> {
        let doc = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(content.as_bytes())
            .build()?;

        let doc_id = doc.id;
        let doc_path = doc.path.clone();

        // Insert into storage
        self.storage.lock().await.insert(doc.clone()).await?;

        // Update both indices
        self.primary_index
            .lock()
            .await
            .insert(doc_id, doc_path.clone())
            .await?;

        // For trigram index, we need to provide content
        self.trigram_index
            .lock()
            .await
            .insert_with_content(doc_id, doc_path, content.as_bytes())
            .await?;

        Ok(())
    }

    async fn search(&self, query_text: &str) -> Result<Vec<kotadb::Document>> {
        // This mimics the routing logic from main.rs
        let query = kotadb::QueryBuilder::new()
            .with_text(query_text)?
            .with_limit(100)?
            .build()?;

        // Route based on wildcard presence (matching the fix in main.rs)
        let doc_ids = if query_text.contains('*') || query_text.is_empty() {
            // Use Primary Index for wildcard/pattern queries
            self.primary_index.lock().await.search(&query).await?
        } else {
            // Use Trigram Index for full-text search queries
            self.trigram_index.lock().await.search(&query).await?
        };

        // Retrieve documents from storage
        let mut documents = Vec::new();
        let storage = self.storage.lock().await;
        for doc_id in doc_ids {
            if let Some(doc) = storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        Ok(documents)
    }
}

#[tokio::test]
async fn test_database_search_wildcard_routing() -> Result<()> {
    // This test ensures wildcard queries are properly routed to the primary index
    // and regular text queries go to the trigram index

    let temp_dir = TempDir::new()?;
    let db = TestDatabase::new(&temp_dir).await?;

    // Insert test documents
    let test_docs = vec![
        (
            "src/main.rs",
            "Main application",
            "fn main() { println!(\"Hello\"); }",
        ),
        ("src/lib.rs", "Library module", "pub mod utils { }"),
        (
            "tests/test.rs",
            "Test file",
            "mod tests { #[test] fn test() {} }",
        ),
        ("README.md", "Documentation", "# Project README"),
        (
            "Cargo.toml",
            "Package manifest",
            "[package] name = \"test\"",
        ),
    ];

    for (path, title, content) in test_docs {
        db.insert(path, title, content).await?;
    }

    // Test 1: Wildcard pattern should use primary index and return filtered results
    let results = db.search("*.rs").await?;
    assert_eq!(
        results.len(),
        3,
        "Wildcard *.rs should find exactly 3 Rust files"
    );

    // Test 2: Pure wildcard should use primary index and return all documents
    let results = db.search("*").await?;
    assert_eq!(
        results.len(),
        5,
        "Pure wildcard should return all 5 documents"
    );

    // Test 3: Text search should use trigram index
    let results = db.search("main").await?;
    // Trigram search should find documents containing "main"
    assert!(
        !results.is_empty(),
        "Text search for 'main' should find at least one document"
    );

    // Test 4: Complex wildcard patterns
    let results = db.search("src/*").await?;
    assert_eq!(
        results.len(),
        2,
        "Pattern src/* should find 2 files in src directory"
    );

    let results = db.search("*.md").await?;
    assert_eq!(results.len(), 1, "Pattern *.md should find 1 markdown file");

    Ok(())
}

#[tokio::test]
async fn test_database_search_routing_consistency() -> Result<()> {
    // Test that the routing decision is consistent and predictable

    let temp_dir = TempDir::new()?;
    let db = TestDatabase::new(&temp_dir).await?;

    // Insert documents with patterns that might be ambiguous
    let test_docs = vec![
        ("star.txt", "Star document", "This file is named star"),
        ("*.config", "Wildcard config", "This is a config file"),
        ("test*file.txt", "Test pattern", "File with pattern in name"),
        ("normal.txt", "Normal file", "Just a normal file"),
    ];

    for (path, title, content) in test_docs {
        db.insert(path, title, content).await?;
    }

    // Test edge cases in routing

    // Query with * should always route to primary index
    let results = db.search("star*").await?;
    // Should match files starting with "star"
    assert!(
        results.iter().any(|d| d.path.as_str() == "star.txt"),
        "Pattern star* should match star.txt"
    );

    // Query without * should route to trigram index
    let results = db.search("star").await?;
    // Trigram search finds content/path containing "star"
    assert!(
        !results.is_empty(),
        "Text search for 'star' should find documents"
    );

    // Mixed patterns
    let results = db.search("*config*").await?;
    assert!(
        results.iter().any(|d| d.path.as_str() == "*.config"),
        "Pattern *config* should match *.config file"
    );

    Ok(())
}

#[tokio::test]
async fn test_database_search_performance_routing() -> Result<()> {
    // Test that wildcard queries don't accidentally trigger expensive trigram operations

    let temp_dir = TempDir::new()?;
    let db = TestDatabase::new(&temp_dir).await?;

    // Insert a large number of documents
    for i in 0..100 {
        let path = format!("file_{}.txt", i);
        let title = format!("Document {}", i);
        let content = format!("Content for document {}", i);
        db.insert(&path, &title, &content).await?;
    }

    // Wildcard query should be fast (primary index)
    let start = std::time::Instant::now();
    let results = db.search("file_*.txt").await?;
    let duration = start.elapsed();

    assert_eq!(results.len(), 100, "Should find all 100 files");
    assert!(
        duration.as_millis() < 100,
        "Wildcard search should complete within 100ms for 100 documents"
    );

    // Another pattern test
    let start = std::time::Instant::now();
    let results = db.search("file_1*.txt").await?;
    let duration = start.elapsed();

    // Should find file_1.txt, file_10.txt through file_19.txt (11 files)
    assert_eq!(results.len(), 11, "Should find 11 files matching file_1*");
    assert!(duration.as_millis() < 50, "Pattern search should be fast");

    Ok(())
}
