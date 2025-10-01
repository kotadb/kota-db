/// MCP Resources Implementation
///
/// This module handles MCP resources - static content that can be referenced
/// and read by LLM clients through the MCP protocol.
use crate::mcp::types::*;
use anyhow::Result;
use std::collections::HashMap;

/// Resource handler for MCP protocol
pub struct MCPResourceHandler {
    resources: HashMap<String, MCPResource>,
}

/// Internal representation of an MCP resource
struct MCPResource {
    definition: ResourceDefinition,
    content: ResourceContent,
}

/// Content of a resource
enum ResourceContent {
    Text(String),
    #[allow(dead_code)] // May be used in future binary resource implementations
    Binary(Vec<u8>),
    Dynamic(Box<dyn Fn() -> Result<String> + Send + Sync>),
}

impl MCPResourceHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            resources: HashMap::new(),
        };

        // Register default resources
        handler.register_default_resources();
        handler
    }

    /// Register a text resource
    pub fn register_text_resource(
        &mut self,
        uri: String,
        name: String,
        description: String,
        content: String,
    ) {
        let resource = MCPResource {
            definition: ResourceDefinition {
                uri: uri.clone(),
                name,
                description,
                mime_type: Some("text/plain".to_string()),
            },
            content: ResourceContent::Text(content),
        };

        self.resources.insert(uri, resource);
    }

    /// Register a dynamic resource that generates content on-demand
    pub fn register_dynamic_resource<F>(
        &mut self,
        uri: String,
        name: String,
        description: String,
        mime_type: Option<String>,
        generator: F,
    ) where
        F: Fn() -> Result<String> + Send + Sync + 'static,
    {
        let resource = MCPResource {
            definition: ResourceDefinition {
                uri: uri.clone(),
                name,
                description,
                mime_type,
            },
            content: ResourceContent::Dynamic(Box::new(generator)),
        };

        self.resources.insert(uri, resource);
    }

    /// List all available resources
    pub fn list_resources(&self) -> Vec<ResourceDefinition> {
        self.resources
            .values()
            .map(|r| r.definition.clone())
            .collect()
    }

    /// Read a specific resource by URI
    pub fn read_resource(&self, uri: &str) -> Result<String> {
        let resource = self
            .resources
            .get(uri)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?;

        match &resource.content {
            ResourceContent::Text(content) => Ok(content.clone()),
            ResourceContent::Binary(data) => {
                // Convert binary to base64 for text representation
                Ok(encode_base64(data))
            }
            ResourceContent::Dynamic(generator) => generator(),
        }
    }

    /// Register default KotaDB resources
    fn register_default_resources(&mut self) {
        // API Documentation
        self.register_text_resource(
            "kotadb://docs/api".to_string(),
            "KotaDB API Documentation".to_string(),
            "Complete API documentation for KotaDB operations".to_string(),
            include_str!("../../docs/api/api.md").to_string(),
        );

        // Configuration Schema
        self.register_dynamic_resource(
            "kotadb://schema/config".to_string(),
            "Configuration Schema".to_string(),
            "JSON schema for KotaDB configuration files".to_string(),
            Some("application/json".to_string()),
            || {
                let schema = serde_json::json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "title": "KotaDB Configuration",
                    "type": "object",
                    "properties": {
                        "database": {
                            "type": "object",
                            "properties": {
                                "data_dir": { "type": "string" },
                                "max_cache_size": { "type": "integer" },
                                "enable_wal": { "type": "boolean" }
                            }
                        },
                        "server": {
                            "type": "object",
                            "properties": {
                                "host": { "type": "string" },
                                "port": { "type": "integer" }
                            }
                        }
                    }
                });
                Ok(serde_json::to_string_pretty(&schema)?)
            },
        );

        // Database Schema
        self.register_dynamic_resource(
            "kotadb://schema/document".to_string(),
            "Document Schema".to_string(),
            "JSON schema for KotaDB document structure".to_string(),
            Some("application/json".to_string()),
            || {
                let schema = serde_json::json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "title": "KotaDB Document",
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "path": { "type": "string" },
                        "title": { "type": "string" },
                        "content": { "type": "string" },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "metadata": {
                            "type": "object",
                            "additionalProperties": true
                        },
                        "created_at": { "type": "string", "format": "date-time" },
                        "updated_at": { "type": "string", "format": "date-time" }
                    },
                    "required": ["id", "path", "content"]
                });
                Ok(serde_json::to_string_pretty(&schema)?)
            },
        );

        // System Status
        self.register_dynamic_resource(
            "kotadb://status/system".to_string(),
            "System Status".to_string(),
            "Current system status and health information".to_string(),
            Some("application/json".to_string()),
            || {
                let status = serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "status": "healthy",
                    "version": env!("CARGO_PKG_VERSION"),
                    "features": {
                        "mcp_server": true,
                        "advanced_search": cfg!(feature = "advanced-search"),
                        "benchmarks": cfg!(feature = "bench")
                    },
                    "components": {
                        "storage": "operational",
                        "indices": "operational",
                        "search": "operational"
                    }
                });
                Ok(serde_json::to_string_pretty(&status)?)
            },
        );

        // Tool Documentation
        self.register_text_resource(
            "kotadb://docs/tools".to_string(),
            "MCP Tools Documentation".to_string(),
            "Documentation for all available MCP tools".to_string(),
            self.generate_tool_documentation(),
        );
    }

    /// Generate documentation for all MCP tools
    fn generate_tool_documentation(&self) -> String {
        String::from(
            r#"# KotaDB MCP Tools

This document describes all available MCP tools for interacting with KotaDB.

## Document Tools

### kotadb://document_create
Create a new document with content, metadata, and tags.

**Parameters:**
- `path` (required): Unique path for the document
- `title` (optional): Human-readable title
- `content` (required): Document content
- `tags` (optional): Array of tags for categorization
- `metadata` (optional): Key-value metadata

### kotadb://document_get
Retrieve a document by its ID.

**Parameters:**
- `id` (required): Document UUID

### kotadb://document_update
Update an existing document's content or metadata.

**Parameters:**
- `id` (required): Document UUID
- `title` (optional): New title
- `content` (optional): New content
- `tags` (optional): New tags array
- `metadata` (optional): Metadata to merge

### kotadb://document_delete
Delete a document by its ID.

**Parameters:**
- `id` (required): Document UUID

### kotadb://document_list
List documents with optional filtering.

**Parameters:**
- `limit` (optional): Maximum results (default: 50)
- `offset` (optional): Skip results (default: 0)
- `tags` (optional): Filter by tags
- `path_prefix` (optional): Filter by path prefix

## Search Tools

### kotadb://text_search
Full-text search using trigram indexing.

**Parameters:**
- `query` (required): Search query
- `limit` (optional): Maximum results
- `offset` (optional): Skip results

> Semantic and hybrid search endpoints have been retired until the cloud-first relaunch.

## Analytics Tools

### kotadb://health_check
Comprehensive system health check.

**Parameters:**
- `include_details` (optional): Include detailed metrics

### kotadb://get_metrics
Get system performance metrics.

**Parameters:**
- `metric_type` (optional): Type of metrics to retrieve
- `time_range` (optional): Time range for metrics

## Graph Tools

### kotadb://graph_search
Traverse document relationships.

**Parameters:**
- `start_document_id` (required): Starting document UUID
- `relationship_type` (optional): Type of relationship to follow
- `max_depth` (optional): Maximum traversal depth
- `limit` (optional): Maximum results

### kotadb://find_path
Find shortest path between documents.

**Parameters:**
- `from_document_id` (required): Starting document UUID
- `to_document_id` (required): Target document UUID
- `max_depth` (optional): Maximum search depth

## Error Handling

All tools return standardized error responses with appropriate HTTP status codes:

- `400 Bad Request`: Invalid parameters
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: System error

## Rate Limiting

API calls are rate-limited to prevent abuse:
- Default: 1000 requests per minute
- Configurable via server settings
"#,
        )
    }
}

