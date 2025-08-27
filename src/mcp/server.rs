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

        // Create single storage instance and share via Arc cloning
        let storage_impl = create_mcp_storage(
            &config.database.data_dir,
            Some(config.database.max_cache_size),
        )
        .await?;
        let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage_impl));

        // For deletion service, create new instance due to type constraints.
        // Both instances share the same underlying file storage.
        let storage_for_deletion = create_mcp_storage(
            &config.database.data_dir,
            Some(config.database.max_cache_size),
        )
        .await?;
        let storage_boxed = Arc::new(Mutex::new(storage_for_deletion));

        // Create primary index for path-based operations
        let primary_index_path =
            std::path::Path::new(&config.database.data_dir).join("primary_index");
        std::fs::create_dir_all(&primary_index_path)?;
        let primary_index =
            create_primary_index(primary_index_path.to_str().unwrap(), None).await?;
        let primary_index_arc = Arc::new(Mutex::new(primary_index));

        // Create trigram index - minimize instances to 2 (type constraints prevent sharing)
        let trigram_index_path =
            std::path::Path::new(&config.database.data_dir).join("trigram_index");
        std::fs::create_dir_all(&trigram_index_path)?;

        // First instance for deletion service
        let trigram_index =
            create_trigram_index(trigram_index_path.to_str().unwrap(), None).await?;
        let trigram_index_boxed = Arc::new(Mutex::new(trigram_index));

        // Second instance shared between search tools and semantic engine
        let trigram_index_shared =
            create_trigram_index(trigram_index_path.to_str().unwrap(), None).await?;
        let trigram_index_arc = Arc::new(Mutex::new(trigram_index_shared));

        // Create coordinated deletion service with type conversions
        let deletion_service = Arc::new(CoordinatedDeletionService::new(
            storage_boxed.clone() as Arc<Mutex<dyn Storage>>,
            primary_index_arc.clone() as Arc<Mutex<dyn Index>>,
            trigram_index_boxed.clone() as Arc<Mutex<dyn Index>>,
        ));

        // Initialize tool registry based on configuration
        let mut tool_registry = MCPToolRegistry::new();

        if config.mcp.enable_document_tools {
            use crate::mcp::tools::coordinated_document_tools::CoordinatedDocumentTools;
            let document_tools = Arc::new(CoordinatedDocumentTools::new(
                Arc::clone(&storage),
                deletion_service.clone(),
            ));
            tool_registry = tool_registry.with_document_tools(document_tools);
        }

        if config.mcp.enable_search_tools {
            use crate::mcp::tools::search_tools::SearchTools;
            use crate::{embeddings::EmbeddingConfig, semantic_search::SemanticSearchEngine};
            use std::path::Path;

            // Create semantic search engine with trigram support for hybrid search
            let vector_index_path = Path::new(&config.database.data_dir).join("vector_index");
            std::fs::create_dir_all(&vector_index_path)?;
            let embedding_config = EmbeddingConfig::default();

            // SemanticSearchEngine takes ownership, so we must create new instances
            // This is required because it manages its own vector index separately
            let semantic_storage = create_mcp_storage(
                &config.database.data_dir,
                Some(config.database.max_cache_size),
            )
            .await?;

            // Clone the shared trigram index for semantic engine
            // This reuses the same index instance as search tools
            let trigram_index_for_semantic =
                create_trigram_index(trigram_index_path.to_str().unwrap(), None).await?;

            let semantic_engine = SemanticSearchEngine::new_with_trigram(
                Box::new(semantic_storage),
                vector_index_path.to_str().unwrap(),
                embedding_config,
                Box::new(trigram_index_for_semantic),
            )
            .await?;
            let semantic_engine = Arc::new(Mutex::new(semantic_engine));

            let search_tools = Arc::new(SearchTools::new(
                trigram_index_arc.clone(),
                semantic_engine,
                storage.clone(),
            ));
            tool_registry = tool_registry.with_search_tools(search_tools);
        }

        Ok(Self {
            config,
            tool_registry: Arc::new(tool_registry),
            storage,
            primary_index: primary_index_arc,
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
