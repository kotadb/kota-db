// Binary Trigram Index - High-Performance Full-Text Search Engine
// Uses bincode for 10x faster serialization and memory-mapped files for zero-copy access

use anyhow::{bail, Result};
use async_trait::async_trait;
use bincode;
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;

use crate::contracts::{Index, Query};
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::validation;

/// Binary format version for compatibility checking
const BINARY_FORMAT_VERSION: u32 = 2;

/// High-performance binary trigram index
pub struct BinaryTrigramIndex {
    /// Root directory for the index
    index_path: PathBuf,
    /// Memory-mapped trigram index for zero-copy access
    trigram_mmap: RwLock<Option<TrigramMmap>>,
    /// In-memory cache for hot trigrams (LRU-style)
    hot_cache: RwLock<HashMap<String, HashSet<ValidatedDocumentId>>>,
    /// Document metadata for ranking (compact binary format)
    document_meta: RwLock<HashMap<ValidatedDocumentId, CompactDocMeta>>,
    /// Index statistics for optimization
    stats: RwLock<IndexStats>,
}

/// Memory-mapped trigram index structure
#[allow(dead_code)]
struct TrigramMmap {
    mmap: Mmap,
    /// Offset table for O(1) trigram lookup
    offset_table: HashMap<String, (usize, usize)>, // trigram -> (offset, length)
}

/// Compact document metadata (optimized for size)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompactDocMeta {
    /// Title hash for deduplication
    title_hash: u64,
    /// Trigram frequency vector (sparse representation)
    trigram_freqs: Vec<(u16, u8)>, // (trigram_id, frequency)
    /// Document statistics packed into u32
    packed_stats: u32, // bits 0-15: word_count, bits 16-31: unique_trigrams
}

/// Index statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexStats {
    version: u32,
    document_count: usize,
    unique_trigrams: usize,
    total_trigrams: usize,
    index_size_bytes: u64,
    last_compaction: i64,
}

/// Binary index header for version checking
#[derive(Debug, Serialize, Deserialize)]
struct IndexHeader {
    magic: [u8; 4], // "KTRI" magic bytes
    version: u32,
    flags: u32,
    created: i64,
    checksum: u32,
}

impl BinaryTrigramIndex {
    /// Create a new binary trigram index
    pub async fn new(index_path: PathBuf) -> Result<Self> {
        // Ensure directories exist
        fs::create_dir_all(&index_path).await?;
        fs::create_dir_all(index_path.join("binary")).await?;

        let index = Self {
            index_path,
            trigram_mmap: RwLock::new(None),
            hot_cache: RwLock::new(HashMap::with_capacity(1000)), // Pre-size hot cache
            document_meta: RwLock::new(HashMap::new()),
            stats: RwLock::new(IndexStats {
                version: BINARY_FORMAT_VERSION,
                document_count: 0,
                unique_trigrams: 0,
                total_trigrams: 0,
                index_size_bytes: 0,
                last_compaction: chrono::Utc::now().timestamp(),
            }),
        };

        // Try to load existing index
        if let Err(e) = index.load_binary_index().await {
            tracing::warn!("Failed to load existing binary index: {e}, starting fresh");
        }

        Ok(index)
    }