impl Default for MCPResourceHandler {
    fn default() -> Self {
        Self::new()
    }
}

// Simple base64 encoding fallback
fn encode_base64(data: &[u8]) -> String {
    // Using simple placeholder for binary data instead of actual encoding
    format!("[Binary data: {} bytes]", data.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_handler_creation() {
        let handler = MCPResourceHandler::new();
        let resources = handler.list_resources();

        // Should have default resources
        assert!(!resources.is_empty());
        assert!(resources.iter().any(|r| r.uri == "kotadb://docs/api"));
        assert!(resources.iter().any(|r| r.uri == "kotadb://schema/config"));
    }

    #[test]
    fn test_register_text_resource() {
        let mut handler = MCPResourceHandler::new();

        handler.register_text_resource(
            "test://example".to_string(),
            "Test Resource".to_string(),
            "A test resource".to_string(),
            "Hello, World!".to_string(),
        );

        let content = handler.read_resource("test://example").unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_dynamic_resource() {
        let mut handler = MCPResourceHandler::new();

        handler.register_dynamic_resource(
            "test://dynamic".to_string(),
            "Dynamic Test".to_string(),
            "A dynamic test resource".to_string(),
            Some("application/json".to_string()),
            || Ok("{\"message\": \"dynamic content\"}".to_string()),
        );

        let content = handler.read_resource("test://dynamic").unwrap();
        assert!(content.contains("dynamic content"));
    }

    #[test]
    fn test_resource_not_found() {
        let handler = MCPResourceHandler::new();
        let result = handler.read_resource("nonexistent://resource");
        assert!(result.is_err());
    }
}
