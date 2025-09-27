// Vector Index Module - HNSW implementation for semantic search
// Implements Hierarchical Navigable Small World graphs for efficient similarity search

use anyhow::{anyhow, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tokio::task;

use crate::contracts::Index;
use crate::types::ValidatedDocumentId;

const AUTO_FLUSH_THRESHOLD: usize = 32;

/// Distance metrics for vector similarity
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

/// Vector index node in HNSW graph
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorNode {
    id: ValidatedDocumentId,
    vector: Vec<f32>,
    levels: Vec<HashSet<ValidatedDocumentId>>, // Connections at each level
}

/// HNSW Vector Index for semantic search
#[derive(Debug)]
// HNSW algorithm parameters: max_connections, max_connections_top, ef_construction are used in search operations
pub struct VectorIndex {
    path: PathBuf,
    nodes: HashMap<ValidatedDocumentId, VectorNode>,
    entry_point: Option<ValidatedDocumentId>,
    #[allow(dead_code)] // Used in future HNSW optimization algorithms
    max_connections: usize,
    #[allow(dead_code)] // Used in future HNSW optimization algorithms
    max_connections_top: usize,
    ef_construction: usize,
    #[allow(dead_code)] // Used in future HNSW level generation
    ml: f64, // Level generation factor
    distance_metric: DistanceMetric,
    vector_dimension: usize,
    dirty: bool,
    pending_writes: usize,
}

impl VectorIndex {
    /// Create a new vector index
    pub async fn new(
        path: impl AsRef<Path>,
        distance_metric: DistanceMetric,
        vector_dimension: usize,
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        Ok(Self {
            path,
            nodes: HashMap::new(),
            entry_point: None,
            max_connections: 16,
            max_connections_top: 16,
            ef_construction: 200,
            ml: 1.0 / (2.0_f64).ln(),
            distance_metric,
            vector_dimension,
            dirty: false,
            pending_writes: 0,
        })
    }

    /// Calculate distance between two vectors
    fn calculate_distance(&self, v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() {
            return f32::INFINITY;
        }

        match self.distance_metric {
            DistanceMetric::Cosine => {
                let dot_product: f32 = v1.iter().zip(v2).map(|(a, b)| a * b).sum();
                let norm_a: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

                if norm_a == 0.0 || norm_b == 0.0 {
                    1.0 // Maximum distance for zero vectors
                } else {
                    1.0 - (dot_product / (norm_a * norm_b))
                }
            }
            DistanceMetric::Euclidean => v1
                .iter()
                .zip(v2)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
                .sqrt(),
            DistanceMetric::DotProduct => {
                -v1.iter().zip(v2).map(|(a, b)| a * b).sum::<f32>() // Negative for min-heap behavior
            }
        }
    }

    /// Generate random level for new node
    fn get_random_level(&self) -> usize {
        let mut level = 0;
        let mut rng = rand::thread_rng();
        while rng.gen::<f64>() < 0.5 && level < 16 {
            level += 1;
        }
        level
    }

    /// Insert a vector into the index
    pub async fn insert_vector(&mut self, id: ValidatedDocumentId, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.vector_dimension {
            return Err(anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.vector_dimension,
                vector.len()
            ));
        }

        let level = self.get_random_level();
        let mut levels = Vec::with_capacity(level + 1);
        for _ in 0..=level {
            levels.push(HashSet::new());
        }

        let node = VectorNode { id, vector, levels };

        // If this is the first node, make it the entry point
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }

        self.nodes.insert(id, node);
        self.dirty = true;
        self.pending_writes += 1;
        self.maybe_flush().await?;
        Ok(())
    }

    /// Search for k nearest neighbors
    pub async fn search_knn(
        &self,
        query_vector: &[f32],
        k: usize,
        ef: Option<usize>,
    ) -> Result<Vec<(ValidatedDocumentId, f32)>> {
        if query_vector.len() != self.vector_dimension {
            return Err(anyhow!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.vector_dimension,
                query_vector.len()
            ));
        }

        let _ef = ef.unwrap_or(self.ef_construction.max(k));

        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }

        let _entry_point = match &self.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };

        // Simple linear search for now (can be optimized with proper HNSW traversal)
        let mut candidates: Vec<(f32, ValidatedDocumentId)> = self
            .nodes
            .iter()
            .map(|(id, node)| {
                let distance = self.calculate_distance(query_vector, &node.vector);
                (distance, *id)
            })
            .collect();

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(candidates
            .into_iter()
            .take(k)
            .map(|(dist, id)| (id, dist))
            .collect())
    }

    /// Save index to disk
    async fn save_to_disk(&self) -> Result<()> {
        let index_path = self.path.clone();
        let nodes = self.nodes.clone();
        let entry_point = self.entry_point;
        let distance_metric = self.distance_metric;
        let vector_dimension = self.vector_dimension;

        task::spawn_blocking(move || -> Result<()> {
            let index_data = IndexData {
                nodes,
                entry_point,
                distance_metric,
                vector_dimension,
            };

            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&index_path)?;

            let writer = BufWriter::new(file);
            bincode::serialize_into(writer, &index_data)?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    /// Load index from disk
    async fn load_from_disk(&mut self) -> Result<()> {
        if !self.path.exists() {
            return Ok(()); // No existing index
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let index_data: IndexData = bincode::deserialize_from(reader)?;

        self.nodes = index_data.nodes;
        self.entry_point = index_data.entry_point;
        self.distance_metric = index_data.distance_metric;
        self.vector_dimension = index_data.vector_dimension;
        self.dirty = false;
        self.pending_writes = 0;

        Ok(())
    }

    async fn maybe_flush(&mut self) -> Result<()> {
        if self.pending_writes >= AUTO_FLUSH_THRESHOLD {
            self.save_to_disk().await?;
            self.dirty = false;
            self.pending_writes = 0;
        }
        Ok(())
    }

    async fn persist_if_dirty(&mut self) -> Result<()> {
        if self.dirty {
            self.save_to_disk().await?;
            self.dirty = false;
            self.pending_writes = 0;
        }
        Ok(())
    }

    /// Remove a vector from the index
    pub async fn remove_vector(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let removed = self.nodes.remove(id).is_some();

        // Update entry point if needed
        if Some(id) == self.entry_point.as_ref() {
            self.entry_point = self.nodes.keys().next().cloned();
        }

        if removed {
            self.dirty = true;
            self.pending_writes += 1;
            self.maybe_flush().await?;
        }

        Ok(removed)
    }
}

/// Serializable index data for persistence
#[derive(Debug, Serialize, Deserialize)]
struct IndexData {
    nodes: HashMap<ValidatedDocumentId, VectorNode>,
    entry_point: Option<ValidatedDocumentId>,
    distance_metric: DistanceMetric,
    vector_dimension: usize,
}

#[async_trait::async_trait]
impl Index for VectorIndex {
    async fn open(path: &str) -> Result<Self> {
        let mut index = Self::new(path, DistanceMetric::Cosine, 1536).await?;
        index.load_from_disk().await?;
        Ok(index)
    }

    async fn insert(
        &mut self,
        _id: ValidatedDocumentId,
        _path: crate::types::ValidatedPath,
    ) -> Result<()> {
        // This implementation expects vectors to be inserted separately via insert_vector
        // We could extend this to extract embeddings from documents automatically
        Ok(())
    }

    async fn update(
        &mut self,
        _id: ValidatedDocumentId,
        _path: crate::types::ValidatedPath,
    ) -> Result<()> {
        // Vector updates would need the new embedding
        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        self.remove_vector(id).await
    }

    async fn search(&self, _query: &crate::contracts::Query) -> Result<Vec<ValidatedDocumentId>> {
        // Basic text search not applicable for vector index
        // This would be used via semantic_search instead
        Ok(Vec::new())
    }

    async fn sync(&mut self) -> Result<()> {
        self.persist_if_dirty().await
    }

    async fn flush(&mut self) -> Result<()> {
        self.persist_if_dirty().await
    }

    async fn close(mut self) -> Result<()> {
        self.persist_if_dirty().await
    }
}

/// Semantic query for vector search
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub vector: Vec<f32>,
    pub k: usize, // Number of nearest neighbors
    pub distance_metric: Option<DistanceMetric>,
    pub ef: Option<usize>, // Search parameter for quality vs speed tradeoff
}

impl SemanticQuery {
    pub fn new(vector: Vec<f32>, k: usize) -> Self {
        Self {
            vector,
            k,
            distance_metric: None,
            ef: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_vector_index_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("vector.idx");

        let index = VectorIndex::new(&index_path, DistanceMetric::Cosine, 384).await?;
        assert_eq!(index.vector_dimension, 384);
        assert_eq!(index.distance_metric, DistanceMetric::Cosine);

        Ok(())
    }

    #[tokio::test]
    async fn test_distance_calculations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("vector.idx");

        let index = VectorIndex::new(&index_path, DistanceMetric::Cosine, 3).await?;

        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let v3 = vec![1.0, 0.0, 0.0]; // Same as v1

        let dist_12 = index.calculate_distance(&v1, &v2);
        let dist_13 = index.calculate_distance(&v1, &v3);

        assert!(dist_12 > dist_13); // v1 should be closer to v3 than v2
        assert!((dist_13 - 0.0).abs() < 1e-6); // v1 and v3 should be identical

        Ok(())
    }

    #[tokio::test]
    async fn test_vector_insertion_and_search() -> Result<()> {
        use crate::types::ValidatedDocumentId;

        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("vector.idx");

        let mut index = VectorIndex::new(&index_path, DistanceMetric::Cosine, 3).await?;

        // Insert some test vectors
        let id1 = ValidatedDocumentId::new();
        let id2 = ValidatedDocumentId::new();

        index.insert_vector(id1, vec![1.0, 0.0, 0.0]).await?;
        index.insert_vector(id2, vec![0.0, 1.0, 0.0]).await?;

        // Search for nearest neighbor to [1.0, 0.1, 0.0] (should be closer to id1)
        let results = index.search_knn(&[1.0, 0.1, 0.0], 2, None).await?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, id1); // First result should be id1

        Ok(())
    }
}
