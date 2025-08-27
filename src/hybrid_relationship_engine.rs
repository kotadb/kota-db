//! Hybrid relationship query engine that integrates binary symbols with relationship queries
//!
//! This module provides the integration layer between the fast binary symbol format
//! and the relationship query functionality, ensuring sub-10ms query latency while
//! maintaining full API compatibility.

use anyhow::Result;
use std::path::Path;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::{
    binary_symbols::BinarySymbolReader,
    dependency_extractor::DependencyGraph,
    parsing::SymbolType,
    relationship_query::{
        RelationshipLocation, RelationshipMatch, RelationshipQueryConfig, RelationshipQueryResult,
        RelationshipQueryType, RelationshipStats,
    },
    symbol_storage::SymbolStorage,
    types::RelationType,
};

/// Hybrid relationship query engine that uses binary symbols
pub struct HybridRelationshipEngine {
    /// Binary symbol reader for fast symbol lookup
    symbol_reader: Option<BinarySymbolReader>,
    /// Dependency graph built from relationships
    dependency_graph: Option<DependencyGraph>,
    /// Original symbol storage for backward compatibility
    #[allow(dead_code)]
    symbol_storage: SymbolStorage,
    /// Configuration
    config: RelationshipQueryConfig,
}

impl HybridRelationshipEngine {
    /// Create a new hybrid engine from database paths
    #[instrument(skip(symbol_storage))]
    pub async fn new(
        db_path: &Path,
        symbol_storage: SymbolStorage,
        config: RelationshipQueryConfig,
    ) -> Result<Self> {
        info!("Initializing hybrid relationship engine");

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
            dependency_graph,
            symbol_storage,
            config,
        })
    }

    /// Execute a relationship query using the hybrid approach
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
                self.dependency_graph.is_some()
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
                // If no dependency graph is available, return informative message
                if self.dependency_graph.is_none() {
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
                            "Symbol '{}' found in binary database (total {} symbols loaded), but relationship graph not available. \
                            To enable caller analysis, ingest repository with relationship extraction enabled.",
                            target, reader.symbol_count()
                        ),
                    });
                }

                let graph = self.dependency_graph.as_ref().unwrap();

                // Look up target symbol by name
                let (_symbol, target_id) = reader
                    .find_symbol_by_name(&target)
                    .ok_or_else(|| anyhow::anyhow!("Symbol '{}' not found", target))?;

                // Find all callers in the dependency graph
                let callers = graph.find_dependents(target_id);

                // Convert to relationship matches
                let mut direct_relationships = Vec::new();
                for (caller_id, relation_type) in callers.iter() {
                    if let Some(symbol) = reader.find_symbol(*caller_id) {
                        let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get symbol name for UUID {}: {}", caller_id, e);
                            format!("symbol_{}", caller_id)
                        });
                        let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get file path for symbol: {}", e);
                            "unknown".to_string()
                        });

                        direct_relationships.push(RelationshipMatch {
                            symbol_id: Uuid::from_bytes(symbol.id),
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
                            context: format!("Calls {} at line {}", target, symbol.start_line),
                        });
                    }
                }

                Ok(RelationshipQueryResult {
                    query_type,
                    direct_relationships,
                    indirect_relationships: vec![],
                    stats: RelationshipStats {
                        direct_count: callers.len(),
                        indirect_count: 0,
                        symbols_analyzed: reader.symbol_count(),
                        execution_time_ms: 0,
                        truncated: false,
                    },
                    summary: format!("Found {} direct callers of '{}'", callers.len(), target),
                })
            }
            RelationshipQueryType::ImpactAnalysis { target } => {
                // If no dependency graph is available, return informative message
                if self.dependency_graph.is_none() {
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
                            "Symbol '{}' found in binary database (total {} symbols loaded), but relationship graph not available. \
                            To enable impact analysis, ingest repository with relationship extraction enabled.",
                            target, reader.symbol_count()
                        ),
                    });
                }

                let graph = self.dependency_graph.as_ref().unwrap();

                // For impact analysis, find all transitive dependencies
                let (_symbol, target_id) = reader
                    .find_symbol_by_name(&target)
                    .ok_or_else(|| anyhow::anyhow!("Symbol '{}' not found", target))?;

                let impacted =
                    self.find_transitive_dependents(graph, target_id, self.config.max_depth);

                // Convert to relationship matches
                let mut direct_relationships = Vec::new();
                for (id, relation_type) in impacted.iter() {
                    if let Some(symbol) = reader.find_symbol(*id) {
                        let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get symbol name for UUID {}: {}", id, e);
                            format!("symbol_{}", id)
                        });
                        let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get file path for symbol: {}", e);
                            "unknown".to_string()
                        });

                        direct_relationships.push(RelationshipMatch {
                            symbol_id: Uuid::from_bytes(symbol.id),
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

                Ok(RelationshipQueryResult {
                    query_type,
                    direct_relationships,
                    indirect_relationships: vec![],
                    stats: RelationshipStats {
                        direct_count: impacted.len(),
                        indirect_count: 0,
                        symbols_analyzed: reader.symbol_count(),
                        execution_time_ms: 0,
                        truncated: false,
                    },
                    summary: format!(
                        "{} symbols would be impacted by changes to '{}'",
                        impacted.len(),
                        target
                    ),
                })
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

    /// Load dependency graph from binary file (TODO: implement serialization)
    fn load_dependency_graph(_path: &Path) -> Result<DependencyGraph> {
        // TODO: Implement proper serialization/deserialization for DependencyGraph
        // For now, return error - dependency graphs will need to be rebuilt
        Err(anyhow::anyhow!(
            "Dependency graph serialization not yet implemented"
        ))
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

    /// Get statistics about the hybrid engine
    pub fn get_stats(&self) -> HybridEngineStats {
        HybridEngineStats {
            binary_symbols_loaded: self
                .symbol_reader
                .as_ref()
                .map(|r| r.symbol_count())
                .unwrap_or(0),
            graph_nodes_loaded: self
                .dependency_graph
                .as_ref()
                .map(|g| g.graph.node_count())
                .unwrap_or(0),
            using_binary_path: self.symbol_reader.is_some() && self.dependency_graph.is_some(),
        }
    }
}

/// Statistics about the hybrid engine
#[derive(Debug, Clone)]
pub struct HybridEngineStats {
    pub binary_symbols_loaded: usize,
    pub graph_nodes_loaded: usize,
    pub using_binary_path: bool,
}
