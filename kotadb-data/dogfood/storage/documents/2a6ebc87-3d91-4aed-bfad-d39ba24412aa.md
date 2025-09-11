---
tags:
- file
- kota-db
- ext_rs
---
// End-to-End Test Framework Module
// Implements the E2E layer of the testing pyramid (5% of total tests)
// Validates complete user journeys and workflow integration

pub mod command_runner;
pub mod test_codebase_analysis_journey;
pub mod test_environment;

use std::path::Path;

pub use command_runner::CommandRunner;
pub use test_environment::TestEnvironment;

/// E2E test result containing both success status and detailed output
#[derive(Debug, Clone)]
pub struct E2ETestResult {
    pub success: bool,
    pub output: String,
    pub stderr: String,
    pub duration_ms: u128,
}

impl E2ETestResult {
    pub fn new(success: bool, output: String, stderr: String, duration_ms: u128) -> Self {
        Self {
            success,
            output,
            stderr,
            duration_ms,
        }
    }
}

/// Creates a minimal test codebase for E2E testing
pub fn create_test_codebase(base_dir: &Path) -> anyhow::Result<()> {
    use std::fs;
    use std::process::Command;

    // Initialize git repository
    let output = Command::new("git")
        .args(["init"])
        .current_dir(base_dir)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to initialize git repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Create a realistic test codebase structure
    let src_dir = base_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create test Rust files with realistic content
    fs::write(
        src_dir.join("lib.rs"),
        r#"//! Test library for E2E testing

use std::collections::HashMap;

pub struct Storage {
    data: HashMap<String, String>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    
    pub async fn insert(&mut self, key: String, value: String) -> Result<(), String> {
        self.data.insert(key, value);
        Ok(())
    }
    
    pub async fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

pub fn process_data(input: &str) -> String {
    format!("processed: {}", input)
}
"#,
    )?;

    fs::write(
        src_dir.join("config.rs"),
        r#"//! Configuration module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub max_connections: u32,
}

impl Config {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            port: 8080,
            max_connections: 100,
        }
    }
    
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}
"#,
    )?;

    // Create Cargo.toml for realistic project structure
    fs::write(
        base_dir.join("Cargo.toml"),
        r#"[package]
name = "test-codebase"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
"#,
    )?;

    // Configure git user (required for commits)
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(base_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(base_dir)
        .output()?;

    // Add and commit all files
    Command::new("git")
        .args(["add", "."])
        .current_dir(base_dir)
        .output()?;

    let output = Command::new("git")
        .args(["commit", "-m", "Initial test commit"])
        .current_dir(base_dir)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to commit files: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
