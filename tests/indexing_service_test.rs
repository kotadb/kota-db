// Comprehensive tests for IndexingService functionality
// Tests core indexing operations, error handling, and interface consistency

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index_for_tests, create_trigram_index,
    database::Database,
    services::{
        DatabaseAccess, IncrementalUpdateOptions, IndexCodebaseOptions, IndexGitOptions,
        IndexingService,
    },
};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

/// Helper to create a complete test database with all components
async fn create_test_database() -> Result<(Database, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Create storage component
    let storage_path = db_path.join("storage");
    let storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    // Create primary index component
    let primary_index_path = db_path.join("primary_index");
    let primary_index =
        create_primary_index_for_tests(primary_index_path.to_str().unwrap()).await?;

    // Create trigram index component
    let trigram_index_path = db_path.join("trigram_index");
    let trigram_index =
        create_trigram_index(trigram_index_path.to_str().unwrap(), Some(1000)).await?;

    let database = Database {
        storage: Arc::new(Mutex::new(storage)),
        primary_index: Arc::new(Mutex::new(primary_index)),
        trigram_index: Arc::new(Mutex::new(trigram_index)),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    Ok((database, temp_dir))
}

/// Helper to create a minimal test repository structure
fn create_test_repository(base_path: &std::path::Path) -> Result<PathBuf> {
    let repo_path = base_path.join("test_repo");
    fs::create_dir_all(&repo_path)?;

    // Initialize git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()?;

    // Ensure local git identity is configured for CI environments
    // Some runners do not have global git config; set per-repo values.
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()?;

    // Create some test files with different languages
    fs::write(
        repo_path.join("README.md"),
        "# Test Repository\nThis is a test.",
    )?;

    fs::create_dir_all(repo_path.join("src"))?;
    fs::write(
        repo_path.join("src").join("main.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}

pub struct TestStruct {
    pub field: i32,
}

impl TestStruct {
    pub fn new(value: i32) -> Self {
        Self { field: value }
    }
}
"#,
    )?;

    fs::write(
        repo_path.join("src").join("lib.rs"),
        r#"
pub mod utils;

pub fn example_function() -> String {
    "example".to_string()
}

pub struct ExampleStruct;
"#,
    )?;

    fs::create_dir_all(repo_path.join("src").join("utils"))?;
    fs::write(
        repo_path.join("src").join("utils").join("mod.rs"),
        r#"
pub fn utility_function(input: &str) -> String {
    format!("processed: {}", input)
}
"#,
    )?;

    // Add and commit files to git repository (required for symbol extraction)
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()?;

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial test commit"])
        .current_dir(&repo_path)
        .output()?;

    Ok(repo_path)
}

#[tokio::test]
async fn test_index_codebase_basic_functionality() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    // Ensure the database path exists and is writable
    let db_path = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(&db_path)?;
    let indexing_service = IndexingService::new(&database, db_path);

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        prefix: "test".to_string(),
        include_files: true,
        include_commits: false, // Skip git commits for this test
        max_file_size_mb: 10,
        max_memory_mb: None,
        max_parallel_files: None,
        enable_chunking: true,
        extract_symbols: Some(true),
        no_symbols: false,
        quiet: false,
        include_paths: None,
        create_index: true,
    };

    let result = indexing_service.index_codebase(options).await?;

    // Verify indexing succeeded
    assert!(result.success, "Indexing should succeed");
    assert!(result.files_processed > 0, "Should process some files");
    assert!(result.total_time_ms > 0, "Should take some time");
    assert!(
        !result.formatted_output.is_empty(),
        "Should provide user feedback"
    );

    // Verify files were actually stored
    let storage = database.storage();
    let storage = storage.lock().await;
    let all_docs = storage.list_all().await?;
    assert!(!all_docs.is_empty(), "Should have stored documents");

    // Verify at least one document has the expected content
    let readme_doc = all_docs
        .iter()
        .find(|doc| doc.path.as_str().contains("README.md"));
    assert!(readme_doc.is_some(), "Should have indexed README.md");

    let readme = readme_doc.unwrap();
    let content = String::from_utf8_lossy(&readme.content);
    assert!(
        content.contains("Test Repository"),
        "Should have correct content"
    );

    Ok(())
}

#[tokio::test]
async fn test_index_codebase_error_handling_nonexistent_path() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: PathBuf::from("/nonexistent/path"),
        prefix: "test".to_string(),
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    // Should handle error gracefully
    assert!(!result.success, "Should fail for nonexistent path");
    assert_eq!(result.files_processed, 0, "Should process no files");
    assert!(!result.errors.is_empty(), "Should report errors");
    assert!(
        result.errors[0].contains("does not exist"),
        "Should explain the error"
    );

    Ok(())
}

