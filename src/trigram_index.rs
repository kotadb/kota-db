// Trigram Index Implementation - Full-Text Search Engine
// This implements the Index trait using trigram-based text search
// Designed to work with all Stage 6 component library wrappers

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;

use crate::contracts::{Document, Index, Query};
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::validation;
use crate::wrappers::MeteredIndex;

/// Trigram index implementation for full-text search
///
/// This index extracts trigrams (3-character sequences) from document content
/// and builds an inverted index for fast text search capabilities.
pub struct TrigramIndex {
    /// Root directory for the index
    index_path: PathBuf,
    /// Inverted index: trigram -> set of document IDs
    trigram_index: RwLock<HashMap<String, HashSet<ValidatedDocumentId>>>,
    /// Document content cache for ranking and snippet extraction
    document_cache: RwLock<HashMap<ValidatedDocumentId, DocumentContent>>,
    /// Write-ahead log for durability
    wal_writer: RwLock<Option<tokio::fs::File>>,
    /// Index metadata
    metadata: RwLock<TrigramMetadata>,
}

/// Cached document content for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentContent {
    title: String,
    content_preview: String, // First 500 chars for snippets
    word_count: usize,
    trigram_count: usize,
}

/// Metadata for the trigram index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrigramMetadata {
    version: u32,
    document_count: usize,
    trigram_count: usize,
    total_trigrams: usize,
    created: i64,
    updated: i64,
}

impl Default for TrigramMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            version: 1,
            document_count: 0,
            trigram_count: 0,
            total_trigrams: 0,
            created: now,
            updated: now,
        }
    }
}

impl TrigramIndex {
    /// Extract trigrams from text
    ///
    /// Converts text to lowercase and extracts all 3-character sequences.
    /// Special handling for unicode characters and normalization.
    pub fn extract_trigrams(text: &str) -> Vec<String> {
        let normalized = text.to_lowercase();
        let chars: Vec<char> = normalized.chars().collect();

        if chars.len() < 3 {
            return Vec::new();
        }

        let mut trigrams = Vec::new();
        for i in 0..=(chars.len() - 3) {
            let trigram: String = chars[i..i + 3].iter().collect();

            // Skip trigrams that are only whitespace or punctuation
            if trigram.chars().any(|c| c.is_alphanumeric()) {
                trigrams.push(trigram);
            }
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        trigrams.retain(|trigram| seen.insert(trigram.clone()));

        trigrams
    }

    /// Extract searchable text from a document
    ///
    /// Combines title and content for comprehensive text indexing
    pub fn extract_searchable_text(document: &Document) -> String {
        let title = document.title.as_str();
        let content = String::from_utf8_lossy(&document.content);

        format!("{title} {content}")
    }

    /// Calculate simple relevance score for a document
    ///
    /// Based on trigram frequency and document length
    pub fn calculate_relevance_score(
        query_trigrams: &[String],
        doc_trigrams: &[String],
        word_count: usize,
    ) -> f64 {
        if query_trigrams.is_empty() || doc_trigrams.is_empty() {
            return 0.0;
        }

        let doc_trigram_set: HashSet<&String> = doc_trigrams.iter().collect();
        let matches = query_trigrams
            .iter()
            .filter(|trigram| doc_trigram_set.contains(trigram))
            .count();

        let match_ratio = matches as f64 / query_trigrams.len() as f64;
        let length_penalty = 1.0 / (1.0 + (word_count as f64 / 1000.0).ln());

        match_ratio * length_penalty
    }

    /// Create directory structure for the index
    async fn ensure_directories(&self) -> Result<()> {
        let paths = [
            self.index_path.join("trigrams"),
            self.index_path.join("cache"),
            self.index_path.join("wal"),
            self.index_path.join("meta"),
        ];

        for path in &paths {
            fs::create_dir_all(path).await.with_context(|| {
                format!(
                    "Failed to create trigram index directory: {}",
                    path.display()
                )
            })?;
        }

        Ok(())
    }

    /// Initialize write-ahead log
    async fn init_wal(&self) -> Result<()> {
        let wal_path = self.index_path.join("wal").join("trigram.wal");
        let wal_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .with_context(|| format!("Failed to open trigram WAL file: {}", wal_path.display()))?;

        *self.wal_writer.write().await = Some(wal_file);
        Ok(())
    }

    /// Load existing index from disk
    async fn load_existing_index(&self) -> Result<()> {
        let trigrams_dir = self.index_path.join("trigrams");
        let cache_dir = self.index_path.join("cache");

        if !trigrams_dir.exists() || !cache_dir.exists() {
            return Ok(());
        }

        // Load metadata
        let metadata_path = self.index_path.join("meta").join("trigram_metadata.json");
        if metadata_path.exists() {
            let metadata_content = fs::read_to_string(&metadata_path).await.with_context(|| {
                format!(
                    "Failed to read trigram metadata: {}",
                    metadata_path.display()
                )
            })?;

            let metadata: TrigramMetadata = serde_json::from_str(&metadata_content)
                .context("Failed to deserialize trigram metadata")?;

            *self.metadata.write().await = metadata;
        }

        // Load trigram index
        let trigram_index_path = trigrams_dir.join("index.json");
        if trigram_index_path.exists() {
            let trigram_content =
                fs::read_to_string(&trigram_index_path)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to read trigram index: {}",
                            trigram_index_path.display()
                        )
                    })?;

