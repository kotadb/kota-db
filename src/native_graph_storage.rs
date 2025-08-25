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

/// Maximum size for deserialization to prevent memory exhaustion
const MAX_DESERIALIZE_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Version of the graph storage format
#[allow(dead_code)]
const GRAPH_VERSION: u32 = 1;

/// Type alias for edge collections - using HashMap for O(1) lookups
type EdgeList = HashMap<Uuid, EdgeRecord>;

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

        // Recover from WAL first
        storage.recover_from_wal().await?;

        // Then load existing data if present
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

        // Check data size to prevent memory exhaustion
        if data.len() > MAX_DESERIALIZE_SIZE {
            return Err(anyhow::anyhow!(
                "Page size {} exceeds maximum allowed size {}",
                data.len(),
                MAX_DESERIALIZE_SIZE
            ));
        }

        // Parse header with bounded deserialization
        let header_size = std::mem::size_of::<PageHeader>();
        let header: PageHeader = bincode::deserialize(&data[..header_size])?;

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

            // Deserialize node record with size validation
            if size > MAX_DESERIALIZE_SIZE {
                tracing::warn!("Skipping oversized record: {} bytes", size);
                break;
            }
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
        use tokio::io::AsyncWriteExt;

        let data = bincode::serialize(&entry)?;
        let mut wal = self.wal.lock().await;

        // Check if we need to rotate the WAL
        if wal.size + data.len() as u64 > wal.max_size {
            self.rotate_wal(&mut wal).await?;
        }

        // Open or create WAL file if not exists
        if wal.file.is_none() {
            let wal_path = self.db_path.join("wal").join("current.wal");
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&wal_path)
                .await?;
            wal.file = Some(file);
        }

        if let Some(file) = &mut wal.file {
            // Write entry size (4 bytes) and data
            let size_bytes = (data.len() as u32).to_le_bytes();
            file.write_all(&size_bytes).await?;
            file.write_all(&data).await?;

            // Ensure data is written to disk
            file.sync_all().await?;

            wal.size += (size_bytes.len() + data.len()) as u64;
        }

        Ok(())
    }

    /// Rotate the WAL file
    async fn rotate_wal(&self, wal: &mut WriteAheadLog) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // Close current WAL file
        if let Some(mut file) = wal.file.take() {
            file.flush().await?;
            file.sync_all().await?;
        }

        // Rename current WAL to timestamped file
        let timestamp = chrono::Utc::now().timestamp();
        let current_path = self.db_path.join("wal").join("current.wal");
        let archive_path = self
            .db_path
            .join("wal")
            .join(format!("wal_{}.archive", timestamp));

        if current_path.exists() {
            fs::rename(&current_path, &archive_path).await?;
        }

        // Reset WAL size
        wal.size = 0;

        Ok(())
    }

    /// Recover from WAL on startup
    async fn recover_from_wal(&self) -> Result<()> {
        let wal_dir = self.db_path.join("wal");
        if !wal_dir.exists() {
            return Ok(());
        }

        // First process any archived WAL files
        let mut entries = fs::read_dir(&wal_dir).await?;
        let mut archives = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("archive") {
                archives.push(path);
            }
        }

        // Sort archives by timestamp to apply in order
        archives.sort();

        for archive_path in archives {
            self.apply_wal_file(&archive_path).await?;
            fs::remove_file(&archive_path).await?;
        }

        // Then process current WAL if it exists
        let current_wal = wal_dir.join("current.wal");
        if current_wal.exists() {
            self.apply_wal_file(&current_wal).await?;
            // Rotate it to start fresh
            let timestamp = chrono::Utc::now().timestamp();
            let archive_path = wal_dir.join(format!("wal_{}.archive", timestamp));
            fs::rename(&current_wal, &archive_path).await?;
        }

        Ok(())
    }

    /// Apply a single WAL file
    async fn apply_wal_file(&self, path: &Path) -> Result<()> {
        let data = fs::read(path).await?;
        let mut offset = 0;

        while offset + 4 <= data.len() {
            // Read entry size
            let size_bytes: [u8; 4] = data[offset..offset + 4].try_into()?;
            let size = u32::from_le_bytes(size_bytes) as usize;
            offset += 4;

            if offset + size > data.len() {
                break; // Incomplete entry
            }

            // Validate size before deserializing
            if size > MAX_DESERIALIZE_SIZE {
                tracing::warn!("Skipping oversized WAL entry: {} bytes", size);
                break;
            }

            // Apply entry with bounded deserialization
            let entry_data = &data[offset..offset + size];
            if let Ok(entry) = bincode::deserialize::<WalEntry>(entry_data) {
                self.apply_wal_entry(entry).await?;
            }
            offset += size;
        }

        Ok(())
    }

    /// Apply a single WAL entry
    async fn apply_wal_entry(&self, entry: WalEntry) -> Result<()> {
        match entry {
            WalEntry::NodeInsert { id, data } => {
                if let Ok(record) = bincode::deserialize::<NodeRecord>(&data) {
                    let mut nodes = self.nodes.write();
                    nodes.insert(id, record);
                }
            }
            WalEntry::NodeDelete { id } => {
                let mut nodes = self.nodes.write();
                nodes.remove(&id);
            }
            WalEntry::EdgeInsert { from, to, data } => {
                if let Ok(edge) = bincode::deserialize::<GraphEdge>(&data) {
                    let record = EdgeRecord {
                        edge: edge.clone(),
                        page_id: 0,
                        page_offset: 0,
                    };
                    let mut edges_out = self.edges_out.write();
                    edges_out
                        .entry(from)
                        .or_default()
                        .insert(to, record.clone());

                    let mut edges_in = self.edges_in.write();
                    edges_in.entry(to).or_default().insert(from, record);
                }
            }
            WalEntry::EdgeDelete { from, to } => {
                let mut edges_out = self.edges_out.write();
                if let Some(edges) = edges_out.get_mut(&from) {
                    edges.remove(&to);
                }

                let mut edges_in = self.edges_in.write();
                if let Some(edges) = edges_in.get_mut(&to) {
                    edges.remove(&from);
                }
            }
            WalEntry::NodeUpdate { .. } => {
                // Node updates would be handled here
            }
            WalEntry::Checkpoint { .. } => {
                // Checkpoint markers can be ignored during recovery
            }
        }
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
                .insert(to, record.clone());
        }

        // Update reverse index
        {
            let mut edges_in = self.edges_in.write();
            edges_in.entry(to).or_default().insert(from, record);
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
            if depth > max_depth || visited.contains(&node_id) {
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

                if depth < max_depth {
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
        // Use iterative implementation with depth limit to prevent stack overflow
        let max_depth = self.config.max_traversal_depth;
        self.find_paths_iterative(from, to, max_paths, max_depth)
            .await
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
        // Update edge metadata in both indices - O(1) with HashMap
        let mut updated = false;

        {
            let mut edges_out = self.edges_out.write();
            if let Some(edges) = edges_out.get_mut(&from) {
                if let Some(record) = edges.get_mut(&to) {
                    record.edge.metadata = metadata.clone();
                    updated = true;
                }
            }
        }

        if updated {
            let mut edges_in = self.edges_in.write();
            if let Some(edges) = edges_in.get_mut(&to) {
                if let Some(record) = edges.get_mut(&from) {
                    record.edge.metadata = metadata;
                }
            }
        }

        Ok(())
    }

    async fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<bool> {
        let mut removed = false;

        // Remove from forward index - O(1) with HashMap
        {
            let mut edges_out = self.edges_out.write();
            if let Some(edges) = edges_out.get_mut(&from) {
                removed = edges.remove(&to).is_some();
            }
        }

        // Remove from reverse index - O(1) with HashMap
        if removed {
            let mut edges_in = self.edges_in.write();
            if let Some(edges) = edges_in.get_mut(&to) {
                edges.remove(&from);
            }

            // Update stats
            let mut stats = self.stats.write();
            stats.edge_count = stats.edge_count.saturating_sub(1);
        }

        Ok(removed)
    }

    async fn delete_node(&mut self, node_id: Uuid) -> Result<bool> {
        // Check if node exists
        let exists = {
            let nodes = self.nodes.read();
            nodes.contains_key(&node_id)
        };

        if !exists {
            return Ok(false);
        }

        // Remove all outgoing edges
        {
            let mut edges_out = self.edges_out.write();
            if let Some(outgoing) = edges_out.remove(&node_id) {
                // Update reverse indices for target nodes
                let mut edges_in = self.edges_in.write();
                for (target_id, _) in outgoing {
                    if let Some(incoming) = edges_in.get_mut(&target_id) {
                        incoming.remove(&node_id);
                    }
                }
            }
        }

        // Remove all incoming edges
        {
            let mut edges_in = self.edges_in.write();
            if let Some(incoming) = edges_in.remove(&node_id) {
                // Update forward indices for source nodes
                let mut edges_out = self.edges_out.write();
                for (source_id, _) in incoming {
                    if let Some(outgoing) = edges_out.get_mut(&source_id) {
                        outgoing.remove(&node_id);
                    }
                }
            }
        }

        // Remove from type index
        let node_type = {
            let nodes = self.nodes.read();
            nodes.get(&node_id).map(|r| r.node.node_type.clone())
        };

        if let Some(node_type) = node_type {
            let mut nodes_by_type = self.nodes_by_type.write();
            if let Some(type_set) = nodes_by_type.get_mut(&node_type) {
                type_set.remove(&node_id);
            }
        }

        // Remove from name index
        let qualified_name = {
            let nodes = self.nodes.read();
            nodes.get(&node_id).map(|r| r.node.qualified_name.clone())
        };

        if let Some(qualified_name) = qualified_name {
            let mut nodes_by_name = self.nodes_by_name.write();
            if let Some(name_set) = nodes_by_name.get_mut(&qualified_name) {
                name_set.remove(&node_id);
            }
        }

        // Remove the node itself
        {
            let mut nodes = self.nodes.write();
            nodes.remove(&node_id);
        }

        // Write to WAL
        self.write_to_wal(WalEntry::NodeDelete { id: node_id })
            .await?;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.node_count = stats.node_count.saturating_sub(1);
        }

        Ok(true)
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

/// Helper for iterative path finding with cycle detection
impl NativeGraphStorage {
    /// Iterative DFS implementation to prevent stack overflow
    async fn find_paths_iterative(
        &self,
        from: Uuid,
        to: Uuid,
        max_paths: usize,
        max_depth: usize,
    ) -> Result<Vec<GraphPath>> {
        use std::collections::VecDeque;

        #[derive(Clone)]
        struct SearchState {
            current: Uuid,
            path: Vec<Uuid>,
            visited: HashSet<Uuid>,
        }

        let mut paths = Vec::new();
        let mut stack = VecDeque::new();

        // Initialize search
        let initial_state = SearchState {
            current: from,
            path: vec![from],
            visited: HashSet::from([from]),
        };
        stack.push_back(initial_state);

        while let Some(state) = stack.pop_back() {
            // Check if we've found enough paths
            if paths.len() >= max_paths {
                break;
            }

            // Check depth limit to prevent infinite loops
            // Allow paths up to max_depth nodes (not edges)
            if state.path.len() > max_depth + 1 {
                continue;
            }

            // Check if we've reached the target
            if state.current == to {
                paths.push(GraphPath {
                    nodes: state.path.clone(),
                    edges: Vec::new(), // Would populate with actual edges
                    length: state.path.len(),
                });
                continue;
            }

            // Explore neighbors
            let edges = self.get_edges(state.current, Direction::Outgoing).await?;
            for (next_node, _edge) in edges {
                // Skip if already visited (cycle detection)
                if state.visited.contains(&next_node) {
                    continue;
                }

                // Create new state for this path
                let mut new_state = state.clone();
                new_state.current = next_node;
                new_state.path.push(next_node);
                new_state.visited.insert(next_node);

                stack.push_back(new_state);
            }
        }

        Ok(paths)
    }

    /// Persist nodes to disk in page-based format
    async fn persist_nodes(&self) -> Result<()> {
        let nodes_dir = self.db_path.join("nodes");
        fs::create_dir_all(&nodes_dir).await?;

        // Collect data while holding the lock
        let page_data = {
            let nodes = self.nodes.read();
            if nodes.is_empty() {
                return Ok(());
            }

            // Group nodes into pages
            let mut page_data = Vec::new();
            let mut current_page = Vec::new();

            for (id, record) in nodes.iter() {
                let serialized = bincode::serialize(record)?;
                let id_bytes = id.as_bytes();

                // Store ID length, ID, and serialized node record
                let mut entry = Vec::new();
                entry.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
                entry.extend_from_slice(id_bytes);
                entry.extend_from_slice(&(serialized.len() as u32).to_le_bytes());
                entry.extend_from_slice(&serialized);

                if current_page.len() + entry.len() > 4096 - 8 {
                    // Leave room for header
                    if !current_page.is_empty() {
                        page_data.push(current_page);
                        current_page = Vec::new();
                    }
                }
                current_page.extend_from_slice(&entry);
            }

            if !current_page.is_empty() {
                page_data.push(current_page);
            }

            page_data
        }; // Lock released here

        // Write pages to disk
        for (i, page) in page_data.iter().enumerate() {
            let page_path = nodes_dir.join(format!("{:08}.page", i));
            let mut page_bytes = Vec::new();

            // Create proper page header
            let header = PageHeader {
                magic: *GRAPH_MAGIC,
                page_id: i as u32,
                record_count: 1, // Simplified - each page has variable records
                free_offset: page.len() as u16,
                checksum: 0, // TODO: Calculate checksum
            };

            let header_bytes = bincode::serialize(&header)?;
            page_bytes.extend_from_slice(&header_bytes);
            page_bytes.extend_from_slice(page);

            // Pad to 4KB
            while page_bytes.len() < 4096 {
                page_bytes.push(0);
            }

            fs::write(&page_path, &page_bytes).await?;
        }

        Ok(())
    }

    /// Persist edges to disk in page-based format
    async fn persist_edges(&self) -> Result<()> {
        let edges_dir = self.db_path.join("edges");
        fs::create_dir_all(&edges_dir).await?;

        // Collect data while holding the lock
        let page_data = {
            let edges_out = self.edges_out.read();
            if edges_out.is_empty() {
                return Ok(());
            }

            // Collect all edges
            let mut all_edges = Vec::new();
            for (from_id, edges) in edges_out.iter() {
                for (to_id, edge) in edges.iter() {
                    all_edges.push((*from_id, *to_id, edge.clone()));
                }
            }

            // Group edges into pages
            let mut page_data = Vec::new();
            let mut current_page = Vec::new();

            for (from_id, to_id, edge) in all_edges {
                let serialized = bincode::serialize(&edge)?;
                let from_bytes = from_id.as_bytes();
                let to_bytes = to_id.as_bytes();

                // Store edge entry
                let mut entry = Vec::new();
                entry.extend_from_slice(&(from_bytes.len() as u32).to_le_bytes());
                entry.extend_from_slice(from_bytes);
                entry.extend_from_slice(&(to_bytes.len() as u32).to_le_bytes());
                entry.extend_from_slice(to_bytes);
                entry.extend_from_slice(&(serialized.len() as u32).to_le_bytes());
                entry.extend_from_slice(&serialized);

                if current_page.len() + entry.len() > 4096 - 8 {
                    // Leave room for header
                    if !current_page.is_empty() {
                        page_data.push(current_page);
                        current_page = Vec::new();
                    }
                }
                current_page.extend_from_slice(&entry);
            }

            if !current_page.is_empty() {
                page_data.push(current_page);
            }

            page_data
        }; // Lock released here

        // Write pages to disk
        for (i, page) in page_data.iter().enumerate() {
            let page_path = edges_dir.join(format!("{:08}.page", i));
            let mut page_bytes = Vec::new();

            // Simple page header
            page_bytes.extend_from_slice(b"EDGE");
            page_bytes.extend_from_slice(&(page.len() as u32).to_le_bytes());
            page_bytes.extend_from_slice(page);

            // Pad to 4KB
            while page_bytes.len() < 4096 {
                page_bytes.push(0);
            }

            fs::write(&page_path, &page_bytes).await?;
        }

        Ok(())
    }

    /// Flush the write-ahead log
    async fn flush_wal(&self) -> Result<()> {
        let mut wal = self.wal.lock().await;
        if let Some(ref mut file) = wal.file {
            file.sync_all().await?;
        }
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
        // Persist nodes to disk
        self.persist_nodes().await?;

        // Persist edges to disk
        self.persist_edges().await?;

        // Flush WAL
        self.flush_wal().await?;

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        // Flush WAL to ensure durability
        self.flush_wal().await?;
        Ok(())
    }

    async fn close(mut self) -> Result<()> {
        // Sync all data before closing
        self.sync().await?;
        Ok(())
    }
}
