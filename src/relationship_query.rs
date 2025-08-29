//! Relationship query interface for dependency graph navigation
//!
//! This module provides the "killer feature" that differentiates KotaDB from text search tools:
//! the ability to understand and query code relationships. It enables LLMs to perform
//! impact analysis, trace call chains, and understand architectural dependencies.
//!
//! Key capabilities:
//! - "What calls this function?" (reverse dependencies)
//! - "What does this function call?" (forward dependencies)
//! - "What would break if I change this?" (impact analysis)
//! - "Show me the call chain from A to B" (path finding)

use crate::{
    dependency_extractor::DependencyGraph,
    parsing::SymbolType,
    symbol_storage::{RelationType, SymbolStorage},
};
use anyhow::{Context, Result};
use petgraph::{graph::NodeIndex, visit::EdgeRef, Direction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

/// Configuration for relationship query engine
#[derive(Debug, Clone)]
pub struct RelationshipQueryConfig {
    /// Maximum depth for transitive queries (default: 5)
    pub max_depth: usize,
    /// Maximum number of indirect paths to find (default: 1000)
    pub max_indirect_paths: usize,
    /// Maximum number of nodes to visit in a single query (default: 10000)
    pub max_visited_nodes: usize,
}

impl Default for RelationshipQueryConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_indirect_paths: 1000,
            max_visited_nodes: 10000,
        }
    }
}

/// Core relationship query engine that operates on dependency graphs
pub struct RelationshipQueryEngine {
    /// The dependency graph containing all code relationships
    dependency_graph: DependencyGraph,
    /// Symbol storage for additional metadata and symbol management
    #[allow(dead_code)] // Will be used in future enhancements
    symbol_storage: SymbolStorage,
    /// Configuration for query limits
    config: RelationshipQueryConfig,
}

/// Types of relationship queries supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipQueryType {
    /// Find all symbols that call/use the target symbol
    FindCallers { target: String },
    /// Find all symbols that the target symbol calls/uses
    FindCallees { target: String },
    /// Find all symbols that would be impacted by changing the target
    ImpactAnalysis { target: String },
    /// Find the shortest path between two symbols
    CallChain { from: String, to: String },
    /// Find circular dependencies involving the target
    CircularDependencies { target: Option<String> },
    /// Find unused symbols (no incoming dependencies)
    UnusedSymbols { symbol_type: Option<SymbolType> },
    /// Find hotpaths (most frequently called symbols)
    HotPaths { limit: Option<usize> },
    /// Find all dependencies of a specific type
    DependenciesByType {
        target: String,
        relation_type: RelationType,
    },
}

/// Result of a relationship query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipQueryResult {
    /// The type of query that was executed
    pub query_type: RelationshipQueryType,
    /// Direct relationships found
    pub direct_relationships: Vec<RelationshipMatch>,
    /// Indirect relationships (for impact analysis, call chains)
    pub indirect_relationships: Vec<CallPath>,
    /// Statistics about the query execution
    pub stats: RelationshipStats,
    /// Human-readable summary
    pub summary: String,
}

/// A single relationship match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMatch {
    /// The symbol involved in the relationship
    pub symbol_id: Uuid,
    /// Symbol name and metadata
    pub symbol_name: String,
    /// Qualified name (full path)
    pub qualified_name: String,
    /// Symbol type (function, struct, etc.)
    pub symbol_type: SymbolType,
    /// File containing the symbol
    pub file_path: String,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Location of the relationship in source code
    pub location: RelationshipLocation,
    /// Context around the relationship
    pub context: String,
}

/// Location information for a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLocation {
    /// Line number where the relationship occurs
    pub line_number: usize,
    /// Column number
    pub column_number: usize,
    /// Source file path
    pub file_path: String,
}

/// A call path between two symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPath {
    /// The symbols in the path from source to target
    pub path: Vec<Uuid>,
    /// Symbol names for display
    pub symbol_names: Vec<String>,
    /// Total path length
    pub distance: usize,
    /// Path description for humans
    pub description: String,
}

/// Statistics about relationship query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipStats {
    /// Number of direct relationships found
    pub direct_count: usize,
    /// Number of indirect relationships found
    pub indirect_count: usize,
    /// Total symbols analyzed
    pub symbols_analyzed: usize,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether the query result was truncated
    pub truncated: bool,
}

impl RelationshipQueryEngine {
    /// Create a new relationship query engine with default configuration
    pub fn new(dependency_graph: DependencyGraph, symbol_storage: SymbolStorage) -> Self {
        Self::with_config(
            dependency_graph,
            symbol_storage,
            RelationshipQueryConfig::default(),
        )
    }

    /// Create a new relationship query engine with custom configuration
    pub fn with_config(
        dependency_graph: DependencyGraph,
        symbol_storage: SymbolStorage,
        config: RelationshipQueryConfig,
    ) -> Self {
        Self {
            dependency_graph,
            symbol_storage,
            config,
        }
    }

    /// Execute a relationship query
    #[instrument(skip(self))]
    pub async fn execute_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        let start_time = std::time::Instant::now();
        debug!("Executing relationship query: {:?}", query_type);

        let result = match &query_type {
            RelationshipQueryType::FindCallers { target } => self.find_callers(target).await?,
            RelationshipQueryType::FindCallees { target } => self.find_callees(target).await?,
            RelationshipQueryType::ImpactAnalysis { target } => {
                self.impact_analysis(target).await?
            }
            RelationshipQueryType::CallChain { from, to } => self.find_call_chain(from, to).await?,
            RelationshipQueryType::CircularDependencies { target } => {
                self.find_circular_dependencies(target.as_deref()).await?
            }
            RelationshipQueryType::UnusedSymbols { symbol_type } => {
                self.find_unused_symbols(symbol_type.as_ref()).await?
            }
            RelationshipQueryType::HotPaths { limit } => {
                self.find_hot_paths(limit.unwrap_or(10)).await?
            }
            RelationshipQueryType::DependenciesByType {
                target,
                relation_type,
            } => {
                self.find_dependencies_by_type(target, relation_type)
                    .await?
            }
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let mut final_result = result;
        final_result.stats.execution_time_ms = execution_time_ms;

        debug!(
            "Relationship query completed in {}ms, found {} direct and {} indirect relationships",
            execution_time_ms, final_result.stats.direct_count, final_result.stats.indirect_count
        );

        Ok(final_result)
    }

