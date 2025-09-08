//! Symbol-aware index implementation for code-specific searches
//!
//! This module provides a specialized index that integrates with the symbol extraction
//! pipeline to enable intelligent code searches including function signatures,
//! dependencies, and code patterns.

use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::contracts::{Index, Query, Storage};
use crate::parsing::SymbolType;
use crate::symbol_storage::{SymbolEntry, SymbolStorage};
use crate::types::{ValidatedDocumentId, ValidatedPath};

// Pre-compiled regex patterns for common searches
static ERROR_HANDLING_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(try|catch|Result|Error|panic|unwrap|expect)").unwrap());
static ASYNC_AWAIT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(async|await|tokio|futures|spawn)").unwrap());
static TEST_CODE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(#\[test\]|#\[cfg\(test\)]|assert|test_)").unwrap());
static TODO_COMMENTS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(TODO|FIXME|HACK|XXX|NOTE)").unwrap());
static SECURITY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(password|secret|key|token|auth|credential)").unwrap());

/// Code-specific query types for advanced searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeQuery {
    /// Find symbols by exact or fuzzy name match
    SymbolSearch {
        name: String,
        symbol_types: Option<Vec<SymbolType>>,
        fuzzy: bool,
    },
    /// Find functions by signature pattern
    SignatureSearch {
        pattern: String,
        language: Option<String>,
    },
    /// Find all imports/dependencies of a file or symbol
    DependencySearch {
        target: String,
        direction: DependencyDirection,
    },
    /// Find code patterns (e.g., error handling, async patterns)
    PatternSearch {
        pattern: CodePattern,
        scope: SearchScope,
    },
    /// Combined query with multiple criteria
    Combined {
        queries: Vec<CodeQuery>,
        operator: QueryOperator,
    },
}

/// Direction for dependency searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyDirection {
    /// What does this depend on?
    Dependencies,
    /// What depends on this?
    Dependents,
    /// Both directions
    Both,
}

/// Common code patterns to search for
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodePattern {
    /// Error handling patterns (try/catch, Result, etc.)
    ErrorHandling,
    /// Async/await patterns
    AsyncAwait,
    /// Test functions and assertions
    TestCode,
    /// TODO/FIXME comments
    TodoComments,
    /// Security-sensitive patterns (passwords, keys, etc.)
    SecurityPatterns,
    /// Custom regex pattern
    Custom(String),
}

/// Scope for pattern searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchScope {
    /// Search in function bodies only
    Functions,
    /// Search in comments only
    Comments,
    /// Search in imports/uses only
    Imports,
    /// Search everywhere
    All,
}

/// Logical operator for combined queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperator {
    And,
    Or,
    Not,
}

/// Symbol index that provides code-aware search capabilities
pub struct SymbolIndex {
    /// Underlying symbol storage
    symbol_storage: Arc<RwLock<SymbolStorage>>,
    /// Path to index directory
    #[allow(dead_code)]
    index_path: PathBuf,
    /// Cache for frequently accessed symbols
    #[allow(dead_code)]
    symbol_cache: RwLock<HashMap<Uuid, SymbolEntry>>,
    /// Inverted index for fast text searches
    text_index: RwLock<HashMap<String, HashSet<Uuid>>>,
    /// Signature patterns index
    signature_index: RwLock<HashMap<String, HashSet<Uuid>>>,
    /// Cache for compiled custom regex patterns
    regex_cache: RwLock<HashMap<String, Arc<Regex>>>,
    /// Configuration
    config: SymbolIndexConfig,
}

/// Configuration for the symbol index
#[derive(Debug, Clone)]
pub struct SymbolIndexConfig {
    /// Maximum cache size in entries
    pub cache_size: usize,
    /// Enable fuzzy matching
    pub enable_fuzzy: bool,
    /// Fuzzy match threshold (0.0 to 1.0)
    pub fuzzy_threshold: f32,
    /// Maximum results per query
    pub max_results: usize,
}

impl Default for SymbolIndexConfig {
    fn default() -> Self {
        Self {
            cache_size: 10000,
            enable_fuzzy: true,
            fuzzy_threshold: 0.7,
            max_results: 1000,
        }
    }
}

impl SymbolIndex {
    /// Create a new symbol index with default configuration
    pub async fn new(
        index_path: PathBuf,
        storage: Box<dyn crate::contracts::Storage + Send + Sync>,
    ) -> Result<Self> {
        Self::with_config(index_path, storage, SymbolIndexConfig::default()).await
    }

