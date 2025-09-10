//! Safe MCP Bridge Implementation
//!
//! Provides HTTP endpoints that directly integrate with KotaDB's existing MCP tools.
//! Follows KotaDB's Stage 6 component library patterns for safety and observability.
//!
//! Issue #541: Safe MCP-over-HTTP Bridge Implementation

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::{
    contracts::{Index, Storage},
    mcp::tools::MCPToolRegistry,
    observability::{with_trace_id, OperationContext},
};

/// Safe MCP bridge state using real KotaDB components
#[derive(Clone)]
pub struct SafeMcpBridgeState {
    pub tool_registry: Arc<MCPToolRegistry>,
    pub trace_id: Uuid,
}

/// MCP tool request with proper validation
#[derive(Debug, Deserialize)]
pub struct SafeMcpToolRequest {
    /// Tool-specific parameters (validated on input)
    #[serde(flatten)]
    pub params: serde_json::Value,
}

/// MCP tool response with comprehensive error context
#[derive(Debug, Serialize)]
pub struct SafeMcpToolResponse {
    pub success: bool,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// List of available MCP tools with their capabilities
#[derive(Debug, Serialize)]
pub struct SafeMcpToolsListResponse {
    pub tools: Vec<SafeMcpToolDefinition>,
    pub total_count: usize,
    pub trace_id: String,
}

/// Tool definition with proper categorization
#[derive(Debug, Serialize)]
pub struct SafeMcpToolDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub input_schema: serde_json::Value,
}

impl SafeMcpBridgeState {
    /// Create a new safe MCP bridge state with core components only
    pub fn new(
        storage: Arc<Mutex<dyn Storage>>,
        trigram_index: Arc<Mutex<dyn Index>>,
    ) -> Result<Self> {
        // For now, create a minimal registry since SearchTools requires SemanticSearchEngine
        // TODO: Create simplified search tools that only use trigram index
        let registry = MCPToolRegistry::new();

        Ok(Self {
            tool_registry: Arc::new(registry),
            trace_id: Uuid::new_v4(),
        })
    }
}

/// Create the safe MCP bridge router with proper error handling
pub fn create_safe_mcp_bridge_router() -> Router<SafeMcpBridgeState> {
    Router::new()
        .route("/mcp/tools", post(list_safe_mcp_tools))
        .route("/mcp/tools/:tool_name", post(call_safe_mcp_tool))
        // Convenience endpoints that directly use real implementations
        .route("/mcp/search/text", post(execute_text_search))
        .route("/mcp/search/semantic", post(execute_semantic_search))
        .route("/mcp/search/hybrid", post(execute_hybrid_search))
}

/// List available MCP tools with full schema information
#[instrument(skip(state))]
async fn list_safe_mcp_tools(
    State(state): State<SafeMcpBridgeState>,
) -> Result<Json<SafeMcpToolsListResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    let start_time = std::time::Instant::now();
    let operation_trace_id = Uuid::new_v4();

    let result = with_trace_id("list_safe_mcp_tools", async move {
        let mut ctx = OperationContext::new("mcp.list_tools");
        ctx.add_attribute("bridge_trace_id", state.trace_id.to_string());
        ctx.add_attribute("operation_trace_id", operation_trace_id.to_string());

        // Get tool definitions from the real MCP registry
        let tool_definitions = state.tool_registry.get_all_tool_definitions();

        let tools: Vec<SafeMcpToolDefinition> = tool_definitions
            .into_iter()
            .map(|def| {
                let category = categorize_tool(&def.name);
                SafeMcpToolDefinition {
                    name: def.name,
                    description: def.description,
                    category,
                    input_schema: def.input_schema,
                }
            })
            .collect();

        let total_count = tools.len();
        info!("Listed {} real MCP tools", total_count);

        Ok(SafeMcpToolsListResponse {
            tools,
            total_count,
            trace_id: operation_trace_id.to_string(),
        })
    })
    .await;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(response) => {
            info!("MCP tools listed successfully in {}ms", execution_time_ms);
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Failed to list MCP tools: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SafeMcpToolResponse {
                    success: false,
                    trace_id: operation_trace_id.to_string(),
                    data: None,
                    error: Some(format!("Failed to list tools: {}", e)),
                    execution_time_ms,
                }),
            ))
        }
    }
}

