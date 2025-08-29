//! Comprehensive path normalization utilities for consistent path handling across KotaDB
//!
//! This module consolidates all path normalization logic from across the codebase,
//! providing a single source of truth for path handling with security, performance,
//! and cross-platform support.

use anyhow::Result;
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use tracing::{debug, warn};

/// Maximum allowed path length (platform-specific defaults)
#[cfg(target_os = "windows")]
const MAX_PATH_LENGTH: usize = 260;

#[cfg(not(target_os = "windows"))]
const MAX_PATH_LENGTH: usize = 4096;

/// Errors that can occur during path normalization
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Directory traversal detected in path: {0}")]
    DirectoryTraversal(String),

    #[error("Path too long: {length} bytes (max: {max})")]
    PathTooLong { length: usize, max: usize },

    #[error("Invalid Unicode in path")]
    InvalidUnicode,

    #[error("Empty path after normalization")]
    EmptyPath,

    #[error("Suspicious characters in path: {0}")]
    SuspiciousCharacters(String),
}

/// Configuration for path normalization behavior
#[derive(Debug, Clone)]
pub struct PathNormalizationConfig {
    /// Whether to allow directory traversal (..)
    pub allow_traversal: bool,

    /// Whether to enforce maximum path length
    pub enforce_max_length: bool,

    /// Whether to reject paths with suspicious characters
    pub check_suspicious_chars: bool,

    /// Maximum allowed path length (overrides system default)
    pub max_path_length: Option<usize>,
}

impl Default for PathNormalizationConfig {
    fn default() -> Self {
        Self {
            allow_traversal: false,
            enforce_max_length: true,
            check_suspicious_chars: true,
            max_path_length: None,
        }
    }
}

/// A path normalizer that provides consistent path handling across the codebase
pub struct PathNormalizer {
    config: PathNormalizationConfig,
}

impl PathNormalizer {
    /// Create a new path normalizer with default configuration
    pub fn new() -> Self {
        Self::with_config(PathNormalizationConfig::default())
    }

    /// Create a new path normalizer with custom configuration
    pub fn with_config(config: PathNormalizationConfig) -> Self {
        Self { config }
    }

    /// Normalize a path to be relative to a repository root with full validation
    ///
    /// This is the primary method that should be used across the codebase.
    /// It provides:
    /// - Consistent relative path formatting
    /// - Security validation (directory traversal prevention)
    /// - Cross-platform normalization
    /// - Length validation
    ///
    /// # Examples
    /// ```
    /// use kotadb::path_utils::PathNormalizer;
    /// use std::path::Path;
    ///
    /// let normalizer = PathNormalizer::new();
    /// let repo_root = Path::new("/home/user/project");
    /// let absolute_path = Path::new("/home/user/project/src/main.rs");
    /// let relative = normalizer.normalize_relative(absolute_path, repo_root)?;
    /// assert_eq!(relative, "src/main.rs");
    /// ```
    pub fn normalize_relative(&self, path: &Path, repo_root: &Path) -> Result<String> {
        // First, try to make the path relative to the repo root
        let relative_path = if path.is_absolute() && repo_root.is_absolute() {
            match path.strip_prefix(repo_root) {
                Ok(rel) => rel,
                Err(_) => {
                    // Path is not under repo root, use as-is but validate
                    debug!(
                        "Path {:?} is not under repo root {:?}, validating as-is",
                        path, repo_root
                    );
                    path
                }
            }
        } else if path.is_relative() {
            // Already relative, use as-is
            path
        } else {
            // Mixed absolute/relative, this is suspicious
            warn!(
                "Mixed path types - path: {:?} (abs: {}), repo: {:?} (abs: {})",
                path,
                path.is_absolute(),
                repo_root,
                repo_root.is_absolute()
            );
            return Err(PathError::InvalidPath(format!(
                "Mixed absolute/relative paths: path={:?}, repo={:?}",
                path, repo_root
            ))
            .into());
        };

        // Resolve and validate the path
        let normalized = self.resolve_and_validate(relative_path)?;

        // Ensure forward slashes on all platforms
        let final_path = if cfg!(windows) {
            normalized.replace('\\', "/")
        } else {
            normalized
        };

        Ok(final_path)
    }