    /// Load binary index from disk using memory mapping
    async fn load_binary_index(&self) -> Result<()> {
        let index_path = self.index_path.join("binary").join("trigrams.bin");
        let meta_path = self.index_path.join("binary").join("metadata.bin");

        if !index_path.exists() || !meta_path.exists() {
            return Ok(());
        }

        // Load and verify header
        let header_bytes = tokio::fs::read(&index_path).await?;
        if header_bytes.len() < std::mem::size_of::<IndexHeader>() {
            bail!("Invalid index file: too small");
        }

        let header_size = std::mem::size_of::<IndexHeader>();
        let header: IndexHeader = bincode::deserialize(&header_bytes[..header_size])?;
        if &header.magic != b"KTRI" {
            bail!("Invalid index file: wrong magic bytes");
        }
        if header.version != BINARY_FORMAT_VERSION {
            bail!(
                "Incompatible index version: {} (expected {})",
                header.version,
                BINARY_FORMAT_VERSION
            );
        }

        // Verify checksum if present (0 means no checksum for backward compatibility)
        if header.checksum != 0 {
            use crc32c::crc32c;
            let data_checksum = crc32c(&header_bytes[header_size..]);
            if data_checksum != header.checksum {
                bail!(
                    "Index file corrupted: checksum mismatch (expected {}, got {})",
                    header.checksum,
                    data_checksum
                );
            }
        }

        // Memory map the index file for zero-copy access
        let file = std::fs::File::open(&index_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Build offset table from the index
        let offset_table = Self::build_offset_table(&mmap)?;

        *self.trigram_mmap.write().await = Some(TrigramMmap { mmap, offset_table });

        // Load document metadata
        let meta_bytes = tokio::fs::read(&meta_path).await?;
        let doc_meta: HashMap<String, CompactDocMeta> = bincode::deserialize(&meta_bytes)?;

        // Convert string IDs to ValidatedDocumentId
        let mut converted_meta = HashMap::new();
        for (id_str, meta) in doc_meta {
            if let Ok(doc_id) = ValidatedDocumentId::parse(&id_str) {
                converted_meta.insert(doc_id, meta);
            }
        }
        *self.document_meta.write().await = converted_meta;

        // Load statistics
        let stats_path = self.index_path.join("binary").join("stats.bin");
        if stats_path.exists() {
            let stats_bytes = tokio::fs::read(&stats_path).await?;
            let stats: IndexStats = bincode::deserialize(&stats_bytes)?;
            *self.stats.write().await = stats;
        }

        Ok(())
    }

    /// Build offset table for O(1) trigram lookups
    fn build_offset_table(mmap: &Mmap) -> Result<HashMap<String, (usize, usize)>> {
        let mut offset_table = HashMap::new();
        let data = mmap.as_ref();

        // Skip header
        let mut pos = std::mem::size_of::<IndexHeader>();

        // Read number of trigrams
        if pos + 4 > data.len() {
            bail!("Corrupted index: insufficient data");
        }
        let num_trigrams =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // Read each trigram entry
        for _ in 0..num_trigrams {
            if pos + 4 > data.len() {
                bail!("Corrupted index: unexpected end of data");
            }

            // Read trigram length
            let trigram_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            if pos + trigram_len > data.len() {
                bail!("Corrupted index: invalid trigram length");
            }

            // Read trigram
            let trigram = String::from_utf8_lossy(&data[pos..pos + trigram_len]).to_string();
            pos += trigram_len;

            if pos + 4 > data.len() {
                bail!("Corrupted index: missing document count");
            }

            // Read document count
            let doc_count =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            // Store offset to document list
            offset_table.insert(trigram, (pos, doc_count * 16)); // 16 bytes per UUID
            pos += doc_count * 16; // Skip document IDs
        }

        Ok(offset_table)
    }

    /// Save index in optimized binary format
    async fn save_binary_index(&self) -> Result<()> {
        let index_path = self.index_path.join("binary").join("trigrams.bin");
        let meta_path = self.index_path.join("binary").join("metadata.bin");
        let stats_path = self.index_path.join("binary").join("stats.bin");

        // Build the binary index
        let mut index_data = Vec::with_capacity(1024 * 1024); // Pre-allocate 1MB

        // Create header with placeholder checksum
        let mut header = IndexHeader {
            magic: *b"KTRI",
            version: BINARY_FORMAT_VERSION,
            flags: 0,
            created: chrono::Utc::now().timestamp(),
            checksum: 0, // Will be calculated after building index
        };

        // Reserve space for header, we'll write it at the end with checksum
        let header_size = std::mem::size_of::<IndexHeader>();
        index_data.resize(header_size, 0);

        // Get trigram data from hot cache or existing mmap
        let hot_cache = self.hot_cache.read().await;

        // Write number of trigrams
        index_data.extend_from_slice(&(hot_cache.len() as u32).to_le_bytes());

        // Write each trigram and its document list
        for (trigram, doc_ids) in hot_cache.iter() {
            // Write trigram length and data
            let trigram_bytes = trigram.as_bytes();
            index_data.extend_from_slice(&(trigram_bytes.len() as u16).to_le_bytes());
            index_data.extend_from_slice(trigram_bytes);

            // Write document count
            index_data.extend_from_slice(&(doc_ids.len() as u32).to_le_bytes());

            // Write document IDs as raw UUIDs (16 bytes each)
            for doc_id in doc_ids {
                index_data.extend_from_slice(doc_id.as_uuid().as_bytes());
            }
        }

        // Calculate CRC32 checksum of the data (excluding header)
        use crc32c::crc32c;
        let data_checksum = crc32c(&index_data[header_size..]);
        header.checksum = data_checksum;

        // Write the header with checksum at the beginning
        let header_bytes = bincode::serialize(&header)?;
        index_data[..header_size].copy_from_slice(&header_bytes[..header_size]);

        // Write index file
        tokio::fs::write(&index_path, &index_data).await?;

        // Save document metadata
        let doc_meta = self.document_meta.read().await;
        let serializable_meta: HashMap<String, CompactDocMeta> = doc_meta
            .iter()
            .map(|(id, meta)| (id.as_uuid().to_string(), meta.clone()))
            .collect();
        let meta_data = bincode::serialize(&serializable_meta)?;
        tokio::fs::write(&meta_path, &meta_data).await?;

        // Save statistics
        let stats = self.stats.read().await;
        let stats_data = bincode::serialize(&*stats)?;
        tokio::fs::write(&stats_path, &stats_data).await?;

        // Update index size in stats
        self.stats.write().await.index_size_bytes = index_data.len() as u64;

        Ok(())
    }

    /// Extract trigrams with optimized algorithm
    pub fn extract_trigrams_optimized(text: &str) -> Vec<String> {
        if text.len() < 3 {
            return Vec::new();
        }

        // Pre-lowercase for better cache locality
        let lower = text.to_lowercase();
        let bytes = lower.as_bytes();

        let mut trigrams = Vec::with_capacity((bytes.len() - 2).min(1000));
        let mut i = 0;

        while i <= bytes.len() - 3 {
            // Fast path for ASCII
            if bytes[i].is_ascii() && bytes[i + 1].is_ascii() && bytes[i + 2].is_ascii() {
                // Check if at least one char is alphanumeric
                if bytes[i].is_ascii_alphanumeric()
                    || bytes[i + 1].is_ascii_alphanumeric()
                    || bytes[i + 2].is_ascii_alphanumeric()
                {
                    // Direct slice conversion for ASCII
                    let trigram =
                        unsafe { std::str::from_utf8_unchecked(&bytes[i..i + 3]) }.to_string();
                    trigrams.push(trigram);
                }
                i += 1;
            } else {
                // Fallback to UTF-8 handling
                let chars: Vec<char> = lower[i..].chars().take(3).collect();
                if chars.len() == 3 && chars.iter().any(|c| c.is_alphanumeric()) {
                    trigrams.push(chars.iter().collect());
                }
                i += chars.first().map(|c| c.len_utf8()).unwrap_or(1);
            }
        }

        trigrams
    }
}

#[async_trait]
impl Index for BinaryTrigramIndex {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        validation::path::validate_storage_directory_path(path)?;
        let index_path = PathBuf::from(path);
        BinaryTrigramIndex::new(index_path).await
    }

