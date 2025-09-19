use anyhow::Result;
use kotadb::mcp::{config::MCPConfig, MCPServer};
use reqwest::StatusCode;
use tempfile::TempDir;
use tokio::net::TcpListener;

async fn start_test_server() -> Result<(TempDir, std::net::SocketAddr, tokio::task::JoinHandle<()>)>
{
    let temp_dir = TempDir::new()?;
    let mut config = MCPConfig::default();
    config.database.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.mcp.enable_search_tools = false;
    config.mcp.enable_relationship_tools = false;

    let server = MCPServer::new(config).await?;
    let make_service = server
        .streamable_http_router()
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let task = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, make_service).await {
            tracing::error!("Test MCP server terminated unexpectedly: {}", err);
        }
    });

    // Give the server a brief moment to start accepting connections
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Ok((temp_dir, addr, task))
}

#[tokio::test]
async fn streamable_http_enforces_headers_and_serves_initialize() -> Result<()> {
    let (_temp_dir, addr, server_task) = start_test_server().await?;
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/mcp");

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let payload = serde_json::to_vec(&payload)?;

    // Missing required Accept header → 406
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Content-Type", "application/json")
        .body(payload.clone())
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    // Happy path initialize → 200 with session id header
    let resp = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Content-Type", "application/json")
        .body(payload.clone())
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .expect("session header")
        .to_str()
        .expect("header utf8")
        .to_string();
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);

    // Unsupported protocol version → 400
    let resp = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "1999-01-01")
        .header("Content-Type", "application/json")
        .body(payload.clone())
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // GET without SSE Accept header → 406
    let resp = client
        .get(&url)
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json")
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    // GET with proper headers keeps stream open
    let resp = client
        .get(&url)
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "text/event-stream")
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default(),
        "text/event-stream"
    );

    server_task.abort();

    Ok(())
}