    /// Find all callers of a target symbol
    async fn find_callers(&self, target: &str) -> Result<RelationshipQueryResult> {
        let symbol_id = self
            .resolve_symbol_name(target)
            .context("Failed to resolve target symbol")?;

        let dependents = self.dependency_graph.find_dependents(symbol_id);
        let mut relationships = Vec::new();

        for (dependent_id, relation_type) in dependents {
            if let Some(relationship) = self
                .create_relationship_match(dependent_id, relation_type, &symbol_id)
                .await?
            {
                relationships.push(relationship);
            }
        }

        let summary = format!(
            "Found {} symbols that call/use '{}'",
            relationships.len(),
            target
        );

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::FindCallers {
                target: target.to_string(),
            },
            direct_relationships: relationships.clone(),
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: relationships.len(),
                indirect_count: 0,
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0, // Will be set by caller
                truncated: false,
            },
            summary,
        })
    }

    /// Find all callees of a target symbol
    async fn find_callees(&self, target: &str) -> Result<RelationshipQueryResult> {
        let symbol_id = self
            .resolve_symbol_name(target)
            .context("Failed to resolve target symbol")?;

        let dependencies = self.dependency_graph.find_dependencies(symbol_id);
        let mut relationships = Vec::new();

        for (dependency_id, relation_type) in dependencies {
            if let Some(relationship) = self
                .create_relationship_match(symbol_id, relation_type, &dependency_id)
                .await?
            {
                relationships.push(relationship);
            }
        }

        let summary = format!(
            "Found {} symbols that '{}' calls/uses",
            relationships.len(),
            target
        );

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::FindCallees {
                target: target.to_string(),
            },
            direct_relationships: relationships.clone(),
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: relationships.len(),
                indirect_count: 0,
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary,
        })
    }

    /// Perform impact analysis - find all symbols that would be affected by changing the target
    async fn impact_analysis(&self, target: &str) -> Result<RelationshipQueryResult> {
        let symbol_id = self
            .resolve_symbol_name(target)
            .context("Failed to resolve target symbol")?;

        // Use configurable limits to prevent excessive resource usage
        let max_depth = self.config.max_depth;
        let max_indirect_paths = self.config.max_indirect_paths;
        let max_visited_nodes = self.config.max_visited_nodes;

        // Find direct dependents
        let direct_dependents = self.dependency_graph.find_dependents(symbol_id);
        let mut direct_relationships = Vec::new();
        let mut indirect_paths = Vec::new();

        // Process direct dependents
        for (dependent_id, relation_type) in &direct_dependents {
            if let Some(relationship) = self
                .create_relationship_match(*dependent_id, relation_type.clone(), &symbol_id)
                .await?
            {
                direct_relationships.push(relationship);
            }
        }

        // Find transitive dependents using BFS with memory limits
        let mut visited = HashSet::with_capacity(max_visited_nodes);
        let mut queue = VecDeque::new();

        // Start with direct dependents
        for (dependent_id, _) in direct_dependents {
            queue.push_back((dependent_id, vec![symbol_id, dependent_id], 1));
        }

        let mut truncated = false;
        while let Some((current_id, path, distance)) = queue.pop_front() {
            // Memory and depth limits
            if visited.contains(&current_id)
                || distance > max_depth
                || visited.len() >= max_visited_nodes
                || indirect_paths.len() >= max_indirect_paths
            {
                if visited.len() >= max_visited_nodes || indirect_paths.len() >= max_indirect_paths
                {
                    truncated = true;
                }
                continue;
            }
            visited.insert(current_id);

            let transitive_dependents = self.dependency_graph.find_dependents(current_id);
            for (transitive_id, _) in transitive_dependents {
                if !path.contains(&transitive_id) && indirect_paths.len() < max_indirect_paths {
                    let mut new_path = path.clone();
                    new_path.push(transitive_id);

                    let call_path = self.create_call_path(new_path.clone()).await?;
                    indirect_paths.push(call_path);

                    queue.push_back((transitive_id, new_path, distance.saturating_add(1)));
                }
            }
        }

        let summary = format!(
            "Impact analysis for '{}': {} direct impacts, {} indirect impacts",
            target,
            direct_relationships.len(),
            indirect_paths.len()
        );

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::ImpactAnalysis {
                target: target.to_string(),
            },
            direct_relationships: direct_relationships.clone(),
            indirect_relationships: indirect_paths.clone(),
            stats: RelationshipStats {
                direct_count: direct_relationships.len(),
                indirect_count: indirect_paths.len(),
                symbols_analyzed: visited.len(),
                execution_time_ms: 0,
                truncated,
            },
            summary,
        })
    }

    /// Find call chain between two symbols
    async fn find_call_chain(&self, from: &str, to: &str) -> Result<RelationshipQueryResult> {
        let from_id = self
            .resolve_symbol_name(from)
            .context("Failed to resolve 'from' symbol")?;
        let to_id = self
            .resolve_symbol_name(to)
            .context("Failed to resolve 'to' symbol")?;

        let from_node = self
            .dependency_graph
            .symbol_to_node
            .get(&from_id)
            .context("From symbol not found in graph")?;
        let to_node = self
            .dependency_graph
            .symbol_to_node
            .get(&to_id)
            .context("To symbol not found in graph")?;

        // Use Dijkstra's algorithm to find shortest path
        let path_result = petgraph::algo::dijkstra(
            &self.dependency_graph.graph,
            *from_node,
            Some(*to_node),
            |_| 1, // All edges have weight 1
        );

        let mut indirect_paths = Vec::new();
        let summary = if let Some(&distance) = path_result.get(to_node) {
            // Reconstruct the path
            let path = self.reconstruct_path(*from_node, *to_node, &path_result)?;
            let symbol_ids: Vec<Uuid> = path
                .iter()
                .map(|&node_idx| self.dependency_graph.graph[node_idx].symbol_id)
                .collect();

            let call_path = self.create_call_path(symbol_ids).await?;
            indirect_paths.push(call_path);

            format!(
                "Found call chain from '{}' to '{}' with distance {}",
                from, to, distance
            )
        } else {
            format!("No call chain found from '{}' to '{}'", from, to)
        };

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::CallChain {
                from: from.to_string(),
                to: to.to_string(),
            },
            direct_relationships: vec![],
            indirect_relationships: indirect_paths.clone(),
            stats: RelationshipStats {
                direct_count: 0,
                indirect_count: indirect_paths.len(),
                symbols_analyzed: path_result.len(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary,
        })
    }

    /// Find circular dependencies
    async fn find_circular_dependencies(
        &self,
        target: Option<&str>,
    ) -> Result<RelationshipQueryResult> {
        let cycles = self.dependency_graph.find_circular_dependencies();
        let mut indirect_paths = Vec::new();

        let filtered_cycles = if let Some(target_name) = target {
            let target_id = self
                .resolve_symbol_name(target_name)
                .context("Failed to resolve target symbol")?;

            cycles
                .into_iter()
                .filter(|cycle| cycle.contains(&target_id))
                .collect()
        } else {
            cycles
        };

        for cycle in filtered_cycles {
            let call_path = self.create_call_path(cycle).await?;
            indirect_paths.push(call_path);
        }

        let summary = if let Some(target_name) = target {
            format!(
                "Found {} circular dependencies involving '{}'",
                indirect_paths.len(),
                target_name
            )
        } else {
            format!(
                "Found {} circular dependencies in the codebase",
                indirect_paths.len()
            )
        };

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::CircularDependencies {
                target: target.map(String::from),
            },
            direct_relationships: vec![],
            indirect_relationships: indirect_paths.clone(),
            stats: RelationshipStats {
                direct_count: 0,
                indirect_count: indirect_paths.len(),
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary,
        })
    }

    /// Find unused symbols (symbols with no incoming dependencies)
    async fn find_unused_symbols(
        &self,
        symbol_type_filter: Option<&SymbolType>,
    ) -> Result<RelationshipQueryResult> {
        let mut unused_relationships = Vec::new();

        for node_idx in self.dependency_graph.graph.node_indices() {
            let node = &self.dependency_graph.graph[node_idx];

            // Check if symbol type matches filter
            if let Some(filter_type) = symbol_type_filter {
                if &node.symbol_type != filter_type {
                    continue;
                }
            }

            // Check if node has no incoming edges
            if self
                .dependency_graph
                .graph
                .edges_directed(node_idx, Direction::Incoming)
                .count()
                == 0
            {
                // Create a relationship match for the unused symbol
                let location = RelationshipLocation {
                    line_number: 0, // We don't have this info readily available
                    column_number: 0,
                    file_path: node.file_path.display().to_string(),
                };

                let relationship = RelationshipMatch {
                    symbol_id: node.symbol_id,
                    symbol_name: node
                        .qualified_name
                        .split("::")
                        .last()
                        .unwrap_or(&node.qualified_name)
                        .to_string(),
                    qualified_name: node.qualified_name.clone(),
                    symbol_type: node.symbol_type.clone(),
                    file_path: node.file_path.display().to_string(),
                    relation_type: RelationType::Custom("unused".to_string()),
                    location,
                    context: "Symbol has no incoming dependencies".to_string(),
                };

                unused_relationships.push(relationship);
            }
        }

        let summary = if let Some(symbol_type) = symbol_type_filter {
            format!(
                "Found {} unused {:?} symbols",
                unused_relationships.len(),
                symbol_type
            )
        } else {
            format!("Found {} unused symbols", unused_relationships.len())
        };

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::UnusedSymbols {
                symbol_type: symbol_type_filter.cloned(),
            },
            direct_relationships: unused_relationships.clone(),
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: unused_relationships.len(),
                indirect_count: 0,
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary,
        })
    }

    /// Find hot paths (most frequently called symbols)
    async fn find_hot_paths(&self, limit: usize) -> Result<RelationshipQueryResult> {
        let mut symbol_degrees: Vec<(NodeIndex, usize)> = self
            .dependency_graph
            .graph
            .node_indices()
            .map(|node_idx| {
                let in_degree = self
                    .dependency_graph
                    .graph
                    .edges_directed(node_idx, Direction::Incoming)
                    .count();
                (node_idx, in_degree)
            })
            .collect();

        // Sort by in-degree (most called first)
        symbol_degrees.sort_by(|a, b| b.1.cmp(&a.1));

        let mut hot_relationships = Vec::new();
        for (node_idx, in_degree) in symbol_degrees.into_iter().take(limit) {
            if in_degree == 0 {
                break; // No more symbols with incoming dependencies
            }

            let node = &self.dependency_graph.graph[node_idx];
            let location = RelationshipLocation {
                line_number: 0,
                column_number: 0,
                file_path: node.file_path.display().to_string(),
            };

            let relationship = RelationshipMatch {
                symbol_id: node.symbol_id,
                symbol_name: node
                    .qualified_name
                    .split("::")
                    .last()
                    .unwrap_or(&node.qualified_name)
                    .to_string(),
                qualified_name: node.qualified_name.clone(),
                symbol_type: node.symbol_type.clone(),
                file_path: node.file_path.display().to_string(),
                relation_type: RelationType::Custom("hot_path".to_string()),
                location,
                context: format!("Called by {} other symbols", in_degree),
            };

            hot_relationships.push(relationship);
        }

        let summary = format!(
            "Found {} hottest symbols (most frequently called)",
            hot_relationships.len()
        );

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::HotPaths { limit: Some(limit) },
            direct_relationships: hot_relationships.clone(),
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: hot_relationships.len(),
                indirect_count: 0,
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0,
                truncated: hot_relationships.len() == limit,
            },
            summary,
        })
    }

    /// Find dependencies of a specific type
    async fn find_dependencies_by_type(
        &self,
        target: &str,
        relation_type: &RelationType,
    ) -> Result<RelationshipQueryResult> {
        let symbol_id = self
            .resolve_symbol_name(target)
            .context("Failed to resolve target symbol")?;

        let all_dependencies = self.dependency_graph.find_dependencies(symbol_id);
        let mut filtered_relationships = Vec::new();

        for (dependency_id, dep_relation_type) in all_dependencies {
            if &dep_relation_type == relation_type {
                if let Some(relationship) = self
                    .create_relationship_match(symbol_id, dep_relation_type, &dependency_id)
                    .await?
                {
                    filtered_relationships.push(relationship);
                }
            }
        }

        let summary = format!(
            "Found {} {:?} dependencies for '{}'",
            filtered_relationships.len(),
            relation_type,
            target
        );

        Ok(RelationshipQueryResult {
            query_type: RelationshipQueryType::DependenciesByType {
                target: target.to_string(),
                relation_type: relation_type.clone(),
            },
            direct_relationships: filtered_relationships.clone(),
            indirect_relationships: vec![],
            stats: RelationshipStats {
                direct_count: filtered_relationships.len(),
                indirect_count: 0,
                symbols_analyzed: self.dependency_graph.graph.node_count(),
                execution_time_ms: 0,
                truncated: false,
            },
            summary,
        })
    }

    /// Resolve a symbol name to its UUID using advanced fuzzy matching
    fn resolve_symbol_name(&self, name: &str) -> Result<Uuid> {
        // Try direct lookup first
        if let Some(&id) = self.dependency_graph.name_to_symbol.get(name) {
            return Ok(id);
        }

        // Try exact suffix matching (highest priority)
        for (qualified_name, &id) in &self.dependency_graph.name_to_symbol {
            if qualified_name.ends_with(&format!("::{}", name)) {
                return Ok(id);
            }
        }

        // Try prefix matching for qualified names
        for (qualified_name, &id) in &self.dependency_graph.name_to_symbol {
            if qualified_name.contains(&format!("{}::", name)) {
                return Ok(id);
            }
        }

        // Try case-insensitive matching
        let name_lower = name.to_lowercase();
        for (qualified_name, &id) in &self.dependency_graph.name_to_symbol {
            if qualified_name.to_lowercase() == name_lower {
                return Ok(id);
            }
        }

        // Try substring matching with ranking by length similarity
        let mut candidates = Vec::new();
        for (qualified_name, &id) in &self.dependency_graph.name_to_symbol {
            let simple_name = qualified_name.split("::").last().unwrap_or(qualified_name);
            if simple_name.to_lowercase().contains(&name_lower)
                || name_lower.contains(&simple_name.to_lowercase())
            {
                let score = self.calculate_similarity_score(name, simple_name);
                candidates.push((id, score, qualified_name.clone()));
            }
        }

        // Sort by similarity score (higher is better)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((id, score, qualified_name)) = candidates.first() {
            if *score > 0.3 {
                // Minimum similarity threshold
                debug!(
                    "Fuzzy matched '{}' to '{}' with score {:.2}",
                    name, qualified_name, score
                );
                return Ok(*id);
            }
        }

        anyhow::bail!(
            "Symbol '{}' not found in dependency graph (tried {} candidates)",
            name,
            candidates.len()
        )
    }

    /// Calculate similarity score between two strings using a combination of metrics
    fn calculate_similarity_score(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }

        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if a_lower == b_lower {
            return 0.95;
        }

        // Jaro-Winkler-like similarity
        let max_len = a.len().max(b.len()) as f32;
        if max_len == 0.0 {
            return 0.0;
        }

        // Count common characters
        let mut common = 0;
        let min_len = a.len().min(b.len());
        for i in 0..min_len {
            if a_lower.chars().nth(i) == b_lower.chars().nth(i) {
                common += 1;
            } else {
                break; // Only count prefix matches for this simple version
            }
        }

        // Length-normalized similarity with bonus for prefix matches
        let prefix_bonus = if common > 0 { 0.1 } else { 0.0 };
        let base_similarity = common as f32 / max_len;

        // Substring bonus
        let substring_bonus = if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            0.2
        } else {
            0.0
        };

        (base_similarity + prefix_bonus + substring_bonus).min(1.0)
    }

    /// Create a relationship match from symbol IDs and metadata
    async fn create_relationship_match(
        &self,
        source_id: Uuid,
        relation_type: RelationType,
        target_id: &Uuid,
    ) -> Result<Option<RelationshipMatch>> {
        let source_node_idx = self.dependency_graph.symbol_to_node.get(&source_id);
        if source_node_idx.is_none() {
            warn!("Source symbol {} not found in graph", source_id);
            return Ok(None);
        }

        let source_node = &self.dependency_graph.graph[*source_node_idx.unwrap()];

        // Find the specific edge to get location information
        let mut edge_context = "No context available".to_string();
        let mut line_number = 0;
        let mut column_number = 0;

        if let Some(target_node_idx) = self.dependency_graph.symbol_to_node.get(target_id) {
            for edge in self.dependency_graph.graph.edges(*source_node_idx.unwrap()) {
                if edge.target() == *target_node_idx {
                    let edge_data = edge.weight();
                    line_number = edge_data.line_number;
                    column_number = edge_data.column_number;
                    if let Some(ctx) = &edge_data.context {
                        edge_context = ctx.clone();
                    }
                    break;
                }
            }
        }

        let location = RelationshipLocation {
            line_number,
            column_number,
            file_path: source_node.file_path.display().to_string(),
        };

        Ok(Some(RelationshipMatch {
            symbol_id: source_id,
            symbol_name: source_node
                .qualified_name
                .split("::")
                .last()
                .unwrap_or(&source_node.qualified_name)
                .to_string(),
            qualified_name: source_node.qualified_name.clone(),
            symbol_type: source_node.symbol_type.clone(),
            file_path: source_node.file_path.display().to_string(),
            relation_type,
            location,
            context: edge_context,
        }))
    }

    /// Create a call path from a sequence of symbol IDs
    async fn create_call_path(&self, symbol_ids: Vec<Uuid>) -> Result<CallPath> {
        let mut symbol_names = Vec::new();

        for &symbol_id in &symbol_ids {
            if let Some(&node_idx) = self.dependency_graph.symbol_to_node.get(&symbol_id) {
                let node = &self.dependency_graph.graph[node_idx];
                symbol_names.push(node.qualified_name.clone());
            } else {
                symbol_names.push(format!("Unknown({})", symbol_id));
            }
        }

        let description = if symbol_names.len() >= 2 {
            format!(
                "{} → {}",
                symbol_names.first().unwrap(),
                symbol_names.last().unwrap()
            )
        } else {
            symbol_names.join(" → ")
        };

        Ok(CallPath {
            path: symbol_ids,
            symbol_names: symbol_names.clone(),
            distance: symbol_names.len().saturating_sub(1),
            description,
        })
    }

    /// Reconstruct path from Dijkstra result
    fn reconstruct_path(
        &self,
        start: NodeIndex,
        end: NodeIndex,
        distances: &HashMap<NodeIndex, usize>,
    ) -> Result<Vec<NodeIndex>> {
        let mut path = vec![end];
        let mut current = end;

        while current != start {
            let mut found_predecessor = false;

            for edge in self
                .dependency_graph
                .graph
                .edges_directed(current, Direction::Incoming)
            {
                let pred = edge.source();
                if let (Some(&pred_dist), Some(&curr_dist)) =
                    (distances.get(&pred), distances.get(&current))
                {
                    // Use saturating arithmetic to prevent overflow
                    if pred_dist.saturating_add(1) == curr_dist {
                        path.push(pred);
                        current = pred;
                        found_predecessor = true;
                        break;
                    }
                }
            }

            if !found_predecessor {
                anyhow::bail!("Failed to reconstruct path");
            }
        }

        path.reverse();
        Ok(path)
    }
}