    /// Create a new symbol index with custom configuration
    pub async fn with_config(
        index_path: PathBuf,
        storage: Box<dyn crate::contracts::Storage + Send + Sync>,
        config: SymbolIndexConfig,
    ) -> Result<Self> {
        let symbol_storage = Arc::new(RwLock::new(SymbolStorage::new(storage).await?));

        let mut instance = Self {
            symbol_storage,
            index_path,
            symbol_cache: RwLock::new(HashMap::new()),
            text_index: RwLock::new(HashMap::new()),
            signature_index: RwLock::new(HashMap::new()),
            regex_cache: RwLock::new(HashMap::new()),
            config,
        };

        // Build initial indices
        instance.rebuild_indices().await?;

        Ok(instance)
    }

    /// Rebuild all indices from symbol storage
    async fn rebuild_indices(&mut self) -> Result<()> {
        let storage = self.symbol_storage.read().await;

        // Clear existing indices
        self.text_index.write().await.clear();
        self.signature_index.write().await.clear();

        // Rebuild text index - acquire locks in consistent order
        let mut text_index = self.text_index.write().await;
        let mut signature_index = self.signature_index.write().await;

        // Get all symbols by iterating through indexed files
        // Note: This is a simplified version - production would batch this
        let indexed_files = storage.get_indexed_files();
        for file_path in indexed_files {
            let symbols = storage.find_by_file(&file_path);
            for entry in symbols {
                let id = entry.id;

                // Index by name tokens
                for token in Self::tokenize(&entry.symbol.name) {
                    text_index.entry(token).or_default().insert(id);
                }

                // Index by signature tokens if it's a function
                if matches!(
                    entry.symbol.symbol_type,
                    SymbolType::Function | SymbolType::Method
                ) {
                    for token in Self::extract_signature_tokens(&entry.symbol.text) {
                        signature_index.entry(token).or_default().insert(id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove all index entries for a specific file
    async fn remove_file_from_indices(&mut self, file_path: &std::path::Path) -> Result<()> {
        let storage = self.symbol_storage.read().await;

        // Get all symbols for this file that need to be removed
        let symbols_to_remove = storage.find_by_file(file_path);

        // Acquire locks in consistent order
        let mut text_index = self.text_index.write().await;
        let mut signature_index = self.signature_index.write().await;

        // Remove each symbol from the indices
        for entry in symbols_to_remove {
            let id = entry.id;

            // Remove from text index
            for token in Self::tokenize(&entry.symbol.name) {
                if let Some(ids) = text_index.get_mut(&token) {
                    ids.remove(&id);
                    // Clean up empty entries
                    if ids.is_empty() {
                        text_index.remove(&token);
                    }
                }
            }

            // Remove from signature index if it's a function
            if matches!(
                entry.symbol.symbol_type,
                SymbolType::Function | SymbolType::Method
            ) {
                for token in Self::extract_signature_tokens(&entry.symbol.text) {
                    if let Some(ids) = signature_index.get_mut(&token) {
                        ids.remove(&id);
                        // Clean up empty entries
                        if ids.is_empty() {
                            signature_index.remove(&token);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Incrementally update indices for a specific file
    async fn update_indices_for_file(&mut self, file_path: &std::path::Path) -> Result<()> {
        let storage = self.symbol_storage.read().await;

        // Get symbols for this specific file
        let symbols = storage.find_by_file(file_path);

        // Update indices with consistent lock ordering
        let mut text_index = self.text_index.write().await;
        let mut signature_index = self.signature_index.write().await;

        // Note: Old entries should be removed by calling remove_file_from_indices() first

        // Add new entries
        for entry in symbols {
            let id = entry.id;

            // Index by name tokens
            for token in Self::tokenize(&entry.symbol.name) {
                text_index.entry(token).or_default().insert(id);
            }

            // Index by signature tokens if it's a function
            if matches!(
                entry.symbol.symbol_type,
                SymbolType::Function | SymbolType::Method
            ) {
                for token in Self::extract_signature_tokens(&entry.symbol.text) {
                    signature_index.entry(token).or_default().insert(id);
                }
            }
        }

        Ok(())
    }

    /// Tokenize a string for indexing
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    /// Extract signature tokens from function text
    fn extract_signature_tokens(text: &str) -> Vec<String> {
        // Extract parameter types and return types
        // This is a simplified version - real implementation would use AST
        let mut tokens = Vec::new();

        // Look for common type patterns
        let type_patterns = [
            "String", "str", "i32", "u32", "i64", "u64", "f32", "f64", "bool", "Vec", "HashMap",
            "Result", "Option", "Self",
        ];

        for pattern in &type_patterns {
            if text.contains(pattern) {
                tokens.push(pattern.to_lowercase());
            }
        }

        tokens
    }

    /// Execute a code-specific query
    pub async fn search_code(&self, query: &CodeQuery) -> Result<Vec<SymbolSearchResult>> {
        match query {
            CodeQuery::SymbolSearch {
                name,
                symbol_types,
                fuzzy,
            } => {
                self.search_symbols(name, symbol_types.as_ref(), *fuzzy)
                    .await
            }
            CodeQuery::SignatureSearch { pattern, language } => {
                self.search_signatures(pattern, language.as_deref()).await
            }
            CodeQuery::DependencySearch { target, direction } => {
                self.search_dependencies(target, direction).await
            }
            CodeQuery::PatternSearch { pattern, scope } => {
                self.search_patterns(pattern, scope).await
            }
            CodeQuery::Combined { queries, operator } => {
                self.search_combined(queries, operator).await
            }
        }
    }

    /// Search for symbols by name
    async fn search_symbols(
        &self,
        name: &str,
        symbol_types: Option<&Vec<SymbolType>>,
        fuzzy: bool,
    ) -> Result<Vec<SymbolSearchResult>> {
        let storage = self.symbol_storage.read().await;
        let mut results = Vec::new();

        if fuzzy && self.config.enable_fuzzy {
            // Use fuzzy search from symbol storage
            let matches = storage.search(name, self.config.max_results);
            for entry in matches {
                if let Some(types) = symbol_types {
                    if !types.contains(&entry.symbol.symbol_type) {
                        continue;
                    }
                }
                results.push(SymbolSearchResult::from_entry(entry.clone(), 1.0));
            }
        } else {
            // Exact match search
            let entries = storage.find_by_name(name);
            for entry in entries {
                if let Some(types) = symbol_types {
                    if !types.contains(&entry.symbol.symbol_type) {
                        continue;
                    }
                }
                results.push(SymbolSearchResult::from_entry(entry.clone(), 1.0));
            }
        }

        Ok(results)
    }

    /// Search for functions by signature pattern
    async fn search_signatures(
        &self,
        pattern: &str,
        language: Option<&str>,
    ) -> Result<Vec<SymbolSearchResult>> {
        let storage = self.symbol_storage.read().await;
        let signature_index = self.signature_index.read().await;
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        // Find symbols that match signature tokens
        for token in Self::tokenize(pattern) {
            if let Some(symbol_ids) = signature_index.get(&token) {
                for id in symbol_ids {
                    if seen.insert(*id) {
                        if let Some(entry) = storage.get_symbol(id) {
                            // Filter by language if specified using proper enum comparison
                            if let Some(lang) = language {
                                // Use the from_name method to parse language string
                                // This handles all supported languages dynamically
                                let language_matches = if let Some(parsed_lang) =
                                    crate::parsing::SupportedLanguage::from_name(lang)
                                {
                                    // Check if the entry's language matches the requested language
                                    entry.language == parsed_lang
                                } else {
                                    // Unknown/unsupported language requested
                                    // Log warning and skip - better than silently failing
                                    tracing::debug!(
                                        "Signature search requested for unsupported language: {}",
                                        lang
                                    );
                                    false
                                };

                                if !language_matches {
                                    continue;
                                }
                            }

                            // Calculate relevance based on pattern match
                            let relevance =
                                Self::calculate_signature_relevance(&entry.symbol.text, pattern);
                            results.push(SymbolSearchResult::from_entry(entry.clone(), relevance));
                        }
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        results.truncate(self.config.max_results);

        Ok(results)
    }

    /// Calculate relevance score for signature matching
    fn calculate_signature_relevance(signature: &str, pattern: &str) -> f32 {
        let sig_tokens = Self::tokenize(signature);
        let pattern_tokens = Self::tokenize(pattern);

        if sig_tokens.is_empty() || pattern_tokens.is_empty() {
            return 0.0;
        }

        let matches = pattern_tokens
            .iter()
            .filter(|t| sig_tokens.contains(t))
            .count();

        matches as f32 / pattern_tokens.len() as f32
    }

    /// Search for dependencies
    async fn search_dependencies(
        &self,
        target: &str,
        direction: &DependencyDirection,
    ) -> Result<Vec<SymbolSearchResult>> {
        let storage = self.symbol_storage.read().await;
        let mut results = Vec::new();

        // Find the target symbol
        let target_symbols = storage.find_by_name(target);

        for target_symbol in target_symbols {
            match direction {
                DependencyDirection::Dependencies => {
                    // Find what this symbol depends on
                    // Note: Dependencies are stored as strings, not as symbol IDs
                    // We need to look them up to get proper symbol information
                    for dep in &target_symbol.dependencies {
                        // Try to find the actual symbol for this dependency
                        let dep_symbols = storage.find_by_name(dep);
                        if let Some(dep_symbol) = dep_symbols.first() {
                            results
                                .push(SymbolSearchResult::from_entry((*dep_symbol).clone(), 1.0));
                        } else {
                            // If we can't find the symbol, create a deterministic placeholder
                            // Use a hash of the dependency name for consistent IDs
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};

                            let mut hasher = DefaultHasher::new();
                            dep.hash(&mut hasher);
                            let hash = hasher.finish();

                            // Create a deterministic UUID from the hash
                            let bytes = hash.to_be_bytes();
                            let mut uuid_bytes = [0u8; 16];
                            uuid_bytes[..8].copy_from_slice(&bytes);
                            uuid_bytes[8..].copy_from_slice(&bytes); // Duplicate for full UUID

                            results.push(SymbolSearchResult {
                                symbol_id: Uuid::from_bytes(uuid_bytes),
                                document_id: target_symbol.document_id,
                                symbol_name: dep.clone(),
                                symbol_type: SymbolType::Import,
                                file_path: target_symbol.file_path.clone(),
                                qualified_name: dep.clone(),
                                relevance: 0.5, // Lower relevance for unresolved dependencies
                                metadata: {
                                    let mut meta = HashMap::new();
                                    meta.insert("unresolved".to_string(), "true".to_string());
                                    meta
                                },
                            });
                        }
                    }
                }
                DependencyDirection::Dependents => {
                    // Find what depends on this symbol
                    for dependent_id in &target_symbol.dependents {
                        if let Some(dependent) = storage.get_symbol(dependent_id) {
                            results.push(SymbolSearchResult::from_entry(dependent.clone(), 1.0));
                        }
                    }
                }
                DependencyDirection::Both => {
                    // Include both directions without recursion
                    // Dependencies
                    for dep in &target_symbol.dependencies {
                        let dep_symbols = storage.find_by_name(dep);
                        if let Some(dep_symbol) = dep_symbols.first() {
                            results
                                .push(SymbolSearchResult::from_entry((*dep_symbol).clone(), 1.0));
                        } else {
                            // Create deterministic placeholder for unresolved dependency
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};

                            let mut hasher = DefaultHasher::new();
                            dep.hash(&mut hasher);
                            let hash = hasher.finish();

                            let bytes = hash.to_be_bytes();
                            let mut uuid_bytes = [0u8; 16];
                            uuid_bytes[..8].copy_from_slice(&bytes);
                            uuid_bytes[8..].copy_from_slice(&bytes);

                            results.push(SymbolSearchResult {
                                symbol_id: Uuid::from_bytes(uuid_bytes),
                                document_id: target_symbol.document_id,
                                symbol_name: dep.clone(),
                                symbol_type: SymbolType::Import,
                                file_path: target_symbol.file_path.clone(),
                                qualified_name: dep.clone(),
                                relevance: 0.5,
                                metadata: {
                                    let mut meta = HashMap::new();
                                    meta.insert("unresolved".to_string(), "true".to_string());
                                    meta
                                },
                            });
                        }
                    }
                    // Dependents
                    for dependent_id in &target_symbol.dependents {
                        if let Some(dependent) = storage.get_symbol(dependent_id) {
                            results.push(SymbolSearchResult::from_entry(dependent.clone(), 1.0));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Search for code patterns
    async fn search_patterns(
        &self,
        pattern: &CodePattern,
        scope: &SearchScope,
    ) -> Result<Vec<SymbolSearchResult>> {
        let storage = self.symbol_storage.read().await;
        let mut results = Vec::new();

        // Get the regex pattern to use
        let regex_arc: Arc<Regex> = match pattern {
            CodePattern::ErrorHandling => Arc::new(ERROR_HANDLING_PATTERN.clone()),
            CodePattern::AsyncAwait => Arc::new(ASYNC_AWAIT_PATTERN.clone()),
            CodePattern::TestCode => Arc::new(TEST_CODE_PATTERN.clone()),
            CodePattern::TodoComments => Arc::new(TODO_COMMENTS_PATTERN.clone()),
            CodePattern::SecurityPatterns => Arc::new(SECURITY_PATTERN.clone()),
            CodePattern::Custom(regex_str) => {
                // Check cache first
                let cache = self.regex_cache.read().await;
                if let Some(cached_regex) = cache.get(regex_str) {
                    cached_regex.clone()
                } else {
                    drop(cache);
                    // Compile and cache the regex
                    match Regex::new(regex_str) {
                        Ok(compiled_regex) => {
                            let arc_regex = Arc::new(compiled_regex);
                            let mut cache = self.regex_cache.write().await;
                            cache.insert(regex_str.clone(), arc_regex.clone());
                            arc_regex
                        }
                        Err(e) => {
                            // Return empty results for invalid patterns instead of crashing
                            tracing::warn!("Invalid custom regex pattern '{}': {}", regex_str, e);
                            return Ok(Vec::new());
                        }
                    }
                }
            }
        };

        // Search through symbols based on scope
        let indexed_files = storage.get_indexed_files();
        for file_path in indexed_files {
            let symbols = storage.find_by_file(&file_path);
            for entry in symbols {
                let should_search = match scope {
                    SearchScope::Functions => matches!(
                        entry.symbol.symbol_type,
                        SymbolType::Function | SymbolType::Method
                    ),
                    SearchScope::Comments => entry.symbol.symbol_type == SymbolType::Comment,
                    SearchScope::Imports => entry.symbol.symbol_type == SymbolType::Import,
                    SearchScope::All => true,
                };

                if should_search && regex_arc.is_match(&entry.symbol.text) {
                    results.push(SymbolSearchResult::from_entry(entry.clone(), 1.0));
                }
            }
        }

        results.truncate(self.config.max_results);
        Ok(results)
    }

    /// Execute combined queries
    async fn search_combined(
        &self,
        queries: &[CodeQuery],
        operator: &QueryOperator,
    ) -> Result<Vec<SymbolSearchResult>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();
        for query in queries {
            // Manually handle each query type to avoid recursion
            let result = match query {
                CodeQuery::SymbolSearch {
                    name,
                    symbol_types,
                    fuzzy,
                } => {
                    self.search_symbols(name, symbol_types.as_ref(), *fuzzy)
                        .await?
                }
                CodeQuery::SignatureSearch { pattern, language } => {
                    self.search_signatures(pattern, language.as_deref()).await?
                }
                CodeQuery::DependencySearch { target, direction } => {
                    self.search_dependencies(target, direction).await?
                }
                CodeQuery::PatternSearch { pattern, scope } => {
                    self.search_patterns(pattern, scope).await?
                }
                CodeQuery::Combined { .. } => {
                    // Don't allow nested combined queries to avoid complexity
                    Vec::new()
                }
            };
            all_results.push(result);
        }

        match operator {
            QueryOperator::And => {
                // Intersection of all results
                let mut result_map: HashMap<Uuid, SymbolSearchResult> = HashMap::new();

                // Start with first query results
                for result in &all_results[0] {
                    result_map.insert(result.symbol_id, result.clone());
                }

                // Keep only results that appear in all queries
                for results in &all_results[1..] {
                    let current_ids: HashSet<Uuid> = results.iter().map(|r| r.symbol_id).collect();
                    result_map.retain(|id, _| current_ids.contains(id));
                }

                Ok(result_map.into_values().collect())
            }
            QueryOperator::Or => {
                // Union of all results
                let mut result_map: HashMap<Uuid, SymbolSearchResult> = HashMap::new();

                for results in all_results {
                    for result in results {
                        result_map
                            .entry(result.symbol_id)
                            .and_modify(|r| r.relevance = r.relevance.max(result.relevance))
                            .or_insert(result);
                    }
                }

                Ok(result_map.into_values().collect())
            }
            QueryOperator::Not => {
                // First query minus all others
                if all_results.is_empty() {
                    return Ok(Vec::new());
                }

                let mut result_map: HashMap<Uuid, SymbolSearchResult> = HashMap::new();
                for result in &all_results[0] {
                    result_map.insert(result.symbol_id, result.clone());
                }

                // Remove results from other queries
                for results in &all_results[1..] {
                    for result in results {
                        result_map.remove(&result.symbol_id);
                    }
                }

                Ok(result_map.into_values().collect())
            }
        }
    }
}

/// Result of a symbol search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSearchResult {
    pub symbol_id: Uuid,
    pub document_id: ValidatedDocumentId,
    pub symbol_name: String,
    pub symbol_type: SymbolType,
    pub file_path: PathBuf,
    pub qualified_name: String,
    pub relevance: f32,
    pub metadata: HashMap<String, String>,
}

impl SymbolSearchResult {
    /// Create from a symbol entry
    fn from_entry(entry: SymbolEntry, relevance: f32) -> Self {
        Self {
            symbol_id: entry.id,
            document_id: entry.document_id,
            symbol_name: entry.symbol.name.clone(),
            symbol_type: entry.symbol.symbol_type,
            file_path: entry.file_path,
            qualified_name: entry.qualified_name,
            relevance,
            metadata: HashMap::new(),
        }
    }
}

#[async_trait]
impl Index for SymbolIndex {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        use crate::file_storage::create_file_storage;

        let storage = create_file_storage(path, Some(1000)).await?;
        let index_path = PathBuf::from(path);

        Self::new(index_path, Box::new(storage)).await
    }

    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For symbol index, we need the actual content to extract symbols
        // This basic insert just records the mapping
        Ok(())
    }

    async fn insert_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // Parse the content and extract symbols
        use crate::parsing::{CodeParser, SupportedLanguage};

        // Detect language from path extension
        let path_str = path.as_str();
        let language = if path_str.ends_with(".rs") {
            SupportedLanguage::Rust
        } else if path_str.ends_with(".py") {
            // Python not supported yet
            return Ok(());
        } else if path_str.ends_with(".js") || path_str.ends_with(".ts") {
            // JavaScript not supported yet
            return Ok(());
        } else {
            // Skip non-code files
            return Ok(());
        };

        let content_str = String::from_utf8_lossy(content);
        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(&content_str, language)?;

        // First, remove any existing symbols for this file to prevent stale data
        let file_path = std::path::Path::new(path.as_str());
        self.remove_file_from_indices(file_path).await?;

        // Extract symbols
        let mut storage = self.symbol_storage.write().await;
        let _symbol_ids = storage
            .extract_symbols(file_path, parsed, None, None)
            .await?;

        // Use incremental update instead of full rebuild
        drop(storage);
        self.update_indices_for_file(file_path).await?;

        Ok(())
    }

    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For symbol index, we need content to update
        Ok(())
    }

    async fn update_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // Parse the content and extract symbols
        use crate::parsing::{CodeParser, SupportedLanguage};

        // Detect language from path extension
        let path_str = path.as_str();
        let language = if path_str.ends_with(".rs") {
            SupportedLanguage::Rust
        } else if path_str.ends_with(".py") {
            // Python not supported yet
            return Ok(());
        } else if path_str.ends_with(".js") || path_str.ends_with(".ts") {
            // JavaScript not supported yet
            return Ok(());
        } else {
            // Skip non-code files
            return Ok(());
        };

        let content_str = String::from_utf8_lossy(content);
        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(&content_str, language)?;

        // First, remove old symbols for this file to prevent stale data
        let file_path = std::path::Path::new(path.as_str());
        self.remove_file_from_indices(file_path).await?;

        // Extract new symbols
        let mut storage = self.symbol_storage.write().await;
        let _symbol_ids = storage
            .extract_symbols(file_path, parsed, None, None)
            .await?;

        // Use incremental update instead of full rebuild
        drop(storage);
        self.update_indices_for_file(file_path).await?;

        Ok(())
    }

    async fn delete(&mut self, _id: &ValidatedDocumentId) -> Result<bool> {
        // Remove symbols associated with this document
        // TODO: Implement deletion logic
        Ok(true)
    }

    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        // Convert standard query to code query and execute
        let mut results = HashSet::new();

        for term in &query.search_terms {
            let code_query = CodeQuery::SymbolSearch {
                name: term.as_str().to_string(),
                symbol_types: None,
                fuzzy: true,
            };

            let search_results = self.search_code(&code_query).await?;
            for result in search_results {
                results.insert(result.document_id);
            }
        }

        Ok(results.into_iter().collect())
    }

    async fn sync(&mut self) -> Result<()> {
        let mut storage = self.symbol_storage.write().await;
        storage.sync_storage().await
    }

    async fn flush(&mut self) -> Result<()> {
        let mut storage = self.symbol_storage.write().await;
        storage.flush_storage().await
    }

    async fn close(mut self) -> Result<()> {
        // Ensure indices are synced before closing
        self.sync().await?;

        // Clear indices to free memory
        self.text_index.write().await.clear();
        self.signature_index.write().await.clear();
        self.regex_cache.write().await.clear();

        // Try to get exclusive access to storage for cleanup
        match Arc::try_unwrap(self.symbol_storage) {
            Ok(storage) => {
                // We have exclusive access, can close properly
                tracing::debug!("Closing symbol index with exclusive storage access");
                storage.into_inner().close_storage().await
            }
            Err(arc_storage) => {
                // There are still references, but we've done our best to clean up
                let strong_count = Arc::strong_count(&arc_storage);
                let weak_count = Arc::weak_count(&arc_storage);
                tracing::warn!(
                    "Cannot close symbol index storage: {} strong references, {} weak references remain. Storage will be closed when last reference is dropped.",
                    strong_count, weak_count
                );
                // Attempt to at least sync the storage
                if let Ok(mut storage) = arc_storage.try_write() {
                    let _ = storage.sync_storage().await;
                }
                Ok(())
            }
        }
    }
}

/// Create a production-ready symbol index with metering wrapper
///
/// Returns a symbol index instance wrapped with MeteredIndex for:
/// - Performance metrics collection
/// - Operation counting
/// - Latency tracking
///
/// # Arguments
/// * `path` - Directory for storing index data
/// * `storage` - Storage backend to use
pub async fn create_symbol_index(
    path: &str,
    storage: Box<dyn Storage + Send + Sync>,
) -> Result<crate::wrappers::MeteredIndex<SymbolIndex>> {
    use crate::validation;
    use crate::wrappers::MeteredIndex;

    // Validate path for internal storage
    validation::path::validate_storage_directory_path(path)?;

    // Create the base index
    let index = SymbolIndex::new(PathBuf::from(path), storage).await?;

    // Wrap with metering for production use
    Ok(MeteredIndex::new(index, "symbol_index".to_string()))
}

/// Create a test symbol index for unit tests
///
/// Returns an unwrapped symbol index for testing
pub async fn create_symbol_index_for_tests(
    path: &str,
    storage: Box<dyn Storage + Send + Sync>,
) -> Result<SymbolIndex> {
    use crate::validation;

    validation::path::validate_storage_directory_path(path)?;
    SymbolIndex::new(PathBuf::from(path), storage).await
}

#[cfg(test)]
mod tests {
    use super::*;
    // Imports for tests only
    #[allow(unused_imports)]
    use crate::parsing::{CodeParser, SupportedLanguage};

    #[tokio::test]
    async fn test_symbol_index_creation() -> Result<()> {
        let test_dir = format!("test_data/symbol_index_{}", uuid::Uuid::new_v4());
        tokio::fs::create_dir_all(&test_dir).await?;

        let storage = crate::file_storage::create_file_storage(&test_dir, Some(100)).await?;
        let index = SymbolIndex::new(PathBuf::from(&test_dir), Box::new(storage)).await?;

        assert_eq!(index.config.max_results, 1000);

        // Cleanup
        tokio::fs::remove_dir_all(&test_dir).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_code_query_search() -> Result<()> {
        let test_dir = format!("test_data/symbol_index_search_{}", uuid::Uuid::new_v4());
        tokio::fs::create_dir_all(&test_dir).await?;

        let storage = crate::file_storage::create_file_storage(&test_dir, Some(100)).await?;
        let mut index = SymbolIndex::new(PathBuf::from(&test_dir), Box::new(storage)).await?;

        // Add some test code
        let rust_code = r#"
fn calculate_total() -> i32 {
    42
}

fn calculate_average() -> f64 {
    42.0
}
"#;

        let id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("test.rs")?;
        index
            .insert_with_content(id, path, rust_code.as_bytes())
            .await?;

        // Search for "calculate"
        let query = CodeQuery::SymbolSearch {
            name: "calculate".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: true,
        };

        let results = index.search_code(&query).await?;
        assert_eq!(results.len(), 2);

        // Cleanup
        tokio::fs::remove_dir_all(&test_dir).await?;
        Ok(())
    }
}