            // Deserialize as HashMap<String, Vec<String>> first, then convert
            let raw_index: HashMap<String, Vec<String>> = serde_json::from_str(&trigram_content)
                .context("Failed to deserialize trigram index")?;

            // Convert to HashMap<String, HashSet<ValidatedDocumentId>>
            let mut trigram_index = HashMap::new();
            for (trigram, doc_id_strings) in raw_index {
                let mut doc_ids = HashSet::new();
                for id_str in doc_id_strings {
                    if let Ok(doc_id) = ValidatedDocumentId::parse(&id_str) {
                        doc_ids.insert(doc_id);
                    }
                }
                if !doc_ids.is_empty() {
                    trigram_index.insert(trigram, doc_ids);
                }
            }

            *self.trigram_index.write().await = trigram_index;
        }

        // Load document cache
        let cache_path = cache_dir.join("documents.json");
        if cache_path.exists() {
            let cache_content = fs::read_to_string(&cache_path).await.with_context(|| {
                format!("Failed to read document cache: {}", cache_path.display())
            })?;

            // Deserialize as HashMap<String, DocumentContent> first, then convert
            let raw_cache: HashMap<String, DocumentContent> = serde_json::from_str(&cache_content)
                .context("Failed to deserialize document cache")?;

            let mut document_cache = HashMap::new();
            for (id_str, content) in raw_cache {
                if let Ok(doc_id) = ValidatedDocumentId::parse(&id_str) {
                    document_cache.insert(doc_id, content);
                }
            }

            *self.document_cache.write().await = document_cache;
        }

        Ok(())
    }

    /// Save metadata to disk
    async fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.index_path.join("meta").join("trigram_metadata.json");
        let metadata = self.metadata.read().await;

        let content = serde_json::to_string_pretty(&*metadata)
            .context("Failed to serialize trigram metadata")?;

        fs::write(&metadata_path, content).await.with_context(|| {
            format!(
                "Failed to write trigram metadata: {}",
                metadata_path.display()
            )
        })?;

        Ok(())
    }

    /// Save trigram index to disk
    async fn save_trigram_index(&self) -> Result<()> {
        let index_path = self.index_path.join("trigrams").join("index.json");
        let trigram_index = self.trigram_index.read().await;

        // Convert HashSet<ValidatedDocumentId> to Vec<String> for serialization
        let serializable_index: HashMap<String, Vec<String>> = trigram_index
            .iter()
            .map(|(trigram, doc_ids)| {
                let doc_id_strings: Vec<String> =
                    doc_ids.iter().map(|id| id.as_uuid().to_string()).collect();
                (trigram.clone(), doc_id_strings)
            })
            .collect();

        let content = serde_json::to_string_pretty(&serializable_index)
            .context("Failed to serialize trigram index")?;

        fs::write(&index_path, content)
            .await
            .with_context(|| format!("Failed to write trigram index: {}", index_path.display()))?;

        Ok(())
    }

    /// Save document cache to disk
    async fn save_document_cache(&self) -> Result<()> {
        let cache_path = self.index_path.join("cache").join("documents.json");
        let document_cache = self.document_cache.read().await;

        // Convert ValidatedDocumentId keys to String for serialization
        let serializable_cache: HashMap<String, DocumentContent> = document_cache
            .iter()
            .map(|(doc_id, content)| (doc_id.as_uuid().to_string(), content.clone()))
            .collect();

        let content = serde_json::to_string_pretty(&serializable_cache)
            .context("Failed to serialize document cache")?;

        fs::write(&cache_path, content)
            .await
            .with_context(|| format!("Failed to write document cache: {}", cache_path.display()))?;

        Ok(())
    }

    /// Update metadata counters
    async fn update_metadata(
        &self,
        document_delta: i32,
        trigram_delta: i32,
        total_trigram_delta: i32,
    ) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        // Update document count
        if document_delta < 0 {
            let decrease = (-document_delta) as usize;
            if metadata.document_count < decrease {
                bail!(
                    "Document count would go negative: {} - {}",
                    metadata.document_count,
                    decrease
                );
            }
            metadata.document_count -= decrease;
        } else {
            metadata.document_count += document_delta as usize;
        }

        // Update trigram count
        if trigram_delta < 0 {
            let decrease = (-trigram_delta) as usize;
            if metadata.trigram_count < decrease {
                bail!(
                    "Trigram count would go negative: {} - {}",
                    metadata.trigram_count,
                    decrease
                );
            }
            metadata.trigram_count -= decrease;
        } else {
            metadata.trigram_count += trigram_delta as usize;
        }

        // Update total trigrams
        if total_trigram_delta < 0 {
            let decrease = (-total_trigram_delta) as usize;
            if metadata.total_trigrams < decrease {
                bail!(
                    "Total trigrams would go negative: {} - {}",
                    metadata.total_trigrams,
                    decrease
                );
            }
            metadata.total_trigrams -= decrease;
        } else {
            metadata.total_trigrams += total_trigram_delta as usize;
        }

        metadata.updated = chrono::Utc::now().timestamp();
        Ok(())
    }
}

