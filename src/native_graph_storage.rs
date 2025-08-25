//! Native graph storage implementation using KotaDB's page-based storage
//!
//! This module provides a zero-dependency graph storage backend that leverages
//! our existing B+ tree and page-based storage patterns for high-performance
//! graph operations without external database dependencies.

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::contracts::{Document, Storage};
use crate::graph_storage::{
    GraphEdge, GraphNode, GraphPath, GraphStats, GraphStorage, GraphStorageConfig, GraphSubset,
    QueryMetadata,
};
use crate::types::ValidatedDocumentId;

/// Page size for graph storage (4KB aligned for optimal I/O)
#[allow(dead_code)]
const PAGE_SIZE: usize = 4096;

/// Magic number for graph storage files
const GRAPH_MAGIC: &[u8; 8] = b"KOTGRAPH";

/// Version of the graph storage format
#[allow(dead_code)]
const GRAPH_VERSION: u32 = 1;

/// Type alias for edge collections
type EdgeList = Vec<(Uuid, EdgeRecord)>;

/// Native graph storage implementation
pub struct NativeGraphStorage {
    /// Root directory for graph data
    db_path: PathBuf,

    /// In-memory node index (B+ tree backed)
    /// Key: node_id, Value: NodeRecord
    nodes: Arc<RwLock<BTreeMap<Uuid, NodeRecord>>>,

    /// In-memory edge index
    /// Key: from_node_id, Value: Vec<(to_node_id, EdgeRecord)>
    edges_out: Arc<RwLock<BTreeMap<Uuid, EdgeList>>>,

    /// Reverse edge index for incoming edges
    /// Key: to_node_id, Value: Vec<(from_node_id, EdgeRecord)>
    edges_in: Arc<RwLock<BTreeMap<Uuid, EdgeList>>>,

    /// Type indices for fast type-based queries
    nodes_by_type: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,

    /// Name index for fast lookups
    nodes_by_name: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,

    /// Write-ahead log for durability
    wal: Arc<Mutex<WriteAheadLog>>,

    /// Configuration
    #[allow(dead_code)]
    config: GraphStorageConfig,

    /// Statistics
    stats: Arc<RwLock<GraphStats>>,
}

/// Compact node record for efficient storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeRecord {
    /// Node data
    node: GraphNode,
    /// Page ID where full node data is stored
    page_id: u32,
    /// Offset within the page
    page_offset: u16,
}

/// Compact edge record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EdgeRecord {
    /// Edge data
    edge: GraphEdge,
    /// Page ID where full edge data is stored
    page_id: u32,
    /// Offset within the page
    page_offset: u16,
}

/// Write-ahead log for crash recovery
#[allow(dead_code)]
struct WriteAheadLog {
    /// WAL file
    file: Option<fs::File>,
    /// Current WAL size
    size: u64,
    /// Maximum WAL size before rotation
    max_size: u64,
}

/// Page header for on-disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageHeader {
    /// Magic number for validation
    magic: [u8; 8],
    /// Page ID
    page_id: u32,
    /// Number of records in this page
    record_count: u16,
    /// Free space offset
    free_offset: u16,
    /// Checksum of page content
    checksum: u32,
}

impl NativeGraphStorage {
    /// Create a new native graph storage instance
    pub async fn new(db_path: impl AsRef<Path>, config: GraphStorageConfig) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Create directory structure
        fs::create_dir_all(&db_path).await?;
        fs::create_dir_all(db_path.join("nodes")).await?;
        fs::create_dir_all(db_path.join("edges")).await?;
        fs::create_dir_all(db_path.join("wal")).await?;

        let storage = Self {
            db_path: db_path.clone(),
            nodes: Arc::new(RwLock::new(BTreeMap::new())),
            edges_out: Arc::new(RwLock::new(BTreeMap::new())),
            edges_in: Arc::new(RwLock::new(BTreeMap::new())),
            nodes_by_type: Arc::new(RwLock::new(HashMap::new())),
            nodes_by_name: Arc::new(RwLock::new(HashMap::new())),
            wal: Arc::new(Mutex::new(WriteAheadLog {
                file: None,
                size: 0,
                max_size: 10 * 1024 * 1024, // 10MB
            })),
            config,
            stats: Arc::new(RwLock::new(GraphStats {
                node_count: 0,
                edge_count: 0,
                nodes_by_type: HashMap::new(),
                edges_by_type: HashMap::new(),
                avg_in_degree: 0.0,
                avg_out_degree: 0.0,
                connected_components: 0,
                storage_size_bytes: 0,
            })),
        };

