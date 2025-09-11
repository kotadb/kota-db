use crate::contracts::{Index, Query, Storage};
use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::ToolDefinition;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Lightweight Text Search Tools (no embeddings)
pub struct TextSearchTools {
    trigram_index: Arc<Mutex<dyn Index>>,
    storage: Arc<Mutex<dyn Storage>>,
}

impl TextSearchTools {
    pub fn new(trigram_index: Arc<Mutex<dyn Index>>, storage: Arc<Mutex<dyn Storage>>) -> Self {
        Self {
            trigram_index,
            storage,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TextSearchRequest {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[async_trait::async_trait]
impl MCPToolHandler for TextSearchTools {
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
            _ => Err(anyhow::anyhow!("Unknown text search method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "kotadb://text_search".to_string(),
            description: "Search documents using full-text trigram index (no embeddings)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100},
                    "offset": {"type": "integer", "minimum": 0}
                },
                "required": ["query"]
            }),
        }]
    }
}

impl TextSearchTools {
    async fn text_search(&self, request: TextSearchRequest) -> Result<serde_json::Value> {
        let start = Instant::now();

        if request.query.trim().is_empty() {
            return Ok(serde_json::json!({
                "success": true,
                "results": [],
                "total": 0,
                "query_time_ms": start.elapsed().as_millis() as u64
            }));
        }

        let limit = request.limit.unwrap_or(10).min(100);
        let offset = request.offset.unwrap_or(0);

        // Build query (text only, no tags/path for now)
        let query = Query::new(Some(request.query.clone()), None, None, limit + offset)?;

        // Search via trigram index
        let doc_ids = {
            let idx = self.trigram_index.lock().await;
            idx.search(&query).await?
        };

        // Apply pagination
        let ids_page: Vec<_> = doc_ids.into_iter().skip(offset).take(limit).collect();

        // Fetch docs and format response
        let mut results = Vec::new();
        let mut meta = HashMap::new();
        meta.insert("source".to_string(), "trigram".to_string());

        let storage = self.storage.lock().await;
        for doc_id in ids_page {
            if let Some(doc) = storage.get(&doc_id).await? {
                let content = String::from_utf8_lossy(&doc.content);
                let preview = if content.len() > 160 {
                    let cut = &content[..160];
                    (match cut.rfind(' ') {
                        Some(i) => cut[..i].to_string(),
                        None => cut.to_string(),
                    }) + "..."
                } else {
                    content.to_string()
                };

                results.push(serde_json::json!({
                    "id": doc_id.as_uuid().to_string(),
                    "path": doc.path.to_string(),
                    "title": doc.title.to_string(),
                    "content_preview": preview,
                    "metadata": meta,
                }));
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "results": results,
            "total": results.len(),
            "query_time_ms": start.elapsed().as_millis() as u64
        }))
    }
}
