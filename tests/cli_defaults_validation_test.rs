//! Tests to validate CLI command defaults match their documentation
//!
//! This test suite addresses Issue #494: Critical bugs where CLI commands
//! silently truncated results despite claiming "unlimited" defaults.

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test database with sample data
async fn create_test_database_with_symbols() -> Result<(TempDir, String)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let db_path_str = db_path.to_str().unwrap().to_string();

    // Create sample Rust files with many symbols for testing
    let sample_code = r#"
// Test file with many symbols to validate result limits
pub struct FileStorage {
    path: String,
}

impl FileStorage {
    pub fn new(path: String) -> Self {
        FileStorage { path }
    }
    
    pub fn insert(&mut self, data: &str) {
        // Implementation
    }
    
    pub fn get(&self) -> String {
        self.path.clone()
    }
}

pub fn create_file_storage() -> FileStorage {
    FileStorage::new("/tmp".to_string())
}

pub fn use_storage() {
    let storage = FileStorage::new("/data".to_string());
    storage.get();
}

// Generate many test functions to ensure we have >50 results
"#;

    // Add many test functions to ensure we exceed default limit
    let mut extended_code = sample_code.to_string();
    for i in 0..100 {
        extended_code.push_str(&format!(
            r#"
#[test]
fn test_function_{}() {{
    let storage = FileStorage::new("/test{}".to_string());
    storage.insert("test");
    storage.get();
}}
"#,
            i, i
        ));
    }

    // Write test files
    std::fs::write(temp_dir.path().join("lib.rs"), &extended_code)?;
    std::fs::write(temp_dir.path().join("main.rs"), &extended_code)?;
    std::fs::write(temp_dir.path().join("test.rs"), &extended_code)?;

    // Index the codebase
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path_str,
            "index-codebase",
        ])
        .arg(temp_dir.path())
        .output()?;

    assert!(
        output.status.success(),
        "Failed to index codebase: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok((temp_dir, db_path_str))
}

#[tokio::test]
async fn test_find_callers_unlimited_default() -> Result<()> {
    let (_temp_dir, db_path) = create_test_database_with_symbols().await?;

    // Run find-callers WITHOUT limit flag - should return ALL results
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "find-callers",
            "FileStorage",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_count = stdout.lines().count();

    // Should have more than 50 results (we created 100+ references)
    assert!(
        result_count > 50,
        "find-callers returned only {} results without limit flag, expected >50 (unlimited)",
        result_count
    );

    Ok(())
}

#[tokio::test]
async fn test_find_callers_respects_explicit_limit() -> Result<()> {
    let (_temp_dir, db_path) = create_test_database_with_symbols().await?;

    // Run find-callers WITH explicit limit of 10
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "find-callers",
            "FileStorage",
            "-l",
            "10",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_count = stdout.lines().count();

    // Should have exactly 10 results
    assert_eq!(
        result_count, 10,
        "find-callers with -l 10 returned {} results, expected exactly 10",
        result_count
    );

    Ok(())
}

#[tokio::test]
async fn test_analyze_impact_unlimited_default() -> Result<()> {
    let (_temp_dir, db_path) = create_test_database_with_symbols().await?;

    // Run analyze-impact WITHOUT limit flag - should return ALL results
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "analyze-impact",
            "FileStorage",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not show truncation message
    assert!(
        !stdout.contains("showing first"),
        "analyze-impact shows truncation message despite unlimited default"
    );

    Ok(())
}

#[tokio::test]
async fn test_analyze_impact_respects_explicit_limit() -> Result<()> {
    let (_temp_dir, db_path) = create_test_database_with_symbols().await?;

    // Run analyze-impact WITH explicit limit of 5
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "analyze-impact",
            "FileStorage",
            "-l",
            "5",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_lines: Vec<_> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // Should respect the limit
    assert!(
        result_lines.len() <= 5,
        "analyze-impact with -l 5 returned {} results, expected at most 5",
        result_lines.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_help_text_accuracy() -> Result<()> {
    // Test that help text correctly describes defaults
    let output = Command::new("cargo")
        .args(["run", "--bin", "kotadb", "--", "find-callers", "--help"])
        .output()?;

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Help text should indicate unlimited default
    assert!(
        help_text.contains("unlimited") || help_text.contains("no limit"),
        "find-callers help text doesn't mention unlimited default"
    );

    // Help text should NOT claim a specific default number
    assert!(
        !help_text.contains("default: 50"),
        "find-callers help text incorrectly claims default: 50"
    );

    Ok(())
}

#[tokio::test]
async fn test_search_symbols_default_limit() -> Result<()> {
    let (_temp_dir, db_path) = create_test_database_with_symbols().await?;

    // search-symbols SHOULD have a reasonable default limit (100)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "search-symbols",
            "*",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_count = stdout.lines().count();

    // Should have a reasonable limit by default
    assert!(
        result_count <= 100,
        "search-symbols returned {} results without limit, expected reasonable default (<=100)",
        result_count
    );

    Ok(())
}