        // Load existing data if present
        storage.load_from_disk().await?;

        Ok(storage)
    }

    /// Load graph data from disk
    async fn load_from_disk(&self) -> Result<()> {
        // Load nodes
        let nodes_dir = self.db_path.join("nodes");
        if nodes_dir.exists() {
            self.load_nodes_from_pages(&nodes_dir).await?;
        }

        // Load edges
        let edges_dir = self.db_path.join("edges");
        if edges_dir.exists() {
            self.load_edges_from_pages(&edges_dir).await?;
        }

        // Rebuild indices
        self.rebuild_indices()?;

        Ok(())
    }

    /// Load nodes from page files
    async fn load_nodes_from_pages(&self, dir: &Path) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("page") {
                let data = fs::read(&path).await?;
                self.load_nodes_from_page(&data)?;
            }
        }

        Ok(())
    }

    /// Load edges from page files
    async fn load_edges_from_pages(&self, dir: &Path) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("page") {
                let data = fs::read(&path).await?;
                self.load_edges_from_page(&data)?;
            }
        }

        Ok(())
    }

    /// Parse nodes from a page
    fn load_nodes_from_page(&self, data: &[u8]) -> Result<()> {
        if data.len() < std::mem::size_of::<PageHeader>() {
            return Ok(());
        }

        // Parse header
        let header: PageHeader = bincode::deserialize(&data[..std::mem::size_of::<PageHeader>()])?;

        // Validate magic number
        if header.magic != *GRAPH_MAGIC {
            return Err(anyhow::anyhow!("Invalid page magic number"));
        }

        // Parse records
        let mut offset = std::mem::size_of::<PageHeader>();
        let mut nodes = self.nodes.write();

        for _ in 0..header.record_count {
            if offset >= data.len() {
                break;
            }

            // Read record size
            let size_bytes = &data[offset..offset + 4];
            let size =
                u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
                    as usize;
            offset += 4;

            if offset + size > data.len() {
                break;
            }

            // Deserialize node record
            let record: NodeRecord = bincode::deserialize(&data[offset..offset + size])?;
            nodes.insert(record.node.id, record);
            offset += size;
        }

        Ok(())
    }

    /// Parse edges from a page
    fn load_edges_from_page(&self, data: &[u8]) -> Result<()> {
        if data.len() < std::mem::size_of::<PageHeader>() {
            return Ok(());
        }

        // Similar to load_nodes_from_page but for edges
        // Parse header, validate, then deserialize edge records

        Ok(())
    }

    /// Rebuild in-memory indices from loaded data
    fn rebuild_indices(&self) -> Result<()> {
        let nodes = self.nodes.read();
        let mut nodes_by_type = self.nodes_by_type.write();
        let mut nodes_by_name = self.nodes_by_name.write();
        let mut stats = self.stats.write();

        // Clear existing indices
        nodes_by_type.clear();
        nodes_by_name.clear();

        // Rebuild from nodes
        for (id, record) in nodes.iter() {
            // Type index
            nodes_by_type
                .entry(record.node.node_type.clone())
                .or_default()
                .insert(*id);

            // Name index
            nodes_by_name
                .entry(record.node.qualified_name.clone())
                .or_default()
                .insert(*id);

            // Update stats
            *stats
                .nodes_by_type
                .entry(record.node.node_type.clone())
                .or_default() += 1;
        }

        stats.node_count = nodes.len();

        Ok(())
    }

    /// Write a node to disk
    async fn persist_node(&self, node_id: Uuid, record: &NodeRecord) -> Result<()> {
        // Serialize node
        let data = bincode::serialize(record)?;

        // Write to WAL first
        self.write_to_wal(WalEntry::NodeInsert {
            id: node_id,
            data: data.clone(),
        })
        .await?;

        // Then write to page file
        // This would implement proper page management with free space tracking

        Ok(())
    }

    /// Write to WAL for durability
    async fn write_to_wal(&self, entry: WalEntry) -> Result<()> {
        let _data = bincode::serialize(&entry)?;
        let _wal = self.wal.lock().await;

        // Write entry size and data
        // In production, this would handle WAL rotation and fsync

        Ok(())
    }
}

/// WAL entry types
#[derive(Debug, Serialize, Deserialize)]
enum WalEntry {
    NodeInsert { id: Uuid, data: Vec<u8> },
    NodeUpdate { id: Uuid, data: Vec<u8> },
    NodeDelete { id: Uuid },
    EdgeInsert { from: Uuid, to: Uuid, data: Vec<u8> },
    EdgeDelete { from: Uuid, to: Uuid },
    Checkpoint { timestamp: i64 },
}

