#[cfg(feature = "tree-sitter-parsing")]
use crate::services::search_service::{DatabaseAccess, SearchService, SymbolSearchOptions};
use crate::types::ValidatedDocumentId;
use crate::{
    contracts::{Index, Storage},
    mcp::tools::MCPToolHandler,
};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// MCP symbol search request
#[derive(Debug, Deserialize)]
struct SymbolSearchRequest {
    pattern: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    symbol_type: Option<String>,
}

/// Symbol search tools for MCP
pub struct SymbolTools {
    storage: Arc<Mutex<dyn Storage>>,
    primary_index: Arc<Mutex<dyn Index>>,
    trigram_index: Arc<Mutex<dyn Index>>,
    db_path: PathBuf,
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl SymbolTools {
    pub fn new(
        storage: Arc<Mutex<dyn Storage>>,
        primary_index: Arc<Mutex<dyn Index>>,
        trigram_index: Arc<Mutex<dyn Index>>,
        db_path: PathBuf,
    ) -> Self {
        Self {
            storage,
            primary_index,
            trigram_index,
            db_path,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// Implement DatabaseAccess for SearchService
impl DatabaseAccess for SymbolTools {
    fn storage(&self) -> Arc<Mutex<dyn Storage>> {
        self.storage.clone()
    }
    fn primary_index(&self) -> Arc<Mutex<dyn Index>> {
        self.primary_index.clone()
    }
    fn trigram_index(&self) -> Arc<Mutex<dyn Index>> {
        self.trigram_index.clone()
    }
    fn path_cache(&self) -> Arc<RwLock<HashMap<String, ValidatedDocumentId>>> {
        self.path_cache.clone()
    }
}

#[async_trait::async_trait]
impl MCPToolHandler for SymbolTools {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "kotadb://symbol_search" => {
                let request: SymbolSearchRequest = serde_json::from_value(params)?;

                // Build options with reasonable defaults
                let options = SymbolSearchOptions {
                    pattern: request.pattern,
                    limit: request.limit.unwrap_or(25).min(100),
                    symbol_type: request.symbol_type,
                    quiet: true,
                };

                let service = SearchService::new(self, self.db_path.clone());
                let result = service.search_symbols(options).await?;
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(anyhow::anyhow!("Unknown symbol method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<crate::mcp::types::ToolDefinition> {
        vec![crate::mcp::types::ToolDefinition {
            name: "kotadb://symbol_search".to_string(),
            description: "Search symbols by name with optional wildcard and type filters"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Symbol name pattern (supports '*' wildcard)" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 25 },
                    "symbol_type": { "type": "string", "description": "Optional symbol type filter (Function, Struct, Class, etc.)" }
                },
                "required": ["pattern"]
            }),
        }]
    }
}
