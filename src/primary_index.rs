// Primary Index Implementation - Stage 2: Contract-First Design
// This implements the Index trait using a file-based B+ tree structure
// Designed to work with all Stage 6 component library wrappers

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::contracts::{Index, Query};
use crate::pure::{btree, extract_all_pairs, traverse_pairs_until};
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::validation;
use crate::wrappers::MeteredIndex;

/// Primary index implementation using file-based B+ tree
///
/// This is the basic index engine that implements the Index trait.
/// It should always be used with the Stage 6 MeteredIndex wrapper for production use.
pub struct PrimaryIndex {
    /// Root directory for the index
    index_path: PathBuf,
    /// B+ tree for O(log n) operations (Document ID -> Path)
    btree_root: RwLock<btree::BTreeRoot>,
    /// Write-ahead log for durability
    wal_writer: RwLock<Option<tokio::fs::File>>,
    /// Index metadata
    metadata: RwLock<IndexMetadata>,
    /// State tracking for lazy loading with error protection
    load_state: RwLock<LoadState>,
}

/// Metadata for the primary index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexMetadata {
    version: u32,
    document_count: usize,
    created: i64,
    updated: i64,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            version: 1,
            document_count: 0,
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

/// Write-ahead log entry for durability guarantees
#[derive(Debug, Serialize, Deserialize)]
enum WalEntry {
    Insert { id: Uuid, path: String },
    Delete { id: Uuid },
}

impl PrimaryIndex {
    /// Create a new PrimaryIndex instance
    pub fn new(index_path: PathBuf, _cache_capacity: usize) -> Self {
        Self {
            index_path,
            btree_root: RwLock::new(btree::create_empty_tree()),
            wal_writer: RwLock::new(None),
            metadata: RwLock::new(IndexMetadata::default()),
            load_state: RwLock::new(LoadState::NotLoaded),
        }
    }

    /// Match a path against a wildcard pattern
    /// Supports patterns like "*.rs", "*Controller.rs", "test_*", etc.
    fn matches_wildcard_pattern(path: &str, pattern: &str) -> bool {
        // Handle pure wildcard
        if pattern == "*" {
            return true;
        }

        // Split pattern by '*' to get fixed parts
        let parts: Vec<&str> = pattern.split('*').collect();

        // Handle patterns with wildcards
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue; // Skip empty parts (from consecutive * or leading/trailing *)
            }

