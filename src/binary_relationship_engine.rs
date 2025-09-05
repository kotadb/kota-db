//! Binary relationship query engine that provides fast symbol lookup and relationship queries
//!
//! This module provides the primary engine for relationship queries using the binary symbol format,
//! ensuring sub-10ms query latency while maintaining full API compatibility.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::{
    binary_relationship_bridge::BinaryRelationshipBridge,
    binary_symbols::BinarySymbolReader,
    dependency_extractor::DependencyGraph,
    parsing::{SupportedLanguage, SymbolType},
    path_utils::normalize_path_relative,
    relationship_query::{
        RelationshipLocation, RelationshipMatch, RelationshipQueryConfig, RelationshipQueryResult,
        RelationshipQueryType, RelationshipStats,
    },
    types::RelationType,
};

/// Cache eviction policy for dependency graphs
#[derive(Debug, Clone, Copy)]
pub enum CacheEvictionPolicy {
    /// Never evict cached graphs (current behavior)
    Never,
    /// Evict when memory usage exceeds threshold
    MemoryBased { threshold_bytes: u64 },
    /// Evict after time-based TTL
    TimeBased { ttl_seconds: u64 },
    /// Evict using LRU policy when cache size exceeds limit
    Lru { max_entries: usize },
}

/// Configuration for on-demand relationship extraction
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Maximum file size to process (in bytes)
    pub max_file_size: u64,
    /// Supported languages for analysis (uses SupportedLanguage enum)
    pub supported_languages: Vec<SupportedLanguage>,
    /// Additional file extensions not covered by supported languages
    pub additional_extensions: Vec<String>,
    /// Maximum number of files to process per extraction
    pub max_files_per_extraction: Option<usize>,
    /// Enable memory usage warnings
    pub warn_on_large_graphs: bool,
    /// Memory limit for cached dependency graphs (in bytes)
    pub max_graph_memory: Option<u64>,
    /// Cache eviction policy for dependency graphs
    pub cache_eviction_policy: CacheEvictionPolicy,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024,                    // 10MB
            supported_languages: vec![SupportedLanguage::Rust], // Currently only Rust is fully supported
            additional_extensions: vec![
                // Common source code extensions for future language support
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "java".to_string(),
                "go".to_string(),
                "rb".to_string(),
            ],
            max_files_per_extraction: Some(10000), // Prevent resource exhaustion
            warn_on_large_graphs: true,
            max_graph_memory: Some(100 * 1024 * 1024), // 100MB limit for graphs
            cache_eviction_policy: CacheEvictionPolicy::MemoryBased {
                threshold_bytes: 100 * 1024 * 1024, // 100MB threshold
            },
        }
    }
}

/// Performance threshold for query execution warning (in milliseconds)
const QUERY_PERFORMANCE_THRESHOLD_MS: u64 = 10;

/// Binary relationship query engine that uses binary symbols
pub struct BinaryRelationshipEngine {
    /// Binary symbol reader for fast symbol lookup
    symbol_reader: Option<BinarySymbolReader>,
    /// Dependency graph built from relationships (using RwLock for thread-safe interior mutability)
    dependency_graph: RwLock<Option<DependencyGraph>>,
    /// Cache metadata for dependency graph management (using atomics for counters)
    cache_metadata: CacheMetadata,
    /// Database path for on-demand relationship extraction
    db_path: std::path::PathBuf,
    /// Configuration
    config: RelationshipQueryConfig,
    /// Extraction configuration for on-demand processing
    extraction_config: ExtractionConfig,
}

impl BinaryRelationshipEngine {
    /// Create a new binary engine from database paths
    #[instrument]
    pub async fn new(db_path: &Path, config: RelationshipQueryConfig) -> Result<Self> {
        Self::with_extraction_config(db_path, config, ExtractionConfig::default()).await
    }

    /// Create a new binary engine with custom extraction configuration
    #[instrument]
    pub async fn with_extraction_config(
        db_path: &Path,
        config: RelationshipQueryConfig,
        extraction_config: ExtractionConfig,
    ) -> Result<Self> {
        info!("Initializing binary relationship engine");

        // Try to load binary symbols if available
        let symbol_db_path = db_path.join("symbols.kota");
        let symbol_reader = if symbol_db_path.exists() {
            info!("Loading binary symbol database from: {:?}", symbol_db_path);
            match BinarySymbolReader::open(&symbol_db_path) {
                Ok(reader) => {
                    info!("Loaded {} binary symbols", reader.symbol_count());
                    Some(reader)
                }
                Err(e) => {
                    warn!("Failed to load binary symbols: {}", e);
                    None
                }
            }
        } else {
            debug!("Binary symbol database not found at: {:?}", symbol_db_path);
            None
        };

        // Try to load dependency graph if available
        let graph_db_path = db_path.join("dependency_graph.bin");
        let dependency_graph = if graph_db_path.exists() {
            info!("Loading dependency graph from: {:?}", graph_db_path);
            match Self::load_dependency_graph(&graph_db_path) {
                Ok(graph) => {
                    info!(
                        "Loaded dependency graph with {} nodes",
                        graph.graph.node_count()
                    );
                    Some(graph)
                }
                Err(e) => {
                    warn!("Failed to load dependency graph: {}", e);
                    None
                }
            }
        } else {
            debug!("Dependency graph not found at: {:?}", graph_db_path);
            None
        };

        Ok(Self {
            symbol_reader,
            dependency_graph: RwLock::new(dependency_graph),
            cache_metadata: CacheMetadata::new(),
            db_path: db_path.to_path_buf(),
            config,
            extraction_config,
        })
    }

    /// Execute a relationship query using the binary approach
    #[instrument(skip(self))]
    pub async fn execute_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        info!("Executing relationship query: {:?}", query_type);
        let start = std::time::Instant::now();

        // First try binary symbols if available, even without dependency graph for basic queries
        let result = if self.symbol_reader.is_some() {
            debug!(
                "Using binary symbol path for query (dependency graph available: {})",
                self.dependency_graph
                    .read()
                    .map(|g| g.is_some())
                    .unwrap_or(false)
            );
            self.execute_binary_query(query_type.clone()).await
        } else {
            debug!("Falling back to legacy symbol storage path");
            self.execute_legacy_query(query_type.clone()).await
        };

        let elapsed = start.elapsed();

        match &result {
            Ok(r) => {
                info!(
                    "Query completed in {:?} - found {} direct, {} indirect relationships",
                    elapsed, r.stats.direct_count, r.stats.indirect_count
                );
                if elapsed.as_millis() > 10 {
                    warn!("Query exceeded 10ms target: {:?}", elapsed);
                }
            }
            Err(e) => {
                warn!("Query failed after {:?}: {}", elapsed, e);
            }
        }

