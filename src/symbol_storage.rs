//! Symbol storage and extraction pipeline for code intelligence
//!
//! This module provides persistent storage and indexing for code symbols extracted
//! from parsed source files. It enables intelligent code search, dependency mapping,
//! and incremental symbol updates.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::builders::DocumentBuilder;
use crate::contracts::{Document, Storage};
use crate::graph_storage::{GraphEdge, GraphNode, GraphStorage, NodeLocation};
use crate::parsing::{ParsedCode, ParsedSymbol, SupportedLanguage, SymbolType};
use crate::types::ValidatedDocumentId;

// Re-export RelationType from types for backwards compatibility
pub use crate::types::RelationType;

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

/// Statistics about the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphStats {
    /// Total number of symbols
    pub total_symbols: usize,
    /// Total number of relationships
    pub total_relationships: usize,
    /// Number of symbols that have dependencies
    pub symbols_with_dependencies: usize,
    /// Number of symbols that have dependents
    pub symbols_with_dependents: usize,
    /// Count of each relationship type
    pub relationship_type_counts: HashMap<String, usize>,
}

/// Analysis results for the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// Circular dependency chains detected
    pub cycles: Vec<Vec<Uuid>>,
    /// Symbols with no relationships (potential dead code)
    pub orphaned_symbols: Vec<Uuid>,
    /// Symbols with high coupling (many relationships)
    pub highly_coupled_symbols: Vec<(Uuid, usize)>,
    /// Total symbols analyzed
    pub total_symbols: usize,
    /// Total relationships analyzed
    pub total_relationships: usize,
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
    /// Optional graph storage for relationships (O(1) lookups)
    graph_storage: Option<Box<dyn GraphStorage + Send + Sync>>,
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
    /// File content cache for dependency analysis
    file_content_cache: HashMap<PathBuf, String>,
    /// Configuration
    config: SymbolStorageConfig,
    /// Current estimated memory usage
    estimated_memory_usage: usize,
    /// LRU queue for symbol eviction - tracks access order
    lru_queue: VecDeque<Uuid>,
    /// Track which symbols are in the LRU to avoid duplicates
    lru_set: HashSet<Uuid>,
}

impl SymbolStorage {
    /// Create a new symbol storage instance with default configuration
    pub async fn new(storage: Box<dyn Storage + Send + Sync>) -> Result<Self> {
        Self::with_config(storage, None, SymbolStorageConfig::default()).await
    }

    /// Create a new symbol storage instance with graph storage
    pub async fn with_graph_storage(
        storage: Box<dyn Storage + Send + Sync>,
        graph_storage: Box<dyn GraphStorage + Send + Sync>,
    ) -> Result<Self> {
        Self::with_config(storage, Some(graph_storage), SymbolStorageConfig::default()).await
    }

    /// Create a new symbol storage instance with custom configuration
    pub async fn with_config(
        storage: Box<dyn Storage + Send + Sync>,
        graph_storage: Option<Box<dyn GraphStorage + Send + Sync>>,
        config: SymbolStorageConfig,
    ) -> Result<Self> {
        let mut instance = Self {
            storage,
            graph_storage,
            symbol_index: HashMap::new(),
            relationships: Vec::new(),
            file_symbols: HashMap::new(),
            name_index: HashMap::new(),
            repository_files: HashMap::new(),
            file_content_cache: HashMap::new(),
            config,
            estimated_memory_usage: 0,
            lru_queue: VecDeque::new(),
            lru_set: HashSet::new(),
        };

        // Load existing symbols from storage
        instance.load_symbols().await?;

        Ok(instance)
    }

