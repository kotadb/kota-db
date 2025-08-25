//! Hybrid storage router for dual storage architecture
//!
//! This module provides intelligent routing between document-based FileStorage
//! and graph-based NativeGraphStorage, optimizing for each use case while
//! maintaining a unified API.

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::contracts::{Document, Storage};
use crate::file_storage::FileStorage;
use crate::graph_storage::{GraphEdge, GraphNode, GraphStorage, GraphStorageConfig};
use crate::native_graph_storage::NativeGraphStorage;
use crate::symbol_storage::SymbolEntry;
use crate::types::ValidatedDocumentId;

/// Storage type identifier for routing decisions
#[derive(Debug, Clone, PartialEq)]
enum StorageType {
    Document,
    Graph,
    Both,
}

/// Hybrid storage configuration
#[derive(Debug, Clone)]
pub struct HybridStorageConfig {
    /// Enable graph storage for code intelligence
    pub enable_graph_storage: bool,
    /// Path patterns that should use graph storage
    pub graph_patterns: Vec<String>,
    /// Graph storage configuration
    pub graph_config: GraphStorageConfig,
    /// Cache size for routing decisions
    pub routing_cache_size: usize,
}

impl Default for HybridStorageConfig {
    fn default() -> Self {
        Self {
            enable_graph_storage: true,
            graph_patterns: vec![
                "/symbols/*".to_string(),
                "/relationships/*".to_string(),
                "/dependencies/*".to_string(),
            ],
            graph_config: GraphStorageConfig::default(),
            routing_cache_size: 1000,
        }
    }
}

/// Hybrid storage router implementation
pub struct HybridStorage {
    /// Document storage backend
    document_storage: Arc<RwLock<FileStorage>>,

    /// Graph storage backend (optional)
    graph_storage: Option<Arc<RwLock<NativeGraphStorage>>>,

    /// Configuration
    config: HybridStorageConfig,

    /// Routing cache for performance (using DashMap for concurrent access)
    routing_cache: Arc<DashMap<String, StorageType>>,

    /// Statistics
    stats: Arc<RwLock<HybridStats>>,
}

/// Statistics for hybrid storage operations
#[derive(Debug, Default)]
struct HybridStats {
    /// Number of operations routed to document storage
    document_ops: u64,
    /// Number of operations routed to graph storage
    graph_ops: u64,
    /// Number of operations that hit both storages
    hybrid_ops: u64,
    /// Cache hit rate
    cache_hits: u64,
    /// Cache misses
    cache_misses: u64,
}

impl HybridStorage {
    /// Create a new hybrid storage instance
    pub async fn new(db_path: impl AsRef<Path>, config: HybridStorageConfig) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Initialize document storage
        let document_storage = Arc::new(RwLock::new(
            FileStorage::open(db_path.to_str().unwrap()).await?,
        ));

        // Initialize graph storage if enabled
        let graph_storage = if config.enable_graph_storage {
            let graph_path = db_path.join("graph");
            Some(Arc::new(RwLock::new(
                NativeGraphStorage::new(graph_path, config.graph_config.clone()).await?,
            )))
        } else {
            None
        };

        // Initialize routing cache using DashMap for concurrent access
        let routing_cache = Arc::new(DashMap::with_capacity(config.routing_cache_size));

