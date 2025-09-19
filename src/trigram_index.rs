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
    /// State tracking for lazy loading with error protection
    load_state: RwLock<LoadState>,
}

/// Cached document content for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentContent {
    title: String,
    content_preview: String,    // First 500 chars for snippets
    full_trigrams: Vec<String>, // All trigrams from the document for accurate scoring
    word_count: usize,
    trigram_count: usize,
    /// Pre-computed trigram frequency map for faster relevance scoring
    /// Memory trade-off: ~O(unique_trigrams) per document for O(1) scoring
    /// Typical overhead: 50-200 entries per document (~2-8KB each)
    /// Performance benefit: Eliminates O(n) HashMap rebuilding per query
    trigram_freq: HashMap<String, usize>,
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

/// Loading state for lazy loading with error protection
#[derive(Debug, Clone)]
enum LoadState {
    /// Index has not been loaded yet
    NotLoaded,
    /// Index is currently being loaded (prevents concurrent loads)
    Loading,
    /// Index has been successfully loaded
    Loaded,
    /// Index failed to load with error message (prevents retry storms)
    Failed(String),
}

impl TrigramIndex {
    /// Extract trigrams from text
    ///
    /// Converts text to lowercase and extracts all 3-character sequences.
    /// Special handling for unicode characters and normalization.
    /// Returns ALL trigrams including duplicates to preserve frequency information.
    pub fn extract_trigrams(text: &str) -> Vec<String> {
        let normalized = text.to_lowercase();
        let chars: Vec<char> = normalized.chars().collect();

        if chars.len() < 3 {
            return Vec::new();
        }

        let mut trigrams = Vec::with_capacity(chars.len() - 2);
        for i in 0..=(chars.len() - 3) {
            let trigram: String = chars[i..i + 3].iter().collect();

            // Skip trigrams that are only whitespace or punctuation
            if trigram.chars().any(|c| c.is_alphanumeric()) {
                trigrams.push(trigram);
            }
        }

        // Return all trigrams including duplicates for frequency analysis
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

        // Count trigram frequency in the document
        let mut doc_trigram_freq: HashMap<&String, usize> = HashMap::new();
        for trigram in doc_trigrams {
            *doc_trigram_freq.entry(trigram).or_insert(0) += 1;
        }

        // Calculate match statistics
        let mut total_matches = 0;
        let mut unique_matches = 0;

        for query_trigram in query_trigrams {
            if let Some(&freq) = doc_trigram_freq.get(query_trigram) {
                unique_matches += 1;
                total_matches += freq;
            }
        }

        if unique_matches == 0 {
            return 0.0;
        }

        // Calculate match coverage (what % of query trigrams were found)
        let coverage = unique_matches as f64 / query_trigrams.len() as f64;

        // Calculate term frequency score (how many times query terms appear)
        // More occurrences = higher relevance
        let frequency_score = total_matches as f64;

        // Calculate document relevance
        // Balance between high frequency and reasonable document length
        // We don't want to penalize longer documents too much if they have many matches
        let length_factor = if word_count > 0 {
            // Use logarithmic scaling to reduce the impact of document length
            1.0 / (1.0 + (word_count as f64 / 100.0).ln())
        } else {
            1.0
        };

        // Final score combines:
        // - Coverage: How many of the query trigrams were found (0-1)
        // - Frequency: Raw count of matching trigrams
        // - Length factor: Slight preference for focused documents
        //
        // The frequency component is most important for differentiation
        (coverage * 10.0) + frequency_score + (length_factor * 5.0)
    }

