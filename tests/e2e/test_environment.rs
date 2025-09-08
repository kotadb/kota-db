// TestEnvironment: Isolated E2E Test Environment
// Provides clean, isolated environments for end-to-end testing
// Following Stage 6: Component Library patterns with proper resource management

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command as AsyncCommand;

/// Isolated test environment for E2E testing
///
/// Provides:
/// - Isolated temporary directories
/// - Clean database initialization
/// - Resource cleanup on drop
/// - Reproducible test conditions
pub struct TestEnvironment {
    /// Temporary directory for this test environment
    temp_dir: TempDir,
    /// Database directory path
    db_path: PathBuf,
    /// Test codebase directory path
    codebase_path: PathBuf,
    /// Project root path (for accessing kotadb binary)
    project_root: PathBuf,
}

impl TestEnvironment {
    /// Creates a new isolated test environment
    pub fn new() -> Result<Self> {
        let temp_dir =
            TempDir::new().context("Failed to create temporary directory for E2E test")?;

        let db_path = temp_dir.path().join("test-db");
        let codebase_path = temp_dir.path().join("test-codebase");

        // Create database directory
        std::fs::create_dir_all(&db_path).context("Failed to create test database directory")?;

        // Create test codebase directory
        std::fs::create_dir_all(&codebase_path)
            .context("Failed to create test codebase directory")?;

        // Determine project root by looking for Cargo.toml
        let project_root = find_project_root().context("Could not find KotaDB project root")?;

        Ok(Self {
            temp_dir,
            db_path,
            codebase_path,
            project_root,
        })
    }

    /// Get the database path for this test environment
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Get the test codebase path
    pub fn codebase_path(&self) -> &Path {
        &self.codebase_path
    }

    /// Get the temporary directory path
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the project root path
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Creates a realistic test codebase in the environment
    pub fn setup_test_codebase(&self) -> Result<()> {
        crate::e2e::create_test_codebase(&self.codebase_path)
            .context("Failed to create test codebase")
    }

    /// Build the KotaDB binary if needed (ensures we're testing current code)
    pub async fn ensure_binary_built(&self) -> Result<()> {
        let output = AsyncCommand::new("cargo")
            .arg("build")
            .arg("--bin")
            .arg("kotadb")
            .current_dir(&self.project_root)
            .output()
            .await
            .context("Failed to execute cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to build KotaDB binary: {}", stderr);
        }

        Ok(())
    }

    /// Get the path to the KotaDB binary
    pub fn kotadb_binary_path(&self) -> PathBuf {
        self.project_root.join("target/debug/kotadb")
    }

    /// Cleanup any test artifacts (called automatically on drop)
    pub fn cleanup(&self) {
        // TempDir automatically cleans up on drop
        // Additional cleanup can be added here if needed
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Find the KotaDB project root by looking for Cargo.toml
fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("Could not get current directory")?;

    let mut path = current_dir.as_path();

    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            // Verify this is actually the KotaDB project by checking for kotadb binary
            let cargo_content =
                std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

            if cargo_content.contains("kotadb") || cargo_content.contains("kota-db") {
                return Ok(path.to_path_buf());
            }
        }

        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }

    anyhow::bail!("Could not find KotaDB project root (no Cargo.toml with kotadb binary found)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_creation() -> Result<()> {
        let env = TestEnvironment::new()?;

        // Verify paths exist
        assert!(env.db_path().exists());
        assert!(env.codebase_path().exists());
        assert!(env.temp_path().exists());

        // Verify project root is found
        let cargo_toml = env.project_root().join("Cargo.toml");
        assert!(cargo_toml.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_codebase_setup() -> Result<()> {
        let env = TestEnvironment::new()?;
        env.setup_test_codebase()?;

        // Verify test codebase files exist
        assert!(env.codebase_path().join("Cargo.toml").exists());
        assert!(env.codebase_path().join("src/lib.rs").exists());
        assert!(env.codebase_path().join("src/config.rs").exists());

        Ok(())
    }
}
