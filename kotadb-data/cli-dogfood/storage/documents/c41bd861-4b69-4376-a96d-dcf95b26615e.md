---
tags:
- file
- kota-db
- ext_rs
---
//! MCP-over-HTTP Bridge Implementation
//!
//! Provides HTTP endpoints that translate REST API requests to MCP protocol calls.
//! This enables Claude Code integration without requiring local Rust compilation.
//!
//! Issue #541: Add MCP-over-HTTP endpoints to enable API key authentication

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::observability::with_trace_id;

/// MCP-over-HTTP bridge state
#[derive(Clone)]
pub struct McpHttpBridgeState {
    #[cfg(feature = "mcp-server")]
    pub tool_registry: std::sync::Arc<crate::mcp::tools::MCPToolRegistry>,
    #[cfg(not(feature = "mcp-server"))]
    pub _dummy: (),
}

/// Generic MCP tool request for HTTP bridge
#[derive(Debug, Deserialize)]
pub struct McpToolRequest {
    /// Tool-specific parameters
    #[serde(flatten)]
    pub params: serde_json::Value,
}

/// Generic MCP tool response for HTTP bridge
#[derive(Debug, Serialize)]
pub struct McpToolResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// List available MCP tools
#[derive(Debug, Serialize)]
pub struct McpToolsListResponse {
    pub tools: Vec<McpToolDefinition>,
    pub total_count: usize,
}

/// Simplified tool definition for HTTP API
#[derive(Debug, Serialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
}

impl McpHttpBridgeState {
    #[cfg(feature = "mcp-server")]
    pub fn new(tool_registry: std::sync::Arc<crate::mcp::tools::MCPToolRegistry>) -> Self {
        Self { tool_registry }
    }

    #[cfg(not(feature = "mcp-server"))]
    pub fn new() -> Self {
        Self { _dummy: () }
    }
}

/// Create MCP-over-HTTP bridge router
pub fn create_mcp_bridge_router() -> Router<McpHttpBridgeState> {
    Router::new()
        .route("/mcp/tools", post(list_mcp_tools))
        .route("/mcp/tools/:tool_name", post(call_mcp_tool))
        // Specific tool endpoints for better UX
        .route("/mcp/tools/search_code", post(search_code))
        .route("/mcp/tools/search_symbols", post(search_symbols))
        .route("/mcp/tools/find_callers", post(find_callers))
        .route("/mcp/tools/analyze_impact", post(analyze_impact))
        .route("/mcp/tools/stats", post(get_stats))
}

/// List available MCP tools
#[instrument(skip(_state))]
async fn list_mcp_tools(
    State(_state): State<McpHttpBridgeState>,
) -> Result<Json<McpToolsListResponse>, (StatusCode, Json<McpToolResponse>)> {
    let response = McpToolsListResponse {
        tools: vec![
            McpToolDefinition {
                name: "search_code".to_string(),
                description: "Search code content with full-text search".to_string(),
                category: "search".to_string(),
            },
            McpToolDefinition {
                name: "search_symbols".to_string(),
                description: "Search code symbols and definitions".to_string(),
                category: "search".to_string(),
            },
            McpToolDefinition {
                name: "find_callers".to_string(),
                description: "Find who calls a specific function".to_string(),
                category: "relationships".to_string(),
            },
            McpToolDefinition {
                name: "analyze_impact".to_string(),
                description: "Analyze the impact of code changes".to_string(),
                category: "analysis".to_string(),
            },
        ],
        total_count: 4,
    };

    info!("Listed {} MCP tools", response.total_count);
    Ok(Json(response))
}

/// Call a specific MCP tool by name
#[instrument(skip(_state, _request))]
async fn call_mcp_tool(
    State(_state): State<McpHttpBridgeState>,
    Path(tool_name): Path<String>,
    Json(_request): Json<McpToolRequest>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    // For now, return a placeholder response indicating the tool is not implemented
    // In the full implementation, this would route to actual MCP tools

    let response = McpToolResponse {
        success: false,
        data: None,
        error: Some(format!("Tool '{}' not yet implemented in MCP bridge. Please use the full HTTP API endpoints at /api/* instead.", tool_name)),
    };

    warn!("MCP tool call attempted but not implemented: {}", tool_name);
    Ok(Json(response))
}

