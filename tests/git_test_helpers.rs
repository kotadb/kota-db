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

    /// Creates a test git repository with comprehensive, diverse content for CLI behavior validation
    ///
    /// This variant creates a realistic codebase structure with various types of Rust code
    /// that exercises different CLI interface behaviors. Used for comprehensive UX testing.
    pub async fn new_with_comprehensive_content() -> Result<Self> {
        let repo = Self::new().await?;

        Self::add_comprehensive_content(repo.temp_dir.path())
            .await
            .context("Failed to add comprehensive content")?;

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

    /// Adds comprehensive, diverse content for CLI behavior testing
    async fn add_comprehensive_content(repo_path: &Path) -> Result<()> {
        // Create diverse Rust code that exercises different CLI behaviors

        // Storage module with complex structures
        let storage_content = r#"//! Storage module for data persistence
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct FileStorage {
    data: HashMap<String, Vec<u8>>,
    metadata: Arc<Mutex<StorageMetadata>>,
}

pub struct StorageMetadata {
    total_files: usize,
    total_bytes: u64,
    last_modified: std::time::SystemTime,
}

impl FileStorage {
    /// Creates a new FileStorage instance
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            metadata: Arc::new(Mutex::new(StorageMetadata {
                total_files: 0,
                total_bytes: 0,
                last_modified: std::time::SystemTime::now(),
            })),
        }
    }

    /// Stores data with the given key
    pub async fn store(&mut self, key: String, value: Vec<u8>) -> Result<(), StorageError> {
        self.data.insert(key, value);
        Ok(())
    }

    /// Retrieves data by key
    pub async fn retrieve(&self, key: &str) -> Option<&Vec<u8>> {
        self.data.get(key)
    }

    /// Deletes data by key
    pub fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

#[derive(Debug)]
pub enum StorageError {
    KeyNotFound,
    PermissionDenied,
    InsufficientSpace,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StorageError::KeyNotFound => write!(f, "Key not found"),
            StorageError::PermissionDenied => write!(f, "Permission denied"),
            StorageError::InsufficientSpace => write!(f, "Insufficient space"),
        }
    }
}
"#;

        fs::write(repo_path.join("storage.rs"), storage_content)?;

        // API module with handlers and routes
        let api_content = r#"//! API handlers and routing
use crate::storage::{FileStorage, StorageError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiRequest {
    pub operation: String,
    pub key: String,
    pub data: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<u8>>,
}

pub struct ApiHandler {
    storage: FileStorage,
}

impl ApiHandler {
    pub fn new() -> Self {
        Self {
            storage: FileStorage::new(),
        }
    }

    pub async fn handle_request(&mut self, request: ApiRequest) -> ApiResponse {
        match request.operation.as_str() {
            "store" => self.handle_store(request).await,
            "retrieve" => self.handle_retrieve(request).await,
            "delete" => self.handle_delete(request).await,
            _ => ApiResponse {
                success: false,
                message: "Unknown operation".to_string(),
                data: None,
            },
        }
    }

    async fn handle_store(&mut self, request: ApiRequest) -> ApiResponse {
        if let Some(data) = request.data {
            match self.storage.store(request.key, data).await {
                Ok(_) => ApiResponse {
                    success: true,
                    message: "Data stored successfully".to_string(),
                    data: None,
                },
                Err(e) => ApiResponse {
                    success: false,
                    message: format!("Storage error: {}", e),
                    data: None,
                },
            }
        } else {
            ApiResponse {
                success: false,
                message: "No data provided".to_string(),
                data: None,
            }
        }
    }

    async fn handle_retrieve(&self, request: ApiRequest) -> ApiResponse {
        if let Some(data) = self.storage.retrieve(&request.key).await {
            ApiResponse {
                success: true,
                message: "Data retrieved successfully".to_string(),
                data: Some(data.clone()),
            }
        } else {
            ApiResponse {
                success: false,
                message: "Key not found".to_string(),
                data: None,
            }
        }
    }

    async fn handle_delete(&mut self, request: ApiRequest) -> ApiResponse {
        if self.storage.delete(&request.key) {
            ApiResponse {
                success: true,
                message: "Data deleted successfully".to_string(),
                data: None,
            }
        } else {
            ApiResponse {
                success: false,
                message: "Key not found".to_string(),
                data: None,
            }
        }
    }
}

pub fn create_api_routes() -> Vec<String> {
    vec![
        "/api/store".to_string(),
        "/api/retrieve".to_string(),
        "/api/delete".to_string(),
        "/api/health".to_string(),
    ]
}
"#;

        fs::write(repo_path.join("api.rs"), api_content)?;

        // Utils module with helper functions
        let utils_content = r#"//! Utility functions and helpers
use std::time::{Duration, SystemTime};

pub const MAX_FILE_SIZE: usize = 1024 * 1024; // 1MB
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Validates a key for storage operations
pub fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("Key cannot be empty".to_string());
    }
    if key.len() > 255 {
        return Err("Key too long".to_string());
    }
    if key.contains('/') || key.contains('\\') {
        return Err("Key cannot contain path separators".to_string());
    }
    Ok(())
}