/// Parse natural language relationship queries
pub fn parse_natural_language_relationship_query(query: &str) -> Option<RelationshipQueryType> {
    let query_lower = query.to_lowercase();

    // Handle caller-finding patterns
    if query_lower.contains("what calls") || query_lower.contains("who calls") {
        if let Some(target) = extract_target_from_query(query, "calls") {
            return Some(RelationshipQueryType::FindCallers { target });
        }
    }

    if query_lower.contains("who uses") || query_lower.contains("what uses") {
        if let Some(target) = extract_target_from_query(query, "uses") {
            return Some(RelationshipQueryType::FindCallers { target });
        }
    }

    // Handle "creates" patterns - find what creates/instantiates something
    if query_lower.contains("what creates")
        || query_lower.contains("who creates")
        || query_lower.contains("how is") && query_lower.contains("created")
        || query_lower.contains("what instantiates")
    {
        if let Some(target) = extract_target_from_creates_query(query) {
            return Some(RelationshipQueryType::FindCallers { target });
        }
    }

    if query_lower.contains("find callers of") {
        // Extract target specifically after "find callers of" to avoid matching other "of"s
        if let Some(pos) = query_lower.find("find callers of") {
            let after_phrase = &query[pos + "find callers of".len()..].trim();
            if let Some(target) = after_phrase.split_whitespace().next() {
                let cleaned =
                    target.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
                if !cleaned.is_empty() {
                    return Some(RelationshipQueryType::FindCallers {
                        target: cleaned.to_string(),
                    });
                }
            }
        }
    }

    if query_lower.contains("what does") && query_lower.contains("call") {
        if let Some(target) = extract_target_from_query(query, "does") {
            return Some(RelationshipQueryType::FindCallees { target });
        }
    }

    // Enhanced impact analysis patterns - more flexible
    if query_lower.contains("what would break")
        || query_lower.contains("what breaks")
        || query_lower.contains("impact")
        || query_lower.contains("what would be affected")
        || query_lower.contains("what depends on")
    {
        // Try multiple extraction strategies for impact analysis
        if let Some(target) = extract_target_from_impact_query(query) {
            return Some(RelationshipQueryType::ImpactAnalysis { target });
        }

        // Fallback to original patterns
        if let Some(target) = extract_target_from_query(query, "change") {
            return Some(RelationshipQueryType::ImpactAnalysis { target });
        } else if let Some(target) = extract_target_from_query(query, "break") {
            return Some(RelationshipQueryType::ImpactAnalysis { target });
        }
    }

    if query_lower.contains("call chain") || query_lower.contains("path from") {
        if let (Some(from), Some(to)) = extract_from_to_from_query(query) {
            return Some(RelationshipQueryType::CallChain { from, to });
        }
    }

    if query_lower.contains("circular") || query_lower.contains("cycle") {
        let target = extract_target_from_query(query, "circular")
            .or_else(|| extract_target_from_query(query, "cycle"));
        return Some(RelationshipQueryType::CircularDependencies { target });
    }

    if query_lower.contains("unused") || query_lower.contains("dead code") {
        // Try to extract symbol type from query
        let symbol_type = if query_lower.contains("function") {
            Some(SymbolType::Function)
        } else if query_lower.contains("struct") {
            Some(SymbolType::Struct)
        } else if query_lower.contains("class") {
            Some(SymbolType::Class)
        } else {
            None
        };
        return Some(RelationshipQueryType::UnusedSymbols { symbol_type });
    }

    if query_lower.contains("hot") || query_lower.contains("most called") {
        let limit = extract_number_from_query(query).unwrap_or(10);
        return Some(RelationshipQueryType::HotPaths { limit: Some(limit) });
    }

    None
}

