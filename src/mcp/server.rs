use crate::contracts::{Index, Storage};
use crate::mcp::{config::MCPConfig, tools::MCPToolRegistry};
use crate::wrappers::*;
use crate::{
    create_file_storage, create_primary_index, create_trigram_index, CoordinatedDeletionService,
};
use anyhow::Result;
use jsonrpc_core::{Error as RpcError, IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_http_server::{DomainsValidation, Server};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// MCP Server implementation for KotaDB
pub struct MCPServer {
    config: MCPConfig,
    tool_registry: Arc<MCPToolRegistry>,
    storage: Arc<Mutex<dyn Storage>>,
    #[allow(dead_code)] // Will be used for path-based operations
    primary_index: Arc<Mutex<dyn Index>>,
    #[allow(dead_code)] // Used by CoordinatedDocumentTools for coordinated deletion
    deletion_service: Arc<CoordinatedDeletionService>,
    start_time: Instant,
}

/// Handle to control a running MCP server
pub struct MCPServerHandle {
    server: Server,
}

impl MCPServerHandle {
    /// Wait for the server to finish running
    pub fn wait(self) {
        self.server.wait();
    }

    /// Close the server gracefully
    pub fn close(self) {
        self.server.close();
    }
}

/// JSON-RPC interface for MCP protocol
#[rpc(server)]
pub trait MCPRpc {
    /// Initialize the MCP session
    #[rpc(name = "initialize")]
    fn initialize(&self, params: Params) -> RpcResult<Value>;

    /// List available tools
    #[rpc(name = "tools/list")]
    fn list_tools(&self) -> RpcResult<Value>;

    /// Call a specific tool
    #[rpc(name = "tools/call")]
    fn call_tool(&self, params: Params) -> RpcResult<Value>;

    /// List available resources
    #[rpc(name = "resources/list")]
    fn list_resources(&self) -> RpcResult<Value>;

    /// Read a specific resource
    #[rpc(name = "resources/read")]
    fn read_resource(&self, params: Params) -> RpcResult<Value>;

    /// Get server capabilities
    #[rpc(name = "capabilities")]
    fn capabilities(&self) -> RpcResult<Value>;

    /// Health check endpoint
    #[rpc(name = "ping")]
    fn ping(&self) -> RpcResult<Value>;
}

impl MCPServer {
    /// Create a new MCP server with the given configuration
    pub async fn new(config: MCPConfig) -> Result<Self> {
        tracing::info!("Creating MCP server with config: {:?}", config.mcp);

        // Create SINGLE storage instance shared across most components
        // Note: SemanticSearchEngine will create an additional instance due to its Box<dyn> API
        let storage_impl = create_mcp_storage(
            &config.database.data_dir,
            Some(config.database.max_cache_size),
        )
        .await?;
        let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage_impl));

        // Create SINGLE primary index instance shared across all components
        let primary_index_path =
            std::path::Path::new(&config.database.data_dir).join("primary_index");
        std::fs::create_dir_all(&primary_index_path)?;
        let primary_index =
            create_primary_index(primary_index_path.to_str().unwrap(), None).await?;
        let primary_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));

        // Create SINGLE trigram index instance shared across most components
        // Note: SemanticSearchEngine will create an additional instance due to its Box<dyn> API
        let trigram_index_path =
            std::path::Path::new(&config.database.data_dir).join("trigram_index");
        std::fs::create_dir_all(&trigram_index_path)?;
        let trigram_index =
            create_trigram_index(trigram_index_path.to_str().unwrap(), None).await?;
        let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        // Create coordinated deletion service using the SAME shared instances
        let deletion_service = Arc::new(CoordinatedDeletionService::new(
            Arc::clone(&storage),
            Arc::clone(&primary_index),
            Arc::clone(&trigram_index),
        ));

        // Initialize tool registry based on configuration
        let mut tool_registry = MCPToolRegistry::new();

        // Document tools removed per issue #401 - pure codebase intelligence platform
        if config.mcp.enable_document_tools {
            tracing::warn!("Document tools are disabled - KotaDB has transitioned to pure codebase intelligence (issue #401)");
        }

        // Register lightweight text search tools for MCP
        if config.mcp.enable_search_tools {
            let text_tools = Arc::new(crate::mcp::tools::text_search_tools::TextSearchTools::new(
                trigram_index.clone(),
                storage.clone(),
            ));
            tool_registry = tool_registry.with_text_tools(text_tools);
        }

        #[cfg(feature = "tree-sitter-parsing")]
        if config.mcp.enable_relationship_tools {
            use crate::mcp::tools::relationship_tools::RelationshipTools;
            use crate::services::AnalysisServiceDatabase;
            use std::path::PathBuf;

            // Create database access wrapper for AnalysisService
            struct AnalysisServiceDatabaseImpl {
                storage: Arc<Mutex<dyn Storage>>,
            }

            impl AnalysisServiceDatabase for AnalysisServiceDatabaseImpl {
                fn storage(&self) -> Arc<Mutex<dyn Storage>> {
                    self.storage.clone()
                }
            }

            let database_access: Arc<dyn AnalysisServiceDatabase> =
                Arc::new(AnalysisServiceDatabaseImpl {
                    storage: storage.clone(),
                });

            let db_path = PathBuf::from(&config.database.data_dir);
            let relationship_tools = Arc::new(RelationshipTools::new(database_access, db_path));
            tool_registry = tool_registry.with_relationship_tools(relationship_tools);
        }

        // Symbol tools - enable when tree-sitter parsing and symbols are available
        #[cfg(feature = "tree-sitter-parsing")]
        if config.mcp.enable_relationship_tools {
            use crate::mcp::tools::symbol_tools::SymbolTools;
            use std::path::PathBuf;

            let symbol_tools = Arc::new(SymbolTools::new(
                storage.clone(),
                primary_index.clone(),
                trigram_index.clone(),
                PathBuf::from(&config.database.data_dir),
            ));
            tool_registry = tool_registry.with_symbol_tools(symbol_tools);
        }

        Ok(Self {
            config,
            tool_registry: Arc::new(tool_registry),
            storage,
            primary_index,
            deletion_service,
            start_time: Instant::now(),
        })
    }

    /// Start the MCP server and return a handle to control it
    pub async fn start(self) -> Result<MCPServerHandle> {
        let mut io = IoHandler::new();
        let server_impl = MCPServerImpl {
            config: self.config.clone(),
            tool_registry: self.tool_registry.clone(),
            storage: self.storage.clone(),
            start_time: self.start_time,
        };

        io.extend_with(server_impl.to_delegate());

        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                jsonrpc_http_server::cors::AccessControlAllowOrigin::Any,
            ]))
            .start_http(
                &format!("{}:{}", self.config.server.host, self.config.server.port).parse()?,
            )
            .map_err(|e| anyhow::anyhow!("Failed to start HTTP server: {}", e))?;

        tracing::info!(
            "MCP server started on {}:{}",
            self.config.server.host,
            self.config.server.port
        );

        Ok(MCPServerHandle { server })
    }

    /// Start the MCP server synchronously outside of async context to avoid runtime conflicts
    pub fn start_sync(self) -> Result<MCPServerHandle> {
        let mut io = IoHandler::new();
        let server_impl = MCPServerImpl {
            config: self.config.clone(),
            tool_registry: self.tool_registry.clone(),
            storage: self.storage.clone(),
            start_time: self.start_time,
        };

        io.extend_with(server_impl.to_delegate());

        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                jsonrpc_http_server::cors::AccessControlAllowOrigin::Any,
            ]))
            .start_http(
                &format!("{}:{}", self.config.server.host, self.config.server.port).parse()?,
            )
            .map_err(|e| anyhow::anyhow!("Failed to start HTTP server: {}", e))?;

        tracing::info!(
            "MCP server started on {}:{}",
            self.config.server.host,
            self.config.server.port
        );

        Ok(MCPServerHandle { server })
    }

    /// Get the uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Implementation of the MCP RPC interface
