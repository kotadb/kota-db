// Validated Types - Stage 6: Component Library
// This module provides strongly-typed wrappers that enforce invariants at compile time.
// These types cannot be constructed with invalid data, eliminating entire classes of bugs.

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Represents different types of relationships between code symbols
/// This is used throughout the codebase for dependency tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    /// Symbol imports/uses another
    Imports,
    /// Symbol extends/inherits from another
    Extends,
    /// Symbol implements an interface/trait
    Implements,
    /// Symbol calls/invokes another
    Calls,
    /// Symbol is defined within another
    ChildOf,
    /// Symbol returns another as a type
    Returns,
    /// Symbol references another (weak dependency)
    References,
    /// Custom relationship type
    Custom(String),
}

/// A path that has been validated and is guaranteed to be safe
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatedPath {
    inner: PathBuf,
}

impl ValidatedPath {
    /// Create a new validated path
    ///
    /// # Invariants
    /// - Path is non-empty
    /// - No directory traversal (..)
    /// - No null bytes
    /// - Valid UTF-8
    /// - Not a reserved name (Windows compatibility)
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))?;

        // Use our existing validation
        crate::validation::path::validate_file_path(path_str)?;

        Ok(Self {
            inner: path.to_path_buf(),
        })
    }

    /// Get the inner path
    pub fn as_path(&self) -> &Path {
        &self.inner
    }

    /// Get as string (guaranteed to be valid UTF-8)
    pub fn as_str(&self) -> &str {
        self.inner.to_str().expect("ValidatedPath is always UTF-8")
    }
}

impl fmt::Display for ValidatedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A document ID that is guaranteed to be valid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidatedDocumentId {
    inner: Uuid,
}

impl Default for ValidatedDocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidatedDocumentId {
    /// Create a new document ID
    pub fn new() -> Self {
        Self {
            inner: Uuid::new_v4(),
        }
    }

    /// Create from existing UUID with validation
    pub fn from_uuid(id: Uuid) -> Result<Self> {
        ensure!(!id.is_nil(), "Document ID cannot be nil UUID");
        Ok(Self { inner: id })
    }

    /// Parse from string
    pub fn parse(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s)?;
        Self::from_uuid(uuid)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> Uuid {
        self.inner
    }
}

impl fmt::Display for ValidatedDocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// A non-empty title with enforced length limits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedTitle {
    inner: String,
}

impl ValidatedTitle {
    const MAX_LENGTH: usize = 1024;

    /// Create a new validated title
    ///
    /// # Invariants
    /// - Non-empty after trimming
    /// - Length <= 1024 characters
    pub fn new(title: impl Into<String>) -> Result<Self> {
        let title = title.into();
        let trimmed = title.trim();

        ensure!(!trimmed.is_empty(), "Title cannot be empty");
        ensure!(
            trimmed.len() <= Self::MAX_LENGTH,
            "Title exceeds maximum length of {} characters",
            Self::MAX_LENGTH
        );

        Ok(Self {
            inner: trimmed.to_string(),
        })
    }

    /// Get the title as a string slice
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for ValidatedTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// A non-zero size value
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NonZeroSize {
    inner: u64,
}

impl NonZeroSize {
    /// Create a new non-zero size
    pub fn new(size: u64) -> Result<Self> {
        ensure!(size > 0, "Size must be greater than zero");
        Ok(Self { inner: size })
    }

    /// Get the inner value
    pub fn get(&self) -> u64 {
        self.inner
    }
}

/// A timestamp with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidatedTimestamp {
    inner: i64,
}

impl ValidatedTimestamp {
    /// Create a new validated timestamp
    ///
    /// # Invariants
    /// - Must be positive (after Unix epoch)
    /// - Must be reasonable (not in far future)
    pub fn new(timestamp: i64) -> Result<Self> {
        ensure!(timestamp > 0, "Timestamp must be positive");

        // Check not too far in future (year 3000)
        const YEAR_3000: i64 = 32503680000;
        ensure!(timestamp < YEAR_3000, "Timestamp too far in future");

        Ok(Self { inner: timestamp })
    }

    /// Create a timestamp for the current time
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before Unix epoch")
            .as_secs() as i64;