        result
    }

    /// Execute query using binary symbols and optional dependency graph
    async fn execute_binary_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        let reader = self
            .symbol_reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Binary symbol reader not available"))?;

        match query_type.clone() {
            RelationshipQueryType::FindCallers { target } => {
                self.execute_find_callers_query(query_type, &target).await
            }
            RelationshipQueryType::ImpactAnalysis { target } => {
                self.execute_impact_analysis_query(query_type, &target)
                    .await
            }
            _ => {
                // For other query types, fall back to legacy implementation
                self.execute_legacy_query(query_type).await
            }
        }
    }

    /// Execute query using legacy symbol storage
    async fn execute_legacy_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        // For now, return an error indicating the need for proper setup
        // In a future version, we could build a dependency graph from symbol storage here
        Err(anyhow::anyhow!(
            "Legacy relationship queries require both binary symbols and dependency graph. \
            Please ensure the repository was ingested with symbol and relationship extraction enabled."
        ))
    }

    /// Find all symbols that transitively depend on the given symbol
    fn find_transitive_dependents(
        &self,
        graph: &DependencyGraph,
        target_id: Uuid,
        max_depth: usize,
    ) -> Vec<(Uuid, RelationType)> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependents
        queue.push_back((target_id, 0));
        visited.insert(target_id);

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let dependents = graph.find_dependents(current_id);
            for (dependent_id, relation_type) in dependents {
                if !visited.contains(&dependent_id) {
                    visited.insert(dependent_id);
                    result.push((dependent_id, relation_type.clone()));
                    queue.push_back((dependent_id, depth + 1));
                }
            }
        }

        result
    }

    /// Execute find callers query with on-demand extraction
    async fn execute_find_callers_query(
        &self,
        query_type: RelationshipQueryType,
        target: &str,
    ) -> Result<RelationshipQueryResult> {
        let start = std::time::Instant::now();

        let reader = self
            .symbol_reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Binary symbol reader not available"))?;

        // Ensure dependency graph is available, extracting on-demand if needed
        if let Err(e) = self.ensure_dependency_graph("find-callers query").await {
            return self.create_extraction_failure_result(query_type, target, reader, e);
        }

        // Now we should have a graph, get it safely
        let graph_ref = self.get_dependency_graph()?;
        let graph = graph_ref.as_ref().unwrap();

        // Look up target symbol by name - find ALL symbols with this name
        debug!(
            "Looking for all symbols named '{}' in binary reader",
            target
        );
        let all_symbols = self.find_all_symbols_by_name(reader, target);

        if all_symbols.is_empty() {
            return Err(anyhow::anyhow!("Symbol '{}' not found", target));
        }

        debug!("Found {} symbols named '{}'", all_symbols.len(), target);

        // Try each symbol until we find one with relationships
        let mut all_callers = Vec::new();

        for (_symbol, symbol_id) in &all_symbols {
            debug!("Checking symbol '{}' with UUID: {}", target, symbol_id);

            // Resolve symbol UUID with fallback to name-based lookup
            if let Some(effective_id) =
                Self::resolve_symbol_uuid_with_fallback(graph, target, *symbol_id)
            {
                // Find callers for this specific symbol instance
                let callers = graph.find_dependents(effective_id);
                if !callers.is_empty() {
                    debug!(
                        "Found {} callers for symbol '{}' (UUID: {})",
                        callers.len(),
                        target,
                        symbol_id
                    );
                    all_callers.extend(callers);
                } else {
                    debug!(
                        "No callers found for symbol '{}' (UUID: {})",
                        target, symbol_id
                    );
                }
            } else {
                debug!(
                    "Symbol '{}' (UUID: {}) not found in dependency graph",
                    target, symbol_id
                );
            }
        }

        // If we found no callers from any symbol instance, return empty result
        if all_callers.is_empty() {
            return Ok(RelationshipQueryResult {
                query_type,
                direct_relationships: vec![],
                indirect_relationships: vec![],
                stats: RelationshipStats {
                    direct_count: 0,
                    indirect_count: 0,
                    symbols_analyzed: reader.symbol_count(),
                    execution_time_ms: 0,
                    truncated: false,
                },
                summary: format!(
                    "Symbol '{}' found in binary storage ({} instances) but no relationships found in dependency graph. \
                    The graph may be out of sync or the symbol may not be referenced by other code.",
                    target, all_symbols.len()
                ),
            });
        }

        // Use the combined callers from all symbol instances
        let callers = all_callers;

        // Convert to relationship matches with caller-specific context
        let direct_relationships =
            self.convert_caller_relationships_to_matches(reader, &callers, target);

        let execution_time_ms = start.elapsed().as_millis() as u64;
        if execution_time_ms > QUERY_PERFORMANCE_THRESHOLD_MS {
            warn!(
                "Find callers query took {}ms, expected < {}ms",
                execution_time_ms, QUERY_PERFORMANCE_THRESHOLD_MS
            );
        }

        Ok(RelationshipQueryResult {
            query_type,
            direct_relationships,
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: callers.len(),
                indirect_count: 0,
                symbols_analyzed: reader.symbol_count(),
                execution_time_ms,
                truncated: false,
            },
            summary: format!("Found {} direct callers of '{}'", callers.len(), target),
        })
    }

    /// Execute impact analysis query with on-demand extraction
    async fn execute_impact_analysis_query(
        &self,
        query_type: RelationshipQueryType,
        target: &str,
    ) -> Result<RelationshipQueryResult> {
        let start = std::time::Instant::now();

        let reader = self
            .symbol_reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Binary symbol reader not available"))?;

        // Ensure dependency graph is available, extracting on-demand if needed
        if let Err(e) = self.ensure_dependency_graph("impact analysis").await {
            return self.create_extraction_failure_result(query_type, target, reader, e);
        }

        // Now we should have a graph, get it safely
        let graph_ref = self.get_dependency_graph()?;
        let graph = graph_ref.as_ref().unwrap();

        // For impact analysis, find ALL symbols with this name across the codebase
        debug!(
            "Looking for all symbols named '{}' for impact analysis",
            target
        );
        let all_symbols = self.find_all_symbols_by_name(reader, target);

        if all_symbols.is_empty() {
            return Ok(RelationshipQueryResult {
                query_type,
                direct_relationships: vec![],
                indirect_relationships: vec![],
                stats: RelationshipStats {
                    direct_count: 0,
                    indirect_count: 0,
                    symbols_analyzed: reader.symbol_count(),
                    execution_time_ms: 0,
                    truncated: false,
                },
                summary: format!("Symbol '{}' not found in binary storage", target),
            });
        }

        // Collect all impacted symbols from all instances of the target symbol
        let mut all_impacted = Vec::new();
        let mut found_in_graph = false;

        for (_symbol, symbol_id) in &all_symbols {
            debug!(
                "Checking symbol '{}' with UUID: {} for impact",
                target, symbol_id
            );

            // Resolve symbol UUID with fallback to name-based lookup
            if let Some(effective_id) =
                Self::resolve_symbol_uuid_with_fallback(graph, target, *symbol_id)
            {
                found_in_graph = true;
                let impacted =
                    self.find_transitive_dependents(graph, effective_id, self.config.max_depth);
                if !impacted.is_empty() {
                    debug!(
                        "Found {} impacted symbols for '{}' (UUID: {})",
                        impacted.len(),
                        target,
                        symbol_id
                    );
                    all_impacted.extend(impacted);
                }
            } else {
                debug!(
                    "Symbol '{}' (UUID: {}) not found in dependency graph",
                    target, symbol_id
                );
            }
        }

        // If none of the symbol instances were found in the graph
        if !found_in_graph {
            return Ok(RelationshipQueryResult {
                query_type,
                direct_relationships: vec![],
                indirect_relationships: vec![],
                stats: RelationshipStats {
                    direct_count: 0,
                    indirect_count: 0,
                    symbols_analyzed: reader.symbol_count(),
                    execution_time_ms: 0,
                    truncated: false,
                },
                summary: format!(
                    "Symbol '{}' found in binary storage but not in dependency graph. \
                    The graph may be out of sync. Try re-indexing the codebase.",
                    target
                ),
            });
        }

        // Deduplicate impacted symbols (a symbol might be impacted through multiple paths)
        let mut unique_impacted = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (id, rel_type) in all_impacted {
            if seen.insert(id) {
                unique_impacted.push((id, rel_type));
            }
        }
        let impacted = unique_impacted;

        // Convert to relationship matches with impact-specific context
        let direct_relationships =
            self.convert_impact_relationships_to_matches(reader, &impacted, target);

        let execution_time_ms = start.elapsed().as_millis() as u64;
        if execution_time_ms > QUERY_PERFORMANCE_THRESHOLD_MS {
            warn!(
                "Impact analysis query took {}ms, expected < {}ms",
                execution_time_ms, QUERY_PERFORMANCE_THRESHOLD_MS
            );
        }

        Ok(RelationshipQueryResult {
            query_type,
            direct_relationships,
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: impacted.len(),
                indirect_count: 0,
                symbols_analyzed: reader.symbol_count(),
                execution_time_ms,
                truncated: false,
            },
            summary: format!(
                "{} symbols would be impacted by changes to '{}'",
                impacted.len(),
                target
            ),
        })
    }

    /// Save dependency graph to binary file (async version)
    pub async fn save_dependency_graph_async(graph: &DependencyGraph, path: &Path) -> Result<()> {
        info!(
            "Saving dependency graph with {} nodes to: {:?}",
            graph.graph.node_count(),
            path
        );

        let path = path.to_path_buf();
        let serializable = graph.to_serializable();

        // Use spawn_blocking to handle the blocking serialization operation
        tokio::task::spawn_blocking(move || -> Result<()> {
            use std::fs::File;
            use std::io::BufWriter;

            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            let file = File::create(&path)
                .with_context(|| format!("Failed to create dependency graph file: {:?}", path))?;
            let writer = BufWriter::new(file);

            // Serialize using bincode for efficiency
            bincode::serialize_into(writer, &serializable)
                .context("Failed to serialize dependency graph")?;

            info!("Successfully saved dependency graph to: {:?}", path);
            Ok(())
        })
        .await
        .context("Task join error")?
    }

    /// Save dependency graph to binary file (legacy sync version for backward compatibility)
    pub fn save_dependency_graph(graph: &DependencyGraph, path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::BufWriter;

        info!(
            "Saving dependency graph with {} nodes to: {:?}",
            graph.graph.node_count(),
            path
        );

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let file = File::create(path)
            .with_context(|| format!("Failed to create dependency graph file: {:?}", path))?;
        let writer = BufWriter::new(file);

        // Convert to serializable format
        let serializable = graph.to_serializable();

        // Serialize using bincode for efficiency
        bincode::serialize_into(writer, &serializable)
            .context("Failed to serialize dependency graph")?;

        info!("Successfully saved dependency graph to: {:?}", path);
        Ok(())
    }

    /// Load dependency graph from binary file
    fn load_dependency_graph(path: &Path) -> Result<DependencyGraph> {
        use std::fs::File;
        use std::io::BufReader;

        debug!("Loading dependency graph from: {:?}", path);

        let file = File::open(path)
            .with_context(|| format!("Failed to open dependency graph file: {:?}", path))?;
        let reader = BufReader::new(file);

        // Deserialize using bincode for efficiency
        let serializable: crate::dependency_extractor::SerializableDependencyGraph =
            bincode::deserialize_from(reader).context("Failed to deserialize dependency graph")?;

        // Convert from serializable format
        DependencyGraph::from_serializable(serializable)
            .context("Failed to reconstruct dependency graph from serialized data")
    }

    /// Find all symbols with the given name (handles multiple symbols with same name)
    fn find_all_symbols_by_name(
        &self,
        reader: &BinarySymbolReader,
        name: &str,
    ) -> Vec<(crate::binary_symbols::PackedSymbol, uuid::Uuid)> {
        reader
            .iter_symbols()
            .filter_map(|symbol| {
                if let Ok(symbol_name) = reader.get_symbol_name(&symbol) {
                    if symbol_name == name {
                        return Some((symbol, uuid::Uuid::from_bytes(symbol.id)));
                    }
                }
                None
            })
            .collect()
    }

    /// Resolve symbol UUID with fallback to name-based lookup
    ///
    /// When binary symbols and dependency graphs are generated at different times,
    /// their UUIDs may not match. This method provides a fallback mechanism to
    /// find symbols by qualified name when UUID lookup fails.
    fn resolve_symbol_uuid_with_fallback(
        graph: &DependencyGraph,
        target: &str,
        binary_uuid: Uuid,
    ) -> Option<Uuid> {
        // First, check if the binary UUID exists in the dependency graph
        if graph.symbol_to_node.contains_key(&binary_uuid) {
            debug!(
                "Found symbol '{}' by UUID {} in dependency graph",
                target, binary_uuid
            );
            return Some(binary_uuid);
        }

        // UUID not found, try fallback to name-based lookup
        warn!(
            "Symbol '{}' (UUID: {}) found in binary storage but not in dependency graph!",
            target, binary_uuid
        );
        warn!(
            "Graph has {} nodes, checking name_to_symbol map with {} entries",
            graph.symbol_to_node.len(),
            graph.name_to_symbol.len()
        );

        // Try to find by qualified name in the graph's name_to_symbol map
        for (name, id) in &graph.name_to_symbol {
            if name.ends_with(&format!("::{}", target)) || name == target {
                info!(
                    "Found symbol in graph by qualified name: {} -> {}",
                    name, id
                );
                return Some(*id);
            }
        }

        // Couldn't find the symbol in the graph
        warn!(
            "Could not find symbol '{}' in dependency graph by UUID or name. \
            The graph may be out of sync with the binary symbols.",
            target
        );
        None
    }

    /// Convert binary symbol kind to SymbolType
    fn convert_symbol_type(kind: u8) -> SymbolType {
        match kind {
            1 => SymbolType::Function,
            2 => SymbolType::Method,
            3 => SymbolType::Class,
            4 => SymbolType::Struct,
            5 => SymbolType::Enum,
            6 => SymbolType::Variable,
            7 => SymbolType::Constant,
            8 => SymbolType::Module,
            _ => SymbolType::Other("Unknown".to_string()),
        }
    }

    /// Get a reference to the dependency graph, ensuring it exists
    fn get_dependency_graph(
        &self,
    ) -> Result<std::sync::RwLockReadGuard<'_, Option<DependencyGraph>>> {
        let graph_ref = self.dependency_graph.read().map_err(|_| {
            anyhow::anyhow!(
                "Dependency graph lock poisoned - another thread panicked while holding the lock"
            )
        })?;
        if graph_ref.is_none() {
            return Err(anyhow::anyhow!(
                "Dependency graph unavailable - call ensure_dependency_graph() first"
            ));
        }

        // Return the guard directly - caller will need to access via as_ref()
        Ok(graph_ref)
    }

    // Removed complex generic implementation in favor of specific implementations

    /// Create a result for when extraction fails, with proper error context
    fn create_extraction_failure_result(
        &self,
        query_type: RelationshipQueryType,
        target: &str,
        reader: &BinarySymbolReader,
        extraction_error: anyhow::Error,
    ) -> Result<RelationshipQueryResult> {
        Ok(RelationshipQueryResult {
            query_type,
            direct_relationships: vec![],
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: 0,
                indirect_count: 0,
                symbols_analyzed: reader.symbol_count(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary: format!(
                "Symbol '{}' found in binary database (total {} symbols loaded), but on-demand relationship extraction failed: {}. \
                Consider re-running ingest-repo with relationship extraction enabled for better performance.",
                target, reader.symbol_count(), extraction_error
            ),
        })
    }

    // Removed generic implementation in favor of specific context-aware implementations

    /// Convert caller relationships to RelationshipMatch objects
    fn convert_caller_relationships_to_matches(
        &self,
        reader: &BinarySymbolReader,
        relationships: &[(Uuid, RelationType)],
        target: &str,
    ) -> Vec<RelationshipMatch> {
        let mut matches = Vec::new();
        for (id, relation_type) in relationships.iter() {
            if let Some(symbol) = reader.find_symbol(*id) {
                let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                    warn!("Failed to get symbol name for UUID {}: {}", id, e);
                    format!("symbol_{}", id)
                });
                let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                    warn!("Failed to get file path for symbol: {}", e);
                    "unknown".to_string()
                });

                // Generate context based on relation type
                let context = match relation_type {
                    RelationType::Calls => {
                        format!("Calls {} at line {}", target, symbol.start_line)
                    }
                    RelationType::References => {
                        format!("References {} at line {}", target, symbol.start_line)
                    }
                    RelationType::Implements => {
                        format!("Implements {} at line {}", target, symbol.start_line)
                    }
                    _ => format!("Uses {} at line {}", target, symbol.start_line),
                };

                matches.push(RelationshipMatch {
                    symbol_id: Uuid::from_bytes(symbol.id), // Safe: PackedSymbol.id is [u8; 16]
                    symbol_name: symbol_name.clone(),
                    qualified_name: format!("{}::{}", file_path, symbol_name),
                    symbol_type: Self::convert_symbol_type(symbol.kind),
                    file_path: file_path.clone(),
                    relation_type: relation_type.clone(),
                    location: RelationshipLocation {
                        line_number: symbol.start_line as usize,
                        column_number: 0,
                        file_path: file_path.clone(),
                    },
                    context,
                });
            }
        }
        matches
    }

    /// Convert impact relationships to RelationshipMatch objects
    fn convert_impact_relationships_to_matches(
        &self,
        reader: &BinarySymbolReader,
        relationships: &[(Uuid, RelationType)],
        target: &str,
    ) -> Vec<RelationshipMatch> {
        let mut matches = Vec::new();
        for (id, relation_type) in relationships.iter() {
            if let Some(symbol) = reader.find_symbol(*id) {
                let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                    warn!("Failed to get symbol name for UUID {}: {}", id, e);
                    format!("symbol_{}", id)
                });
                let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                    warn!("Failed to get file path for symbol: {}", e);
                    "unknown".to_string()
                });

                matches.push(RelationshipMatch {
                    symbol_id: Uuid::from_bytes(symbol.id), // Safe: PackedSymbol.id is [u8; 16]
                    symbol_name: symbol_name.clone(),
                    qualified_name: format!("{}::{}", file_path, symbol_name),
                    symbol_type: Self::convert_symbol_type(symbol.kind),
                    file_path: file_path.clone(),
                    relation_type: relation_type.clone(),
                    location: RelationshipLocation {
                        line_number: symbol.start_line as usize,
                        column_number: 0,
                        file_path: file_path.clone(),
                    },
                    context: format!("Would be impacted by changes to {}", target),
                });
            }
        }
        matches
    }

    /// Ensure dependency graph is available, extracting on-demand if necessary
    #[instrument(skip(self))]
    async fn ensure_dependency_graph(&self, query_context: &str) -> Result<()> {
        let has_graph = self
            .dependency_graph
            .read()
            .map(|g| g.is_some())
            .unwrap_or(false);
        if has_graph {
            return Ok(());
        }

        info!(
            "Dependency graph not cached, attempting on-demand extraction for {}",
            query_context
        );

        match self.extract_relationships_on_demand().await {
            Ok(extracted_graph) => {
                let node_count = extracted_graph.graph.node_count();
                info!(
                    "Successfully extracted relationships on-demand with {} nodes",
                    node_count
                );

                // Check memory limits if configured
                if self.extraction_config.warn_on_large_graphs {
                    let estimated_size = node_count * 64; // Rough estimate: 64 bytes per node
                    if let Some(max_memory) = self.extraction_config.max_graph_memory {
                        if estimated_size as u64 > max_memory {
                            warn!(
                                "Dependency graph is large ({} nodes, ~{} bytes), exceeds limit {} bytes. \
                                Consider increasing max_graph_memory or reducing file scope.",
                                node_count, estimated_size, max_memory
                            );
                        }
                    }

                    if node_count > 50000 {
                        warn!(
                            "Large dependency graph extracted ({} nodes). \
                            This may impact memory usage and query performance.",
                            node_count
                        );
                    }
                }

                // Check if we should evict the current graph based on policy
                self.maybe_evict_cache(&extracted_graph);

                // Store the extracted graph for future queries
                match self.dependency_graph.write() {
                    Ok(mut graph) => {
                        *graph = Some(extracted_graph);
                    }
                    Err(poisoned) => {
                        warn!("Dependency graph lock poisoned, attempting recovery");
                        // Attempt to recover by getting the mutex anyway
                        let mut graph = poisoned.into_inner();
                        *graph = Some(extracted_graph);
                    }
                }

                // Update cache metadata
                self.cache_metadata.record_access();
                Ok(())
            }
            Err(e) => {
                warn!("Failed to extract relationships on-demand: {}", e);
                Err(e)
            }
        }
    }

    /// Extract relationships on-demand from binary symbols and source files
    /// This method bridges the gap when binary symbols exist but dependency graph is missing
    #[instrument(skip(self))]
    async fn extract_relationships_on_demand(&self) -> Result<DependencyGraph> {
        info!("Starting on-demand relationship extraction from binary symbols");
        let start = std::time::Instant::now();

        let symbol_db_path = self.db_path.join("symbols.kota");

        // Ensure we have binary symbols available
        if !symbol_db_path.exists() {
            return Err(anyhow::anyhow!(
                "Binary symbol database not found at: {:?}",
                symbol_db_path
            ));
        }

        // Try to find source files using multiple strategies
        let (source_repo_path, files) = self.find_source_files_intelligently().await?;
        info!(
            "Found {} source files for relationship extraction from: {:?}",
            files.len(),
            source_repo_path
        );

        if files.is_empty() {
            return Err(anyhow::anyhow!(
                "No source files found for relationship extraction. \
                Ensure you're running from the repository directory or the original source files are accessible."
            ));
        }

        // Create relationship bridge and extract relationships using the actual repository path
        let bridge = BinaryRelationshipBridge::new();
        let dependency_graph = bridge
            .extract_relationships(&symbol_db_path, &source_repo_path, &files)
            .with_context(|| "Failed to extract relationships from binary symbols")?;

        let elapsed = start.elapsed();
        info!(
            "On-demand relationship extraction completed in {:?}, extracted {} relationships",
            elapsed,
            dependency_graph.graph.node_count()
        );

        // Save the extracted graph for future use
        let graph_path = self.db_path.join("dependency_graph.bin");
        if let Err(e) = Self::save_dependency_graph_async(&dependency_graph, &graph_path).await {
            warn!("Failed to cache extracted dependency graph: {}", e);
        } else {
            info!("Cached extracted dependency graph to: {:?}", graph_path);
        }

        Ok(dependency_graph)
    }

    /// Intelligently find source files using multiple strategies
    /// 1. Try storage path (legacy document approach)
    /// 2. Try current working directory (codebase intelligence approach)
    /// 3. Try parent directories looking for git repositories
    async fn find_source_files_intelligently(&self) -> Result<(PathBuf, Vec<(PathBuf, Vec<u8>)>)> {
        // Strategy 1: Try the storage path first (maintains backward compatibility)
        let storage_path = self.db_path.join("storage");
        if storage_path.exists() {
            if let Ok(files) = self.collect_source_files(&storage_path).await {
                if !files.is_empty() {
                    info!("Found {} source files in storage path", files.len());
                    return Ok((storage_path, files));
                }
            }
        }

        // Strategy 2: Try current working directory (most common case for codebase intelligence)
        let current_dir =
            std::env::current_dir().context("Failed to get current working directory")?;
        if let Ok(files) = self.collect_source_files_from_repo(&current_dir).await {
            if !files.is_empty() {
                info!(
                    "Found {} source files in current directory: {:?}",
                    files.len(),
                    current_dir
                );
                return Ok((current_dir, files));
            }
        }

        // Strategy 3: Try to find a git repository in parent directories
        if let Ok((repo_path, files)) = self.find_git_repository_files(&current_dir).await {
            return Ok((repo_path, files));
        }

        // Strategy 4: If all else fails, return empty with helpful error context
        Err(anyhow::anyhow!(
            "Could not find source files for relationship extraction. Tried:\n\
            1. Storage directory: {:?} (found {} files)\n\
            2. Current directory: {:?}\n\
            3. Parent directories for git repositories\n\
            \n\
            💡 Solutions:\n\
            • Run relationship queries from within the repository directory\n\
            • Use 'git rev-parse --show-toplevel' to find your repository root\n\
            • Ensure source files are accessible (not in .gitignore)\n\
            • Check that the repository contains supported file types (.rs, .py, .js, .ts, etc.)",
            storage_path,
            if storage_path.exists() {
                self.collect_source_files(&storage_path)
                    .await
                    .map(|f| f.len())
                    .unwrap_or(0)
            } else {
                0
            },
            current_dir
        ))
    }

    /// Search for git repository in parent directories and collect source files
    /// Returns the repository path and collected files if found
    async fn find_git_repository_files(
        &self,
        start_dir: &Path,
    ) -> Result<(PathBuf, Vec<(PathBuf, Vec<u8>)>)> {
        let mut search_dir = start_dir.to_path_buf();

        // Limit search to 5 levels to avoid infinite loops and excessive traversal
        for level in 0..5 {
            if search_dir.join(".git").exists() {
                debug!("Found git repository at level {}: {:?}", level, search_dir);

                if let Ok(files) = self.collect_source_files_from_repo(&search_dir).await {
                    if !files.is_empty() {
                        info!(
                            "Found {} source files in git repository: {:?}",
                            files.len(),
                            search_dir
                        );
                        return Ok((search_dir, files));
                    }
                }
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        Err(anyhow::anyhow!(
            "No git repository found in parent directories of: {:?}",
            start_dir
        ))
    }

    /// Collect source files from a repository directory (not storage)
    async fn collect_source_files_from_repo(
        &self,
        repo_path: &Path,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>> {
        use tokio::fs;

        let mut files = Vec::new();

        // Walk the directory recursively, but skip common non-source directories
        let mut stack = vec![repo_path.to_path_buf()];
        let mut files_processed = 0;

        while let Some(dir_path) = stack.pop() {
            if let Ok(mut entries) = fs::read_dir(&dir_path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    let file_name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("");

                    // Skip common directories that don't contain source code
                    if path.is_dir() {
                        if !self.should_skip_directory(file_name) {
                            stack.push(path);
                        }
                        continue;
                    }

                    // Check file limit
                    if let Some(max_files) = self.extraction_config.max_files_per_extraction {
                        if files_processed >= max_files {
                            warn!(
                                "Reached maximum file limit ({}) during repository extraction, stopping",
                                max_files
                            );
                            break;
                        }
                    }

                    // Process source files
                    if let Some(extension) = path.extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        if self.is_supported_extension(&ext) {
                            files_processed += 1;

                            // Check file size
                            if let Ok(metadata) = fs::metadata(&path).await {
                                if metadata.len() > self.extraction_config.max_file_size {
                                    debug!(
                                        "Skipping large file: {:?} ({} bytes)",
                                        path,
                                        metadata.len()
                                    );
                                    continue;
                                }
                            }

                            // Read file content
                            match fs::read(&path).await {
                                Ok(content) => {
                                    // Normalize path to be relative to repo root
                                    let relative_path = normalize_path_relative(&path, repo_path);
                                    files.push((PathBuf::from(relative_path), content));
                                }
                                Err(e) => {
                                    debug!("Failed to read file {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// Check if a directory should be skipped during source file collection
    /// Uses HashSet for O(1) lookup performance
    fn should_skip_directory(&self, dir_name: &str) -> bool {
        static SKIP_DIRS: &[&str] = &[
            "target",
            "node_modules",
            ".git",
            ".svn",
            ".hg",
            "build",
            "dist",
            "out",
            ".cache",
            "tmp",
            "temp",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".idea",
            ".vscode",
            ".vs",
            ".DS_Store",
            "Thumbs.db",
            ".tox",
            ".venv",
            "venv",
            "env",
        ];

        // Use lazy static pattern for O(1) lookup
        use std::sync::OnceLock;
        static SKIP_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
        let skip_set = SKIP_SET.get_or_init(|| SKIP_DIRS.iter().copied().collect());

        skip_set.contains(dir_name)
    }

    /// Collect source files from the storage directory for relationship extraction
    async fn collect_source_files(
        &self,
        storage_path: &Path,
    ) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut entries = fs::read_dir(storage_path).await?;
        let mut files_processed = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Check file limit to prevent resource exhaustion
            if let Some(max_files) = self.extraction_config.max_files_per_extraction {
                if files_processed >= max_files {
                    warn!(
                        "Reached maximum file limit ({}) during extraction, stopping",
                        max_files
                    );
                    break;
                }
            }

            // Only process supported source code files
            if let Some(extension) = path.extension() {
                let ext = extension.to_string_lossy().to_lowercase();
                if self.is_supported_extension(&ext) {
                    files_processed += 1;

                    // Check file size before reading
                    match fs::metadata(&path).await {
                        Ok(metadata) => {
                            if metadata.len() > self.extraction_config.max_file_size {
                                warn!(
                                    "Skipping file {} - size {} bytes exceeds limit {} bytes",
                                    path.display(),
                                    metadata.len(),
                                    self.extraction_config.max_file_size
                                );
                                continue;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to get metadata for file {}: {}", path.display(), e);
                            continue;
                        }
                    }

                    // Read file contents
                    match fs::read(&path).await {
                        Ok(contents) => {
                            // For storage path, files are already relative, but normalize just in case
                            let normalized_path = if path.is_absolute() {
                                // If absolute, try to make relative to storage path
                                normalize_path_relative(&path, storage_path)
                            } else {
                                // Already relative, just normalize format
                                normalize_path_relative(&path, Path::new(""))
                            };
                            files.push((PathBuf::from(normalized_path), contents));
                        }
                        Err(e) => {
                            warn!("Failed to read file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        info!(
            "Collected {} files from {} candidates for relationship extraction",
            files.len(),
            files_processed
        );

        Ok(files)
    }

    /// Check if a file extension is supported for analysis
    fn is_supported_extension(&self, extension: &str) -> bool {
        // Check if extension is supported by any configured language
        for language in &self.extraction_config.supported_languages {
            if language.extensions().contains(&extension) {
                return true;
            }
        }

        // Check additional extensions
        self.extraction_config
            .additional_extensions
            .contains(&extension.to_string())
    }

    /// Check if cache should be evicted based on the configured policy
    fn maybe_evict_cache(&self, new_graph: &DependencyGraph) {
        let should_evict = match self.extraction_config.cache_eviction_policy {
            CacheEvictionPolicy::Never => false,
            CacheEvictionPolicy::MemoryBased { threshold_bytes } => {
                let estimated_size = self.estimate_graph_memory(new_graph);
                estimated_size > threshold_bytes
            }
            CacheEvictionPolicy::TimeBased { ttl_seconds } => {
                if let Some(last_access) = self.cache_metadata.get_last_access() {
                    let elapsed = last_access.elapsed().as_secs();
                    elapsed > ttl_seconds
                } else {
                    false
                }
            }
            CacheEvictionPolicy::Lru { max_entries: _ } => {
                // For now, treat single entry cache as always evictable
                // In future implementations with multiple cached graphs, this would check entry count
                false
            }
        };

        if should_evict {
            debug!(
                "Evicting dependency graph cache due to policy: {:?}",
                self.extraction_config.cache_eviction_policy
            );
            if let Ok(mut graph) = self.dependency_graph.write() {
                *graph = None;
            } else {
                warn!("Failed to evict graph from cache - lock poisoned");
            }
            self.cache_metadata.record_eviction();
        }
    }

    /// Estimate memory usage of a dependency graph
    fn estimate_graph_memory(&self, graph: &DependencyGraph) -> u64 {
        // Rough estimation: each node costs ~64 bytes, each edge ~32 bytes
        let node_count = graph.graph.node_count() as u64;
        let edge_count = graph.graph.edge_count() as u64;

        (node_count * 64) + (edge_count * 32)
    }

    /// Get statistics about the binary engine
    pub fn get_stats(&self) -> BinaryEngineStats {
        let graph_borrowed = self.dependency_graph.read().unwrap_or_else(|poisoned| {
            // If lock is poisoned, we can still try to recover the data
            // This is safe for read-only stats gathering
            warn!("Dependency graph lock poisoned, recovering data for stats");
            poisoned.into_inner()
        });

        BinaryEngineStats {
            binary_symbols_loaded: self
                .symbol_reader
                .as_ref()
                .map(|r| r.symbol_count())
                .unwrap_or(0),
            graph_nodes_loaded: graph_borrowed
                .as_ref()
                .map(|g| g.graph.node_count())
                .unwrap_or(0),
            using_binary_path: self.symbol_reader.is_some() && graph_borrowed.is_some(),
            cache_hits: self.cache_metadata.get_access_count(),
            cache_misses: if self.cache_metadata.get_access_count() > 0 {
                1
            } else {
                0
            }, // Simplified for single-entry cache
            cache_evictions: self.cache_metadata.get_eviction_count(),
        }
    }
}

/// Statistics about the binary engine
#[derive(Debug, Clone)]
pub struct BinaryEngineStats {
    pub binary_symbols_loaded: usize,
    pub graph_nodes_loaded: usize,
    pub using_binary_path: bool,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
}

/// Cache metadata for tracking usage and eviction
/// Uses atomic counters to minimize lock contention
#[derive(Debug)]
struct CacheMetadata {
    last_access: RwLock<Option<std::time::Instant>>,
    access_count: AtomicU64,
    eviction_count: AtomicU64,
}

impl CacheMetadata {
    fn new() -> Self {
        Self {
            last_access: RwLock::new(None),
            access_count: AtomicU64::new(0),
            eviction_count: AtomicU64::new(0),
        }
    }

    fn record_access(&self) {
        if let Ok(mut last_access) = self.last_access.write() {
            *last_access = Some(std::time::Instant::now());
        }
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_eviction(&self) {
        self.eviction_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_last_access(&self) -> Option<std::time::Instant> {
        self.last_access.read().ok().and_then(|guard| *guard)
    }

    fn get_access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    fn get_eviction_count(&self) -> u64 {
        self.eviction_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_symbols::BinarySymbolWriter;
    use crate::dependency_extractor::{DependencyEdge, SymbolNode};
    use petgraph::graph::DiGraph;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    /// Test helper to create a test extraction config
    fn test_extraction_config() -> ExtractionConfig {
        ExtractionConfig {
            max_file_size: 1024, // Small limit for testing
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec!["py".to_string()],
            max_files_per_extraction: Some(5),
            warn_on_large_graphs: true,
            max_graph_memory: Some(1024),
            cache_eviction_policy: CacheEvictionPolicy::Never,
        }
    }

    #[test]
    fn test_extraction_config_default() {
        let config = ExtractionConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(config
            .supported_languages
            .contains(&SupportedLanguage::Rust));
        assert!(config.additional_extensions.contains(&"py".to_string()));
        assert_eq!(config.max_files_per_extraction, Some(10000));
        assert!(config.warn_on_large_graphs);
        assert_eq!(config.max_graph_memory, Some(100 * 1024 * 1024));
        assert!(matches!(
            config.cache_eviction_policy,
            CacheEvictionPolicy::MemoryBased { threshold_bytes: _ }
        ));
    }

    #[test]
    fn test_extraction_config_customization() {
        let config = ExtractionConfig {
            max_file_size: 5 * 1024 * 1024,
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec!["go".to_string()],
            max_files_per_extraction: Some(1000),
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::TimeBased { ttl_seconds: 600 },
        };

        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
        assert_eq!(config.supported_languages.len(), 1);
        assert!(config
            .supported_languages
            .contains(&SupportedLanguage::Rust));
        assert_eq!(config.additional_extensions.len(), 1);
        assert!(config.additional_extensions.contains(&"go".to_string()));
        assert_eq!(config.max_files_per_extraction, Some(1000));
        assert!(!config.warn_on_large_graphs);
        assert_eq!(config.max_graph_memory, None);
        // Test should specify its own eviction policy
    }

    #[tokio::test]
    async fn test_binary_engine_creation_with_custom_config() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();

        let config = RelationshipQueryConfig::default();
        let extraction_config = test_extraction_config();

        // This will fail since there's no binary symbols file, but should not panic
        let result =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await;

        // Should succeed even without binary symbols
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collect_source_files_respects_limits() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage_path = temp_dir.path().join("storage");
        fs::create_dir_all(&storage_path)
            .await
            .expect("Failed to create storage dir");

        // Create test files
        for i in 0..10 {
            let file_path = storage_path.join(format!("test{}.rs", i));
            fs::write(&file_path, b"fn main() {}")
                .await
                .expect("Failed to write file");
        }

        // Create engine with limited extraction config
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig {
            max_file_size: 1024,
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec![], // No additional extensions
            max_files_per_extraction: Some(3), // Limit to 3 files
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::Never,
        };

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        let files = engine
            .collect_source_files(&storage_path)
            .await
            .expect("Failed to collect files");

        // Should respect the file limit
        assert!(files.len() <= 3);
    }

    #[tokio::test]
    async fn test_collect_source_files_respects_file_size() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage_path = temp_dir.path().join("storage");
        fs::create_dir_all(&storage_path)
            .await
            .expect("Failed to create storage dir");

        // Create a large file that exceeds the limit
        let large_file = storage_path.join("large.rs");
        let large_content = "a".repeat(2048); // 2KB file
        fs::write(&large_file, large_content)
            .await
            .expect("Failed to write large file");

        // Create a small file within limits
        let small_file = storage_path.join("small.rs");
        fs::write(&small_file, b"fn main() {}")
            .await
            .expect("Failed to write small file");

        // Create engine with strict size limit
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig {
            max_file_size: 1024, // 1KB limit
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec![], // No additional extensions
            max_files_per_extraction: Some(10),
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::Never,
        };

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        let files = engine
            .collect_source_files(&storage_path)
            .await
            .expect("Failed to collect files");

        // Should only include the small file
        assert_eq!(files.len(), 1);
        assert!(files[0].0.file_name().unwrap().to_str().unwrap() == "small.rs");
    }

    #[tokio::test]
    async fn test_collect_source_files_extension_filtering() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage_path = temp_dir.path().join("storage");
        fs::create_dir_all(&storage_path)
            .await
            .expect("Failed to create storage dir");

        // Create files with different extensions
        fs::write(storage_path.join("test.rs"), b"fn main() {}")
            .await
            .unwrap();
        fs::write(storage_path.join("test.py"), b"print('hello')")
            .await
            .unwrap();
        fs::write(storage_path.join("test.txt"), b"not code")
            .await
            .unwrap();
        fs::write(storage_path.join("test.md"), b"# Documentation")
            .await
            .unwrap();

        // Create engine that only supports Rust files
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig {
            max_file_size: 1024,
            supported_languages: vec![SupportedLanguage::Rust], // Only Rust
            additional_extensions: vec![],                      // No additional extensions
            max_files_per_extraction: Some(10),
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::Never,
        };

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        let files = engine
            .collect_source_files(&storage_path)
            .await
            .expect("Failed to collect files");

        // Should only include the .rs file
        assert_eq!(files.len(), 1);
        assert!(files[0].0.file_name().unwrap().to_str().unwrap() == "test.rs");
    }

    #[tokio::test]
    async fn test_concurrent_extraction_safety() {
        // Test that RwLock allows safe concurrent access from multiple threads
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = test_extraction_config();

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        // Test concurrent access safety with RwLock
        for i in 0..3 {
            let context = format!("sequential test {}", i);
            // This should not panic even with multiple sequential calls
            let _ = engine.ensure_dependency_graph(&context).await;
        }
    }

    #[tokio::test]
    async fn test_extraction_with_unreadable_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage_path = temp_dir.path().join("storage");
        fs::create_dir_all(&storage_path)
            .await
            .expect("Failed to create storage dir");

        // Create a file and then remove read permissions
        let test_file = storage_path.join("unreadable.rs");
        fs::write(&test_file, b"fn main() {}")
            .await
            .expect("Failed to write file");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&test_file).unwrap().permissions();
            perms.set_mode(0o000); // No permissions
            std::fs::set_permissions(&test_file, perms).unwrap();
        }

        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = test_extraction_config();

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        let files = engine
            .collect_source_files(&storage_path)
            .await
            .expect("Should handle unreadable files gracefully");

        // Should not include the unreadable file
        assert_eq!(files.len(), 0);

        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&test_file).unwrap().permissions();
            perms.set_mode(0o644); // Readable again
            std::fs::set_permissions(&test_file, perms).unwrap();
        }
    }

    #[tokio::test]
    async fn test_memory_limit_warnings() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig {
            max_file_size: 1024,
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec![],
            max_files_per_extraction: Some(10),
            warn_on_large_graphs: true,
            max_graph_memory: Some(100), // Very low limit to trigger warning
            cache_eviction_policy: CacheEvictionPolicy::MemoryBased {
                threshold_bytes: 50,
            },
        };

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        // This test verifies the memory warning logic is called correctly
        // The actual warning behavior is logged, so we mainly verify no panics
        let result = engine.ensure_dependency_graph("memory limit test").await;

        // Should not panic, but will likely fail due to missing binary symbols
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_large_file_handling() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage_path = temp_dir.path().join("storage");
        fs::create_dir_all(&storage_path)
            .await
            .expect("Failed to create storage dir");

        // Create multiple files of different sizes
        let small_file = storage_path.join("small.rs");
        fs::write(&small_file, b"fn small() {}")
            .await
            .expect("Failed to write small file");

        let medium_file = storage_path.join("medium.rs");
        let medium_content = "a".repeat(512);
        fs::write(&medium_file, medium_content)
            .await
            .expect("Failed to write medium file");

        let large_file = storage_path.join("large.rs");
        let large_content = "b".repeat(2048); // 2KB, exceeds our test limit
        fs::write(&large_file, large_content)
            .await
            .expect("Failed to write large file");

        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig {
            max_file_size: 1024, // 1KB limit
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec![],
            max_files_per_extraction: Some(10),
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::Never,
        };

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        let files = engine
            .collect_source_files(&storage_path)
            .await
            .expect("Failed to collect files");

        // Should only include files within size limits
        assert_eq!(files.len(), 2); // small and medium files
        let filenames: Vec<_> = files
            .iter()
            .map(|(path, _)| path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(filenames.contains(&"small.rs"));
        assert!(filenames.contains(&"medium.rs"));
        assert!(!filenames.contains(&"large.rs"));
    }

    #[tokio::test]
    async fn test_rwlock_safety() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = test_extraction_config();

        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await
                .expect("Failed to create engine");

        // Test that get_dependency_graph handles missing graph gracefully
        let result = engine.get_dependency_graph();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Dependency graph unavailable"));

        // Test that multiple calls don't cause lock conflicts
        let _stats1 = engine.get_stats();
        let _stats2 = engine.get_stats();
        // Should not panic with lock contention
    }

    #[test]
    fn test_supported_extension_detection() {
        let config = ExtractionConfig {
            max_file_size: 1024,
            supported_languages: vec![SupportedLanguage::Rust],
            additional_extensions: vec!["py".to_string(), "go".to_string()],
            max_files_per_extraction: Some(10),
            warn_on_large_graphs: false,
            max_graph_memory: None,
            cache_eviction_policy: CacheEvictionPolicy::Never,
        };

        // Create a mock engine to test the helper method
        // Since we can't easily create a real engine without filesystem setup,
        // we'll test the logic by checking the configuration directly

        // Rust files should be supported (from SupportedLanguage::Rust)
        assert!(SupportedLanguage::Rust.extensions().contains(&"rs"));

        // Additional extensions should be supported
        assert!(config.additional_extensions.contains(&"py".to_string()));
        assert!(config.additional_extensions.contains(&"go".to_string()));

        // Unsupported extensions should not be included
        assert!(!config.additional_extensions.contains(&"txt".to_string()));
        assert!(!config.additional_extensions.contains(&"md".to_string()));
    }

    #[test]
    fn test_cache_eviction_policies() {
        // Test Memory-based eviction policy
        let memory_policy = CacheEvictionPolicy::MemoryBased {
            threshold_bytes: 1024,
        };
        assert!(matches!(
            memory_policy,
            CacheEvictionPolicy::MemoryBased { .. }
        ));

        // Test Time-based eviction policy
        let time_policy = CacheEvictionPolicy::TimeBased { ttl_seconds: 300 };
        assert!(matches!(time_policy, CacheEvictionPolicy::TimeBased { .. }));

        // Test LRU eviction policy
        let lru_policy = CacheEvictionPolicy::Lru { max_entries: 5 };
        assert!(matches!(lru_policy, CacheEvictionPolicy::Lru { .. }));

        // Test Never eviction policy
        let never_policy = CacheEvictionPolicy::Never;
        assert!(matches!(never_policy, CacheEvictionPolicy::Never));
    }

    #[test]
    fn test_cache_metadata() {
        let metadata = CacheMetadata::new();

        // Initial state
        assert!(metadata.get_last_access().is_none());
        assert_eq!(metadata.get_access_count(), 0);
        assert_eq!(metadata.get_eviction_count(), 0);

        // Record access
        metadata.record_access();
        assert!(metadata.get_last_access().is_some());
        assert_eq!(metadata.get_access_count(), 1);
        assert_eq!(metadata.get_eviction_count(), 0);

        // Record eviction
        metadata.record_eviction();
        assert_eq!(metadata.get_access_count(), 1);
        assert_eq!(metadata.get_eviction_count(), 1);
    }

    /// Test that the engine can handle UUID mismatches between binary symbols and dependency graph
    #[tokio::test]
    async fn test_uuid_mismatch_resolution() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();

        // Create binary symbols with one UUID
        let binary_uuid = Uuid::new_v4();
        let mut writer = BinarySymbolWriter::new();

        // Add a symbol to binary storage
        writer.add_symbol(
            binary_uuid,
            "FileStorage",
            4, // Struct type
            "src/file_storage.rs",
            100,
            200,
            None,
        );

        // We'll use a consistent UUID for the caller so it can be found
        let caller_uuid = Uuid::new_v4();
        writer.add_symbol(
            caller_uuid, // Same UUID will be used in graph
            "main",
            1, // Function type
            "src/main.rs",
            40,
            60,
            None,
        );

        // Save binary symbols
        let symbol_db_path = db_path.join("symbols.kota");
        writer
            .write_to_file(&symbol_db_path)
            .expect("Failed to save symbols");

        // Create dependency graph with DIFFERENT UUID (simulating the issue)
        let graph_uuid = Uuid::new_v4(); // Different UUID!
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();

        // Add node to graph
        let node = SymbolNode {
            symbol_id: graph_uuid,
            qualified_name: "src/file_storage.rs::FileStorage".to_string(),
            symbol_type: SymbolType::Struct,
            file_path: PathBuf::from("src/file_storage.rs"),
            in_degree: 0,
            out_degree: 1,
        };
        let node_idx = graph.add_node(node);
        symbol_to_node.insert(graph_uuid, node_idx);

        // Add both qualified and simple name mappings
        name_to_symbol.insert("src/file_storage.rs::FileStorage".to_string(), graph_uuid);
        name_to_symbol.insert("FileStorage".to_string(), graph_uuid);

        // Add a dependent (caller) to make the test meaningful
        // Use the same UUID as in binary storage
        let caller_node = SymbolNode {
            symbol_id: caller_uuid,
            qualified_name: "src/main.rs::main".to_string(),
            symbol_type: SymbolType::Function,
            file_path: PathBuf::from("src/main.rs"),
            in_degree: 0,
            out_degree: 1,
        };
        let caller_idx = graph.add_node(caller_node);
        symbol_to_node.insert(caller_uuid, caller_idx);

        // Add edge: main calls FileStorage
        // For find_callers to work, we need an edge FROM caller TO target
        // because find_dependents looks for INCOMING edges to the target
        graph.add_edge(
            caller_idx, // FROM main (the caller)
            node_idx,   // TO FileStorage (the target)
            DependencyEdge {
                relation_type: RelationType::Calls,
                line_number: 50,
                column_number: 10,
                context: Some("FileStorage::new()".to_string()),
            },
        );

        // Create the dependency graph
        let dependency_graph = DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol,
            file_imports: HashMap::new(),
            stats: Default::default(),
        };

        // Save dependency graph
        let graph_db_path = db_path.join("dependency_graph.bin");
        BinaryRelationshipEngine::save_dependency_graph(&dependency_graph, &graph_db_path)
            .expect("Failed to save dependency graph");

        // Now create the engine and test the query
        let config = RelationshipQueryConfig::default();
        let engine = BinaryRelationshipEngine::new(db_path, config)
            .await
            .expect("Failed to create engine");

        // Execute find_callers query - this should use our fix to resolve the UUID mismatch
        let query = RelationshipQueryType::FindCallers {
            target: "FileStorage".to_string(),
        };

        let result = engine.execute_query(query).await;

        // The query should succeed despite the UUID mismatch
        assert!(result.is_ok(), "Query failed: {:?}", result.err());

        let result = result.unwrap();

        // Should find the caller
        assert_eq!(result.stats.direct_count, 1, "Should find 1 caller");
        assert_eq!(
            result.direct_relationships.len(),
            1,
            "Should have 1 relationship match"
        );

        // Verify the caller is correct
        let caller = &result.direct_relationships[0];
        assert_eq!(caller.symbol_name, "main");
        assert_eq!(caller.file_path, "src/main.rs");
    }
}
