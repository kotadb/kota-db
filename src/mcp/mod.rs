/// Model Context Protocol (MCP) Server Implementation for KotaDB
///
/// This module provides a JSON-RPC server that exposes KotaDB functionality
/// through the Model Context Protocol, enabling seamless LLM integration.
pub mod config;
pub mod resources;
pub mod server;
pub mod services_tools;
pub mod tools;
pub mod types;
pub mod streamable_http;

pub use config::MCPConfig;
pub use server::MCPServer;
pub use types::*;

use anyhow::Result;

/// Initialize the MCP server with the given configuration
pub async fn init_mcp_server(config: MCPConfig) -> Result<MCPServer> {
    tracing::info!(
        "Initializing MCP server v{} on {}:{}",
        config.mcp.server_version,
        config.server.host,
        config.server.port
    );

    MCPServer::new(config).await
}
