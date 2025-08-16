//! Repository file organization system for KotaDB
//!
//! This module handles the organization of repository files, manages file lifecycle
//! operations (moves, renames, deletions), and tracks file history across commits.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::git::document_metadata::{GitMetadata, RepositoryOrganizationConfig};
use crate::git::types::{CommitInfo, FileEntry};
use crate::{Storage, ValidatedDocumentId, ValidatedPath};

/// Manages repository file organization and lifecycle
pub struct FileOrganizationManager {
    config: RepositoryOrganizationConfig,
    /// Cache of file path -> document ID mappings for quick lookups
    file_path_cache: HashMap<String, ValidatedDocumentId>,
    /// Cache of file histories: file_path -> list of (commit_hash, doc_id)
    file_history_cache: HashMap<String, Vec<(String, ValidatedDocumentId)>>,
}

impl FileOrganizationManager {
    /// Create a new file organization manager
    pub fn new(config: RepositoryOrganizationConfig) -> Self {
        Self {
            config,
            file_path_cache: HashMap::new(),
            file_history_cache: HashMap::new(),
        }
    }

    /// Create a KotaDB document path for a repository file
    pub fn create_document_path(
        &self,
        repo_name: &str,
        file_path: &str,
        commit_hash: Option<&str>,
    ) -> Result<String> {
        let base_path = if self.config.separate_repo_directories {
            format!("{}/{}", self.config.base_data_dir, repo_name)
        } else {
            self.config.base_data_dir.clone()
        };

        let file_path_normalized = file_path.trim_start_matches('/');

        let doc_path = if self.config.include_commit_in_path {
            if let Some(hash) = commit_hash {
                let short_hash = &hash[..8.min(hash.len())];
                format!(
                    "{}/files/{}/{}",
                    base_path, short_hash, file_path_normalized
                )
            } else {
                format!("{}/files/{}", base_path, file_path_normalized)
            }
        } else {
            format!("{}/files/{}", base_path, file_path_normalized)
        };

        Ok(doc_path)
    }

    /// Handle file creation in repository
    pub async fn handle_file_creation<S: Storage + ?Sized>(
        &mut self,
        storage: &mut S,
        file_entry: &FileEntry,
        commit_info: &CommitInfo,
        repo_name: &str,
        repo_path: &str,
    ) -> Result<ValidatedDocumentId> {
        debug!(
            "Creating file document for {} in repository {}",
            file_entry.path, repo_name
        );

        // Create git metadata
        let git_metadata = GitMetadata::new(
            repo_name.to_string(),
            repo_path.to_string(),
            commit_info.sha.clone(),
            "main".to_string(), // TODO: Get actual branch from commit info
            commit_info.author_name.clone(),
            commit_info.author_email.clone(),
            commit_info.timestamp,
            commit_info.message.clone(),
            file_entry.path.clone(),
            file_entry.size,
            file_entry.is_binary,
        );

        // Create document path
        let doc_path = self.create_document_path(
            repo_name,
            &file_entry.path,
            if self.config.include_commit_in_path {
                Some(&commit_info.sha)
            } else {
                None
            },
        )?;

        // Build the document
        let validated_path = ValidatedPath::new(&doc_path)?;
        let title = if file_entry.is_binary {
            format!("Binary File: {}", file_entry.path)
        } else {
            format!("File: {}", file_entry.path)
        };

        let mut doc_builder = crate::DocumentBuilder::new()
            .path(&doc_path)?
            .title(&title)?;

        // Set content based on file type
        if file_entry.is_binary {
            let metadata_content = format!(
                "# Binary File: {}\n\n\
                - **Repository**: {}\n\
                - **Commit**: {} ({})\n\
                - **Size**: {} bytes\n\
                - **Author**: {} <{}>\n\
                - **Date**: {}\n",
                file_entry.path,
                repo_name,
                &commit_info.sha[..8],
                commit_info.sha,
                file_entry.size,
                commit_info.author_name,
                commit_info.author_email,
                commit_info.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            );
            doc_builder = doc_builder.content(metadata_content.as_bytes());
        } else {
            doc_builder = doc_builder.content(file_entry.content.clone());
        }

        // Add tags
        doc_builder = doc_builder.tag("git-file")?;
        doc_builder = doc_builder.tag(&format!("repo:{}", repo_name))?;
        doc_builder = doc_builder.tag(&format!("commit:{}", &commit_info.sha[..8]))?;

        if let Some(ref ext) = file_entry.extension {
            doc_builder = doc_builder.tag(&format!("ext:{}", ext))?;
        }

        if file_entry.is_binary {
            doc_builder = doc_builder.tag("binary")?;
        } else {
            doc_builder = doc_builder.tag("text")?;
        }

        let document = doc_builder.build()?;
        let doc_id = document.id;

        // Store the document
        storage.insert(document).await?;

        // Update caches
        self.file_path_cache.insert(file_entry.path.clone(), doc_id);

        // Update file history if tracking is enabled
        if self.config.track_file_history {
            let history = self
                .file_history_cache
                .entry(file_entry.path.clone())
                .or_default();

            history.push((commit_info.sha.clone(), doc_id));

            // Limit history size if configured
            if let Some(max_versions) = self.config.max_file_versions {
                if history.len() > max_versions {
                    // Remove oldest versions (keep most recent)
                    history.drain(0..history.len() - max_versions);
                }
            }
        }

        info!(
            "Created document {} for file {} in repository {}",
            doc_id, file_entry.path, repo_name
        );

        Ok(doc_id)
    }

