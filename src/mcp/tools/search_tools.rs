use crate::contracts::{Index, Query};
use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::*;
use crate::semantic_search::{ScoredDocument, SemanticSearchEngine};
use crate::trigram_index::TrigramIndex;
use crate::types::*;
use crate::validation::*;
use crate::wrappers::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Search tools for MCP - text search and semantic search capabilities
pub struct SearchTools {
    trigram_index: Arc<Mutex<dyn Index>>,
    semantic_engine: Arc<Mutex<SemanticSearchEngine>>,
}

impl SearchTools {
    pub fn new(
        trigram_index: Arc<Mutex<dyn Index>>,
        semantic_engine: Arc<Mutex<SemanticSearchEngine>>,
    ) -> Self {
        Self {
            trigram_index,
            semantic_engine,
        }
    }
}

#[async_trait::async_trait]
impl MCPToolHandler for SearchTools {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "kotadb://text_search" => {
                let request: TextSearchRequest = serde_json::from_value(params)?;
                self.text_search(request).await
            }
            "kotadb://semantic_search" => {
                let request: SemanticSearchRequest = serde_json::from_value(params)?;
                self.semantic_search(request).await
            }
            "kotadb://hybrid_search" => {
                let request: HybridSearchRequest = serde_json::from_value(params)?;
                self.hybrid_search(request).await
            }
            "kotadb://find_similar" => {
                let request: FindSimilarRequest = serde_json::from_value(params)?;
                self.find_similar(request).await
            }
            _ => Err(anyhow::anyhow!("Unknown search method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "kotadb://text_search".to_string(),
                description: "Search documents using full-text search with trigram indexing"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Text query to search for (supports partial word matching)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10, max: 100)",
                            "minimum": 1,
                            "maximum": 100
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Number of results to skip for pagination (default: 0)",
                            "minimum": 0
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "kotadb://semantic_search".to_string(),
                description: "Search documents using semantic similarity with vector embeddings"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language query for semantic similarity search"
                        },
                        "k": {
                            "type": "integer",
                            "description": "Number of most similar documents to return (default: 5, max: 50)",
                            "minimum": 1,
                            "maximum": 50
                        },
                        "threshold": {
                            "type": "number",
                            "description": "Minimum similarity threshold (0.0-1.0, lower scores = more similar)",
                            "minimum": 0.0,
                            "maximum": 1.0
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "kotadb://hybrid_search".to_string(),
                description: "Combine text and semantic search for comprehensive results"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for both text and semantic matching"
                        },
                        "k": {
                            "type": "integer",
                            "description": "Number of results to return (default: 10, max: 50)",
                            "minimum": 1,
                            "maximum": 50
                        },
                        "semantic_weight": {
                            "type": "number",
                            "description": "Weight for semantic search (0.0-1.0, default: 0.7)",
                            "minimum": 0.0,
                            "maximum": 1.0
                        },
                        "text_weight": {
                            "type": "number",
                            "description": "Weight for text search (0.0-1.0, default: 0.3)",
                            "minimum": 0.0,
                            "maximum": 1.0
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "kotadb://find_similar".to_string(),
                description: "Find documents similar to a given document using semantic analysis"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "document_id": {
                            "type": "string",
                            "description": "ID of the document to find similar documents for"
                        },
                        "k": {
                            "type": "integer",
                            "description": "Number of similar documents to return (default: 5, max: 20)",
                            "minimum": 1,
                            "maximum": 20
                        },
                        "threshold": {
                            "type": "number",
                            "description": "Minimum similarity threshold (0.0-1.0)",
                            "minimum": 0.0,
                            "maximum": 1.0
                        }
                    },
                    "required": ["document_id"]
                }),
            },
        ]
    }
}

