use crate::contracts::Storage;
use crate::dependency_extractor::DependencyGraph;
use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::*;
use crate::parsing::SymbolType;
use crate::relationship_query::{RelationshipQueryEngine, RelationshipQueryType};
use crate::symbol_storage::SymbolStorage;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Relationship query tools for MCP - the killer feature that differentiates KotaDB
/// from text search tools by enabling LLM impact analysis and architectural understanding
pub struct RelationshipTools {
    relationship_engine: Option<RelationshipQueryEngine>,
    #[allow(dead_code)]
    storage: Arc<Mutex<dyn Storage>>,
}

impl RelationshipTools {
    pub fn new(storage: Arc<Mutex<dyn Storage>>) -> Self {
        Self {
            relationship_engine: None,
            storage,
        }
    }

    /// Initialize with dependency graph and symbol storage
    pub fn with_relationship_engine(
        mut self,
        dependency_graph: DependencyGraph,
        symbol_storage: SymbolStorage,
    ) -> Self {
        self.relationship_engine = Some(RelationshipQueryEngine::new(
            dependency_graph,
            symbol_storage,
        ));
        self
    }
}

#[async_trait::async_trait]
impl MCPToolHandler for RelationshipTools {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "kotadb://find_callers" => {
                let request: FindCallersRequest = serde_json::from_value(params)?;
                self.find_callers(request).await
            }
            "kotadb://find_callees" => {
                let request: FindCalleesRequest = serde_json::from_value(params)?;
                self.find_callees(request).await
            }
            "kotadb://impact_analysis" => {
                let request: ImpactAnalysisRequest = serde_json::from_value(params)?;
                self.impact_analysis(request).await
            }
            "kotadb://call_chain" => {
                let request: CallChainRequest = serde_json::from_value(params)?;
                self.call_chain(request).await
            }
            "kotadb://circular_dependencies" => {
                let request: CircularDependenciesRequest = serde_json::from_value(params)?;
                self.circular_dependencies(request).await
            }
            "kotadb://unused_symbols" => {
                let request: UnusedSymbolsRequest = serde_json::from_value(params)?;
                self.unused_symbols(request).await
            }
            "kotadb://hot_paths" => {
                let request: HotPathsRequest = serde_json::from_value(params)?;
                self.hot_paths(request).await
            }
            "kotadb://relationship_query" => {
                let request: NaturalLanguageRelationshipRequest = serde_json::from_value(params)?;
                self.natural_language_relationship_query(request).await
            }
            _ => Err(anyhow::anyhow!("Unknown relationship method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "kotadb://find_callers".to_string(),
                description: "Find all symbols that call or use a target symbol - the reverse dependency analysis that grep can't do".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Name or qualified name of the target symbol to find callers for"
                        }
                    },
                    "required": ["target"]
                }),
            },
            ToolDefinition {
                name: "kotadb://find_callees".to_string(),
                description: "Find all symbols that a target symbol calls or uses - forward dependency analysis".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Name or qualified name of the target symbol to find callees for"
                        }
                    },
                    "required": ["target"]
                }),
            },
            ToolDefinition {
                name: "kotadb://impact_analysis".to_string(),
                description: "Analyze what would break if you change a symbol - the killer feature for safe refactoring".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Name or qualified name of the target symbol to analyze impact for"
                        }
                    },
                    "required": ["target"]
                }),
            },
            ToolDefinition {
                name: "kotadb://call_chain".to_string(),
                description: "Find the call chain path between two symbols - trace execution flow".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "from": {
                            "type": "string",
                            "description": "Starting symbol name"
                        },
                        "to": {
                            "type": "string",
                            "description": "Target symbol name"
                        }
                    },
                    "required": ["from", "to"]
                }),
            },
            ToolDefinition {
                name: "kotadb://circular_dependencies".to_string(),
                description: "Find circular dependencies in the codebase - identify architectural problems".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Optional: focus on cycles involving this specific symbol",
                            "nullable": true
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "kotadb://unused_symbols".to_string(),
                description: "Find unused symbols (dead code) - optimize codebase size and complexity".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "symbol_type": {
                            "type": "string",
                            "description": "Optional: filter by symbol type (Function, Struct, Class, etc.)",
                            "enum": ["Function", "Struct", "Class", "Interface", "Variable", "Constant", "Method"]
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "kotadb://hot_paths".to_string(),
                description: "Find the most frequently called symbols - identify performance bottlenecks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of hot paths to return (default: 10, max: 50)",
                            "minimum": 1,
                            "maximum": 50
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "kotadb://relationship_query".to_string(),
                description: "Natural language relationship queries - ask questions about code relationships in plain English".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language query about relationships (e.g., 'what calls FileStorage?', 'what would break if I change StorageError?')"
                        }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }
}

