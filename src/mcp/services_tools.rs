// Services-Only MCP Tools - Clean Implementation for Interface Parity
//
// This module provides clean MCP tools that expose KotaDB functionality exclusively
// through the services layer, ensuring complete interface parity with CLI and HTTP API.
//
// No legacy code, no deprecated tools - pure services architecture.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    database::Database,
    mcp::types::ToolDefinition,
    services::{
        AnalysisService, BenchmarkOptions, BenchmarkService, CallersOptions, ImpactOptions,
        IndexCodebaseOptions, IndexingService, OverviewOptions, SearchOptions, SearchService,
        StatsOptions, StatsService, SymbolSearchOptions, ValidationOptions, ValidationService,
    },
};

/// Clean services-only MCP tools that provide complete interface parity
pub struct ServicesMCPTools {
    database: Arc<Database>,
    db_path: PathBuf,
}

impl ServicesMCPTools {
    /// Create new services MCP tools instance
    pub fn new(database: Arc<Database>, db_path: PathBuf) -> Self {
        Self { database, db_path }
    }

    /// Get all tool definitions for services-based MCP tools
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            self.define_database_stats_tool(),
            self.define_benchmark_performance_tool(),
            self.define_validate_database_tool(),
            self.define_health_check_tool(),
            self.define_index_codebase_tool(),
            self.define_search_symbols_tool(),
            self.define_incremental_update_tool(),
        ]
    }

    /// Handle MCP tool calls by routing to appropriate service
    pub async fn handle_tool_call(&self, method: &str, params: Value) -> Result<Value> {
        tracing::debug!("Handling services MCP tool call: {}", method);

        match method {
            "kotadb://database_stats" => self.handle_database_stats(params).await,
            "kotadb://benchmark_performance" => self.handle_benchmark_performance(params).await,
            "kotadb://validate_database" => self.handle_validate_database(params).await,
            "kotadb://health_check" => self.handle_health_check(params).await,
            "kotadb://index_codebase" => self.handle_index_codebase(params).await,
            "kotadb://search_symbols" => self.handle_search_symbols(params).await,
            "kotadb://incremental_update" => self.handle_incremental_update(params).await,
            _ => Err(anyhow::anyhow!("Unknown services MCP method: {}", method)),
        }
    }

    /// Define database statistics tool
    fn define_database_stats_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://database_stats".to_string(),
            description: "Get comprehensive database statistics including documents, symbols, and relationships".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "basic": {
                        "type": "boolean",
                        "description": "Show only basic document statistics",
                        "default": false
                    },
                    "symbols": {
                        "type": "boolean", 
                        "description": "Show detailed symbol analysis",
                        "default": true
                    },
                    "relationships": {
                        "type": "boolean",
                        "description": "Show relationship and dependency data", 
                        "default": true
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    /// Handle database stats tool call
    async fn handle_database_stats(&self, params: Value) -> Result<Value> {
        let basic = params
            .get("basic")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let symbols = params
            .get("symbols")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let relationships = params
            .get("relationships")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let stats_service = StatsService::new(&*self.database, self.db_path.clone());
        let options = StatsOptions {
            basic,
            symbols,
            relationships,
            detailed: false,
            quiet: true, // MCP output should be structured
        };

        let result = stats_service.get_statistics(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define benchmark performance tool
    fn define_benchmark_performance_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://benchmark_performance".to_string(),
            description: "Run performance benchmarks on database operations".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operations": {
                        "type": "integer",
                        "description": "Number of operations to perform",
                        "default": 1000,
                        "minimum": 100,
                        "maximum": 50000
                    },
                    "benchmark_type": {
                        "type": "string",
                        "description": "Type of benchmark to run",
                        "enum": ["storage", "index", "query", "all"],
                        "default": "all"
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    /// Handle benchmark performance tool call
    async fn handle_benchmark_performance(&self, params: Value) -> Result<Value> {
        let operations = params
            .get("operations")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as usize;
        let benchmark_type = params
            .get("benchmark_type")
            .and_then(|v| v.as_str())
            .unwrap_or("all")
            .to_string();

        let benchmark_service = BenchmarkService::new(&*self.database, self.db_path.clone());
        let options = BenchmarkOptions {
            operations,
            benchmark_type,
            format: "json".to_string(),
            max_search_queries: 100,
            quiet: true,
            warm_up_operations: Some(10),
            concurrent_operations: Some(1),
        };

        let result = benchmark_service.run_benchmark(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define validate database tool
    fn define_validate_database_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://validate_database".to_string(),
            description: "Validate database integrity and consistency".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "check_integrity": {
                        "type": "boolean",
                        "description": "Check storage and index integrity",
                        "default": true
                    },
                    "check_consistency": {
                        "type": "boolean",
                        "description": "Check cross-index consistency",
                        "default": true
                    },
                    "repair_issues": {
                        "type": "boolean",
                        "description": "Attempt to repair detected issues",
                        "default": false
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    /// Handle validate database tool call
    async fn handle_validate_database(&self, params: Value) -> Result<Value> {
        let check_integrity = params
            .get("check_integrity")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let check_consistency = params
            .get("check_consistency")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let repair_issues = params
            .get("repair_issues")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let validation_service = ValidationService::new(&*self.database, self.db_path.clone());
        let options = ValidationOptions {
            check_integrity,
            check_consistency,
            check_performance: false,
            deep_scan: false,
            repair_issues,
            quiet: true,
        };

        let result = validation_service.validate_database(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define health check tool
    fn define_health_check_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://health_check".to_string(),
            description: "Perform comprehensive database health check".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deep_scan": {
                        "type": "boolean",
                        "description": "Perform deep health scan",
                        "default": false
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    /// Handle health check tool call
    async fn handle_health_check(&self, params: Value) -> Result<Value> {
        let deep_scan = params
            .get("deep_scan")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let validation_service = ValidationService::new(&*self.database, self.db_path.clone());
        let options = ValidationOptions {
            check_integrity: true,
            check_consistency: true,
            check_performance: true,
            deep_scan,
            repair_issues: false,
            quiet: true,
        };

        let result = validation_service.validate_database(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define index codebase tool
    fn define_index_codebase_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://index_codebase".to_string(),
            description: "Index a codebase repository with symbol extraction".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the repository to index"
                    },
                    "prefix": {
                        "type": "string",
                        "description": "Prefix for document paths",
                        "default": "repos"
                    },
                    "include_files": {
                        "type": "boolean",
                        "description": "Include file contents in indexing",
                        "default": true
                    },
                    "include_commits": {
                        "type": "boolean",
                        "description": "Include commit history",
                        "default": true
                    },
                    "extract_symbols": {
                        "type": "boolean",
                        "description": "Extract code symbols using tree-sitter",
                        "default": true
                    }
                },
                "required": ["repo_path"],
                "additionalProperties": false
            }),
        }
    }

    /// Handle index codebase tool call
    async fn handle_index_codebase(&self, params: Value) -> Result<Value> {
        let repo_path = params
            .get("repo_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("repo_path is required"))?;

        let prefix = params
            .get("prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("repos")
            .to_string();

        let include_files = params
            .get("include_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_commits = params
            .get("include_commits")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let extract_symbols = params.get("extract_symbols").and_then(|v| v.as_bool());

        let indexing_service = IndexingService::new(&*self.database, self.db_path.clone());
        let options = IndexCodebaseOptions {
            repo_path: PathBuf::from(repo_path),
            prefix,
            include_files,
            include_commits,
            max_file_size_mb: 10,
            max_memory_mb: None,
            max_parallel_files: None,
            enable_chunking: true,
            extract_symbols,
            no_symbols: false,
            quiet: true,
        };

        let result = indexing_service.index_codebase(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define search symbols tool
    fn define_search_symbols_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://search_symbols".to_string(),
            description:
                "Search for code symbols (functions, classes, variables) by name or pattern"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Symbol name or pattern to search for (supports wildcards)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                        "default": 25,
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "symbol_type": {
                        "type": "string",
                        "description": "Filter by specific symbol type (function, class, variable, etc.)",
                        "default": null
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    /// Handle search symbols tool call
    async fn handle_search_symbols(&self, params: Value) -> Result<Value> {
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("pattern is required"))?
            .to_string();

        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(25) as usize;
        let symbol_type = params
            .get("symbol_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let search_service = SearchService::new(&*self.database, self.db_path.clone());
        let options = SymbolSearchOptions {
            pattern,
            limit,
            symbol_type,
            quiet: true,
        };

        let result = search_service.search_symbols(options).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Define incremental update tool
    fn define_incremental_update_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kotadb://incremental_update".to_string(),
            description: "Perform incremental update of indexed codebase".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the repository to update"
                    },
                    "force_full_reindex": {
                        "type": "boolean",
                        "description": "Force full re-indexing instead of incremental",
                        "default": false
                    },
                    "extract_symbols": {
                        "type": "boolean",
                        "description": "Extract symbols during update",
                        "default": true
                    }
                },
                "required": ["repo_path"],
                "additionalProperties": false
            }),
        }
    }

    /// Handle incremental update tool call
    async fn handle_incremental_update(&self, params: Value) -> Result<Value> {
        let repo_path = params
            .get("repo_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("repo_path is required"))?;

        let force_full_reindex = params
            .get("force_full_reindex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let extract_symbols = params.get("extract_symbols").and_then(|v| v.as_bool());

        let indexing_service = IndexingService::new(&*self.database, self.db_path.clone());

        // Use either incremental update or full reindex based on parameters
        let result = if force_full_reindex {
            let options = IndexCodebaseOptions {
                repo_path: PathBuf::from(repo_path),
                prefix: "repos".to_string(),
                include_files: true,
                include_commits: true,
                max_file_size_mb: 10,
                max_memory_mb: None,
                max_parallel_files: None,
                enable_chunking: true,
                extract_symbols,
                no_symbols: false,
                quiet: true,
            };
            indexing_service.index_codebase(options).await?
        } else {
            // Try incremental update if available, otherwise fall back to full index
            match indexing_service
                .incremental_update(&PathBuf::from(repo_path))
                .await
            {
                Ok(update_result) => serde_json::to_value(update_result)?,
                Err(_) => {
                    // Fall back to full indexing if incremental update not available
                    let options = IndexCodebaseOptions {
                        repo_path: PathBuf::from(repo_path),
                        prefix: "repos".to_string(),
                        include_files: true,
                        include_commits: true,
                        max_file_size_mb: 10,
                        max_memory_mb: None,
                        max_parallel_files: None,
                        enable_chunking: true,
                        extract_symbols,
                        no_symbols: false,
                        quiet: true,
                    };
                    serde_json::to_value(indexing_service.index_codebase(options).await?)?
                }
            }
        };

        Ok(result)
    }
}
