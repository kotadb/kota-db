---
tags:
- file
- kota-db
- ext_rs
---
//! Git metadata extensions for KotaDB documents
//!
//! This module provides git-specific metadata that can be attached to documents
//! to enable repository file organization and history tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Git-specific metadata for a document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitMetadata {
    /// Repository name this document belongs to
    pub repository_name: String,
    /// Full repository path on the filesystem
    pub repository_path: String,
    /// Commit hash where this version of the file was captured
    pub commit_hash: String,
    /// Short commit hash (first 8 characters)
    pub commit_hash_short: String,
    /// Branch name where this commit exists
    pub branch: String,
    /// Commit author name
    pub author_name: String,
    /// Commit author email
    pub author_email: String,
    /// Commit timestamp
    pub commit_timestamp: DateTime<Utc>,
    /// Commit message
    pub commit_message: String,
    /// File path within the repository
    pub file_path: String,
    /// File size in bytes at this commit
    pub file_size: usize,
    /// Whether this is a binary file
    pub is_binary: bool,
    /// File extension (if any)
    pub file_extension: Option<String>,
    /// Files changed in the same commit (for context)
    pub files_changed_in_commit: Vec<String>,
    /// Number of lines added in this commit (for this file)
    pub lines_added: Option<usize>,
    /// Number of lines deleted in this commit (for this file)
    pub lines_deleted: Option<usize>,
}

impl GitMetadata {
    /// Create new git metadata
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository_name: String,
        repository_path: String,
        commit_hash: String,
        branch: String,
        author_name: String,
        author_email: String,
        commit_timestamp: DateTime<Utc>,
        commit_message: String,
        file_path: String,
        file_size: usize,
        is_binary: bool,
    ) -> Self {
        let commit_hash_short = commit_hash.chars().take(8).collect();
        let file_extension = std::path::Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        Self {
            repository_name,
            repository_path,
            commit_hash,
            commit_hash_short,
            branch,
            author_name,
            author_email,
            commit_timestamp,
            commit_message,
            file_path,
            file_size,
            is_binary,
            file_extension,
            files_changed_in_commit: Vec::new(),
            lines_added: None,
            lines_deleted: None,
        }
    }

    /// Get a unique identifier for this file version
    pub fn file_version_id(&self) -> String {
        format!("{}:{}", self.file_path, self.commit_hash_short)
    }

    /// Get a human-readable file description
    pub fn file_description(&self) -> String {
        format!(
            "{} ({}@{})",
            self.file_path, self.repository_name, self.commit_hash_short
        )
    }

    /// Check if this file was recently modified (within the last N commits)
    pub fn is_recently_modified(&self, recent_threshold: chrono::Duration) -> bool {
        let now = Utc::now();
        now.signed_duration_since(self.commit_timestamp) <= recent_threshold
    }
}

/// Extended document type that includes git metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDocument {
    /// Base document
    pub document: crate::Document,
    /// Git-specific metadata
    pub git_metadata: GitMetadata,
}

impl GitDocument {
    /// Create a new git document
    pub fn new(document: crate::Document, git_metadata: GitMetadata) -> Self {
        Self {
            document,
            git_metadata,
        }
    }

    /// Get the document ID
    pub fn id(&self) -> crate::ValidatedDocumentId {
        self.document.id
    }

    /// Get the file path within the repository
    pub fn repo_file_path(&self) -> &str {
        &self.git_metadata.file_path
    }

    /// Get the repository name
    pub fn repository_name(&self) -> &str {
        &self.git_metadata.repository_name
    }

    /// Get the commit hash
    pub fn commit_hash(&self) -> &str {
        &self.git_metadata.commit_hash
    }

    /// Check if this document represents a binary file
    pub fn is_binary(&self) -> bool {
        self.git_metadata.is_binary
    }
}

/// Repository organization configuration
#[derive(Debug, Clone)]
pub struct RepositoryOrganizationConfig {
    /// Base data directory for repository analysis
    pub base_data_dir: String,
    /// Whether to create separate directories per repository
    pub separate_repo_directories: bool,
    /// Whether to include commit hash in document paths
    pub include_commit_in_path: bool,
    /// Whether to track file history across commits
    pub track_file_history: bool,
    /// Maximum number of file versions to keep per file
    pub max_file_versions: Option<usize>,
    /// Maximum total memory usage for file history cache (in MB)
    pub max_cache_memory_mb: Option<usize>,
    /// Maximum number of files to track in history cache
    pub max_tracked_files: Option<usize>,
}

impl Default for RepositoryOrganizationConfig {
    fn default() -> Self {
        Self {
            base_data_dir: "data/analysis".to_string(),
            separate_repo_directories: true,
            include_commit_in_path: false,
            track_file_history: true,
            max_file_versions: Some(10), // Keep last 10 versions by default
            max_cache_memory_mb: Some(100), // 100MB default cache limit
            max_tracked_files: Some(10000), // Track up to 10k files by default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_git_metadata_creation() {
        let metadata = GitMetadata::new(
            "kota-db".to_string(),
            "/path/to/kota-db".to_string(),
            "abc123def456".to_string(),
            "main".to_string(),
            "Test Author".to_string(),
            "test@example.com".to_string(),
            Utc::now(),
            "feat: add new feature".to_string(),
            "src/main.rs".to_string(),
            1024,
            false,
        );

        assert_eq!(metadata.repository_name, "kota-db");
        assert_eq!(metadata.commit_hash_short, "abc123de");
        assert_eq!(metadata.file_extension, Some("rs".to_string()));
        assert!(!metadata.is_binary);
    }

    #[test]
    fn test_file_version_id() {
        let metadata = GitMetadata::new(
            "repo".to_string(),
            "/path".to_string(),
            "abcdef123456".to_string(),
            "main".to_string(),
            "Author".to_string(),
            "author@example.com".to_string(),
            Utc::now(),
            "commit".to_string(),
            "file.txt".to_string(),
            100,
            false,
        );

        assert_eq!(metadata.file_version_id(), "file.txt:abcdef12");
    }

    #[test]
    fn test_file_extension_extraction() {
        let metadata = GitMetadata::new(
            "repo".to_string(),
            "/path".to_string(),
            "abc123".to_string(),
            "main".to_string(),
            "Author".to_string(),
            "author@example.com".to_string(),
            Utc::now(),
            "commit".to_string(),
            "file.TXT".to_string(),
            100,
            false,
        );

        assert_eq!(metadata.file_extension, Some("txt".to_string()));
    }

    #[test]
    fn test_no_file_extension() {
        let metadata = GitMetadata::new(
            "repo".to_string(),
            "/path".to_string(),
            "abc123".to_string(),
            "main".to_string(),
            "Author".to_string(),
            "author@example.com".to_string(),
            Utc::now(),
            "commit".to_string(),
            "README".to_string(),
            100,
            false,
        );

        assert_eq!(metadata.file_extension, None);
    }
}