        Ok(Self {
            document_storage,
            graph_storage,
            config,
            routing_cache,
            stats: Arc::new(RwLock::new(HybridStats::default())),
        })
    }

    /// Determine which storage backend to use for a given path
    fn determine_storage_type(&self, path: &str) -> StorageType {
        // Validate and canonicalize path to prevent traversal attacks
        let safe_path = self.sanitize_path(path);

        // Check if path matches graph patterns
        for pattern in &self.config.graph_patterns {
            if self.matches_pattern(&safe_path, pattern) {
                return StorageType::Graph;
            }
        }

        // Special cases for hybrid operations
        if safe_path.contains("/symbols/") && safe_path.ends_with(".md") {
            // Symbol documentation goes to both
            return StorageType::Both;
        }

        // Default to document storage
        StorageType::Document
    }

    /// Sanitize path to prevent directory traversal attacks
    fn sanitize_path(&self, path: &str) -> String {
        // Remove any directory traversal attempts
        let cleaned = path
            .replace("..", "")
            .replace("./", "")
            .replace("~", "")
            .replace("\\", "/");

        // Ensure path starts with /
        if !cleaned.starts_with('/') {
            format!("/{}", cleaned)
        } else {
            cleaned
        }
    }

    /// Simple pattern matching (could be enhanced with glob)
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix("*") {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }

    /// Route operation to appropriate storage with caching
    async fn route_operation(&self, path: &str) -> StorageType {
        // Check cache first
        if let Some(entry) = self.routing_cache.get(path) {
            let storage_type = entry.clone();
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
            return storage_type;
        }

        // Determine storage type
        let storage_type = self.determine_storage_type(path);

        // Update cache
        self.routing_cache
            .insert(path.to_string(), storage_type.clone());

        let mut stats = self.stats.write().await;
        stats.cache_misses += 1;

        storage_type
    }

    /// Convert a document to a graph node (for symbol data)
    fn document_to_node(&self, doc: &Document) -> Result<Option<(Uuid, GraphNode)>> {
        // Check if this is a symbol document
        if !doc.path.as_str().starts_with("/symbols/") {
            return Ok(None);
        }

        // Parse symbol data from document content
        let content = String::from_utf8_lossy(&doc.content);

        // Try to deserialize as SymbolEntry
        if let Ok(symbol_entry) = serde_json::from_str::<SymbolEntry>(&content) {
            let node = GraphNode {
                id: symbol_entry.id,
                node_type: format!("{:?}", symbol_entry.symbol.symbol_type),
                qualified_name: symbol_entry.qualified_name,
                file_path: symbol_entry.file_path.to_string_lossy().to_string(),
                location: crate::graph_storage::NodeLocation {
                    start_line: symbol_entry.symbol.start_line,
                    start_column: symbol_entry.symbol.start_column,
                    end_line: symbol_entry.symbol.end_line,
                    end_column: symbol_entry.symbol.end_column,
                },
                metadata: Default::default(),
                updated_at: doc.updated_at.timestamp(),
            };

            Ok(Some((symbol_entry.id, node)))
        } else {
            Ok(None)
        }
    }

    /// Get statistics about hybrid storage operations
    pub async fn get_stats(&self) -> Result<HybridStatsReport> {
        let stats = self.stats.read().await;
        let cache_hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
            stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64
        } else {
            0.0
        };

        Ok(HybridStatsReport {
            document_ops: stats.document_ops,
            graph_ops: stats.graph_ops,
            hybrid_ops: stats.hybrid_ops,
            cache_hit_rate,
            total_ops: stats.document_ops + stats.graph_ops + stats.hybrid_ops,
        })
    }
}

/// Public statistics report
#[derive(Debug, Clone)]
pub struct HybridStatsReport {
    pub document_ops: u64,
    pub graph_ops: u64,
    pub hybrid_ops: u64,
    pub cache_hit_rate: f64,
    pub total_ops: u64,
}

