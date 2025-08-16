//! Git repository interaction and reading

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::git::types::{CommitInfo, FileEntry, IngestionOptions, RepositoryMetadata};

/// Wrapper around a git repository for reading and analysis
pub struct GitRepository {
    #[cfg(feature = "git-integration")]
    repo: git2::Repository,
    path: PathBuf,
    #[allow(dead_code)]
    options: IngestionOptions,
}

impl GitRepository {
    /// Open a git repository from a path
    pub fn open(path: impl AsRef<Path>, options: IngestionOptions) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        #[cfg(feature = "git-integration")]
        {
            info!("Opening git repository at: {:?}", path);
            let repo = git2::Repository::open(&path)
                .with_context(|| format!("Failed to open git repository at {:?}", path))?;

            Ok(Self {
                repo,
                path,
                options,
            })
        }

        #[cfg(not(feature = "git-integration"))]
        {
            anyhow::bail!(
                "Git integration feature not enabled. Rebuild with --features git-integration"
            );
        }
    }

    /// Get metadata about the repository
    pub fn metadata(&self) -> Result<RepositoryMetadata> {
        #[cfg(feature = "git-integration")]
        {
            let head = self.repo.head().context("Failed to get repository HEAD")?;

            let branch_name = head.shorthand().unwrap_or("HEAD").to_string();

            // Count commits
            let mut revwalk = self.repo.revwalk()?;
            revwalk.push_head()?;
            let commit_count = revwalk.count();

            // Get repository name from path
            let name = self
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Try to get remote URL
            let url = self
                .repo
                .find_remote("origin")
                .ok()
                .and_then(|remote| remote.url().map(String::from));

            Ok(RepositoryMetadata {
                name,
                url,
                path: self.path.clone(),
                default_branch: branch_name,
                commit_count,
                created_at: None, // Would need to find first commit
                updated_at: Utc::now(),
            })
        }

        #[cfg(not(feature = "git-integration"))]
        {
            anyhow::bail!("Git integration feature not enabled");
        }
    }

    /// List all files in the repository at current HEAD
    pub fn list_files(&self) -> Result<Vec<FileEntry>> {
        #[cfg(feature = "git-integration")]
        {
            let mut files = Vec::new();
            let head = self.repo.head()?;
            let tree = head.peel_to_tree()?;

            self.walk_tree(&tree, "", &mut files)?;

            Ok(files)
        }

        #[cfg(not(feature = "git-integration"))]
        {
            anyhow::bail!("Git integration feature not enabled");
        }
    }

    #[cfg(feature = "git-integration")]
    fn walk_tree(&self, tree: &git2::Tree, prefix: &str, files: &mut Vec<FileEntry>) -> Result<()> {
        use std::path::Path;

        for entry in tree.iter() {
            let name = entry.name();
            if name.is_none() {
                continue;
            }
            let name = name.unwrap();
            let path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };

            // Check exclusion patterns
            if self.should_exclude(&path) {
                debug!("Excluding path: {}", path);
                continue;
            }

            match entry.kind() {
                Some(git2::ObjectType::Tree) => {
                    // Recursively walk subdirectories
                    if let Ok(subtree) = self.repo.find_tree(entry.id()) {
                        self.walk_tree(&subtree, &path, files)?;
                    }
                }
                Some(git2::ObjectType::Blob) => {
                    // Process file
                    if let Ok(blob) = self.repo.find_blob(entry.id()) {
                        let content = blob.content().to_vec();
                        let size = content.len();

                        // Skip large files
                        if size > self.options.max_file_size {
                            debug!("Skipping large file: {} ({} bytes)", path, size);
                            continue;
                        }

                        let extension = Path::new(&path)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(String::from);

                        // Check extension filters
                        if !self.should_include_extension(&extension) {
                            debug!("Skipping file with excluded extension: {}", path);
                            continue;
                        }

                        let is_binary = content.contains(&0);

                        files.push(FileEntry {
                            path: path.clone(),
                            content: if self.options.include_file_contents {
                                content
                            } else {
                                vec![]
                            },
                            size,
                            is_binary,
                            extension,
                            mime_type: None, // Could detect with mime crate
                            last_commit: String::new(), // Would need to look up
                            last_modified: Utc::now(), // Would need to look up
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[cfg(feature = "git-integration")]
    fn should_exclude(&self, path: &str) -> bool {
        self.options
            .exclude_patterns
            .iter()
            .any(|pattern| path.contains(pattern))
    }

    #[cfg(feature = "git-integration")]
    fn should_include_extension(&self, extension: &Option<String>) -> bool {
        // If include list is specified, only include those extensions
        if !self.options.include_extensions.is_empty() {
            return extension
                .as_ref()
                .map(|ext| self.options.include_extensions.contains(ext))
                .unwrap_or(false);
        }

        // Otherwise, exclude the exclusion list
        if let Some(ext) = extension {
            !self.options.exclude_extensions.contains(ext)
        } else {
            true // Files without extensions are included by default
        }
    }

    /// Get recent commits from the repository
    pub fn get_commits(&self, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        #[cfg(feature = "git-integration")]
        {
            let mut commits = Vec::new();
            let mut revwalk = self.repo.revwalk()?;
            revwalk.push_head()?;

            let max_commits = limit.or(self.options.max_history_depth).unwrap_or(1000);

            for (i, oid) in revwalk.enumerate() {
                if i >= max_commits {
                    break;
                }

                let oid = oid?;
                let commit = self.repo.find_commit(oid)?;

                let commit_info = self.commit_to_info(&commit)?;
                commits.push(commit_info);
            }

            Ok(commits)
        }

        #[cfg(not(feature = "git-integration"))]
        {
            anyhow::bail!("Git integration feature not enabled");
        }
    }

    #[cfg(feature = "git-integration")]
    fn commit_to_info(&self, commit: &git2::Commit) -> Result<CommitInfo> {
        let sha = commit.id().to_string();
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();

        let timestamp = Utc
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        let parents = commit.parent_ids().map(|id| id.to_string()).collect();

        // For files changed, we'd need to diff with parent
        // This is simplified for now
        let files_changed = vec![];
        let insertions = 0;
        let deletions = 0;

        Ok(CommitInfo {
            sha,
            message,
            author_name,
            author_email,
            timestamp,
            parents,
            files_changed,
            insertions,
            deletions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_repository_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let options = IngestionOptions::default();

        // This test will fail if not in a git repo, which is expected
        let result = GitRepository::open(temp_dir.path(), options);

        #[cfg(feature = "git-integration")]
        assert!(result.is_err()); // Not a git repo

        #[cfg(not(feature = "git-integration"))]
        assert!(result.is_err()); // Feature not enabled

        Ok(())
    }
}
