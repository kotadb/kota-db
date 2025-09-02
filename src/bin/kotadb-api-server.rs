//! KotaDB SaaS API Server
//!
//! Production HTTP server with API key authentication,
//! rate limiting, and codebase intelligence features.

use anyhow::Result;
use clap::Parser;
use kotadb::{
    api_keys::ApiKeyConfig, create_file_storage, http_server::start_saas_server,
    observability::init_logging,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "KotaDB SaaS API Server")]
struct Args {
    /// Data directory path
    #[arg(short = 'd', long, default_value = "/data", env = "KOTADB_DATA_DIR")]
    data_dir: PathBuf,

    /// Server port
    #[arg(short = 'p', long, default_value = "8080", env = "PORT")]
    port: u16,

    /// PostgreSQL database URL for API keys
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Maximum database connections
    #[arg(long, default_value = "10", env = "DATABASE_MAX_CONNECTIONS")]
    max_connections: u32,

    /// Database connection timeout in seconds
    #[arg(long, default_value = "30", env = "DATABASE_CONNECT_TIMEOUT")]
    connect_timeout: u64,

    /// Default rate limit (requests per minute)
    #[arg(long, default_value = "60", env = "DEFAULT_RATE_LIMIT")]
    default_rate_limit: u32,

    /// Default monthly quota (requests per month)
    #[arg(long, default_value = "1000000", env = "DEFAULT_MONTHLY_QUOTA")]
    default_monthly_quota: u64,

    /// Enable quiet mode (minimal logging)
    #[arg(short = 'q', long, env = "QUIET_MODE")]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if !args.quiet {
        init_logging()?;
    }

    info!("🚀 Starting KotaDB SaaS API Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {}", args.data_dir.display());
    info!("Port: {}", args.port);

    // Ensure data directory exists
    std::fs::create_dir_all(&args.data_dir)?;

    // Create storage backend
    let storage_path = args.data_dir.join("storage");
    let storage = create_file_storage(
        storage_path.to_str().unwrap(),
        Some(1000), // Cache capacity
    )
    .await?;
    let storage = Arc::new(Mutex::new(storage));

    // Configure API key service
    let api_key_config = ApiKeyConfig {
        database_url: args.database_url,
        max_connections: args.max_connections,
        connect_timeout_seconds: args.connect_timeout,
        default_rate_limit: args.default_rate_limit,
        default_monthly_quota: args.default_monthly_quota,
    };

    // Start the server
    start_saas_server(storage, args.data_dir, api_key_config, args.port).await?;

    Ok(())
}
