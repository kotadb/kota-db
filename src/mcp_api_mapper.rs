//! Simple API field mapper for MCP requests (aligned with kotadb:// methods)
//!
//! Maps structured operations to MCP tool calls without NLP, using stable
//! method names exposed by `MCPToolRegistry` implementations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::mcp::tools::MCPToolRegistry;

/// Simple API request structure that maps directly to MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub method: String,    // e.g., "kotadb://text_search"
    pub parameters: Value, // tool-specific parameters
    pub trace_id: String,
}

/// Supported MCP operations - aligned with KotaDB tool methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum MCPOperation {
    #[serde(rename = "search_content")]
    SearchContent {
        query: String,
        limit: Option<usize>,
        offset: Option<usize>,
    },

    // Not currently exposed as an MCP tool; prefer API/CLI for now
    #[serde(rename = "search_symbols")]
    SearchSymbols {
        pattern: String,
        symbol_types: Option<Vec<String>>,
    },

    #[serde(rename = "find_callers")]
    FindCallers { target: String },

    #[serde(rename = "analyze_impact")]
    AnalyzeImpact { target: String },

    #[serde(rename = "get_overview")]
    GetOverview {
        focus: Option<String>,
        detailed: bool,
    },
}

/// Simple, reliable API mapper - no complex parsing or NLP
pub struct MCPApiMapper {
    tool_registry: std::sync::Arc<MCPToolRegistry>,
}

impl MCPApiMapper {
    pub fn new(tool_registry: std::sync::Arc<MCPToolRegistry>) -> Self {
        Self { tool_registry }
    }

    /// Convert structured operation directly to MCP request (kotadb:// methods)
    pub fn operation_to_request(&self, operation: MCPOperation) -> Result<MCPRequest> {
        let trace_id = Uuid::new_v4().to_string();

        let (method, parameters) = match operation {
            MCPOperation::SearchContent {
                query,
                limit,
                offset,
            } => (
                "kotadb://text_search",
                json!({ "query": query, "limit": limit.unwrap_or(10), "offset": offset.unwrap_or(0) }),
            ),
            MCPOperation::SearchSymbols {
                pattern,
                symbol_types,
            } => ("kotadb://symbol_search", {
                // Best-effort mapping: take first type if provided
                let sym_type = symbol_types.and_then(|v| v.into_iter().next());
                json!({ "pattern": pattern, "symbol_type": sym_type })
            }),
            MCPOperation::FindCallers { target } => {
                ("kotadb://find_callers", json!({ "symbol": target }))
            }
            MCPOperation::AnalyzeImpact { target } => {
                ("kotadb://impact_analysis", json!({ "symbol": target }))
            }
            MCPOperation::GetOverview { focus, detailed } => (
                "kotadb://codebase_overview",
                json!({ "focus": focus, "detailed": detailed }),
            ),
        };

        Ok(MCPRequest {
            method: method.to_string(),
            parameters,
            trace_id,
        })
    }

    /// Execute the MCP request and return the response
    pub async fn execute_request(&self, request: MCPRequest) -> Result<Value> {
        self.tool_registry
            .handle_tool_call(&request.method, request.parameters)
            .await
            .context("Failed to execute MCP tool")
    }

    /// Convenience method: operation -> request -> execution in one call
    pub async fn execute_operation(&self, operation: MCPOperation) -> Result<Value> {
        let request = self.operation_to_request(operation)?;
        self.execute_request(request).await
    }

    /// Parse simple structured requests from JSON
    pub fn parse_json_request(&self, json_str: &str) -> Result<MCPOperation> {
        serde_json::from_str(json_str).context("Failed to parse JSON request into MCPOperation")
    }

    /// Convert simple key-value parameters to operations
    pub fn params_to_operation(&self, params: HashMap<String, String>) -> Result<MCPOperation> {
        let operation_type = params
            .get("operation")
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation' parameter"))?;

        match operation_type.as_str() {
            "search_content" => {
                let query = params
                    .get("query")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter for search_content"))?
                    .clone();
                let limit = params.get("limit").and_then(|s| s.parse().ok());
                let offset = params.get("offset").and_then(|s| s.parse().ok());
                Ok(MCPOperation::SearchContent {
                    query,
                    limit,
                    offset,
                })
            }
            "search_symbols" => {
                let pattern = params
                    .get("pattern")
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing 'pattern' parameter for search_symbols")
                    })?
                    .clone();
                let symbol_types = params
                    .get("symbol_types")
                    .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());
                Ok(MCPOperation::SearchSymbols {
                    pattern,
                    symbol_types,
                })
            }
            "find_callers" => {
                let target = params
                    .get("target")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'target' parameter for find_callers"))?
                    .clone();
                Ok(MCPOperation::FindCallers { target })
            }
            "analyze_impact" => {
                let target = params
                    .get("target")
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing 'target' parameter for analyze_impact")
                    })?
                    .clone();
                Ok(MCPOperation::AnalyzeImpact { target })
            }
            "get_overview" => {
                let focus = params.get("focus").cloned();
                let detailed = params
                    .get("detailed")
                    .map(|s| s.parse().unwrap_or(false))
                    .unwrap_or(false);
                Ok(MCPOperation::GetOverview { focus, detailed })
            }
            _ => Err(anyhow::anyhow!(
                "Unknown operation type: {}",
                operation_type
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_mapper() -> MCPApiMapper {
        let tool_registry = Arc::new(MCPToolRegistry::new());
        MCPApiMapper::new(tool_registry)
    }

    #[test]
    fn test_search_content_operation_maps_to_text_search() {
        let mapper = create_test_mapper();

        let operation = MCPOperation::SearchContent {
            query: "async fn".to_string(),
            limit: Some(10),
            offset: Some(2),
        };
        let request = mapper.operation_to_request(operation).unwrap();

        assert_eq!(request.method, "kotadb://text_search");
        assert_eq!(request.parameters["query"], "async fn");
        assert_eq!(request.parameters["limit"], 10);
        assert_eq!(request.parameters["offset"], 2);
        assert!(!request.trace_id.is_empty());
    }

    #[test]
    fn test_params_to_operation_and_json_parse() {
        let mapper = create_test_mapper();

        let mut params = HashMap::new();
        params.insert("operation".to_string(), "find_callers".to_string());
        params.insert("target".to_string(), "FileStorage::new".to_string());

        let op = mapper.params_to_operation(params).unwrap();
        match op {
            MCPOperation::FindCallers { target } => assert_eq!(target, "FileStorage::new"),
            _ => panic!("Expected FindCallers"),
        }

        let json_request =
            r#"{ "operation": "get_overview", "focus": "storage", "detailed": true }"#;
        let op2 = mapper.parse_json_request(json_request).unwrap();
        match op2 {
            MCPOperation::GetOverview { focus, detailed } => {
                assert_eq!(focus, Some("storage".to_string()));
                assert!(detailed);
            }
            _ => panic!("Expected GetOverview"),
        }
    }
}
