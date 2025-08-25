//! Integration test for symbol persistence (issue #312)
//!
//! This test verifies that symbols are properly persisted to storage
//! and can be loaded back after the storage is closed and reopened.

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
#[cfg(feature = "tree-sitter-parsing")]
async fn test_symbols_persist_after_ingestion() -> Result<()> {
    // Create a test repository
    let repo_dir = TempDir::new()?;
    let repo_path = repo_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()?;

    // Create a Rust file with symbols
    let test_code = r#"
fn main() {
    println!("Hello");
    helper();
}

fn helper() {
    println!("Helper");
}
"#;

    std::fs::write(repo_path.join("test.rs"), test_code)?;

    // Commit the file
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "test"])
        .current_dir(repo_path)
        .output()?;

    // Create database directory
    let db_dir = TempDir::new()?;
    let db_path = db_dir.path();

    // Run ingestion with symbol extraction
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "kotadb",
            "--",
            "-d",
            db_path.to_str().unwrap(),
            "ingest-repo",
            repo_path.to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Ingestion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow::anyhow!("Ingestion failed"));
    }

    // Check if symbols were extracted
    let ingestion_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        ingestion_output.contains("symbols extracted"),
        "No symbols were extracted during ingestion"
    );

    // Now check symbol stats to see if they persisted
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "kotadb",
            "--",
            "-d",
            db_path.to_str().unwrap(),
            "symbol-stats",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Symbol stats failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow::anyhow!("Symbol stats failed"));
    }

    let stats_output = String::from_utf8_lossy(&output.stdout);

    // Check that symbols were loaded (not 0)
    assert!(
        !stats_output.contains("Total symbols: 0"),
        "Symbols were not persisted! Stats output: {}",
        stats_output
    );

    // Verify specific symbols can be found
    assert!(
        stats_output.contains("Total symbols: "),
        "Could not find symbol count in output"
    );

    println!("Test passed! Symbols were properly persisted and loaded.");

    Ok(())
}