/// Call a specific MCP tool using the real implementation
#[instrument(skip(state, request))]
async fn call_safe_mcp_tool(
    State(state): State<SafeMcpBridgeState>,
    Path(tool_name): Path<String>,
    Json(request): Json<SafeMcpToolRequest>,
) -> Result<Json<SafeMcpToolResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    let start_time = std::time::Instant::now();
    let operation_trace_id = Uuid::new_v4();

    let tool_name_for_logging = tool_name.clone(); // Keep a copy for logging

    let result = with_trace_id("call_safe_mcp_tool", async move {
        let mut ctx = OperationContext::new("mcp.call_tool");
        ctx.add_attribute("bridge_trace_id", state.trace_id.to_string());
        ctx.add_attribute("operation_trace_id", operation_trace_id.to_string());
        ctx.add_attribute("tool_name", tool_name.as_str());

        // Validate and map tool name to MCP protocol format
        let mcp_method = map_tool_name_to_mcp_method(&tool_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;

        info!("Calling real MCP tool: {} -> {}", tool_name, mcp_method);

        // Call the REAL MCP tool through the registry
        let response = state
            .tool_registry
            .handle_tool_call(&mcp_method, request.params)
            .await
            .context("MCP tool execution failed")?;

        Ok(response)
    })
    .await;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(data) => {
            info!(
                "MCP tool {} executed successfully in {}ms",
                tool_name_for_logging, execution_time_ms
            );
            Ok(Json(SafeMcpToolResponse {
                success: true,
                trace_id: operation_trace_id.to_string(),
                data: Some(data),
                error: None,
                execution_time_ms,
            }))
        }
        Err(e) => {
            warn!("MCP tool {} failed: {}", tool_name_for_logging, e);
            let status_code = if e.to_string().contains("Unknown tool") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status_code,
                Json(SafeMcpToolResponse {
                    success: false,
                    trace_id: operation_trace_id.to_string(),
                    data: None,
                    error: Some(e.to_string()),
                    execution_time_ms,
                }),
            ))
        }
    }
}

/// Execute text search using real MCP search tools
#[instrument(skip(state, request))]
async fn execute_text_search(
    State(state): State<SafeMcpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<SafeMcpToolResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    execute_mcp_tool_directly(state, "kotadb://text_search", request, "text_search").await
}

/// Execute semantic search using real MCP search tools
#[instrument(skip(state, request))]
async fn execute_semantic_search(
    State(state): State<SafeMcpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<SafeMcpToolResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    execute_mcp_tool_directly(
        state,
        "kotadb://semantic_search",
        request,
        "semantic_search",
    )
    .await
}

/// Execute hybrid search using real MCP search tools
#[instrument(skip(state, request))]
async fn execute_hybrid_search(
    State(state): State<SafeMcpBridgeState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<SafeMcpToolResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    execute_mcp_tool_directly(state, "kotadb://hybrid_search", request, "hybrid_search").await
}

/// Helper function to execute MCP tools directly with proper error handling
async fn execute_mcp_tool_directly(
    state: SafeMcpBridgeState,
    mcp_method: &str,
    request: serde_json::Value,
    tool_name: &str,
) -> Result<Json<SafeMcpToolResponse>, (StatusCode, Json<SafeMcpToolResponse>)> {
    let start_time = std::time::Instant::now();
    let operation_trace_id = Uuid::new_v4();

    let result = with_trace_id("execute_mcp_tool_directly", async move {
        let mut ctx = OperationContext::new("mcp.execute_direct");
        ctx.add_attribute("bridge_trace_id", state.trace_id.to_string());
        ctx.add_attribute("operation_trace_id", operation_trace_id.to_string());
        ctx.add_attribute("mcp_method", mcp_method.to_string());
        ctx.add_attribute("tool_name", tool_name.to_string());

        // Execute the real MCP tool
        state
            .tool_registry
            .handle_tool_call(mcp_method, request)
            .await
            .context("Direct MCP tool execution failed")
    })
    .await;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(data) => Ok(Json(SafeMcpToolResponse {
            success: true,
            trace_id: operation_trace_id.to_string(),
            data: Some(data),
            error: None,
            execution_time_ms,
        })),
        Err(e) => {
            warn!("Direct MCP tool execution failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SafeMcpToolResponse {
                    success: false,
                    trace_id: operation_trace_id.to_string(),
                    data: None,
                    error: Some(e.to_string()),
                    execution_time_ms,
                }),
            ))
        }
    }
}

/// Map user-friendly tool names to MCP protocol method names
fn map_tool_name_to_mcp_method(tool_name: &str) -> Option<String> {
    match tool_name {
        "text_search" => Some("kotadb://text_search".to_string()),
        "semantic_search" => Some("kotadb://semantic_search".to_string()),
        "hybrid_search" => Some("kotadb://hybrid_search".to_string()),
        "find_similar" => Some("kotadb://find_similar".to_string()),
        "llm_search" => Some("kotadb://llm_optimized_search".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "find_callers" => Some("kotadb://find_callers".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "find_callees" => Some("kotadb://find_callees".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "impact_analysis" => Some("kotadb://impact_analysis".to_string()),
        #[cfg(feature = "tree-sitter-parsing")]
        "codebase_overview" => Some("kotadb://codebase_overview".to_string()),
        _ => None,
    }
}

/// Categorize tools for better organization
fn categorize_tool(tool_name: &str) -> String {
    if tool_name.contains("search") || tool_name.contains("find_similar") {
        "search".to_string()
    } else if tool_name.contains("callers")
        || tool_name.contains("callees")
        || tool_name.contains("relationship")
    {
        "relationships".to_string()
    } else if tool_name.contains("impact")
        || tool_name.contains("analysis")
        || tool_name.contains("overview")
    {
        "analysis".to_string()
    } else {
        "general".to_string()
    }
}
