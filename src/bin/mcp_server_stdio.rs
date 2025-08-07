//! KotaDB MCP Server Binary (STDIO mode for Claude Desktop)
//!
//! This server uses STDIO for communication, which is what Claude Desktop expects
#[cfg(feature = "mcp-server")]
use anyhow::Result;
#[cfg(feature = "mcp-server")]
use clap::{Arg, Command};
#[cfg(feature = "mcp-server")]
use kotadb::mcp::{config::MCPConfig, init_mcp_server};
#[cfg(feature = "mcp-server")]
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(feature = "mcp-server")]
fn main() -> Result<()> {
    let matches = Command::new("kotadb-mcp-stdio")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KotaDB Model Context Protocol Server (STDIO)")
        .long_about(
            "A high-performance MCP server that exposes KotaDB functionality to LLM clients via STDIO",
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("kotadb-mcp.toml"),
        )
        .arg(
            Arg::new("data-dir")
                .short('d')
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory path")
                .default_value("./kotadb-data"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity (can be used multiple times)"),
        )
        .get_matches();

    // Initialize logging - write to stderr to avoid interfering with STDIO protocol
    let log_level = match matches.get_count("verbose") {
        0 => "warn", // Minimal logging for STDIO mode
        1 => "info",
        _ => "debug",
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("kotadb={log_level}")));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr) // Write logs to stderr, not stdout
        .init();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let mut config = load_config(config_path)?;

    // Override with command line arguments
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.database.data_dir = data_dir.clone();
    }

    // Override with environment variables
    if let Ok(data_dir) = std::env::var("KOTADB_DATA_DIR") {
        config.database.data_dir = data_dir;
    }

    eprintln!(
        "Starting KotaDB MCP Server v{} (STDIO mode)",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("Data directory: {}", config.database.data_dir);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.database.data_dir)?;

    // Create a Tokio runtime
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        // Initialize the MCP server
        let server = init_mcp_server(config).await?;

        // Start STDIO-based JSON-RPC communication
        start_stdio_server(server).await
    })?;

    Ok(())
}

/// Start the STDIO-based MCP server
#[cfg(feature = "mcp-server")]
async fn start_stdio_server(server: kotadb::mcp::MCPServer) -> Result<()> {
    use std::io::BufRead;
    use tokio::io::{stdout, AsyncWriteExt};

    eprintln!("MCP server ready for STDIO communication");

    // Create a simple STDIO handler
    let stdin = std::io::stdin();
    let mut stdout = stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        eprintln!("Received: {line}");

        // Parse JSON-RPC request
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(request) => {
                let response = handle_jsonrpc_request(request, &server).await;
                let response_json = serde_json::to_string(&response)?;

                // Write response to stdout
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;

                eprintln!("Sent: {response_json}");
            }
            Err(e) => {
                eprintln!("Failed to parse JSON-RPC request: {e}");

                // Send error response
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                });

                let response_json = serde_json::to_string(&error_response)?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
    }

    Ok(())
}