#[derive(Clone)]
struct MCPServerImpl {
    config: MCPConfig,
    tool_registry: Arc<MCPToolRegistry>,
    #[allow(dead_code)] // Storage will be used when implementing tool handlers
    storage: Arc<Mutex<dyn Storage>>,
    start_time: Instant,
}

impl MCPRpc for MCPServerImpl {
    fn initialize(&self, _params: Params) -> RpcResult<Value> {
        tracing::info!("MCP session initialized");

        let response = serde_json::json!({
            "protocolVersion": self.config.mcp.protocol_version,
            "serverInfo": {
                "name": self.config.mcp.server_name,
                "version": self.config.mcp.server_version
            },
            "capabilities": {
                "tools": {},
                "resources": {},
                "logging": {}
            }
        });

        Ok(response)
    }

    fn list_tools(&self) -> RpcResult<Value> {
        let tools = self.tool_registry.get_all_tool_definitions();
        let response = serde_json::json!({
            "tools": tools
        });

        tracing::debug!("Listed {} tools", tools.len());
        Ok(response)
    }

    fn call_tool(&self, params: Params) -> RpcResult<Value> {
        let request: serde_json::Value = params
            .parse()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {e}")))?;

        let name = request
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("Missing 'name' parameter"))?;

        let arguments = request
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        tracing::debug!("Calling tool: {}", name);

