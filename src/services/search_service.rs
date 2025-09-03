// SearchService - Unified search functionality for CLI, MCP, and API interfaces
//
// This service extracts search logic from main.rs to enable feature parity
// across all KotaDB interfaces while maintaining identical behavior.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::{
    binary_symbols::BinarySymbolReader,
    llm_search::{ContextConfig, LLMSearchEngine, LLMSearchResponse, RelevanceConfig},
    Document, Index, Storage, ValidatedDocumentId,
};

// Trait for database access needed by SearchService
// This allows the service to work with the Database from main.rs
pub trait DatabaseAccess: Send + Sync {
    fn storage(&self) -> Arc<Mutex<dyn Storage>>;
    fn primary_index(&self) -> Arc<Mutex<dyn Index>>;
    fn trigram_index(&self) -> Arc<Mutex<dyn Index>>;
    fn path_cache(&self) -> Arc<RwLock<HashMap<String, ValidatedDocumentId>>>;
}

/// Configuration options for content search
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub limit: usize,
    pub tags: Option<Vec<String>>,
    pub context: String,
    pub quiet: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 10,
            tags: None,
            context: "medium".to_string(),
            quiet: false,
        }
    }
}

/// Configuration options for symbol search
#[derive(Debug, Clone)]
pub struct SymbolSearchOptions {
    pub pattern: String,
    pub limit: usize,
    pub symbol_type: Option<String>,
    pub quiet: bool,
}

impl Default for SymbolSearchOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            limit: 25,
            symbol_type: None,
            quiet: false,
        }
    }
}

/// Search result for content search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub documents: Vec<Document>,
    pub total_count: usize,
    pub llm_response: Option<LLMSearchResponse>,
    pub search_type: SearchType,
}

/// Search result for symbol search
#[derive(Debug, Clone)]
pub struct SymbolResult {
    pub matches: Vec<SymbolMatch>,
    pub total_symbols: usize,
}

/// Individual symbol match
#[derive(Debug, Clone)]
pub struct SymbolMatch {
    pub name: String,
    pub file_path: String,
    pub start_line: u32,
    pub kind: String,
}

/// Type of search performed
#[derive(Debug, Clone)]
pub enum SearchType {
    LLMOptimized,
    RegularSearch,
    WildcardSearch,
}

/// Unified search service that handles both content and symbol search
pub struct SearchService<'a> {
    database: &'a dyn DatabaseAccess,
    symbol_db_path: PathBuf,
}

impl<'a> SearchService<'a> {
    /// Create a new SearchService instance
    pub fn new(database: &'a dyn DatabaseAccess, symbol_db_path: PathBuf) -> Self {
        Self {
            database,
            symbol_db_path,
        }
    }

    /// Search for content using the same logic as CLI SearchCode command
    pub async fn search_content(&self, options: SearchOptions) -> Result<SearchResult> {
        // Handle empty query
        if options.query.is_empty() {
            return Ok(SearchResult {
                documents: vec![],
                total_count: 0,
                llm_response: None,
                search_type: SearchType::RegularSearch,
            });
        }

        // Use LLM-optimized search for non-wildcard queries when content is not minimal
        if options.query != "*" && options.context != "none" {
            // Try LLM-optimized search with fallback to regular search on error
            match self.try_llm_search(&options).await {
                Ok(response) => {
                    return Ok(SearchResult {
                        documents: vec![], // Documents are embedded in LLM response
                        total_count: response.optimization.total_matches,
                        llm_response: Some(response),
                        search_type: SearchType::LLMOptimized,
                    });
                }
                Err(_) => {
                    // Fall back to regular search
                    let (documents, total_count) = self
                        .regular_search(&options.query, &options.tags, options.limit)
                        .await?;
                    return Ok(SearchResult {
                        documents,
                        total_count,
                        llm_response: None,
                        search_type: SearchType::RegularSearch,
                    });
                }
            }
        }

        // Use regular search for wildcard or when context is none
        let (documents, total_count) = self
            .regular_search(&options.query, &options.tags, options.limit)
            .await?;

        Ok(SearchResult {
            documents,
            total_count,
            llm_response: None,
            search_type: if options.query == "*" {
                SearchType::WildcardSearch
            } else {
                SearchType::RegularSearch
            },
        })
    }