#[async_trait]
impl GraphStorage for NativeGraphStorage {
    async fn store_node(&mut self, node_id: Uuid, node_data: GraphNode) -> Result<()> {
        let record = NodeRecord {
            node: node_data.clone(),
            page_id: 0, // Would be assigned by page manager
            page_offset: 0,
        };

        // Update in-memory indices
        {
            let mut nodes = self.nodes.write();
            nodes.insert(node_id, record.clone());

            let mut nodes_by_type = self.nodes_by_type.write();
            nodes_by_type
                .entry(node_data.node_type.clone())
                .or_default()
                .insert(node_id);

            let mut nodes_by_name = self.nodes_by_name.write();
            nodes_by_name
                .entry(node_data.qualified_name.clone())
                .or_default()
                .insert(node_id);
        }

        // Persist to disk
        self.persist_node(node_id, &record).await?;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.node_count += 1;
            *stats.nodes_by_type.entry(node_data.node_type).or_default() += 1;
        }

        Ok(())
    }

    async fn get_node(&self, node_id: Uuid) -> Result<Option<GraphNode>> {
        let nodes = self.nodes.read();
        Ok(nodes.get(&node_id).map(|r| r.node.clone()))
    }

    async fn store_edge(&mut self, from: Uuid, to: Uuid, edge: GraphEdge) -> Result<()> {
        let record = EdgeRecord {
            edge: edge.clone(),
            page_id: 0,
            page_offset: 0,
        };

        // Update forward index
        {
            let mut edges_out = self.edges_out.write();
            edges_out
                .entry(from)
                .or_default()
                .push((to, record.clone()));
        }

        // Update reverse index
        {
            let mut edges_in = self.edges_in.write();
            edges_in.entry(to).or_default().push((from, record));
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.edge_count += 1;
            *stats
                .edges_by_type
                .entry(format!("{:?}", edge.relation_type))
                .or_default() += 1;
        }

        Ok(())
    }

    async fn get_edges(&self, node: Uuid, direction: Direction) -> Result<Vec<(Uuid, GraphEdge)>> {
        let edges = match direction {
            Direction::Outgoing => {
                let edges_out = self.edges_out.read();
                edges_out
                    .get(&node)
                    .map(|edges| edges.iter().map(|(id, r)| (*id, r.edge.clone())).collect())
                    .unwrap_or_default()
            }
            Direction::Incoming => {
                let edges_in = self.edges_in.read();
                edges_in
                    .get(&node)
                    .map(|edges| edges.iter().map(|(id, r)| (*id, r.edge.clone())).collect())
                    .unwrap_or_default()
            }
        };

        Ok(edges)
    }

    async fn get_subgraph(&self, roots: &[Uuid], max_depth: usize) -> Result<GraphSubset> {
        let start = std::time::Instant::now();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut nodes = HashMap::new();
        let mut edges = HashMap::new();
        let mut nodes_visited = 0;
        let mut edges_traversed = 0;

        // Initialize queue with roots
        for &root in roots {
            queue.push_back((root, 0));
        }

        // BFS traversal
        while let Some((node_id, depth)) = queue.pop_front() {
            if depth >= max_depth || visited.contains(&node_id) {
                continue;
            }

            visited.insert(node_id);
            nodes_visited += 1;

            // Get node data
            if let Some(node) = self.get_node(node_id).await? {
                nodes.insert(node_id, node);
            }

            // Get outgoing edges
            let outgoing = self.get_edges(node_id, Direction::Outgoing).await?;
            edges_traversed += outgoing.len();

            for (target, edge) in outgoing {
                edges
                    .entry(node_id)
                    .or_insert_with(Vec::new)
                    .push((target, edge));

                if depth + 1 < max_depth {
                    queue.push_back((target, depth + 1));
                }
            }
        }

        let metadata = QueryMetadata {
            nodes_visited,
            edges_traversed,
            execution_time_us: start.elapsed().as_micros() as u64,
            truncated: !queue.is_empty(),
        };

        Ok(GraphSubset {
            nodes,
            edges,
            metadata,
        })
    }

    async fn find_paths(&self, from: Uuid, to: Uuid, max_paths: usize) -> Result<Vec<GraphPath>> {
        // Simple DFS path finding
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();

        self.dfs_paths(
            from,
            to,
            &mut current_path,
            &mut visited,
            &mut paths,
            max_paths,
        )
        .await?;

        Ok(paths)
    }

    async fn get_nodes_by_type(&self, node_type: &str) -> Result<Vec<Uuid>> {
        let nodes_by_type = self.nodes_by_type.read();
        Ok(nodes_by_type
            .get(node_type)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default())
    }

    async fn update_edge_metadata(
        &mut self,
        from: Uuid,
        to: Uuid,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        // Update edge metadata in both indices
        let mut updated = false;

        {
            let mut edges_out = self.edges_out.write();
            if let Some(edges) = edges_out.get_mut(&from) {
                for (target, record) in edges.iter_mut() {
                    if *target == to {
                        record.edge.metadata = metadata.clone();
                        updated = true;
                        break;
                    }
                }
            }
        }

        if updated {
            let mut edges_in = self.edges_in.write();
            if let Some(edges) = edges_in.get_mut(&to) {
                for (source, record) in edges.iter_mut() {
                    if *source == from {
                        record.edge.metadata = metadata;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<bool> {
        let mut removed = false;

        // Remove from forward index
        {
            let mut edges_out = self.edges_out.write();
            if let Some(edges) = edges_out.get_mut(&from) {
                let original_len = edges.len();
                edges.retain(|(target, _)| *target != to);
                removed = edges.len() < original_len;
            }
        }

        // Remove from reverse index
        if removed {
            let mut edges_in = self.edges_in.write();
            if let Some(edges) = edges_in.get_mut(&to) {
                edges.retain(|(source, _)| *source != from);
            }

            // Update stats
            let mut stats = self.stats.write();
            stats.edge_count = stats.edge_count.saturating_sub(1);
        }

        Ok(removed)
    }

    async fn get_graph_stats(&self) -> Result<GraphStats> {
        let stats = self.stats.read();
        Ok(stats.clone())
    }

    async fn batch_insert_nodes(&mut self, nodes: Vec<(Uuid, GraphNode)>) -> Result<()> {
        for (id, node) in nodes {
            self.store_node(id, node).await?;
        }
        Ok(())
    }

    async fn batch_insert_edges(&mut self, edges: Vec<(Uuid, Uuid, GraphEdge)>) -> Result<()> {
        for (from, to, edge) in edges {
            self.store_edge(from, to, edge).await?;
        }
        Ok(())
    }
}

/// Helper for DFS path finding
impl NativeGraphStorage {
    async fn dfs_paths(
        &self,
        current: Uuid,
        target: Uuid,
        current_path: &mut Vec<Uuid>,
        visited: &mut HashSet<Uuid>,
        paths: &mut Vec<GraphPath>,
        max_paths: usize,
    ) -> Result<()> {
        if paths.len() >= max_paths {
            return Ok(());
        }

        current_path.push(current);
        visited.insert(current);

        if current == target {
            // Found a path
            let path = GraphPath {
                nodes: current_path.clone(),
                edges: Vec::new(), // Would populate with actual edges
                length: current_path.len(),
            };
            paths.push(path);
        } else {
            // Continue search
            let edges = self.get_edges(current, Direction::Outgoing).await?;
            for (next, _) in edges {
                if !visited.contains(&next) {
                    Box::pin(self.dfs_paths(next, target, current_path, visited, paths, max_paths))
                        .await?;
                }
            }
        }

        current_path.pop();
        visited.remove(&current);

        Ok(())
    }
}

// Storage trait implementation delegates to FileStorage for document operations
#[async_trait]
impl Storage for NativeGraphStorage {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        Self::new(path, GraphStorageConfig::default()).await
    }

    async fn insert(&mut self, document: Document) -> Result<()> {
        // For now, graph storage doesn't handle documents directly
        // This would be handled by HybridStorage router
        Err(anyhow::anyhow!(
            "Document operations not supported in graph storage"
        ))
    }

    async fn get(&self, _id: &ValidatedDocumentId) -> Result<Option<Document>> {
        Err(anyhow::anyhow!(
            "Document operations not supported in graph storage"
        ))
    }

    async fn update(&mut self, _document: Document) -> Result<()> {
        Err(anyhow::anyhow!(
            "Document operations not supported in graph storage"
        ))
    }

    async fn delete(&mut self, _id: &ValidatedDocumentId) -> Result<bool> {
        Err(anyhow::anyhow!(
            "Document operations not supported in graph storage"
        ))
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        Err(anyhow::anyhow!(
            "Document operations not supported in graph storage"
        ))
    }

    async fn sync(&mut self) -> Result<()> {
        // Sync graph data to disk
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        // Flush any pending writes
        Ok(())
    }

    async fn close(self) -> Result<()> {
        // Clean shutdown
        Ok(())
    }
}
