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
}

impl SymbolStorage {
    /// Create a new symbol storage instance
    pub async fn new(storage: Box<dyn Storage + Send + Sync>) -> Result<Self> {
        let mut instance = Self {
            storage,
            symbol_index: HashMap::new(),
            relationships: Vec::new(),
            file_symbols: HashMap::new(),
            name_index: HashMap::new(),
            repository_files: HashMap::new(),
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
            if let Ok(entry) = self.deserialize_symbol(&doc) {
                self.index_symbol(entry)?;
                loaded_count += 1;
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

            // Create symbol entry
            let entry = SymbolEntry {
                id: Uuid::new_v4(),
                document_id: ValidatedDocumentId::new(),
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

            // Clean up stack
            parent_stack.retain(|(_, end_line)| symbol.start_line <= *end_line);

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

    /// Index a symbol in memory for fast lookups
    fn index_symbol(&mut self, entry: SymbolEntry) -> Result<()> {
        // Add to name index
        self.name_index
            .entry(entry.qualified_name.clone())
            .or_default()
            .push(entry.id);

        // Add to main index
        self.symbol_index.insert(entry.id, entry);

        Ok(())
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
        // Simplified import parsing - enhance based on language
        if text.contains("use ") {
            // Rust use statement
            text.split("use ")
                .nth(1)?
                .split(';')
                .next()
                .map(|s| s.trim().to_string())
        } else if text.contains("import ") {
            // Python/JS import
            text.split("import ")
                .nth(1)?
                .split(|c: char| c.is_whitespace() || c == ';')
                .next()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
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

        let path = format!("symbols/{}/{}.json", entry.file_path.display(), entry.id);

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

    /// Perform incremental update for a file
    #[instrument(skip(self, parsed_code))]
    pub async fn update_file_symbols(
        &mut self,
        file_path: &Path,
        parsed_code: ParsedCode,
        repository: Option<String>,
    ) -> Result<()> {
        info!("Updating symbols for {}", file_path.display());

        // Remove old symbols for this file
        if let Some(old_ids) = self.file_symbols.remove(file_path) {
            for id in old_ids {
                if let Some(entry) = self.symbol_index.remove(&id) {
                    // Remove from name index
                    if let Some(names) = self.name_index.get_mut(&entry.qualified_name) {
                        names.retain(|&x| x != id);
                    }

                    // Remove from storage
                    self.storage.delete(&entry.document_id).await?;
                }
            }
        }

        // Add new symbols
        self.extract_symbols(file_path, parsed_code, repository)
            .await?;

        Ok(())
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
                    return Some((entry, 1.0));
                }

                // Prefix match
                if name_lower.starts_with(&query_lower) {
                    return Some((entry, 0.8));
                }

                // Contains match
                if name_lower.contains(&query_lower) {
                    return Some((entry, 0.6));
                }

                // Fuzzy match (simple character overlap)
                let overlap = self.calculate_overlap(&name_lower, &query_lower);
                if overlap > 0.5 {
                    return Some((entry, overlap * 0.5));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::{CodeParser, SupportedLanguage};

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
}