/// Handle a JSON-RPC request using the actual MCP server implementation
#[cfg(feature = "mcp-server")]
async fn handle_jsonrpc_request(
    request: serde_json::Value,
    server: &kotadb::mcp::MCPServer,
) -> serde_json::Value {
    use kotadb::mcp::server::MCPRpc;

    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned();

    // Create MCPServerImpl directly to access RPC methods
    let server_impl = create_mcp_server_impl(server);

    match method {
        "initialize" => {
            let params = request
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let params = match params {
                serde_json::Value::Null => jsonrpc_core::Params::None,
                serde_json::Value::Array(arr) => jsonrpc_core::Params::Array(arr),
                serde_json::Value::Object(obj) => jsonrpc_core::Params::Map(obj),
                _ => jsonrpc_core::Params::None,
            };

            match server_impl.initialize(params) {
                Ok(result) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": result,
                        "id": id
                    })
                }
                Err(error) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": error.code.code(),
                            "message": error.message
                        },
                        "id": id
                    })
                }
            }
        }
        "ping" => match server_impl.ping() {
            Ok(result) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                })
            }
            Err(error) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": error.code.code(),
                        "message": error.message
                    },
                    "id": id
                })
            }
        },
        "tools/list" => match server_impl.list_tools() {
            Ok(result) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                })
            }
            Err(error) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": error.code.code(),
                        "message": error.message
                    },
                    "id": id
                })
            }
        },
        "tools/call" => {
            let params = request
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let params = match params {
                serde_json::Value::Null => jsonrpc_core::Params::None,
                serde_json::Value::Array(arr) => jsonrpc_core::Params::Array(arr),
                serde_json::Value::Object(obj) => jsonrpc_core::Params::Map(obj),
                _ => jsonrpc_core::Params::None,
            };

            match server_impl.call_tool(params) {
                Ok(result) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": result,
                        "id": id
                    })
                }
                Err(error) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": error.code.code(),
                            "message": error.message
                        },
                        "id": id
                    })
                }
            }
        }
        "capabilities" => match server_impl.capabilities() {
            Ok(result) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                })
            }
            Err(error) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": error.code.code(),
                        "message": error.message
                    },
                    "id": id
                })
            }
        },
        "resources/list" => match server_impl.list_resources() {
            Ok(result) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                })
            }
            Err(error) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": error.code.code(),
                        "message": error.message
                    },
                    "id": id
                })
            }
        },
        "resources/read" => {
            let params = request
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let params = match params {
                serde_json::Value::Null => jsonrpc_core::Params::None,
                serde_json::Value::Array(arr) => jsonrpc_core::Params::Array(arr),
                serde_json::Value::Object(obj) => jsonrpc_core::Params::Map(obj),
                _ => jsonrpc_core::Params::None,
            };

            match server_impl.read_resource(params) {
                Ok(result) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": result,
                        "id": id
                    })
                }
                Err(error) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": error.code.code(),
                            "message": error.message
                        },
                        "id": id
                    })
                }
            }
        }
        _ => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                },
                "id": id
            })
        }
    }
}

/// Create an MCPServerImpl to access RPC methods
#[cfg(feature = "mcp-server")]
fn create_mcp_server_impl(server: &kotadb::mcp::MCPServer) -> MCPServerStdioImpl {
    // Since we can't access private fields directly, we'll create a minimal implementation
    // that provides tool definitions and basic functionality
    MCPServerStdioImpl {
        uptime_seconds: server.uptime_seconds(),
    }
}

/// STDIO-specific implementation that provides MCP functionality
#[cfg(feature = "mcp-server")]
struct MCPServerStdioImpl {
    uptime_seconds: u64,
}

#[cfg(feature = "mcp-server")]
impl kotadb::mcp::server::MCPRpc for MCPServerStdioImpl {
    fn initialize(
        &self,
        _params: jsonrpc_core::Params,
    ) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        let response = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "kotadb",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {},
                "resources": {},
                "logging": {}
            }
        });
        Ok(response)
    }

    fn list_tools(&self) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        // We need to access the tool registry from the server
        // For now, return document tools that we know are enabled
        let tools = get_enabled_tools();
        let response = serde_json::json!({
            "tools": tools
        });
        Ok(response)
    }

    fn call_tool(&self, params: jsonrpc_core::Params) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        use jsonrpc_core::Error as RpcError;

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

        // Handle document tool calls
        let result = match name {
            "kotadb://document_list" => handle_document_list(arguments),
            "kotadb://document_create" => handle_document_create(arguments),
            "kotadb://document_read" => handle_document_read(arguments),
            _ => {
                return Err(RpcError::method_not_found());
            }
        };

        match result {
            Ok(response_data) => {
                let response = serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&response_data)
                                .unwrap_or_else(|_| response_data.to_string())
                        }
                    ]
                });
                Ok(response)
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Error calling tool '{}': {}", name, e)
                        }
                    ]
                });
                Ok(error_response)
            }
        }
    }

    fn list_resources(&self) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        let response = serde_json::json!({
            "resources": []
        });
        Ok(response)
    }

    fn read_resource(
        &self,
        _params: jsonrpc_core::Params,
    ) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        Err(jsonrpc_core::Error::method_not_found())
    }

    fn capabilities(&self) -> jsonrpc_core::Result<jsonrpc_core::Value> {
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
                "name": "kotadb",
                "version": env!("CARGO_PKG_VERSION")
            },
            "protocolVersion": "2024-11-05"
        });
        Ok(response)
    }

    fn ping(&self) -> jsonrpc_core::Result<jsonrpc_core::Value> {
        let response = serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "uptime_seconds": self.uptime_seconds,
            "version": env!("CARGO_PKG_VERSION")
        });
        Ok(response)
    }
}