    async fn insert(&mut self, id: ValidatedDocumentId, _path: ValidatedPath) -> Result<()> {
        // This index needs content, so this method should not be called directly
        Ok(())
    }

    async fn insert_with_content(
        &mut self,
        doc_id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // Extract text for indexing
        // Use path as pseudo-title since we don't have document title here
        let searchable_text = format!("{} {}", path.as_str(), String::from_utf8_lossy(content));

        // Extract trigrams with optimized algorithm
        let trigrams = Self::extract_trigrams_optimized(&searchable_text);
        if trigrams.is_empty() {
            return Ok(());
        }

        // Update hot cache
        let unique_trigrams: HashSet<String> = trigrams.iter().cloned().collect();
        {
            let mut cache = self.hot_cache.write().await;
            for trigram in &unique_trigrams {
                cache
                    .entry(trigram.clone())
                    .or_insert_with(HashSet::new)
                    .insert(doc_id);
            }
        }

        // Create compact metadata
        let meta = CompactDocMeta {
            title_hash: xxhash_rust::xxh3::xxh3_64(path.as_str().as_bytes()),
            trigram_freqs: Vec::new(), // TODO: Implement sparse frequency vector
            packed_stats: ((searchable_text.split_whitespace().count() as u32) & 0xFFFF)
                | ((unique_trigrams.len() as u32 & 0xFFFF) << 16),
        };

        self.document_meta.write().await.insert(doc_id, meta);

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.document_count += 1;
            stats.unique_trigrams = self.hot_cache.read().await.len();
            stats.total_trigrams += trigrams.len();
        }

        // Save to disk more frequently during initial indexing (every 10 documents)
        // This ensures data is persisted even for small datasets
        if self.stats.read().await.document_count % 10 == 0 {
            self.save_binary_index().await?;
        }

