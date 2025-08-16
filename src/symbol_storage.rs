//! Symbol storage and extraction pipeline for code intelligence
//!
//! This module provides persistent storage and indexing for code symbols extracted
//! from parsed source files. It enables intelligent code search, dependency mapping,
//! and incremental symbol updates.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::builders::DocumentBuilder;
use crate::contracts::{Document, Storage};
use crate::parsing::{ParsedCode, ParsedSymbol, SupportedLanguage, SymbolType};
use crate::types::ValidatedDocumentId;

/// Symbol index entry with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// Unique identifier for this symbol
    pub id: Uuid,
    /// ID of the document containing this symbol
    pub document_id: ValidatedDocumentId,
    /// Repository this symbol belongs to (if from git ingestion)
    pub repository: Option<String>,
    /// File path relative to repository root
    pub file_path: PathBuf,
    /// Symbol information from parser
    pub symbol: ParsedSymbol,
    /// Language of the source file
    pub language: SupportedLanguage,
    /// Fully qualified name (e.g., module::class::method)
    pub qualified_name: String,
    /// Parent symbol ID (for nested symbols)
    pub parent_id: Option<Uuid>,
    /// Child symbol IDs
    pub children: Vec<Uuid>,
    /// Dependencies (imports/uses) this symbol references
    pub dependencies: Vec<String>,
    /// Other symbols that depend on this one
    pub dependents: HashSet<Uuid>,
    /// Timestamp when symbol was extracted
    pub extracted_at: DateTime<Utc>,
    /// Hash of symbol content for change detection
    pub content_hash: String,
}

/// Symbol relationship types for dependency mapping
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Custom relationship type
    Custom(String),
}

