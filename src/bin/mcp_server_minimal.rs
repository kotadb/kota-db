//! Minimal MCP Server Binary for testing
#[cfg(feature = "mcp-server")]
use anyhow::Result;
#[cfg(feature = "mcp-server")]
use clap::{Arg, Command};
#[cfg(feature = "mcp-server")]
use kotadb::mcp::{config::MCPConfig, server_minimal::MCPServerMinimal};
#[cfg(feature = "mcp-server")]
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(feature = "mcp-server")]
fn main() -> Result<()> {
    let matches = Command::new("kotadb-mcp-minimal")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KotaDB Minimal MCP Server")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("kotadb-mcp.toml"),
        )
        .get_matches();

    // Initialize logging
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kotadb=info"));

    fmt().with_env_filter(filter).with_target(false).init();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = load_config(config_path)?;

    tracing::info!("Starting minimal MCP server v{}", env!("CARGO_PKG_VERSION"));

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.database.data_dir)?;

    // Create a Tokio runtime
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let _server = MCPServerMinimal::new(config).await?;
        tracing::info!("Minimal MCP server started successfully");

        // For testing, just run for a few seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        tracing::info!("Test complete, server stopping");
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

/// Load configuration from file or use defaults
#[cfg(feature = "mcp-server")]
fn load_config(config_path: &str) -> Result<MCPConfig> {
    if std::path::Path::new(config_path).exists() {
        tracing::info!("Loading configuration from: {}", config_path);
        MCPConfig::from_file(config_path)
    } else {
        tracing::warn!(
            "Configuration file not found: {}, using defaults",
            config_path
        );
        Ok(MCPConfig::default())
    }
}

#[cfg(not(feature = "mcp-server"))]
fn main() {
    eprintln!("MCP server support is disabled. Build with --features mcp-server to enable.");
    std::process::exit(1);
}