    /// Optimized relevance score calculation using pre-computed frequency map
    pub fn calculate_relevance_score_optimized(
        query_trigrams: &[String],
        doc_trigram_freq: &HashMap<String, usize>,
        word_count: usize,
    ) -> f64 {
        if query_trigrams.is_empty() || doc_trigram_freq.is_empty() {
            return 0.0;
        }

        // Calculate match statistics using pre-computed frequency map
        let mut total_matches = 0;
        let mut unique_matches = 0;

        for query_trigram in query_trigrams {
            if let Some(&freq) = doc_trigram_freq.get(query_trigram) {
                unique_matches += 1;
                total_matches += freq;
            }
        }

        if unique_matches == 0 {
            return 0.0;
        }

        // Calculate match coverage (what % of query trigrams were found)
        let coverage = unique_matches as f64 / query_trigrams.len() as f64;

        // Calculate term frequency score (how many times query terms appear)
        let frequency_score = total_matches as f64;

        // Calculate document relevance with logarithmic length scaling
        let length_factor = if word_count > 0 {
            1.0 / (1.0 + (word_count as f64 / 100.0).ln())
        } else {
            1.0
        };

        // Final score combines coverage, frequency, and length factor
        (coverage * 10.0) + frequency_score + (length_factor * 5.0)
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

    /// Ensure index is loaded (lazy loading with error protection)
    async fn ensure_loaded(&self) -> Result<()> {
        // Fast path: check if already loaded or failed
        {
            let state = self.load_state.read().await;
            match &*state {
                LoadState::Loaded => return Ok(()),
                LoadState::Failed(err) => {
                    return Err(anyhow::anyhow!(
                        "Index previously failed to load and retry is disabled: {}",
                        err
                    ));
                }
                LoadState::Loading => {
                    // Another thread is loading, wait briefly then check again
                    drop(state);
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let state = self.load_state.read().await;
                    match &*state {
                        LoadState::Loaded => return Ok(()),
                        LoadState::Failed(err) => {
                            return Err(anyhow::anyhow!(
                                "Index failed to load during concurrent attempt: {}",
                                err
                            ));
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Index loading is taking too long, another thread may have failed"
                            ));
                        }
                    }
                }
                LoadState::NotLoaded => {
                    // Continue to loading logic below
                }
            }
        }

        // Acquire write lock to begin loading
        let mut state = self.load_state.write().await;

        // Double-check after acquiring write lock (another thread might have loaded)
        match &*state {
            LoadState::Loaded => return Ok(()),
            LoadState::Failed(err) => {
                return Err(anyhow::anyhow!("Index previously failed to load: {}", err));
            }
            LoadState::Loading => {
                // This should be rare due to our earlier check, but handle gracefully
                return Err(anyhow::anyhow!(
                    "Concurrent loading detected - this should not happen"
                ));
            }
            LoadState::NotLoaded => {
                // Good to proceed with loading
            }
        }

        // Set loading state
        *state = LoadState::Loading;
        drop(state); // Release write lock during loading

        // Load the index with error capture
        tracing::info!("Lazy loading trigram index on first access");
        let start = std::time::Instant::now();

        // Log memory pressure warning for large indices
        tracing::warn!("Loading large trigram index may consume significant memory (~132MB+)");

        let load_result = self.load_existing_index().await;

