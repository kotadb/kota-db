//! Git Repository Test Helpers
//!
//! This module provides utilities for creating real git repositories in tests,
//! following KotaDB's anti-mock testing philosophy. These helpers ensure that
//! integration tests run against realistic git repositories instead of mocked
//! or empty directories.
//!
//! Addresses Issue #509: Fix failing integration tests with git repository setup issues

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// A test git repository with proper initialization and sample code
pub struct TestGitRepository {
    pub temp_dir: TempDir,
    pub path: String,
}

impl TestGitRepository {
    /// Creates a new test git repository with realistic Rust code structure
    ///
    /// This creates a proper git repository with:
    /// - Git initialization with proper config
    /// - Sample Rust code files with realistic symbols
    /// - Multiple commits to simulate a real repository
    /// - Proper file structure that KotaDB can analyze
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory for test git repository")?;

        let repo_path = temp_dir.path();
        let path_str = repo_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in repository path"))?
            .to_string();

        // Initialize git repository
        Self::run_git_command(repo_path, &["init"])
            .context("Failed to initialize git repository")?;

        // Configure git for testing (required for commits)
        Self::run_git_command(repo_path, &["config", "user.name", "KotaDB Test"])
            .context("Failed to configure git user name")?;
        Self::run_git_command(repo_path, &["config", "user.email", "test@kotadb.dev"])
            .context("Failed to configure git user email")?;

        // Create initial commit with basic structure
        Self::create_initial_structure(repo_path)
            .await
            .context("Failed to create initial repository structure")?;

        Ok(TestGitRepository {
            temp_dir,
            path: path_str,
        })
    }

    /// Creates a test git repository with extensive symbol data for limit testing
    ///
    /// This variant generates large amounts of symbols and relationships to test
    /// CLI commands that need to validate result limits and pagination.
    pub async fn new_with_extensive_symbols() -> Result<Self> {
        let repo = Self::new().await?;

        Self::add_extensive_symbol_data(repo.temp_dir.path())
            .await
            .context("Failed to add extensive symbol data")?;

        Ok(repo)
    }

    /// Creates minimal repository structure for fast testing
    async fn create_initial_structure(repo_path: &Path) -> Result<()> {
        // Create the simplest possible Rust file that still has symbols
        // Exactly matching the working integration test pattern
        let rust_content = r#"pub struct FileStorage {
    field1: String,
    field2: i32,
}

impl FileStorage {
    pub fn new() -> Self {
        FileStorage {
            field1: String::new(),
            field2: 0,
        }
    }
    
    pub fn method1(&self) -> &str {
        &self.field1
    }
}

pub fn standalone_function(x: i32) -> i32 {
    x * 2
}

pub enum TestEnum {
    Variant1,
    Variant2(String),
}

pub const TEST_CONSTANT: i32 = 42;
"#;

        fs::write(repo_path.join("test.rs"), rust_content)?;

        // Create lib.rs - minimal
        let lib_content = r#"mod test;

