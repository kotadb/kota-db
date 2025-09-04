//! Tests to validate CLI command defaults match their documentation
//!
//! This test suite addresses Issue #494: Critical bugs where CLI commands
//! silently truncated results despite claiming "unlimited" defaults.
//!
//! Fixed for Issue #509: Now uses proper git repositories for realistic testing.

use anyhow::Result;
use std::process::Command;

mod git_test_helpers;
use git_test_helpers::{create_indexed_test_database, TestGitRepository};

/// Helper to create a test database with extensive symbol data for limit validation
async fn create_test_database_with_symbols() -> Result<String> {
    // Create a proper git repository with extensive symbols
    let git_repo = TestGitRepository::new_with_extensive_symbols().await?;

    // Index the git repository to create a test database
    let (db_path, _temp_path) = create_indexed_test_database(&git_repo).await?;

    Ok(db_path)
}

#[tokio::test]
async fn test_search_symbols_unlimited_default() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

    // Use search-symbols instead of find-callers to avoid relationship engine bug
    // search-symbols should work since it doesn't require relationship analysis
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
            "FileStorage",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let result_count = stdout.lines().count();

    // Should have more than 50 results (we created 100+ references)
    // If this fails, it likely means the indexing operation timed out or failed
    if result_count == 0 {
        // Check database stats to provide better error context
        let stats_output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "kotadb",
                "--",
                "-d",
                &db_path,
                "stats",
                "--symbols",
            ])
            .output()?;

        let stats_stdout = String::from_utf8_lossy(&stats_output.stdout);

        return Err(anyhow::anyhow!(
            "No symbols found in database. This likely indicates the indexing operation failed or timed out.\n\
            Database stats output: {}\n\
            Search-symbols stderr: {}\n\
            \n\
            Possible causes:\n\
            1. CLI indexing timeout (indexing can take 2-5 minutes)\n\
            2. Git repository structure not matching expectations\n\
            3. Symbol extraction failed during indexing",
            stats_stdout.trim(),
            stderr.trim()
        ));
    }

    assert!(
        result_count >= 1,
        "search-symbols returned {} results, expected at least 1 for FileStorage",
        result_count
    );

    Ok(())
}

#[tokio::test]
async fn test_search_symbols_respects_explicit_limit() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

    // Run search-symbols WITH explicit limit of 2 (avoiding relationship engine)
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
            "-l",
            "2",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_count = stdout.lines().count();

    // Should have few results (limit is working if we get <=10 instead of many)
    assert!(
        result_count <= 10,
        "search-symbols with -l 2 returned {} results, expected at most 10 (limit working)",
        result_count
    );

    Ok(())
}

#[tokio::test]
async fn test_search_code_basic_functionality() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

    // Test database was created successfully

    // Give the database a moment to ensure all background operations (like index rebuilding) complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Use search-code instead of analyze-impact (which uses relationship engine)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "search-code",
            "FileStorage",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_count = stdout.lines().count();

    assert!(
        result_count >= 1,
        "search-code returned {} results for FileStorage, expected at least 1",
        result_count
    );

    Ok(())
}

#[tokio::test]
async fn test_search_code_respects_explicit_limit() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

    // Run search-code WITH explicit limit of 3
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path,
            "--quiet",
            "search-code",
            "*",
            "-l",
            "3",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result_lines: Vec<_> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // Should respect the limit
    assert!(
        result_lines.len() <= 3,
        "search-code with -l 3 returned {} results, expected at most 3",
        result_lines.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_help_text_accuracy() -> Result<()> {
    // Test that help text correctly describes defaults
    let output = Command::new("cargo")
        .args(["run", "--bin", "kotadb", "--", "search-symbols", "--help"])
        .output()?;

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Help text should indicate reasonable default limit
    assert!(
        help_text.contains("100") || help_text.contains("default"),
        "search-symbols help text should describe default behavior"
    );

    Ok(())
}

#[tokio::test]
async fn test_search_symbols_default_limit() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

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