        Self { inner: timestamp }
    }

    /// Get the inner timestamp
    pub fn as_secs(&self) -> i64 {
        self.inner
    }
}

/// Ordered pair of timestamps (created, updated)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampPair {
    created: ValidatedTimestamp,
    updated: ValidatedTimestamp,
}

impl TimestampPair {
    /// Create a new timestamp pair
    ///
    /// # Invariants
    /// - Updated >= Created
    pub fn new(created: ValidatedTimestamp, updated: ValidatedTimestamp) -> Result<Self> {
        ensure!(
            updated.as_secs() >= created.as_secs(),
            "Updated timestamp must be >= created timestamp"
        );

        Ok(Self { created, updated })
    }

    /// Create a new pair with both timestamps set to now
    pub fn now() -> Self {
        let now = ValidatedTimestamp::now();
        Self {
            created: now,
            updated: now,
        }
    }

    /// Get created timestamp
    pub fn created(&self) -> ValidatedTimestamp {
        self.created
    }

    /// Get updated timestamp
    pub fn updated(&self) -> ValidatedTimestamp {
        self.updated
    }

    /// Update the updated timestamp to now
    pub fn touch(&mut self) {
        self.updated = ValidatedTimestamp::now();
    }
}

/// A validated tag with enforced constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatedTag {
    inner: String,
}

impl ValidatedTag {
    /// Create a new validated tag
    ///
    /// # Invariants
    /// - Non-empty
    /// - Max 128 characters
    /// - Only alphanumeric, dash, underscore, space
    pub fn new(tag: impl Into<String>) -> Result<Self> {
        let tag = tag.into();
        crate::validation::index::validate_tag(&tag)?;
        Ok(Self { inner: tag })
    }

    /// Get the tag as a string slice
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for ValidatedTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// A validated search query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedSearchQuery {
    text: String,
    min_length: usize,
}

impl ValidatedSearchQuery {
    /// Create a new validated search query with enhanced sanitization
    ///
    /// # Invariants
    /// - Non-empty after trimming
    /// - Meets minimum length requirement
    /// - Not too long (max 1024 chars)
    /// - Free from injection patterns
    /// - Properly sanitized
    pub fn new(query: impl Into<String>, min_length: usize) -> Result<Self> {
        let query = query.into();

        // Apply comprehensive sanitization
        let sanitized = crate::query_sanitization::sanitize_search_query(&query)?;

        // Use sanitized text for validation
        let trimmed = sanitized.text.trim();

        ensure!(!trimmed.is_empty(), "Search query cannot be empty");
        ensure!(
            trimmed.len() >= min_length,
            "Search query must be at least {} characters",
            min_length
        );
        ensure!(
            trimmed.len() <= 1024,
            "Search query too long (max 1024 characters)"
        );

        Ok(Self {
            text: trimmed.to_string(),
            min_length,
        })
    }

    /// Get the query text
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Type-safe page ID that cannot be zero
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ValidatedPageId {
    inner: u64,
}

impl ValidatedPageId {
    /// Create a new page ID
    ///
    /// # Invariants
    /// - Must be > 0 (0 is reserved for null)
    /// - Must be < MAX_PAGES
    pub fn new(id: u64) -> Result<Self> {
        crate::validation::storage::validate_page_id(id)?;
        Ok(Self { inner: id })
    }

    /// Get the inner page ID
    pub fn get(&self) -> u64 {
        self.inner
    }
}

/// A limit value with enforced bounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedLimit {
    inner: usize,
    max: usize,
}

impl ValidatedLimit {
    /// Create a new validated limit
    pub fn new(limit: usize, max: usize) -> Result<Self> {
        ensure!(limit > 0, "Limit must be greater than zero");
        ensure!(limit <= max, "Limit exceeds maximum of {}", max);

        Ok(Self { inner: limit, max })
    }

    /// Get the limit value
    pub fn get(&self) -> usize {
        self.inner
    }

    /// Get the maximum allowed value
    pub fn max(&self) -> usize {
        self.max
    }
}

/// State machine for document lifecycle
pub mod state {
    use super::*;

    /// Document state marker traits
    pub trait DocumentState {}

    /// Draft state - document not yet persisted
    pub struct Draft;
    impl DocumentState for Draft {}

    /// Persisted state - document saved to storage
    pub struct Persisted;
    impl DocumentState for Persisted {}

