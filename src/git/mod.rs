//! Git repository integration for KotaDB
//!
//! This module provides functionality for ingesting git repositories into KotaDB,
//! enabling codebase analysis and intelligence features.

mod ingestion;
mod repository;
pub mod types;

pub use ingestion::{IngestResult, IngestionConfig, RepositoryIngester};
pub use repository::GitRepository;
pub use types::{CommitInfo, FileEntry, IngestionOptions, RepositoryMetadata};

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_git_module_imports() -> Result<()> {
        // Basic test to ensure module structure is correct
        let _temp = TempDir::new()?;
        Ok(())
    }
}
