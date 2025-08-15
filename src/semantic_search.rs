// Semantic Search Module - Combines embeddings with vector index for semantic queries
// Provides high-level interface for document semantic search with auto-embedding

use anyhow::{anyhow, Result};

use crate::contracts::{Document, Index, Storage};
use crate::embeddings::{EmbeddingConfig, EmbeddingService};
use crate::types::ValidatedDocumentId;
use crate::vector_index::{DistanceMetric, VectorIndex};

/// Semantic search engine that combines storage, vector index, and embeddings
pub struct SemanticSearchEngine {
    storage: Box<dyn Storage>,
    vector_index: VectorIndex,
    embedding_service: EmbeddingService,
}

impl SemanticSearchEngine {
    /// Create a new semantic search engine
    pub async fn new(
        storage: Box<dyn Storage>,
        vector_index_path: &str,
        embedding_config: EmbeddingConfig,
    ) -> Result<Self> {
        let embedding_dimension = embedding_config.dimension;

        let vector_index = VectorIndex::new(
            vector_index_path,
            DistanceMetric::Cosine, // Default to cosine similarity for semantic search
            embedding_dimension,
        )
        .await?;

        let embedding_service = EmbeddingService::new(embedding_config).await?;

        Ok(Self {
            storage,
            vector_index,
            embedding_service,
        })
    }

    /// Insert a document with automatic embedding generation
    pub async fn insert_document(&mut self, mut document: Document) -> Result<()> {
        // Generate embedding if not provided
        if document.embedding.is_none() {
            let content_text = self.extract_text_content(&document)?;
            let embedding = self.embedding_service.embed_text(&content_text).await?;
            document.embedding = Some(embedding);
        }

        // Insert document into storage
        self.storage.insert(document.clone()).await?;

        // Add vector to index if embedding exists
        if let Some(embedding) = &document.embedding {
            self.vector_index
                .insert_vector(document.id, embedding.clone())
                .await?;
        }

        Ok(())
    }

    /// Update a document with automatic re-embedding if content changed
    pub async fn update_document(&mut self, mut document: Document) -> Result<()> {
        // Get existing document to compare
        let existing = self.storage.get(&document.id).await?;

        let needs_reembedding = match &existing {
            Some(existing_doc) => {
                // Re-embed if content changed or no embedding exists
                existing_doc.content != document.content || document.embedding.is_none()
            }
            None => return Err(anyhow!("Document {} not found for update", document.id)),
        };

        // Generate new embedding if needed
        if needs_reembedding {
            let content_text = self.extract_text_content(&document)?;
            let embedding = self.embedding_service.embed_text(&content_text).await?;
            document.embedding = Some(embedding);
        }

        // Update document in storage
        self.storage.update(document.clone()).await?;

        // Update vector in index if embedding exists
        if let Some(embedding) = &document.embedding {
            self.vector_index
                .insert_vector(document.id, embedding.clone())
                .await?;
        }

        Ok(())
    }

    /// Delete a document from both storage and vector index
    pub async fn delete_document(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let storage_deleted = self.storage.delete(id).await?;
        let vector_deleted = self.vector_index.remove_vector(id).await?;

        Ok(storage_deleted || vector_deleted)
    }