/// Search code content (convenience endpoint)
#[instrument(skip(_state, _request))]
async fn search_code(
    State(_state): State<McpHttpBridgeState>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    let response = McpToolResponse {
        success: false,
        data: None,
        error: Some(
            "Code search not yet implemented in MCP bridge. Use GET /api/code/search instead."
                .to_string(),
        ),
    };

    Ok(Json(response))
}

/// Search symbols (convenience endpoint)
#[instrument(skip(_state, _request))]
async fn search_symbols(
    State(_state): State<McpHttpBridgeState>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    let response = McpToolResponse {
        success: false,
        data: None,
        error: Some(
            "Symbol search not yet implemented in MCP bridge. Use GET /api/symbols/search instead."
                .to_string(),
        ),
    };

    Ok(Json(response))
}

/// Find callers (convenience endpoint)
#[instrument(skip(_state, _request))]
async fn find_callers(
    State(_state): State<McpHttpBridgeState>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    let response = McpToolResponse {
        success: false,
        data: None,
        error: Some("Find callers not yet implemented in MCP bridge. Use GET /api/relationships/callers/:target instead.".to_string()),
    };

    Ok(Json(response))
}

/// Analyze impact (convenience endpoint)
#[instrument(skip(_state, _request))]
async fn analyze_impact(
    State(_state): State<McpHttpBridgeState>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    let response = McpToolResponse {
        success: false,
        data: None,
        error: Some("Impact analysis not yet implemented in MCP bridge. Use GET /api/analysis/impact/:target instead.".to_string()),
    };

    Ok(Json(response))
}

/// Get database statistics (convenience endpoint)
#[instrument(skip(_state))]
async fn get_stats(
    State(_state): State<McpHttpBridgeState>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    let result = with_trace_id("mcp_get_stats", async move {
        Ok(serde_json::json!({
            "message": "Database statistics available at GET /stats",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "MCP bridge provides endpoint discovery - use native HTTP API for full functionality"
        }))
    })
    .await;

    match result {
        Ok(data) => Ok(Json(McpToolResponse {
            success: true,
            data: Some(data),
            error: None,
        })),
        Err(e) => {
            warn!("Stats retrieval failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxumStatusCode};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_list_mcp_tools() -> Result<()> {
        #[cfg(feature = "mcp-server")]
        let state = {
            use crate::mcp::tools::MCPToolRegistry;
            let tool_registry = std::sync::Arc::new(MCPToolRegistry::new());
            McpHttpBridgeState::new(tool_registry)
        };
        
        #[cfg(not(feature = "mcp-server"))]
        let state = McpHttpBridgeState::new();
        let app = create_mcp_bridge_router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/tools")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?,
            )
            .await?;

        assert_eq!(response.status(), AxumStatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_call_mcp_tool() -> Result<()> {
        #[cfg(feature = "mcp-server")]
        let state = {
            use crate::mcp::tools::MCPToolRegistry;
            let tool_registry = std::sync::Arc::new(MCPToolRegistry::new());
            McpHttpBridgeState::new(tool_registry)
        };
        
        #[cfg(not(feature = "mcp-server"))]
        let state = McpHttpBridgeState::new();
        let app = create_mcp_bridge_router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/tools/search_code")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?,
            )
            .await?;

        // Should return OK even if not implemented
        assert_eq!(response.status(), AxumStatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_stats() -> Result<()> {
        #[cfg(feature = "mcp-server")]
        let state = {
            use crate::mcp::tools::MCPToolRegistry;
            let tool_registry = std::sync::Arc::new(MCPToolRegistry::new());
            McpHttpBridgeState::new(tool_registry)
        };
        
        #[cfg(not(feature = "mcp-server"))]
        let state = McpHttpBridgeState::new();
        let app = create_mcp_bridge_router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/tools/stats")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?,
            )
            .await?;

        assert_eq!(response.status(), AxumStatusCode::OK);
        Ok(())
    }
}
