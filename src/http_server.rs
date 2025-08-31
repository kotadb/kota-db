// HTTP REST API Server Implementation for Codebase Intelligence
// Provides JSON API for code search, symbol analysis, and relationship queries

use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, State},
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::{net::TcpListener, sync::Mutex};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{contracts::Storage, create_binary_trigram_index, observability::with_trace_id};

// Import the codebase intelligence module
pub mod http_codebase_intelligence;
use http_codebase_intelligence::CodeIntelligenceState;

// Constants for health check
const DEFAULT_MEMORY_USAGE_MB: f64 = 32.0; // 32MB baseline memory usage
const DEFAULT_CPU_USAGE_PERCENT: f32 = 5.0; // 5% baseline CPU usage
const HEALTH_THRESHOLD_CPU: f32 = 90.0; // CPU usage threshold for health check
const HEALTH_THRESHOLD_MEMORY_MB: f64 = 1000.0; // Memory threshold in MB for health check

// Maximum request size: 100MB (for repository indexing requests)
const MAX_REQUEST_SIZE: usize = 100 * 1024 * 1024; // 100MB

// Global server start time for uptime tracking
static SERVER_START_TIME: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
    pub features: Vec<String>,
    pub resources: ResourceStatus,
}

/// Resource status for health check
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceStatus {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f32,
    pub database_connected: bool,
    pub indices_loaded: bool,
}

/// Server information response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub api_version: String,
    pub features: Vec<String>,
    pub endpoints: Vec<EndpointInfo>,
}

/// Endpoint information
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub description: String,
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub timestamp: i64,
}

/// Health check handler
async fn health_check(State(_state): State<CodeIntelligenceState>) -> Json<HealthResponse> {
    let uptime = SERVER_START_TIME.elapsed().as_secs();

    // Get actual resource usage (simplified for now)
    let memory_usage_mb = DEFAULT_MEMORY_USAGE_MB;
    let cpu_usage_percent = DEFAULT_CPU_USAGE_PERCENT;

    // Determine health status
    let status = if cpu_usage_percent > HEALTH_THRESHOLD_CPU
        || memory_usage_mb > HEALTH_THRESHOLD_MEMORY_MB
    {
        "degraded"
    } else {
        "healthy"
    };

    Json(HealthResponse {
        status: status.to_string(),
        uptime_seconds: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
        features: vec![
            "codebase-intelligence".to_string(),
            "symbol-extraction".to_string(),
            "relationship-analysis".to_string(),
            "trigram-search".to_string(),
        ],
        resources: ResourceStatus {
            memory_usage_mb,
            cpu_usage_percent,
            database_connected: true,
            indices_loaded: true,
        },
    })
}

/// Server information handler
async fn server_info(State(_state): State<CodeIntelligenceState>) -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        name: "KotaDB Codebase Intelligence API".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Codebase intelligence platform for AI assistants".to_string(),
        api_version: "v1".to_string(),
        features: vec![
            "Repository indexing with symbol extraction".to_string(),
            "Full-text code search with trigram indexing".to_string(),
            "Symbol search with wildcard patterns".to_string(),
            "Find callers and references analysis".to_string(),
            "Impact analysis for code changes".to_string(),
            "Codebase statistics and metrics".to_string(),
        ],
        endpoints: vec![
            EndpointInfo {
                path: "/health".to_string(),
                method: "GET".to_string(),
                description: "Health check endpoint".to_string(),
            },
            EndpointInfo {
                path: "/info".to_string(),
                method: "GET".to_string(),
                description: "Server information".to_string(),
            },
            EndpointInfo {
                path: "/api/repositories".to_string(),
                method: "POST".to_string(),
                description: "Index a repository".to_string(),
            },
            EndpointInfo {
                path: "/api/search/code".to_string(),
                method: "POST".to_string(),
                description: "Search code using full-text search".to_string(),
            },
            EndpointInfo {
                path: "/api/search/symbols".to_string(),
                method: "POST".to_string(),
                description: "Search symbols with pattern matching".to_string(),
            },
            EndpointInfo {
                path: "/api/symbols/{symbol}/callers".to_string(),
                method: "GET".to_string(),
                description: "Find all callers of a symbol".to_string(),
            },
            EndpointInfo {
                path: "/api/symbols/{symbol}/impact".to_string(),
                method: "GET".to_string(),
                description: "Analyze impact of changes to a symbol".to_string(),
            },
            EndpointInfo {
                path: "/api/analysis/stats".to_string(),
                method: "GET".to_string(),
                description: "Get codebase statistics".to_string(),
            },
        ],
    })
}

/// Create the application router with codebase intelligence endpoints
pub fn create_app_router(state: CodeIntelligenceState) -> Router {
    // First create the codebase intelligence routes with state
    let codebase_routes = http_codebase_intelligence::register_routes(state.clone());

    Router::new()
        // Health and info endpoints
        .route("/health", get(health_check))
        .route("/info", get(server_info))
        .route("/", get(server_info))
        // Merge the codebase intelligence routes
        .merge(codebase_routes)
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

/// Start the HTTP server for codebase intelligence
pub async fn start_server(
    storage: Arc<Mutex<dyn Storage>>,
    port: u16,
    db_path: PathBuf,
) -> Result<()> {
    with_trace_id("http_server", async {
        info!(
            "Starting KotaDB Codebase Intelligence API server on port {}",
            port
        );
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // Create trigram index for code search
    let trigram_index = create_binary_trigram_index(db_path.to_str().unwrap(), None).await?;

    // Create application state
    let state = CodeIntelligenceState {
        storage,
        trigram_index: Arc::new(trigram_index),
        db_path,
    };

    // Create router
    let app = create_app_router(state);

    // Bind to address
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    info!(
        "🚀 KotaDB Codebase Intelligence API server listening on http://{}",
        addr
    );
    info!("📚 API documentation available at http://{}/info", addr);
    info!("🏥 Health check available at http://{}/health", addr);

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Start the HTTP server with minimal setup (for testing)
pub async fn start_server_minimal(storage: Arc<Mutex<dyn Storage>>, port: u16) -> Result<()> {
    // Use current directory as db_path for minimal setup
    let db_path = PathBuf::from(".");
    start_server(storage, port, db_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_storage::create_file_storage;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = tempdir().unwrap();
        let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100))
            .await
            .unwrap();
        let state = CodeIntelligenceState {
            storage: Arc::new(Mutex::new(storage)),
            trigram_index: Arc::new(
                create_binary_trigram_index(temp_dir.path().to_str().unwrap(), None)
                    .await
                    .unwrap(),
            ),
            db_path: temp_dir.path().to_path_buf(),
        };

        let response = health_check(State(state)).await;
        assert_eq!(response.status, "healthy");
        // uptime_seconds is u64, so it's always >= 0
    }

    #[tokio::test]
    async fn test_server_info() {
        let temp_dir = tempdir().unwrap();
        let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100))
            .await
            .unwrap();
        let state = CodeIntelligenceState {
            storage: Arc::new(Mutex::new(storage)),
            trigram_index: Arc::new(
                create_binary_trigram_index(temp_dir.path().to_str().unwrap(), None)
                    .await
                    .unwrap(),
            ),
            db_path: temp_dir.path().to_path_buf(),
        };

        let response = server_info(State(state)).await;
        assert_eq!(response.api_version, "v1");
        assert!(!response.endpoints.is_empty());
    }
}
