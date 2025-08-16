//! Git repository ingestion into KotaDB

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, instrument, warn};

use crate::builders::DocumentBuilder;
use crate::git::repository::GitRepository;
use crate::git::types::{CommitInfo, FileEntry, IngestionOptions};
use crate::Document;
use crate::Storage;

/// Configuration for repository ingestion
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Prefix for document paths in KotaDB
    pub path_prefix: String,
    /// Repository-specific options
    pub options: IngestionOptions,
    /// Whether to create an index document for the repository
    pub create_index: bool,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            path_prefix: "repos".to_string(),
            options: IngestionOptions::default(),
            create_index: true,
        }
    }
}

/// Ingests git repositories into KotaDB
pub struct RepositoryIngester {
    config: IngestionConfig,
}

impl RepositoryIngester {
    /// Create a new repository ingester
    pub fn new(config: IngestionConfig) -> Self {
        Self { config }
    }

    /// Sanitize repository name for safe filesystem usage
    fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_lowercase()
    }

    /// Ingest a git repository into KotaDB storage
    #[instrument(skip(self, storage, repo_path))]
    pub async fn ingest<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
    ) -> Result<IngestResult> {
        let repo_path = repo_path.as_ref();
        info!("Starting repository ingestion from: {:?}", repo_path);

        // Open the repository
        let repo = GitRepository::open(repo_path, self.config.options.clone())
            .context("Failed to open git repository")?;

        // Get repository metadata
        let metadata = repo
            .metadata()
            .context("Failed to get repository metadata")?;

        let safe_repo_name = Self::sanitize_name(&metadata.name);
        info!(
            "Repository: {} ({} commits) - using safe name: {}",
            metadata.name, metadata.commit_count, safe_repo_name
        );

        let mut result = IngestResult::default();

        // Create repository index document if requested
        if self.config.create_index {
            let index_doc = self.create_index_document(&metadata, &safe_repo_name)?;
            storage
                .insert(index_doc)
                .await
                .context("Failed to insert repository index document")?;
            result.documents_created += 1;
        }

        // Ingest files
        if self.config.options.include_file_contents {
            let files = repo
                .list_files()
                .context("Failed to list repository files")?;

            info!("Found {} files to ingest", files.len());

            for file in files {
                match self.create_file_document(&safe_repo_name, &file) {
                    Ok(doc) => {
                        if let Err(e) = storage.insert(doc).await {
                            warn!("Failed to insert file document {}: {}", file.path, e);
                            result.errors += 1;
                        } else {
                            result.documents_created += 1;
                            result.files_ingested += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create document for {}: {}", file.path, e);
                        result.errors += 1;
                    }
                }
            }
        }

        // Ingest commit history
        if self.config.options.include_commit_history {
            let commits = repo
                .get_commits(None)
                .context("Failed to get repository commits")?;

            info!("Processing {} commits", commits.len());

            for commit in commits {
                match self.create_commit_document(&safe_repo_name, &commit) {
                    Ok(doc) => {
                        if let Err(e) = storage.insert(doc).await {
                            warn!("Failed to insert commit document {}: {}", commit.sha, e);
                            result.errors += 1;
                        } else {
                            result.documents_created += 1;
                            result.commits_ingested += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create document for commit {}: {}", commit.sha, e);
                        result.errors += 1;
                    }
                }
            }
        }

        info!(
            "Ingestion complete: {} documents created ({} files, {} commits), {} errors",
            result.documents_created, result.files_ingested, result.commits_ingested, result.errors
        );

        Ok(result)
    }

    fn create_index_document(
        &self,
        metadata: &crate::git::types::RepositoryMetadata,
        safe_name: &str,
    ) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let path = format!("{}/{}/index.md", prefix, safe_name);

        let content = format!(
            "# Repository: {}\n\n\
            - **Path**: {}\n\
            - **Branch**: {}\n\
            - **Commits**: {}\n\
            - **Updated**: {}\n",
            metadata.name,
            metadata.path.display(),
            metadata.default_branch,
            metadata.commit_count,
            metadata.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(url) = &metadata.url {
            let content = format!("{}\n- **URL**: {}\n", content, url);
        }

        let mut builder = DocumentBuilder::new()
            .path(&path)?
            .title(format!("Repository: {}", metadata.name))?;
        builder = builder.content(content.as_bytes());
        builder = builder.tag("repository")?;
        builder = builder.tag("index")?;
        builder = builder.tag(&metadata.name)?;
        builder.build()
    }

    fn create_file_document(&self, repo_name: &str, file: &FileEntry) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let doc_path = format!("{}/{}/files/{}", prefix, repo_name, file.path);

        let title = format!("File: {}", file.path);

        let mut builder = DocumentBuilder::new().path(&doc_path)?.title(&title)?;

        // Add content based on whether it's binary
        if file.is_binary {
            let metadata = format!(
                "# Binary File: {}\n\n\
                - **Size**: {} bytes\n\
                - **Type**: Binary\n",
                file.path, file.size
            );
            builder = builder.content(metadata.as_bytes());
        } else {
            builder = builder.content(file.content.clone());
        }

        // Add tags
        builder = builder.tag("file")?;
        builder = builder.tag(repo_name)?;

        if let Some(ext) = &file.extension {
            builder = builder.tag(&format!("ext:{}", ext))?;
        }

        builder.build()
    }

    fn create_commit_document(&self, repo_name: &str, commit: &CommitInfo) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let doc_path = format!(
            "{}/{}/commits/{}.md",
            prefix,
            repo_name,
            &commit.sha[..8] // Use short SHA for path
        );

        let content = format!(
            "# Commit: {}\n\n\
            **Author**: {} <{}>\n\
            **Date**: {}\n\n\
            ## Message\n\
            {}\n\n\
            ## Details\n\
            - **SHA**: {}\n\
            - **Parents**: {}\n\
            - **Changes**: {} insertions, {} deletions\n",
            &commit.sha[..8],
            commit.author_name,
            commit.author_email,
            commit.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            commit.message,
            commit.sha,
            if commit.parents.is_empty() {
                "none".to_string()
            } else {
                commit.parents.join(", ")
            },
            commit.insertions,
            commit.deletions
        );

        let mut builder = DocumentBuilder::new()
            .path(&doc_path)?
            .title(format!("Commit: {}", &commit.sha[..8]))?;
        builder = builder.content(content.as_bytes());
        builder = builder.tag("commit")?;
        builder = builder.tag(repo_name)?;
        builder = builder.tag(&commit.author_name)?;
        builder.build()
    }
}