            // First part must match at beginning unless pattern starts with *
            if i == 0 && !pattern.starts_with('*') {
                if !path.starts_with(part) {
                    return false;
                }
                pos = part.len();
            }
            // Last part must match at end unless pattern ends with *
            else if i == parts.len() - 1 && !pattern.ends_with('*') {
                if !path.ends_with(part) {
                    return false;
                }
            }
            // Middle parts or wildcard-bounded parts can appear anywhere after current position
            else if let Some(found_pos) = path[pos..].find(part) {
                pos += found_pos + part.len();
            } else {
                return false;
            }
        }

        true
    }

    /// Create directory structure for the index
    async fn ensure_directories(&self) -> Result<()> {
        let paths = [
            self.index_path.join("data"),
            self.index_path.join("wal"),
            self.index_path.join("meta"),
        ];

        for path in &paths {
            fs::create_dir_all(path)
                .await
                .with_context(|| format!("Failed to create index directory: {}", path.display()))?;
        }

        Ok(())
    }

    /// Initialize write-ahead log
    async fn init_wal(&self) -> Result<()> {
        let wal_path = self.index_path.join("wal").join("current.wal");
        let wal_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .with_context(|| format!("Failed to open WAL file: {}", wal_path.display()))?;

        *self.wal_writer.write().await = Some(wal_file);
        Ok(())
    }

    /// Ensure index is loaded (lazy loading with error protection)
    async fn ensure_loaded(&self) -> Result<()> {
        // Fast path: wait for in-flight loads or bail on prior failures
        loop {
            let state = self.load_state.read().await;
            match &*state {
                LoadState::Loaded => return Ok(()),
                LoadState::Failed(err) => {
                    return Err(anyhow::anyhow!(
                        "Primary index previously failed to load and retry is disabled: {}",
                        err
                    ));
                }
                LoadState::Loading => {
                    drop(state);
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    continue;
                }
                LoadState::NotLoaded => break,
            }
        }

        // Acquire write lock to begin loading
        let mut state = self.load_state.write().await;

        // Double-check after acquiring write lock
        match &*state {
            LoadState::Loaded => return Ok(()),
            LoadState::Failed(err) => {
                return Err(anyhow::anyhow!(
                    "Primary index previously failed to load: {}",
                    err
                ));
            }
            LoadState::Loading => {
                return Err(anyhow::anyhow!(
                    "Concurrent loading detected in primary index - this should not happen"
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
        tracing::info!("Lazy loading primary index on first access");
        let start = std::time::Instant::now();

        // Log memory pressure information
        tracing::info!("Loading primary index (B+ tree structure)");

        let load_result = self.load_existing_index().await;

        // Update state based on result
        let mut state = self.load_state.write().await;
        match load_result {
            Ok(()) => {
                *state = LoadState::Loaded;
                let elapsed = start.elapsed();
                tracing::info!("Primary index loaded successfully in {:?}", elapsed);

                // Monitor loading performance
                if elapsed.as_millis() > 500 {
                    tracing::warn!(
                        "Primary index loading took {}ms - performance may be impacted",
                        elapsed.as_millis()
                    );
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                *state = LoadState::Failed(error_msg.clone());
                tracing::error!("Failed to load primary index: {}", error_msg);
                Err(e.context("Failed to lazy load primary index"))
            }
        }
    }

    /// Load existing index from disk
    async fn load_existing_index(&self) -> Result<()> {
        let data_dir = self.index_path.join("data");

        // Load metadata
        let metadata_path = self.index_path.join("meta").join("metadata.json");
        if metadata_path.exists() {
            let metadata_content = fs::read_to_string(&metadata_path)
                .await
                .with_context(|| format!("Failed to read metadata: {}", metadata_path.display()))?;

            let metadata: IndexMetadata = serde_json::from_str(&metadata_content)
                .context("Failed to deserialize index metadata")?;

            *self.metadata.write().await = metadata;
        }

        // Load B+ tree data
        let btree_path = data_dir.join("btree_data.json");
        if btree_path.exists() {
            let btree_content = fs::read_to_string(&btree_path).await.with_context(|| {
                format!("Failed to read B+ tree data: {}", btree_path.display())
            })?;

            // For now, rebuild tree from key-value pairs (future optimization: serialize tree structure)
            let raw_mappings: HashMap<String, String> = serde_json::from_str(&btree_content)
                .context("Failed to deserialize B+ tree data")?;

            // Rebuild B+ tree from stored key-value pairs
            let mut btree_root = btree::create_empty_tree();
            for (id_str, path_str) in raw_mappings {
                let uuid = Uuid::parse_str(&id_str)
                    .with_context(|| format!("Invalid UUID in B+ tree data: {id_str}"))?;

                let doc_id = ValidatedDocumentId::from_uuid(uuid)
                    .with_context(|| format!("Invalid document ID: {id_str}"))?;

                let validated_path = ValidatedPath::new(&path_str)
                    .with_context(|| format!("Invalid path in B+ tree data: {path_str}"))?;

                btree_root = btree::insert_into_tree(btree_root, doc_id, validated_path)
                    .with_context(|| {
                        format!("Failed to insert into B+ tree: {id_str} -> {path_str}")
                    })?;
            }

            *self.btree_root.write().await = btree_root;
        }

        // Apply any pending WAL entries to bring the tree fully up to date
        if self.apply_wal_entries().await? {
            self.refresh_metadata_from_tree().await?;
        }

        Ok(())
    }

    /// Replay WAL entries that were not yet persisted to disk
    async fn apply_wal_entries(&self) -> Result<bool> {
        let wal_path = self.index_path.join("wal").join("current.wal");
        if !wal_path.exists() {
            return Ok(false);
        }

        let file = match tokio::fs::File::open(&wal_path).await {
            Ok(file) => file,
            Err(e) => {
                // If the WAL disappeared between exists() and open(), treat it as empty
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(false);
                }
                return Err(e)
                    .with_context(|| format!("Failed to open WAL file: {}", wal_path.display()));
            }
        };

        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut entries = Vec::new();

        loop {
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .await
                .context("Failed to read WAL entry")?;
            if bytes == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry: WalEntry = match serde_json::from_str(trimmed) {
                Ok(entry) => entry,
                Err(e) => {
                    tracing::warn!("Skipping malformed WAL entry: {}", e);
                    continue;
                }
            };
            entries.push(entry);
        }

        if entries.is_empty() {
            return Ok(false);
        }

        let mut btree_root = self.btree_root.write().await;

        for entry in entries {
            match entry {
                WalEntry::Insert { id, path } => {
                    let doc_id = ValidatedDocumentId::from_uuid(id)
                        .with_context(|| format!("Invalid document id in WAL: {}", id))?;
                    let validated_path = ValidatedPath::new(&path)
                        .with_context(|| format!("Invalid path in WAL: {}", path))?;

                    *btree_root =
                        btree::insert_into_tree(btree_root.clone(), doc_id, validated_path)
                            .context("Failed to apply WAL insert to B+ tree")?;
                }
                WalEntry::Delete { id } => {
                    let doc_id = ValidatedDocumentId::from_uuid(id)
                        .with_context(|| format!("Invalid document id in WAL: {}", id))?;

                    if btree::search_in_tree(&btree_root, &doc_id).is_some() {
                        *btree_root = btree::delete_from_tree(btree_root.clone(), &doc_id)
                            .context("Failed to apply WAL delete to B+ tree")?;
                    }
                }
            }
        }

        Ok(true)
    }

    /// Recalculate metadata based on the current in-memory tree
    async fn refresh_metadata_from_tree(&self) -> Result<()> {
        let btree_root = self.btree_root.read().await;
        let document_count = extract_all_pairs(&btree_root)?.len();
        drop(btree_root);

        let mut metadata = self.metadata.write().await;
        metadata.document_count = document_count;
        metadata.updated = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Append an entry to the WAL before mutating in-memory structures
    async fn append_wal_entry(&self, entry: &WalEntry) -> Result<()> {
        let mut writer_guard = self.wal_writer.write().await;
        let file = writer_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("WAL writer not initialized"))?;

        let mut payload = serde_json::to_vec(entry).context("Failed to serialize WAL entry")?;
        payload.push(b'\n');
        file.write_all(&payload)
            .await
            .context("Failed to append WAL entry")?;
        file.flush().await.context("Failed to flush WAL entry")?;
        file.sync_all().await.context("Failed to sync WAL entry")?;

        Ok(())
    }

    /// Reset the WAL after a successful flush so old entries don't replay twice
    async fn reset_wal(&self) -> Result<()> {
        let wal_path = self.index_path.join("wal").join("current.wal");

        {
            let mut writer = self.wal_writer.write().await;
            if let Some(file) = writer.as_mut() {
                file.sync_all()
                    .await
                    .context("Failed to sync WAL before reset")?;
                file.set_len(0)
                    .await
                    .context("Failed to truncate WAL during reset")?;
                file.seek(SeekFrom::Start(0))
                    .await
                    .context("Failed to seek WAL during reset")?;
            }
        }

        // Reinitialize the WAL writer so subsequent operations append cleanly
        self.init_wal().await
    }

    /// Save metadata to disk
    async fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.index_path.join("meta").join("metadata.json");
        let metadata = self.metadata.read().await;

        let content =
            serde_json::to_string_pretty(&*metadata).context("Failed to serialize metadata")?;

        fs::write(&metadata_path, content)
            .await
            .with_context(|| format!("Failed to write metadata: {}", metadata_path.display()))?;

        Ok(())
    }

    /// Save B+ tree data to disk
    async fn save_mappings(&self) -> Result<()> {
        let btree_path = self.index_path.join("data").join("btree_data.json");
        let btree_root = self.btree_root.read().await;

        // Extract all key-value pairs from B+ tree for serialization
        // Future optimization: serialize the tree structure directly
        let all_pairs = extract_all_pairs(&btree_root)?;

        // Convert to serializable format
        let raw_mappings: HashMap<String, String> = all_pairs
            .iter()
            .map(|(doc_id, path)| (doc_id.as_uuid().to_string(), path.to_string()))
            .collect();

        let content = serde_json::to_string_pretty(&raw_mappings)
            .context("Failed to serialize B+ tree data")?;

        fs::write(&btree_path, content)
            .await
            .with_context(|| format!("Failed to write B+ tree data: {}", btree_path.display()))?;

        Ok(())
    }

    /// Update metadata counters
    async fn update_metadata(&self, document_count_delta: i32) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        if document_count_delta < 0 {
            let decrease = (-document_count_delta) as usize;
            if metadata.document_count < decrease {
                bail!(
                    "Document count would go negative: {} - {}",
                    metadata.document_count,
                    decrease
                );
            }
            metadata.document_count -= decrease;
        } else {
            metadata.document_count += document_count_delta as usize;
        }

        metadata.updated = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Validate preconditions for insert operation
    fn validate_insert_preconditions(
        key: &ValidatedDocumentId,
        _value: &ValidatedPath,
    ) -> Result<()> {
        // Key validation
        let uuid = key.as_uuid();
        if uuid.is_nil() {
            bail!("Insert precondition failed: Key cannot be nil UUID");
        }

        // Value validation - ValidatedPath already ensures non-empty and valid format
        // Additional checks can be added here if needed

        Ok(())
    }

    /// Validate preconditions for delete operation
    fn validate_delete_preconditions(key: &ValidatedDocumentId) -> Result<()> {
        let uuid = key.as_uuid();
        if uuid.is_nil() {
            bail!("Delete precondition failed: Key cannot be nil UUID");
        }

        Ok(())
    }

    /// Validate preconditions for search operation
    fn validate_search_preconditions(_query: &Query) -> Result<()> {
        // Query validation is handled by Query::new() constructor
        // Additional index-specific validation can be added here

        Ok(())
    }

    /// Validate postcondition that entry is searchable after insert
    async fn validate_insert_postcondition(
        &self,
        key: &ValidatedDocumentId,
        value: &ValidatedPath,
    ) -> Result<()> {
        let btree_root = self.btree_root.read().await;

        match btree::search_in_tree(&btree_root, key) {
            Some(stored_path) => {
                if stored_path != *value {
                    bail!("Insert postcondition failed: Stored path {} does not match inserted path {}", 
                          stored_path, value);
                }
                Ok(())
            }
            None => {
                bail!(
                    "Insert postcondition failed: Key {} not found after insertion",
                    key.as_uuid()
                );
            }
        }
    }

    /// Validate postcondition that key is not searchable after delete
    async fn validate_delete_postcondition(&self, key: &ValidatedDocumentId) -> Result<()> {
        let btree_root = self.btree_root.read().await;

        if btree::search_in_tree(&btree_root, key).is_some() {
            bail!(
                "Delete postcondition failed: Key {} still exists after deletion",
                key.as_uuid()
            );
        }

        Ok(())
    }
}

#[async_trait]
impl Index for PrimaryIndex {
    /// Open an index instance at the given path
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Validate path for internal storage (allows absolute paths)
        validation::path::validate_storage_directory_path(path)?;

        let index_path = PathBuf::from(path);
        let index = Self {
            index_path: index_path.clone(),
            btree_root: RwLock::new(btree::create_empty_tree()),
            wal_writer: RwLock::new(None),
            metadata: RwLock::new(IndexMetadata::default()),
            load_state: RwLock::new(LoadState::NotLoaded),
        };

        // Ensure directory structure exists
        index.ensure_directories().await?;

        // Initialize WAL
        index.init_wal().await?;

        // Skip loading existing index - will be loaded lazily on first search
        // This dramatically improves cold start performance
        tracing::debug!("Skipping primary index load during initialization for faster cold start");

        Ok(index)
    }

    /// Insert a key-value pair into the primary index
    ///
    /// # Contract
    /// - Preconditions: Key must be non-nil, Value must be valid path
    /// - Postconditions: Entry is searchable immediately, previous value overwritten
    /// - Invariants: Document count increases by 1 (if new key)
    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_insert_preconditions(&id, &path)?;

        let was_new_key;

        self.append_wal_entry(&WalEntry::Insert {
            id: id.as_uuid(),
            path: path.to_string(),
        })
        .await?;

        // Check if key exists before insertion (for metadata counting)
        {
            let btree_root = self.btree_root.read().await;
            was_new_key = btree::search_in_tree(&btree_root, &id).is_none();
        }

        // Insert into B+ tree using pure functions (O(log n))
        {
            let mut btree_root = self.btree_root.write().await;
            *btree_root = btree::insert_into_tree(btree_root.clone(), id, path.clone())
                .context("Failed to insert into B+ tree")?;
        }

        // Update metadata
        if was_new_key {
            self.update_metadata(1).await?;
        } else {
            // Update timestamp even for overwrites
            let mut metadata = self.metadata.write().await;
            metadata.updated = chrono::Utc::now().timestamp();
        }

        // Stage 2: Contract enforcement - validate postconditions
        self.validate_insert_postcondition(&id, &path)
            .await
            .context("Insert postcondition validation failed")?;

        Ok(())
    }

    /// Remove a key from the primary index
    ///
    /// # Contract  
    /// - Preconditions: Key must be valid format
    /// - Postconditions: Key no longer appears in searches, space reclaimed
    /// - Invariants: Document count decreases by 1 (if key existed)
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_delete_preconditions(id)?;

        let existed;

        // Check if key exists before deletion
        {
            let btree_root = self.btree_root.read().await;
            existed = btree::search_in_tree(&btree_root, id).is_some();
        }

        if existed {
            self.append_wal_entry(&WalEntry::Delete { id: id.as_uuid() })
                .await?;

            // Use O(log n) B+ tree deletion algorithm
            let mut btree_root = self.btree_root.write().await;

            *btree_root = btree::delete_from_tree(btree_root.clone(), id)
                .context("Failed to delete from B+ tree")?;

            // Update metadata
            self.update_metadata(-1).await?;
        }

        // Stage 2: Contract enforcement - validate postconditions
        self.validate_delete_postcondition(id)
            .await
            .context("Delete postcondition validation failed")?;

        Ok(existed)
    }

    /// Search the primary index
    ///
    /// # Contract
    /// - Preconditions: Query must be valid for index type
    /// - Postconditions: Results sorted by relevance, all matches returned
    /// - Invariants: Does not modify index state
    ///
    /// # Note
    /// Primary index only supports wildcard searches (no text search terms).
    /// For text search, use a dedicated text search index.
    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        // Ensure index is loaded before searching (lazy loading)
        self.ensure_loaded().await?;

        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_search_preconditions(query)?;

        // DEBUG: Log query details
        tracing::debug!(
            "Primary Index search called with query: path_pattern={:?}, search_terms={:?}, limit={}",
            query.path_pattern,
            query.search_terms,
            query.limit.get()
        );

        let btree_root = self.btree_root.read().await;

        tracing::debug!(
            "Primary Index contains {} document(s) after loading",
            btree_root.total_keys
        );

        // Check for wildcard patterns in path_pattern field first, then search_terms
        // The QueryBuilder puts wildcard queries in path_pattern (see issue #337)
        let wildcard_pattern = if let Some(ref pattern) = query.path_pattern {
            // Use the path_pattern field if it's set (this is where wildcards go)
            tracing::debug!("Using path_pattern: '{}'", pattern);
            Some(pattern.clone())
        } else if query.search_terms.len() == 1 {
            // Fall back to checking search_terms for backward compatibility
            let term = &query.search_terms[0];
            if term.as_str().contains('*') {
                tracing::debug!("Using wildcard from search_terms: '{}'", term.as_str());
                Some(term.as_str().to_string())
            } else {
                tracing::debug!("No wildcard in search_terms: '{}'", term.as_str());
                None
            }
        } else if query.search_terms.is_empty() {
            // Empty search terms means return all
            tracing::debug!("Empty search terms, returning all documents");
            None
        } else {
            // Multiple search terms not supported for wildcard patterns
            tracing::debug!("Multiple search terms not supported for wildcard patterns");
            return Ok(Vec::new());
        };

        // Filter results based on wildcard pattern if present
        let limit_value = query.limit.get();
        let mut matched_ids: Vec<Uuid> = Vec::new();

        if let Some(pattern) = wildcard_pattern {
            tracing::debug!("Filtering documents with pattern: '{}'", pattern);

            traverse_pairs_until(&btree_root, |doc_id, path| {
                let path_str = path.as_str();
                let matches = Self::matches_wildcard_pattern(path_str, &pattern);
                if matches {
                    tracing::debug!("Pattern '{}' MATCHES path: '{}'", pattern, path_str);
                    matched_ids.push(doc_id.as_uuid());
                }
                matched_ids.len() < limit_value
            });
        } else {
            tracing::debug!(
                "No pattern specified, returning up to {} documents",
                limit_value
            );

            traverse_pairs_until(&btree_root, |doc_id, _| {
                matched_ids.push(doc_id.as_uuid());
                matched_ids.len() < limit_value
            });
        }

        if matched_ids.len() > limit_value {
            matched_ids.truncate(limit_value);
        }

        let mut results = Vec::with_capacity(matched_ids.len());
        for uuid in matched_ids {
            match ValidatedDocumentId::from_uuid(uuid) {
                Ok(id) => results.push(id),
                Err(_) => {
                    tracing::warn!("Skipping invalid document id {} during search", uuid);
                }
            }
        }

        tracing::debug!("Primary Index search returning {} results", results.len());

        Ok(results)
    }

    /// Flush index to disk
    ///
    /// # Contract
    /// - Preconditions: Index must be valid
    /// - Postconditions: All changes persisted, index recoverable after crash
    /// - Invariants: Index state unchanged
    async fn flush(&mut self) -> Result<()> {
        // Save all persistent state
        self.save_metadata()
            .await
            .context("Failed to save metadata during flush")?;

        self.save_mappings()
            .await
            .context("Failed to save mappings during flush")?;

        // Sync WAL
        self.reset_wal()
            .await
            .context("Failed to reset WAL after flush")?;

        Ok(())
    }

    /// Update an existing entry in the index
    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For B+ tree, update is the same as insert (it overwrites)
        self.insert(id, path).await
    }

    /// Sync changes to persistent storage
    async fn sync(&mut self) -> Result<()> {
        // Similar to flush for this implementation
        self.flush().await
    }

    /// Close the index instance
    async fn close(self) -> Result<()> {
        // Save final state
        // Note: We need to work around the fact that close() consumes self
        // but save methods require &self

        // For this simple implementation, we just drop the WAL writer
        // In a real implementation, we'd properly close all resources

        drop(self.wal_writer);
        Ok(())
    }
}

