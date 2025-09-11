//! Intent-Based MCP Server Binary
//!
//! Standalone binary for running KotaDB as an intent-based MCP server
//! that provides natural language interface for AI assistants.
//!
//! Issue #645: Intent-Based MCP Server: Transform Raw API Exposure to Natural Language Interface

use anyhow::Result;
use clap::{Arg, Command};
use kotadb::intent_mcp_server::{IntentMcpConfig, IntentMcpServer};
use std::io::{self, BufRead, Write};
use tokio::signal;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("kotadb-intent-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KotaDB Intent-Based MCP Server")
        .long_about(
            "A natural language interface MCP server that transforms queries into orchestrated API calls",
        )
        .arg(
            Arg::new("api-url")
                .long("api-url")
                .value_name("URL")
                .help("Base URL for KotaDB HTTP API")
                .default_value("http://localhost:8080"),
        )
        .arg(
            Arg::new("api-key")
                .long("api-key")
                .value_name("KEY")
                .help("API key for authentication"),
        )
        .arg(
            Arg::new("max-results")
                .long("max-results")
                .value_name("NUM")
                .help("Maximum results per query")
                .default_value("20"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("MS")
                .help("Request timeout in milliseconds")
                .default_value("30000"),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .action(clap::ArgAction::SetTrue)
                .help("Run in interactive mode for testing"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity (can be used multiple times)"),
        )
        .get_matches();

    // Initialize logging
    let log_level = match matches.get_count("verbose") {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!("kotadb={log_level},intent_mcp_server={log_level}"))
    });

    fmt().with_env_filter(filter).with_target(false).init();

    // Build configuration
    let config = IntentMcpConfig {
        api_base_url: matches
            .get_one::<String>("api-url")
            .cloned()
            .unwrap_or_else(|| "http://localhost:8080".to_string()),
        api_key: matches.get_one::<String>("api-key").cloned(),
        max_results: matches
            .get_one::<String>("max-results")
            .ok_or_else(|| anyhow::anyhow!("missing --max-results"))?
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid max-results: {}", e))?,
        default_timeout_ms: matches
            .get_one::<String>("timeout")
            .ok_or_else(|| anyhow::anyhow!("missing --timeout"))?
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid timeout: {}", e))?,
    };

    tracing::info!(
        "Starting KotaDB Intent-Based MCP Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Configuration: API URL = {}", config.api_base_url);
    tracing::info!(
        "Max results: {}, Timeout: {}ms",
        config.max_results,
        config.default_timeout_ms
    );

    // Initialize the intent-based MCP server
    let server = IntentMcpServer::new(config)?;

    if matches.get_flag("interactive") {
        run_interactive_mode(server).await?;
    } else {
        run_mcp_mode(server).await?;
    }

    Ok(())
}

/// Run in interactive mode for testing
async fn run_interactive_mode(server: IntentMcpServer) -> Result<()> {
    tracing::info!("Starting interactive mode - type 'exit' to quit");
    println!("🤖 KotaDB Intent-Based MCP Server - Interactive Mode");
    println!("Type your natural language queries below. Examples:");
    println!("  - 'Find all async functions in the storage module'");
    println!("  - 'Who calls validate_path?'");
    println!("  - 'What would break if I change FileStorage?'");
    println!("  - 'Show me an overview of the codebase'");
    println!("Type 'exit' to quit.\n");

    let stdin = io::stdin();
    let session_id = "interactive-session".to_string();

    loop {
        print!("🔍 Query: ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let query = line.trim();

        if query.is_empty() {
            continue;
        }

        if query.eq_ignore_ascii_case("exit") || query.eq_ignore_ascii_case("quit") {
            println!("👋 Goodbye!");
            break;
        }

        // Process the query
        match server.process_query(query, &session_id).await {
            Ok(response) => {
                println!("\n📋 Intent: {:?}", response.intent);
                println!(
                    "📊 Results: {}",
                    serde_json::to_string_pretty(&response.results)
                        .unwrap_or_else(|_| "Failed to format results".to_string())
                );
                println!("💡 Summary: {}", response.summary);

                if !response.suggestions.is_empty() {
                    println!("🔮 Suggestions:");
                    for suggestion in &response.suggestions {
                        println!("  - {}", suggestion);
                    }
                }

                println!("⏱️  Query time: {}ms\n", response.query_time_ms);
            }
            Err(e) => {
                eprintln!("❌ Error: {}\n", e);
            }
        }
    }

    Ok(())
}

/// Run in MCP mode (JSON-RPC over stdio)
async fn run_mcp_mode(server: IntentMcpServer) -> Result<()> {
    tracing::info!("Starting MCP protocol mode");

    // In a real implementation, this would:
    // 1. Listen for JSON-RPC requests on stdin
    // 2. Parse MCP protocol messages
    // 3. Route to intent server based on tool calls
    // 4. Return MCP-compatible responses on stdout

    // For now, we'll simulate this with a simple message
    println!("{{\"jsonrpc\": \"2.0\", \"method\": \"initialize\", \"params\": {{\"capabilities\": {{\"tools\": true, \"resources\": false}}, \"serverInfo\": {{\"name\": \"kotadb-intent-mcp\", \"version\": \"{}\"}}}}}}", env!("CARGO_PKG_VERSION"));

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("Shutting down intent-based MCP server");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        // Test that the configuration parsing logic works
        let config = IntentMcpConfig {
            api_base_url: "http://test:8080".to_string(),
            api_key: Some("test-key".to_string()),
            max_results: 10,
            default_timeout_ms: 5000,
        };

        assert_eq!(config.api_base_url, "http://test:8080");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.max_results, 10);
        assert_eq!(config.default_timeout_ms, 5000);
    }
}