    /// Handle file modification in repository
    pub async fn handle_file_modification<S: Storage + ?Sized>(
        &mut self,
        storage: &mut S,
        file_entry: &FileEntry,
        commit_info: &CommitInfo,
        repo_name: &str,
        repo_path: &str,
    ) -> Result<ValidatedDocumentId> {
        debug!(
            "Handling file modification for {} in repository {}",
            file_entry.path, repo_name
        );

        // For now, treat modifications the same as creation
        // In a more sophisticated implementation, we might:
        // 1. Update existing document if not tracking history
        // 2. Create new version if tracking history
        // 3. Link versions together

        self.handle_file_creation(storage, file_entry, commit_info, repo_name, repo_path)
            .await
    }

    /// Handle file deletion in repository
    pub async fn handle_file_deletion<S: Storage + ?Sized>(
        &mut self,
        storage: &mut S,
        file_path: &str,
        commit_info: &CommitInfo,
        repo_name: &str,
    ) -> Result<bool> {
        debug!(
            "Handling file deletion for {} in repository {}",
            file_path, repo_name
        );

        // Find the document ID for this file
        if let Some(doc_id) = self.file_path_cache.get(file_path) {
            // Delete the document
            let deleted = storage.delete(doc_id).await?;

            if deleted {
                // Remove from cache
                self.file_path_cache.remove(file_path);

                // Create a deletion record if tracking history
                if self.config.track_file_history {
                    let deletion_doc_path = self.create_document_path(
                        repo_name,
                        &format!("{}.deleted", file_path),
                        Some(&commit_info.sha),
                    )?;

                    let deletion_content = format!(
                        "# File Deleted: {}\n\n\
                        - **Repository**: {}\n\
                        - **Deleted in commit**: {} ({})\n\
                        - **Author**: {} <{}>\n\
                        - **Date**: {}\n\
                        - **Message**: {}\n",
                        file_path,
                        repo_name,
                        &commit_info.sha[..8],
                        commit_info.sha,
                        commit_info.author_name,
                        commit_info.author_email,
                        commit_info.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        commit_info.message
                    );

                    let deletion_doc = crate::DocumentBuilder::new()
                        .path(&deletion_doc_path)?
                        .title(format!("Deleted: {}", file_path))?
                        .content(deletion_content.as_bytes())
                        .tag("git-deletion")?
                        .tag(&format!("repo:{}", repo_name))?
                        .tag(&format!("commit:{}", &commit_info.sha[..8]))?
                        .build()?;

                    storage.insert(deletion_doc).await?;
                }

                info!(
                    "Deleted document for file {} in repository {}",
                    file_path, repo_name
                );
                Ok(true)
            } else {
                warn!(
                    "Failed to delete document for file {} in repository {}",
                    file_path, repo_name
                );
                Ok(false)
            }
        } else {
            debug!(
                "File {} not found in cache for repository {}",
                file_path, repo_name
            );
            Ok(false)
        }
    }

    /// Handle file rename/move in repository
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_file_rename<S: Storage + ?Sized>(
        &mut self,
        storage: &mut S,
        old_path: &str,
        new_path: &str,
        file_entry: &FileEntry,
        commit_info: &CommitInfo,
        repo_name: &str,
        repo_path: &str,
    ) -> Result<ValidatedDocumentId> {
        debug!(
            "Handling file rename from {} to {} in repository {}",
            old_path, new_path, repo_name
        );

        // If we have the old file, mark it as moved
        if let Some(old_doc_id) = self.file_path_cache.get(old_path) {
            // Create a move record
            let move_doc_path = self.create_document_path(
                repo_name,
                &format!("{}.moved", old_path),
                Some(&commit_info.sha),
            )?;

            let move_content = format!(
                "# File Moved: {} → {}\n\n\
                - **Repository**: {}\n\
                - **Moved in commit**: {} ({})\n\
                - **Author**: {} <{}>\n\
                - **Date**: {}\n\
                - **Message**: {}\n\
                - **New location**: {}\n",
                old_path,
                new_path,
                repo_name,
                &commit_info.sha[..8],
                commit_info.sha,
                commit_info.author_name,
                commit_info.author_email,
                commit_info.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                commit_info.message,
                new_path
            );

            let move_doc = crate::DocumentBuilder::new()
                .path(&move_doc_path)?
                .title(format!("Moved: {} → {}", old_path, new_path))?
                .content(move_content.as_bytes())
                .tag("git-move")?
                .tag(&format!("repo:{}", repo_name))?
                .tag(&format!("commit:{}", &commit_info.sha[..8]))?
                .build()?;

            storage.insert(move_doc).await?;

            // Remove old path from cache
            self.file_path_cache.remove(old_path);
        }

        // Create new document at new location
        self.handle_file_creation(storage, file_entry, commit_info, repo_name, repo_path)
            .await
    }