    /// Search for symbols using the same logic as CLI SearchSymbols command
    pub async fn search_symbols(&self, options: SymbolSearchOptions) -> Result<SymbolResult> {
        let symbol_db_path = self.symbol_db_path.join("symbols.kota");

        // Check if symbols database exists
        if !symbol_db_path.exists() {
            return Ok(SymbolResult {
                matches: vec![],
                total_symbols: 0,
            });
        }

        // Open binary symbol reader
        let reader = BinarySymbolReader::open(&symbol_db_path)?;
        let total_symbols = reader.symbol_count();

        if total_symbols == 0 {
            return Ok(SymbolResult {
                matches: vec![],
                total_symbols: 0,
            });
        }

        // Search symbols using the same logic as main.rs
        let mut matches = Vec::new();
        let mut seen_symbols = HashSet::new();
        let pattern_lower = options.pattern.to_lowercase();

        for packed_symbol in reader.iter_symbols() {
            if let Ok(symbol_name) = reader.get_symbol_name(&packed_symbol) {
                let symbol_name_lower = symbol_name.to_lowercase();

                // Match against pattern - check for wildcards first, then substring
                let is_match = if pattern_lower.contains('*') {
                    matches_wildcard_pattern(&symbol_name_lower, &pattern_lower)
                } else {
                    symbol_name_lower.contains(&pattern_lower)
                };

                if is_match {
                    // Filter by type if specified
                    if let Some(ref filter_type) = options.symbol_type {
                        let filter_lower = filter_type.to_lowercase();
                        let type_str = format!("{}", packed_symbol.kind).to_lowercase();
                        if !type_str.contains(&filter_lower) {
                            continue;
                        }
                    }

                    // Get file path for display
                    let file_path = reader
                        .get_symbol_file_path(&packed_symbol)
                        .unwrap_or_else(|_| "<unknown>".to_string());

                    // Create a unique key for deduplication (name + file + line)
                    let unique_key =
                        format!("{}:{}:{}", symbol_name, file_path, packed_symbol.start_line);

                    // Only add if we haven't seen this exact symbol before
                    if seen_symbols.insert(unique_key) {
                        matches.push(SymbolMatch {
                            name: symbol_name,
                            file_path,
                            start_line: packed_symbol.start_line,
                            kind: format!("{}", packed_symbol.kind),
                        });

                        if matches.len() >= options.limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(SymbolResult {
            matches,
            total_symbols,
        })
    }

    /// Perform LLM-optimized search
    async fn try_llm_search(&self, options: &SearchOptions) -> Result<LLMSearchResponse> {
        // Create LLM search engine with appropriate context configuration
        let context_config = match options.context.as_str() {
            "none" | "minimal" => ContextConfig {
                token_budget: 2000,
                max_snippet_chars: 200,
                match_context_size: 30,
                ..Default::default()
            },
            "medium" => ContextConfig {
                token_budget: 4000,
                max_snippet_chars: 500,
                match_context_size: 50,
                ..Default::default()
            },
            "full" => ContextConfig {
                token_budget: 8000,
                max_snippet_chars: 1000,
                match_context_size: 100,
                ..Default::default()
            },
            _ => ContextConfig::default(),
        };

        let llm_engine = LLMSearchEngine::with_config(RelevanceConfig::default(), context_config);

        // Perform LLM-optimized search
        let storage_arc = self.database.storage();
        let trigram_index_arc = self.database.trigram_index();
        let storage = storage_arc.lock().await;
        let trigram_index = trigram_index_arc.lock().await;

        llm_engine
            .search_optimized(
                &options.query,
                &*storage,
                &*trigram_index,
                Some(options.limit),
            )
            .await
    }

    /// Perform regular search using the database - same logic as Database::search_with_count
    async fn regular_search(
        &self,
        query: &str,
        tags: &Option<Vec<String>>,
        limit: usize,
    ) -> Result<(Vec<Document>, usize)> {
        use crate::QueryBuilder;

        // Handle empty queries
        if query.is_empty() {
            return Ok((Vec::new(), 0));
        }

        // Build query
        let mut query_builder = QueryBuilder::new();

        if query != "*" {
            query_builder = query_builder.with_text(query)?;
        }

        if let Some(tag_list) = tags {
            for tag in tag_list {
                query_builder = query_builder.with_tag(tag.clone())?;
            }
        }

        query_builder = query_builder.with_limit(limit)?;
        let query_obj = query_builder.build()?;

        // Route to appropriate index based on query type
        let doc_ids = if query.contains('*') {
            // Use Primary Index for wildcard/pattern queries
            self.database
                .primary_index()
                .lock()
                .await
                .search(&query_obj)
                .await?
        } else {
            // Use Trigram Index for full-text search queries
            self.database
                .trigram_index()
                .lock()
                .await
                .search(&query_obj)
                .await?
        };

        // Store total count before limiting
        let total_count = doc_ids.len();

        // Retrieve documents from storage
        let doc_ids_limited: Vec<_> = doc_ids.into_iter().take(limit).collect();
        let mut documents = Vec::with_capacity(doc_ids_limited.len());
        let storage_arc = self.database.storage();
        let storage = storage_arc.lock().await;

        for doc_id in doc_ids_limited {
            if let Some(doc) = storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        Ok((documents, total_count))
    }
}

/// Match a string against a wildcard pattern
/// Copied from main.rs to maintain identical behavior
fn matches_wildcard_pattern(text: &str, pattern: &str) -> bool {
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
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
            continue;
        }

        // Last part must match at end unless pattern ends with *
        if i == parts.len() - 1 && !pattern.ends_with('*') {
            if !text.ends_with(part) {
                return false;
            }
            continue;
        }

        // Middle parts must exist in order
        if let Some(found_pos) = text[pos..].find(part) {
            pos += found_pos + part.len();
        } else {
            return false;
        }
    }

    true
}