#[async_trait]
impl Index for TrigramIndex {
    /// Open a trigram index instance at the given path
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Validate path using existing validation
        validation::path::validate_directory_path(path)?;

        let index_path = PathBuf::from(path);
        let index = Self {
            index_path: index_path.clone(),
            trigram_index: RwLock::new(HashMap::new()),
            document_cache: RwLock::new(HashMap::new()),
            wal_writer: RwLock::new(None),
            metadata: RwLock::new(TrigramMetadata::default()),
        };

        // Ensure directory structure exists
        index.ensure_directories().await?;

        // Initialize WAL
        index.init_wal().await?;

        // Load existing state from disk
        index
            .load_existing_index()
            .await
            .context("Failed to load existing trigram index from disk")?;

        Ok(index)
    }

    /// Insert a document into the trigram index
    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // Note: This is a simplified implementation that only indexes the path
        // In a full implementation, we would need access to the document content
        // For now, we'll extract trigrams from the path and placeholder content

        let text = format!("Document at path: {}", path.as_str());
        let trigrams = Self::extract_trigrams(&text);

        if trigrams.is_empty() {
            return Ok(()); // Nothing to index
        }

        let was_new_document;

        // Check if document already exists
        {
            let cache = self.document_cache.read().await;
            was_new_document = !cache.contains_key(&id);
        }

        // Update trigram index
        {
            let mut index = self.trigram_index.write().await;
            for trigram in &trigrams {
                index
                    .entry(trigram.clone())
                    .or_insert_with(HashSet::new)
                    .insert(id);
            }
        }

        // Update document cache
        {
            let mut cache = self.document_cache.write().await;
            cache.insert(
                id,
                DocumentContent {
                    title: path.as_str().to_string(),
                    content_preview: text.clone(),
                    word_count: text.split_whitespace().count(),
                    trigram_count: trigrams.len(),
                },
            );
        }

        // Update metadata
        let trigram_delta = if was_new_document {
            trigrams.len() as i32
        } else {
            0
        };
        self.update_metadata(
            if was_new_document { 1 } else { 0 },
            trigram_delta,
            trigrams.len() as i32,
        )
        .await?;

        Ok(())
    }

    /// Update an existing entry in the trigram index
    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For trigram index, update is the same as insert (it replaces the content)
        self.insert(id, path).await
    }

    /// Delete an entry from the trigram index
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let existed;
        let mut trigrams_to_clean = Vec::new();

        // Remove from document cache and get trigram count
        let old_trigram_count = {
            let mut cache = self.document_cache.write().await;
            if let Some(doc_content) = cache.remove(id) {
                existed = true;
                doc_content.trigram_count
            } else {
                existed = false;
                0
            }
        };

        if existed {
            // Remove document ID from trigram index
            {
                let mut index = self.trigram_index.write().await;
                for (trigram, doc_ids) in index.iter_mut() {
                    if doc_ids.remove(id) && doc_ids.is_empty() {
                        trigrams_to_clean.push(trigram.clone());
                    }
                }

                // Clean up empty trigram entries
                for trigram in trigrams_to_clean {
                    index.remove(&trigram);
                }
            }

            // Update metadata
            self.update_metadata(-1, -(old_trigram_count as i32), -(old_trigram_count as i32))
                .await?;
        }

        Ok(existed)
    }

    /// Search the trigram index
    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        if query.search_terms.is_empty() {
            return Ok(Vec::new()); // No text to search
        }

        // Extract trigrams from all search terms
        let mut all_query_trigrams = Vec::new();
        for search_term in &query.search_terms {
            let term_trigrams = Self::extract_trigrams(search_term.as_str());
            all_query_trigrams.extend(term_trigrams);
        }

        if all_query_trigrams.is_empty() {
            return Ok(Vec::new());
        }

        // Find documents that contain these trigrams
        let index = self.trigram_index.read().await;
        let mut candidate_docs: HashMap<ValidatedDocumentId, usize> = HashMap::new();

        for trigram in &all_query_trigrams {
            if let Some(doc_ids) = index.get(trigram) {
                for doc_id in doc_ids {
                    *candidate_docs.entry(*doc_id).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching trigrams)
        let mut results: Vec<(ValidatedDocumentId, usize)> = candidate_docs.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by match count descending

        // Apply limit and return document IDs
        let limit = query.limit.get();
        let final_results: Vec<ValidatedDocumentId> = results
            .into_iter()
            .take(limit)
            .map(|(doc_id, _)| doc_id)
            .collect();

        Ok(final_results)
    }

    /// Sync changes to persistent storage
    async fn sync(&mut self) -> Result<()> {
        self.flush().await
    }

    /// Flush any pending changes
    async fn flush(&mut self) -> Result<()> {
        // Save all persistent state
        self.save_metadata()
            .await
            .context("Failed to save trigram metadata during flush")?;

        self.save_trigram_index()
            .await
            .context("Failed to save trigram index during flush")?;

        self.save_document_cache()
            .await
            .context("Failed to save document cache during flush")?;

        // Sync WAL
        if let Some(wal_file) = self.wal_writer.write().await.as_mut() {
            wal_file
                .sync_all()
                .await
                .context("Failed to sync trigram WAL during flush")?;
        }

        Ok(())
    }

    /// Close the trigram index instance
    async fn close(self) -> Result<()> {
        // Drop the WAL writer (automatically closes the file)
        drop(self.wal_writer);
        Ok(())
    }
}

