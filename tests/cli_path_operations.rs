// Integration tests for CLI path-based operations
// Tests the fix for issue #131: CLI commands consistently use paths

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run CLI commands
fn run_cli_command(db_path: &str, args: &[&str]) -> Result<String> {
    // Use release binary in CI, debug otherwise
    let binary_path = if std::env::var("CI").is_ok() {
        "./target/release/kotadb"
    } else {
        "./target/debug/kotadb"
    };

    // Check if binary exists and provide helpful error
    if !std::path::Path::new(binary_path).exists() {
        eprintln!(
            "Binary not found at: {}. Current dir: {:?}",
            binary_path,
            std::env::current_dir()
        );
        return Err(anyhow::anyhow!(
            "KotaDB binary not found at {}. Please run 'cargo build --release' first.",
            binary_path
        ));
    }

    let output = Command::new(binary_path)
        .arg("--db-path")
        .arg(db_path)
        .args(args)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Include stderr in output for debugging
    Ok(format!("{}\n{}", stdout, stderr))
}

#[test]
fn test_cli_insert_and_get_by_path() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert a document with relative path
    let output = run_cli_command(
        db_path,
        &["insert", "test/doc.md", "Test Doc", "Test content"],
    )?;
    assert!(output.contains("Document inserted successfully"));

    // Get the document by path
    let output = run_cli_command(db_path, &["get", "test/doc.md"])?;
    assert!(output.contains("Document found"));
    assert!(output.contains("test/doc.md"));
    assert!(output.contains("Test Doc"));
    assert!(output.contains("Test content"));

    Ok(())
}

#[test]
fn test_cli_update_by_path() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert a document with relative path
    run_cli_command(
        db_path,
        &["insert", "test/doc.md", "Original", "Original content"],
    )?;

    // Update the document by path
    let output = run_cli_command(
        db_path,
        &[
            "update",
            "test/doc.md",
            "--title",
            "Updated",
            "--content",
            "Updated content",
        ],
    )?;

    // Debug output if test fails
    if !output.contains("Document updated successfully") {
        eprintln!("Update output: {}", output);
    }
    assert!(
        output.contains("Document updated successfully"),
        "Output was: {}",
        output
    );

    // Verify the update
    let output = run_cli_command(db_path, &["get", "test/doc.md"])?;
    assert!(output.contains("Updated"));
    assert!(output.contains("Updated content"));
    assert!(!output.contains("Original"));

    Ok(())
}

#[test]
fn test_cli_delete_by_path() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert a document with relative path
    run_cli_command(
        db_path,
        &["insert", "test/doc.md", "To Delete", "Delete me"],
    )?;

    // Delete the document by path
    let output = run_cli_command(db_path, &["delete", "test/doc.md"])?;
    assert!(output.contains("Document deleted successfully"));

    // Verify deletion
    let output = run_cli_command(db_path, &["get", "test/doc.md"])?;
    assert!(output.contains("Document not found"));

    Ok(())
}

#[test]
fn test_cli_path_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Try to get non-existent document with relative path
    let output = run_cli_command(db_path, &["get", "nonexistent.md"])?;
    assert!(output.contains("Document not found"));

    // Try to update non-existent document with relative path
    let output = run_cli_command(db_path, &["update", "nonexistent.md", "--title", "New"])?;
    assert!(
        output.contains("not found") || output.contains("Error"),
        "Output: {}",
        output
    );

    // Try to delete non-existent document with relative path
    let output = run_cli_command(db_path, &["delete", "nonexistent.md"])?;
    assert!(output.contains("Document not found"));

    Ok(())
}

#[test]
fn test_cli_update_path_change() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert a document with relative path
    run_cli_command(db_path, &["insert", "old/path.md", "Test", "Content"])?;

    // Update with new path
    let output = run_cli_command(
        db_path,
        &["update", "old/path.md", "--new-path", "new/path.md"],
    )?;

    // Debug output if test fails
    if !output.contains("Document updated successfully") {
        eprintln!("Update path change output: {}", output);
    }
    assert!(
        output.contains("Document updated successfully"),
        "Output was: {}",
        output
    );

    // Old path should not exist
    let output = run_cli_command(db_path, &["get", "old/path.md"])?;
    assert!(output.contains("Document not found"));

    // New path should exist
    let output = run_cli_command(db_path, &["get", "new/path.md"])?;
    assert!(output.contains("Document found"));
    assert!(output.contains("new/path.md"));

    Ok(())
}

#[test]
fn test_cli_multiple_documents_with_paths() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert multiple documents with relative paths
    let paths = vec![
        ("docs/readme.md", "README", "Read me first"),
        ("docs/guide.md", "Guide", "User guide"),
        ("notes/todo.md", "TODO", "Task list"),
    ];

    for (path, title, content) in &paths {
        run_cli_command(db_path, &["insert", path, title, content])?;
    }

    // Verify each can be retrieved by path
    for (path, title, content) in &paths {
        let output = run_cli_command(db_path, &["get", path])?;
        assert!(output.contains(path));
        assert!(output.contains(title));
        assert!(output.contains(content));
    }

    // List all documents
    let output = run_cli_command(db_path, &["list"])?;
    assert!(output.contains("3 total"));
    for (path, _, _) in &paths {
        assert!(output.contains(path));
    }

    Ok(())
}

/// Performance test for path cache
#[test]
fn test_path_cache_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();

    // Insert 100 documents with relative paths
    for i in 0..100 {
        let path = format!("test/doc{}.md", i);
        let title = format!("Document {}", i);
        let content = format!("Content for document {}", i);
        run_cli_command(db_path, &["insert", &path, &title, &content])?;
    }

    // Time a get operation (should be fast with cache)
    let start = std::time::Instant::now();
    let output = run_cli_command(db_path, &["get", "test/doc50.md"])?;
    let duration = start.elapsed();

    assert!(output.contains("Document found"));
    assert!(output.contains("Document 50"));

    // Should be very fast (< 100ms even with CLI overhead)
    assert!(
        duration.as_millis() < 100,
        "Get by path took too long: {:?}",
        duration
    );

    Ok(())
}