    /// Get file history for a given file path
    pub fn get_file_history(&self, file_path: &str) -> Option<&Vec<(String, ValidatedDocumentId)>> {
        self.file_history_cache.get(file_path)
    }

    /// Get document ID for a file path
    pub fn get_document_id(&self, file_path: &str) -> Option<ValidatedDocumentId> {
        self.file_path_cache.get(file_path).copied()
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.file_path_cache.clear();
        self.file_history_cache.clear();
    }

    /// Get statistics about managed files
    pub fn get_statistics(&self) -> FileOrganizationStats {
        let total_files = self.file_path_cache.len();
        let total_versions = self
            .file_history_cache
            .values()
            .map(|history| history.len())
            .sum();

        FileOrganizationStats {
            total_files,
            total_versions,
            files_with_history: self.file_history_cache.len(),
        }
    }
}

/// Statistics about file organization
#[derive(Debug, Clone)]
pub struct FileOrganizationStats {
    /// Total number of unique files being tracked
    pub total_files: usize,
    /// Total number of file versions across all files
    pub total_versions: usize,
    /// Number of files that have version history
    pub files_with_history: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_file_storage;
    use crate::git::types::CommitInfo;
    use chrono::Utc;
    use tempfile::TempDir;

    fn create_test_commit() -> CommitInfo {
        CommitInfo {
            sha: "abc123def456".to_string(),
            parents: vec![],
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            timestamp: Utc::now(),
            message: "Test commit".to_string(),
            insertions: 10,
            deletions: 5,
            files_changed: vec!["test.rs".to_string()],
        }
    }

    fn create_test_file_entry() -> FileEntry {
        use chrono::Utc;
        FileEntry {
            path: "src/main.rs".to_string(),
            content: b"fn main() { println!(\"Hello\"); }".to_vec(),
            size: 30,
            is_binary: false,
            extension: Some("rs".to_string()),
            mime_type: Some("text/x-rust".to_string()),
            last_commit: "abc123def456".to_string(),
            last_modified: Utc::now(),
        }
    }

    #[test]
    fn test_document_path_creation() {
        let config = RepositoryOrganizationConfig::default();
        let manager = FileOrganizationManager::new(config);

        let path = manager
            .create_document_path("kota-db", "src/main.rs", None)
            .unwrap();
        assert_eq!(path, "data/analysis/kota-db/files/src/main.rs");

        let path_with_commit = manager
            .create_document_path("kota-db", "src/main.rs", Some("abc123def"))
            .unwrap();
        assert_eq!(path_with_commit, "data/analysis/kota-db/files/src/main.rs");
    }

    #[test]
    fn test_document_path_with_commit() {
        let config = RepositoryOrganizationConfig {
            include_commit_in_path: true,
            ..Default::default()
        };
        let manager = FileOrganizationManager::new(config);

        let path = manager
            .create_document_path("kota-db", "src/main.rs", Some("abc123def456"))
            .unwrap();
        assert_eq!(path, "data/analysis/kota-db/files/abc123de/src/main.rs");
    }

    #[tokio::test]
    async fn test_file_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;

        let config = RepositoryOrganizationConfig::default();
        let mut manager = FileOrganizationManager::new(config);

        let file_entry = create_test_file_entry();
        let commit_info = create_test_commit();

        let doc_id = manager
            .handle_file_creation(
                &mut storage,
                &file_entry,
                &commit_info,
                "test-repo",
                "/path/to/repo",
            )
            .await?;

        // Verify document was created
        let doc = storage.get(&doc_id).await?.unwrap();
        assert!(doc.path.as_str().contains("test-repo"));
        assert!(doc.path.as_str().contains("src/main.rs"));

        // Verify cache was updated
        assert_eq!(manager.get_document_id("src/main.rs"), Some(doc_id));

        Ok(())
    }

    #[test]
    fn test_statistics() {
        let config = RepositoryOrganizationConfig::default();
        let mut manager = FileOrganizationManager::new(config);

        // Add some test data to caches
        let doc_id1 = crate::ValidatedDocumentId::new();
        let doc_id2 = crate::ValidatedDocumentId::new();

        manager
            .file_path_cache
            .insert("file1.rs".to_string(), doc_id1);
        manager
            .file_path_cache
            .insert("file2.rs".to_string(), doc_id2);

        manager.file_history_cache.insert(
            "file1.rs".to_string(),
            vec![("commit1".to_string(), doc_id1)],
        );

        let stats = manager.get_statistics();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.total_versions, 1);
        assert_eq!(stats.files_with_history, 1);
    }
}