    /// Modified state - document has unsaved changes
    pub struct Modified;
    impl DocumentState for Modified {}

    /// Type-safe document with state
    #[derive(Debug)]
    pub struct TypedDocument<S: DocumentState> {
        pub id: ValidatedDocumentId,
        pub path: ValidatedPath,
        pub hash: [u8; 32],
        pub size: NonZeroSize,
        pub timestamps: TimestampPair,
        pub title: ValidatedTitle,
        pub word_count: u32,
        _state: PhantomData<S>,
    }

    impl TypedDocument<Draft> {
        /// Create a new draft document
        pub fn new(
            path: ValidatedPath,
            hash: [u8; 32],
            size: NonZeroSize,
            title: ValidatedTitle,
            word_count: u32,
        ) -> Self {
            Self {
                id: ValidatedDocumentId::new(),
                path,
                hash,
                size,
                timestamps: TimestampPair::now(),
                title,
                word_count,
                _state: PhantomData,
            }
        }

        /// Transition to persisted state
        pub fn into_persisted(self) -> TypedDocument<Persisted> {
            TypedDocument {
                id: self.id,
                path: self.path,
                hash: self.hash,
                size: self.size,
                timestamps: self.timestamps,
                title: self.title,
                word_count: self.word_count,
                _state: PhantomData,
            }
        }
    }

    impl TypedDocument<Persisted> {
        /// Mark document as modified
        pub fn into_modified(mut self) -> TypedDocument<Modified> {
            self.timestamps.touch();
            TypedDocument {
                id: self.id,
                path: self.path,
                hash: self.hash,
                size: self.size,
                timestamps: self.timestamps,
                title: self.title,
                word_count: self.word_count,
                _state: PhantomData,
            }
        }
    }

    impl TypedDocument<Modified> {
        /// Save changes and return to persisted state
        pub fn into_persisted(self) -> TypedDocument<Persisted> {
            TypedDocument {
                id: self.id,
                path: self.path,
                hash: self.hash,
                size: self.size,
                timestamps: self.timestamps,
                title: self.title,
                word_count: self.word_count,
                _state: PhantomData,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validated_path() {
        // Valid paths
        assert!(ValidatedPath::new("test/file.md").is_ok());
        assert!(ValidatedPath::new("relative/path.txt").is_ok());

        // Invalid paths
        assert!(ValidatedPath::new("").is_err());
        assert!(ValidatedPath::new("../../../etc/passwd").is_err());
        assert!(ValidatedPath::new("file\0with\0null").is_err());
    }

    #[test]
    fn test_validated_title() {
        // Valid titles
        assert!(ValidatedTitle::new("Test Document").is_ok());
        assert!(ValidatedTitle::new("  Trimmed Title  ").is_ok());

        // Invalid titles
        assert!(ValidatedTitle::new("").is_err());
        assert!(ValidatedTitle::new("   ").is_err());
        assert!(ValidatedTitle::new("x".repeat(2000)).is_err());
    }

    #[test]
    fn test_non_zero_size() {
        assert!(NonZeroSize::new(1).is_ok());
        assert!(NonZeroSize::new(1024).is_ok());
        assert!(NonZeroSize::new(0).is_err());
    }

    #[test]
    fn test_timestamp_pair() {
        let created = ValidatedTimestamp::new(1000).expect("Test timestamp should be valid");
        let updated = ValidatedTimestamp::new(2000).expect("Test timestamp should be valid");

        // Valid pair
        assert!(TimestampPair::new(created, updated).is_ok());

        // Invalid: updated before created
        assert!(TimestampPair::new(updated, created).is_err());
    }

    #[test]
    fn test_document_state_machine() {
        use state::*;

        // Create draft
        let draft = TypedDocument::<Draft>::new(
            ValidatedPath::new("test.md").expect("Test path should be valid"),
            [0u8; 32],
            NonZeroSize::new(1024).expect("Test size should be valid"),
            ValidatedTitle::new("Test").expect("Test title should be valid"),
            100,
        );

        // Can only transition to persisted
        let persisted = draft.into_persisted();

        // Can transition to modified
        let modified = persisted.into_modified();

        // Can go back to persisted
        let _persisted_again = modified.into_persisted();
    }
}