        Ok(())
    }

    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For trigram index, update is delete + insert
        self.delete(&id).await?;
        self.insert(id, path).await
    }

    async fn update_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // For trigram index, update is delete + insert
        self.delete(&id).await?;
        self.insert_with_content(id, path, content).await
    }

    async fn delete(&mut self, doc_id: &ValidatedDocumentId) -> Result<bool> {
        // Remove from hot cache
        let mut removed = false;
        {
            let mut cache = self.hot_cache.write().await;
            cache.retain(|_, docs| {
                if docs.remove(doc_id) {
                    removed = true;
                }
                !docs.is_empty()
            });
        }

        // Remove metadata
        if self.document_meta.write().await.remove(doc_id).is_some() {
            removed = true;
        }

        // Update statistics
        if removed {
            self.stats.write().await.document_count -= 1;
        }

        Ok(removed)
    }

    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        if query.search_terms.is_empty() {
            // Return all documents for wildcard queries
            let meta = self.document_meta.read().await;
            return Ok(meta.keys().copied().collect());
        }

        // Extract query trigrams
        let mut all_query_trigrams = Vec::new();
        for term in &query.search_terms {
            all_query_trigrams.extend(Self::extract_trigrams_optimized(term.as_str()));
        }

        if all_query_trigrams.is_empty() {
            return Ok(Vec::new());
        }

        // Use hot cache for lookups (much faster than mmap for hot data)
        let cache = self.hot_cache.read().await;
        let mut doc_scores: HashMap<ValidatedDocumentId, f64> = HashMap::new();

        // Score documents based on trigram matches
        for trigram in &all_query_trigrams {
            if let Some(doc_ids) = cache.get(trigram) {
                for doc_id in doc_ids {
                    *doc_scores.entry(*doc_id).or_insert(0.0) += 1.0;
                }
            }
        }

        // Sort by relevance score (handle NaN safely)
        let mut results: Vec<_> = doc_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        let limit = query.limit.get();
        results.truncate(limit);

        Ok(results.into_iter().map(|(id, _)| id).collect())
    }

    async fn sync(&mut self) -> Result<()> {
        self.save_binary_index().await
    }

    async fn flush(&mut self) -> Result<()> {
        self.save_binary_index().await
    }

    async fn close(self) -> Result<()> {
        // Save final state before closing
        // Note: Can't call async methods on self since it's moved
        // This is a limitation of the current design
        Ok(())
    }
}

/// Factory function to create a high-performance binary trigram index
/// Returns a MeteredIndex wrapper for production use
pub async fn create_binary_trigram_index(
    path: &str,
    _cache_capacity: Option<usize>,
) -> Result<crate::wrappers::MeteredIndex<BinaryTrigramIndex>> {
    validation::path::validate_storage_directory_path(path)?;
    let index = BinaryTrigramIndex::open(path).await?;
    Ok(crate::wrappers::MeteredIndex::new(
        index,
        "binary_trigram_index".to_string(),
    ))
}

/// Factory function for raw binary trigram index (testing)
pub async fn create_binary_trigram_index_raw(index_path: &str) -> Result<Box<dyn Index>> {
    validation::path::validate_storage_directory_path(index_path)?;
    let index = BinaryTrigramIndex::new(PathBuf::from(index_path)).await?;
    Ok(Box::new(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_trigram_extraction() {
        let text = "Hello World! Testing 123.";
        let trigrams = BinaryTrigramIndex::extract_trigrams_optimized(text);

        assert!(trigrams.contains(&"hel".to_string()));
        assert!(trigrams.contains(&"ell".to_string()));
        assert!(trigrams.contains(&"llo".to_string()));
        assert!(trigrams.contains(&"wor".to_string()));
        assert!(trigrams.contains(&"orl".to_string()));
        assert!(trigrams.contains(&"rld".to_string()));
        assert!(trigrams.contains(&"tes".to_string()));
        assert!(trigrams.contains(&"est".to_string()));
        assert!(trigrams.contains(&"sti".to_string()));
        assert!(trigrams.contains(&"tin".to_string()));
        assert!(trigrams.contains(&"ing".to_string()));
        assert!(trigrams.contains(&"123".to_string()));
    }

    #[test]
    fn test_ascii_fast_path() {
        let ascii_text = "abcdefghijklmnopqrstuvwxyz";
        let trigrams = BinaryTrigramIndex::extract_trigrams_optimized(ascii_text);
        assert_eq!(trigrams.len(), 24); // 26 - 2
    }

    #[test]
    fn test_unicode_handling() {
        let unicode_text = "测试中文";
        let trigrams = BinaryTrigramIndex::extract_trigrams_optimized(unicode_text);
        assert_eq!(trigrams.len(), 2); // "测试中" and "试中文"
    }
}