pub fn library_function() {
    println!("Library function");
}
"#;

        fs::write(repo_path.join("lib.rs"), lib_content)?;

        // Add all files and create initial commit
        Self::run_git_command(repo_path, &["add", "."]).context("Failed to add files to git")?;
        Self::run_git_command(repo_path, &["commit", "-m", "Initial commit"])
            .context("Failed to create initial commit")?;

        Ok(())
    }

    /// Adds just enough symbols for testing result limits (smaller set for performance)
    async fn add_extensive_symbol_data(repo_path: &Path) -> Result<()> {
        // Create just enough functions to test limits without causing performance issues
        let mut extensive_code =
            String::from("// Functions that use FileStorage\nuse crate::test::*;\n\n");

        // Add functions that reference FileStorage - enough to test limits but not too many
        for i in 0..60 {
            extensive_code.push_str(&format!(
                r#"pub fn test_function_{i}() {{
    let storage = FileStorage::new();
    storage.method1();
}}

"#,
            ));
        }

        fs::write(repo_path.join("callers.rs"), extensive_code)
            .context("Failed to create callers.rs")?;

        // Update lib.rs to include the new module
        let lib_path = repo_path.join("lib.rs");
        let mut lib_content = fs::read_to_string(&lib_path).context("Failed to read lib.rs")?;
        lib_content.push_str("\nmod callers;\n");
        fs::write(&lib_path, lib_content).context("Failed to update lib.rs with callers module")?;

        // Commit the additional symbols
        Self::run_git_command(repo_path, &["add", "."])
            .context("Failed to add caller files to git")?;
        Self::run_git_command(repo_path, &["commit", "-m", "Add caller functions"])
            .context("Failed to commit caller functions")?;

        Ok(())
    }

    /// Runs a git command in the specified repository directory
    fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .context(format!(
                "Failed to execute git command: git {}",
                args.join(" ")
            ))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Git command failed: git {}\nStderr: {}\nStdout: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        Ok(())
    }

    /// Returns the path to the git repository as a Path
    pub fn path_ref(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Creates a test database with the given git repository
///
/// This helper function takes a TestGitRepository and runs the KotaDB
/// index-codebase command on it, returning the database path for use in tests.
pub async fn create_indexed_test_database(
    git_repo: &TestGitRepository,
) -> Result<(String, String)> {
    let db_temp_dir =
        TempDir::new().context("Failed to create temporary directory for test database")?;

    let db_path = db_temp_dir.path().join("test_db");
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in database path"))?
        .to_string();

    // Repository created successfully

    // Index the git repository using KotaDB (symbols are enabled by default)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path_str,
            "index-codebase",
            &git_repo.path,
        ])
        .output()
        .context("Failed to execute kotadb index-codebase command")?;

    // Check indexing results
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to index codebase. Status: {:?}\nStderr: {}\nStdout: {}",
            output.status.code(),
            stderr,
            stdout
        ));
    }

    // Return both the database path and a path that keeps the temp directory alive
    // We return the TempDir path to ensure it doesn't get dropped
    Ok((
        db_path_str,
        db_temp_dir.path().to_str().unwrap().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_repository_creation() -> Result<()> {
        let repo = TestGitRepository::new().await?;

        // Verify git repository was created properly
        let git_dir = repo.path_ref().join(".git");
        assert!(git_dir.exists(), "Git directory should exist");

        // Verify files were created (they are in the root, not src/)
        let lib_file = repo.path_ref().join("lib.rs");
        assert!(lib_file.exists(), "lib.rs should exist");

        // Verify git log shows commits
        let output = Command::new("git")
            .current_dir(repo.path_ref())
            .args(["log", "--oneline"])
            .output()?;

        assert!(output.status.success(), "Git log should work");
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(log.contains("Initial commit"), "Should have initial commit");

        Ok(())
    }

    #[tokio::test]
    async fn test_extensive_symbols_repository() -> Result<()> {
        let repo = TestGitRepository::new_with_extensive_symbols().await?;

        // Verify callers file was created (it's callers.rs, not extensive_symbols.rs)
        let extensive_file = repo.path_ref().join("callers.rs");
        assert!(extensive_file.exists(), "callers.rs should exist");

        // Verify the file contains many symbols
        let content = fs::read_to_string(&extensive_file)?;
        assert!(
            content.contains("test_function_30"),
            "Should contain test functions"
        );
        assert!(
            content.contains("FileStorage::new"),
            "Should contain FileStorage references"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_indexed_database_creation() -> Result<()> {
        let repo = TestGitRepository::new().await?;
        let (db_path, _temp_path) = create_indexed_test_database(&repo).await?;

        // Verify database directory was created
        assert!(
            Path::new(&db_path).exists(),
            "Database directory should exist"
        );

        Ok(())
    }
}