    /// Perform semantic search using natural language query
    pub async fn semantic_search(
        &self,
        query: &str,
        k: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredDocument>> {
        // Generate embedding for the query
        let query_embedding = self.embedding_service.embed_text(query).await?;

        // Search vector index for similar embeddings
        let similar_docs = self
            .vector_index
            .search_knn(&query_embedding, k * 2, None) // Get more results to filter by threshold
            .await?;

        // Filter by score threshold if provided
        let filtered_docs: Vec<_> = match score_threshold {
            Some(threshold) => similar_docs
                .into_iter()
                .filter(|(_, score)| *score <= threshold) // Lower scores are better for distance metrics
                .collect(),
            None => similar_docs,
        };

        // Retrieve full documents and combine with scores
        let filtered_limited: Vec<_> = filtered_docs.into_iter().take(k).collect();
        let mut results = Vec::with_capacity(filtered_limited.len());
        for (doc_id, score) in filtered_limited {
            if let Some(document) = self.storage.get(&doc_id).await? {
                results.push(ScoredDocument {
                    document,
                    semantic_score: score,
                    query_text: query.to_string(),
                });
            }
        }

        Ok(results)
    }

    /// Perform hybrid search combining semantic and text search
    pub async fn hybrid_search(
        &self,
        query: &str,
        k: usize,
        semantic_weight: f32,
        _text_weight: f32,
    ) -> Result<Vec<ScoredDocument>> {
        // Perform semantic search
        let semantic_results = self.semantic_search(query, k * 2, None).await?;

        // TODO: Add text search integration with trigram index
        // For now, just return semantic results with adjusted scores
        let semantic_limited: Vec<_> = semantic_results.into_iter().take(k).collect();
        let mut hybrid_results = Vec::with_capacity(semantic_limited.len());
        for mut result in semantic_limited {
            // Apply semantic weight to score
            result.semantic_score *= semantic_weight;
            hybrid_results.push(result);
        }

        Ok(hybrid_results)
    }

    /// Find similar documents to a given document
    pub async fn find_similar(
        &self,
        document_id: &ValidatedDocumentId,
        k: usize,
    ) -> Result<Vec<ScoredDocument>> {
        // Get the document
        let document = self
            .storage
            .get(document_id)
            .await?
            .ok_or_else(|| anyhow!("Document {} not found", document_id))?;

        // Extract text content and search
        let content_text = self.extract_text_content(&document)?;
        self.semantic_search(&content_text, k + 1, None)
            .await
            .map(|results| {
                // Filter out the original document from results
                results
                    .into_iter()
                    .filter(|scored| scored.document.id != *document_id)
                    .take(k)
                    .collect()
            })
    }

    /// Get embeddings statistics
    pub async fn embedding_stats(&self) -> Result<EmbeddingStats> {
        let (cache_size, cache_capacity) = self.embedding_service.cache_stats().await;

        Ok(EmbeddingStats {
            model_name: self.embedding_service.model_name().to_string(),
            dimension: self.embedding_service.dimension(),
            cache_size,
            cache_capacity,
        })
    }

    /// Reindex all documents (useful when changing embedding models)
    pub async fn reindex_all(&mut self) -> Result<usize> {
        let documents = self.storage.list_all().await?;
        let mut reindexed_count = 0;

        for mut document in documents {
            // Generate new embedding
            let content_text = self.extract_text_content(&document)?;
            let embedding = self.embedding_service.embed_text(&content_text).await?;
            document.embedding = Some(embedding.clone());

            // Update storage with new embedding
            self.storage.update(document.clone()).await?;

            // Update vector index
            self.vector_index
                .insert_vector(document.id, embedding)
                .await?;

            reindexed_count += 1;
        }

        Ok(reindexed_count)
    }

    /// Extract text content from document for embedding
    fn extract_text_content(&self, document: &Document) -> Result<String> {
        // Try to convert content to string (assuming UTF-8)
        let content_str = String::from_utf8_lossy(&document.content);

        // Combine title and content for better semantic representation
        let full_text = format!("{}\n\n{}", document.title.as_str(), content_str);

        Ok(full_text)
    }

    /// Sync all components to disk
    pub async fn sync(&mut self) -> Result<()> {
        self.storage.sync().await?;
        self.vector_index.sync().await?;
        Ok(())
    }

    /// Close the search engine (syncs data before closing)
    pub async fn close(mut self) -> Result<()> {
        // Sync all data first
        self.sync().await?;
        // Vector index can be closed since it's owned
        self.vector_index.close().await?;
        Ok(())
    }
}

/// Document with semantic similarity score
#[derive(Debug, Clone)]
pub struct ScoredDocument {
    pub document: Document,
    pub semantic_score: f32,
    pub query_text: String,
}

impl ScoredDocument {
    /// Convert score to similarity percentage (higher is better)
    pub fn similarity_percentage(&self) -> f32 {
        // Convert distance to similarity (assuming cosine distance)
        (1.0 - self.semantic_score.max(0.0)).max(0.0) * 100.0
    }
}

/// Statistics about the embedding service
#[derive(Debug, Clone)]
pub struct EmbeddingStats {
    pub model_name: String,
    pub dimension: usize,
    pub cache_size: usize,
    pub cache_capacity: usize,
}

/// Configuration for hybrid search weights
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    pub semantic_weight: f32,
    pub text_weight: f32,
    pub score_threshold: Option<f32>,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.7,
            text_weight: 0.3,
            score_threshold: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::models;
    use crate::file_storage::FileStorage;
    use crate::types::{ValidatedPath, ValidatedTag, ValidatedTitle};
    use chrono::Utc;
    use tempfile::TempDir;

