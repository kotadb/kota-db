//! Git-specific types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata about a git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// Repository name
    pub name: String,
    /// Repository URL (if remote)
    pub url: Option<String>,
    /// Local path to the repository
    pub path: PathBuf,
    /// Default branch name
    pub default_branch: String,
    /// Total number of commits
    pub commit_count: usize,
    /// Repository creation timestamp
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Information about a single file in the repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File path relative to repository root
    pub path: String,
    /// File content as bytes
    pub content: Vec<u8>,
    /// File size in bytes
    pub size: usize,
    /// Whether this is a binary file
    pub is_binary: bool,
    /// File extension (if any)
    pub extension: Option<String>,
    /// MIME type (if detected)
    pub mime_type: Option<String>,
    /// Last commit SHA that modified this file
    pub last_commit: String,
    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
}

/// Information about a git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit SHA
    pub sha: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent commit SHAs
    pub parents: Vec<String>,
    /// Files changed in this commit
    pub files_changed: Vec<String>,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

/// Configuration for repository ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionOptions {
    /// Include file contents in documents
    pub include_file_contents: bool,
    /// Include commit history
    pub include_commit_history: bool,
    /// Extract symbols from source code files during ingestion
    pub extract_symbols: bool,
    /// Maximum file size to ingest (in bytes)
    pub max_file_size: usize,
    /// File extensions to include (empty = all)
    pub include_extensions: Vec<String>,
    /// File extensions to exclude
    pub exclude_extensions: Vec<String>,
    /// Paths to exclude (gitignore patterns)
    pub exclude_patterns: Vec<String>,
    /// Branch to ingest (None = current branch)
    pub branch: Option<String>,
    /// Maximum depth for commit history (None = unlimited)
    pub max_history_depth: Option<usize>,
    /// Memory limits configuration for ingestion process
    pub memory_limits: Option<crate::memory::MemoryLimitsConfig>,
}

impl Default for IngestionOptions {
    fn default() -> Self {
        Self {
            include_file_contents: true,
            include_commit_history: true,
            extract_symbols: true, // Enable symbol extraction by default
            max_file_size: 10 * 1024 * 1024, // 10MB
            include_extensions: vec![],
            exclude_extensions: vec![
                "bin".to_string(),
                "exe".to_string(),
                "dll".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "pdf".to_string(),
                "zip".to_string(),
                "tar".to_string(),
                "gz".to_string(),
            ],
            exclude_patterns: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".DS_Store".to_string(),
            ],
            branch: None,
            max_history_depth: Some(1000),
            memory_limits: None, // Default to no memory limits for backward compatibility
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ingestion_options() {
        let options = IngestionOptions::default();
        assert!(options.include_file_contents);
        assert!(options.include_commit_history);
        assert!(options.extract_symbols);
        assert_eq!(options.max_file_size, 10 * 1024 * 1024);
        assert!(!options.exclude_extensions.is_empty());
    }

    #[test]
    fn test_repository_metadata_serialization() {
        let metadata = RepositoryMetadata {
            name: "test-repo".to_string(),
            url: Some("https://github.com/test/repo".to_string()),
            path: PathBuf::from("/tmp/test-repo"),
            default_branch: "main".to_string(),
            commit_count: 42,
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: RepositoryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, metadata.name);
        assert_eq!(deserialized.commit_count, metadata.commit_count);
    }
}