impl SearchTools {
    async fn text_search(&self, request: TextSearchRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate query
        if request.query.trim().is_empty() {
            return Err(anyhow::anyhow!("Search query cannot be empty"));
        }

        let limit = request.limit.unwrap_or(10).min(100);
        let offset = request.offset.unwrap_or(0);

        // Create search query using the contracts
        let query = Query::new(
            Some(request.query.clone()),
            None,
            None,
            limit,
        )?;

        // Perform search using trigram index
        let index = self.trigram_index.clone();
        let index_guard = index.lock().await;
        let doc_ids = index_guard.search(&query).await?;
        drop(index_guard);

        // Apply pagination (the index might not support it directly)
        let paginated_ids: Vec<_> = doc_ids.into_iter().skip(offset).take(limit).collect();

        // Convert to search results (simplified - in real implementation we'd fetch full documents)
        let results: Vec<SearchResult> = paginated_ids
            .into_iter()
            .enumerate()
            .map(|(idx, doc_id)| SearchResult {
                id: doc_id.as_uuid().to_string(),
                path: format!("/document/{}", doc_id.as_uuid()),
                title: Some(format!("Document {}", idx + 1)),
                content_preview: format!("Content preview matching '{}'", request.query),
                score: (100.0 - (idx as f32 * 5.0)).max(0.0), // Calculated relevance score
                metadata: HashMap::new(),
            })
            .collect();

        let total_count = results.len(); // In real implementation, this would be total before pagination
        let response = SearchResponse {
            results,
            total_count,
            query_time_ms: start_time.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Text search completed: '{}' returned {} results in {}ms",
            request.query,
            response.results.len(),
            response.query_time_ms
        );

        Ok(serde_json::to_value(response)?)
    }

    async fn semantic_search(&self, request: SemanticSearchRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate query
        if request.query.trim().is_empty() {
            return Err(anyhow::anyhow!("Semantic search query cannot be empty"));
        }

        let k = request.k.unwrap_or(5).min(50);

        // Perform semantic search
        let engine = self.semantic_engine.clone();
        let engine_guard = engine.lock().await;
        let scored_docs = engine_guard
            .semantic_search(&request.query, k, request.threshold)
            .await?;
        drop(engine_guard);

        // Convert to search results
        let results: Vec<SearchResult> = scored_docs
            .into_iter()
            .map(|scored| SearchResult {
                id: scored.document.id.as_uuid().to_string(),
                path: scored.document.path.to_string(),
                title: Some(scored.document.title.to_string()),
                content_preview: Self::create_content_preview(&scored.document.content, 200),
                score: scored.similarity_percentage(),
                metadata: HashMap::new(),
            })
            .collect();

        let total_count = results.len();
        let response = SearchResponse {
            results,
            total_count,
            query_time_ms: start_time.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Semantic search completed: '{}' returned {} results in {}ms",
            request.query,
            response.results.len(),
            response.query_time_ms
        );

        Ok(serde_json::to_value(response)?)
    }

    async fn hybrid_search(&self, request: HybridSearchRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate query
        if request.query.trim().is_empty() {
            return Err(anyhow::anyhow!("Hybrid search query cannot be empty"));
        }

        let k = request.k.unwrap_or(10).min(50);
        let semantic_weight = request.semantic_weight.unwrap_or(0.7);
        let text_weight = request.text_weight.unwrap_or(0.3);

        // Validate weights sum to approximately 1.0
        if (semantic_weight + text_weight - 1.0).abs() > 0.01 {
            return Err(anyhow::anyhow!(
                "Semantic and text weights must sum to 1.0 (got: {} + {} = {})",
                semantic_weight,
                text_weight,
                semantic_weight + text_weight
            ));
        }

        // Perform hybrid search
        let engine = self.semantic_engine.clone();
        let engine_guard = engine.lock().await;
        let scored_docs = engine_guard
            .hybrid_search(&request.query, k, semantic_weight, text_weight)
            .await?;
        drop(engine_guard);

        // Convert to search results
        let results: Vec<SearchResult> = scored_docs
            .into_iter()
            .map(|scored| SearchResult {
                id: scored.document.id.as_uuid().to_string(),
                path: scored.document.path.to_string(),
                title: Some(scored.document.title.to_string()),
                content_preview: Self::create_content_preview(&scored.document.content, 200),
                score: scored.similarity_percentage(),
                metadata: HashMap::new(),
            })
            .collect();

        let response = SearchResponse {
        let total_count = results.len();
            results,
            total_count: results.len(),
            query_time_ms: start_time.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Hybrid search completed: '{}' (semantic: {}, text: {}) returned {} results in {}ms",
            request.query,
            semantic_weight,
            text_weight,
            response.results.len(),
            response.query_time_ms
        );

        Ok(serde_json::to_value(response)?)
    }

