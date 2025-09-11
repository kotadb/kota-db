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
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

#[cfg(feature = "mcp-server")]
use crate::mcp::tools::MCPToolRegistry;
use crate::observability::with_trace_id;

/// MCP-over-HTTP bridge state
#[derive(Clone)]
pub struct McpHttpBridgeState {
    #[cfg(feature = "mcp-server")]
    pub tool_registry: Option<std::sync::Arc<MCPToolRegistry>>, // Optional: caller may not wire it
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
    pub error: Option<McpErrorPayload>,
}

#[derive(Debug, Serialize)]
pub struct McpErrorPayload {
    pub code: String,
    pub message: String,
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
    pub fn new(tool_registry: Option<std::sync::Arc<MCPToolRegistry>>) -> Self {
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
        .route("/mcp/tools", get(list_mcp_tools))
        .route("/mcp/tools", post(list_mcp_tools)) // Backward-compatible; prefer GET
        .route("/mcp/tools/:tool_name", post(call_mcp_tool))
        // Specific tool endpoints for better UX
        .route("/mcp/tools/search_code", post(search_code))
        .route("/mcp/tools/search_symbols", post(search_symbols))
        .route("/mcp/tools/find_callers", post(find_callers))
        .route("/mcp/tools/analyze_impact", post(analyze_impact))
        .route("/mcp/tools/stats", post(get_stats))
        .route("/mcp/tools/stats", get(get_stats))
}

/// List available MCP tools
#[instrument(skip(state))]
async fn list_mcp_tools(
    State(state): State<McpHttpBridgeState>,
) -> Result<Json<McpToolsListResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(feature = "mcp-server")]
    if let Some(registry) = state.tool_registry.clone() {
        let tools = registry
            .get_all_tool_definitions()
            .into_iter()
            .map(|t| McpToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                category: categorize_tool(&t.name),
            })
            .collect::<Vec<_>>();
        let response = McpToolsListResponse {
            total_count: tools.len(),
            tools,
        };
        info!("Listed {} MCP tools from registry", response.total_count);
        return Ok(Json(response));
    }

    // Fallback static list when registry is not available or feature disabled
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

    info!("Listed {} MCP tools (static)", response.total_count);
    Ok(Json(response))
}

fn categorize_tool(name: &str) -> String {
    if name.contains("symbol") || name.contains("search") {
        "search".into()
    } else if name.contains("caller") || name.contains("relationship") {
        "relationships".into()
    } else if name.contains("impact") || name.contains("analysis") {
        "analysis".into()
    } else {
        "general".into()
    }
}

/// Call a specific MCP tool by name
#[instrument(skip(state, request))]
async fn call_mcp_tool(
    State(state): State<McpHttpBridgeState>,
    Path(tool_name): Path<String>,
    Json(request): Json<McpToolRequest>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(not(feature = "mcp-server"))]
    {
        return Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some(McpErrorPayload {
                code: "feature_disabled".to_string(),
                message: "MCP server feature is disabled".to_string(),
            }),
        }));
    }

    #[cfg(feature = "mcp-server")]
    {
        let method = map_tool_name_to_mcp_method(&tool_name);
        let Some(method) = method else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "tool_not_found".to_string(),
                        message: format!("Unknown tool: {}", tool_name),
                    }),
                }),
            ));
        };

        let Some(registry) = state.tool_registry.clone() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "registry_unavailable".to_string(),
                        message: "MCP tool registry not configured".to_string(),
                    }),
                }),
            ));
        };

        match registry.handle_tool_call(&method, request.params).await {
            Ok(value) => Ok(Json(McpToolResponse {
                success: true,
                data: Some(value),
                error: None,
            })),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
                }),
            )),
        }
    }
}

/// Search code content (convenience endpoint)
#[instrument(skip(state, request))]
async fn search_code(
    State(state): State<McpHttpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(not(feature = "mcp-server"))]
    {
        return Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some(McpErrorPayload {
                code: "feature_disabled".to_string(),
                message: "MCP server feature is disabled".to_string(),
            }),
        }));
    }

    #[cfg(feature = "mcp-server")]
    {
        let Some(registry) = state.tool_registry.clone() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "registry_unavailable".to_string(),
                        message: "MCP tool registry not configured".to_string(),
                    }),
                }),
            ));
        };
        match registry
            .handle_tool_call("kotadb://text_search", request)
            .await
        {
            Ok(value) => Ok(Json(McpToolResponse {
                success: true,
                data: Some(value),
                error: None,
            })),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
                }),
            )),
        }
    }
}

