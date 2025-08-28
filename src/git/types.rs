//! Git-specific types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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

/// Memory management configuration for ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum memory usage in bytes (None = unlimited)
    pub max_memory_bytes: Option<usize>,
    /// Maximum files to process in a single batch
    pub max_files_per_batch: usize,
    /// Maximum relationships to keep in memory
    pub max_relationships_per_document: usize,
    /// Maximum total relationships in the dependency graph
    pub max_total_relationships: usize,
    /// Memory pressure check interval during processing
    pub memory_check_interval_ms: u64,
    /// Enable memory pressure detection
    pub enable_memory_pressure_detection: bool,
    /// Enable graceful degradation when approaching limits
    pub enable_graceful_degradation: bool,
    /// Memory usage threshold (0.0-1.0) to trigger warnings
    pub memory_warning_threshold: f64,
    /// Memory usage threshold (0.0-1.0) to trigger backpressure
    pub memory_backpressure_threshold: f64,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024), // 512MB default
            max_files_per_batch: 1000,
            max_relationships_per_document: 10_000,
            max_total_relationships: 100_000,
            memory_check_interval_ms: 1000, // Check every second
            enable_memory_pressure_detection: true,
            enable_graceful_degradation: true,
            memory_warning_threshold: 0.7,       // Warn at 70%
            memory_backpressure_threshold: 0.85, // Backpressure at 85%
        }
    }
}

/// Current memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageInfo {
    /// Current memory usage in bytes
    pub current_usage_bytes: usize,
    /// Peak memory usage during this session
    pub peak_usage_bytes: usize,
    /// Number of documents currently in memory
    pub documents_in_memory: usize,
    /// Number of relationships currently in memory
    pub relationships_in_memory: usize,
    /// Memory usage as percentage of limit (0.0-1.0)
    pub usage_percentage: f64,
    /// Whether memory pressure is detected
    pub memory_pressure_detected: bool,
    /// Timestamp of this measurement
    pub timestamp: DateTime<Utc>,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    /// Memory usage is normal
    Normal,
    /// Memory usage is elevated but manageable
    Warning,
    /// Memory usage is high, backpressure should be applied
    High,
    /// Memory usage is critical, immediate action required
    Critical,
}

/// Progress information including memory usage
#[derive(Debug, Clone)]
pub struct IngestionProgress {
    /// Files processed so far
    pub files_processed: usize,
    /// Total files discovered
    pub total_files: usize,
    /// Symbols extracted so far
    pub symbols_extracted: usize,
    /// Relationships found so far
    pub relationships_found: usize,
    /// Current memory usage information
    pub memory_usage: MemoryUsageInfo,
    /// Estimated time remaining
    pub estimated_remaining: Option<Duration>,
    /// Current processing phase
    pub current_phase: String,
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
    /// Memory management configuration
    pub memory_limits: MemoryLimits,
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
            memory_limits: MemoryLimits::default(),
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
