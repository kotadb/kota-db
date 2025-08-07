use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Protocol Types
/// Based on Model Context Protocol specification

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Tool Definition for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Resource Definition for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
}

/// KotaDB-specific types for MCP tools

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCreateRequest {
    pub path: String,
    pub title: Option<String>,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCreateResponse {
    pub id: String,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGetRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGetResponse {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub k: Option<usize>,
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub content_preview: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: usize,
    pub query_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    pub metric_type: String,
    pub time_range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub metrics: HashMap<String, serde_json::Value>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchRequest {
    pub start_document_id: String,
    pub relationship_type: Option<String>,
    pub max_depth: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub distance: usize,
    pub relationship_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchResponse {
    pub nodes: Vec<GraphNode>,
    pub total_count: usize,
    pub query_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub storage_status: String,
    pub indices_status: HashMap<String, String>,
}

/// Error codes for MCP responses
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // KotaDB-specific error codes
    pub const DOCUMENT_NOT_FOUND: i32 = -32001;
    pub const STORAGE_ERROR: i32 = -32002;
    pub const INDEX_ERROR: i32 = -32003;
    pub const VALIDATION_ERROR: i32 = -32004;
    pub const SEARCH_ERROR: i32 = -32005;
}

impl MCPError {
    pub fn parse_error(message: &str) -> Self {
        Self {
            code: error_codes::PARSE_ERROR,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn invalid_request(message: &str) -> Self {
        Self {
            code: error_codes::INVALID_REQUEST,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: error_codes::METHOD_NOT_FOUND,
            message: format!("Method '{method}' not found"),
            data: None,
        }
    }

    pub fn invalid_params(message: &str) -> Self {
        Self {
            code: error_codes::INVALID_PARAMS,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            code: error_codes::INTERNAL_ERROR,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn document_not_found(id: &str) -> Self {
        Self {
            code: error_codes::DOCUMENT_NOT_FOUND,
            message: format!("Document with ID '{id}' not found"),
            data: None,
        }
    }

    pub fn storage_error(message: &str) -> Self {
        Self {
            code: error_codes::STORAGE_ERROR,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn validation_error(message: &str) -> Self {
        Self {
            code: error_codes::VALIDATION_ERROR,
            message: message.to_string(),
            data: None,
        }
    }
}
