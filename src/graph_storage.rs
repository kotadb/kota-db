//! Graph-based storage backend for code intelligence features
//!
//! This module provides a specialized storage implementation optimized for
//! graph operations such as dependency tracking, call graphs, and impact analysis.
//! It complements the document-based FileStorage for a dual storage architecture.

use anyhow::Result;
use async_trait::async_trait;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::contracts::Storage;
use crate::symbol_storage::RelationType;

/// Extended storage trait for graph-specific operations
#[async_trait]
pub trait GraphStorage: Storage {
    /// Store a node (symbol) in the graph
    async fn store_node(&mut self, node_id: Uuid, node_data: GraphNode) -> Result<()>;

    /// Retrieve a node by ID
    async fn get_node(&self, node_id: Uuid) -> Result<Option<GraphNode>>;

    /// Store an edge (relationship) between two nodes
    async fn store_edge(&mut self, from: Uuid, to: Uuid, edge: GraphEdge) -> Result<()>;

    /// Get all edges for a node in a specific direction
    async fn get_edges(&self, node: Uuid, direction: Direction) -> Result<Vec<(Uuid, GraphEdge)>>;

    /// Get a subgraph starting from specified roots up to a certain depth
    async fn get_subgraph(&self, roots: &[Uuid], max_depth: usize) -> Result<GraphSubset>;

    /// Find all paths between two nodes (up to a maximum number)
    async fn find_paths(&self, from: Uuid, to: Uuid, max_paths: usize) -> Result<Vec<GraphPath>>;

    /// Get nodes by type (e.g., all functions, all classes)
    async fn get_nodes_by_type(&self, node_type: &str) -> Result<Vec<Uuid>>;

    /// Update edge metadata without recreating the edge
    /// Updates all edges between the two nodes with the given metadata
    async fn update_edge_metadata(
        &mut self,
        from: Uuid,
        to: Uuid,
        metadata: HashMap<String, String>,
    ) -> Result<()>;

    /// Update edge metadata for a specific relationship type
    async fn update_edge_metadata_by_type(
        &mut self,
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
        metadata: HashMap<String, String>,
    ) -> Result<()>;

    /// Remove all edges between two nodes
    async fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<bool>;

    /// Remove a specific edge by relationship type
    async fn remove_edge_by_type(
        &mut self,
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
    ) -> Result<bool>;

    /// Delete a node and all its edges
    async fn delete_node(&mut self, node_id: Uuid) -> Result<bool>;

    /// Get graph statistics
    async fn get_graph_stats(&self) -> Result<GraphStats>;

    /// Perform batch operations for efficiency
    async fn batch_insert_nodes(&mut self, nodes: Vec<(Uuid, GraphNode)>) -> Result<()>;
    async fn batch_insert_edges(&mut self, edges: Vec<(Uuid, Uuid, GraphEdge)>) -> Result<()>;
}

/// Node representation in the graph storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier
    pub id: Uuid,
    /// Node type (e.g., "function", "class", "module")
    pub node_type: String,
    /// Qualified name (e.g., "kotadb::storage::FileStorage")
    pub qualified_name: String,
    /// File path containing this node
    pub file_path: String,
    /// Line and column information
    pub location: NodeLocation,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp of last update
    pub updated_at: i64,
}

/// Location information for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLocation {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Edge representation in the graph storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Type of relationship
    pub relation_type: RelationType,
    /// Location where the relationship occurs
    pub location: NodeLocation,
    /// Context snippet
    pub context: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp of edge creation
    pub created_at: i64,
}

/// Subset of the graph returned by queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSubset {
    /// Nodes in the subset
    pub nodes: HashMap<Uuid, GraphNode>,
    /// Edges in the subset (from_id -> [(to_id, edge)])
    pub edges: HashMap<Uuid, Vec<(Uuid, GraphEdge)>>,
    /// Query metadata
    pub metadata: QueryMetadata,
}

/// Path through the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPath {
    /// Sequence of node IDs in the path
    pub nodes: Vec<Uuid>,
    /// Edges along the path
    pub edges: Vec<GraphEdge>,
    /// Total path length
    pub length: usize,
}

/// Query metadata for performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Number of nodes visited during query
    pub nodes_visited: usize,
    /// Number of edges traversed
    pub edges_traversed: usize,
    /// Query execution time in microseconds
    pub execution_time_us: u64,
    /// Whether the query was truncated due to limits
    pub truncated: bool,
}

/// Statistics about the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Node count by type
    pub nodes_by_type: HashMap<String, usize>,
    /// Edge count by relation type
    pub edges_by_type: HashMap<String, usize>,
    /// Average in-degree
    pub avg_in_degree: f64,
    /// Average out-degree
    pub avg_out_degree: f64,
    /// Number of connected components
    pub connected_components: usize,
    /// Storage size in bytes
    pub storage_size_bytes: u64,
}

/// Configuration for graph storage backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStorageConfig {
    /// Maximum number of nodes to keep in memory cache
    pub cache_size: usize,
    /// Enable write-ahead logging
    pub enable_wal: bool,
    /// Compression type for storage
    pub compression: CompressionType,
    /// Sync mode for durability
    pub sync_mode: SyncMode,
    /// Maximum depth for traversal queries
    pub max_traversal_depth: usize,
    /// Maximum number of paths to return in path queries
    pub max_path_results: usize,
}

impl Default for GraphStorageConfig {
    fn default() -> Self {
        Self {
            cache_size: 10_000,
            enable_wal: true,
            compression: CompressionType::Snappy,
            sync_mode: SyncMode::Normal,
            max_traversal_depth: 10,
            max_path_results: 1000,
        }
    }
}

/// Compression types for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Snappy,
    Zstd,
    Lz4,
}

/// Sync modes for durability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    /// Sync on every write (slowest, most durable)
    Full,
    /// Sync periodically (balanced)
    Normal,
    /// Minimal syncing (fastest, least durable)
    Fast,
}