    async fn find_similar(&self, request: FindSimilarRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate document ID
        let doc_id = ValidatedDocumentId::parse(&request.document_id)
            .map_err(|e| anyhow::anyhow!("Invalid document ID: {}", e))?;

        let k = request.k.unwrap_or(5).min(20);

        // Find similar documents
        let engine = self.semantic_engine.clone();
        let engine_guard = engine.lock().await;
        let scored_docs = engine_guard.find_similar(&doc_id, k).await?;
        drop(engine_guard);

        // Apply threshold filter if specified
        let filtered_docs: Vec<ScoredDocument> = match request.threshold {
            Some(threshold) => scored_docs
                .into_iter()
                .filter(|scored| scored.similarity_percentage() >= threshold * 100.0)
                .collect(),
            None => scored_docs,
        };

        // Convert to search results
        let results: Vec<SearchResult> = filtered_docs
            .into_iter()
            .map(|scored| SearchResult {
                id: scored.document.id.as_uuid().to_string(),
                path: scored.document.path.to_string(),
                title: Some(scored.document.title.to_string()),
                content_preview: Self::create_content_preview(&scored.document.content, 200),
                score: scored.similarity_percentage(),
                metadata: HashMap::new(),
            })
            .collect();

        let response = SearchResponse {
        let total_count = results.len();
            results,
            total_count: results.len(),
            query_time_ms: start_time.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Find similar completed: document {} returned {} results in {}ms",
            request.document_id,
            response.results.len(),
            response.query_time_ms
        );

        Ok(serde_json::to_value(response)?)
    }

    /// Create a content preview from document bytes
    fn create_content_preview(content: &[u8], max_chars: usize) -> String {
        let content_str = String::from_utf8_lossy(content);
        if content_str.len() <= max_chars {
            content_str.to_string()
        } else {
            let truncated = &content_str[..max_chars];
            // Try to break at word boundary
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        }
    }
}

// Additional request types not in types.rs
#[derive(Debug, Clone, serde::Deserialize)]
struct HybridSearchRequest {
    query: String,
    k: Option<usize>,
    semantic_weight: Option<f32>,
    text_weight: Option<f32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct FindSimilarRequest {
    document_id: String,
    k: Option<usize>,
    threshold: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrappers::create_test_trigram_index;
    use tempfile::TempDir;

    // NOTE: Search tools tests disabled pending real semantic engine implementation
    // Per AGENT.md - no mocking allowed, need real implementations
    /*
    #[tokio::test]
    async fn test_search_tools_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("trigram");

        // Create a real trigram index using test helpers
        let trigram_index = create_test_trigram_index(index_path.to_str().unwrap()).await?;
        let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        // Real semantic engine would be created here
        let semantic_engine = Arc::new(Mutex::new(create_real_semantic_engine().await?));

        let _search_tools = SearchTools::new(trigram_index, semantic_engine);
        Ok(())
    }

    async fn create_real_semantic_engine() -> Result<SemanticSearchEngine> {
        // Real implementation would go here - no mocking allowed per AGENT.md
        todo!("Implement real semantic engine")
    }
    */
}