#[async_trait]
impl Storage for HybridStorage {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        Self::new(path, HybridStorageConfig::default()).await
    }

    async fn insert(&mut self, document: Document) -> Result<()> {
        let path = document.path.as_str();
        let storage_type = self.route_operation(path).await;

        match storage_type {
            StorageType::Document => {
                let mut storage = self.document_storage.write().await;
                storage.insert(document).await?;

                let mut stats = self.stats.write().await;
                stats.document_ops += 1;
            }
            StorageType::Graph => {
                // Convert document to graph node if applicable
                if let Some(graph_storage) = &self.graph_storage {
                    if let Some((id, node)) = self.document_to_node(&document)? {
                        let mut storage = graph_storage.write().await;
                        storage.store_node(id, node).await?;

                        let mut stats = self.stats.write().await;
                        stats.graph_ops += 1;
                    }
                } else {
                    // Fallback to document storage if graph not enabled
                    let mut storage = self.document_storage.write().await;
                    storage.insert(document).await?;

                    let mut stats = self.stats.write().await;
                    stats.document_ops += 1;
                }
            }
            StorageType::Both => {
                // Store in both backends
                let mut doc_storage = self.document_storage.write().await;
                doc_storage.insert(document.clone()).await?;

                if let Some(graph_storage) = &self.graph_storage {
                    if let Some((id, node)) = self.document_to_node(&document)? {
                        let mut graph = graph_storage.write().await;
                        graph.store_node(id, node).await?;
                    }
                }

                let mut stats = self.stats.write().await;
                stats.hybrid_ops += 1;
            }
        }

        Ok(())
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        // For gets, we need to check document storage first
        // Graph storage doesn't store full documents
        let storage = self.document_storage.read().await;
        storage.get(id).await
    }

    async fn update(&mut self, document: Document) -> Result<()> {
        let path = document.path.as_str();
        let storage_type = self.route_operation(path).await;

        match storage_type {
            StorageType::Document => {
                let mut storage = self.document_storage.write().await;
                storage.update(document).await?;
            }
            StorageType::Graph => {
                if let Some(graph_storage) = &self.graph_storage {
                    if let Some((id, node)) = self.document_to_node(&document)? {
                        let mut storage = graph_storage.write().await;
                        storage.store_node(id, node).await?;
                    }
                }
            }
            StorageType::Both => {
                let mut doc_storage = self.document_storage.write().await;
                doc_storage.update(document.clone()).await?;

                if let Some(graph_storage) = &self.graph_storage {
                    if let Some((id, node)) = self.document_to_node(&document)? {
                        let mut graph = graph_storage.write().await;
                        graph.store_node(id, node).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        // Delete from document storage
        let mut doc_storage = self.document_storage.write().await;
        let deleted = doc_storage.delete(id).await?;

        // Also try to delete from graph storage if it might be there
        if let Some(graph_storage) = &self.graph_storage {
            // Graph storage would need a delete_node method
            // For now, we just track in document storage
        }

        Ok(deleted)
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        // List from document storage only
        // Graph nodes aren't full documents
        let storage = self.document_storage.read().await;
        storage.list_all().await
    }

    async fn sync(&mut self) -> Result<()> {
        // Sync both storages
        let mut doc_storage = self.document_storage.write().await;
        doc_storage.sync().await?;

        if let Some(graph_storage) = &self.graph_storage {
            let mut graph = graph_storage.write().await;
            graph.sync().await?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        // Flush both storages
        let mut doc_storage = self.document_storage.write().await;
        doc_storage.flush().await?;

        if let Some(graph_storage) = &self.graph_storage {
            let mut graph = graph_storage.write().await;
            graph.flush().await?;
        }

        Ok(())
    }

    async fn close(self) -> Result<()> {
        // Close both storages
        let doc_storage = Arc::try_unwrap(self.document_storage)
            .map_err(|_| anyhow::anyhow!("Cannot close document storage with active references"))?
            .into_inner();
        doc_storage.close().await?;

        if let Some(graph_storage) = self.graph_storage {
            let graph = Arc::try_unwrap(graph_storage)
                .map_err(|_| anyhow::anyhow!("Cannot close graph storage with active references"))?
                .into_inner();
            graph.close().await?;
        }

        Ok(())
    }
}

// Implement GraphStorage trait to expose graph operations
#[async_trait]
impl GraphStorage for HybridStorage {
    async fn store_node(&mut self, node_id: Uuid, node_data: GraphNode) -> Result<()> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            let result = storage.store_node(node_id, node_data).await;

            // Track graph operation
            let mut stats = self.stats.write().await;
            stats.graph_ops += 1;

            result
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn get_node(&self, node_id: Uuid) -> Result<Option<GraphNode>> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.get_node(node_id).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn store_edge(&mut self, from: Uuid, to: Uuid, edge: GraphEdge) -> Result<()> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            let result = storage.store_edge(from, to, edge).await;

            // Track graph operation
            let mut stats = self.stats.write().await;
            stats.graph_ops += 1;

            result
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn get_edges(
        &self,
        node: Uuid,
        direction: petgraph::Direction,
    ) -> Result<Vec<(Uuid, GraphEdge)>> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.get_edges(node, direction).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn get_subgraph(
        &self,
        roots: &[Uuid],
        max_depth: usize,
    ) -> Result<crate::graph_storage::GraphSubset> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.get_subgraph(roots, max_depth).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn find_paths(
        &self,
        from: Uuid,
        to: Uuid,
        max_paths: usize,
    ) -> Result<Vec<crate::graph_storage::GraphPath>> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.find_paths(from, to, max_paths).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn get_nodes_by_type(&self, node_type: &str) -> Result<Vec<Uuid>> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.get_nodes_by_type(node_type).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn update_edge_metadata(
        &mut self,
        from: Uuid,
        to: Uuid,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<()> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            storage.update_edge_metadata(from, to, metadata).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<bool> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            storage.remove_edge(from, to).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn delete_node(&mut self, node_id: Uuid) -> Result<bool> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            storage.delete_node(node_id).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn get_graph_stats(&self) -> Result<crate::graph_storage::GraphStats> {
        if let Some(graph_storage) = &self.graph_storage {
            let storage = graph_storage.read().await;
            storage.get_graph_stats().await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn batch_insert_nodes(&mut self, nodes: Vec<(Uuid, GraphNode)>) -> Result<()> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            storage.batch_insert_nodes(nodes).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }

    async fn batch_insert_edges(&mut self, edges: Vec<(Uuid, Uuid, GraphEdge)>) -> Result<()> {
        if let Some(graph_storage) = &self.graph_storage {
            let mut storage = graph_storage.write().await;
            storage.batch_insert_edges(edges).await
        } else {
            Err(anyhow::anyhow!("Graph storage not enabled"))
        }
    }
}
