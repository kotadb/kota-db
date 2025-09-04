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
async fn test_find_callers_unlimited_default() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

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
    let stderr = String::from_utf8_lossy(&output.stderr);
    let result_count = stdout.lines().count();

    // Debug: Check what's actually in the database first
    let stats_output = Command::new("cargo")
        .args(["run", "--bin", "kotadb", "--", "-d", &db_path, "stats"])
        .output()?;

    eprintln!("DEBUG: Database stats status: {:?}", stats_output.status);
    eprintln!(
        "DEBUG: Database stats stdout:\n{}",
        String::from_utf8_lossy(&stats_output.stdout)
    );
    eprintln!(
        "DEBUG: Database stats stderr:\n{}",
        String::from_utf8_lossy(&stats_output.stderr)
    );

    // Debug output for troubleshooting
    eprintln!("DEBUG: find-callers command status: {:?}", output.status);
    eprintln!("DEBUG: find-callers stdout:\n{}", stdout);
    eprintln!("DEBUG: find-callers stderr:\n{}", stderr);
    eprintln!("DEBUG: result count: {}", result_count);

    // Should have more than 50 results (we created 100+ references)
    assert!(
        result_count > 50,
        "find-callers returned only {} results without limit flag, expected >50 (unlimited)\nStdout: {}\nStderr: {}",
        result_count, stdout, stderr
    );

    Ok(())
}

#[tokio::test]
async fn test_find_callers_respects_explicit_limit() -> Result<()> {
    let db_path = create_test_database_with_symbols().await?;

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
    let db_path = create_test_database_with_symbols().await?;

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
    let db_path = create_test_database_with_symbols().await?;

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