        // Handle tool call asynchronously
        let tool_registry = self.tool_registry.clone();
        let method = name.to_string();

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { tool_registry.handle_tool_call(&method, arguments).await })
        });

        match result {
            Ok(response) => {
                let wrapped_response = serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&response)
                                .unwrap_or_else(|_| response.to_string())
                        }
                    ]
                });
                Ok(wrapped_response)
            }
            Err(e) => {
                tracing::error!("Tool call failed: {}", e);
                Err(RpcError::internal_error())
            }
        }
    }

    fn list_resources(&self) -> RpcResult<Value> {
        // For now, return empty resources - can be extended later
        let response = serde_json::json!({
            "resources": []
        });

        Ok(response)
    }

    fn read_resource(&self, _params: Params) -> RpcResult<Value> {
        // For now, return not implemented
        Err(RpcError::method_not_found())
    }

    fn capabilities(&self) -> RpcResult<Value> {
        let response = serde_json::json!({
            "capabilities": {
                "tools": {
                    "listChanged": false,
                    "supportsProgress": false
                },
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                },
                "logging": {},
                "prompts": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": self.config.mcp.server_name,
                "version": self.config.mcp.server_version
            },
            "protocolVersion": self.config.mcp.protocol_version
        });

        Ok(response)
    }

    fn ping(&self) -> RpcResult<Value> {
        let response = serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "uptime_seconds": self.start_time.elapsed().as_secs(),
            "version": self.config.mcp.server_version
        });

        Ok(response)
    }
}

/// Helper function to create wrapped storage using the component library
async fn create_mcp_storage(data_dir: &str, cache_size: Option<usize>) -> Result<impl Storage> {
    // Use the component library factory function
    let storage = create_file_storage(data_dir, cache_size).await?;
    Ok(storage)
}

/// Create fully wrapped storage with all safety guarantees
#[allow(dead_code)] // Utility function for future storage implementations
async fn create_wrapped_storage<S: Storage + 'static>(
    storage: S,
    cache_size: usize,
) -> Result<impl Storage> {
    let cached = CachedStorage::new(storage, cache_size);
    let retryable = RetryableStorage::new(cached);
    let validated = ValidatedStorage::new(retryable);
    let traced = TracedStorage::new(validated);
    Ok(traced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_mcp_server_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = MCPConfig::default();
        config.database.data_dir = temp_dir.path().to_string_lossy().to_string();
        // Disable search tools for this test to avoid embedding model requirements
        config.mcp.enable_search_tools = false;

        let server = MCPServer::new(config).await?;
        assert!(server.uptime_seconds() < 1); // Should be very fresh
        Ok(())
    }

    #[tokio::test]
    async fn test_tool_registry_initialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = MCPConfig::default();
        config.database.data_dir = temp_dir.path().to_string_lossy().to_string();
        // Disable search tools for this test to avoid embedding model requirements
        config.mcp.enable_search_tools = false;

        let server = MCPServer::new(config).await?;
        let tools = server.tool_registry.get_all_tool_definitions();

        // Should have tools from enabled categories
        assert!(!tools.is_empty());
        assert!(tools
            .iter()
            .any(|t| t.name.starts_with("kotadb://document_")));
        // Search tools are currently disabled
        // assert!(tools
        //     .iter()
        //     .any(|t| t.name.starts_with("kotadb://text_search")));

        Ok(())
    }
}