        // Update state based on result
        let mut state = self.load_state.write().await;
        match load_result {
            Ok(()) => {
                *state = LoadState::Loaded;
                let elapsed = start.elapsed();
                tracing::info!("Trigram index loaded successfully in {:?}", elapsed);

                // Log performance metrics for monitoring
                if elapsed.as_millis() > 1000 {
                    tracing::warn!(
                        "Trigram index loading took {}ms - consider memory optimization",
                        elapsed.as_millis()
                    );
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                *state = LoadState::Failed(error_msg.clone());
                tracing::error!("Failed to load trigram index: {}", error_msg);
                Err(e.context("Failed to lazy load trigram index"))
            }
        }
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

            // Try to deserialize with the new format, handle old format gracefully
            let mut document_cache = HashMap::new();

            // Try to parse as a JSON value first to handle backwards compatibility
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&cache_content) {
                if let Some(cache_obj) = json_value.as_object() {
                    for (id_str, doc_value) in cache_obj {
                        if let Ok(doc_id) = ValidatedDocumentId::parse(id_str) {
                            // Check if this is the old format (missing full_trigrams)
                            if let Some(doc_obj) = doc_value.as_object() {
                                let has_full_trigrams = doc_obj.contains_key("full_trigrams");

                                if has_full_trigrams {
                                    // New format - deserialize normally
                                    if let Ok(content) =
                                        serde_json::from_value::<DocumentContent>(doc_value.clone())
                                    {
                                        document_cache.insert(doc_id, content);
                                    }
                                } else {
                                    // Old format - need to reconstruct trigrams from preview
                                    let title = doc_obj
                                        .get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let content_preview = doc_obj
                                        .get("content_preview")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let word_count = doc_obj
                                        .get("word_count")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        as usize;
                                    let trigram_count = doc_obj
                                        .get("trigram_count")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        as usize;

                                    // Reconstruct trigrams from title and preview
                                    let doc_text = format!("{} {}", title, content_preview);
                                    let full_trigrams = Self::extract_trigrams(&doc_text);

                                    // Pre-compute trigram frequency map for performance
                                    let mut trigram_freq = HashMap::new();
                                    for trigram in &full_trigrams {
                                        *trigram_freq.entry(trigram.clone()).or_insert(0) += 1;
                                    }

                                    document_cache.insert(
                                        doc_id,
                                        DocumentContent {
                                            title,
                                            content_preview,
                                            full_trigrams,
                                            word_count,
                                            trigram_count,
                                            trigram_freq,
                                        },
                                    );
                                }
                            }
                        }
                    }
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
        // Validate path for internal storage (allows absolute paths)
        validation::path::validate_storage_directory_path(path)?;

        let index_path = PathBuf::from(path);
        let index = Self {
            index_path: index_path.clone(),
            trigram_index: RwLock::new(HashMap::new()),
            document_cache: RwLock::new(HashMap::new()),
            wal_writer: RwLock::new(None),
            metadata: RwLock::new(TrigramMetadata::default()),
            load_state: RwLock::new(LoadState::NotLoaded),
        };

        // Ensure directory structure exists
        index.ensure_directories().await?;

        // Initialize WAL
        index.init_wal().await?;

        // Skip loading existing index - will be loaded lazily on first search
        // This dramatically improves cold start performance from 2s to <100ms
        tracing::debug!("Skipping index load during initialization for faster cold start");

        Ok(index)
    }

    /// Insert a document into the trigram index
    async fn insert(&mut self, _id: ValidatedDocumentId, _path: ValidatedPath) -> Result<()> {
        // Trigram index requires document content to function properly.
        // The insert() method from the Index trait doesn't provide content,
        // so we return an error directing callers to use insert_with_content() instead.
        bail!(
            "Trigram index requires document content. Use insert_with_content() instead of insert()"
        )
    }

    /// Update an existing entry in the trigram index
    async fn update(&mut self, _id: ValidatedDocumentId, _path: ValidatedPath) -> Result<()> {
        // Trigram index requires document content to function properly.
        // The update() method from the Index trait doesn't provide content,
        // so we return an error directing callers to use update_with_content() or insert_with_content() instead.
        bail!(
            "Trigram index requires document content. Use update_with_content() or insert_with_content() instead of update()"
        )
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
        // Ensure index is loaded before searching (lazy loading)
        self.ensure_loaded().await?;

        if query.search_terms.is_empty() {
            // Empty search query for trigram index (content search) returns no results
            // This is different from primary index which handles path-based wildcard queries
            // Content searches require actual search terms
            return Ok(Vec::new());
        }

        // Extract trigrams from all search terms
        // Estimate ~10 trigrams per search term on average, with reasonable bounds
        let estimated_capacity = (query.search_terms.len() * 10).clamp(16, 1000);
        let mut all_query_trigrams = Vec::with_capacity(estimated_capacity);
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
        let mut total_trigram_hits = 0;

        for trigram in &all_query_trigrams {
            if let Some(doc_ids) = index.get(trigram) {
                for doc_id in doc_ids {
                    *candidate_docs.entry(*doc_id).or_insert(0) += 1;
                }
                total_trigram_hits += doc_ids.len();
            }
        }

        // If no trigrams found matches at all, return empty results early
        // This handles cases where query contains completely unknown patterns
        if candidate_docs.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate minimum match threshold with improved precision
        // This prevents false positives from random trigram matches
        debug_assert!(
            !all_query_trigrams.is_empty(),
            "Should not reach threshold calculation with empty trigrams"
        );
        let min_match_threshold = if all_query_trigrams.len() <= 3 {
            // For very short queries (1-3 trigrams), require all trigrams to match
            all_query_trigrams.len()
        } else if all_query_trigrams.len() <= 6 {
            // For short queries (4-6 trigrams), require 80% match to reduce false positives
            std::cmp::max(
                all_query_trigrams.len() * 8 / 10,
                all_query_trigrams.len() - 1,
            )
        } else {
            // For longer queries, require at least 60% of trigrams to match
            // This is more strict than the previous 30% to improve precision
            std::cmp::max(3, (all_query_trigrams.len() * 6) / 10)
        };

        // Filter by minimum threshold first
        let mut filtered_candidates: Vec<ValidatedDocumentId> = candidate_docs
            .iter()
            .filter(|(_, match_count)| **match_count >= min_match_threshold)
            .map(|(doc_id, _)| *doc_id)
            .collect();

        // Optional fallback (feature-gated). Default: no fallback to avoid false positives.
        if cfg!(feature = "aggressive-trigram-thresholds")
            && filtered_candidates.is_empty()
            && !candidate_docs.is_empty()
        {
            tracing::debug!(
                "Strict threshold {} eliminated all {} candidates, applying fallback for query with {} trigrams",
                min_match_threshold, candidate_docs.len(), all_query_trigrams.len()
            );

            // Progressive fallback: try increasingly relaxed thresholds
            let fallback_thresholds = if all_query_trigrams.len() <= 3 {
                // For very short queries, try 2/3 then 1/3 trigrams
                vec![all_query_trigrams.len().saturating_sub(1), 1]
            } else if all_query_trigrams.len() <= 6 {
                // For medium queries, try 50% then 33% then minimum of 2
                vec![
                    all_query_trigrams.len() / 2,
                    all_query_trigrams.len() / 3,
                    std::cmp::min(2, all_query_trigrams.len()),
                ]
            } else {
                // For long queries, try 40% then 30% then minimum of 3
                vec![
                    (all_query_trigrams.len() * 4) / 10,
                    all_query_trigrams.len() / 3,
                    std::cmp::min(3, all_query_trigrams.len()),
                ]
            };

            for fallback_threshold in fallback_thresholds {
                if fallback_threshold > 0 && fallback_threshold < min_match_threshold {
                    filtered_candidates = candidate_docs
                        .iter()
                        .filter(|(_, match_count)| **match_count >= fallback_threshold)
                        .map(|(doc_id, _)| *doc_id)
                        .collect();

                    if !filtered_candidates.is_empty() {
                        tracing::debug!(
                            "Fallback threshold {} found {} candidates",
                            fallback_threshold,
                            filtered_candidates.len()
                        );
                        break;
                    }
                }
            }

            // If we still have no results, take the top candidates by trigram match count
            if filtered_candidates.is_empty() {
                let mut sorted_candidates: Vec<_> = candidate_docs.iter().collect();
                sorted_candidates.sort_by(|a, b| b.1.cmp(a.1));
                filtered_candidates = sorted_candidates
                    .into_iter()
                    .take(std::cmp::min(5, candidate_docs.len())) // Take top 5 matches
                    .map(|(doc_id, _)| *doc_id)
                    .collect();

                if !filtered_candidates.is_empty() {
                    tracing::debug!(
                        "Final fallback: returning top {} candidates by trigram matches",
                        filtered_candidates.len()
                    );
                }
            }
        }

        // Calculate relevance scores for each candidate document using optimized scoring
        let document_cache = self.document_cache.read().await;
        let mut scored_results: Vec<(ValidatedDocumentId, f64)> =
            Vec::with_capacity(filtered_candidates.len());

        for doc_id in filtered_candidates {
            if let Some(doc_content) = document_cache.get(&doc_id) {
                // Use the optimized scoring with pre-computed frequency map
                let score = Self::calculate_relevance_score_optimized(
                    &all_query_trigrams,
                    &doc_content.trigram_freq,
                    doc_content.word_count,
                );

                // Debug logging to understand scoring
                #[cfg(test)]
                {
                    eprintln!(
                        "Doc {:?}: score={:.4}, word_count={}, trigram_count={}",
                        doc_id, score, doc_content.word_count, doc_content.trigram_count
                    );
                }

                scored_results.push((doc_id, score));
            }
        }

        // Use partial sort for better performance when we only need top-K results
        let limit = query.limit.get();
        if scored_results.len() > limit {
            // Partial sort: only sort the top 'limit' elements
            scored_results.select_nth_unstable_by(limit - 1, |a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.as_uuid().cmp(&b.0.as_uuid()))
            });
            scored_results.truncate(limit);
        }