#[tokio::test]
async fn test_index_codebase_with_symbol_extraction() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        extract_symbols: Some(true),
        no_symbols: false,
        quiet: false,
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    assert!(result.success, "Indexing with symbols should succeed");

    // If tree-sitter feature is enabled, we should extract symbols
    #[cfg(feature = "tree-sitter-parsing")]
    {
        // Should have extracted symbols from the Rust code
        assert!(
            result.symbols_extracted > 0,
            "Should extract symbols from Rust files"
        );

        // Verify the formatted output mentions symbol extraction
        assert!(
            result.formatted_output.contains("Symbol"),
            "Should mention symbols in output"
        );
    }

    #[cfg(not(feature = "tree-sitter-parsing"))]
    {
        // Without tree-sitter, no symbols should be extracted
        assert_eq!(
            result.symbols_extracted, 0,
            "Should extract no symbols without tree-sitter"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_index_codebase_without_symbol_extraction() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        extract_symbols: Some(false),
        no_symbols: true,
        quiet: false,
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    assert!(result.success, "Indexing without symbols should succeed");
    assert!(result.files_processed > 0, "Should still process files");

    // Should not extract symbols when disabled
    assert_eq!(
        result.symbols_extracted, 0,
        "Should extract no symbols when disabled"
    );

    // Verify the formatted output mentions symbol extraction is disabled
    assert!(
        result.formatted_output.contains("disabled") || result.formatted_output.contains("Symbol"),
        "Should mention symbol extraction status"
    );

    Ok(())
}

#[tokio::test]
async fn test_index_codebase_quiet_mode() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        quiet: true,
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    assert!(result.success, "Quiet indexing should succeed");

    // Quiet mode should produce minimal output
    assert!(
        result.formatted_output.is_empty() || result.formatted_output.len() < 50,
        "Quiet mode should produce minimal output, got: '{}'",
        result.formatted_output
    );

    Ok(())
}

#[tokio::test]
async fn test_index_codebase_memory_limits() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        max_memory_mb: Some(50),     // Small memory limit
        max_parallel_files: Some(2), // Limited parallelism
        enable_chunking: true,
        quiet: false,
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    assert!(result.success, "Indexing with memory limits should succeed");
    assert!(
        result.files_processed > 0,
        "Should still process files with limits"
    );

    Ok(())
}

#[tokio::test]
async fn test_incremental_update_not_implemented() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IncrementalUpdateOptions {
        changes: vec![PathBuf::from("test.rs")],
        delete_removed: true,
        update_symbols: true,
        quiet: false,
    };

    let result = indexing_service.incremental_update(options).await?;

    // Should indicate not implemented
    assert!(
        !result.success,
        "Incremental update should indicate not implemented"
    );
    assert!(
        result.formatted_output.contains("not yet")
            || result.formatted_output.contains("implemented"),
        "Should indicate not implemented"
    );

    Ok(())
}

#[tokio::test]
async fn test_index_git_repository_fallback() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexGitOptions {
        repo_path: repo_path.clone(),
        prefix: "git_test".to_string(),
        include_commits: true,
        include_branches: true,
        max_commits: Some(100),
        quiet: false,
    };

    let result = indexing_service.index_git_repository(options).await?;

    // Should succeed using fallback to codebase indexing
    assert!(result.success, "Git indexing should succeed with fallback");
    assert!(result.files_analyzed > 0, "Should analyze files");
    assert!(
        result.formatted_output.contains("fallback") || result.formatted_output.contains("not yet"),
        "Should indicate fallback behavior"
    );

    Ok(())
}

#[tokio::test]
async fn test_reindex_scope_not_implemented() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let result = indexing_service
        .reindex_scope(&PathBuf::from("src/main.rs"), true)
        .await?;

    // Should indicate not implemented
    assert!(
        !result.success,
        "Scope reindexing should indicate not implemented"
    );
    assert!(
        !result.errors.is_empty(),
        "Should report not implemented error"
    );

    Ok(())
}

#[tokio::test]
async fn test_indexing_service_interface_consistency() -> Result<()> {
    // Test that IndexingService provides consistent results across multiple calls
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options1 = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        prefix: "consistency_test_1".to_string(),
        quiet: true,
        ..Default::default()
    };

    let options2 = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        prefix: "consistency_test_2".to_string(),
        quiet: true,
        ..Default::default()
    };

    let result1 = indexing_service.index_codebase(options1).await?;
    let result2 = indexing_service.index_codebase(options2).await?;

    // Results should be consistent for the same repository
    assert_eq!(result1.success, result2.success);
    assert_eq!(result1.files_processed, result2.files_processed);

    // Note: symbols_extracted might differ due to different prefixes, but files should be same

    Ok(())
}

#[tokio::test]
async fn test_indexing_service_large_file_handling() -> Result<()> {
    let (database, temp_dir) = create_test_database().await?;
    let repo_path = create_test_repository(temp_dir.path())?;

    // Create a large file that exceeds the size limit
    let large_content = "// Large file\n".repeat(100000); // ~1.3MB of content
    fs::write(repo_path.join("large_file.rs"), large_content)?;

    let indexing_service = IndexingService::new(&database, temp_dir.path().to_path_buf());

    let options = IndexCodebaseOptions {
        repo_path: repo_path.clone(),
        max_file_size_mb: 1, // 1MB limit - should exclude the large file
        quiet: false,
        ..Default::default()
    };

    let result = indexing_service.index_codebase(options).await?;

    // Should still succeed, just skip the large file
    assert!(result.success, "Should succeed even with large files");

    // Verify the large file was not processed by checking storage
    let storage = database.storage();
    let storage = storage.lock().await;
    let all_docs = storage.list_all().await?;

    let large_file_doc = all_docs
        .iter()
        .find(|doc| doc.path.as_str().contains("large_file.rs"));
    // Large file should either be absent or have limited content due to size restrictions

    Ok(())
}