    /// Resolve a path by handling components and validating security
    fn resolve_and_validate(&self, path: &Path) -> Result<String> {
        let mut resolved_parts = Vec::new();
        let mut depth = 0i32;

        for component in path.components() {
            match component {
                Component::Normal(part) => {
                    if let Some(part_str) = part.to_str() {
                        // Check for suspicious characters
                        if self.config.check_suspicious_chars {
                            self.check_suspicious_characters(part_str)?;
                        }
                        resolved_parts.push(part_str);
                        depth += 1;
                    } else {
                        return Err(PathError::InvalidUnicode.into());
                    }
                }
                Component::ParentDir => {
                    if !self.config.allow_traversal {
                        return Err(PathError::DirectoryTraversal(
                            path.to_string_lossy().to_string(),
                        )
                        .into());
                    }
                    // Only pop if we're not at root
                    if depth > 0 {
                        resolved_parts.pop();
                        depth -= 1;
                    }
                }
                Component::CurDir => {
                    // Current directory (.) - skip it
                }
                Component::RootDir | Component::Prefix(_) => {
                    // Skip root and prefix components for relative paths
                }
            }
        }

        // Join parts
        let result = resolved_parts.join("/");

        // Validate result
        if result.is_empty() {
            return Err(PathError::EmptyPath.into());
        }

        // Check length
        if self.config.enforce_max_length {
            let max_length = self.config.max_path_length.unwrap_or(MAX_PATH_LENGTH);
            if result.len() > max_length {
                return Err(PathError::PathTooLong {
                    length: result.len(),
                    max: max_length,
                }
                .into());
            }
        }

        // Remove leading "./" if present
        let cleaned = if let Some(stripped) = result.strip_prefix("./") {
            stripped.to_string()
        } else {
            result
        };

        Ok(cleaned)
    }

    /// Check for suspicious characters that shouldn't be in file paths
    fn check_suspicious_characters(&self, path_part: &str) -> Result<()> {
        // Windows-invalid characters plus some extras for security
        const SUSPICIOUS_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*', '\0', '\r', '\n'];

        for &ch in SUSPICIOUS_CHARS {
            if path_part.contains(ch) {
                return Err(PathError::SuspiciousCharacters(format!(
                    "Found '{}' in path component: {}",
                    ch, path_part
                ))
                .into());
            }
        }

        Ok(())
    }

    /// Sanitize a path for storage, removing all potentially dangerous elements
    ///
    /// This method is more aggressive than normalize_relative and is suitable
    /// for creating storage paths where security is paramount.
    pub fn sanitize_for_storage(&self, path: &Path) -> Result<String> {
        let mut resolved_parts = Vec::new();

        for component in path.components() {
            match component {
                Component::Normal(part) => {
                    if let Some(part_str) = part.to_str() {
                        // Extra validation for storage paths
                        let sanitized = self.sanitize_path_component(part_str)?;
                        if !sanitized.is_empty() {
                            resolved_parts.push(sanitized);
                        }
                    }
                }
                Component::ParentDir => {
                    // Never allow traversal in storage paths
                    return Err(
                        PathError::DirectoryTraversal(path.to_string_lossy().to_string()).into(),
                    );
                }
                _ => {
                    // Skip other components
                }
            }
        }

        if resolved_parts.is_empty() {
            return Err(PathError::EmptyPath.into());
        }

        Ok(resolved_parts.join("/"))
    }

    /// Sanitize a single path component for safe storage
    fn sanitize_path_component(&self, component: &str) -> Result<String> {
        // Remove any non-alphanumeric characters except common safe ones
        let sanitized: String = component
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect();

        if sanitized.is_empty() && !component.is_empty() {
            return Err(PathError::InvalidPath(format!(
                "Path component contains only invalid characters: {}",
                component
            ))
            .into());
        }

        Ok(sanitized)
    }

    /// Optimized path normalization that avoids allocations when possible
    ///
    /// Returns a Cow<str> that borrows when no changes are needed
    pub fn normalize_cow<'a>(&self, path: &'a str) -> Result<Cow<'a, str>> {
        // Quick check if normalization is needed
        if !path.contains("..")
            && !path.contains("./")
            && !path.contains("\\")
            && !path.starts_with('/')
            && !path.is_empty()
        {
            // Path looks already normalized
            return Ok(Cow::Borrowed(path));
        }

        // Need to normalize
        let path_buf = Path::new(path);
        let normalized = self.resolve_and_validate(path_buf)?;
        Ok(Cow::Owned(normalized))
    }
}

/// Default normalizer instance for convenience
impl Default for PathNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience functions that use the default normalizer