        // Sort the top results for deterministic ordering
        scored_results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    // For equal scores, sort by document ID for deterministic ordering
                    a.0.as_uuid().cmp(&b.0.as_uuid())
                })
        });

        // Return the top results
        let final_results: Vec<ValidatedDocumentId> = scored_results
            .into_iter()
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

    /// Insert with document content for proper trigram indexing
    ///
    /// This method extracts trigrams from the actual document content,
    /// providing the full-text search capability that trigram indices are designed for.
    async fn insert_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // Convert content to string for trigram extraction
        let content_str = String::from_utf8_lossy(content);

        // Create a Document-like structure for text extraction
        // We need title and content for comprehensive indexing
        let title = path
            .as_str()
            .split('/')
            .next_back()
            .unwrap_or(path.as_str());
        let searchable_text = format!("{} {}", title, content_str);

        // Extract trigrams from the full content
        let trigrams = Self::extract_trigrams(&searchable_text);

        if trigrams.is_empty() {
            return Ok(()); // Nothing to index
        }

        let was_new_document;

        // Check if document already exists
        {
            let cache = self.document_cache.read().await;
            was_new_document = !cache.contains_key(&id);
        }

        // Update trigram index (use unique trigrams for the index)
        {
            let mut index = self.trigram_index.write().await;
            let unique_trigrams: HashSet<String> = trigrams.iter().cloned().collect();
            for trigram in unique_trigrams {
                index.entry(trigram).or_insert_with(HashSet::new).insert(id);
            }
        }

        // Update document cache with actual content
        {
            let mut cache = self.document_cache.write().await;
            let content_preview = if content_str.len() > 500 {
                // Truncate content preview to ~500 characters (not bytes) to avoid
                // splitting multi-byte UTF-8 sequences
                let truncate_at = content_str
                    .char_indices()
                    .nth(497) // Get the 497th character position
                    .map(|(i, _)| i)
                    .unwrap_or(content_str.len());
                format!("{}...", &content_str[..truncate_at])
            } else {
                content_str.to_string()
            };

            // Pre-compute trigram frequency map for performance
            let mut trigram_freq = HashMap::new();
            for trigram in &trigrams {
                *trigram_freq.entry(trigram.clone()).or_insert(0) += 1;
            }

            cache.insert(
                id,
                DocumentContent {
                    title: title.to_string(),
                    content_preview,
                    full_trigrams: trigrams.clone(),
                    word_count: searchable_text.split_whitespace().count(),
                    trigram_count: trigrams.len(),
                    trigram_freq,
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

    /// Update with document content for proper trigram indexing
    ///
    /// This method updates the trigram index with new document content,
    /// removing old trigrams and adding new ones.
    async fn update_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // First delete the existing entry
        self.delete(&id).await?;

        // Then insert with new content
        self.insert_with_content(id, path, content).await
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
    // Validate path for internal storage (allows absolute paths)
    validation::path::validate_storage_directory_path(path)?;

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
    use uuid::Uuid;

    #[test]
    fn test_trigram_extraction() {
        let text = "hello world";
        let trigrams = TrigramIndex::extract_trigrams(text);

        // Expected trigrams with proper frequency
        let unique_trigrams: HashSet<String> = trigrams.into_iter().collect();
        assert!(unique_trigrams.contains("hel"));
        assert!(unique_trigrams.contains("ell"));
        assert!(unique_trigrams.contains("llo"));
        assert!(unique_trigrams.contains("wor"));
        assert!(unique_trigrams.contains("orl"));
        assert!(unique_trigrams.contains("rld"));
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
        let unique_trigrams: HashSet<String> = trigrams.into_iter().collect();

        // Should be lowercase
        assert!(unique_trigrams.contains("hel"));
        assert!(unique_trigrams.contains("wor"));
        assert!(!unique_trigrams.contains("HEL"));
        assert!(!unique_trigrams.contains("WOR"));
    }

    #[tokio::test]
    async fn test_trigram_index_basic_operations() -> Result<()> {
        let test_dir = format!("test_data/trigram_basic_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let mut index = TrigramIndex::open(&test_dir).await?;

        // Test insertion with content (required for trigram index)
        let doc_id = ValidatedDocumentId::new();
        let doc_path = ValidatedPath::new("test/document.md")?;
        let content = b"Test document with searchable content for trigram indexing";

        index.insert_with_content(doc_id, doc_path, content).await?;

        // Test that metadata was updated
        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 1);
            assert!(metadata.trigram_count > 0);
        }

        // Clean up test directory
        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Performance regression test - run with --ignored for performance validation"]
    async fn test_search_performance_regression() -> Result<()> {
        use std::time::Instant;

        let test_dir = format!("test_data/trigram_perf_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let mut index = TrigramIndex::open(&test_dir).await?;

        // Insert multiple documents to simulate realistic load
        for i in 0..50 {
            let doc_id = ValidatedDocumentId::new();
            let doc_path = ValidatedPath::new(format!("test/document_{}.rs", i))?;
            let content = format!(
                "Storage functionality implementation for document {}. \
                This contains trigram indexing patterns, search algorithms, \
                and performance optimizations for fast retrieval. \
                Additional content with storage, indexing, search terms \
                to create realistic trigram distributions.",
                i
            );

            index
                .insert_with_content(doc_id, doc_path, content.as_bytes())
                .await?;
        }

        // Test search performance with realistic query
        let query = crate::contracts::Query {
            search_terms: vec![crate::types::ValidatedSearchQuery::new("storage", 1)?],
            limit: crate::types::ValidatedLimit::new(10, 100_000)?,
            ..Default::default()
        };

        // Warm up (first query may be slower due to cold caches)
        let _ = index.search(&query).await?;

        // Measure actual performance
        let start = Instant::now();
        let results = index.search(&query).await?;
        let duration = start.elapsed();

        // Performance regression check: must stay under 10ms threshold
        assert!(
            duration.as_millis() < 10,
            "Search performance regression detected: {}ms > 10ms threshold. \
            Results count: {}",
            duration.as_millis(),
            results.len()
        );

        // Ensure we're actually finding results
        assert!(!results.is_empty(), "Search should find matching documents");

        // Clean up test directory
        let _ = std::fs::remove_dir_all(&test_dir);

        println!(
            "✅ Search performance: {}ms (target: <10ms)",
            duration.as_millis()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_unicode_content_preview_truncation() -> Result<()> {
        let test_dir = format!("test_data/trigram_unicode_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let mut index = TrigramIndex::open(&test_dir).await?;

        // Create a test document with Unicode content that has multi-byte characters
        // around the 497 character mark
        let mut long_content = String::new();

        // Add ASCII content first
        for _ in 0..490 {
            long_content.push('a');
        }

        // Add multi-byte Unicode characters around the truncation point
        long_content.push_str("中文字符测试"); // Chinese characters
        long_content.push_str("🚀📝🎯🔥💡"); // Emojis
        long_content.push_str("русский текст"); // Cyrillic

        // Add more content to ensure we exceed 500 bytes
        for _ in 0..100 {
            long_content.push('b');
        }

        // Insert document with content
        let doc_id = ValidatedDocumentId::new();
        let doc_path = ValidatedPath::new("test/unicode.md")?;
        index
            .insert_with_content(doc_id, doc_path, long_content.as_bytes())
            .await?;

        // Verify that insertion succeeded without panic
        let cache = index.document_cache.read().await;
        let doc_content = cache.get(&doc_id).expect("Document should be cached");

        // Check that the preview was truncated properly
        assert!(doc_content.content_preview.ends_with("..."));

        // Ensure the preview doesn't cut in the middle of a Unicode character
        // by verifying it's valid UTF-8
        assert!(doc_content.content_preview.is_char_boundary(
            doc_content.content_preview.len() - 3 // Before the "..."
        ));

        // Clean up test directory
        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }
}
