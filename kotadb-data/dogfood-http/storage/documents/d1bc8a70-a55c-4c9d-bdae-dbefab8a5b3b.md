---
tags:
- file
- kota-db
- ext_rs
---
/// MCP Tools Implementation
///
/// This module contains the actual tool implementations that expose
/// KotaDB functionality through the Model Context Protocol.
///
/// Note: Document tools removed per issue #401 - KotaDB is now a pure codebase intelligence platform
pub mod search_tools;

/// Relationship query tools - the killer feature for LLM code understanding
#[cfg(feature = "tree-sitter-parsing")]
pub mod relationship_tools;

use crate::mcp::types::*;
use anyhow::Result;
use std::sync::Arc;

/// Trait for MCP tool handlers
#[async_trait::async_trait]
pub trait MCPToolHandler {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value>;
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
}

/// Main tool registry that coordinates all MCP tools
/// Document tools removed per issue #401 - pure codebase intelligence platform
pub struct MCPToolRegistry {
    pub search_tools: Option<Arc<search_tools::SearchTools>>,
    #[cfg(feature = "tree-sitter-parsing")]
    pub relationship_tools: Option<Arc<relationship_tools::RelationshipTools>>,
}

impl Default for MCPToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPToolRegistry {
    pub fn new() -> Self {
        Self {
            search_tools: None,
            #[cfg(feature = "tree-sitter-parsing")]
            relationship_tools: None,
        }
    }

    /// Register search tools
    pub fn with_search_tools(mut self, tools: Arc<search_tools::SearchTools>) -> Self {
        self.search_tools = Some(tools);
        self
    }

    /// Register relationship tools
    #[cfg(feature = "tree-sitter-parsing")]
    pub fn with_relationship_tools(
        mut self,
        tools: Arc<relationship_tools::RelationshipTools>,
    ) -> Self {
        self.relationship_tools = Some(tools);
        self
    }

    /// Get all available tool definitions
    pub fn get_all_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions = Vec::new();

        if let Some(tools) = &self.search_tools {
            definitions.extend(tools.get_tool_definitions());
        }
        #[cfg(feature = "tree-sitter-parsing")]
        if let Some(tools) = &self.relationship_tools {
            definitions.extend(tools.get_tool_definitions());
        }

        definitions
    }

    /// Handle a tool call by routing to the appropriate handler
    pub async fn handle_tool_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        tracing::debug!("Handling tool call: {}", method);

        // Route to appropriate tool handler based on method prefix
        // Note: Document tools removed per issue #401
        match method {
            m if m.starts_with("kotadb://document_") => {
                Err(anyhow::anyhow!(
                    "Document tools removed in codebase intelligence transition (issue #401). Use search and relationship tools instead."
                ))
            }
            m if m.starts_with("kotadb://text_search")
                || m.starts_with("kotadb://semantic_search")
                || m.starts_with("kotadb://hybrid_search")
                || m.starts_with("kotadb://find_similar")
                || m.starts_with("kotadb://llm_optimized_search") =>
            {
                if let Some(tools) = &self.search_tools {
                    tools.handle_call(method, params).await
                } else {
                    Err(anyhow::anyhow!("Search tools not enabled"))
                }
            }
            #[cfg(feature = "tree-sitter-parsing")]
            m if m.starts_with("kotadb://find_callers")
                || m.starts_with("kotadb://find_callees")
                || m.starts_with("kotadb://impact_analysis")
                || m.starts_with("kotadb://call_chain")
                || m.starts_with("kotadb://circular_dependencies")
                || m.starts_with("kotadb://unused_symbols")
                || m.starts_with("kotadb://hot_paths")
                || m.starts_with("kotadb://relationship_query")
                || m.starts_with("kotadb://codebase_overview") =>
            {
                if let Some(tools) = &self.relationship_tools {
                    tools.handle_call(method, params).await
                } else {
                    Err(anyhow::anyhow!("Relationship tools not enabled"))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