/// Search symbols (convenience endpoint)
#[instrument(skip(state, request))]
async fn search_symbols(
    State(state): State<McpHttpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(not(feature = "mcp-server"))]
    {
        return Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some(McpErrorPayload {
                code: "feature_disabled".to_string(),
                message: "MCP server feature is disabled".to_string(),
            }),
        }));
    }

    #[cfg(feature = "mcp-server")]
    {
        let Some(registry) = state.tool_registry.clone() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "registry_unavailable".to_string(),
                        message: "MCP tool registry not configured".to_string(),
                    }),
                }),
            ));
        };

        match registry
            .handle_tool_call("kotadb://symbol_search", request)
            .await
        {
            Ok(value) => Ok(Json(McpToolResponse {
                success: true,
                data: Some(value),
                error: None,
            })),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
                }),
            )),
        }
    }
}

/// Find callers (convenience endpoint)
#[instrument(skip(state, request))]
async fn find_callers(
    State(state): State<McpHttpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(all(feature = "mcp-server", feature = "tree-sitter-parsing"))]
    {
        let Some(registry) = state.tool_registry.clone() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "registry_unavailable".to_string(),
                        message: "MCP tool registry not configured".to_string(),
                    }),
                }),
            ));
        };
        match registry
            .handle_tool_call("kotadb://find_callers", request)
            .await
        {
            Ok(value) => Ok(Json(McpToolResponse {
                success: true,
                data: Some(value),
                error: None,
            })),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
                }),
            )),
        }
    }

    #[cfg(not(all(feature = "mcp-server", feature = "tree-sitter-parsing")))]
    {
        // Fallback when relationship tools are not available
        Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(McpToolResponse {
                success: false,
                data: None,
                error: Some(McpErrorPayload {
                    code: "feature_disabled".to_string(),
                    message: "Relationship tools are not available".to_string(),
                }),
            }),
        ))
    }
}

/// Analyze impact (convenience endpoint)
#[instrument(skip(state, request))]
async fn analyze_impact(
    State(state): State<McpHttpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<McpToolResponse>, (StatusCode, Json<McpToolResponse>)> {
    #[cfg(all(feature = "mcp-server", feature = "tree-sitter-parsing"))]
    {
        let Some(registry) = state.tool_registry.clone() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "registry_unavailable".to_string(),
                        message: "MCP tool registry not configured".to_string(),
                    }),
                }),
            ));
        };
        match registry
            .handle_tool_call("kotadb://impact_analysis", request)
            .await
        {
            Ok(value) => Ok(Json(McpToolResponse {
                success: true,
                data: Some(value),
                error: None,
            })),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(McpToolResponse {
                    success: false,
                    data: None,
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
                }),
            )),
        }
    }

    #[cfg(not(all(feature = "mcp-server", feature = "tree-sitter-parsing")))]
    {
        // Fallback when relationship tools are not available
        Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(McpToolResponse {
                success: false,
                data: None,
                error: Some(McpErrorPayload {
                    code: "feature_disabled".to_string(),
                    message: "Relationship tools are not available".to_string(),
                }),
            }),
        ))
    }
}

/// Map user-friendly tool names to MCP protocol method names
#[allow(dead_code)]
fn map_tool_name_to_mcp_method(tool: &str) -> Option<String> {
    match tool {
        "search_code" | "text_search" => Some("kotadb://text_search".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "search_symbols" | "symbol_search" => Some("kotadb://symbol_search".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "find_callers" => Some("kotadb://find_callers".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "analyze_impact" | "impact_analysis" => Some("kotadb://impact_analysis".to_string()),
        _ => None,
    }
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
                    error: Some(McpErrorPayload {
                        code: "internal_error".to_string(),
                        message: e.to_string(),
                    }),
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
            McpHttpBridgeState::new(Some(tool_registry))
        };

        #[cfg(not(feature = "mcp-server"))]
        let state = McpHttpBridgeState::new();
        let app = create_mcp_bridge_router().with_state(state);
        let app2 = app.clone();

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

        // Also verify GET works
        let response_get = app2
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/mcp/tools")
                    .header("content-type", "application/json")
                    .body(Body::from(""))?,
            )
            .await?;
        assert_eq!(response_get.status(), AxumStatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_call_mcp_tool() -> Result<()> {
        #[cfg(feature = "mcp-server")]
        let state = {
            use crate::mcp::tools::MCPToolRegistry;
            let tool_registry = std::sync::Arc::new(MCPToolRegistry::new());
            McpHttpBridgeState::new(Some(tool_registry))
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

        // Depending on wiring, may be Not Implemented or Internal Server Error if tools not registered
        let status = response.status();
        assert!(
            status == AxumStatusCode::NOT_IMPLEMENTED
                || status == AxumStatusCode::INTERNAL_SERVER_ERROR
                || status == AxumStatusCode::OK,
            "unexpected status: {}",
            status
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_get_stats() -> Result<()> {
        #[cfg(feature = "mcp-server")]
        let state = {
            use crate::mcp::tools::MCPToolRegistry;
            let tool_registry = std::sync::Arc::new(MCPToolRegistry::new());
            McpHttpBridgeState::new(Some(tool_registry))
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