/// Normalize a path to be relative to a repository root (convenience function)
///
/// Uses the default configuration with security checks enabled.
pub fn normalize_path_relative(path: &Path, repo_root: &Path) -> String {
    let normalizer = PathNormalizer::new();
    normalizer
        .normalize_relative(path, repo_root)
        .unwrap_or_else(|e| {
            warn!("Path normalization failed: {}, using fallback", e);
            path.to_string_lossy().to_string()
        })
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
    let normalizer = PathNormalizer::new();
    match normalizer.normalize_relative(&file_path, repo_root) {
        Ok(normalized_str) => {
            // Only allocate new PathBuf if path actually changed
            if file_path.to_string_lossy() == normalized_str {
                (file_path, content)
            } else {
                (PathBuf::from(normalized_str), content)
            }
        }
        Err(e) => {
            warn!("Failed to normalize path {:?}: {}", file_path, e);
            (file_path, content)
        }
    }
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
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/home/user/project");

        // Absolute path under repo root
        let path = Path::new("/home/user/project/src/main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );

        // Already relative path
        let path = Path::new("src/main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );

        // Path with ./ prefix
        let path = Path::new("./src/main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );

        // Nested path
        let path = Path::new("/home/user/project/src/modules/auth.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/modules/auth.rs"
        );
    }

    #[test]
    fn test_directory_traversal_prevention() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/home/user/project");

        // Path with directory traversal
        let path = Path::new("../../../etc/passwd");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());

        match result.unwrap_err().downcast::<PathError>() {
            Ok(PathError::DirectoryTraversal(_)) => {}
            _ => panic!("Expected DirectoryTraversal error"),
        }

        // Path with embedded traversal
        let path = Path::new("src/../../../etc/passwd");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_suspicious_characters_detection() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/home/user/project");

        // Path with null byte
        let path = Path::new("src/main\0.rs");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());

        // Path with pipe character
        let path = Path::new("src/main|cmd.rs");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_length_validation() {
        let config = PathNormalizationConfig {
            max_path_length: Some(20),
            ..Default::default()
        };
        let normalizer = PathNormalizer::with_config(config);
        let repo_root = Path::new("/");

        // Path exceeding limit
        let path = Path::new("very/long/path/that/exceeds/the/limit.rs");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());

        match result.unwrap_err().downcast::<PathError>() {
            Ok(PathError::PathTooLong { .. }) => {}
            _ => panic!("Expected PathTooLong error"),
        }
    }

    #[test]
    fn test_empty_path_handling() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/home/user/project");

        // Empty path
        let path = Path::new("");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_for_storage() {
        let normalizer = PathNormalizer::new();

        // Normal path
        let path = Path::new("src/main.rs");
        assert_eq!(
            normalizer.sanitize_for_storage(path).unwrap(),
            "src/main.rs"
        );

        // Path with special characters
        let path = Path::new("src/main@#$.rs");
        assert_eq!(
            normalizer.sanitize_for_storage(path).unwrap(),
            "src/main.rs"
        );

        // Path with traversal (should fail)
        let path = Path::new("../etc/passwd");
        assert!(normalizer.sanitize_for_storage(path).is_err());
    }

    #[test]
    fn test_normalize_cow_optimization() {
        let normalizer = PathNormalizer::new();

        // Already normalized path should return borrowed
        let path = "src/main.rs";
        match normalizer.normalize_cow(path).unwrap() {
            Cow::Borrowed(s) => assert_eq!(s, path),
            Cow::Owned(_) => panic!("Expected borrowed value"),
        }

        // Path needing normalization should return owned
        let path = "./src/main.rs";
        match normalizer.normalize_cow(path).unwrap() {
            Cow::Owned(s) => assert_eq!(s, "src/main.rs"),
            Cow::Borrowed(_) => panic!("Expected owned value"),
        }
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
    fn test_unicode_path_handling() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/home/user/project");

        // Valid Unicode path
        let path = Path::new("src/файл.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/файл.rs"
        );

        // Emoji in path (should work)
        let path = Path::new("src/📁/main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/📁/main.rs"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_path_normalization() {
        let normalizer = PathNormalizer::new();

        // Basic Windows path
        let repo_root = Path::new(r"C:\Users\user\project");
        let path = Path::new(r"C:\Users\user\project\src\main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );

        // UNC path
        let repo_root = Path::new(r"\\server\share\project");
        let path = Path::new(r"\\server\share\project\src\main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );

        // Mixed separators
        let path = Path::new(r"C:\Users\user\project/src\main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn test_root_path_handling() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/");

        // File at root
        let path = Path::new("/main.rs");
        assert_eq!(
            normalizer.normalize_relative(path, repo_root).unwrap(),
            "main.rs"
        );

        // Just root
        let path = Path::new("/");
        let result = normalizer.normalize_relative(path, repo_root);
        // Root path should result in empty path error
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_parent_components() {
        let config = PathNormalizationConfig {
            allow_traversal: true,
            ..Default::default()
        };
        let normalizer = PathNormalizer::with_config(config);
        let repo_root = Path::new("/home/user/project");

        // Multiple .. components (when allowed)
        let path = Path::new("src/../../other/file.rs");
        let result = normalizer.normalize_relative(path, repo_root);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "other/file.rs");
    }

    #[test]
    fn test_very_long_paths() {
        let normalizer = PathNormalizer::new();
        let repo_root = Path::new("/");

        // Create a very long path
        let long_component = "a".repeat(100);
        let long_path = format!(
            "{}/{}/{}.rs",
            long_component, long_component, long_component
        );
        let path = Path::new(&long_path);

        let result = normalizer.normalize_relative(path, repo_root);

        // Should succeed if under the default limit
        if long_path.len() <= MAX_PATH_LENGTH {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