    async fn create_test_search_engine() -> Result<(SemanticSearchEngine, TempDir)> {
        let temp_dir = TempDir::new()?;

        // Create storage
        let storage_path = temp_dir.path().join("storage");
        let storage = Box::new(FileStorage::open(storage_path.to_str().unwrap()).await?);

        // Create embedding config (using local model for testing)
        let model_path = temp_dir.path().join("test_model.onnx");
        let embedding_config = models::local_minilm_l6_v2(model_path);

        // Create vector index path
        let vector_index_path = temp_dir.path().join("vector.idx");

        let search_engine = SemanticSearchEngine::new(
            storage,
            vector_index_path.to_str().unwrap(),
            embedding_config,
        )
        .await?;

        Ok((search_engine, temp_dir))
    }

    #[tokio::test]
    async fn test_semantic_search_engine_creation() -> Result<()> {
        let (_engine, _temp_dir) = create_test_search_engine().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_document_insertion_with_auto_embedding() -> Result<()> {
        let (mut engine, _temp_dir) = create_test_search_engine().await?;

        let document = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/doc.md").expect("Test path should be valid"),
            ValidatedTitle::new("Test Document")?,
            b"This is a test document about machine learning.".to_vec(),
            vec![ValidatedTag::new("test")?],
            Utc::now(),
            Utc::now(),
        );

        // Verify the document starts without embedding
        assert!(document.embedding.is_none());

        engine.insert_document(document.clone()).await?;

        // Verify document was inserted with embedding
        let retrieved = engine.storage.get(&document.id).await?.unwrap();
        assert!(
            retrieved.embedding.is_some(),
            "Document should have an embedding after insertion"
        );
        assert_eq!(retrieved.embedding.as_ref().unwrap().len(), 384); // MiniLM dimension

        Ok(())
    }

    #[tokio::test]
    async fn test_semantic_search() -> Result<()> {
        let (mut engine, _temp_dir) = create_test_search_engine().await?;

        // Insert test documents
        let doc1 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/ml.md").expect("Test path should be valid"),
            ValidatedTitle::new("Machine Learning Guide")?,
            b"Machine learning is a subset of artificial intelligence.".to_vec(),
            vec![],
            Utc::now(),
            Utc::now(),
        );

        let doc2 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/cooking.md").expect("Test path should be valid"),
            ValidatedTitle::new("Cooking Recipe")?,
            b"How to cook pasta with tomato sauce.".to_vec(),
            vec![],
            Utc::now(),
            Utc::now(),
        );

        engine.insert_document(doc1).await?;
        engine.insert_document(doc2).await?;

        // Search for AI-related content
        let results = engine
            .semantic_search("artificial intelligence", 5, None)
            .await?;

        assert!(!results.is_empty());
        // The ML document should be more similar to "artificial intelligence" query

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_stats() -> Result<()> {
        let (engine, _temp_dir) = create_test_search_engine().await?;

        let stats = engine.embedding_stats().await?;
        assert_eq!(stats.model_name, "all-MiniLM-L6-v2");
        assert_eq!(stats.dimension, 384);

        Ok(())
    }
}