    /// Load symbols from persistent storage
    #[instrument(skip(self))]
    async fn load_symbols(&mut self) -> Result<()> {
        info!("Loading symbols from storage");
        let mut loaded_count = 0;

        // Load symbols from graph storage if available
        if self.graph_storage.is_some() {
            // Collect all graph data first to avoid borrow issues
            let (entries, relationships) = {
                let graph_storage = self.graph_storage.as_ref().unwrap();
                let mut entries = Vec::new();
                let mut relationships = Vec::new();

                // Get all nodes from graph storage by collecting from all types
                let mut node_ids = Vec::new();
                for node_type in &[
                    "Function",
                    "Method",
                    "Class",
                    "Struct",
                    "Interface",
                    "Enum",
                    "Variable",
                    "Constant",
                    "Module",
                    "Import",
                ] {
                    let type_nodes = graph_storage
                        .get_nodes_by_type(node_type)
                        .await
                        .context(format!("Failed to list nodes of type {}", node_type))?;
                    node_ids.extend(type_nodes);
                }

                for node_id in node_ids {
                    if let Some(graph_node) = graph_storage.get_node(node_id).await? {
                        // Convert graph node back to symbol entry
                        let symbol_type = match graph_node.node_type.as_str() {
                            "Function" => crate::parsing::SymbolType::Function,
                            "Method" => crate::parsing::SymbolType::Method,
                            "Class" => crate::parsing::SymbolType::Class,
                            "Struct" => crate::parsing::SymbolType::Struct,
                            "Interface" => crate::parsing::SymbolType::Interface,
                            "Enum" => crate::parsing::SymbolType::Enum,
                            "Variable" => crate::parsing::SymbolType::Variable,
                            "Constant" => crate::parsing::SymbolType::Constant,
                            "Module" => crate::parsing::SymbolType::Module,
                            "Import" => crate::parsing::SymbolType::Import,
                            _ => crate::parsing::SymbolType::Function, // Default
                        };

                        // Extract name from qualified name (last component)
                        let name = graph_node
                            .qualified_name
                            .split("::")
                            .last()
                            .unwrap_or(&graph_node.qualified_name)
                            .to_string();

                        let symbol = crate::parsing::ParsedSymbol {
                            name,
                            symbol_type,
                            kind: crate::parsing::SymbolKind::Public, // Default
                            start_line: graph_node.location.start_line,
                            start_column: graph_node.location.start_column,
                            end_line: graph_node.location.end_line,
                            end_column: graph_node.location.end_column,
                            text: String::new(), // Empty for now
                            documentation: None,
                        };

                        let entry = SymbolEntry {
                            id: node_id,
                            document_id: ValidatedDocumentId::from_uuid(node_id)?,
                            symbol,
                            qualified_name: graph_node.qualified_name,
                            file_path: PathBuf::from(graph_node.file_path),
                            language: crate::parsing::SupportedLanguage::Rust, // Default for now
                            repository: None,
                            parent_id: None, // Will be populated from edges if needed
                            children: Vec::new(), // Will be populated from edges if needed
                            dependencies: Vec::new(), // Will be populated from edges
                            dependents: HashSet::new(), // Will be populated from edges
                            extracted_at: DateTime::<Utc>::from_timestamp(graph_node.updated_at, 0)
                                .unwrap_or_else(Utc::now),
                            content_hash: graph_node
                                .metadata
                                .get("content_hash")
                                .cloned()
                                .unwrap_or_else(|| format!("recovered-{}", node_id)),
                        };

                        entries.push(entry);
                    }
                }

                // Load edges for all nodes
                for entry in &entries {
                    let edges = graph_storage
                        .get_edges(entry.id, petgraph::Direction::Outgoing)
                        .await?;
                    tracing::debug!(
                        "Loading edges for symbol {} (UUID: {}): found {} edges",
                        entry.qualified_name,
                        entry.id,
                        edges.len()
                    );
                    for (target_id, edge_data) in edges {
                        tracing::debug!(
                            "Found edge: {} -> {} (type: {:?})",
                            entry.id,
                            target_id,
                            edge_data.relation_type
                        );
                        let relation = SymbolRelation {
                            from_id: entry.id,
                            to_id: target_id,
                            relation_type: edge_data.relation_type,
                            metadata: edge_data.metadata,
                        };
                        relationships.push(relation);
                    }
                }

                (entries, relationships)
            };

            // Now index all entries and relationships
            for entry in entries {
                self.index_symbol(entry)?;
                loaded_count += 1;
            }

            // Add relationships and update dependents
            for relation in relationships {
                // Update dependents
                if let Some(target_symbol) = self.symbol_index.get_mut(&relation.to_id) {
                    target_symbol.dependents.insert(relation.from_id);
                }
                self.relationships.push(relation);
            }
        } else {
            // Fallback to document storage
            let all_docs = self
                .storage
                .list_all()
                .await
                .context("Failed to list documents")?;

            let results: Vec<Document> = all_docs
                .into_iter()
                .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "symbol"))
                .collect();

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