/// Formats file size in human-readable format
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Calculates elapsed time since a timestamp
pub fn elapsed_since(start: SystemTime) -> Duration {
    SystemTime::now().duration_since(start).unwrap_or(Duration::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key() {
        assert!(validate_key("valid_key").is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key("a".repeat(256).as_str()).is_err());
        assert!(validate_key("invalid/key").is_err());
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0.00 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
    }
}
"#;

        fs::write(repo_path.join("utils.rs"), utils_content)?;

        // Update lib.rs to include all new modules
        let lib_content = r#"//! Comprehensive test codebase for CLI behavior validation
pub mod test;
pub mod storage;
pub mod api;  
pub mod utils;

pub use storage::{FileStorage, StorageError};
pub use api::{ApiHandler, ApiRequest, ApiResponse};
pub use utils::{validate_key, format_file_size};

/// Main application entry point
pub fn run_application() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting comprehensive test application");
    
    let mut handler = ApiHandler::new();
    println!("API handler initialized");
    
    Ok(())
}

/// Configuration structure
#[derive(Debug, Clone)]
pub struct Config {
    pub max_connections: usize,
    pub timeout: std::time::Duration,
    pub storage_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_connections: 100,
            timeout: utils::DEFAULT_TIMEOUT,
            storage_path: "./data".to_string(),
        }
    }
}
"#;

        fs::write(repo_path.join("lib.rs"), lib_content)?;

        // Add and commit all the new files
        Self::run_git_command(repo_path, &["add", "."])
            .context("Failed to add comprehensive files to git")?;
        Self::run_git_command(
            repo_path,
            &["commit", "-m", "Add comprehensive test content"],
        )
        .context("Failed to commit comprehensive content")?;

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
/// index-codebase command on it, returning the database path and TempDir for use in tests.
pub async fn create_indexed_test_database(
    git_repo: &TestGitRepository,
) -> Result<(String, TempDir)> {
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

    // Return both the database path and the TempDir to keep it alive
    // We return the TempDir itself to ensure it doesn't get dropped
    Ok((db_path_str, db_temp_dir))
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
        let (db_path, _temp_dir) = create_indexed_test_database(&repo).await?;

        // Verify database directory was created
        assert!(
            Path::new(&db_path).exists(),
            "Database directory should exist at path: {}",
            db_path
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_comprehensive_content_repository() -> Result<()> {
        let repo = TestGitRepository::new_with_comprehensive_content().await?;

        // Verify comprehensive modules were created
        let storage_file = repo.path_ref().join("storage.rs");
        let api_file = repo.path_ref().join("api.rs");
        let utils_file = repo.path_ref().join("utils.rs");

        assert!(storage_file.exists(), "storage.rs should exist");
        assert!(api_file.exists(), "api.rs should exist");
        assert!(utils_file.exists(), "utils.rs should exist");

        // Verify content includes diverse structures and functions
        let storage_content = fs::read_to_string(&storage_file)?;
        assert!(
            storage_content.contains("FileStorage") && storage_content.contains("async fn store"),
            "Storage module should contain FileStorage with async methods"
        );

        let api_content = fs::read_to_string(&api_file)?;
        assert!(
            api_content.contains("ApiHandler") && api_content.contains("handle_request"),
            "API module should contain ApiHandler with request handling"
        );

        let utils_content = fs::read_to_string(&utils_file)?;
        assert!(
            utils_content.contains("validate_key") && utils_content.contains("format_file_size"),
            "Utils module should contain helper functions"
        );

        Ok(())
    }
}