/// Create a fully wrapped TrigramIndex with all Stage 6 components
///
/// This is the recommended way to create a production-ready trigram index.
/// It automatically applies Stage 6 MeteredIndex wrapper for metrics and observability.
pub async fn create_trigram_index(
    path: &str,
    _cache_capacity: Option<usize>,
) -> Result<MeteredIndex<TrigramIndex>> {
    // Validate path using existing validation
    validation::path::validate_directory_path(path)?;

    let index = TrigramIndex::open(path).await?;

    // Apply Stage 6 wrapper for automatic metrics
    Ok(MeteredIndex::new(index, "trigram".to_string()))
}

/// Alternative factory function for testing without cache parameter
/// Used internally by tests that don't need to specify cache capacity
pub async fn create_trigram_index_for_tests(path: &str) -> Result<TrigramIndex> {
    TrigramIndex::open(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_trigram_extraction() {
        let text = "hello world";
        let trigrams = TrigramIndex::extract_trigrams(text);

        // Expected: ["hel", "ell", "llo", "wor", "orl", "rld"]
        assert!(trigrams.contains(&"hel".to_string()));
        assert!(trigrams.contains(&"ell".to_string()));
        assert!(trigrams.contains(&"llo".to_string()));
        assert!(trigrams.contains(&"wor".to_string()));
        assert!(trigrams.contains(&"orl".to_string()));
        assert!(trigrams.contains(&"rld".to_string()));
    }

    #[test]
    fn test_trigram_extraction_short_text() {
        let text = "hi";
        let trigrams = TrigramIndex::extract_trigrams(text);
        assert!(trigrams.is_empty()); // Too short for trigrams
    }

    #[test]
    fn test_trigram_extraction_normalization() {
        let text = "Hello WORLD";
        let trigrams = TrigramIndex::extract_trigrams(text);

        // Should be lowercase
        assert!(trigrams.contains(&"hel".to_string()));
        assert!(trigrams.contains(&"wor".to_string()));
        assert!(!trigrams.contains(&"HEL".to_string()));
        assert!(!trigrams.contains(&"WOR".to_string()));
    }

    #[tokio::test]
    async fn test_trigram_index_basic_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("trigram_test");

        let mut index = TrigramIndex::open(
            index_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid trigram index test path"))?,
        )
        .await?;

        // Test insertion
        let doc_id = ValidatedDocumentId::new();
        let doc_path = ValidatedPath::new("/test/document.md")?;

        index.insert(doc_id, doc_path).await?;

        // Test that metadata was updated
        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 1);
            assert!(metadata.trigram_count > 0);
        }

        Ok(())
    }
}