/// Create a fully wrapped PrimaryIndex with all Stage 6 components
///
/// This is the recommended way to create a production-ready primary index.
/// It automatically applies Stage 6 MeteredIndex wrapper for metrics and observability.
pub async fn create_primary_index(
    path: &str,
    _cache_capacity: Option<usize>,
) -> Result<MeteredIndex<PrimaryIndex>> {
    // Stage 2: Validate path for internal storage (allows absolute paths)
    validation::path::validate_storage_directory_path(path)?;

    let index_path = PathBuf::from(path);
    let index = PrimaryIndex {
        index_path,
        btree_root: RwLock::new(btree::create_empty_tree()),
        wal_writer: RwLock::new(None),
        metadata: RwLock::new(IndexMetadata::default()),
        load_state: RwLock::new(LoadState::NotLoaded),
    };

    // Ensure directory structure exists
    index.ensure_directories().await?;

    // Initialize WAL
    index.init_wal().await?;

    // Load existing data
    index.load_existing_index().await?;

    // Apply Stage 6 wrapper for automatic metrics
    Ok(MeteredIndex::new(index, "primary".to_string()))
}

/// Alternative factory function for testing without cache parameter
/// Used internally by tests that don't need to specify cache capacity
pub async fn create_primary_index_for_tests(path: &str) -> Result<PrimaryIndex> {
    validation::path::validate_storage_directory_path(path)?;

    let index_path = PathBuf::from(path);
    let index = PrimaryIndex {
        index_path,
        btree_root: RwLock::new(btree::create_empty_tree()),
        wal_writer: RwLock::new(None),
        metadata: RwLock::new(IndexMetadata::default()),
        load_state: RwLock::new(LoadState::NotLoaded),
    };

    index.ensure_directories().await?;
    index.init_wal().await?;
    index.load_existing_index().await?;

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_matches_wildcard_pattern() {
        // Test pure wildcard
        assert!(PrimaryIndex::matches_wildcard_pattern("anything", "*"));
        assert!(PrimaryIndex::matches_wildcard_pattern("test.rs", "*"));
        assert!(PrimaryIndex::matches_wildcard_pattern("", "*"));

        // Test suffix wildcard
        assert!(PrimaryIndex::matches_wildcard_pattern("test.rs", "*.rs"));
        assert!(PrimaryIndex::matches_wildcard_pattern("main.rs", "*.rs"));
        assert!(!PrimaryIndex::matches_wildcard_pattern("test.tsx", "*.rs"));
        assert!(!PrimaryIndex::matches_wildcard_pattern("test", "*.rs"));

        // Test prefix wildcard
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "test_file.rs",
            "test_*"
        ));
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "test_another.tsx",
            "test_*"
        ));
        assert!(!PrimaryIndex::matches_wildcard_pattern(
            "main_test.rs",
            "test_*"
        ));

        // Test suffix wildcard with pattern
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "UserController.rs",
            "*Controller.rs"
        ));
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "AuthController.rs",
            "*Controller.rs"
        ));
        assert!(!PrimaryIndex::matches_wildcard_pattern(
            "ControllerUser.rs",
            "*Controller.rs"
        ));
        assert!(!PrimaryIndex::matches_wildcard_pattern(
            "UserController.tsx",
            "*Controller.rs"
        ));

        // Test complex patterns
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "UserPanel.tsx",
            "*Panel.tsx"
        ));
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "AdminPanel.tsx",
            "*Panel.tsx"
        ));
        assert!(PrimaryIndex::matches_wildcard_pattern(
            "Panel.tsx",
            "*Panel.tsx"
        )); // Should match
        assert!(!PrimaryIndex::matches_wildcard_pattern(
            "UserPanel.rs",
            "*Panel.tsx"
        ));
    }

    #[tokio::test]
    async fn test_primary_index_contract_enforcement() -> Result<()> {
        let test_dir = format!("test_data/primary_contract_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let mut index = create_primary_index_for_tests(&test_dir).await?;

        // Test precondition validation
        let valid_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let valid_path = ValidatedPath::new("test/contract.md")?;

        // This should succeed
        index.insert(valid_id, valid_path.clone()).await?;

        // Test postcondition - document should be findable
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], valid_id);

        // Clean up test directory
        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_metadata_management() -> Result<()> {
        let test_dir = format!("test_data/primary_metadata_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let mut index = create_primary_index_for_tests(&test_dir).await?;

        // Check initial metadata
        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 0);
            assert_eq!(metadata.version, 1);
        }

        // Insert document and check metadata update
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/metadata.md")?;

        index.insert(doc_id, doc_path).await?;

        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 1);
        }

        // Delete document and check metadata update
        index.delete(&doc_id).await?;

        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 0);
        }

        // Clean up test directory
        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_wal_recovery() -> Result<()> {
        let test_dir = format!("test_data/primary_wal_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/wal.md")?;

        {
            let mut index = create_primary_index_for_tests(&test_dir).await?;
            index.insert(doc_id, doc_path.clone()).await?;
            // Intentionally skip flush so the WAL must replay on reopen
        }

        let reopened_index = create_primary_index_for_tests(&test_dir).await?;
        let query = Query::new(Some("test/wal.md".to_string()), None, None, 10)?;
        let results = reopened_index.search(&query).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], doc_id);

        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_wal_recovery_skips_truncated_entries() -> Result<()> {
        let test_dir = format!("test_data/primary_wal_truncated_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("test/truncated.md")?;

        {
            let mut index = create_primary_index_for_tests(&test_dir).await?;
            index.insert(doc_id, doc_path.clone()).await?;

            // Append a truncated WAL line to simulate a torn write
            let wal_path = PathBuf::from(&test_dir).join("wal").join("current.wal");
            let mut file = std::fs::OpenOptions::new().append(true).open(&wal_path)?;
            use std::io::Write;
            file.write_all(b"{\"Insert\":")?; // intentionally truncated JSON
        }

        let reopened_index = create_primary_index_for_tests(&test_dir).await?;
        let query = Query::new(Some("test/truncated.md".to_string()), None, None, 10)?;
        let results = reopened_index.search(&query).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], doc_id);

        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }
}
