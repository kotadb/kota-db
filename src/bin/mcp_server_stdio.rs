//! KotaDB MCP Server - STDIO Version
//!
//! A simple STDIO-based MCP server that avoids HTTP runtime conflicts
#[cfg(feature = "mcp-server")]
use anyhow::Result;
#[cfg(feature = "mcp-server")]
use clap::{Arg, Command};
#[cfg(feature = "mcp-server")]
use kotadb::mcp::config::MCPConfig;
#[cfg(feature = "mcp-server")]
use serde_json::{json, Value};
#[cfg(feature = "mcp-server")]
use std::io::{self, BufRead, BufReader, Write};
#[cfg(feature = "mcp-server")]
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(feature = "mcp-server")]
fn main() -> Result<()> {
    let matches = Command::new("kotadb-mcp-stdio")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KotaDB MCP Server (STDIO)")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("kotadb-mcp-dev.toml"),
        )
        .get_matches();

    // Initialize logging to stderr (stdout is used for MCP communication)
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kotadb=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = load_config(config_path)?;

    eprintln!(
        "Starting KotaDB MCP Server (STDIO) v{}",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("Configuration loaded from: {}", config_path);
    eprintln!("Data directory: {}", config.database.data_dir);

    // Create data directory
    std::fs::create_dir_all(&config.database.data_dir)?;

    // Start STDIO server
    run_stdio_server(config)?;

    Ok(())
}

#[cfg(feature = "mcp-server")]
fn run_stdio_server(config: MCPConfig) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        eprintln!("Received: {}", line);

        // Parse JSON-RPC request
        let request: Value = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Invalid JSON: {}", e);
                continue;
            }
        };

        // Handle request
        let response = handle_request(&request, &config);

        // Send response
        let response_str = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", response_str)?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(feature = "mcp-server")]
fn handle_request(request: &Value, config: &MCPConfig) -> Value {
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned().unwrap_or(json!(1));

    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": config.mcp.protocol_version,
                "serverInfo": {
                    "name": config.mcp.server_name,
                    "version": config.mcp.server_version
                },
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "logging": {}
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "ok",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "version": config.mcp.server_version,
                "message": "KotaDB MCP STDIO server is running"
            }
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": []
            }
        }),
        "capabilities" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": config.mcp.server_name,
                    "version": config.mcp.server_version
                },
                "protocolVersion": config.mcp.protocol_version
            }
        }),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found",
                "data": format!("Unknown method: {}", method)
            }
        }),
    }
}

#[cfg(feature = "mcp-server")]
fn load_config(config_path: &str) -> Result<MCPConfig> {
    if std::path::Path::new(config_path).exists() {
        eprintln!("Loading configuration from: {}", config_path);
        MCPConfig::from_file(config_path)
    } else {
        eprintln!(
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
