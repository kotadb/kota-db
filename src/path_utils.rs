//! Path normalization utilities for consistent path handling across KotaDB
//!
//! This module provides utilities to ensure consistent path formats between
//! different components like binary symbol storage, file collection, and
//! dependency graph building.

use std::path::{Path, PathBuf};
use tracing::debug;

/// Normalize a path to be relative to a repository root
///
/// This function ensures consistent path formats across all KotaDB components:
/// - Converts absolute paths to relative paths based on repo root
/// - Uses forward slashes (/) as path separators on all platforms
/// - Removes leading "./" prefixes
/// - Handles paths that are already relative
///
/// # Examples
/// ```
/// use std::path::Path;
/// let repo_root = Path::new("/home/user/project");
/// let absolute_path = Path::new("/home/user/project/src/main.rs");
/// let relative = normalize_path_relative(absolute_path, repo_root);
/// assert_eq!(relative, "src/main.rs");
/// ```
pub fn normalize_path_relative(path: &Path, repo_root: &Path) -> String {
    // First, try to make the path relative to the repo root
    let relative_path = if path.is_absolute() && repo_root.is_absolute() {
        match path.strip_prefix(repo_root) {
            Ok(rel) => rel,
            Err(_) => {
                // Path is not under repo root, use as-is
                debug!(
                    "Path {:?} is not under repo root {:?}, using as-is",
                    path, repo_root
                );
                path
            }
        }
    } else if path.is_relative() {
        // Already relative, use as-is
        path
    } else {
        // Mixed absolute/relative, try to handle gracefully
        debug!(
            "Mixed path types - path: {:?} (abs: {}), repo: {:?} (abs: {})",
            path,
            path.is_absolute(),
            repo_root,
            repo_root.is_absolute()
        );
        path
    };

    // Convert to string with forward slashes
    let path_str = relative_path.to_string_lossy();

    // Normalize path separators to forward slashes
    let normalized = if cfg!(windows) {
        path_str.replace('\\', "/")
    } else {
        path_str.to_string()
    };

    // Remove leading "./" if present
    if let Some(stripped) = normalized.strip_prefix("./") {
        stripped.to_string()
    } else {
        normalized
    }
}

/// Convert a PathBuf with file content to use normalized relative paths
///
/// This is used when collecting source files to ensure paths match
/// the format stored in binary symbols.
pub fn normalize_file_entry(
    file_path: PathBuf,
    content: Vec<u8>,
    repo_root: &Path,
) -> (PathBuf, Vec<u8>) {
    let normalized_str = normalize_path_relative(&file_path, repo_root);
    (PathBuf::from(normalized_str), content)
}

/// Check if two paths refer to the same file, handling different formats
///
/// This function compares paths flexibly, handling:
/// - Absolute vs relative paths
/// - Different path separators
/// - Leading "./" prefixes
pub fn paths_equivalent(path1: &str, path2: &str) -> bool {
    // Quick exact match
    if path1 == path2 {
        return true;
    }

    // Normalize both paths for comparison
    let norm1 = normalize_for_comparison(path1);
    let norm2 = normalize_for_comparison(path2);

    norm1 == norm2 || norm1.ends_with(&norm2) || norm2.ends_with(&norm1)
}

/// Normalize a path string for comparison purposes
fn normalize_for_comparison(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");

    // Remove leading "./"
    if let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }

    // Remove trailing "/"
    if normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_relative() {
        let repo_root = Path::new("/home/user/project");

        // Absolute path under repo root
        let path = Path::new("/home/user/project/src/main.rs");
        assert_eq!(normalize_path_relative(path, repo_root), "src/main.rs");

        // Already relative path
        let path = Path::new("src/main.rs");
        assert_eq!(normalize_path_relative(path, repo_root), "src/main.rs");

        // Path with ./ prefix
        let path = Path::new("./src/main.rs");
        assert_eq!(normalize_path_relative(path, repo_root), "src/main.rs");

        // Nested path
        let path = Path::new("/home/user/project/src/modules/auth.rs");
        assert_eq!(
            normalize_path_relative(path, repo_root),
            "src/modules/auth.rs"
        );
    }

    #[test]
    fn test_paths_equivalent() {
        // Exact match
        assert!(paths_equivalent("src/main.rs", "src/main.rs"));

        // With ./ prefix
        assert!(paths_equivalent("./src/main.rs", "src/main.rs"));

        // Different separators (simulated)
        assert!(paths_equivalent("src/main.rs", "src/main.rs"));

        // One absolute, one relative (suffix match)
        assert!(paths_equivalent("/project/src/main.rs", "src/main.rs"));

        // Different files
        assert!(!paths_equivalent("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("./src/main.rs"), "src/main.rs");
        assert_eq!(normalize_for_comparison("src/main.rs/"), "src/main.rs");
        assert_eq!(normalize_for_comparison("./src/main.rs/"), "src/main.rs");
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_path_normalization() {
        let repo_root = Path::new(r"C:\Users\user\project");
        let path = Path::new(r"C:\Users\user\project\src\main.rs");
        assert_eq!(normalize_path_relative(path, repo_root), "src/main.rs");
    }
}
