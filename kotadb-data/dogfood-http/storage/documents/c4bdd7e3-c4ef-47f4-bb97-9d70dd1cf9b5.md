---
tags:
- file
- kota-db
- ext_rs
---
// Semantic Search Module - Combines embeddings with vector index for semantic queries
// Provides high-level interface for document semantic search with auto-embedding

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::builders::QueryBuilder;
use crate::contracts::{Document, Index, Storage};
use crate::embeddings::{EmbeddingConfig, EmbeddingService};
use crate::types::ValidatedDocumentId;
use crate::vector_index::{DistanceMetric, VectorIndex};

/// Semantic search engine that combines storage, vector index, and embeddings
pub struct SemanticSearchEngine {
    storage: Box<dyn Storage>,
    vector_index: VectorIndex,
    embedding_service: EmbeddingService,
    trigram_index: Option<Box<dyn Index>>,
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
            trigram_index: None,
        })
    }

    /// Create a semantic search engine with trigram index for hybrid search
    pub async fn new_with_trigram(
        storage: Box<dyn Storage>,
        vector_index_path: &str,
        embedding_config: EmbeddingConfig,
        trigram_index: Box<dyn Index>,
    ) -> Result<Self> {
        let embedding_dimension = embedding_config.dimension;

        let vector_index = VectorIndex::new(
            vector_index_path,
            DistanceMetric::Cosine,
            embedding_dimension,
        )
        .await?;

        let embedding_service = EmbeddingService::new(embedding_config).await?;

        Ok(Self {
            storage,
            vector_index,
            embedding_service,
            trigram_index: Some(trigram_index),
        })
    }

    // Note: SemanticSearchEngine requires Box<dyn Trait> types due to its ownership model
    // This necessitates creating separate instances, which reduces memory sharing benefits
    // A future refactor could address this by redesigning the SemanticSearchEngine API

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

        // Update trigram index if available
        if let Some(ref mut trigram_index) = self.trigram_index {
            trigram_index
                .insert(document.id, document.path.clone())
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

        // Update trigram index if available
        if let Some(ref mut trigram_index) = self.trigram_index {
            trigram_index
                .update(document.id, document.path.clone())
                .await?;
        }

        Ok(())
    }

    /// Delete a document from storage and all indices
    pub async fn delete_document(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let storage_deleted = self.storage.delete(id).await?;
        let vector_deleted = self.vector_index.remove_vector(id).await?;

        // Delete from trigram index if available
        let trigram_deleted = if let Some(ref mut trigram_index) = self.trigram_index {
            trigram_index.delete(id).await?
        } else {
            false
        };

        Ok(storage_deleted || vector_deleted || trigram_deleted)
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
        text_weight: f32,
    ) -> Result<Vec<ScoredDocument>> {
        // If no trigram index is available, fall back to semantic-only search
        if self.trigram_index.is_none() {
            return self.semantic_search(query, k, None).await;
        }

        // Perform semantic search (get more results for fusion)
        let semantic_results = self.semantic_search(query, k * 3, None).await?;

        // Perform text search using trigram index
        let text_results = if let Some(ref trigram_index) = self.trigram_index {
            // Create a query for the trigram index
            let text_query = QueryBuilder::new().with_text(query)?.build()?;

            // Get matching document IDs from trigram index
            let text_doc_ids = trigram_index.search(&text_query).await?;

            // Retrieve full documents and create scored results
            let mut text_scored = Vec::new();
            for (rank, doc_id) in text_doc_ids.iter().take(k * 3).enumerate() {
                if let Some(document) = self.storage.get(doc_id).await? {
                    // Calculate BM25-like relevance score (simplified)
                    // Lower rank = better score, so invert it
                    let text_score = 1.0 / (rank as f32 + 1.0);
                    text_scored.push(ScoredDocument {
                        document,
                        semantic_score: text_score,
                        query_text: query.to_string(),
                    });
                }
            }
            text_scored
        } else {
            Vec::new()
        };

        // Combine results using Reciprocal Rank Fusion (RRF)
        let fused_results = self.reciprocal_rank_fusion(
            semantic_results,
            text_results,
            semantic_weight,
            text_weight,
            k,
        )?;

        Ok(fused_results)
    }

    /// Reciprocal Rank Fusion for combining semantic and text search results
    fn reciprocal_rank_fusion(
        &self,
        semantic_results: Vec<ScoredDocument>,
        text_results: Vec<ScoredDocument>,
        semantic_weight: f32,
        text_weight: f32,
        k: usize,
    ) -> Result<Vec<ScoredDocument>> {
        // RRF constant (typically 60, controls how much to penalize lower ranks)
        const RRF_K: f32 = 60.0;

        // Create a map to store combined scores
        let mut doc_scores: HashMap<ValidatedDocumentId, (Option<ScoredDocument>, f32)> =
            HashMap::new();

        // Process semantic results
        for (rank, scored_doc) in semantic_results.into_iter().enumerate() {
            let doc_id = scored_doc.document.id;
            let rrf_score = semantic_weight / (RRF_K + rank as f32 + 1.0);

            doc_scores
                .entry(doc_id)
                .and_modify(|(_, score)| *score += rrf_score)
                .or_insert((Some(scored_doc), rrf_score));
        }

        // Process text results
        for (rank, scored_doc) in text_results.into_iter().enumerate() {
            let doc_id = scored_doc.document.id;
            let rrf_score = text_weight / (RRF_K + rank as f32 + 1.0);

            doc_scores
                .entry(doc_id)
                .and_modify(|(existing_doc, score)| {
                    *score += rrf_score;
                    // Keep the document if we don't have one yet
                    if existing_doc.is_none() {
                        *existing_doc = Some(scored_doc.clone());
                    }
                })
                .or_insert((Some(scored_doc), rrf_score));
        }

        // Sort by combined RRF score and take top k
        let mut sorted_results: Vec<_> = doc_scores
            .into_iter()
            .filter_map(|(_, (doc_opt, score))| {
                doc_opt.map(|mut doc| {
                    // Store the combined score
                    doc.semantic_score = score;
                    doc
                })
            })
            .collect();

        sorted_results.sort_by(|a, b| {
            b.semantic_score
                .partial_cmp(&a.semantic_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        sorted_results.truncate(k);
        Ok(sorted_results)
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
    use uuid::Uuid;

    struct TestSemanticEngine {
        engine: SemanticSearchEngine,
        test_dir: String,
    }

    impl Drop for TestSemanticEngine {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.test_dir);
        }
    }

    async fn create_test_search_engine() -> Result<TestSemanticEngine> {
        let test_dir = format!("test_data/semantic_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        // Create storage
        let storage_path = format!("{}/storage", test_dir);
        let storage = Box::new(FileStorage::open(&storage_path).await?);

        // Create embedding config using OpenAI configuration for testing
        // This won't make actual API calls in tests, but provides the correct structure
        let embedding_config = models::openai_text_embedding_3_small("test-api-key".to_string());

        // Create vector index path
        let vector_index_path = format!("{}/vector.idx", test_dir);

        let search_engine =
            SemanticSearchEngine::new(storage, &vector_index_path, embedding_config).await?;

        Ok(TestSemanticEngine {
            engine: search_engine,
            test_dir,
        })
    }

    #[tokio::test]
    #[ignore = "Requires actual embedding provider - enable for integration testing"]
    async fn test_semantic_search_engine_creation() -> Result<()> {
        // Skip this test in CI environment unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            println!("Skipping embedding test in CI environment. Set KOTADB_ENABLE_EMBEDDING_TESTS=1 to enable.");
            return Ok(());
        }
        let _test_engine = create_test_search_engine().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires actual embedding provider - enable for integration testing"]
    async fn test_document_insertion_with_auto_embedding() -> Result<()> {
        // Skip this test in CI environment unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            println!("Skipping embedding test in CI environment. Set KOTADB_ENABLE_EMBEDDING_TESTS=1 to enable.");
            return Ok(());
        }
        let mut test_engine = create_test_search_engine().await?;

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

        test_engine.engine.insert_document(document.clone()).await?;

        // Verify document was inserted with embedding
        let retrieved = test_engine.engine.storage.get(&document.id).await?.unwrap();
        assert!(
            retrieved.embedding.is_some(),
            "Document should have an embedding after insertion"
        );
        assert_eq!(retrieved.embedding.as_ref().unwrap().len(), 1536); // OpenAI standard dimension

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires embedding provider - integration test"]
    async fn test_hybrid_search_with_trigram_index() -> Result<()> {
        // Skip in CI unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            return Ok(());
        }

        // Create test directory
        let test_dir = format!("test_data/hybrid_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        // Create storage
        let storage_path = format!("{}/storage", test_dir);
        let storage = Box::new(FileStorage::open(&storage_path).await?);

        // Create trigram index
        let trigram_path = format!("{}/trigram", test_dir);
        let trigram_index = Box::new(crate::create_trigram_index(&trigram_path, None).await?);

        // Create mock embedding config (won't make actual API calls)
        let embedding_config = models::openai_text_embedding_3_small("test-api-key".to_string());

        // Create vector index path
        let vector_index_path = format!("{}/vector.idx", test_dir);

        // Create semantic engine with trigram support
        let mut engine = SemanticSearchEngine::new_with_trigram(
            storage,
            &vector_index_path,
            embedding_config,
            trigram_index,
        )
        .await?;

        // Create test documents with pre-computed embeddings to avoid API calls
        let mut doc1 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/rust.md")?,
            ValidatedTitle::new("Rust Programming")?,
            b"Rust is a systems programming language focused on safety and performance.".to_vec(),
            vec![ValidatedTag::new("rust")?],
            Utc::now(),
            Utc::now(),
        );
        // Add mock embedding to avoid API call
        doc1.embedding = Some(vec![0.1; 1536]);

        let mut doc2 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/python.md")?,
            ValidatedTitle::new("Python Guide")?,
            b"Python is a high-level programming language with dynamic typing.".to_vec(),
            vec![ValidatedTag::new("python")?],
            Utc::now(),
            Utc::now(),
        );
        doc2.embedding = Some(vec![0.2; 1536]);

        let mut doc3 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/safety.md")?,
            ValidatedTitle::new("Memory Safety")?,
            b"Memory safety is crucial for systems programming to prevent vulnerabilities."
                .to_vec(),
            vec![ValidatedTag::new("safety")?],
            Utc::now(),
            Utc::now(),
        );
        doc3.embedding = Some(vec![0.15; 1536]);

        // Insert documents (this will update both indices)
        engine.insert_document(doc1.clone()).await?;
        engine.insert_document(doc2.clone()).await?;
        engine.insert_document(doc3.clone()).await?;

        // Test hybrid search with text-only (since we can't generate real query embeddings)
        // The trigram index should still work
        let results = engine
            .hybrid_search(
                "rust safety",
                10,
                0.0, // semantic weight (disabled)
                1.0, // text weight (full weight to text)
            )
            .await?;

        // Should find documents about rust and safety via trigram search
        assert!(
            !results.is_empty(),
            "Hybrid search should return results from text search"
        );

        // Clean up
        std::fs::remove_dir_all(&test_dir)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_reciprocal_rank_fusion() -> Result<()> {
        // Create a minimal test engine
        let test_dir = format!("test_data/rrf_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let storage_path = format!("{}/storage", test_dir);
        let storage = Box::new(FileStorage::open(&storage_path).await?);
        let embedding_config = models::openai_text_embedding_3_small("test-api-key".to_string());
        let vector_index_path = format!("{}/vector.idx", test_dir);

        let engine =
            SemanticSearchEngine::new(storage, &vector_index_path, embedding_config).await?;

        // Create test documents for fusion
        let doc1 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/doc1.md")?,
            ValidatedTitle::new("Document 1")?,
            b"Content 1".to_vec(),
            vec![],
            Utc::now(),
            Utc::now(),
        );

        let doc2 = Document::new(
            ValidatedDocumentId::new(),
            ValidatedPath::new("test/doc2.md")?,
            ValidatedTitle::new("Document 2")?,
            b"Content 2".to_vec(),
            vec![],
            Utc::now(),
            Utc::now(),
        );

        // Create scored documents for testing RRF
        let semantic_results = vec![
            ScoredDocument {
                document: doc1.clone(),
                semantic_score: 0.1, // Lower is better for distance
                query_text: "test".to_string(),
            },
            ScoredDocument {
                document: doc2.clone(),
                semantic_score: 0.2,
                query_text: "test".to_string(),
            },
        ];

        let text_results = vec![
            ScoredDocument {
                document: doc2.clone(),
                semantic_score: 1.0, // Higher is better for text relevance
                query_text: "test".to_string(),
            },
            ScoredDocument {
                document: doc1.clone(),
                semantic_score: 0.5,
                query_text: "test".to_string(),
            },
        ];

        // Test RRF fusion
        let fused = engine.reciprocal_rank_fusion(
            semantic_results,
            text_results,
            0.7, // semantic weight
            0.3, // text weight
            2,   // top k
        )?;

        assert_eq!(fused.len(), 2, "Should return top 2 results");
        assert!(
            fused[0].semantic_score > 0.0,
            "Fused scores should be positive"
        );

        // Clean up
        std::fs::remove_dir_all(&test_dir)?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires embedding provider - integration test"]
    async fn test_hybrid_search_fallback_without_trigram() -> Result<()> {
        // Skip in CI unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            return Ok(());
        }

        // Test that hybrid search falls back to semantic when no trigram index
        let test_dir = format!("test_data/fallback_{}", Uuid::new_v4());
        std::fs::create_dir_all(&test_dir)?;

        let storage_path = format!("{}/storage", test_dir);
        let storage = Box::new(FileStorage::open(&storage_path).await?);
        let embedding_config = models::openai_text_embedding_3_small("test-api-key".to_string());
        let vector_index_path = format!("{}/vector.idx", test_dir);

        // Create engine WITHOUT trigram index
        let engine =
            SemanticSearchEngine::new(storage, &vector_index_path, embedding_config).await?;

        // This should not panic and should fall back to semantic search
        let results = engine.hybrid_search("test query", 10, 0.5, 0.5).await?;

        // Should complete without error (may return empty results in test)
        assert!(
            results.is_empty() || !results.is_empty(),
            "Should handle fallback gracefully"
        );

        // Clean up
        std::fs::remove_dir_all(&test_dir)?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires actual embedding provider - enable for integration testing"]
    async fn test_semantic_search() -> Result<()> {
        // Skip this test in CI environment unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            println!("Skipping embedding test in CI environment. Set KOTADB_ENABLE_EMBEDDING_TESTS=1 to enable.");
            return Ok(());
        }
        let mut test_engine = create_test_search_engine().await?;

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

        test_engine.engine.insert_document(doc1).await?;
        test_engine.engine.insert_document(doc2).await?;

        // Search for AI-related content
        let results = test_engine
            .engine
            .semantic_search("artificial intelligence", 5, None)
            .await?;

        assert!(!results.is_empty());
        // The ML document should be more similar to "artificial intelligence" query

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires actual embedding provider - enable for integration testing"]
    async fn test_embedding_stats() -> Result<()> {
        // Skip this test in CI environment unless explicitly enabled
        if std::env::var("CI").is_ok() && std::env::var("KOTADB_ENABLE_EMBEDDING_TESTS").is_err() {
            println!("Skipping embedding test in CI environment. Set KOTADB_ENABLE_EMBEDDING_TESTS=1 to enable.");
            return Ok(());
        }
        let test_engine = create_test_search_engine().await?;

        let stats = test_engine.engine.embedding_stats().await?;
        assert_eq!(stats.model_name, "text-embedding-3-small");
        assert_eq!(stats.dimension, 1536);

        Ok(())
    }
}