impl RelationshipTools {
    async fn find_callers(&self, request: FindCallersRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let query_type = RelationshipQueryType::FindCallers {
            target: request.target.clone(),
        };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Find callers completed: '{}' found {} callers in {}ms",
            request.target,
            result.stats.direct_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "find_callers",
            "target": request.target,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn find_callees(&self, request: FindCalleesRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let query_type = RelationshipQueryType::FindCallees {
            target: request.target.clone(),
        };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Find callees completed: '{}' found {} callees in {}ms",
            request.target,
            result.stats.direct_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "find_callees",
            "target": request.target,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn impact_analysis(&self, request: ImpactAnalysisRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let query_type = RelationshipQueryType::ImpactAnalysis {
            target: request.target.clone(),
        };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Impact analysis completed: '{}' found {} direct and {} indirect impacts in {}ms",
            request.target,
            result.stats.direct_count,
            result.stats.indirect_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "impact_analysis",
            "target": request.target,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn call_chain(&self, request: CallChainRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let query_type = RelationshipQueryType::CallChain {
            from: request.from.clone(),
            to: request.to.clone(),
        };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Call chain completed: '{}' to '{}' found {} paths in {}ms",
            request.from,
            request.to,
            result.stats.indirect_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "call_chain",
            "from": request.from,
            "to": request.to,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn circular_dependencies(
        &self,
        request: CircularDependenciesRequest,
    ) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let query_type = RelationshipQueryType::CircularDependencies {
            target: request.target.clone(),
        };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Circular dependencies completed: found {} cycles in {}ms",
            result.stats.indirect_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "circular_dependencies",
            "target": request.target,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn unused_symbols(&self, request: UnusedSymbolsRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        // Parse symbol type from string if provided
        let symbol_type = request.symbol_type.as_deref().and_then(|s| match s {
            "Function" => Some(SymbolType::Function),
            "Struct" => Some(SymbolType::Struct),
            "Class" => Some(SymbolType::Class),
            "Interface" => Some(SymbolType::Interface),
            "Variable" => Some(SymbolType::Variable),
            "Constant" => Some(SymbolType::Constant),
            "Method" => Some(SymbolType::Method),
            _ => None,
        });

        let query_type = RelationshipQueryType::UnusedSymbols { symbol_type };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Unused symbols completed: found {} unused symbols in {}ms",
            result.stats.direct_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "unused_symbols",
            "symbol_type": request.symbol_type,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn hot_paths(&self, request: HotPathsRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        let limit = request.limit.unwrap_or(10).min(50);
        let query_type = RelationshipQueryType::HotPaths { limit: Some(limit) };

        let result = engine.execute_query(query_type).await?;

        tracing::info!(
            "Hot paths completed: found {} hot paths in {}ms",
            result.stats.direct_count,
            result.stats.execution_time_ms
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "hot_paths",
            "limit": limit,
            "result": result,
            "markdown": result.to_markdown()
        }))
    }

    async fn natural_language_relationship_query(
        &self,
        request: NaturalLanguageRelationshipRequest,
    ) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let engine = self
            .relationship_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relationship engine not initialized"))?;

        // Parse the natural language query
        if let Some(query_type) =
            crate::relationship_query::parse_natural_language_relationship_query(&request.query)
        {
            let result = engine.execute_query(query_type).await?;

            tracing::info!(
                "Natural language relationship query completed: '{}' in {}ms",
                request.query,
                result.stats.execution_time_ms
            );

            Ok(serde_json::json!({
                "success": true,
                "query": request.query,
                "result": result,
                "markdown": result.to_markdown()
            }))
        } else {
            Err(anyhow::anyhow!(
                "Could not parse relationship query: '{}'. Try queries like 'what calls FileStorage?' or 'what would break if I change StorageError?'",
                request.query
            ))
        }
    }
}

// Request types for MCP tools
#[derive(Debug, Clone, serde::Deserialize)]
struct FindCallersRequest {
    target: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct FindCalleesRequest {
    target: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ImpactAnalysisRequest {
    target: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CallChainRequest {
    from: String,
    to: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CircularDependenciesRequest {
    target: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct UnusedSymbolsRequest {
    symbol_type: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct HotPathsRequest {
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct NaturalLanguageRelationshipRequest {
    query: String,
}

#[cfg(test)]
mod tests {
    // NOTE: Relationship tools tests disabled pending real implementation
    // Per AGENT.md - no mocking allowed, need real implementations

    // Tests would go here when relationship query engine is fully tested
    // For now, the functionality is tested through integration tests
}