/// Result of repository ingestion
#[derive(Debug, Default)]
pub struct IngestResult {
    /// Number of documents created
    pub documents_created: usize,
    /// Number of files ingested
    pub files_ingested: usize,
    /// Number of commits ingested
    pub commits_ingested: usize,
    /// Number of errors encountered
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_file_storage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ingester_creation() -> Result<()> {
        let config = IngestionConfig::default();
        let _ingester = RepositoryIngester::new(config);
        Ok(())
    }

    #[test]
    fn test_repository_name_sanitization() {
        assert_eq!(RepositoryIngester::sanitize_name("my-repo"), "my-repo");
        assert_eq!(RepositoryIngester::sanitize_name("My_Repo"), "my_repo");
        assert_eq!(RepositoryIngester::sanitize_name("repo.name"), "repo.name");
        assert_eq!(
            RepositoryIngester::sanitize_name("repo/with/slashes"),
            "repo-with-slashes"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("repo with spaces"),
            "repo-with-spaces"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("UPPERCASE-REPO"),
            "uppercase-repo"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("@special#chars$"),
            "special-chars"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("--leading-trailing--"),
            "leading-trailing"
        );
    }

    #[tokio::test]
    async fn test_ingest_nonexistent_repo() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;

        let config = IngestionConfig::default();
        let ingester = RepositoryIngester::new(config);

        let result = ingester.ingest("/nonexistent/path", &mut storage).await;
        assert!(result.is_err());

        Ok(())
    }
}
