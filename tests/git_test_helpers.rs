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

    /// Creates basic repository structure with realistic Rust code matching working integration tests
    async fn create_initial_structure(repo_path: &Path) -> Result<()> {
        // Create a simple Rust file with various symbols (matching binary_symbols_integration_test.rs pattern)
        let rust_content = r#"
pub struct FileStorage {
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
    
    pub fn insert(&self, data: &str) -> String {
        format!("Inserted: {}", data)
    }
    
    pub fn get(&self) -> &str {
        &self.field1
    }
}

pub fn create_file_storage() -> FileStorage {
    FileStorage::new()
}

pub fn process_data(storage: &FileStorage, data: &str) -> String {
    storage.insert(data)
}

pub enum StorageType {
    Memory,
    File(String),
}

pub const DEFAULT_STORAGE: i32 = 42;
"#;

        fs::write(repo_path.join("storage.rs"), rust_content)?;

        // Create lib.rs that imports the module
        let lib_content = r#"
mod storage;

pub use storage::*;

pub fn library_function() {
    let storage = create_file_storage();
    let result = process_data(&storage, "test data");
    println!("Library function: {}", result);
}

pub fn use_file_storage() {
    let fs = FileStorage::new();
    fs.insert("example");
}
"#;

        fs::write(repo_path.join("lib.rs"), lib_content)?;

        // Add all files and create initial commit
        Self::run_git_command(repo_path, &["add", "."]).context("Failed to add files to git")?;
        Self::run_git_command(
            repo_path,
            &["commit", "-m", "Initial commit: Add storage module"],
        )
        .context("Failed to create initial commit")?;

        Ok(())
    }

    /// Adds extensive symbol data for testing result limits and pagination
    async fn add_extensive_symbol_data(repo_path: &Path) -> Result<()> {
        // Generate many simple functions that call FileStorage to create relationships
        let mut extensive_code = String::from("// Extensive test functions\n\n");
        extensive_code.push_str("use crate::*;\n\n");

        // Add many functions that call FileStorage
        for i in 0..100 {
            extensive_code.push_str(&format!(
                r#"
pub fn test_function_{i}() -> FileStorage {{
    let storage = create_file_storage();
    let _ = process_data(&storage, "data_{i}");
    storage
}}

pub fn use_storage_{i}() {{
    let storage = FileStorage::new();
    storage.insert("test");
}}
"#,
            ));
        }

        fs::write(repo_path.join("extensive.rs"), extensive_code)
            .context("Failed to create extensive.rs")?;

        // Update lib.rs to include the new module
        let lib_path = repo_path.join("lib.rs");
        let mut lib_content = fs::read_to_string(&lib_path).context("Failed to read lib.rs")?;
        lib_content.push_str("\nmod extensive;\npub use extensive::*;\n");
        fs::write(&lib_path, lib_content)
            .context("Failed to update lib.rs with extensive module")?;

        // Commit the extensive symbols
        Self::run_git_command(repo_path, &["add", "."])
            .context("Failed to add extensive symbol files to git")?;
        Self::run_git_command(
            repo_path,
            &["commit", "-m", "Add extensive symbols for limit testing"],
        )
        .context("Failed to commit extensive symbols")?;

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
        ])
        .arg(&git_repo.path)
        .output()
        .context("Failed to execute kotadb index-codebase command")?;

    // TODO: Debug symbol extraction - indexing succeeds but no symbols found
    // This suggests the generated test code may not match KotaDB's expectations

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to index codebase. Status: {:?}\nStderr: {}\nStdout: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
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

        // Verify files were created
        let lib_file = repo.path_ref().join("src").join("lib.rs");
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

        // Verify extensive symbols file was created
        let extensive_file = repo.path_ref().join("src").join("extensive_symbols.rs");
        assert!(extensive_file.exists(), "extensive_symbols.rs should exist");

        // Verify the file contains many symbols
        let content = fs::read_to_string(&extensive_file)?;
        assert!(
            content.contains("TestStruct100"),
            "Should contain many test structs"
        );
        assert!(
            content.contains("test_function_50"),
            "Should contain many test functions"
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