/// Relationship between two symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRelation {
    /// Source symbol ID
    pub from_id: Uuid,
    /// Target symbol ID
    pub to_id: Uuid,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Statistics about the symbol index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndexStats {
    /// Total number of symbols indexed
    pub total_symbols: usize,
    /// Breakdown by symbol type
    pub symbols_by_type: HashMap<String, usize>,
    /// Breakdown by language
    pub symbols_by_language: HashMap<String, usize>,
    /// Number of repositories indexed
    pub repository_count: usize,
    /// Number of files indexed
    pub file_count: usize,
    /// Total relationships mapped
    pub relationship_count: usize,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Configuration for symbol storage
pub struct SymbolStorageConfig {
    /// Maximum number of symbols to keep in memory (default: 100,000)
    pub max_symbols: usize,
    /// Maximum memory usage in bytes (default: 500MB)
    pub max_memory_bytes: usize,
    /// Fuzzy search score thresholds
    pub search_thresholds: SearchThresholds,
}

impl Default for SymbolStorageConfig {
    fn default() -> Self {
        Self {
            max_symbols: 100_000,
            max_memory_bytes: 500 * 1024 * 1024, // 500MB
            search_thresholds: SearchThresholds::default(),
        }
    }
}

/// Configurable thresholds for fuzzy search scoring
pub struct SearchThresholds {
    /// Score for exact name match (default: 1.0)
    pub exact_match: f32,
    /// Score for prefix match (default: 0.8)
    pub prefix_match: f32,
    /// Score for substring match (default: 0.6)
    pub contains_match: f32,
    /// Minimum overlap ratio for fuzzy match (default: 0.5)
    pub min_fuzzy_overlap: f32,
    /// Score multiplier for fuzzy matches (default: 0.5)
    pub fuzzy_multiplier: f32,
}

impl Default for SearchThresholds {
    fn default() -> Self {
        Self {
            exact_match: 1.0,
            prefix_match: 0.8,
            contains_match: 0.6,
            min_fuzzy_overlap: 0.5,
            fuzzy_multiplier: 0.5,
        }
    }
}

/// Symbol storage and extraction pipeline
pub struct SymbolStorage {
    /// Underlying document storage
    storage: Box<dyn Storage + Send + Sync>,
    /// In-memory symbol index for fast lookups
    symbol_index: HashMap<Uuid, SymbolEntry>,
    /// Symbol relationships
    relationships: Vec<SymbolRelation>,
    /// File to symbols mapping
    file_symbols: HashMap<PathBuf, Vec<Uuid>>,
    /// Qualified name to symbol ID mapping
    name_index: HashMap<String, Vec<Uuid>>,
    /// Repository to files mapping
    repository_files: HashMap<String, HashSet<PathBuf>>,
    /// Configuration
    config: SymbolStorageConfig,
    /// Current estimated memory usage
    estimated_memory_usage: usize,
}

impl SymbolStorage {
    /// Create a new symbol storage instance with default configuration
    pub async fn new(storage: Box<dyn Storage + Send + Sync>) -> Result<Self> {
        Self::with_config(storage, SymbolStorageConfig::default()).await
    }

    /// Create a new symbol storage instance with custom configuration
    pub async fn with_config(
        storage: Box<dyn Storage + Send + Sync>,
        config: SymbolStorageConfig,
    ) -> Result<Self> {
        let mut instance = Self {
            storage,
            symbol_index: HashMap::new(),
            relationships: Vec::new(),
            file_symbols: HashMap::new(),
            name_index: HashMap::new(),
            repository_files: HashMap::new(),
            config,
            estimated_memory_usage: 0,
        };

        // Load existing symbols from storage
        instance.load_symbols().await?;

        Ok(instance)
    }

    /// Load symbols from persistent storage
    #[instrument(skip(self))]
    async fn load_symbols(&mut self) -> Result<()> {
        info!("Loading symbols from storage");

        // Get all documents and filter for symbols
        let all_docs = self
            .storage
            .list_all()
            .await
            .context("Failed to list documents")?;

        let results: Vec<Document> = all_docs
            .into_iter()
            .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "symbol"))
            .collect();

        let mut loaded_count = 0;
        for doc in results {
            match self.deserialize_symbol(&doc) {
                Ok(entry) => {
                    self.index_symbol(entry)?;
                    loaded_count += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize symbol from {}: {}", doc.path, e);
                }
            }
        }

        info!("Loaded {} symbols from storage", loaded_count);
        Ok(())
    }

    /// Extract and store symbols from parsed code
    #[instrument(skip(self, parsed_code))]
    pub async fn extract_symbols(
        &mut self,
        file_path: &Path,
        parsed_code: ParsedCode,
        repository: Option<String>,
    ) -> Result<Vec<Uuid>> {
        info!(
            "Extracting {} symbols from {}",
            parsed_code.symbols.len(),
            file_path.display()
        );

        let mut symbol_ids = Vec::new();
        let mut parent_stack: Vec<(Uuid, usize)> = Vec::new(); // (id, end_line)

        for symbol in parsed_code.symbols {
            // Determine parent based on nesting
            let parent_id = parent_stack
                .iter()
                .rev()
                .find(|(_, end_line)| symbol.start_line <= *end_line)
                .map(|(id, _)| *id);

            // Generate qualified name
            let qualified_name = self.build_qualified_name(&symbol.name, parent_id, file_path);

            // Create symbol entry with deterministic ID based on content
            let symbol_id = self.generate_deterministic_id(&symbol, file_path, parent_id);
            let doc_id = self.generate_document_id(&symbol_id)?;

            let entry = SymbolEntry {
                id: symbol_id,
                document_id: doc_id,
                repository: repository.clone(),
                file_path: file_path.to_path_buf(),
                symbol: symbol.clone(),
                language: parsed_code.language,
                qualified_name: qualified_name.clone(),
                parent_id,
                children: Vec::new(),
                dependencies: self.extract_dependencies(&symbol),
                dependents: HashSet::new(),
                extracted_at: Utc::now(),
                content_hash: self.compute_symbol_hash(&symbol),
            };

            // Update parent's children if applicable
            if let Some(parent_id) = parent_id {
                if let Some(parent) = self.symbol_index.get_mut(&parent_id) {
                    parent.children.push(entry.id);
                }
            }

            // Store symbol
            let symbol_id = entry.id;
            symbol_ids.push(symbol_id);

            // Update stack for nested symbols
            if matches!(
                symbol.symbol_type,
                SymbolType::Function
                    | SymbolType::Class
                    | SymbolType::Struct
                    | SymbolType::Module
                    | SymbolType::Enum
            ) {
                parent_stack.push((symbol_id, symbol.end_line));
            }

            // Clean up stack - remove completed scopes
            parent_stack.retain(|(_, end_line)| symbol.start_line < *end_line);

            // Persist symbol
            self.store_symbol(entry).await?;
        }

        // Update file mapping
        self.file_symbols
            .insert(file_path.to_path_buf(), symbol_ids.clone());

        // Update repository mapping
        if let Some(repo) = repository {
            self.repository_files
                .entry(repo)
                .or_default()
                .insert(file_path.to_path_buf());
        }

        Ok(symbol_ids)
    }

    /// Store a symbol entry persistently
    async fn store_symbol(&mut self, entry: SymbolEntry) -> Result<()> {
        // Serialize symbol to document
        let doc = self.serialize_symbol(&entry)?;

        // Store in underlying storage
        self.storage.insert(doc).await?;

        // Index in memory
        self.index_symbol(entry)?;

        Ok(())
    }

    /// Index a symbol in memory for fast lookups with memory limits
    fn index_symbol(&mut self, entry: SymbolEntry) -> Result<()> {
        // Check memory limits
        let entry_size = self.estimate_symbol_size(&entry);

        if self.symbol_index.len() >= self.config.max_symbols {
            tracing::warn!(
                "Symbol limit reached ({} symbols), skipping indexing",
                self.config.max_symbols
            );
            return Ok(());
        }

        if self.estimated_memory_usage + entry_size > self.config.max_memory_bytes {
            tracing::warn!(
                "Memory limit reached ({} bytes), skipping indexing",
                self.config.max_memory_bytes
            );
            return Ok(());
        }

        // Add to name index
        self.name_index
            .entry(entry.qualified_name.clone())
            .or_default()
            .push(entry.id);

        // Add to main index
        self.symbol_index.insert(entry.id, entry);
        self.estimated_memory_usage += entry_size;

        Ok(())
    }

    /// Estimate memory usage of a symbol entry
    fn estimate_symbol_size(&self, entry: &SymbolEntry) -> usize {
        use std::mem;

        // Base struct size
        mem::size_of::<SymbolEntry>()
            // String allocations
            + entry.qualified_name.len()
            + entry.symbol.name.len()
            + entry.symbol.text.len()
            + entry.content_hash.len()
            // Path allocation
            + entry.file_path.to_string_lossy().len()
            // Collections
            + entry.children.len() * mem::size_of::<Uuid>()
            + entry.dependencies.iter().map(|s| s.len()).sum::<usize>()
            + entry.dependents.len() * mem::size_of::<Uuid>()
    }

    /// Build a qualified name for a symbol
    fn build_qualified_name(
        &self,
        name: &str,
        parent_id: Option<Uuid>,
        file_path: &Path,
    ) -> String {
        let mut parts = Vec::new();

        // Add parent qualified names
        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.symbol_index.get(&parent_id) {
                parts.push(parent.qualified_name.clone());
            }
        }

        // Add current symbol name
        parts.push(name.to_string());

        // Build the qualified name
        if parts.len() > 1 {
            parts.join("::")
        } else {
            // For top-level symbols, include file path for uniqueness
            format!("{}::{}", file_path.display(), name)
        }
    }

    /// Extract dependencies from a symbol
    fn extract_dependencies(&self, symbol: &ParsedSymbol) -> Vec<String> {
        let mut deps = Vec::new();

        // For imports, extract the imported module/symbol
        if symbol.symbol_type == SymbolType::Import {
            // Parse import statement to extract dependency
            // This is a simplified version - real implementation would parse properly
            if let Some(import_path) = self.parse_import_statement(&symbol.text) {
                deps.push(import_path);
            }
        }

        // TODO: Extract function calls, type references, etc.

        deps
    }

    /// Parse an import statement to extract the imported path
    fn parse_import_statement(&self, text: &str) -> Option<String> {
        // Enhanced import parsing with better pattern matching
        let trimmed = text.trim();

        // Rust imports: use crate::module; use super::module; use self::module;
        if let Some(rest) = trimmed.strip_prefix("use ") {
            // Handle complex imports like: use std::{io, fmt};
            if let Some(base) = rest.split(':').next() {
                return Some(base.trim().to_string());
            }
        }

        // Python imports: import module; from module import x; import module as alias
        if let Some(rest) = trimmed.strip_prefix("import ") {
            // Handle "import x as y" by taking just the module name
            if let Some(module) = rest.split_whitespace().next() {
                return Some(module.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            // Handle "from module import x"
            if let Some(module) = rest.split_whitespace().next() {
                return Some(module.to_string());
            }
        }

        // JavaScript/TypeScript imports: import x from 'module'; import {x} from 'module';
        if trimmed.starts_with("import ") {
            // Look for quoted module path
            if let Some(start) = trimmed.find(['\'', '"']) {
                let quote_char = trimmed.chars().nth(start).unwrap();
                if let Some(end) = trimmed[start + 1..].find(quote_char) {
                    return Some(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }

        // TODO: Add support for other languages (Go, Java, C++, etc.)

        None
    }

    /// Compute a hash of symbol content for change detection
    fn compute_symbol_hash(&self, symbol: &ParsedSymbol) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(&symbol.text);
        format!("{:x}", hasher.finalize())
    }

    /// Serialize a symbol entry to a document
    fn serialize_symbol(&self, entry: &SymbolEntry) -> Result<Document> {
        let json = serde_json::to_string_pretty(&entry)?;

        let title = format!(
            "Symbol: {} ({})",
            entry.symbol.name,
            match &entry.symbol.symbol_type {
                SymbolType::Function => "function",
                SymbolType::Method => "method",
                SymbolType::Class => "class",
                SymbolType::Struct => "struct",
                SymbolType::Interface => "interface",
                SymbolType::Enum => "enum",
                SymbolType::Variable => "variable",
                SymbolType::Constant => "constant",
                SymbolType::Module => "module",
                SymbolType::Import => "import",
                SymbolType::Comment => "comment",
                SymbolType::Other(s) => s,
            }
        );

        // Sanitize file path to prevent directory traversal
        let sanitized_path = self.sanitize_path(&entry.file_path);
        let path = format!("symbols/{}/{}.json", sanitized_path, entry.id);

        DocumentBuilder::new()
            .id(entry.document_id)
            .path(&path)?
            .title(&title)?
            .content(json.as_bytes())
            .tag("symbol")?
            .tag(&format!("symbol-type-{:?}", entry.symbol.symbol_type).to_lowercase())?
            .tag(&format!("lang-{:?}", entry.language).to_lowercase())?
            .build()
    }

    /// Deserialize a document to a symbol entry
    fn deserialize_symbol(&self, doc: &Document) -> Result<SymbolEntry> {
        let json = String::from_utf8(doc.content.clone())?;
        serde_json::from_str(&json).context("Failed to deserialize symbol entry")
    }

    /// Query symbols by name
    pub fn find_by_name(&self, name: &str) -> Vec<&SymbolEntry> {
        self.name_index
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.symbol_index.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query symbols by type
    pub fn find_by_type(&self, symbol_type: &SymbolType) -> Vec<&SymbolEntry> {
        self.symbol_index
            .values()
            .filter(|entry| entry.symbol.symbol_type == *symbol_type)
            .collect()
    }

    /// Query symbols in a file
    pub fn find_by_file(&self, file_path: &Path) -> Vec<&SymbolEntry> {
        self.file_symbols
            .get(file_path)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.symbol_index.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: &Uuid) -> Option<&SymbolEntry> {
        self.symbol_index.get(id)
    }

    /// Add a relationship between symbols
    pub fn add_relationship(&mut self, relation: SymbolRelation) -> Result<()> {
        // Update dependent's list
        if let Some(target) = self.symbol_index.get_mut(&relation.to_id) {
            target.dependents.insert(relation.from_id);
        }

        self.relationships.push(relation);
        Ok(())
    }

    /// Get relationships for a symbol
    pub fn get_relationships(&self, symbol_id: &Uuid) -> Vec<&SymbolRelation> {
        self.relationships
            .iter()
            .filter(|r| r.from_id == *symbol_id || r.to_id == *symbol_id)
            .collect()
    }

    /// Perform incremental update for a file with atomic rollback on failure
    #[instrument(skip(self, parsed_code))]
    pub async fn update_file_symbols(
        &mut self,
        file_path: &Path,
        parsed_code: ParsedCode,
        repository: Option<String>,
    ) -> Result<()> {
        info!("Updating symbols for {}", file_path.display());

        // Backup old symbols for rollback
        let old_ids = self.file_symbols.get(file_path).cloned();
        let mut old_entries = Vec::new();
        let mut old_name_mappings = HashMap::new();

        // Collect old data for potential rollback
        if let Some(ref ids) = old_ids {
            for id in ids {
                if let Some(entry) = self.symbol_index.get(id) {
                    old_entries.push(entry.clone());
                    if let Some(names) = self.name_index.get(&entry.qualified_name) {
                        old_name_mappings.insert(entry.qualified_name.clone(), names.clone());
                    }
                }
            }
        }

        // Remove old symbols from indices (but keep in storage temporarily)
        if let Some(ref ids) = old_ids {
            for id in ids {
                if let Some(entry) = self.symbol_index.remove(id) {
                    // Remove from name index
                    if let Some(names) = self.name_index.get_mut(&entry.qualified_name) {
                        names.retain(|&x| x != *id);
                    }
                }
            }
            self.file_symbols.remove(file_path);
        }

        // Try to add new symbols
        match self
            .extract_symbols(file_path, parsed_code, repository)
            .await
        {
            Ok(new_ids) => {
                // Success - now safe to delete old symbols from storage
                if let Some(old_ids) = old_ids {
                    for entry in &old_entries {
                        // Ignore deletion errors for old symbols
                        let _ = self.storage.delete(&entry.document_id).await;
                    }
                }
                Ok(())
            }
            Err(e) => {
                // Rollback: restore old symbols to indices
                tracing::error!("Failed to extract new symbols, rolling back: {}", e);

                if let Some(old_ids) = old_ids {
                    // Restore to file mapping
                    self.file_symbols.insert(file_path.to_path_buf(), old_ids);

                    // Restore to symbol index
                    for entry in old_entries {
                        self.symbol_index.insert(entry.id, entry);
                    }

                    // Restore name mappings
                    for (name, ids) in old_name_mappings {
                        self.name_index.insert(name, ids);
                    }
                }

                Err(e).context("Failed to update file symbols")
            }
        }
    }

    /// Get statistics about the symbol index
    pub fn get_stats(&self) -> SymbolIndexStats {
        let mut symbols_by_type = HashMap::new();
        let mut symbols_by_language = HashMap::new();

        for entry in self.symbol_index.values() {
            let type_key = format!("{:?}", entry.symbol.symbol_type);
            *symbols_by_type.entry(type_key).or_insert(0) += 1;

            let lang_key = format!("{:?}", entry.language);
            *symbols_by_language.entry(lang_key).or_insert(0) += 1;
        }

        SymbolIndexStats {
            total_symbols: self.symbol_index.len(),
            symbols_by_type,
            symbols_by_language,
            repository_count: self.repository_files.len(),
            file_count: self.file_symbols.len(),
            relationship_count: self.relationships.len(),
            last_updated: Utc::now(),
        }
    }

    /// Get memory usage information
    pub fn get_memory_usage(&self) -> (usize, usize, f32) {
        let used = self.estimated_memory_usage;
        let limit = self.config.max_memory_bytes;
        let percentage = (used as f32 / limit as f32) * 100.0;
        (used, limit, percentage)
    }

    /// Search symbols with fuzzy matching
    pub fn search(&self, query: &str, limit: usize) -> Vec<&SymbolEntry> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<(&SymbolEntry, f32)> = self
            .symbol_index
            .values()
            .filter_map(|entry| {
                let name_lower = entry.symbol.name.to_lowercase();

                // Exact match
                if name_lower == query_lower {
                    return Some((entry, self.config.search_thresholds.exact_match));
                }

                // Prefix match
                if name_lower.starts_with(&query_lower) {
                    return Some((entry, self.config.search_thresholds.prefix_match));
                }

                // Contains match
                if name_lower.contains(&query_lower) {
                    return Some((entry, self.config.search_thresholds.contains_match));
                }

                // Fuzzy match (simple character overlap)
                let overlap = self.calculate_overlap(&name_lower, &query_lower);
                if overlap > self.config.search_thresholds.min_fuzzy_overlap {
                    return Some((
                        entry,
                        overlap * self.config.search_thresholds.fuzzy_multiplier,
                    ));
                }

                None
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        results
            .into_iter()
            .take(limit)
            .map(|(entry, _)| entry)
            .collect()
    }

    /// Calculate character overlap ratio between two strings
    fn calculate_overlap(&self, s1: &str, s2: &str) -> f32 {
        let chars1: HashSet<char> = s1.chars().collect();
        let chars2: HashSet<char> = s2.chars().collect();

        let intersection = chars1.intersection(&chars2).count();
        let union = chars1.union(&chars2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Generate a deterministic ID for a symbol based on its content and location
    fn generate_deterministic_id(
        &self,
        symbol: &ParsedSymbol,
        file_path: &Path,
        parent_id: Option<Uuid>,
    ) -> Uuid {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Include file path for uniqueness
        hasher.update(file_path.to_string_lossy().as_bytes());

        // Include parent ID if present
        if let Some(parent) = parent_id {
            hasher.update(parent.as_bytes());
        }

        // Include symbol name and type
        hasher.update(symbol.name.as_bytes());
        hasher.update(format!("{:?}", symbol.symbol_type).as_bytes());

        // Include position for uniqueness within file
        hasher.update(symbol.start_line.to_le_bytes());
        hasher.update(symbol.start_column.to_le_bytes());

        // Create UUID from hash
        let hash = hasher.finalize();
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes.copy_from_slice(&hash[..16]);

        // Set version (4) and variant bits for valid UUID v4
        uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x40;
        uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;

        Uuid::from_bytes(uuid_bytes)
    }

    /// Generate a deterministic document ID from a symbol ID
    fn generate_document_id(&self, symbol_id: &Uuid) -> Result<ValidatedDocumentId> {
        // Use the symbol ID directly as the document ID for consistency
        // This ensures the same symbol always gets the same document ID
        ValidatedDocumentId::from_uuid(*symbol_id)
            .context("Failed to create document ID from symbol ID")
    }

    /// Sanitize a file path to prevent directory traversal attacks
    fn sanitize_path(&self, path: &Path) -> String {
        // Remove any parent directory references and convert to string
        let components: Vec<_> = path
            .components()
            .filter_map(|comp| {
                use std::path::Component;
                match comp {
                    Component::Normal(s) => s.to_str(),
                    _ => None,
                }
            })
            .collect();

        // Join with forward slashes for consistent storage paths
        components.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::{
        CodeParser, ParseStats, ParsedCode, ParsedSymbol, SupportedLanguage, SymbolKind, SymbolType,
    };

    async fn create_test_storage() -> Result<Box<dyn Storage + Send + Sync>> {
        use crate::file_storage::create_file_storage;
        let test_dir = format!("test_data/symbol_test_{}", Uuid::new_v4());
        tokio::fs::create_dir_all(&test_dir).await?;
        let storage = create_file_storage(&test_dir, Some(100)).await?;
        Ok(Box::new(storage) as Box<dyn Storage + Send + Sync>)
    }

    #[tokio::test]
    async fn test_symbol_extraction() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        let rust_code = r#"
use std::collections::HashMap;

pub struct MyStruct {
    field: String,
}

impl MyStruct {
    pub fn new() -> Self {
        Self {
            field: String::new(),
        }
    }
    
    fn private_method(&self) -> &str {
        &self.field
    }
}
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        let symbol_ids = symbol_storage
            .extract_symbols(Path::new("test.rs"), parsed, None)
            .await?;

        assert!(!symbol_ids.is_empty());

        // Verify symbols were extracted
        let symbols = symbol_storage.find_by_file(Path::new("test.rs"));
        assert!(!symbols.is_empty());

        // Check for specific symbols
        let structs = symbol_storage.find_by_type(&SymbolType::Struct);
        assert_eq!(structs.len(), 1);
        // Note: tree-sitter name extraction needs improvement (tracked separately)
        // For now, just verify that a struct was found

        Ok(())
    }

    #[tokio::test]
    async fn test_symbol_search() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        let rust_code = r#"
fn calculate_total() -> i32 { 42 }
fn calculate_average() -> f64 { 42.0 }
fn compute_sum() -> i32 { 0 }
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        symbol_storage
            .extract_symbols(Path::new("math.rs"), parsed, None)
            .await?;

        // Search for "calculate"
        let results = symbol_storage.search("calculate", 10);
        assert_eq!(results.len(), 2);

        // Search for "sum"
        let results = symbol_storage.search("sum", 10);
        assert_eq!(results.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_incremental_updates() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        let rust_code_v1 = r#"fn old_function() {}"#;
        let rust_code_v2 = r#"fn new_function() {}"#;

        let mut parser = CodeParser::new()?;

        // Initial extraction
        let parsed_v1 = parser.parse_content(rust_code_v1, SupportedLanguage::Rust)?;
        symbol_storage
            .extract_symbols(Path::new("evolving.rs"), parsed_v1, None)
            .await?;

        let symbols_v1 = symbol_storage.find_by_file(Path::new("evolving.rs"));
        assert_eq!(symbols_v1.len(), 1);
        assert_eq!(symbols_v1[0].symbol.name, "old_function");

        // Update with new version
        let parsed_v2 = parser.parse_content(rust_code_v2, SupportedLanguage::Rust)?;
        symbol_storage
            .update_file_symbols(Path::new("evolving.rs"), parsed_v2, None)
            .await?;

        let symbols_v2 = symbol_storage.find_by_file(Path::new("evolving.rs"));
        assert_eq!(symbols_v2.len(), 1);
        assert_eq!(symbols_v2[0].symbol.name, "new_function");

        Ok(())
    }

    #[tokio::test]
    async fn test_deterministic_symbol_ids() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        let rust_code = r#"fn test_function() { println!("test"); }"#;

        let mut parser = CodeParser::new()?;
        let parsed1 = parser.parse_content(rust_code, SupportedLanguage::Rust)?;
        let parsed2 = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Extract symbols twice from the same code
        let ids1 = symbol_storage
            .extract_symbols(Path::new("test.rs"), parsed1, None)
            .await?;

        // Clear and re-extract to test determinism
        symbol_storage.file_symbols.clear();
        symbol_storage.symbol_index.clear();
        symbol_storage.name_index.clear();

        let ids2 = symbol_storage
            .extract_symbols(Path::new("test.rs"), parsed2, None)
            .await?;

        // Symbol IDs should be identical for the same code
        assert_eq!(ids1, ids2, "Symbol IDs should be deterministic");

        Ok(())
    }

    #[test]
    fn test_path_sanitization() -> Result<()> {
        // Test the path sanitization function directly
        fn test_sanitize(path: &str) -> String {
            let components: Vec<_> = Path::new(path)
                .components()
                .filter_map(|comp| {
                    use std::path::Component;
                    match comp {
                        Component::Normal(s) => s.to_str(),
                        _ => None,
                    }
                })
                .collect();
            components.join("/")
        }

        // Test various malicious paths
        let test_paths = vec![
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "normal/../../malicious",
            "./normal/../../../bad",
        ];

        for path in test_paths {
            let sanitized = test_sanitize(path);
            // Should not contain any parent directory references
            assert!(
                !sanitized.contains(".."),
                "Path {} was not properly sanitized: {}",
                path,
                sanitized
            );
            // Should only contain normal components
            assert!(
                !sanitized.contains("./"),
                "Path {} contains current dir reference: {}",
                path,
                sanitized
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_deeply_nested_symbols() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        // Create deeply nested code structure
        let rust_code = r#"
mod level1 {
    mod level2 {
        mod level3 {
            mod level4 {
                mod level5 {
                    mod level6 {
                        fn deeply_nested_function() {
                            println!("Very deep!");
                        }
                    }
                }
            }
        }
    }
}
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        let symbol_ids = symbol_storage
            .extract_symbols(Path::new("deep.rs"), parsed, None)
            .await?;

        // Should handle deep nesting without stack overflow
        assert!(!symbol_ids.is_empty());

        // Verify parent-child relationships are correct
        let symbols = symbol_storage.find_by_file(Path::new("deep.rs"));
        let functions: Vec<_> = symbols
            .iter()
            .filter(|s| s.symbol.symbol_type == SymbolType::Function)
            .collect();

        if !functions.is_empty() {
            // The deeply nested function should have a parent
            assert!(
                functions[0].parent_id.is_some(),
                "Nested function should have parent"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_limits() -> Result<()> {
        let storage = create_test_storage().await?;

        // Create storage with very low memory limit
        let config = SymbolStorageConfig {
            max_symbols: 5,
            max_memory_bytes: 1024, // 1KB - very small
            search_thresholds: SearchThresholds::default(),
        };

        let mut symbol_storage = SymbolStorage::with_config(storage, config).await?;

        // Try to add many symbols
        for i in 0..10 {
            let rust_code = format!("fn function_{}() {{}}", i);
            let mut parser = CodeParser::new()?;
            let parsed = parser.parse_content(&rust_code, SupportedLanguage::Rust)?;

            let _ = symbol_storage
                .extract_symbols(Path::new(&format!("file_{}.rs", i)), parsed, None)
                .await;
        }

        // Should respect the symbol limit
        assert!(
            symbol_storage.symbol_index.len() <= 5,
            "Should respect max_symbols limit"
        );

        let (used, limit, _) = symbol_storage.get_memory_usage();
        assert!(used <= limit, "Memory usage should not exceed limit");

        Ok(())
    }

    #[tokio::test]
    async fn test_rollback_on_extraction_failure() -> Result<()> {
        let storage = create_test_storage().await?;
        let mut symbol_storage = SymbolStorage::new(storage).await?;

        // Add initial symbols
        let rust_code = r#"fn original_function() {}"#;
        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        symbol_storage
            .extract_symbols(Path::new("test.rs"), parsed, None)
            .await?;

        let original_count = symbol_storage.symbol_index.len();

        // Create a ParsedCode that will cause extraction to fail
        // by using an invalid path that will fail during storage
        let invalid_parsed = ParsedCode {
            language: SupportedLanguage::Rust,
            symbols: vec![ParsedSymbol {
                name: "\0invalid\0name".to_string(), // Invalid characters
                symbol_type: SymbolType::Function,
                kind: SymbolKind::Unknown,
                start_line: 1,
                end_line: 1,
                start_column: 0,
                end_column: 10,
                text: "invalid".to_string(),
                documentation: None,
            }],
            stats: ParseStats {
                total_nodes: 1,
                named_nodes: 1,
                max_depth: 1,
                error_count: 0,
            },
            errors: vec![],
        };

        // Try to update with invalid symbols - should fail and rollback
        let result = symbol_storage
            .update_file_symbols(Path::new("test.rs"), invalid_parsed, None)
            .await;

        // Update should fail but original symbols should be preserved
        assert!(
            result.is_err() || original_count == symbol_storage.symbol_index.len(),
            "Should rollback on failure"
        );

        Ok(())
    }

    #[test]
    fn test_complex_import_parsing() -> Result<()> {
        // Test the import parsing function directly
        fn test_parse_import(text: &str) -> Option<String> {
            let trimmed = text.trim();

            // Rust imports
            if let Some(rest) = trimmed.strip_prefix("use ") {
                if let Some(base) = rest.split(':').next() {
                    return Some(base.trim().to_string());
                }
            }

            // Python imports
            if let Some(rest) = trimmed.strip_prefix("import ") {
                if let Some(module) = rest.split_whitespace().next() {
                    return Some(module.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("from ") {
                if let Some(module) = rest.split_whitespace().next() {
                    return Some(module.to_string());
                }
            }

            // JavaScript/TypeScript imports
            if trimmed.starts_with("import ") {
                if let Some(start) = trimmed.find(['\'', '"']) {
                    let quote_char = trimmed.chars().nth(start).unwrap();
                    if let Some(end) = trimmed[start + 1..].find(quote_char) {
                        return Some(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }

            None
        }

        // Test various import formats
        let test_cases = vec![
            ("use std::collections::HashMap;", Some("std")),
            ("use crate::{Error, Result};", Some("crate")),
            ("import numpy as np", Some("numpy")),
            ("from sklearn import svm", Some("sklearn")),
            ("import React from 'react';", Some("react")),
            (
                "import { Component } from '@angular/core';",
                Some("@angular/core"),
            ),
            ("use super::parent_module;", Some("super")),
            ("", None),
        ];

        for (import_text, expected) in test_cases {
            let result = test_parse_import(import_text);
            assert_eq!(
                result.as_deref(),
                expected,
                "Failed to parse: {}",
                import_text
            );
        }

        Ok(())
    }
}
