// Builder Patterns - Stage 6: Component Library
// This module provides fluent builder APIs for constructing complex objects
// with sensible defaults and compile-time validation.

use crate::contracts::{Document, Query, StorageMetrics};
use crate::types::*;
use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Fluent builder for creating Documents
pub struct DocumentBuilder {
    id: Option<ValidatedDocumentId>,
    path: Option<ValidatedPath>,
    title: Option<ValidatedTitle>,
    content: Option<Vec<u8>>,
    tags: Vec<ValidatedTag>,
    word_count: Option<u32>,
    timestamps: Option<TimestampPair>,
}

impl DocumentBuilder {
    /// Create a new document builder
    pub fn new() -> Self {
        Self {
            id: None,
            path: None,
            title: None,
            content: None,
            tags: Vec::new(),
            word_count: None,
            timestamps: None,
        }
    }

    /// Set the document ID
    /// If not specified, a new UUID will be generated automatically
    pub fn id(mut self, id: ValidatedDocumentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the document ID from a UUID
    /// If not specified, a new UUID will be generated automatically
    pub fn id_from_uuid(mut self, uuid: uuid::Uuid) -> Result<Self> {
        self.id = Some(ValidatedDocumentId::from_uuid(uuid)?);
        Ok(self)
    }

    /// Set the document path
    pub fn path(mut self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        self.path = Some(ValidatedPath::new(path)?);
        Ok(self)
    }

    /// Set the document title
    pub fn title(mut self, title: impl Into<String>) -> Result<Self> {
        self.title = Some(ValidatedTitle::new(title)?);
        Ok(self)
    }

    /// Set the document content and automatically calculate hash and size
    pub fn content(mut self, content: impl Into<Vec<u8>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the word count (or calculate from content if not provided)
    pub fn word_count(mut self, count: u32) -> Self {
        self.word_count = Some(count);
        self
    }

    /// Add a tag to the document
    pub fn tag(mut self, tag: &str) -> Result<Self> {
        self.tags.push(ValidatedTag::new(tag)?);
        Ok(self)
    }

    /// Set custom timestamps
    pub fn timestamps(mut self, created: i64, updated: i64) -> Result<Self> {
        let created = ValidatedTimestamp::new(created)?;
        let updated = ValidatedTimestamp::new(updated)?;
        self.timestamps = Some(TimestampPair::new(created, updated)?);
        Ok(self)
    }

    /// Build the document
    pub fn build(self) -> Result<Document> {
        let path = self
            .path
            .ok_or_else(|| anyhow::anyhow!("Document path is required"))?;

        let title = self
            .title
            .ok_or_else(|| anyhow::anyhow!("Document title is required"))?;

        let content = self
            .content
            .ok_or_else(|| anyhow::anyhow!("Document content is required"))?;

        // Calculate hash
        let _hash = crate::pure::metadata::calculate_hash(&content);

        // Calculate word count if not provided
        let _word_count = self.word_count.unwrap_or_else(|| {
            let text = String::from_utf8_lossy(&content);
            text.split_whitespace().count() as u32
        });

        // Use provided timestamps or create new ones
        let timestamps = self.timestamps.unwrap_or_else(TimestampPair::now);

        // Use provided ID or generate new one
        let document_id = self.id.unwrap_or_default();

        Ok(Document::new(
            document_id,
            path,
            title,
            content,
            self.tags,
            DateTime::<Utc>::from_timestamp(timestamps.created().as_secs(), 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid created timestamp"))?,
            DateTime::<Utc>::from_timestamp(timestamps.updated().as_secs(), 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid updated timestamp"))?,
        ))
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluent builder for creating Queries
pub struct QueryBuilder {
    text: Option<String>,
    tags: Vec<ValidatedTag>,
    date_range: Option<(ValidatedTimestamp, ValidatedTimestamp)>,
    limit: Option<ValidatedLimit>,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            text: None,
            tags: Vec::new(),
            date_range: None,
            limit: None,
        }
    }

    /// Add text search criteria with enhanced sanitization
    pub fn with_text(mut self, text: impl Into<String>) -> Result<Self> {
        let text = text.into();

        // Apply comprehensive sanitization
        let sanitized = crate::query_sanitization::sanitize_search_query(&text)?;

        // Check if query became empty after sanitization
        if sanitized.is_empty() && text.trim() != "*" {
            bail!("Search text became empty after sanitization");
        }

        // Log warnings if query was modified
        if sanitized.was_modified {
            tracing::debug!(
                "Query sanitized: original='{}', sanitized='{}', warnings={:?}",
                text,
                sanitized.text,
                sanitized.warnings
            );
        }

        self.text = Some(sanitized.text);
        Ok(self)
    }

    /// Add a tag filter with sanitization
    pub fn with_tag(mut self, tag: impl Into<String>) -> Result<Self> {
        let tag_str = tag.into();
        let sanitized = crate::query_sanitization::sanitize_tag(&tag_str)?;
        self.tags.push(ValidatedTag::new(sanitized)?);
        Ok(self)
    }

    /// Add multiple tag filters with sanitization
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<Self> {
        for tag in tags {
            let tag_str = tag.into();
            let sanitized = crate::query_sanitization::sanitize_tag(&tag_str)?;
            self.tags.push(ValidatedTag::new(sanitized)?);
        }
        Ok(self)
    }

    /// Set date range filter
    pub fn with_date_range(mut self, start: i64, end: i64) -> Result<Self> {
        let start = ValidatedTimestamp::new(start)?;
        let end = ValidatedTimestamp::new(end)?;
        ensure!(
            end.as_secs() >= start.as_secs(),
            "End date must be >= start date"
        );
        self.date_range = Some((start, end));
        Ok(self)
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        // Allow higher limits for operations that need to see all documents
        // The previous hardcoded limit of 1000 was causing validation failures
        const MAX_QUERY_LIMIT: usize = 100_000;
        self.limit = Some(ValidatedLimit::new(limit, MAX_QUERY_LIMIT)?);
        Ok(self)
    }

    /// Build the query
    pub fn build(self) -> Result<Query> {
        let tags = if self.tags.is_empty() {
            None
        } else {
            Some(
                self.tags
                    .into_iter()
                    .map(|t| t.as_str().to_string())
                    .collect(),
            )
        };

        let _date_range = self
            .date_range
            .map(|(start, end)| (start.as_secs(), end.as_secs()));

        let limit = self.limit.map(|l| l.get()).unwrap_or(10);

        // Check if the text contains wildcards and should be treated as a path pattern
        let (query_text, path_pattern) = if let Some(ref text) = self.text {
            if text.contains('*') {
                // This is a wildcard pattern, use it as path pattern
                (None, Some(text.clone()))
            } else {
                // Regular text search
                (self.text.clone(), None)
            }
        } else {
            (None, None)
        };

        // Create query with proper support for wildcards and tags
        let mut query = Query::new(query_text, tags.clone(), path_pattern.clone(), limit)?;

        // Set tags properly since Query::new doesn't handle them correctly
        if let Some(tag_strings) = tags {
            query.tags = tag_strings
                .into_iter()
                .map(ValidatedTag::new)
                .collect::<Result<Vec<_>, _>>()?;
        }

        // Set path_pattern directly if it wasn't handled by Query::new
        if let Some(pattern) = path_pattern {
            query.path_pattern = Some(pattern);
        }

        Ok(query)
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration builder for storage
pub struct StorageConfigBuilder {
    path: Option<ValidatedPath>,
    cache_size: Option<usize>,
    sync_interval: Option<Duration>,
    compression_enabled: bool,
    encryption_key: Option<[u8; 32]>,
}

impl StorageConfigBuilder {
    /// Create a new storage config builder
    pub fn new() -> Self {
        Self {
            path: None,
            cache_size: Some(100 * 1024 * 1024), // 100MB default
            sync_interval: Some(Duration::from_secs(60)), // 1 minute default
            compression_enabled: true,
            encryption_key: None,
        }
    }

    /// Set the storage path
    pub fn path(mut self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        self.path = Some(ValidatedPath::new(path)?);
        Ok(self)
    }

    /// Set cache size in bytes
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = Some(size);
        self
    }

    /// Disable caching
    pub fn no_cache(mut self) -> Self {
        self.cache_size = None;
        self
    }

    /// Set sync interval
    pub fn sync_interval(mut self, interval: Duration) -> Self {
        self.sync_interval = Some(interval);
        self
    }

    /// Enable/disable compression
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Set encryption key
    pub fn encryption_key(mut self, key: [u8; 32]) -> Self {
        self.encryption_key = Some(key);
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<StorageConfig> {
        let path = self
            .path
            .ok_or_else(|| anyhow::anyhow!("Storage path is required"))?;

        Ok(StorageConfig {
            path,
            cache_size: self.cache_size,
            sync_interval: self.sync_interval,
            compression_enabled: self.compression_enabled,
            encryption_key: self.encryption_key,
        })
    }
}

impl Default for StorageConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage configuration
pub struct StorageConfig {
    pub path: ValidatedPath,
    pub cache_size: Option<usize>,
    pub sync_interval: Option<Duration>,
    pub compression_enabled: bool,
    pub encryption_key: Option<[u8; 32]>,
}

/// Index configuration builder
pub struct IndexConfigBuilder {
    name: Option<String>,
    max_memory: Option<usize>,
    persistence_enabled: bool,
    fuzzy_search: bool,
    similarity_threshold: f32,
}

impl IndexConfigBuilder {
    /// Create a new index config builder
    pub fn new() -> Self {
        Self {
            name: None,
            max_memory: Some(50 * 1024 * 1024), // 50MB default
            persistence_enabled: true,
            fuzzy_search: true,
            similarity_threshold: 0.8,
        }
    }

    /// Set index name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set maximum memory usage
    pub fn max_memory(mut self, bytes: usize) -> Self {
        self.max_memory = Some(bytes);
        self
    }

    /// Enable/disable persistence
    pub fn persistence(mut self, enabled: bool) -> Self {
        self.persistence_enabled = enabled;
        self
    }

    /// Enable/disable fuzzy search
    pub fn fuzzy_search(mut self, enabled: bool) -> Self {
        self.fuzzy_search = enabled;
        self
    }

    /// Set similarity threshold for fuzzy search (0.0 - 1.0)
    pub fn similarity_threshold(mut self, threshold: f32) -> Result<Self> {
        ensure!(
            (0.0..=1.0).contains(&threshold),
            "Similarity threshold must be between 0.0 and 1.0"
        );
        self.similarity_threshold = threshold;
        Ok(self)
    }

    /// Build the configuration
    pub fn build(self) -> Result<IndexConfig> {
        let name = self
            .name
            .ok_or_else(|| anyhow::anyhow!("Index name is required"))?;

        Ok(IndexConfig {
            name,
            max_memory: self.max_memory,
            persistence_enabled: self.persistence_enabled,
            fuzzy_search: self.fuzzy_search,
            similarity_threshold: self.similarity_threshold,
        })
    }
}

impl Default for IndexConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Index configuration
pub struct IndexConfig {
    pub name: String,
    pub max_memory: Option<usize>,
    pub persistence_enabled: bool,
    pub fuzzy_search: bool,
    pub similarity_threshold: f32,
}

/// Builder for storage metrics
pub struct MetricsBuilder {
    total_documents: u64,
    total_size_bytes: u64,
    avg_document_size: f64,
    storage_efficiency: f64,
    fragmentation: f64,
}

impl MetricsBuilder {
    /// Create a new metrics builder
    pub fn new() -> Self {
        Self {
            total_documents: 0,
            total_size_bytes: 0,
            avg_document_size: 0.0,
            storage_efficiency: 0.0,
            fragmentation: 0.0,
        }
    }

    /// Set document count
    pub fn document_count(mut self, count: u64) -> Self {
        self.total_documents = count;
        self
    }

    /// Set total size
    pub fn total_size(mut self, bytes: u64) -> Self {
        self.total_size_bytes = bytes;
        self
    }

    /// Set average document size
    pub fn avg_document_size(mut self, size: f64) -> Self {
        self.avg_document_size = size;
        self
    }

    /// Set storage efficiency
    pub fn storage_efficiency(mut self, efficiency: f64) -> Self {
        self.storage_efficiency = efficiency;
        self
    }

    /// Set fragmentation
    pub fn fragmentation(mut self, fragmentation: f64) -> Self {
        self.fragmentation = fragmentation;
        self
    }

    /// Build the metrics
    pub fn build(self) -> Result<StorageMetrics> {
        // Calculate avg_document_size if not set
        let avg_size = if self.avg_document_size > 0.0 {
            self.avg_document_size
        } else if self.total_documents > 0 {
            self.total_size_bytes as f64 / self.total_documents as f64
        } else {
            0.0
        };

        let metrics = StorageMetrics {
            total_documents: self.total_documents,
            total_size_bytes: self.total_size_bytes,
            avg_document_size: avg_size,
            storage_efficiency: self.storage_efficiency,
            fragmentation: self.fragmentation,
        };

        Ok(metrics)
    }
}

impl Default for MetricsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_builder() {
        let doc = DocumentBuilder::new()
            .path("test/doc.md")
            .expect("Valid path should not fail")
            .title("Test Document")
            .expect("Valid title should not fail")
            .content(b"Hello, world!")
            .build();

        assert!(doc.is_ok());
        let doc = doc.expect("Document build should succeed");
        assert_eq!(doc.path.as_str(), "test/doc.md");
        assert_eq!(doc.title.as_str(), "Test Document");
        assert_eq!(doc.size, 13);
    }

    #[test]
    fn test_document_builder_with_custom_id() {
        use uuid::Uuid;

        // Create a specific UUID to test with
        let custom_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let custom_id = ValidatedDocumentId::from_uuid(custom_uuid).unwrap();

        let doc = DocumentBuilder::new()
            .id(custom_id)
            .path("test/doc.md")
            .expect("Valid path should not fail")
            .title("Test Document")
            .expect("Valid title should not fail")
            .content(b"Hello, world!")
            .build();

        assert!(doc.is_ok());
        let doc = doc.expect("Document build should succeed");

        // Verify the document uses the specified ID, not a generated one
        assert_eq!(doc.id.as_uuid(), custom_uuid);
        assert_eq!(doc.path.as_str(), "test/doc.md");
        assert_eq!(doc.title.as_str(), "Test Document");
    }

    #[test]
    fn test_document_builder_generates_id_when_not_specified() {
        let doc1 = DocumentBuilder::new()
            .path("test/doc1.md")
            .expect("Valid path should not fail")
            .title("Test Document 1")
            .expect("Valid title should not fail")
            .content(b"Content 1")
            .build()
            .expect("Document build should succeed");

        let doc2 = DocumentBuilder::new()
            .path("test/doc2.md")
            .expect("Valid path should not fail")
            .title("Test Document 2")
            .expect("Valid title should not fail")
            .content(b"Content 2")
            .build()
            .expect("Document build should succeed");

        // Verify different documents get different generated IDs
        assert_ne!(doc1.id.as_uuid(), doc2.id.as_uuid());

        // Verify IDs are valid UUIDs (not nil)
        assert_ne!(doc1.id.as_uuid(), uuid::Uuid::nil());
        assert_ne!(doc2.id.as_uuid(), uuid::Uuid::nil());
    }

    #[test]
    fn test_document_builder_id_from_uuid() {
        use uuid::Uuid;

        // Create a specific UUID to test with
        let custom_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let doc = DocumentBuilder::new()
            .id_from_uuid(custom_uuid)
            .expect("Valid UUID should not fail")
            .path("test/doc.md")
            .expect("Valid path should not fail")
            .title("Test Document")
            .expect("Valid title should not fail")
            .content(b"Hello, world!")
            .build();

        assert!(doc.is_ok());
        let doc = doc.expect("Document build should succeed");

        // Verify the document uses the specified UUID
        assert_eq!(doc.id.as_uuid(), custom_uuid);
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .with_text("search term")
            .expect("Valid search term should not fail")
            .with_tag("rust")
            .expect("Valid tag should not fail")
            .with_tag("database")
            .expect("Valid tag should not fail")
            .with_limit(50)
            .expect("Valid limit should not fail")
            .build();

        assert!(query.is_ok());
        let query = query.expect("Query build should succeed");
        assert_eq!(query.search_terms.len(), 1);
        assert_eq!(query.search_terms[0].as_str(), "search term");
        assert_eq!(query.tags.len(), 2);
        assert_eq!(query.limit.get(), 50);
    }

    #[test]
    fn test_storage_config_builder() {
        let config = StorageConfigBuilder::new()
            .path("data/kotadb")
            .expect("Valid path should not fail")
            .cache_size(200 * 1024 * 1024)
            .compression(true)
            .build();

        assert!(config.is_ok());
        let config = config.expect("Config build should succeed");
        assert_eq!(config.path.as_str(), "data/kotadb");
        assert_eq!(config.cache_size, Some(200 * 1024 * 1024));
        assert!(config.compression_enabled);
    }

    #[test]
    fn test_builder_validation() {
        // Missing required fields
        let doc = DocumentBuilder::new().build();
        assert!(doc.is_err());

        let query = QueryBuilder::new().build();
        assert!(query.is_ok()); // Query has defaults

        let storage = StorageConfigBuilder::new().build();
        assert!(storage.is_err()); // Path is required
    }
}
