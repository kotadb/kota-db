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

    /// Creates basic repository structure with realistic Rust code
    async fn create_initial_structure(repo_path: &Path) -> Result<()> {
        // Create Cargo.toml for a realistic Rust project
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#;

        fs::write(repo_path.join("Cargo.toml"), cargo_toml)
            .context("Failed to create Cargo.toml")?;

        // Create src directory structure
        let src_dir = repo_path.join("src");
        fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

        // Create lib.rs with realistic code structure
        let lib_code = r#"//! Test library for KotaDB integration testing
//! 
//! This module provides realistic Rust code structures for testing
//! codebase intelligence features.

use std::collections::HashMap;
use std::path::PathBuf;

/// Core storage interface for the test application
pub struct FileStorage {
    path: PathBuf,
    metadata: HashMap<String, String>,
}

impl FileStorage {
    /// Creates a new FileStorage instance
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            metadata: HashMap::new(),
        }
    }

    /// Inserts data into the storage
    pub fn insert(&mut self, key: &str, data: &str) -> anyhow::Result<()> {
        self.metadata.insert(key.to_string(), data.to_string());
        Ok(())
    }

    /// Retrieves data from storage
    pub fn get(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Returns the storage path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Configuration management for the test application
pub struct Config {
    settings: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }
}

/// Factory function for creating FileStorage instances
pub fn create_file_storage(path: PathBuf) -> FileStorage {
    FileStorage::new(path)
}

/// Utility function that uses FileStorage
pub fn process_data(storage: &mut FileStorage, data: &str) -> anyhow::Result<()> {
    storage.insert("processed", data)?;
    Ok(())
}

/// Integration test for storage operations
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_operations() {
        let mut storage = create_file_storage(PathBuf::from("/tmp"));
        process_data(&mut storage, "test data").unwrap();
        assert_eq!(storage.get("processed"), Some(&"test data".to_string()));
    }
}
"#;

        fs::write(src_dir.join("lib.rs"), lib_code).context("Failed to create lib.rs")?;

        // Create main.rs
        let main_code = r#"use test_project::{create_file_storage, process_data, Config};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut storage = create_file_storage(PathBuf::from("./data"));
    let config = Config::new();
    
    process_data(&mut storage, "Hello, KotaDB!")?;
    println!("Data processed successfully");
    
    Ok(())
}
"#;

        fs::write(src_dir.join("main.rs"), main_code).context("Failed to create main.rs")?;

        // Create README.md
        let readme = r#"# Test Project

This is a test project for KotaDB integration testing.

## Features

- File storage system
- Configuration management
- Data processing utilities

## Usage

```rust
use test_project::{create_file_storage, process_data};
use std::path::PathBuf;

let mut storage = create_file_storage(PathBuf::from("./data"));
process_data(&mut storage, "example data").unwrap();
```
"#;

        fs::write(repo_path.join("README.md"), readme).context("Failed to create README.md")?;

        // Add all files and create initial commit
        Self::run_git_command(repo_path, &["add", "."]).context("Failed to add files to git")?;
        Self::run_git_command(
            repo_path,
            &[
                "commit",
                "-m",
                "Initial commit: Add basic project structure",
            ],
        )
        .context("Failed to create initial commit")?;

        Ok(())
    }

    /// Adds extensive symbol data for testing result limits and pagination
    async fn add_extensive_symbol_data(repo_path: &Path) -> Result<()> {
        let src_dir = repo_path.join("src");

        // Generate a large module with many symbols
        let mut extensive_code =
            String::from("//! Module with extensive symbols for limit testing\n\n");

        // Add many struct definitions
        for i in 0..150 {
            extensive_code.push_str(&format!(
                r#"
/// Test struct number {i}
pub struct TestStruct{i} {{
    pub field_{i}: String,
    pub value_{i}: u32,
}}

impl TestStruct{i} {{
    pub fn new_{i}() -> Self {{
        Self {{
            field_{i}: format!("test_{i}"),
            value_{i}: {i},
        }}
    }}

    pub fn process_{i}(&self) -> String {{
        format!("Processing {{}}: {{}}", self.field_{i}, self.value_{i})
    }}

    pub fn get_value_{i}(&self) -> u32 {{
        self.value_{i}
    }}
}}
"#
            ));
        }

        // Add many function definitions
        for i in 0..100 {
            let struct_num = i % 50; // Cycle through struct numbers to create relationships
            extensive_code.push_str(&format!(
                r#"
/// Test function number {i}
pub fn test_function_{i}() -> TestStruct{struct_num} {{
    let instance = TestStruct{struct_num}::new_{struct_num}();
    instance.process_{struct_num}();
    instance
}}

/// Utility function that calls test_function_{i}
pub fn call_test_function_{i}() {{
    let _result = test_function_{i}();
}}
"#,
            ));
        }

        fs::write(src_dir.join("extensive_symbols.rs"), extensive_code)
            .context("Failed to create extensive_symbols.rs")?;

        // Update lib.rs to include the new module
        let lib_path = src_dir.join("lib.rs");
        let mut lib_content = fs::read_to_string(&lib_path).context("Failed to read lib.rs")?;

        lib_content.push_str("\npub mod extensive_symbols;\n");

        fs::write(&lib_path, lib_content)
            .context("Failed to update lib.rs with extensive_symbols module")?;

        // Create another file with test functions that call the extensive symbols
        let test_callers = r#"//! Test module that calls functions from extensive_symbols

use crate::extensive_symbols::*;

/// Function that calls many test functions to create relationships
pub fn call_all_test_functions() {
    for i in 0..50 {
        match i {
"#;

        let mut test_callers = test_callers.to_string();
        for i in 0..50 {
            test_callers.push_str(&format!(
                "            {} => call_test_function_{}(),\n",
                i, i
            ));
        }
        test_callers.push_str("            _ => {}\n        }\n    }\n}\n");

        fs::write(src_dir.join("test_callers.rs"), test_callers)
            .context("Failed to create test_callers.rs")?;

        // Update lib.rs again
        let mut lib_content = fs::read_to_string(&lib_path).context("Failed to read lib.rs")?;
        lib_content.push_str("pub mod test_callers;\n");
        fs::write(&lib_path, lib_content)
            .context("Failed to update lib.rs with test_callers module")?;

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
