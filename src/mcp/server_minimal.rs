use crate::contracts::Storage;
use crate::create_file_storage;
use crate::mcp::config::MCPConfig;
use anyhow::Result;
use jsonrpc_core::{IoHandler, Params, Result as RpcResult, Value};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::DomainsValidation;
use jsonrpc_http_server::ServerBuilder;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// MCP Server implementation for KotaDB (minimal version)
pub struct MCPServerMinimal {
    config: MCPConfig,
    storage: Arc<Mutex<dyn Storage>>,
    start_time: Instant,
}

/// JSON-RPC interface for MCP protocol (minimal)
#[rpc(server)]
pub trait MCPRpcMinimal {
    /// Initialize the MCP session
    #[rpc(name = "initialize")]
    fn initialize(&self, _params: Params) -> RpcResult<Value>;

    /// Health check endpoint
    #[rpc(name = "ping")]
    fn ping(&self) -> RpcResult<Value>;

    /// Get server capabilities
    #[rpc(name = "capabilities")]
    fn capabilities(&self) -> RpcResult<Value>;
}

impl MCPServerMinimal {
    /// Create a new minimal MCP server with the given configuration
    pub async fn new(config: MCPConfig) -> Result<Self> {
        tracing::info!("Creating minimal MCP server with config: {:?}", config.mcp);

        // Create wrapped storage using the component library
        let storage = create_file_storage(
            &config.database.data_dir,
            Some(config.database.max_cache_size),
        )
        .await?;
        let storage = Arc::new(Mutex::new(storage));

        Ok(Self {
            config,
            storage,
            start_time: Instant::now(),
        })
    }

    /// Start the MCP server
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting minimal MCP server...");

        let server_impl = MCPServerMinimalImpl {
            config: self.config.clone(),
            storage: self.storage.clone(),
            start_time: self.start_time,
        };

        let mut io = IoHandler::new();
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
            "Minimal MCP server started on {}:{}",
            self.config.server.host,
            self.config.server.port
        );

        server.wait();
        Ok(())
    }

    /// Get the uptime in seconds
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Implementation of the minimal MCP RPC interface
#[derive(Clone)]
struct MCPServerMinimalImpl {
    #[allow(dead_code)] // Config will be used for future configuration features
    config: MCPConfig,
    #[allow(dead_code)] // Storage will be used when implementing tool handlers
    storage: Arc<Mutex<dyn Storage>>,
    start_time: Instant,
}

impl MCPRpcMinimal for MCPServerMinimalImpl {
    fn initialize(&self, _params: Params) -> RpcResult<Value> {
        let response = serde_json::json!({
            "protocolVersion": "1.0",
            "serverInfo": {
                "name": "kotadb-mcp",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {},
                "resources": {}
            }
        });
        Ok(response)
    }

    fn ping(&self) -> RpcResult<Value> {
        let uptime = self.start_time.elapsed().as_secs();
        let response = serde_json::json!({
            "status": "ok",
            "uptime": uptime,
            "message": "KotaDB MCP server is running"
        });
        Ok(response)
    }

    fn capabilities(&self) -> RpcResult<Value> {
        let response = serde_json::json!({
            "tools": {},
            "resources": {}
        });
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_minimal_mcp_server_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().to_str().unwrap();

        let mut config = MCPConfig::default();
        config.database.data_dir = data_dir.to_string();
        let server = MCPServerMinimal::new(config).await?;

        assert!(server.uptime() < 5); // Should be very recent
        Ok(())
    }
}