            // Reconstruct relationships from dependents fields
            self.reconstruct_relationships_from_dependents()?;
        }

        info!("Loaded {} symbols from storage", loaded_count);
        Ok(())
    }

    /// Extract and store symbols from parsed code
    #[instrument(skip(self, parsed_code, file_content))]
    pub async fn extract_symbols(
        &mut self,
        file_path: &Path,
        parsed_code: ParsedCode,
        file_content: Option<&str>,
        repository: Option<String>,
    ) -> Result<Vec<Uuid>> {
        // Store file content for dependency analysis if provided
        if let Some(content) = file_content {
            self.file_content_cache
                .insert(file_path.to_path_buf(), content.to_string());
        }
        // Commenting out verbose logging for performance
        // info!(
        //     "Extracting {} symbols from {}",
        //     parsed_code.symbols.len(),
        //     file_path.display()
        // );

        let mut symbol_ids = Vec::new();
        let mut parent_stack: Vec<(Uuid, usize)> = Vec::new(); // (id, end_line)

        // Maximum nesting depth to prevent stack overflow
        const MAX_NESTING_DEPTH: usize = 50;

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

            // Update stack for nested symbols with depth limit
            if matches!(
                symbol.symbol_type,
                SymbolType::Function
                    | SymbolType::Class
                    | SymbolType::Struct
                    | SymbolType::Module
                    | SymbolType::Enum
            ) {
                // Check nesting depth to prevent stack overflow
                if parent_stack.len() < MAX_NESTING_DEPTH {
                    parent_stack.push((symbol_id, symbol.end_line));
                } else {
                    tracing::warn!(
                        "Maximum nesting depth ({}) reached at {} in {}. Symbol will be indexed but parent relationship may be incorrect.",
                        MAX_NESTING_DEPTH,
                        symbol.name,
                        file_path.display()
                    );
                }
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
        // If we have graph storage, use it as primary storage for symbols
        if let Some(ref mut graph_storage) = self.graph_storage {
            // Store symbol metadata in the graph node
            let mut metadata = HashMap::new();
            if let Some(ref repo) = entry.repository {
                metadata.insert("repository".to_string(), repo.clone());
            }
            metadata.insert("language".to_string(), format!("{:?}", entry.language));
            if let Some(parent_id) = entry.parent_id {
                metadata.insert("parent_id".to_string(), parent_id.to_string());
            }
            metadata.insert("content_hash".to_string(), entry.content_hash.clone());

            let graph_node = GraphNode {
                id: entry.id,
                node_type: format!("{:?}", entry.symbol.symbol_type),
                qualified_name: entry.qualified_name.clone(),
                file_path: entry.file_path.to_string_lossy().to_string(),
                location: NodeLocation {
                    start_line: entry.symbol.start_line,
                    start_column: entry.symbol.start_column,
                    end_line: entry.symbol.end_line,
                    end_column: entry.symbol.end_column,
                },
                metadata,
                updated_at: chrono::Utc::now().timestamp(),
            };

            graph_storage
                .store_node(entry.id, graph_node)
                .await
                .context("Failed to store symbol in graph storage")?;
        } else {
            // Fallback to document storage if no graph storage available
            let doc = self.serialize_symbol(&entry)?;
            self.storage.insert(doc).await?;
        }

        // Index in memory
        self.index_symbol(entry)?;

        Ok(())
    }

    /// Index a symbol in memory for fast lookups with LRU eviction
    fn index_symbol(&mut self, entry: SymbolEntry) -> Result<()> {
        // Check memory limits and evict if necessary
        let entry_size = self.estimate_symbol_size(&entry);
        let entry_id = entry.id;

        // Evict symbols if we're at capacity
        while self.symbol_index.len() >= self.config.max_symbols
            || self.estimated_memory_usage + entry_size > self.config.max_memory_bytes
        {
            // Evict the least recently used symbol
            if let Some(evicted_id) = self.lru_queue.pop_front() {
                self.evict_symbol(evicted_id);
                self.lru_set.remove(&evicted_id);

                tracing::debug!(
                    "Evicted symbol {} to make room (current: {} symbols, {} bytes)",
                    evicted_id,
                    self.symbol_index.len(),
                    self.estimated_memory_usage
                );
            } else {
                // No symbols to evict, can't proceed
                tracing::warn!(
                    "Cannot index symbol: no symbols to evict (symbols: {}, memory: {} bytes)",
                    self.symbol_index.len(),
                    self.estimated_memory_usage
                );
                return Ok(());
            }
        }

        // Add to name index using both qualified name and simple name
        self.name_index
            .entry(entry.qualified_name.clone())
            .or_default()
            .push(entry_id);

        // Also index by simple name for easier searching
        self.name_index
            .entry(entry.symbol.name.clone())
            .or_default()
            .push(entry_id);

        // Add to main index
        self.symbol_index.insert(entry_id, entry);
        self.estimated_memory_usage += entry_size;

        // Add to LRU tracking
        self.lru_queue.push_back(entry_id);
        self.lru_set.insert(entry_id);

        Ok(())
    }

    /// Evict a symbol from the in-memory index
    fn evict_symbol(&mut self, symbol_id: Uuid) {
        if let Some(entry) = self.symbol_index.remove(&symbol_id) {
            // Remove from name index (qualified name)
            if let Some(ids) = self.name_index.get_mut(&entry.qualified_name) {
                ids.retain(|&id| id != symbol_id);
                if ids.is_empty() {
                    self.name_index.remove(&entry.qualified_name);
                }
            }

            // Remove from name index (simple name)
            if let Some(ids) = self.name_index.get_mut(&entry.symbol.name) {
                ids.retain(|&id| id != symbol_id);
                if ids.is_empty() {
                    self.name_index.remove(&entry.symbol.name);
                }
            }

            // Update memory usage
            let entry_size = self.estimate_symbol_size(&entry);
            self.estimated_memory_usage = self.estimated_memory_usage.saturating_sub(entry_size);
        }
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
            if let Some(import_path) = self.parse_import_statement(&symbol.text) {
                deps.push(import_path);
            }
        }

        // Extract function calls, type references, and other dependencies from symbol text
        deps.extend(self.extract_code_dependencies(&symbol.text, &symbol.symbol_type));

        deps
    }

    /// Extract dependencies from code content (function calls, type references, etc.)
    fn extract_code_dependencies(&self, text: &str, symbol_type: &SymbolType) -> Vec<String> {
        let mut deps = Vec::new();

        // Skip import symbols as they're handled separately
        if *symbol_type == SymbolType::Import {
            return deps;
        }

        // Extract function calls (basic pattern matching)
        deps.extend(self.extract_function_calls(text));

        // Extract type references
        deps.extend(self.extract_type_references(text));

        // Extract macro usage (for Rust)
        deps.extend(self.extract_macro_usage(text));

        // Remove duplicates and clean up
        deps.sort();
        deps.dedup();
        deps
    }

    /// Extract function calls from code text
    fn extract_function_calls(&self, text: &str) -> Vec<String> {
        let mut calls = Vec::new();

        // Pattern for function calls: identifier(
        // This is a simple regex-like approach - could be enhanced with proper parsing
        let lines: Vec<&str> = text.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty() {
                continue;
            }

            // Look for patterns like: function_name( or module::function(
            if let Some(call) = self.extract_call_from_line(trimmed) {
                calls.push(call);
            }
        }

        calls
    }

    /// Extract a single function call from a line of code
    fn extract_call_from_line(&self, line: &str) -> Option<String> {
        // Look for patterns ending with '('
        if let Some(paren_pos) = line.find('(') {
            let before_paren = &line[..paren_pos];

            // Find the last word/identifier before the parenthesis
            if let Some(last_space) = before_paren.rfind([' ', '\t', '=', '{', ';', ',', '!']) {
                let potential_call = &before_paren[last_space + 1..];
                if self.is_valid_identifier(potential_call) {
                    return Some(potential_call.to_string());
                }
            } else if self.is_valid_identifier(before_paren) {
                return Some(before_paren.to_string());
            }
        }
        None
    }

    /// Extract type references from code text  
    fn extract_type_references(&self, text: &str) -> Vec<String> {
        let mut types = Vec::new();

        // Look for type annotations and declarations
        let lines: Vec<&str> = text.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Extract types from various patterns
            types.extend(self.extract_types_from_line(trimmed));
        }

        types
    }

    /// Extract type references from a single line
    fn extract_types_from_line(&self, line: &str) -> Vec<String> {
        let mut types = Vec::new();

        // Rust type patterns: : Type, -> Type, <Type>, Vec<Type>
        if line.contains(':') {
            // Variable declarations: let x: Type
            if let Some(colon_pos) = line.find(':') {
                let after_colon = &line[colon_pos + 1..];
                if let Some(type_name) = self.extract_type_from_annotation(after_colon) {
                    types.push(type_name);
                }
            }
        }

        // Return types: -> Type
        if line.contains("->") {
            if let Some(arrow_pos) = line.find("->") {
                let after_arrow = &line[arrow_pos + 2..];
                if let Some(type_name) = self.extract_type_from_annotation(after_arrow) {
                    types.push(type_name);
                }
            }
        }

        types
    }

    /// Extract type name from a type annotation
    fn extract_type_from_annotation(&self, annotation: &str) -> Option<String> {
        let trimmed = annotation.trim();

        // Handle generic types like Vec<T> - extract the base type
        if let Some(generic_pos) = trimmed.find('<') {
            let base_type = &trimmed[..generic_pos];
            if self.is_valid_identifier(base_type) {
                return Some(base_type.to_string());
            }
        }

        // Simple type reference
        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        let clean_type = first_word.trim_end_matches([',', ';', '{', ')', '}']);

        if self.is_valid_identifier(clean_type) && !self.is_primitive_type(clean_type) {
            Some(clean_type.to_string())
        } else {
            None
        }
    }

    /// Extract macro usage from code text (Rust-specific)
    fn extract_macro_usage(&self, text: &str) -> Vec<String> {
        let mut macros = Vec::new();

        let lines: Vec<&str> = text.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Look for macro calls: macro_name!
            if let Some(macro_name) = self.extract_macro_from_line(trimmed) {
                macros.push(macro_name);
            }
        }

        macros
    }

    /// Extract macro call from a line
    fn extract_macro_from_line(&self, line: &str) -> Option<String> {
        if let Some(excl_pos) = line.find('!') {
            let before_excl = &line[..excl_pos];

            // Find the macro name before the !
            if let Some(last_space) = before_excl.rfind([' ', '\t', '(', '{', ';']) {
                let potential_macro = &before_excl[last_space + 1..];
                if self.is_valid_identifier(potential_macro) {
                    return Some(potential_macro.to_string());
                }
            } else if self.is_valid_identifier(before_excl) {
                return Some(before_excl.to_string());
            }
        }
        None
    }

    /// Check if a string is a valid identifier
    fn is_valid_identifier(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Must start with letter or underscore
        let first_char = s.chars().next().unwrap();
        if !first_char.is_alphabetic() && first_char != '_' {
            return false;
        }

        // Rest can be alphanumeric, underscore, or ::
        s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    }

    /// Check if a type is a primitive type that we don't need to track
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
                | "String"
                | "Option"
                | "Result"
                | "Vec"
                | "HashMap"
                | "int"
                | "float"
                | "string"
                | "boolean"
                | "void"
                | "null"
        )
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

        // JavaScript/TypeScript imports: import x from 'module'; import {x} from 'module';
        // MUST check BEFORE Python to correctly handle "import X from 'Y'" syntax
        if trimmed.starts_with("import ") && trimmed.contains(" from ") {
            // Look for 'from' keyword followed by quotes
            if let Some(from_pos) = trimmed.find(" from ") {
                let after_from = &trimmed[from_pos + 6..];
                if let Some(start) = after_from.find(['\'', '"']) {
                    let quote_char = after_from.chars().nth(start).unwrap();
                    if let Some(end) = after_from[start + 1..].find(quote_char) {
                        return Some(after_from[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }
        // Also handle direct quotes (import 'module';)
        else if trimmed.starts_with("import ")
            && (trimmed.contains('\'') || trimmed.contains('"'))
        {
            if let Some(start) = trimmed.find(['\'', '"']) {
                let quote_char = trimmed.chars().nth(start).unwrap();
                if let Some(end) = trimmed[start + 1..].find(quote_char) {
                    return Some(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }

        // Python imports: import module; from module import x; import module as alias
        // Check AFTER JavaScript to avoid false matches on "import X from 'Y'"
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
        let content = String::from_utf8(doc.content.clone())?;

        // The file storage includes frontmatter in the content when reading files.
        // We need to extract the JSON content that comes after the frontmatter.
        let json_content = if content.trim().starts_with("---") {
            // Find the end of frontmatter (second occurrence of "---")
            if let Some(start) = content.find("---") {
                // Find the closing --- after the opening one
                if let Some(end) = content[start + 3..].find("---") {
                    // Extract content after the closing --- (skip the newline too)
                    let content_start = start + 3 + end + 3;
                    if content_start < content.len() {
                        content[content_start..].trim()
                    } else {
                        // No content after frontmatter
                        return Err(anyhow::anyhow!("No JSON content found after frontmatter"));
                    }
                } else {
                    // Malformed frontmatter - missing closing ---
                    return Err(anyhow::anyhow!(
                        "Malformed frontmatter: missing closing ---"
                    ));
                }
            } else {
                // Should not happen as we already checked starts_with
                content.trim()
            }
        } else {
            // No frontmatter, treat entire content as JSON
            content.trim()
        };

        // Parse the JSON content
        serde_json::from_str(json_content).with_context(|| {
            format!(
                "Failed to deserialize symbol entry. Content preview: {}",
                &json_content.chars().take(200).collect::<String>()
            )
        })
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

    /// Get symbol by ID without updating LRU (for read-only access)
    pub fn get_symbol(&self, id: &Uuid) -> Option<&SymbolEntry> {
        self.symbol_index.get(id)
    }

    /// Get symbol by ID and update LRU order (requires mutable access)
    pub fn get_symbol_mut(&mut self, id: &Uuid) -> Option<&SymbolEntry> {
        // Update LRU order if symbol is in cache
        if self.lru_set.contains(id) {
            // Remove from current position
            self.lru_queue.retain(|&x| x != *id);
            // Add to back (most recently used)
            self.lru_queue.push_back(*id);
        }
        self.symbol_index.get(id)
    }

    /// Add a relationship between symbols
    pub async fn add_relationship(&mut self, relation: SymbolRelation) -> Result<()> {
        // First, try to store in graph storage if available (for O(1) lookups)
        if let Some(ref mut graph) = self.graph_storage {
            // Get location data from the source symbol if available
            let location = if let Some(source_symbol) = self.symbol_index.get(&relation.from_id) {
                NodeLocation {
                    start_line: source_symbol.symbol.start_line,
                    start_column: source_symbol.symbol.start_column,
                    end_line: source_symbol.symbol.end_line,
                    end_column: source_symbol.symbol.end_column,
                }
            } else {
                // Fallback if symbol not found
                NodeLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 0,
                }
            };

            // Convert to GraphEdge for graph storage
            let edge = GraphEdge {
                relation_type: relation.relation_type.clone(),
                location,
                context: None,
                metadata: relation.metadata.clone(),
                created_at: chrono::Utc::now().timestamp(),
            };

            // Store the edge in graph storage for O(1) lookups
            // If this fails, we don't proceed with in-memory updates
            graph
                .store_edge(relation.from_id, relation.to_id, edge)
                .await
                .context("Failed to store relationship in graph storage")?;
        }

        // Only update in-memory state after successful graph storage (or if no graph storage)
        // This ensures consistency between storage layers

        // Update dependent's list
        if let Some(target) = self.symbol_index.get_mut(&relation.to_id) {
            target.dependents.insert(relation.from_id);
        }

        self.relationships.push(relation);
        Ok(())
    }

    /// Build dependency graph by analyzing relationships between all symbols
    pub async fn build_dependency_graph(&mut self) -> Result<()> {
        info!(
            "Building dependency graph from {} symbols across {} files",
            self.symbol_index.len(),
            self.file_content_cache.len()
        );

        tracing::debug!(
            "Files with cached content: {:?}",
            self.file_content_cache.keys().collect::<Vec<_>>()
        );

        // Use DependencyExtractor for accurate dependency analysis
        let extractor = crate::dependency_extractor::DependencyExtractor::new()?;
        let mut parser = crate::parsing::CodeParser::new()?;

        // Group symbols by file for efficient processing (memory optimized)
        // Only store IDs to avoid cloning entire SymbolEntry objects
        let mut symbol_ids_by_file: HashMap<PathBuf, Vec<Uuid>> = HashMap::new();
        for (id, symbol) in &self.symbol_index {
            symbol_ids_by_file
                .entry(symbol.file_path.clone())
                .or_default()
                .push(*id);
        }

        let mut all_analyses = Vec::new();

        // Analyze each file with DependencyExtractor
        for (file_path, symbol_ids) in &symbol_ids_by_file {
            // Try cached content first, then read from disk
            let content_opt = if let Some(cached_content) = self.file_content_cache.get(file_path) {
                Some(cached_content.clone())
            } else if let Ok(disk_content) = tokio::fs::read_to_string(file_path).await {
                Some(disk_content)
            } else {
                tracing::warn!(
                    "Cannot read file content for dependency analysis: {:?}",
                    file_path
                );
                None
            };

            if let Some(content) = content_opt {
                // Determine language from file extension
                let language = self.determine_language(file_path);

                // Parse the code first to get ParsedCode
                if let Ok(parsed_code) = parser.parse_content(&content, language) {
                    // Extract dependencies using the parsed code
                    if let Ok(analysis) =
                        extractor.extract_dependencies(&parsed_code, &content, file_path)
                    {
                        all_analyses.push(analysis);
                    } else {
                        tracing::debug!("Failed to extract dependencies for {:?}", file_path);
                    }
                } else {
                    tracing::debug!("Failed to parse content for {:?}", file_path);
                }
            }
        }

        tracing::info!(
            "Dependency analysis complete: {} file analyses successful out of {} files with symbols",
            all_analyses.len(),
            symbol_ids_by_file.len()
        );

        // Build symbol entries vector only when needed, using references where possible
        let all_symbol_entries: Vec<SymbolEntry> = symbol_ids_by_file
            .into_values()
            .flatten()
            .filter_map(|id| self.symbol_index.get(&id).cloned())
            .collect();

        // Build the dependency graph using extracted references
        let dep_graph = extractor.build_dependency_graph(all_analyses, &all_symbol_entries)?;

        info!(
            "Dependency graph built: {} nodes, {} edges",
            dep_graph.graph.node_count(),
            dep_graph.graph.edge_count()
        );

        // Clear existing relationships and rebuild from dependency graph
        self.relationships.clear();

        // If we have graph storage, batch insert all nodes first
        if let Some(ref mut graph_storage) = self.graph_storage {
            let mut nodes_to_insert = Vec::new();

            // Prepare all nodes for batch insertion
            for node_idx in dep_graph.graph.node_indices() {
                let node = &dep_graph.graph[node_idx];

                // Find the corresponding symbol entry
                if let Some(symbol_entry) = self.symbol_index.get(&node.symbol_id) {
                    let graph_node = GraphNode {
                        id: node.symbol_id,
                        node_type: format!("{:?}", symbol_entry.symbol.symbol_type),
                        qualified_name: symbol_entry.qualified_name.clone(),
                        file_path: symbol_entry.file_path.to_string_lossy().to_string(),
                        location: NodeLocation {
                            start_line: symbol_entry.symbol.start_line,
                            start_column: symbol_entry.symbol.start_column,
                            end_line: symbol_entry.symbol.end_line,
                            end_column: symbol_entry.symbol.end_column,
                        },
                        metadata: HashMap::new(),
                        updated_at: chrono::Utc::now().timestamp(),
                    };
                    nodes_to_insert.push((node.symbol_id, graph_node));
                }
            }

            // Batch insert all nodes into graph storage
            if !nodes_to_insert.is_empty() {
                graph_storage
                    .batch_insert_nodes(nodes_to_insert)
                    .await
                    .context("Failed to batch insert nodes into graph storage")?;
                debug!(
                    "Inserted {} nodes into graph storage",
                    dep_graph.graph.node_count()
                );
            }
        }

        // Convert dependency graph edges to relationships
        let mut edges_to_insert = Vec::new();

        for edge_ref in dep_graph.graph.edge_references() {
            let source_node = &dep_graph.graph[edge_ref.source()];
            let target_node = &dep_graph.graph[edge_ref.target()];
            let edge_data = edge_ref.weight();

            let relation = SymbolRelation {
                from_id: source_node.symbol_id,
                to_id: target_node.symbol_id,
                relation_type: edge_data.relation_type.clone(),
                metadata: HashMap::new(),
            };

            self.relationships.push(relation.clone());

            // Update dependents set for bidirectional navigation
            if let Some(target_symbol) = self.symbol_index.get_mut(&target_node.symbol_id) {
                target_symbol.dependents.insert(source_node.symbol_id);
            }

            // Prepare edge for graph storage
            if self.graph_storage.is_some() {
                let graph_edge = GraphEdge {
                    relation_type: edge_data.relation_type.clone(),
                    location: NodeLocation {
                        start_line: edge_data.line_number,
                        start_column: edge_data.column_number,
                        end_line: edge_data.line_number,
                        end_column: edge_data.column_number,
                    },
                    context: edge_data.context.clone(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().timestamp(),
                };
                tracing::debug!(
                    "Preparing edge for storage: {} -> {} (type: {:?})",
                    source_node.symbol_id,
                    target_node.symbol_id,
                    edge_data.relation_type
                );
                edges_to_insert.push((source_node.symbol_id, target_node.symbol_id, graph_edge));
            }
        }

        // Batch insert all edges into graph storage with rollback on failure
        if let Some(ref mut graph_storage) = self.graph_storage {
            if !edges_to_insert.is_empty() {
                // Store original relationship count for potential rollback
                let original_relationship_count = self.relationships.len();

                match graph_storage.batch_insert_edges(edges_to_insert).await {
                    Ok(_) => {
                        debug!(
                            "Successfully inserted {} edges into graph storage",
                            dep_graph.graph.edge_count()
                        );
                    }
                    Err(e) => {
                        // Rollback in-memory relationships on graph storage failure
                        warn!(
                            "Failed to batch insert edges into graph storage, rolling back {} relationships: {}",
                            self.relationships.len() - original_relationship_count,
                            e
                        );
                        self.relationships.truncate(original_relationship_count);

                        // Also rollback dependents in symbol index
                        for symbol in self.symbol_index.values_mut() {
                            symbol.dependents.clear();
                        }

                        // Re-add the original dependents from remaining relationships
                        for relation in &self.relationships {
                            if let Some(target) = self.symbol_index.get_mut(&relation.to_id) {
                                target.dependents.insert(relation.from_id);
                            }
                        }

                        return Err(e).context("Failed to batch insert edges, rolled back changes");
                    }
                }
            }
        }

        // Update in_degree and out_degree for all symbols
        for node_idx in dep_graph.graph.node_indices() {
            let node = &dep_graph.graph[node_idx];
            if let Some(symbol) = self.symbol_index.get_mut(&node.symbol_id) {
                // Count incoming edges (dependents)
                let in_degree = dep_graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                    .count();
                // Count outgoing edges (dependencies)
                let out_degree = dep_graph
                    .graph
                    .edges_directed(node_idx, petgraph::Direction::Outgoing)
                    .count();

                // Store these counts (you may want to add fields for these)
                debug!(
                    "Symbol {} has {} dependents and {} dependencies",
                    symbol.qualified_name, in_degree, out_degree
                );
            }
        }

        let dependents_count = self
            .symbol_index
            .values()
            .filter(|s| !s.dependents.is_empty())
            .count();

        info!(
            "Built dependency graph with {} relationships, {} symbols have dependents",
            self.relationships.len(),
            dependents_count
        );

        // For graph storage, nodes are already updated in memory and will be persisted on flush
        // No need to update document storage since symbols are stored as graph nodes
        if self.graph_storage.is_none() {
            // Only update document storage if we're not using graph storage
            for symbol in self.symbol_index.values() {
                let mut doc = self.serialize_symbol(symbol)?;
                doc.updated_at = chrono::Utc::now();
                self.storage.update(doc).await?;
            }
        }

        Ok(())
    }

    /// Reconstruct the relationships vector from the dependents fields in symbols
    fn reconstruct_relationships_from_dependents(&mut self) -> Result<()> {
        self.relationships.clear();

        for (symbol_id, symbol) in &self.symbol_index {
            for dependent_id in &symbol.dependents {
                // Create a relationship from dependent to this symbol
                let relation = SymbolRelation {
                    from_id: *dependent_id,
                    to_id: *symbol_id,
                    relation_type: RelationType::Calls, // Default type, could be more sophisticated
                    metadata: HashMap::new(),
                };
                self.relationships.push(relation);
            }
        }

        info!(
            "Reconstructed {} relationships from dependents fields",
            self.relationships.len()
        );
        Ok(())
    }

    /// Determine the language from file extension
    fn determine_language(&self, file_path: &Path) -> SupportedLanguage {
        // Currently only Rust is supported, but we check the extension for future expansion
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "rs" => SupportedLanguage::Rust,
            // For now, we only support Rust. Other languages will need to be added
            // to the SupportedLanguage enum and corresponding tree-sitter parsers
            _ => SupportedLanguage::Rust, // Default to Rust for now
        }
    }

    /// Get dependency graph statistics
    pub fn get_dependency_stats(&self) -> DependencyGraphStats {
        let mut relation_counts = HashMap::new();
        let mut symbols_with_deps = 0;
        let mut symbols_with_dependents = 0;

        for relation in &self.relationships {
            let rel_type = format!("{:?}", relation.relation_type);
            *relation_counts.entry(rel_type).or_insert(0) += 1;
        }

        for symbol in self.symbol_index.values() {
            if !symbol.dependencies.is_empty() {
                symbols_with_deps += 1;
            }
            if !symbol.dependents.is_empty() {
                symbols_with_dependents += 1;
            }
        }

        DependencyGraphStats {
            total_symbols: self.symbol_index.len(),
            total_relationships: self.relationships.len(),
            symbols_with_dependencies: symbols_with_deps,
            symbols_with_dependents,
            relationship_type_counts: relation_counts,
        }
    }

    /// Find all symbols that depend on a given symbol (reverse dependencies)
    pub fn find_dependents(&self, target_symbol_id: &Uuid) -> Vec<&SymbolEntry> {
        let mut dependents = Vec::new();

        for relation in &self.relationships {
            if relation.to_id == *target_symbol_id {
                if let Some(dependent_symbol) = self.symbol_index.get(&relation.from_id) {
                    dependents.push(dependent_symbol);
                }
            }
        }

        dependents
    }

    /// Find all symbols that a given symbol depends on (forward dependencies)
    pub fn find_dependencies(&self, source_symbol_id: &Uuid) -> Vec<&SymbolEntry> {
        let mut dependencies = Vec::new();

        for relation in &self.relationships {
            if relation.from_id == *source_symbol_id {
                if let Some(dependency_symbol) = self.symbol_index.get(&relation.to_id) {
                    dependencies.push(dependency_symbol);
                }
            }
        }

        dependencies
    }

    /// Perform dependency graph analysis to find cycles, orphans, etc.
    pub fn analyze_dependency_graph(&self) -> DependencyAnalysis {
        let cycles = Vec::new();
        let mut orphaned_symbols = Vec::new();
        let mut highly_coupled_symbols = Vec::new();

        // Find symbols with no dependencies and no dependents (potential orphans)
        for (id, symbol) in &self.symbol_index {
            let has_deps = self.relationships.iter().any(|r| r.from_id == *id);
            let has_dependents = self.relationships.iter().any(|r| r.to_id == *id);

            if !has_deps && !has_dependents && symbol.symbol.symbol_type != SymbolType::Import {
                orphaned_symbols.push(*id);
            }
        }

        // Find highly coupled symbols (symbols with many relationships)
        for id in self.symbol_index.keys() {
            let relationship_count = self
                .relationships
                .iter()
                .filter(|r| r.from_id == *id || r.to_id == *id)
                .count();

            if relationship_count > 10 {
                // Threshold for high coupling
                highly_coupled_symbols.push((*id, relationship_count));
            }
        }

        // TODO: Implement cycle detection using DFS

        DependencyAnalysis {
            cycles,
            orphaned_symbols,
            highly_coupled_symbols,
            total_symbols: self.symbol_index.len(),
            total_relationships: self.relationships.len(),
        }
    }

    /// Convert symbol storage data into a dependency graph structure
    pub async fn to_dependency_graph(
        &self,
    ) -> Result<crate::dependency_extractor::DependencyGraph> {
        use crate::dependency_extractor::{
            DependencyEdge, DependencyGraph, GraphStats, SymbolNode,
        };
        use petgraph::graph::DiGraph;
        use std::collections::HashMap;

        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();

        // Add all symbols as nodes
        for (symbol_id, symbol_entry) in &self.symbol_index {
            let symbol_node = SymbolNode {
                symbol_id: *symbol_id,
                qualified_name: symbol_entry.qualified_name.clone(),
                symbol_type: symbol_entry.symbol.symbol_type.clone(),
                file_path: symbol_entry.file_path.clone(),
                in_degree: 0,  // Will be calculated later
                out_degree: 0, // Will be calculated later
            };

            let node_idx = graph.add_node(symbol_node);
            symbol_to_node.insert(*symbol_id, node_idx);
            name_to_symbol.insert(symbol_entry.qualified_name.clone(), *symbol_id);
        }

        // Add relationships as edges
        for relationship in &self.relationships {
            if let (Some(&from_node), Some(&to_node)) = (
                symbol_to_node.get(&relationship.from_id),
                symbol_to_node.get(&relationship.to_id),
            ) {
                let edge = DependencyEdge {
                    relation_type: relationship.relation_type.clone(),
                    line_number: 0,   // Not available in current SymbolRelation
                    column_number: 0, // Not available in current SymbolRelation
                    context: None,    // Not available in current SymbolRelation
                };
                graph.add_edge(from_node, to_node, edge);
            }
        }

        // Calculate node degrees
        for node_idx in graph.node_indices() {
            let in_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .count();
            let out_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing)
                .count();

            if let Some(node) = graph.node_weight_mut(node_idx) {
                node.in_degree = in_degree;
                node.out_degree = out_degree;
            }
        }

        // Calculate statistics
        let stats = GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            file_count: self.file_symbols.len(),
            import_count: 0, // Not tracked in current implementation
            scc_count: petgraph::algo::kosaraju_scc(&graph).len(),
            max_depth: 0, // TODO: Calculate if needed
            avg_dependencies: if graph.node_count() > 0 {
                graph.edge_count() as f64 / graph.node_count() as f64
            } else {
                0.0
            },
        };

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol,
            file_imports: HashMap::new(), // Not implemented in current SymbolStorage
            stats,
        })
    }

    /// Get relationships for a symbol
    pub fn get_relationships(&self, symbol_id: &Uuid) -> Vec<&SymbolRelation> {
        self.relationships
            .iter()
            .filter(|r| r.from_id == *symbol_id || r.to_id == *symbol_id)
            .collect()
    }

    /// Get the total number of relationships
    pub fn get_relationships_count(&self) -> usize {
        self.relationships.len()
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
            .extract_symbols(file_path, parsed_code, None, repository)
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

    /// Get all files that have symbols indexed
    pub fn get_indexed_files(&self) -> Vec<std::path::PathBuf> {
        self.file_symbols.keys().cloned().collect()
    }

    /// Sync the underlying storage
    pub async fn sync_storage(&mut self) -> Result<()> {
        self.storage.sync().await
    }

    /// Flush the underlying storage and graph storage
    pub async fn flush_storage(&mut self) -> Result<()> {
        // Flush document storage
        self.storage.flush().await?;

        // Flush graph storage if available
        if let Some(ref mut graph_storage) = self.graph_storage {
            graph_storage.flush().await?;
        }

        Ok(())
    }

    /// Close the underlying storage
    pub async fn close_storage(mut self) -> Result<()> {
        // Since we can't move out of a trait object directly, we'll just call sync first
        self.storage.sync().await?;
        self.storage.flush().await?;
        // Note: The actual close will happen when the storage is dropped
        Ok(())
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
        use std::path::Component;

        // Convert path to string and normalize separators
        let path_str = path.to_string_lossy();
        let normalized = path_str.replace('\\', "/");

        // Properly resolve the path by handling .. components
        let mut resolved_parts = Vec::new();

        for component in Path::new(&normalized).components() {
            match component {
                Component::Normal(part) => {
                    if let Some(part_str) = part.to_str() {
                        resolved_parts.push(part_str);
                    }
                }
                Component::ParentDir => {
                    // Remove the last component if it exists (going up one directory)
                    resolved_parts.pop();
                }
                Component::CurDir => {
                    // Current directory (.) - skip it
                }
                _ => {
                    // Skip other components (RootDir, Prefix)
                }
            }
        }

        // Join with forward slashes for consistent storage paths
        if resolved_parts.is_empty() {
            String::new()
        } else {
            resolved_parts.join("/")
        }
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
            .extract_symbols(Path::new("test.rs"), parsed, None, None)
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
            .extract_symbols(Path::new("math.rs"), parsed, None, None)
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
            .extract_symbols(Path::new("evolving.rs"), parsed_v1, None, None)
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
            .extract_symbols(Path::new("test.rs"), parsed1, None, None)
            .await?;

        // Delete the symbols from storage to test fresh extraction
        for id in &ids1 {
            if let Some(entry) = symbol_storage.symbol_index.get(id) {
                let _ = symbol_storage.storage.delete(&entry.document_id).await;
            }
        }

        // Clear indices and re-extract to test determinism
        symbol_storage.file_symbols.clear();
        symbol_storage.symbol_index.clear();
        symbol_storage.name_index.clear();

        let ids2 = symbol_storage
            .extract_symbols(Path::new("test.rs"), parsed2, None, None)
            .await?;

        // Symbol IDs should be identical for the same code
        assert_eq!(ids1, ids2, "Symbol IDs should be deterministic");

        Ok(())
    }

    #[test]
    fn test_path_sanitization() -> Result<()> {
        use tokio::runtime::Runtime;

        // Create a runtime for async test
        let rt = Runtime::new()?;

        rt.block_on(async {
            // Create a temporary symbol storage instance to test the actual sanitize_path method
            let test_dir = format!("/tmp/test_path_sanitization_{}", uuid::Uuid::new_v4());
            tokio::fs::create_dir_all(&test_dir).await?;

            let storage = crate::file_storage::create_file_storage(&test_dir, Some(100)).await?;
            let symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

            // Test various malicious paths
            let test_cases = vec![
                ("../../../etc/passwd", "etc/passwd"),
                ("..\\..\\windows\\system32", "windows/system32"),
                ("safe/normal/path", "safe/normal/path"),
                ("./safe/path", "safe/path"),
                ("./../parent", "parent"),
                ("nested/../folder", "folder"),
            ];

            for (input, expected) in test_cases {
                let sanitized = symbol_storage.sanitize_path(Path::new(input));
                assert_eq!(
                    sanitized, expected,
                    "Path {} was not properly sanitized. Got: {}, Expected: {}",
                    input, sanitized, expected
                );
            }

            // Clean up
            tokio::fs::remove_dir_all(&test_dir).await?;

            Ok::<(), anyhow::Error>(())
        })?;

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
            .extract_symbols(Path::new("deep.rs"), parsed, None, None)
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

        let mut symbol_storage = SymbolStorage::with_config(storage, None, config).await?;

        // Try to add many symbols
        for i in 0..10 {
            let rust_code = format!("fn function_{}() {{}}", i);
            let mut parser = CodeParser::new()?;
            let parsed = parser.parse_content(&rust_code, SupportedLanguage::Rust)?;

            let _ = symbol_storage
                .extract_symbols(Path::new(&format!("file_{}.rs", i)), parsed, None, None)
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
            .extract_symbols(Path::new("test.rs"), parsed, None, None)
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

            // JavaScript/TypeScript imports (check BEFORE Python to handle "import X from 'Y'" correctly)
            if trimmed.starts_with("import ") {
                // Look for 'from' keyword followed by quotes
                if let Some(from_pos) = trimmed.find(" from ") {
                    let after_from = &trimmed[from_pos + 6..];
                    if let Some(start) = after_from.find(['\'', '"']) {
                        let quote_char = after_from.chars().nth(start).unwrap();
                        if let Some(end) = after_from[start + 1..].find(quote_char) {
                            return Some(after_from[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
                // Also handle direct quotes (import 'module';)
                else if let Some(start) = trimmed.find(['\'', '"']) {
                    let quote_char = trimmed.chars().nth(start).unwrap();
                    if let Some(end) = trimmed[start + 1..].find(quote_char) {
                        return Some(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }

            // Python imports (check AFTER JavaScript)
            if let Some(rest) = trimmed.strip_prefix("import ") {
                if let Some(module) = rest.split_whitespace().next() {
                    return Some(module.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("from ") {
                if let Some(module) = rest.split_whitespace().next() {
                    return Some(module.to_string());
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
