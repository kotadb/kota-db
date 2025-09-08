use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::*;
use crate::services::{AnalysisService, AnalysisServiceDatabase, CallersOptions, ImpactOptions};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Relationship query tools for MCP - the killer feature that differentiates KotaDB
/// from text search tools by enabling LLM impact analysis and architectural understanding
pub struct RelationshipTools {
    database: Arc<dyn AnalysisServiceDatabase>,
    db_path: PathBuf,
}

impl RelationshipTools {
    pub fn new(database: Arc<dyn AnalysisServiceDatabase>, db_path: PathBuf) -> Self {
        Self { database, db_path }
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
            "kotadb://codebase_overview" => {
                let request: CodebaseOverviewRequest = serde_json::from_value(params)?;
                self.codebase_overview(request).await
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
            ToolDefinition {
                name: "kotadb://codebase_overview".to_string(),
                description: "Generate comprehensive codebase overview with metrics, symbols, relationships, and architectural insights".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "description": "Output format: 'human' for readable text or 'json' for structured data",
                            "enum": ["human", "json"],
                            "default": "human"
                        },
                        "top_symbols_limit": {
                            "type": "integer",
                            "description": "Number of top referenced symbols to include (default: 10, max: 50)",
                            "minimum": 1,
                            "maximum": 50,
                            "default": 10
                        },
                        "entry_points_limit": {
                            "type": "integer",
                            "description": "Number of entry points to include (default: 20, max: 100)",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 20
                        }
                    }
                }),
            },
        ]
    }
}

impl RelationshipTools {
    async fn find_callers(&self, request: FindCallersRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let mut analysis_service =
            AnalysisService::new(self.database.as_ref(), self.db_path.clone());
        let options = CallersOptions {
            target: request.target.clone(),
            limit: None,
            quiet: false,
        };

        let result = analysis_service.find_callers(options).await?;
        let execution_time = start_time.elapsed().as_millis();

        tracing::info!(
            "Find callers completed: '{}' found {} callers in {}ms",
            request.target,
            result.total_count,
            execution_time
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "find_callers",
            "target": request.target,
            "callers": result.callers,
            "total_count": result.total_count,
            "markdown": result.markdown
        }))
    }

    async fn find_callees(&self, request: FindCalleesRequest) -> Result<serde_json::Value> {
        // TODO: Implement find_callees in AnalysisService for Phase 3
        Err(anyhow::anyhow!(
            "Find callees '{}' is not yet implemented in Phase 2. Available in Phase 3.",
            request.target
        ))
    }

    async fn impact_analysis(&self, request: ImpactAnalysisRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let mut analysis_service =
            AnalysisService::new(self.database.as_ref(), self.db_path.clone());
        let options = ImpactOptions {
            target: request.target.clone(),
            limit: None,
            quiet: false,
        };

        let result = analysis_service.analyze_impact(options).await?;
        let execution_time = start_time.elapsed().as_millis();

        tracing::info!(
            "Impact analysis completed: '{}' found {} impacts in {}ms",
            request.target,
            result.total_count,
            execution_time
        );

        Ok(serde_json::json!({
            "success": true,
            "query_type": "impact_analysis",
            "target": request.target,
            "impacts": result.impacts,
            "total_count": result.total_count,
            "markdown": result.markdown
        }))
    }

    async fn call_chain(&self, request: CallChainRequest) -> Result<serde_json::Value> {
        // TODO: Implement call_chain in AnalysisService for Phase 3
        Err(anyhow::anyhow!(
            "Call chain from '{}' to '{}' is not yet implemented in Phase 2. Available in Phase 3.",
            request.from,
            request.to
        ))
    }

    async fn circular_dependencies(
        &self,
        request: CircularDependenciesRequest,
    ) -> Result<serde_json::Value> {
        // TODO: Implement circular_dependencies in AnalysisService for Phase 3
        Err(anyhow::anyhow!(
            "Circular dependencies analysis is not yet implemented in Phase 2. Available in Phase 3."
        ))
    }

    async fn unused_symbols(&self, request: UnusedSymbolsRequest) -> Result<serde_json::Value> {
        // TODO: Implement unused_symbols in AnalysisService for Phase 3
        Err(anyhow::anyhow!(
            "Unused symbols analysis is not yet implemented in Phase 2. Available in Phase 3."
        ))
    }

    async fn hot_paths(&self, request: HotPathsRequest) -> Result<serde_json::Value> {
        // TODO: Implement hot_paths in AnalysisService for Phase 3
        Err(anyhow::anyhow!(
            "Hot paths analysis is not yet implemented in Phase 2. Available in Phase 3."
        ))
    }

    async fn natural_language_relationship_query(
        &self,
        request: NaturalLanguageRelationshipRequest,
    ) -> Result<serde_json::Value> {
        Err(anyhow::anyhow!(
            "Natural language query '{}' is not supported. Use direct commands instead:\n\
            - Use 'find-callers <symbol>' instead of 'what calls X?'\n\
            - Use 'analyze-impact <symbol>' instead of 'what would break if I change X?'\n\
            - Use direct symbol search commands",
            request.query
        ))
    }

    async fn codebase_overview(
        &self,
        request: CodebaseOverviewRequest,
    ) -> Result<serde_json::Value> {
        use crate::services::OverviewOptions;

        let analysis_service = AnalysisService::new(self.database.as_ref(), self.db_path.clone());
        let options = OverviewOptions {
            format: request.format.unwrap_or_else(|| "human".to_string()),
            top_symbols_limit: request.top_symbols_limit.unwrap_or(10),
            entry_points_limit: request.entry_points_limit.unwrap_or(20),
            quiet: false,
        };

        let result = analysis_service.generate_overview(options).await?;

        Ok(serde_json::json!({
            "success": true,
            "query_type": "codebase_overview",
            "overview_data": result.overview_data,
            "formatted_output": result.formatted_output
        }))
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
#[allow(dead_code)] // Phase 3 feature
struct CircularDependenciesRequest {
    target: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)] // Phase 3 feature
struct UnusedSymbolsRequest {
    symbol_type: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)] // Phase 3 feature
struct HotPathsRequest {
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct NaturalLanguageRelationshipRequest {
    query: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CodebaseOverviewRequest {
    format: Option<String>,
    top_symbols_limit: Option<usize>,
    entry_points_limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    // NOTE: Relationship tools tests disabled pending real implementation
    // Per AGENT.md - no mocking allowed, need real implementations

    // Tests would go here when relationship query engine is fully tested
    // For now, the functionality is tested through integration tests
}