/// Validates if a symbol name is properly formatted
/// Supports simple identifiers (foo, _bar) and qualified names (Foo::Bar, std::vec::Vec)
fn is_valid_symbol_name(symbol: &str) -> bool {
    if symbol.is_empty() {
        return false;
    }

    // Handle qualified names (contains ::)
    if symbol.contains("::") {
        return symbol
            .split("::")
            .all(|part| !part.is_empty() && is_valid_simple_identifier(part));
    }

    // Handle simple identifiers
    is_valid_simple_identifier(symbol)
}

/// Validates a simple identifier (no :: separators)
fn is_valid_simple_identifier(identifier: &str) -> bool {
    if identifier.is_empty() {
        return false;
    }

    // Must start with alphabetic or underscore
    let mut chars = identifier.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Extract target symbol name from query with enhanced validation
fn extract_target_from_query(query: &str, keyword: &str) -> Option<String> {
    let query_lower = query.to_lowercase();
    if let Some(pos) = query_lower.find(keyword) {
        let after_keyword = &query[pos + keyword.len()..];
        let words: Vec<&str> = after_keyword.split_whitespace().collect();

        // Look for the next meaningful word that could be a symbol name
        for word in words {
            let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
            if !cleaned.is_empty()
                && cleaned.len() > 1
                && !cleaned.eq_ignore_ascii_case("this")
                && is_valid_symbol_name(cleaned)
            {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Enhanced extraction for impact analysis queries that handle various phrasings
fn extract_target_from_impact_query(query: &str) -> Option<String> {
    let query_lower = query.to_lowercase();

    // Pattern: "what would break if I change X?" or "If I change X, what would break?"
    if query_lower.contains("if i change") || query_lower.contains("if you change") {
        if let Some(pos) = query_lower.find("change") {
            let after_change = &query[pos + "change".len()..];
            // Look for symbol after "change"
            let words: Vec<&str> = after_change.split_whitespace().collect();
            for word in words {
                let cleaned =
                    word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
                if !cleaned.is_empty() && cleaned.len() > 1 && is_valid_symbol_name(cleaned) {
                    return Some(cleaned.to_string());
                }
            }
        }
    }

    // Pattern: "what depends on X?" or "what would be affected by X?"
    if query_lower.contains("depends on") {
        return extract_target_from_query(query, "depends on");
    }

    if query_lower.contains("affected by") {
        return extract_target_from_query(query, "affected by");
    }

    // Pattern: "X impact" or "impact of X"
    if query_lower.contains("impact of") {
        return extract_target_from_query(query, "impact of");
    }

    None
}

/// Constructor mapping for generic terms to specific function patterns
/// This allows "what creates storage?" to map to "create_storage" etc.
static CONSTRUCTOR_MAPPINGS: &[(&str, &str)] = &[
    ("storage", "create_storage"),
    ("config", "create_config"),
    ("index", "create_index"),
    ("connection", "create_connection"),
    ("pool", "create_pool"),
    ("client", "create_client"),
    ("server", "create_server"),
    ("engine", "create_engine"),
    ("builder", "create_builder"),
];

/// Maps generic terms to constructor function patterns
fn map_generic_to_constructor(generic_term: &str) -> String {
    let term_lower = generic_term.to_lowercase();

    // Check predefined mappings
    for &(generic, constructor) in CONSTRUCTOR_MAPPINGS {
        if term_lower == generic {
            return constructor.to_string();
        }
    }

    // Fallback: if no specific mapping exists, try common patterns
    if term_lower.ends_with("s") {
        // Handle plurals: "storages" -> "create_storage"
        let singular = &term_lower[..term_lower.len() - 1];
        for &(generic, constructor) in CONSTRUCTOR_MAPPINGS {
            if singular == generic {
                return constructor.to_string();
            }
        }
    }

    // Default: prepend "create_" to the term
    format!("create_{}", term_lower)
}

/// Extract target for "creates" queries like "what creates storage?"
fn extract_target_from_creates_query(query: &str) -> Option<String> {
    let query_lower = query.to_lowercase();

    // Pattern: "what creates X?" - look for word after "creates"
    if query_lower.contains("creates") {
        if let Some(target) = extract_target_from_query(query, "creates") {
            // Check if this is a generic term that should be mapped to constructor patterns
            let target_lower = target.to_lowercase();

            // If it's already a specific symbol name (contains uppercase, underscores, or ::), use as-is
            if target.chars().any(|c| c.is_uppercase())
                || target.contains('_')
                || target.contains("::")
                || target.chars().all(|c| c.is_uppercase())
            {
                // All caps like "HTTP"
                return Some(target);
            }

            // Otherwise, treat as generic term and map to constructor
            return Some(map_generic_to_constructor(&target_lower));
        }
    }

    // Pattern: "how is X created?" - look for word before "created"
    if query_lower.contains("created") {
        if let Some(pos) = query_lower.find("created") {
            let before_created = &query[..pos];
            let words: Vec<&str> = before_created.split_whitespace().collect();
            // Look for the last meaningful word before "created"
            for word in words.iter().rev() {
                let cleaned =
                    word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
                if !cleaned.is_empty()
                    && cleaned.len() > 1
                    && !cleaned.eq_ignore_ascii_case("is")
                    && !cleaned.eq_ignore_ascii_case("how")
                    && is_valid_symbol_name(cleaned)
                {
                    return Some(cleaned.to_string());
                }
            }
        }
    }

    // Pattern: "what instantiates X?"
    if query_lower.contains("instantiates") {
        return extract_target_from_query(query, "instantiates");
    }

    None
}

/// Extract from and to symbols for call chain queries
fn extract_from_to_from_query(query: &str) -> (Option<String>, Option<String>) {
    let query_lower = query.to_lowercase();

    // Pattern: "path from X to Y" or "call chain from X to Y"
    if let Some(from_pos) = query_lower.find("from") {
        if let Some(to_pos) = query_lower.find("to") {
            if to_pos > from_pos {
                let from_part = &query[from_pos + 4..to_pos].trim();
                let to_part = &query[to_pos + 2..].trim();

                let from = from_part
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':'))
                    .filter(|s| !s.is_empty())
                    .map(String::from);

                let to = to_part
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':'))
                    .filter(|s| !s.is_empty())
                    .map(String::from);

                return (from, to);
            }
        }
    }

    (None, None)
}

/// Extract a number from query
fn extract_number_from_query(query: &str) -> Option<usize> {
    for word in query.split_whitespace() {
        if let Ok(num) = word.parse::<usize>() {
            return Some(num);
        }
    }
    None
}

impl RelationshipQueryResult {
    /// Limit the number of results returned
    pub fn limit_results(&mut self, limit: usize) {
        // Truncate direct relationships if they exceed the limit
        if self.direct_relationships.len() > limit {
            self.direct_relationships.truncate(limit);
            // Update stats to reflect the truncation
            self.stats.direct_count = limit;
        }

        // Truncate indirect relationships/call paths if they exceed the limit
        if self.indirect_relationships.len() > limit {
            self.indirect_relationships.truncate(limit);
            // Update stats to reflect the truncation
            self.stats.indirect_count = limit;
        }

        // Update summary to indicate results were limited
        if self.stats.direct_count == limit || self.stats.indirect_count == limit {
            self.summary = format!("{} (limited to {} results)", self.summary, limit);
        }
    }

    /// Format the result as markdown for LLM consumption
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", self.summary));

        if !self.direct_relationships.is_empty() {
            output.push_str("## Direct Relationships\n\n");
            for (i, rel) in self.direct_relationships.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **{}** (`{:?}` in `{}`)\n",
                    i + 1,
                    rel.symbol_name,
                    rel.symbol_type,
                    rel.file_path
                        .split('/')
                        .next_back()
                        .unwrap_or(&rel.file_path)
                ));
                output.push_str(&format!(
                    "   - **Qualified Name:** `{}`\n",
                    rel.qualified_name
                ));
                output.push_str(&format!("   - **Relationship:** {:?}\n", rel.relation_type));
                output.push_str(&format!(
                    "   - **Location:** {}:{}\n",
                    rel.location.line_number, rel.location.column_number
                ));
                if !rel.context.is_empty() {
                    output.push_str(&format!("   - **Context:** {}\n", rel.context));
                }
                output.push('\n');
            }
        }

        if !self.indirect_relationships.is_empty() {
            output.push_str("## Indirect Relationships (Call Paths)\n\n");
            for (i, path) in self.indirect_relationships.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **Path (distance: {}):** {}\n",
                    i + 1,
                    path.distance,
                    path.description
                ));
                output.push_str(&format!(
                    "   - **Full Path:** {}\n",
                    path.symbol_names.join(" → ")
                ));
                output.push('\n');
            }
        }

        output.push_str("## Query Statistics\n\n");
        output.push_str(&format!(
            "- **Direct Relationships:** {}\n",
            self.stats.direct_count
        ));
        output.push_str(&format!(
            "- **Indirect Relationships:** {}\n",
            self.stats.indirect_count
        ));
        output.push_str(&format!(
            "- **Symbols Analyzed:** {}\n",
            self.stats.symbols_analyzed
        ));
        output.push_str(&format!(
            "- **Execution Time:** {}ms\n",
            self.stats.execution_time_ms
        ));
        if self.stats.truncated {
            output.push_str("- **Note:** Results were truncated\n");
        }

        output
    }

    /// Format as JSON for programmatic consumption
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize relationship query result to JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_natural_language_callers_query() {
        let query_type =
            parse_natural_language_relationship_query("what calls FileStorage::insert");
        assert!(matches!(
            query_type,
            Some(RelationshipQueryType::FindCallers { .. })
        ));

        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(target, "FileStorage::insert");
        }
    }

    #[test]
    fn test_parse_natural_language_impact_query() {
        let query_type =
            parse_natural_language_relationship_query("what would break if I change StorageError");
        assert!(matches!(
            query_type,
            Some(RelationshipQueryType::ImpactAnalysis { .. })
        ));

        if let Some(RelationshipQueryType::ImpactAnalysis { target }) = query_type {
            assert_eq!(target, "StorageError");
        }
    }

    #[test]
    fn test_parse_call_chain_query() {
        let query_type =
            parse_natural_language_relationship_query("show call chain from main to handle_error");
        assert!(matches!(
            query_type,
            Some(RelationshipQueryType::CallChain { .. })
        ));

        if let Some(RelationshipQueryType::CallChain { from, to }) = query_type {
            assert_eq!(from, "main");
            assert_eq!(to, "handle_error");
        }
    }

    #[test]
    fn test_parse_unused_symbols_query() {
        let query_type = parse_natural_language_relationship_query("find unused functions");
        assert!(matches!(
            query_type,
            Some(RelationshipQueryType::UnusedSymbols { .. })
        ));

        if let Some(RelationshipQueryType::UnusedSymbols { symbol_type }) = query_type {
            assert_eq!(symbol_type, Some(SymbolType::Function));
        }
    }

    #[test]
    fn test_parse_who_uses_query() {
        // Test "who uses" pattern - should find callers
        let query_type = parse_natural_language_relationship_query("who uses FileStorage?");
        assert!(
            matches!(query_type, Some(RelationshipQueryType::FindCallers { .. })),
            "Query 'who uses FileStorage?' should be parsed as FindCallers"
        );

        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(target, "FileStorage");
        }
    }

    #[test]
    fn test_parse_what_uses_query() {
        // Test "what uses" pattern - should also find callers
        let query_type = parse_natural_language_relationship_query("what uses Config::new");
        assert!(
            matches!(query_type, Some(RelationshipQueryType::FindCallers { .. })),
            "Query 'what uses Config::new' should be parsed as FindCallers"
        );

        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(target, "Config::new");
        }
    }

    #[test]
    fn test_parse_find_callers_of_query() {
        // Test "find callers of" pattern
        let query_type =
            parse_natural_language_relationship_query("find callers of Storage::insert");
        assert!(
            matches!(query_type, Some(RelationshipQueryType::FindCallers { .. })),
            "Query 'find callers of Storage::insert' should be parsed as FindCallers"
        );

        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(target, "Storage::insert");
        }
    }

    #[test]
    fn test_issue_431_parsing_failures() {
        // Test cases from Issue #431 to verify specific parsing problems

        // ✅ This should work according to issue
        let query_type =
            parse_natural_language_relationship_query("what calls create_file_storage?");
        assert!(
            matches!(query_type, Some(RelationshipQueryType::FindCallers { .. })),
            "Query 'what calls create_file_storage?' should be parsed as FindCallers"
        );

        // ❌ This fails according to issue - should parse as ImpactAnalysis
        let query_type =
            parse_natural_language_relationship_query("what would break if I change FileStorage?");
        assert!(
            matches!(query_type, Some(RelationshipQueryType::ImpactAnalysis { .. })),
            "Query 'what would break if I change FileStorage?' should be parsed as ImpactAnalysis but currently fails"
        );

        // ❌ This fails according to issue - alternative phrasing
        let query_type =
            parse_natural_language_relationship_query("If I change FileStorage, what would break?");
        assert!(
            matches!(
                query_type,
                Some(RelationshipQueryType::ImpactAnalysis { .. })
            ),
            "Query 'If I change FileStorage, what would break?' should be parsed as ImpactAnalysis"
        );

        // ❌ This fails according to issue - should find functions that create storage
        let query_type = parse_natural_language_relationship_query("what creates storage?");
        // This could be interpreted as finding callers of create* functions or impact analysis
        // For now, let's expect it to at least parse to something reasonable
        assert!(
            query_type.is_some(),
            "Query 'what creates storage?' should parse to some valid query type"
        );

        // Test case from existing test that should work
        let query_type =
            parse_natural_language_relationship_query("what would break if I change StorageError");
        assert!(
            matches!(
                query_type,
                Some(RelationshipQueryType::ImpactAnalysis { .. })
            ),
            "Query 'what would break if I change StorageError' should work (from existing test)"
        );
    }

    #[test]
    fn test_enhanced_natural_language_patterns() {
        // Test comprehensive natural language patterns added in Issue #431 fix

        // Enhanced impact analysis patterns
        let patterns = vec![
            ("what would break if I change Config?", "ImpactAnalysis"),
            ("If I change Database, what would break?", "ImpactAnalysis"),
            ("what breaks if I modify Parser?", "ImpactAnalysis"),
            ("what would be affected by Server?", "ImpactAnalysis"),
            ("what depends on Utils?", "ImpactAnalysis"),
            ("impact of changing Router", "ImpactAnalysis"),
        ];

        for (query, expected) in &patterns {
            let query_type = parse_natural_language_relationship_query(query);
            assert!(
                matches!(
                    query_type,
                    Some(RelationshipQueryType::ImpactAnalysis { .. })
                ),
                "Query '{}' should be parsed as {} but got {:?}",
                query,
                expected,
                query_type
            );
        }

        // Enhanced "creates" patterns
        let create_patterns = vec![
            ("what creates Config?", "FindCallers"),
            ("who creates Storage?", "FindCallers"),
            ("how is Database created?", "FindCallers"),
            ("what instantiates Parser?", "FindCallers"),
        ];

        for (query, expected) in &create_patterns {
            let query_type = parse_natural_language_relationship_query(query);
            assert!(
                matches!(query_type, Some(RelationshipQueryType::FindCallers { .. })),
                "Query '{}' should be parsed as {} but got {:?}",
                query,
                expected,
                query_type
            );
        }

        // Test that "what creates storage?" maps to "create_storage"
        let query_type = parse_natural_language_relationship_query("what creates storage?");
        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(
                target, "create_storage",
                "Generic 'storage' should map to 'create_storage' pattern"
            );
        } else {
            panic!("Query 'what creates storage?' should parse as FindCallers");
        }
    }

    #[test]
    fn test_symbol_validation_edge_cases() {
        // Test enhanced symbol validation with various edge cases

        // Valid simple identifiers
        assert!(is_valid_symbol_name("foo"));
        assert!(is_valid_symbol_name("_bar"));
        assert!(is_valid_symbol_name("foo123"));
        assert!(is_valid_symbol_name("FileStorage"));
        assert!(is_valid_symbol_name("HTTP_CLIENT"));

        // Valid qualified names
        assert!(is_valid_symbol_name("std::vec::Vec"));
        assert!(is_valid_symbol_name("MyModule::Config"));
        assert!(is_valid_symbol_name("foo::bar::baz"));

        // Invalid cases
        assert!(!is_valid_symbol_name(""));
        assert!(!is_valid_symbol_name("123invalid"));
        assert!(!is_valid_symbol_name(":invalid::"));
        assert!(!is_valid_symbol_name("::invalid"));
        assert!(!is_valid_symbol_name("invalid::"));
        assert!(!is_valid_symbol_name("foo::::bar"));
        assert!(!is_valid_symbol_name("foo..bar"));
        assert!(!is_valid_symbol_name("foo-bar"));
    }

    #[test]
    fn test_constructor_mapping_system() {
        // Test the extensible constructor mapping system

        // Predefined mappings
        assert_eq!(map_generic_to_constructor("storage"), "create_storage");
        assert_eq!(map_generic_to_constructor("config"), "create_config");
        assert_eq!(map_generic_to_constructor("engine"), "create_engine");

        // Case insensitive
        assert_eq!(map_generic_to_constructor("Storage"), "create_storage");
        assert_eq!(map_generic_to_constructor("CONFIG"), "create_config");

        // Plural handling
        assert_eq!(map_generic_to_constructor("storages"), "create_storage");
        assert_eq!(map_generic_to_constructor("configs"), "create_config");

        // Unknown terms get default pattern
        assert_eq!(map_generic_to_constructor("unknown"), "create_unknown");
        assert_eq!(map_generic_to_constructor("database"), "create_database");
    }

    #[test]
    fn test_creates_query_with_specific_symbols() {
        // Test that specific symbol names are preserved, not mapped

        // Specific class names (contain uppercase) - should use as-is
        let query_type = parse_natural_language_relationship_query("what creates FileStorage?");
        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(
                target, "FileStorage",
                "Specific class names should not be mapped"
            );
        } else {
            panic!("Should parse as FindCallers");
        }

        // Function names with underscores - should use as-is
        let query_type =
            parse_natural_language_relationship_query("what creates create_file_storage?");
        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(
                target, "create_file_storage",
                "Function names should not be mapped"
            );
        } else {
            panic!("Should parse as FindCallers");
        }

        // Qualified names - should use as-is
        let query_type = parse_natural_language_relationship_query("what creates std::vec::Vec?");
        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            assert_eq!(
                target, "std::vec::Vec",
                "Qualified names should not be mapped"
            );
        } else {
            panic!("Should parse as FindCallers");
        }
    }

    #[test]
    fn test_robustness_edge_cases() {
        // Test parser robustness with various edge cases

        // Multiple targets in query (should parse first valid one)
        let query_type = parse_natural_language_relationship_query(
            "what would break if I change Config and Database?",
        );
        assert!(
            matches!(
                query_type,
                Some(RelationshipQueryType::ImpactAnalysis { .. })
            ),
            "Should parse first valid target from multi-target query"
        );

        // Very long queries should still work
        let long_query = "what would break if I change FileStorage and it affects everything in the system and causes issues?";
        let query_type = parse_natural_language_relationship_query(long_query);
        assert!(
            matches!(
                query_type,
                Some(RelationshipQueryType::ImpactAnalysis { .. })
            ),
            "Should handle long queries gracefully"
        );

        // Unicode in queries (should be handled properly)
        let unicode_query = "what creates FileStorage?"; // Contains smart quotes
        let query_type = parse_natural_language_relationship_query(unicode_query);
        assert!(
            query_type.is_some(),
            "Should handle unicode characters gracefully"
        );

        // Empty and whitespace queries
        assert!(parse_natural_language_relationship_query("").is_none());
        assert!(parse_natural_language_relationship_query("   ").is_none());
        assert!(parse_natural_language_relationship_query("what").is_none());

        // Malformed symbol names should be rejected
        let malformed_query = "what creates :invalid::?";
        let query_type = parse_natural_language_relationship_query(malformed_query);
        // Should either fail to parse or handle gracefully
        if let Some(RelationshipQueryType::FindCallers { target }) = query_type {
            // If it does parse, the target should be cleaned up
            assert!(
                is_valid_symbol_name(&target),
                "Malformed symbols should be cleaned or rejected"
            );
        }
    }

    #[test]
    fn test_performance_considerations() {
        // Test performance with reasonable query sizes

        // Large number of repeated patterns - should not hang
        let repetitive_query = "what creates storage storage storage storage storage?";
        let start = std::time::Instant::now();
        let _result = parse_natural_language_relationship_query(repetitive_query);
        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Query parsing should be fast even with repetitive patterns"
        );

        // Very long symbol names - should handle gracefully
        let long_symbol = "a".repeat(1000);
        let long_query = format!("what creates {}?", long_symbol);
        let start = std::time::Instant::now();
        let _result = parse_natural_language_relationship_query(&long_query);
        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Should handle long symbol names efficiently"
        );
    }
}
