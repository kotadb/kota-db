// Regression test for issue #337: Wildcard pattern filtering not working correctly
// This test ensures that wildcard patterns like "*.rs" properly filter documents
// instead of returning all documents in the database

use anyhow::Result;
use kotadb::{create_file_storage, DocumentBuilder, Index, QueryBuilder, Storage};
use tempfile::TempDir;

// Helper to create test storage and index
async fn create_test_environment() -> Result<(impl Storage, impl Index, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Ensure directories exist
    let storage_path = format!("{db_path}/storage");
    let index_path = format!("{db_path}/index");

    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(&index_path)?;
    std::fs::create_dir_all(format!("{storage_path}/documents"))?;
    std::fs::create_dir_all(format!("{storage_path}/indices"))?;
    std::fs::create_dir_all(format!("{storage_path}/wal"))?;
    std::fs::create_dir_all(format!("{storage_path}/meta"))?;

    let storage = create_file_storage(&storage_path, Some(100)).await?;
    let index = kotadb::create_primary_index_for_tests(&index_path).await?;

    Ok((storage, index, temp_dir))
}

#[tokio::test]
async fn test_issue_337_wildcard_returns_filtered_not_all() -> Result<()> {
    // This test verifies the fix for issue #337
    // Previously, wildcard patterns would return ALL documents instead of filtered results

    let (mut storage, mut index, _temp_dir) = create_test_environment().await?;

    // Create test documents with various file types
    let test_docs = vec![
        ("main.rs", "Main Rust file"),
        ("lib.rs", "Library Rust file"),
        ("test.rs", "Test Rust file"),
        ("README.md", "Markdown documentation"),
        ("package.json", "Node.js package file"),
        ("index.html", "HTML file"),
        ("style.css", "CSS stylesheet"),
        ("script.js", "JavaScript file"),
    ];

    for (path, title) in test_docs {
        let doc = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(format!("Content for {}", path).as_bytes())
            .build()?;

        storage.insert(doc.clone()).await?;
        index.insert(doc.id, doc.path.clone()).await?;
    }

    // Test 1: "*.rs" should return only Rust files, not all 8 documents
    let query = QueryBuilder::new().with_text("*.rs")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        3,
        "*.rs should find exactly 3 Rust files, not all {} documents",
        8
    );

    // Test 2: "*.md" should return only Markdown files
    let query = QueryBuilder::new().with_text("*.md")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 1, "*.md should find exactly 1 Markdown file");

    // Test 3: "*.json" should return only JSON files
    let query = QueryBuilder::new().with_text("*.json")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 1, "*.json should find exactly 1 JSON file");

    // Test 4: "*.html" should return only HTML files
    let query = QueryBuilder::new().with_text("*.html")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 1, "*.html should find exactly 1 HTML file");

    // Test 5: Non-existent extension should return empty
    let query = QueryBuilder::new().with_text("*.py")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        0,
        "*.py should find 0 files (no Python files exist)"
    );

    // Test 6: Pure wildcard "*" should return all documents
    let query = QueryBuilder::new().with_text("*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        8,
        "Pure wildcard * should return all 8 documents"
    );

    Ok(())
}

#[tokio::test]
async fn test_issue_337_routing_to_correct_index() -> Result<()> {
    // This test ensures wildcard queries are routed to the primary index
    // and not to the trigram index which doesn't support pattern matching

    let (mut storage, mut index, _temp_dir) = create_test_environment().await?;

    // Insert test documents
    let test_docs = vec![
        ("src/main.rs", "Main source"),
        ("src/lib.rs", "Library"),
        ("README.md", "Documentation"),
    ];

    for (path, title) in test_docs {
        let doc = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(format!("Content for {}", path).as_bytes())
            .build()?;

        storage.insert(doc.clone()).await?;
        index.insert(doc.id, doc.path.clone()).await?;
    }

    // Test wildcard search through the index
    let query = QueryBuilder::new().with_text("*.rs")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        2,
        "Database search for *.rs should find exactly 2 Rust files"
    );

    // Test that patterns with wildcards in different positions work
    let query = QueryBuilder::new().with_text("src/*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        2,
        "Pattern src/* should find files in src directory"
    );

    let query = QueryBuilder::new().with_text("*lib*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(
        results.len(),
        1,
        "Pattern *lib* should find files containing 'lib'"
    );

    Ok(())
}

#[tokio::test]
async fn test_issue_337_complex_wildcard_patterns() -> Result<()> {
    // Test more complex wildcard patterns to ensure comprehensive fix

    let (mut storage, mut index, _temp_dir) = create_test_environment().await?;

    // Create test documents with complex naming patterns
    let test_docs = vec![
        ("user_controller.rs", "User controller"),
        ("auth_controller.rs", "Auth controller"),
        ("user_service.rs", "User service"),
        ("auth_service.rs", "Auth service"),
        ("test_user.rs", "User tests"),
        ("test_auth.rs", "Auth tests"),
        ("user.model.ts", "User model"),
        ("auth.model.ts", "Auth model"),
    ];

    for (path, title) in test_docs {
        let doc = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(format!("Content for {}", path).as_bytes())
            .build()?;

        storage.insert(doc.clone()).await?;
        index.insert(doc.id, doc.path.clone()).await?;
    }

    // Test patterns with wildcards at different positions

    // Suffix pattern: all controllers
    let query = QueryBuilder::new().with_text("*_controller.rs")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 2, "Should find 2 controller files");

    // Prefix pattern: all test files
    let query = QueryBuilder::new().with_text("test_*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 2, "Should find 2 test files");

    // Middle wildcard: all user-related files
    let query = QueryBuilder::new().with_text("*user*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 4, "Should find 4 user-related files");

    // Multiple wildcards: TypeScript model files
    let query = QueryBuilder::new().with_text("*.model.ts")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 2, "Should find 2 TypeScript model files");

    // Extension wildcard with prefix (auth_controller.rs, auth_service.rs, auth.model.ts)
    let query = QueryBuilder::new().with_text("auth*")?.build()?;
    let results = index.search(&query).await?;
    assert_eq!(results.len(), 3, "Should find 3 auth-prefixed files");

    Ok(())
}