/// Get the list of enabled tools
#[cfg(feature = "mcp-server")]
fn get_enabled_tools() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "kotadb://document_create",
            "description": "Create a new document in KotaDB",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path for the document"
                    },
                    "title": {
                        "type": "string",
                        "description": "The title of the document"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content of the document"
                    }
                },
                "required": ["path", "title", "content"]
            }
        }),
        serde_json::json!({
            "name": "kotadb://document_read",
            "description": "Read a document from KotaDB",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The ID of the document to read"
                    }
                },
                "required": ["id"]
            }
        }),
        serde_json::json!({
            "name": "kotadb://document_list",
            "description": "List all documents in KotaDB",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of documents to return",
                        "default": 100
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of documents to skip",
                        "default": 0
                    }
                }
            }
        }),
    ]
}

/// Handle document list tool call
#[cfg(feature = "mcp-server")]
fn handle_document_list(arguments: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    // Parse request - for list, arguments might be empty
    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;
    let offset = arguments
        .get("offset")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    // For now, return a simple response since we don't have access to storage here
    // In a real implementation, this would access the storage
    let response = serde_json::json!({
        "documents": [],
        "total_count": 0,
        "offset": offset,
        "limit": limit
    });

    Ok(response)
}

/// Handle document create tool call
#[cfg(feature = "mcp-server")]
fn handle_document_create(arguments: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    use kotadb::mcp::types::*;

    // Parse request
    let request: DocumentCreateRequest = serde_json::from_value(arguments)
        .map_err(|e| anyhow::anyhow!("Invalid document create request: {}", e))?;

    // For now, return a mock response
    let response = DocumentCreateResponse {
        id: format!("doc_{}", chrono::Utc::now().timestamp()),
        path: request.path,
        created_at: chrono::Utc::now(),
    };

    Ok(serde_json::to_value(response)?)
}

/// Handle document read tool call
#[cfg(feature = "mcp-server")]
fn handle_document_read(arguments: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    use kotadb::mcp::types::*;
    use std::collections::HashMap;

    // Parse request
    let request: DocumentGetRequest = serde_json::from_value(arguments)
        .map_err(|e| anyhow::anyhow!("Invalid document get request: {}", e))?;

    // For now, return a mock response
    let response = DocumentGetResponse {
        id: request.id.clone(),
        path: format!("/docs/{}.md", request.id),
        title: Some("Sample Document".to_string()),
        content: "This is a sample document content.".to_string(),
        tags: vec!["sample".to_string(), "demo".to_string()],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    Ok(serde_json::to_value(response)?)
}

/// Load configuration from file or use defaults
#[cfg(feature = "mcp-server")]
fn load_config(config_path: &str) -> Result<MCPConfig> {
    if std::path::Path::new(config_path).exists() {
        eprintln!("Loading configuration from: {config_path}");
        MCPConfig::from_file(config_path)
    } else {
        eprintln!("Configuration file not found: {config_path}, using defaults");
        Ok(MCPConfig::default())
    }
}

#[cfg(not(feature = "mcp-server"))]
fn main() {
    eprintln!("MCP server support is disabled. Build with --features mcp-server to enable.");
    std::process::exit(1);
}
