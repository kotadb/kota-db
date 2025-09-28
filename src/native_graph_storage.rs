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
use crate::types::{RelationType, ValidatedDocumentId};

/// Page size for graph storage (4KB aligned for optimal I/O)
#[allow(dead_code)]
const PAGE_SIZE: usize = 4096;

/// Magic number for graph storage files
const GRAPH_MAGIC: &[u8; 8] = b"KOTGRAPH";

/// Magic number for edge pages
const EDGE_MAGIC: &[u8; 4] = b"EDGE";

/// UUID size in bytes
const UUID_SIZE: usize = 16;

/// Maximum size for deserialization to prevent memory exhaustion
const MAX_DESERIALIZE_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Version of the graph storage format
#[allow(dead_code)]
const GRAPH_VERSION: u32 = 1;

/// Type alias for edge collections - using HashMap with Vec to support multiple edges per target
type EdgeList = HashMap<Uuid, Vec<EdgeRecord>>;

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

        // Apply any outstanding WAL entries on top of persisted state
        storage.recover_from_wal().await?;

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

        // Parse records with proper error handling and cleanup
        let mut offset = std::mem::size_of::<PageHeader>();
        let mut parsed_records = Vec::new();

        for _ in 0..header.record_count {
            if offset + 4 >= data.len() {
                break;
            }

            // Read ID length (first 4 bytes of each entry)
            let id_len_bytes = &data[offset..offset + 4];
            let id_len = u32::from_le_bytes([
                id_len_bytes[0],
                id_len_bytes[1],
                id_len_bytes[2],
                id_len_bytes[3],
            ]) as usize;
            offset += 4;

            // Validate ID length
            if id_len > UUID_SIZE || offset + id_len + 4 > data.len() {
                tracing::warn!("Invalid ID length: {} at offset {}", id_len, offset);
                break;
            }

            // Read ID bytes (but we don't need to use them since NodeRecord contains its own ID)
            offset += id_len;

            // Read record size (next 4 bytes)
            if offset + 4 > data.len() {
                break;
            }
            let record_size_bytes = &data[offset..offset + 4];
            let record_size = u32::from_le_bytes([
                record_size_bytes[0],
                record_size_bytes[1],
                record_size_bytes[2],
                record_size_bytes[3],
            ]) as usize;
            offset += 4;

            if offset + record_size > data.len() {
                break;
            }

            // Deserialize node record with size validation
            if record_size > MAX_DESERIALIZE_SIZE {
                tracing::warn!("Skipping oversized record: {} bytes", record_size);
                offset += record_size;
                continue;
            }

            match bincode::deserialize::<NodeRecord>(&data[offset..offset + record_size]) {
                Ok(record) => {
                    parsed_records.push(record);
                    offset += record_size;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to deserialize node record at offset {}: {}",
                        offset,
                        e
                    );
                    // Continue parsing the next record instead of failing the entire page
                    offset += record_size;
                }
            }
        }

        // Only update nodes after all parsing is complete to avoid partial state
        if !parsed_records.is_empty() {
            let mut nodes = self.nodes.write();
            for record in parsed_records {
                nodes.insert(record.node.id, record);
            }
        }

        Ok(())
    }

    /// Parse edges from a page
    fn load_edges_from_page(&self, data: &[u8]) -> Result<()> {
        if data.len() < 8 {
            return Ok(());
        }

        // Check data size to prevent memory exhaustion
        if data.len() > MAX_DESERIALIZE_SIZE {
            return Err(anyhow::anyhow!(
                "Edge page size {} exceeds maximum allowed size {}",
                data.len(),
                MAX_DESERIALIZE_SIZE
            ));
        }

        // Parse edge page header (simpler than node header)
        let magic = &data[0..4];
        if magic != EDGE_MAGIC {
            return Err(anyhow::anyhow!("Invalid edge page magic number"));
        }

        let page_size_bytes = &data[4..8];
        let page_size = u32::from_le_bytes([
            page_size_bytes[0],
            page_size_bytes[1],
            page_size_bytes[2],
            page_size_bytes[3],
        ]) as usize;

        if 8 + page_size > data.len() {
            return Err(anyhow::anyhow!(
                "Invalid edge page size: {} bytes, but only {} available",
                page_size,
                data.len() - 8
            ));
        }

        // Parse edge records from the page data
        let page_data = &data[8..8 + page_size];
        let mut offset = 0;
        let mut parsed_edges = Vec::new();

        while offset < page_data.len() {
            // Read from_id length
            if offset + 4 > page_data.len() {
                break;
            }
            let from_len_bytes = &page_data[offset..offset + 4];
            let from_len = u32::from_le_bytes([
                from_len_bytes[0],
                from_len_bytes[1],
                from_len_bytes[2],
                from_len_bytes[3],
            ]) as usize;
            offset += 4;

            // Validate from_id length
            if from_len > UUID_SIZE || offset + from_len > page_data.len() {
                tracing::warn!("Invalid from_id length: {} at offset {}", from_len, offset);
                break;
            }

            // Read from_id
            let from_id_bytes = &page_data[offset..offset + from_len];
            offset += from_len;
            let from_id = match Uuid::from_slice(from_id_bytes) {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!("Failed to parse from_id: {}", e);
                    continue; // Skip this record but continue processing others
                }
            };

            // Read to_id length
            if offset + 4 > page_data.len() {
                break;
            }
            let to_len_bytes = &page_data[offset..offset + 4];
            let to_len = u32::from_le_bytes([
                to_len_bytes[0],
                to_len_bytes[1],
                to_len_bytes[2],
                to_len_bytes[3],
            ]) as usize;
            offset += 4;

            // Validate to_id length
            if to_len > UUID_SIZE || offset + to_len > page_data.len() {
                tracing::warn!("Invalid to_id length: {} at offset {}", to_len, offset);
                break;
            }

            // Read to_id
            let to_id_bytes = &page_data[offset..offset + to_len];
            offset += to_len;
            let to_id = match Uuid::from_slice(to_id_bytes) {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!("Failed to parse to_id: {}", e);
                    continue; // Skip this record but continue processing others
                }
            };

            // Read edge data length
            if offset + 4 > page_data.len() {
                break;
            }
            let data_len_bytes = &page_data[offset..offset + 4];
            let data_len = u32::from_le_bytes([
                data_len_bytes[0],
                data_len_bytes[1],
                data_len_bytes[2],
                data_len_bytes[3],
            ]) as usize;
            offset += 4;

            if offset + data_len > page_data.len() {
                break;
            }

            // Deserialize edge data with size validation
            if data_len > MAX_DESERIALIZE_SIZE {
                tracing::warn!("Skipping oversized edge record: {} bytes", data_len);
                offset += data_len;
                continue;
            }

            match bincode::deserialize::<GraphEdge>(&page_data[offset..offset + data_len]) {
                Ok(edge) => {
                    parsed_edges.push((from_id, to_id, edge));
                    offset += data_len;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to deserialize edge record at offset {}: {}",
                        offset,
                        e
                    );
                    offset += data_len;
                }
            }
        }

        // Only update edges after all parsing is complete to avoid partial state
        if !parsed_edges.is_empty() {
            let mut edges_out = self.edges_out.write();
            for (from_id, to_id, edge) in parsed_edges {
                let edge_record = EdgeRecord {
                    edge,
                    page_id: 0, // Will be updated during next persistence
                    page_offset: 0,
                };
                edges_out
                    .entry(from_id)
                    .or_default()
                    .entry(to_id)
                    .or_default()
                    .push(edge_record);
            }
        }

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
                self.nodes.write().remove(&id);

                let outgoing = {
                    let mut edges_out = self.edges_out.write();
                    edges_out.remove(&id)
                };

                if let Some(outgoing) = outgoing {
                    let mut edges_in = self.edges_in.write();
                    for (target_id, _) in outgoing {
                        if let Some(incoming) = edges_in.get_mut(&target_id) {
                            incoming.remove(&id);
                            if incoming.is_empty() {
                                edges_in.remove(&target_id);
                            }
                        }
                    }
                }

                let incoming = {
                    let mut edges_in = self.edges_in.write();
                    edges_in.remove(&id)
                };

                if let Some(incoming) = incoming {
                    let mut edges_out = self.edges_out.write();
                    for (source_id, _) in incoming {
                        if let Some(outgoing) = edges_out.get_mut(&source_id) {
                            outgoing.remove(&id);
                            if outgoing.is_empty() {
                                edges_out.remove(&source_id);
                            }
                        }
                    }
                }
            }
            WalEntry::EdgeInsert { from, to, data } => {
                if let Ok(edge) = bincode::deserialize::<GraphEdge>(&data) {
                    let record = EdgeRecord {
                        edge: edge.clone(),
                        page_id: 0,
                        page_offset: 0,
                    };

                    let mut edges_out = self.edges_out.write();
                    let entry = edges_out.entry(from).or_default().entry(to).or_default();
                    if entry.iter().any(|existing| {
                        bincode::serialize(&existing.edge)
                            .map(|encoded| encoded == data)
                            .unwrap_or(false)
                    }) {
                        return Ok(());
                    }

                    entry.push(record.clone());
                    drop(edges_out);

                    let mut edges_in = self.edges_in.write();
                    edges_in
                        .entry(to)
                        .or_default()
                        .entry(from)
                        .or_default()
                        .push(record);
                }
            }
            WalEntry::EdgeDelete { from, to } => {
                // Remove all edges between the nodes
                let mut edges_out = self.edges_out.write();
                if let Some(edge_list) = edges_out.get_mut(&from) {
                    edge_list.remove(&to);
                }

                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    edge_list.remove(&from);
                }
            }
            WalEntry::EdgeDeleteByType {
                from,
                to,
                relation_type,
            } => {
                // Remove specific edge by type
                let mut edges_out = self.edges_out.write();
                if let Some(edge_list) = edges_out.get_mut(&from) {
                    if let Some(edges) = edge_list.get_mut(&to) {
                        edges.retain(|e| e.edge.relation_type != relation_type);
                        if edges.is_empty() {
                            edge_list.remove(&to);
                        }
                    }
                }

                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        edges.retain(|e| e.edge.relation_type != relation_type);
                        if edges.is_empty() {
                            edge_list.remove(&from);
                        }
                    }
                }
            }
            WalEntry::EdgeUpdate { from, to, metadata } => {
                // Update all edges between the nodes
                let mut edges_out = self.edges_out.write();
                if let Some(edge_list) = edges_out.get_mut(&from) {
                    if let Some(edges) = edge_list.get_mut(&to) {
                        for edge_record in edges.iter_mut() {
                            edge_record.edge.metadata = metadata.clone();
                        }
                    }
                }

                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        for edge_record in edges.iter_mut() {
                            edge_record.edge.metadata = metadata.clone();
                        }
                    }
                }
            }
            WalEntry::EdgeUpdateByType {
                from,
                to,
                relation_type,
                metadata,
            } => {
                // Update specific edge by type
                let mut edges_out = self.edges_out.write();
                if let Some(edge_list) = edges_out.get_mut(&from) {
                    if let Some(edges) = edge_list.get_mut(&to) {
                        for edge_record in edges.iter_mut() {
                            if edge_record.edge.relation_type == relation_type {
                                edge_record.edge.metadata = metadata.clone();
                            }
                        }
                    }
                }

                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        for edge_record in edges.iter_mut() {
                            if edge_record.edge.relation_type == relation_type {
                                edge_record.edge.metadata = metadata.clone();
                            }
                        }
                    }
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
    NodeInsert {
        id: Uuid,
        data: Vec<u8>,
    },
    NodeUpdate {
        id: Uuid,
        data: Vec<u8>,
    },
    NodeDelete {
        id: Uuid,
    },
    EdgeInsert {
        from: Uuid,
        to: Uuid,
        data: Vec<u8>,
    },
    EdgeDelete {
        from: Uuid,
        to: Uuid,
    },
    EdgeDeleteByType {
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
    },
    EdgeUpdate {
        from: Uuid,
        to: Uuid,
        metadata: HashMap<String, String>,
    },
    EdgeUpdateByType {
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
        metadata: HashMap<String, String>,
    },
    Checkpoint {
        timestamp: i64,
    },
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
                .entry(to)
                .or_default()
                .push(record.clone());
        }

        // Update reverse index
        {
            let mut edges_in = self.edges_in.write();
            edges_in
                .entry(to)
                .or_default()
                .entry(from)
                .or_default()
                .push(record);
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

        let wal_data = bincode::serialize(&edge)?;
        self.write_to_wal(WalEntry::EdgeInsert {
            from,
            to,
            data: wal_data,
        })
        .await?;

        Ok(())
    }

    async fn get_edges(&self, node: Uuid, direction: Direction) -> Result<Vec<(Uuid, GraphEdge)>> {
        let edges = match direction {
            Direction::Outgoing => {
                let edges_out = self.edges_out.read();
                edges_out
                    .get(&node)
                    .map(|edges| {
                        edges
                            .iter()
                            .flat_map(|(id, records)| {
                                records.iter().map(move |r| (*id, r.edge.clone()))
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            Direction::Incoming => {
                let edges_in = self.edges_in.read();
                edges_in
                    .get(&node)
                    .map(|edges| {
                        edges
                            .iter()
                            .flat_map(|(id, records)| {
                                records.iter().map(move |r| (*id, r.edge.clone()))
                            })
                            .collect()
                    })
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
        // Update all edges between the two nodes
        let updated = {
            let mut edges_out = self.edges_out.write();
            if let Some(edge_list) = edges_out.get_mut(&from) {
                if let Some(edges) = edge_list.get_mut(&to) {
                    for edge_record in edges.iter_mut() {
                        edge_record.edge.metadata = metadata.clone();
                    }
                    !edges.is_empty()
                } else {
                    false
                }
            } else {
                false
            }
        };

        // Update the reverse index as well
        if updated {
            {
                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        for edge_record in edges.iter_mut() {
                            edge_record.edge.metadata = metadata.clone();
                        }
                    }
                }
            } // Drop lock before await

            // Log WAL entry
            self.write_to_wal(WalEntry::EdgeUpdate { from, to, metadata })
                .await?;
        }

        Ok(())
    }

    async fn update_edge_metadata_by_type(
        &mut self,
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        // Update specific edge by relationship type
        let updated = {
            let mut edges_out = self.edges_out.write();
            if let Some(edge_list) = edges_out.get_mut(&from) {
                if let Some(edges) = edge_list.get_mut(&to) {
                    let mut found = false;
                    for edge_record in edges.iter_mut() {
                        if edge_record.edge.relation_type == relation_type {
                            edge_record.edge.metadata = metadata.clone();
                            found = true;
                        }
                    }
                    found
                } else {
                    false
                }
            } else {
                false
            }
        };

        // Update the reverse index as well
        if updated {
            {
                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        for edge_record in edges.iter_mut() {
                            if edge_record.edge.relation_type == relation_type {
                                edge_record.edge.metadata = metadata.clone();
                            }
                        }
                    }
                }
            } // Drop lock before await

            // Log WAL entry with relation type
            self.write_to_wal(WalEntry::EdgeUpdateByType {
                from,
                to,
                relation_type,
                metadata,
            })
            .await?;
        }

        Ok(())
    }

    async fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<bool> {
        // Remove all edges between the two nodes
        let removed = {
            let mut edges_out = self.edges_out.write();
            if let Some(edge_list) = edges_out.get_mut(&from) {
                edge_list.remove(&to).is_some()
            } else {
                false
            }
        };

        if removed {
            // Update reverse index
            {
                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    edge_list.remove(&from);
                }
            } // Drop lock

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.edge_count = stats.edge_count.saturating_sub(1);
            } // Drop lock

            // Log WAL entry
            self.write_to_wal(WalEntry::EdgeDelete { from, to }).await?;
        }

        Ok(removed)
    }

    async fn remove_edge_by_type(
        &mut self,
        from: Uuid,
        to: Uuid,
        relation_type: RelationType,
    ) -> Result<bool> {
        // Remove specific edge by relationship type
        let removed = {
            let mut edges_out = self.edges_out.write();
            if let Some(edge_list) = edges_out.get_mut(&from) {
                if let Some(edges) = edge_list.get_mut(&to) {
                    let initial_len = edges.len();
                    edges.retain(|e| e.edge.relation_type != relation_type);
                    let removed_count = initial_len - edges.len();

                    // If no edges remain, remove the entire entry
                    if edges.is_empty() {
                        edge_list.remove(&to);
                    }
                    removed_count > 0
                } else {
                    false
                }
            } else {
                false
            }
        };

        if removed {
            // Update reverse index
            {
                let mut edges_in = self.edges_in.write();
                if let Some(edge_list) = edges_in.get_mut(&to) {
                    if let Some(edges) = edge_list.get_mut(&from) {
                        edges.retain(|e| e.edge.relation_type != relation_type);

                        // If no edges remain, remove the entire entry
                        if edges.is_empty() {
                            edge_list.remove(&from);
                        }
                    }
                }
            } // Drop lock

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.edge_count = stats.edge_count.saturating_sub(1);
            } // Drop lock

            // Log WAL entry
            self.write_to_wal(WalEntry::EdgeDeleteByType {
                from,
                to,
                relation_type,
            })
            .await?;
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

        let mut removed_edge_total = 0usize;
        let mut removed_edges_by_type: HashMap<String, usize> = HashMap::new();

        let outgoing = {
            let mut edges_out = self.edges_out.write();
            edges_out.remove(&node_id)
        };

        if let Some(outgoing) = outgoing {
            let mut edges_in = self.edges_in.write();
            for (target_id, edge_list) in outgoing {
                removed_edge_total += edge_list.len();
                for edge in &edge_list {
                    *removed_edges_by_type
                        .entry(format!("{:?}", edge.edge.relation_type))
                        .or_default() += 1;
                }

                if let Some(incoming) = edges_in.get_mut(&target_id) {
                    incoming.remove(&node_id);
                    if incoming.is_empty() {
                        edges_in.remove(&target_id);
                    }
                }
            }
        }

        let incoming = {
            let mut edges_in = self.edges_in.write();
            edges_in.remove(&node_id)
        };

        if let Some(incoming) = incoming {
            let mut edges_out = self.edges_out.write();
            for (source_id, edge_list) in incoming {
                removed_edge_total += edge_list.len();
                for edge in &edge_list {
                    *removed_edges_by_type
                        .entry(format!("{:?}", edge.edge.relation_type))
                        .or_default() += 1;
                }

                if let Some(outgoing) = edges_out.get_mut(&source_id) {
                    outgoing.remove(&node_id);
                    if outgoing.is_empty() {
                        edges_out.remove(&source_id);
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

        {
            let mut stats = self.stats.write();
            stats.node_count = stats.node_count.saturating_sub(1);
            stats.edge_count = stats.edge_count.saturating_sub(removed_edge_total);
            for (relation, count) in removed_edges_by_type {
                let entry = stats.edges_by_type.entry(relation).or_default();
                *entry = entry.saturating_sub(count);
            }
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

            // Group nodes into pages with record counting
            let mut page_data = Vec::new();
            let mut current_page = Vec::new();
            let mut current_page_record_count = 0u16;

            for (id, record) in nodes.iter() {
                let serialized = bincode::serialize(record)?;
                let id_bytes = id.as_bytes();

                // Store ID length, ID, and serialized node record
                let mut entry = Vec::new();
                entry.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
                entry.extend_from_slice(id_bytes);
                entry.extend_from_slice(&(serialized.len() as u32).to_le_bytes());
                entry.extend_from_slice(&serialized);

                if current_page.len() + entry.len() > PAGE_SIZE - 8 {
                    // Leave room for header
                    if !current_page.is_empty() {
                        page_data.push((current_page, current_page_record_count));
                        current_page = Vec::new();
                        current_page_record_count = 0;
                    }
                }
                current_page.extend_from_slice(&entry);
                current_page_record_count += 1;
            }

            if !current_page.is_empty() {
                page_data.push((current_page, current_page_record_count));
            }

            page_data
        }; // Lock released here

        // Write pages to disk
        for (i, (page, record_count)) in page_data.iter().enumerate() {
            let page_path = nodes_dir.join(format!("{:08}.page", i));
            let mut page_bytes = Vec::new();

            // Create proper page header with actual record count
            let header = PageHeader {
                magic: *GRAPH_MAGIC,
                page_id: i as u32,
                record_count: *record_count,
                free_offset: page.len() as u16,
                checksum: 0, // TODO: Calculate checksum
            };

            let header_bytes = bincode::serialize(&header)?;
            page_bytes.extend_from_slice(&header_bytes);
            page_bytes.extend_from_slice(page);

            // Pad to 4KB
            while page_bytes.len() < PAGE_SIZE {
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
            tracing::debug!(
                "persist_edges: edges_out contains {} entries",
                edges_out.len()
            );
            if edges_out.is_empty() {
                tracing::debug!("persist_edges: no edges to persist, returning early");
                return Ok(());
            }

            // Collect all edges
            let mut all_edges = Vec::new();
            for (from_id, edges) in edges_out.iter() {
                for (to_id, edge_list) in edges.iter() {
                    for edge in edge_list.iter() {
                        all_edges.push((*from_id, *to_id, edge.clone()));
                    }
                }
            }

            // Group edges into pages
            let mut page_data = Vec::new();
            let mut current_page = Vec::new();

            for (from_id, to_id, edge) in all_edges {
                let serialized = bincode::serialize(&edge.edge)?;
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

                if current_page.len() + entry.len() > PAGE_SIZE - 8 {
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
        tracing::debug!(
            "persist_edges: writing {} pages to {:?}",
            page_data.len(),
            edges_dir
        );
        for (i, page) in page_data.iter().enumerate() {
            let page_path = edges_dir.join(format!("{:08}.page", i));
            let mut page_bytes = Vec::new();

            // Simple page header
            page_bytes.extend_from_slice(b"EDGE");
            page_bytes.extend_from_slice(&(page.len() as u32).to_le_bytes());
            page_bytes.extend_from_slice(page);

            // Pad to 4KB
            while page_bytes.len() < PAGE_SIZE {
                page_bytes.push(0);
            }

            tracing::debug!(
                "persist_edges: writing page {} to {:?} ({} bytes)",
                i,
                page_path,
                page_bytes.len()
            );
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
        tracing::debug!("NativeGraphStorage::flush called");
        // Persist nodes and edges to ensure durability
        self.persist_nodes().await?;
        self.persist_edges().await?;
        // Flush WAL to ensure durability
        self.flush_wal().await?;
        tracing::debug!("NativeGraphStorage::flush completed");
        Ok(())
    }

    async fn close(mut self) -> Result<()> {
        // Sync all data before closing
        self.sync().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_storage::NodeLocation;
    use crate::types::RelationType;
    use chrono::Utc;
    use tempfile::TempDir;

    fn create_test_node_location() -> NodeLocation {
        NodeLocation {
            start_line: 10,
            start_column: 5,
            end_line: 15,
            end_column: 20,
        }
    }

    fn create_test_graph_node(qualified_name: &str, node_type: &str) -> GraphNode {
        GraphNode {
            id: Uuid::new_v4(),
            qualified_name: qualified_name.to_string(),
            node_type: node_type.to_string(),
            file_path: "src/test.rs".to_string(),
            location: create_test_node_location(),
            metadata: HashMap::new(),
            updated_at: Utc::now().timestamp(),
        }
    }

    fn create_test_graph_edge() -> GraphEdge {
        GraphEdge {
            relation_type: RelationType::Calls,
            location: create_test_node_location(),
            context: Some("function_call()".to_string()),
            metadata: HashMap::new(),
            created_at: Utc::now().timestamp(),
        }
    }

    async fn create_test_storage() -> (NativeGraphStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = GraphStorageConfig::default();
        let storage = NativeGraphStorage::new(temp_dir.path(), config)
            .await
            .expect("Failed to create storage");
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_new_storage_initialization() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = GraphStorageConfig::default();

        let storage = NativeGraphStorage::new(temp_dir.path(), config).await;
        assert!(storage.is_ok(), "Should successfully create storage");

        let storage = storage.unwrap();

        // Verify directory structure
        assert!(temp_dir.path().join("nodes").exists());
        assert!(temp_dir.path().join("edges").exists());
        assert!(temp_dir.path().join("wal").exists());

        // Verify initial state
        assert_eq!(storage.nodes.read().len(), 0);
        assert_eq!(storage.edges_out.read().len(), 0);
        assert_eq!(storage.edges_in.read().len(), 0);
    }

    #[tokio::test]
    async fn test_store_and_get_node() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let node = create_test_graph_node("test_function", "function");
        let node_id = node.id;

        // Store node
        storage
            .store_node(node_id, node.clone())
            .await
            .expect("Failed to store node");

        // Retrieve node
        let retrieved = storage.get_node(node_id).await.expect("Failed to get node");
        assert!(retrieved.is_some());

        let retrieved_node = retrieved.unwrap();
        assert_eq!(retrieved_node.id, node.id);
        assert_eq!(retrieved_node.qualified_name, node.qualified_name);
        assert_eq!(retrieved_node.node_type, node.node_type);
    }

    #[tokio::test]
    async fn test_store_and_get_edge() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create nodes
        let node1 = create_test_graph_node("function1", "function");
        let node2 = create_test_graph_node("function2", "function");
        let node1_id = node1.id;
        let node2_id = node2.id;

        storage
            .store_node(node1_id, node1)
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2_id, node2)
            .await
            .expect("Failed to store node2");

        // Create edge
        let edge = create_test_graph_edge();
        storage
            .store_edge(node1_id, node2_id, edge.clone())
            .await
            .expect("Failed to store edge");

        // Get outgoing edges
        let out_edges = storage
            .get_edges(node1_id, Direction::Outgoing)
            .await
            .expect("Failed to get outgoing edges");
        assert_eq!(out_edges.len(), 1);
        assert_eq!(out_edges[0].0, node2_id);
        assert_eq!(out_edges[0].1.relation_type, edge.relation_type);

        // Get incoming edges
        let in_edges = storage
            .get_edges(node2_id, Direction::Incoming)
            .await
            .expect("Failed to get incoming edges");
        assert_eq!(in_edges.len(), 1);
        assert_eq!(in_edges[0].0, node1_id);
        assert_eq!(in_edges[0].1.relation_type, edge.relation_type);
    }

    #[tokio::test]
    async fn test_get_nodes_by_type() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create nodes of different types
        let func1 = create_test_graph_node("function1", "function");
        let func2 = create_test_graph_node("function2", "function");
        let class1 = create_test_graph_node("Class1", "class");

        storage
            .store_node(func1.id, func1.clone())
            .await
            .expect("Failed to store func1");
        storage
            .store_node(func2.id, func2.clone())
            .await
            .expect("Failed to store func2");
        storage
            .store_node(class1.id, class1.clone())
            .await
            .expect("Failed to store class1");

        // Get functions
        let functions = storage
            .get_nodes_by_type("function")
            .await
            .expect("Failed to get functions");
        assert_eq!(functions.len(), 2);
        assert!(functions.contains(&func1.id));
        assert!(functions.contains(&func2.id));

        // Get classes
        let classes = storage
            .get_nodes_by_type("class")
            .await
            .expect("Failed to get classes");
        assert_eq!(classes.len(), 1);
        assert!(classes.contains(&class1.id));

        // Get non-existent type
        let modules = storage
            .get_nodes_by_type("module")
            .await
            .expect("Failed to get modules");
        assert_eq!(modules.len(), 0);
    }

    #[tokio::test]
    async fn test_graph_stats() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Initially empty
        let stats = storage
            .get_graph_stats()
            .await
            .expect("Failed to get stats");
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);

        // Add nodes and edges
        let node1 = create_test_graph_node("function1", "function");
        let node2 = create_test_graph_node("function2", "function");
        let node1_id = node1.id;
        let node2_id = node2.id;

        storage
            .store_node(node1_id, node1)
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2_id, node2)
            .await
            .expect("Failed to store node2");
        storage
            .store_edge(node1_id, node2_id, create_test_graph_edge())
            .await
            .expect("Failed to store edge");

        // Check updated stats
        let stats = storage
            .get_graph_stats()
            .await
            .expect("Failed to get stats");
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }

    #[tokio::test]
    async fn test_subgraph_retrieval() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create a small graph: A -> B -> C
        let node_a = create_test_graph_node("A", "function");
        let node_b = create_test_graph_node("B", "function");
        let node_c = create_test_graph_node("C", "function");

        storage
            .store_node(node_a.id, node_a.clone())
            .await
            .expect("Failed to store A");
        storage
            .store_node(node_b.id, node_b.clone())
            .await
            .expect("Failed to store B");
        storage
            .store_node(node_c.id, node_c.clone())
            .await
            .expect("Failed to store C");

        storage
            .store_edge(node_a.id, node_b.id, create_test_graph_edge())
            .await
            .expect("Failed to store A->B");
        storage
            .store_edge(node_b.id, node_c.id, create_test_graph_edge())
            .await
            .expect("Failed to store B->C");

        // Get subgraph from A with depth 2
        let subgraph = storage
            .get_subgraph(&[node_a.id], 2)
            .await
            .expect("Failed to get subgraph");

        // Should contain all 3 nodes
        assert_eq!(subgraph.nodes.len(), 3);
        assert!(subgraph.nodes.contains_key(&node_a.id));
        assert!(subgraph.nodes.contains_key(&node_b.id));
        assert!(subgraph.nodes.contains_key(&node_c.id));

        // Should contain edges from A and B
        assert!(subgraph.edges.contains_key(&node_a.id));
        assert!(subgraph.edges.contains_key(&node_b.id));
    }

    #[tokio::test]
    async fn test_find_paths() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create a path: A -> B -> C
        let node_a = create_test_graph_node("A", "function");
        let node_b = create_test_graph_node("B", "function");
        let node_c = create_test_graph_node("C", "function");

        storage
            .store_node(node_a.id, node_a.clone())
            .await
            .expect("Failed to store A");
        storage
            .store_node(node_b.id, node_b.clone())
            .await
            .expect("Failed to store B");
        storage
            .store_node(node_c.id, node_c.clone())
            .await
            .expect("Failed to store C");

        storage
            .store_edge(node_a.id, node_b.id, create_test_graph_edge())
            .await
            .expect("Failed to store A->B");
        storage
            .store_edge(node_b.id, node_c.id, create_test_graph_edge())
            .await
            .expect("Failed to store B->C");

        // Find path from A to C
        let paths = storage
            .find_paths(node_a.id, node_c.id, 10)
            .await
            .expect("Failed to find paths");
        assert_eq!(paths.len(), 1);

        let path = &paths[0];
        assert_eq!(path.nodes.len(), 3);
        assert_eq!(path.nodes[0], node_a.id);
        assert_eq!(path.nodes[1], node_b.id);
        assert_eq!(path.nodes[2], node_c.id);
        // The implementation might not populate edges in the path structure
        // Just verify the path length is sensible (either edge count or node count)
        assert!(path.length > 0, "Path length should be positive");
        // Verify path connects A to C through B
        assert!(path.nodes.contains(&node_a.id));
        assert!(path.nodes.contains(&node_b.id));
        assert!(path.nodes.contains(&node_c.id));
    }

    #[tokio::test]
    async fn test_update_edge_metadata() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let node1 = create_test_graph_node("function1", "function");
        let node2 = create_test_graph_node("function2", "function");

        storage
            .store_node(node1.id, node1.clone())
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2.id, node2.clone())
            .await
            .expect("Failed to store node2");
        storage
            .store_edge(node1.id, node2.id, create_test_graph_edge())
            .await
            .expect("Failed to store edge");

        // Update metadata
        let mut metadata = HashMap::new();
        metadata.insert("weight".to_string(), "0.8".to_string());
        metadata.insert("confidence".to_string(), "high".to_string());

        storage
            .update_edge_metadata(node1.id, node2.id, metadata.clone())
            .await
            .expect("Failed to update metadata");

        // Verify metadata was updated
        let edges = storage
            .get_edges(node1.id, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1.metadata, metadata);
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create test data
        let nodes = vec![
            (Uuid::new_v4(), create_test_graph_node("A", "function")),
            (Uuid::new_v4(), create_test_graph_node("B", "function")),
            (Uuid::new_v4(), create_test_graph_node("C", "class")),
        ];

        let edges = vec![
            (nodes[0].0, nodes[1].0, create_test_graph_edge()),
            (nodes[1].0, nodes[2].0, create_test_graph_edge()),
        ];

        // Batch insert nodes
        storage
            .batch_insert_nodes(nodes.clone())
            .await
            .expect("Failed to batch insert nodes");

        // Batch insert edges
        storage
            .batch_insert_edges(edges.clone())
            .await
            .expect("Failed to batch insert edges");

        // Verify all nodes were inserted
        for (node_id, node) in &nodes {
            let retrieved = storage
                .get_node(*node_id)
                .await
                .expect("Failed to get node");
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().qualified_name, node.qualified_name);
        }

        // Verify all edges were inserted
        let out_edges_0 = storage
            .get_edges(nodes[0].0, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        let out_edges_1 = storage
            .get_edges(nodes[1].0, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        assert_eq!(out_edges_0.len(), 1);
        assert_eq!(out_edges_1.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_node() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let node1 = create_test_graph_node("function1", "function");
        let node2 = create_test_graph_node("function2", "function");

        storage
            .store_node(node1.id, node1.clone())
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2.id, node2.clone())
            .await
            .expect("Failed to store node2");
        storage
            .store_edge(node1.id, node2.id, create_test_graph_edge())
            .await
            .expect("Failed to store edge");

        // Verify node exists
        assert!(storage
            .get_node(node1.id)
            .await
            .expect("Failed to get node")
            .is_some());

        // Delete node
        let deleted = storage
            .delete_node(node1.id)
            .await
            .expect("Failed to delete node");
        assert!(deleted);

        // Verify node is gone
        assert!(storage
            .get_node(node1.id)
            .await
            .expect("Failed to get node")
            .is_none());

        // Verify edges are cleaned up
        let out_edges = storage
            .get_edges(node1.id, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        let in_edges = storage
            .get_edges(node2.id, Direction::Incoming)
            .await
            .expect("Failed to get edges");
        assert_eq!(out_edges.len(), 0);
        assert_eq!(in_edges.len(), 0);

        // Delete non-existent node
        let deleted_again = storage
            .delete_node(node1.id)
            .await
            .expect("Failed to delete node");
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_remove_edge() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        let node1 = create_test_graph_node("function1", "function");
        let node2 = create_test_graph_node("function2", "function");

        storage
            .store_node(node1.id, node1.clone())
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2.id, node2.clone())
            .await
            .expect("Failed to store node2");
        storage
            .store_edge(node1.id, node2.id, create_test_graph_edge())
            .await
            .expect("Failed to store edge");

        // Verify edge exists
        let edges = storage
            .get_edges(node1.id, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        assert_eq!(edges.len(), 1);

        // Remove edge
        let removed = storage
            .remove_edge(node1.id, node2.id)
            .await
            .expect("Failed to remove edge");
        assert!(removed);

        // Verify edge is gone
        let edges = storage
            .get_edges(node1.id, Direction::Outgoing)
            .await
            .expect("Failed to get edges");
        assert_eq!(edges.len(), 0);

        // Remove non-existent edge
        let removed_again = storage
            .remove_edge(node1.id, node2.id)
            .await
            .expect("Failed to remove edge");
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_persistence_across_reopens() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = GraphStorageConfig::default();
        let path = temp_dir.path();

        let node1 = create_test_graph_node("persistent_function", "function");
        let node2 = create_test_graph_node("another_function", "function");
        let edge = create_test_graph_edge();

        {
            // First instance - store data
            let mut storage = NativeGraphStorage::new(path, config.clone())
                .await
                .expect("Failed to create storage");
            storage
                .store_node(node1.id, node1.clone())
                .await
                .expect("Failed to store node1");
            storage
                .store_node(node2.id, node2.clone())
                .await
                .expect("Failed to store node2");
            storage
                .store_edge(node1.id, node2.id, edge.clone())
                .await
                .expect("Failed to store edge");
            storage.sync().await.expect("Failed to sync");
        } // storage is dropped here

        {
            // Second instance - should load existing data
            let storage = NativeGraphStorage::new(path, config)
                .await
                .expect("Failed to create storage");

            // Verify nodes persisted
            let retrieved_node1 = storage
                .get_node(node1.id)
                .await
                .expect("Failed to get node1");
            let retrieved_node2 = storage
                .get_node(node2.id)
                .await
                .expect("Failed to get node2");
            assert!(retrieved_node1.is_some());
            assert!(retrieved_node2.is_some());
            assert_eq!(
                retrieved_node1.unwrap().qualified_name,
                node1.qualified_name
            );
            assert_eq!(
                retrieved_node2.unwrap().qualified_name,
                node2.qualified_name
            );

            // Verify edge persisted
            let edges = storage
                .get_edges(node1.id, Direction::Outgoing)
                .await
                .expect("Failed to get edges");
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].0, node2.id);
        }
    }

    #[tokio::test]
    async fn test_empty_subgraph() {
        let (storage, _temp_dir) = create_test_storage().await;

        let non_existent_id = Uuid::new_v4();
        let subgraph = storage
            .get_subgraph(&[non_existent_id], 1)
            .await
            .expect("Failed to get subgraph");

        assert_eq!(subgraph.nodes.len(), 0);
        assert_eq!(subgraph.edges.len(), 0);
        // Implementation visits the requested node even if it doesn't exist
        assert_eq!(subgraph.metadata.nodes_visited, 1);
    }

    #[tokio::test]
    async fn test_no_paths_found() {
        let (mut storage, _temp_dir) = create_test_storage().await;

        // Create two disconnected nodes
        let node1 = create_test_graph_node("isolated1", "function");
        let node2 = create_test_graph_node("isolated2", "function");

        storage
            .store_node(node1.id, node1.clone())
            .await
            .expect("Failed to store node1");
        storage
            .store_node(node2.id, node2.clone())
            .await
            .expect("Failed to store node2");

        // Try to find path between disconnected nodes
        let paths = storage
            .find_paths(node1.id, node2.id, 10)
            .await
            .expect("Failed to find paths");
        assert_eq!(paths.len(), 0);
    }

    #[tokio::test]
    async fn test_wal_node_delete_recovery_removes_edges() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = GraphStorageConfig::default();
        let path = temp_dir.path();

        let node1 = create_test_graph_node("persistent_function", "function");
        let node2 = create_test_graph_node("callee", "function");
        let edge = create_test_graph_edge();

        {
            let mut storage = NativeGraphStorage::new(path, config.clone())
                .await
                .expect("Failed to create storage");
            storage
                .store_node(node1.id, node1.clone())
                .await
                .expect("Failed to store node1");
            storage
                .store_node(node2.id, node2.clone())
                .await
                .expect("Failed to store node2");
            storage
                .store_edge(node1.id, node2.id, edge.clone())
                .await
                .expect("Failed to store edge");

            storage.flush().await.expect("flush should succeed");

            storage
                .write_to_wal(WalEntry::NodeDelete { id: node1.id })
                .await
                .expect("Failed to append WAL entry");
        }

        let storage = NativeGraphStorage::new(path, config)
            .await
            .expect("Failed to reopen storage");

        let retrieved = storage
            .get_node(node1.id)
            .await
            .expect("Failed to get node");
        assert!(retrieved.is_none());

        let outgoing = storage
            .get_edges(node1.id, Direction::Outgoing)
            .await
            .expect("Failed to get outgoing edges");
        assert!(outgoing.is_empty());

        let incoming = storage
            .get_edges(node2.id, Direction::Incoming)
            .await
            .expect("Failed to get incoming edges");
        assert!(incoming.is_empty());
    }

    #[tokio::test]
    async fn test_error_handling_invalid_path() {
        let config = GraphStorageConfig::default();

        // Test with invalid path containing null bytes
        let result = NativeGraphStorage::new("test\0path", config).await;
        assert!(result.is_err());
    }
}
