// Services-Only HTTP Server - Clean Implementation for Interface Parity
//
// This module provides a clean HTTP API that exposes KotaDB functionality exclusively
// through the services layer, ensuring complete interface parity with the CLI.
//
// No legacy code, no deprecated endpoints, no document CRUD - pure services architecture.

use anyhow::{Context, Result};
use axum::{
    extract::{Query as AxumQuery, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

#[cfg(all(feature = "mcp-server", feature = "tree-sitter-parsing"))]
use crate::mcp::tools::symbol_tools::SymbolTools;
#[cfg(feature = "mcp-server")]
use crate::mcp::tools::MCPToolRegistry;
#[cfg(feature = "mcp-server")]
use crate::mcp_http_bridge::{create_mcp_bridge_router, McpHttpBridgeState};
use crate::{
    database::Database,
    services::{
        AnalysisService, BenchmarkOptions, BenchmarkService, CallersOptions, ImpactOptions,
        IndexCodebaseOptions, IndexingService, OverviewOptions, SearchOptions, SearchService,
        StatsOptions, StatsService, SymbolSearchOptions, ValidationOptions, ValidationService,
    },
};
use crate::{observability::with_trace_id, Index, Storage};

/// Application state for services-only HTTP server
#[derive(Clone)]
pub struct ServicesAppState {
    pub storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    pub primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    pub trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    pub db_path: PathBuf,
    /// Optional API key service for SaaS functionality
    pub api_key_service: Option<Arc<crate::ApiKeyService>>,
    /// Background job registry for indexing tasks
    pub jobs: Arc<RwLock<HashMap<String, JobStatus>>>,
    /// Simple repository registry persisted under db_path/repositories.json
    pub repositories: Arc<RwLock<Vec<RepositoryRecord>>>,
}

impl ServicesAppState {
    /// Validate that the state is configured correctly for SaaS mode
    pub fn validate_saas_mode(&self) -> Result<(), String> {
        if self.api_key_service.is_none() {
            return Err("SaaS mode requires API key service to be configured".to_string());
        }
        Ok(())
    }

    /// Check if this state is configured for SaaS mode
    pub fn is_saas_mode(&self) -> bool {
        self.api_key_service.is_some()
    }

    fn repo_registry_path(&self) -> PathBuf {
        self.db_path.join("repositories.json")
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub services_enabled: Vec<String>,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Minimal background job status for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    pub id: String,
    pub repo_path: String,
    pub status: String, // queued | running | completed | failed
    pub progress: Option<f32>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub error: Option<String>,
}

/// Simple repository registry record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub url: Option<String>,
    pub last_indexed: Option<String>,
}

/// Stats request parameters
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub basic: Option<bool>,
    pub symbols: Option<bool>,
    pub relationships: Option<bool>,
}

/// Benchmark request
#[derive(Debug, Deserialize)]
pub struct BenchmarkRequest {
    pub operations: Option<usize>,
    pub benchmark_type: Option<String>,
    pub format: Option<String>,
}

/// Validation request
#[derive(Debug, Deserialize)]
pub struct ValidationRequest {
    pub check_integrity: Option<bool>,
    pub check_consistency: Option<bool>,
    pub repair: Option<bool>,
}

/// Index codebase request
#[derive(Debug, Deserialize)]
pub struct IndexCodebaseRequest {
    pub repo_path: String,
    pub prefix: Option<String>,
    pub include_files: Option<bool>,
    pub include_commits: Option<bool>,
    pub extract_symbols: Option<bool>,
}

/// v1 repository registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRepositoryRequest {
    pub path: Option<String>,
    pub git_url: Option<String>,
    pub branch: Option<String>,
}

/// v1 repository registration response
#[derive(Debug, Serialize)]
pub struct RegisterRepositoryResponse {
    pub job_id: String,
    pub repository_id: String,
    pub status: String,
}

/// v1 repository listing response
#[derive(Debug, Serialize)]
pub struct ListRepositoriesResponse {
    pub repositories: Vec<RepositoryRecord>,
}

/// v1 index status response
#[derive(Debug, Serialize)]
pub struct IndexStatusResponse {
    pub job: Option<JobStatus>,
}

/// Codebase overview request
#[derive(Debug, Deserialize)]
pub struct CodebaseOverviewRequest {
    pub format: Option<String>,
    pub top_symbols_limit: Option<usize>,
    pub entry_points_limit: Option<usize>,
}

// ================================================================================================
// ENHANCED API STRUCTURES - Standards Compliant, Non-Breaking
// ================================================================================================

/// Search request with format options and validation
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub search_type: Option<String>,
    pub format: Option<String>, // "simple", "rich", "cli" (default: rich)
}

/// Symbol search request with format options
#[derive(Debug, Deserialize)]
pub struct SymbolSearchRequest {
    pub pattern: String,
    pub limit: Option<usize>,
    pub symbol_type: Option<String>,
    pub format: Option<String>, // "simple", "rich", "cli" (default: rich)
}

/// Callers request with better field names and validation
#[derive(Debug, Deserialize)]
pub struct CallersRequest {
    pub symbol: String, // Intuitive field name
    pub limit: Option<usize>,
    pub format: Option<String>, // "simple", "rich", "cli" (default: rich)
    pub include_indirect: Option<bool>,
}

/// Impact analysis request with better field names
#[derive(Debug, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub symbol: String, // Intuitive field name
    pub limit: Option<usize>,
    pub format: Option<String>, // "simple", "rich", "cli" (default: rich)
    pub max_depth: Option<u32>,
}

/// Simple response format for search operations - CLI-like
#[derive(Debug, Serialize)]
pub struct SimpleSearchResponse {
    pub results: Vec<String>, // Just file paths
    pub total_count: usize,
    pub query_time_ms: u64,
}

/// Simple response format for symbol search - CLI-like  
#[derive(Debug, Serialize)]
pub struct SimpleSymbolResponse {
    pub symbols: Vec<String>, // Just symbol names
    pub total_count: usize,
}

/// Simple response format for analysis operations - CLI-like
#[derive(Debug, Serialize)]
pub struct SimpleAnalysisResponse {
    pub results: Vec<String>, // Just relevant items
    pub total_count: usize,
}

/// CLI-format response that exactly matches command-line output
#[derive(Debug, Serialize)]
pub struct CliFormatResponse {
    pub output: String, // Exact CLI output format
}

/// Standardized API error with comprehensive information
#[derive(Debug, Serialize)]
pub struct StandardApiError {
    pub error_type: String,
    pub message: String,
    pub details: Option<String>,
    pub suggestions: Vec<String>,
    pub error_code: Option<u32>,
}

// ================================================================================================
// SHARED ERROR HANDLING - DRY Principle Compliance
// ================================================================================================

/// Standard result type for API operations
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<StandardApiError>)>;

/// Standardized error handling for JSON parsing failures
fn handle_json_parsing_error(
    error: axum::extract::rejection::JsonRejection,
    endpoint: &str,
) -> (StatusCode, Json<StandardApiError>) {
    let (error_type, message, suggestions) = match error {
        axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => (
            "missing_content_type",
            "Request must include 'Content-Type: application/json' header",
            vec![
                format!("Add header: 'Content-Type: application/json'"),
                format!("Example: curl -H 'Content-Type: application/json' -d '{{\"symbol\":\"test\"}}' /api/{}", endpoint)
            ]
        ),
        axum::extract::rejection::JsonRejection::JsonDataError(_) => (
            "invalid_json_data", 
            "JSON data is invalid or malformed",
            vec![
                "Validate JSON syntax with a JSON validator".to_string(),
                "Ensure all required fields are provided".to_string(),
                "Check for trailing commas or other syntax errors".to_string()
            ]
        ),
        axum::extract::rejection::JsonRejection::JsonSyntaxError(_) => (
            "json_syntax_error",
            "JSON contains syntax errors",
            vec![
                "Validate JSON with an online JSON validator".to_string(),
                "Common issues: missing quotes, trailing commas, unescaped characters".to_string()
            ]
        ),
        _ => (
            "request_parsing_error",
            "Failed to parse request body",
            vec![
                "Ensure valid JSON format".to_string(),
                "Include proper Content-Type header".to_string()
            ]
        )
    };

    (
        StatusCode::BAD_REQUEST,
        Json(StandardApiError {
            error_type: error_type.to_string(),
            message: message.to_string(),
            details: Some(format!("Endpoint: {}", endpoint)),
            suggestions,
            error_code: Some(400),
        }),
    )
}

/// Standardized error handling for service operation failures
fn handle_service_error(
    error: anyhow::Error,
    operation: &str,
) -> (StatusCode, Json<StandardApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(StandardApiError {
            error_type: format!("{}_failed", operation),
            message: error.to_string(),
            details: Some(format!("Operation: {}", operation)),
            suggestions: vec![
                "Check system resources and database connectivity".to_string(),
                "Verify input parameters are valid".to_string(),
                "Contact system administrator if problem persists".to_string(),
            ],
            error_code: Some(500),
        }),
    )
}

/// Standardized validation error handling with helpful messages
fn handle_validation_error(
    field_name: &str,
    message: &str,
    endpoint: &str,
) -> (StatusCode, Json<StandardApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(StandardApiError {
            error_type: "validation_error".to_string(),
            message: format!("Validation failed for '{}': {}", field_name, message),
            details: Some(format!("Endpoint: {}", endpoint)),
            suggestions: vec![
                format!("Ensure '{}' meets validation requirements", field_name),
                "Check API documentation for valid field formats".to_string(),
                "Use ValidatedPath for file paths and ValidatedTitle for titles".to_string(),
            ],
            error_code: Some(400),
        }),
    )
}

/// Create clean services-only HTTP server
pub fn create_services_server(
    storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    db_path: PathBuf,
) -> Router {
    let state = ServicesAppState {
        storage: storage.clone(),
        primary_index: primary_index.clone(),
        trigram_index: trigram_index.clone(),
        db_path: db_path.clone(),
        api_key_service: None, // No authentication for basic services server
        jobs: Arc::new(RwLock::new(HashMap::new())),
        repositories: Arc::new(RwLock::new(load_repositories_from_disk(db_path.as_path()))),
    };

    let base_router = Router::new()
        // Health endpoint
        .route("/health", get(health_check))
        // Versioned v1 endpoints
        .route("/api/v1/analysis/stats", get(get_stats))
        .route("/api/v1/search/code", post(search_code_v1_post))
        .route("/api/v1/search/symbols", post(search_symbols_v1_post))
        .route("/api/v1/symbols/:symbol/callers", get(find_callers_v1_get))
        .route("/api/v1/symbols/:symbol/impact", get(analyze_impact_v1_get))
        .route("/api/v1/symbols", get(list_symbols_v1))
        .route("/api/v1/files/symbols/*path", get(file_symbols_v1))
        .route("/api/v1/repositories", post(register_repository_v1))
        .route("/api/v1/repositories", get(list_repositories_v1))
        .route("/api/v1/index/status", get(index_status_v1))
        // Statistics Service endpoints
        .route("/api/stats", get(get_stats))
        // Benchmark Service endpoints
        .route("/api/benchmark", post(run_benchmark))
        // Validation Service endpoints
        .route("/api/validate", post(validate_database))
        .route("/api/health-check", get(health_check_detailed))
        // Indexing Service endpoints
        .route("/api/index-codebase", post(index_codebase))
        // Search Service endpoints - Enhanced implementations with multi-format support
        .route("/api/search-code", get(search_code_enhanced))
        .route("/api/search-symbols", get(search_symbols_enhanced))
        // Analysis Service endpoints - Enhanced implementations with improved UX
        .route("/api/find-callers", post(find_callers_enhanced))
        .route("/api/analyze-impact", post(analyze_impact_enhanced))
        .route("/api/codebase-overview", get(codebase_overview))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    // Optionally mount MCP bridge without auth for local development
    // Available by default when compiled with mcp-server feature
    #[cfg(feature = "mcp-server")]
    let base_router = {
        let mut registry = MCPToolRegistry::new();
        // Register lightweight text search
        {
            let text_tools = Arc::new(crate::mcp::tools::text_search_tools::TextSearchTools::new(
                trigram_index.clone(),
                storage.clone(),
            ));
            registry = registry.with_text_tools(text_tools);
        }
        #[cfg(feature = "tree-sitter-parsing")]
        {
            let symbol_tools = Arc::new(SymbolTools::new(
                storage.clone(),
                primary_index.clone(),
                trigram_index.clone(),
                db_path.clone(),
            ));
            registry = registry.with_symbol_tools(symbol_tools);
        }
        let mcp_state = McpHttpBridgeState::new(Some(Arc::new(registry)));
        let mcp_router = create_mcp_bridge_router().with_state(mcp_state);
        base_router.merge(mcp_router)
    };

    base_router
}

/// Start the services-only HTTP server
pub async fn start_services_server(
    storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    db_path: PathBuf,
    port: u16,
) -> Result<()> {
    let app = create_services_server(storage, primary_index, trigram_index, db_path);

    // Try to bind to the port with enhanced error handling
    let listener = match TcpListener::bind(&format!("0.0.0.0:{port}")).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to start server on port {}: {}", port, e);

            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!("Port {} is already in use. Try these alternatives:", port);
                error!("   - Use a different port: --port {}", port + 1);

                // Cross-platform command suggestions
                if cfg!(unix) {
                    error!("   - Check port usage: lsof -ti:{}", port);
                    error!("   - Kill process using port: kill $(lsof -ti:{})", port);
                } else {
                    error!("   - Check port usage: netstat -ano | findstr :{}", port);
                    error!("   - Kill process using port: taskkill /PID <PID> /F");
                }
            }

            return Err(e).context(format!(
                "Failed to bind to port {}. Port may be in use or insufficient permissions",
                port
            ));
        }
    };

    info!("🚀 KotaDB Services HTTP Server starting on port {}", port);
    info!("🎯 Clean services-only architecture - no legacy endpoints");
    info!("📄 Available endpoints:");
    info!("   GET    /health                    - Server health check");
    info!("   GET    /api/stats                 - Database statistics (StatsService)");
    info!("   POST   /api/benchmark             - Performance benchmarks (BenchmarkService)");
    info!("   POST   /api/validate              - Database validation (ValidationService)");
    info!("   GET    /api/health-check          - Detailed health check (ValidationService)");
    info!("   POST   /api/index-codebase        - Index repository (IndexingService)");
    info!("   GET    /api/search-code           - Search code content (SearchService)");
    info!("   GET    /api/search-symbols        - Search symbols (SearchService)");
    info!("   POST   /api/find-callers          - Find callers (AnalysisService)");
    info!("   POST   /api/analyze-impact        - Impact analysis (AnalysisService)");
    info!("   GET    /api/codebase-overview     - Codebase overview (AnalysisService)");
    info!("");
    info!("🟢 Server ready at http://localhost:{}", port);
    info!("   Health check: curl http://localhost:{}/health", port);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Create services server with SaaS capabilities (API key authentication)
pub async fn create_services_saas_server(
    storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    db_path: PathBuf,
    api_key_config: crate::ApiKeyConfig,
) -> Result<Router> {
    use crate::auth_middleware::auth_middleware;

    // Initialize API key service
    let api_key_service = Arc::new(crate::ApiKeyService::new(api_key_config).await?);

    let state = ServicesAppState {
        storage,
        primary_index,
        trigram_index,
        db_path: db_path.clone(),
        api_key_service: Some(api_key_service.clone()),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        repositories: Arc::new(RwLock::new(load_repositories_from_disk(db_path.as_path()))),
    };

    // Create authenticated routes (require API key)
    let authenticated_routes = Router::new()
        // v1 endpoints
        .route("/api/v1/analysis/stats", get(get_stats))
        .route("/api/v1/search/code", post(search_code_v1_post))
        .route("/api/v1/search/symbols", post(search_symbols_v1_post))
        .route("/api/v1/symbols/:symbol/callers", get(find_callers_v1_get))
        .route("/api/v1/symbols/:symbol/impact", get(analyze_impact_v1_get))
        .route("/api/v1/symbols", get(list_symbols_v1))
        .route("/api/v1/files/symbols/*path", get(file_symbols_v1))
        .route("/api/v1/repositories", post(register_repository_v1))
        .route("/api/v1/repositories", get(list_repositories_v1))
        .route("/api/v1/index/status", get(index_status_v1))
        // Statistics Service endpoints
        .route("/api/stats", get(get_stats))
        // Benchmark Service endpoints
        .route("/api/benchmark", post(run_benchmark))
        // Validation Service endpoints
        .route("/api/validate", post(validate_database))
        // Indexing Service endpoints
        .route("/api/index-codebase", post(index_codebase))
        // Search Service endpoints - Enhanced implementations with multi-format support
        .route("/api/search-code", get(search_code_enhanced))
        .route("/api/search-symbols", get(search_symbols_enhanced))
        // Analysis Service endpoints - Enhanced implementations with improved UX
        .route("/api/find-callers", post(find_callers_enhanced))
        .route("/api/analyze-impact", post(analyze_impact_enhanced))
        .route("/api/codebase-overview", get(codebase_overview))
        .layer(axum::middleware::from_fn_with_state(
            api_key_service.clone(),
            auth_middleware,
        ));

    // Create internal routes (require internal API key)
    let internal_routes = Router::new()
        .route("/internal/create-api-key", post(create_api_key_handler))
        .layer(axum::middleware::from_fn_with_state(
            api_key_service.clone(),
            auth_middleware, // Will check for internal key
        ));

    Ok(Router::new()
        // Public endpoints (no authentication required)
        .route("/health", get(health_check))
        .route("/api/health-check", get(health_check_detailed))
        // Merge authenticated routes
        .merge(authenticated_routes)
        // Merge internal routes
        .merge(internal_routes)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        ))
}

/// Start services server with SaaS capabilities (API key authentication)
pub async fn start_services_saas_server(
    storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    db_path: PathBuf,
    api_key_config: crate::ApiKeyConfig,
    port: u16,
) -> Result<()> {
    let app = create_services_saas_server(
        storage,
        primary_index,
        trigram_index,
        db_path.clone(),
        api_key_config,
    )
    .await?;

    // Try to bind to the port with enhanced error handling
    let listener = match TcpListener::bind(&format!("0.0.0.0:{port}")).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to start server on port {}: {}", port, e);

            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!("Port {} is already in use. Try these alternatives:", port);
                #[cfg(target_os = "macos")]
                {
                    error!("   - Find process using port: sudo lsof -i :{}", port);
                    error!("   - Kill process using port: sudo kill -9 <PID>");
                }
                #[cfg(target_os = "linux")]
                {
                    error!(
                        "   - Find process using port: sudo netstat -tulpn | grep :{}",
                        port
                    );
                    error!("   - Kill process using port: sudo kill -9 <PID>");
                }
                #[cfg(target_os = "windows")]
                {
                    error!("   - Check port usage: netstat -ano | findstr :{}", port);
                    error!("   - Kill process using port: taskkill /PID <PID> /F");
                }
            }

            return Err(e).context(format!(
                "Failed to bind to port {}. Port may be in use or insufficient permissions",
                port
            ));
        }
    };

    info!("🚀 KotaDB Services SaaS Server starting on port {}", port);
    info!("🔐 API key authentication enabled");
    info!("🎯 Clean services architecture with SaaS capabilities");
    info!("📄 Available endpoints:");
    info!("   GET    /health                    - Server health check (public)");
    info!("   GET    /api/health-check          - Detailed health check (public)");
    info!("   🔐 Authenticated endpoints (require API key):");
    info!("   GET    /api/stats                 - Database statistics (StatsService)");
    info!("   POST   /api/benchmark             - Performance benchmarks (BenchmarkService)");
    info!("   POST   /api/validate              - Database validation (ValidationService)");
    info!("   POST   /api/index-codebase        - Index repository (IndexingService)");
    info!("   GET    /api/search-code           - Search code content (SearchService)");
    info!("   GET    /api/search-symbols        - Search symbols (SearchService)");
    info!("   POST   /api/find-callers          - Find callers (AnalysisService)");
    info!("   POST   /api/analyze-impact        - Impact analysis (AnalysisService)");
    info!("   GET    /api/codebase-overview     - Codebase overview (AnalysisService)");
    info!("   🔒 Internal endpoints (require internal API key):");
    info!("   POST   /internal/create-api-key   - Create new API key");
    info!("");
    info!("🟢 SaaS Server ready at http://localhost:{}", port);
    info!("   Health check: curl http://localhost:{}/health", port);
    info!(
        "   Authenticated example: curl -H 'X-API-Key: your-key' http://localhost:{}/api/stats",
        port
    );

    axum::serve(listener, app).await?;
    Ok(())
}

/// Basic health check
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services_enabled: vec![
            "StatsService".to_string(),
            "BenchmarkService".to_string(),
            "ValidationService".to_string(),
            "IndexingService".to_string(),
            "SearchService".to_string(),
            "AnalysisService".to_string(),
        ],
    })
}

/// Get database statistics via StatsService
async fn get_stats(
    State(state): State<ServicesAppState>,
    AxumQuery(params): AxumQuery<StatsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_stats", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let stats_service = StatsService::new(&database, state.db_path.clone());

        let options = StatsOptions {
            basic: params.basic.unwrap_or(false),
            symbols: params.symbols.unwrap_or(true),
            relationships: params.relationships.unwrap_or(true),
            detailed: false,
            quiet: false,
        };

        stats_service.get_statistics(options).await
    })
    .await;

    match result {
        Ok(stats) => {
            let json_value = serde_json::to_value(stats).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to get stats: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "stats_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Run performance benchmarks via BenchmarkService
async fn run_benchmark(
    State(state): State<ServicesAppState>,
    Json(request): Json<BenchmarkRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_benchmark", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let benchmark_service = BenchmarkService::new(&database, state.db_path.clone());

        let options = BenchmarkOptions {
            operations: request.operations.unwrap_or(1000),
            benchmark_type: request.benchmark_type.unwrap_or_else(|| "all".to_string()),
            format: request.format.unwrap_or_else(|| "json".to_string()),
            max_search_queries: 100,
            quiet: false,
            warm_up_operations: Some(100),
            concurrent_operations: Some(1),
        };

        benchmark_service.run_benchmark(options).await
    })
    .await;

    match result {
        Ok(benchmark_result) => {
            let json_value = serde_json::to_value(benchmark_result).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to run benchmark: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "benchmark_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

// ================================================================================================
// v1 ROUTE HANDLERS - Thin wrappers mapping to existing services
// ================================================================================================

#[derive(Debug, Deserialize)]
struct V1SearchCodeBody {
    pub query: String,
    pub limit: Option<usize>,
    pub format: Option<String>,
}

async fn search_code_v1_post(
    State(state): State<ServicesAppState>,
    request_result: Result<Json<V1SearchCodeBody>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<serde_json::Value> {
    let Json(body) = request_result.map_err(|e| handle_json_parsing_error(e, "v1 search-code"))?;
    let query = body.query.clone();
    let limit = body.limit;
    let format = body.format.clone();
    // Reuse search_code_enhanced by constructing AxumQuery<SearchRequest>
    let request = SearchRequest {
        query,
        limit,
        search_type: Some("medium".to_string()),
        format,
    };

    // Inline the logic of search_code_enhanced to avoid duplicate parsing
    if request.query.trim().is_empty() {
        return Err(handle_validation_error(
            "query",
            "Query cannot be empty",
            "search-code",
        ));
    }

    let result = with_trace_id("api_v1_search_code", async move {
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        let search_service = SearchService::new(&database, state.db_path.clone());
        let options = SearchOptions {
            query: request.query,
            limit: request.limit.unwrap_or(10),
            tags: None,
            context: request.search_type.unwrap_or_else(|| "medium".to_string()),
            quiet: false,
        };
        search_service.search_content(options).await
    })
    .await;

    match result {
        Ok(search_result) => {
            let format = request.format.unwrap_or_else(|| "rich".to_string());
            let response_value = match format.as_str() {
                "simple" => {
                    let file_paths: Vec<String> =
                        if let Some(ref llm_response) = search_result.llm_response {
                            llm_response
                                .results
                                .iter()
                                .map(|doc| doc.path.clone())
                                .collect()
                        } else {
                            search_result
                                .documents
                                .iter()
                                .map(|doc| doc.path.to_string())
                                .collect()
                        };
                    serde_json::to_value(SimpleSearchResponse {
                        results: file_paths,
                        total_count: search_result.total_count,
                        query_time_ms: 0,
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?
                }
                "cli" => {
                    let cli_output = format_search_as_cli(&search_result);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?
                }
                _ => serde_json::to_value(search_result)
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?,
            };
            Ok(Json(response_value))
        }
        Err(e) => Err(handle_service_error(e, "search_code")),
    }
}

#[derive(Debug, Deserialize)]
struct V1SearchSymbolsBody {
    pub pattern: String,
    pub limit: Option<usize>,
    pub symbol_type: Option<String>,
    pub format: Option<String>,
}

async fn search_symbols_v1_post(
    State(state): State<ServicesAppState>,
    request_result: Result<Json<V1SearchSymbolsBody>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<serde_json::Value> {
    let Json(body) =
        request_result.map_err(|e| handle_json_parsing_error(e, "v1 search-symbols"))?;
    if body.pattern.trim().is_empty() {
        return Err(handle_validation_error(
            "pattern",
            "Pattern cannot be empty",
            "search-symbols",
        ));
    }

    let result = with_trace_id("api_v1_search_symbols", async move {
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        let search_service = SearchService::new(&database, state.db_path.clone());
        let options = SymbolSearchOptions {
            pattern: body.pattern,
            limit: body.limit.unwrap_or(25),
            symbol_type: body.symbol_type,
            quiet: false,
        };
        search_service.search_symbols(options).await
    })
    .await;

    match result {
        Ok(symbol_result) => {
            let format = body.format.unwrap_or_else(|| "rich".to_string());
            let response_value = match format.as_str() {
                "simple" => {
                    let names: Vec<String> = symbol_result
                        .matches
                        .iter()
                        .map(|m| m.name.clone())
                        .collect();
                    serde_json::to_value(SimpleSymbolResponse {
                        symbols: names,
                        total_count: symbol_result.total_symbols,
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?
                }
                "cli" => {
                    let cli_output = format_symbols_as_cli(&symbol_result);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?
                }
                _ => serde_json::to_value(symbol_result)
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?,
            };
            Ok(Json(response_value))
        }
        Err(e) => Err(handle_service_error(e, "symbol_search")),
    }
}

/// GET /api/v1/symbols/:symbol/callers
async fn find_callers_v1_get(
    State(state): State<ServicesAppState>,
    axum::extract::Path(symbol): axum::extract::Path<String>,
    AxumQuery(q): AxumQuery<CallersRequest>,
) -> ApiResult<serde_json::Value> {
    if symbol.trim().is_empty() {
        return Err(handle_validation_error(
            "symbol",
            "Symbol name cannot be empty",
            "symbols/:symbol/callers",
        ));
    }

    let result = with_trace_id("api_v1_find_callers", async move {
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());
        let options = CallersOptions {
            target: symbol,
            limit: q.limit,
            quiet: false,
        };
        analysis_service.find_callers(options).await
    })
    .await;

    match result {
        Ok(callers_result) => {
            Ok(Json(serde_json::to_value(callers_result).map_err(|e| {
                handle_service_error(anyhow::anyhow!(e), "find_callers")
            })?))
        }
        Err(e) => Err(handle_service_error(e, "find_callers")),
    }
}

/// GET /api/v1/symbols/:symbol/impact
async fn analyze_impact_v1_get(
    State(state): State<ServicesAppState>,
    axum::extract::Path(symbol): axum::extract::Path<String>,
    AxumQuery(q): AxumQuery<ImpactAnalysisRequest>,
) -> ApiResult<serde_json::Value> {
    if symbol.trim().is_empty() {
        return Err(handle_validation_error(
            "symbol",
            "Symbol name cannot be empty",
            "symbols/:symbol/impact",
        ));
    }

    let result = with_trace_id("api_v1_analyze_impact", async move {
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());
        let options = ImpactOptions {
            target: symbol,
            limit: q.limit,
            quiet: false,
        };
        analysis_service.analyze_impact(options).await
    })
    .await;

    match result {
        Ok(impact_result) => {
            Ok(Json(serde_json::to_value(impact_result).map_err(|e| {
                handle_service_error(anyhow::anyhow!(e), "analyze_impact")
            })?))
        }
        Err(e) => Err(handle_service_error(e, "analyze_impact")),
    }
}

/// GET /api/v1/symbols (basic listing)
#[derive(Debug, Deserialize)]
struct ListSymbolsQuery {
    pattern: Option<String>,
    limit: Option<usize>,
    symbol_type: Option<String>,
}

async fn list_symbols_v1(
    State(state): State<ServicesAppState>,
    AxumQuery(q): AxumQuery<ListSymbolsQuery>,
) -> ApiResult<serde_json::Value> {
    let database = Database {
        storage: state.storage.clone(),
        primary_index: state.primary_index.clone(),
        trigram_index: state.trigram_index.clone(),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };
    let search_service = SearchService::new(&database, state.db_path.clone());
    let options = SymbolSearchOptions {
        pattern: q.pattern.unwrap_or_else(|| "*".to_string()),
        limit: q.limit.unwrap_or(50),
        symbol_type: q.symbol_type,
        quiet: false,
    };
    match search_service.search_symbols(options).await {
        Ok(symbol_result) => {
            Ok(Json(serde_json::to_value(symbol_result).map_err(|e| {
                handle_service_error(anyhow::anyhow!(e), "list_symbols")
            })?))
        }
        Err(e) => Err(handle_service_error(e, "list_symbols")),
    }
}

/// GET /api/v1/files/*path -> symbols in file
async fn file_symbols_v1(
    State(state): State<ServicesAppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> ApiResult<serde_json::Value> {
    // Read binary symbols file and filter by file_path
    let symbol_db_path = state.db_path.join("symbols.kota");
    match crate::binary_symbols::BinarySymbolReader::open(&symbol_db_path) {
        Ok(reader) => {
            let mut entries = Vec::new();
            for s in reader.iter_symbols() {
                if let Ok(file_path) = reader.get_symbol_file_path(&s) {
                    if file_path == path {
                        let name = reader.get_symbol_name(&s).unwrap_or_default();
                        entries.push(serde_json::json!({
                            "name": name,
                            "kind": format!("{:?}", s.kind),
                            "start_line": s.start_line,
                            "end_line": s.end_line,
                        }));
                    }
                }
            }
            Ok(Json(serde_json::json!({"file": path, "symbols": entries})))
        }
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(StandardApiError {
                error_type: "symbols_db_missing".into(),
                message: format!("{}", e),
                details: Some("symbols.kota not found or unreadable".into()),
                suggestions: vec!["Run indexing with symbol extraction enabled".into()],
                error_code: Some(404),
            }),
        )),
    }
}

/// POST /api/v1/repositories -> start background indexing job
async fn register_repository_v1(
    State(state): State<ServicesAppState>,
    request_result: Result<
        Json<RegisterRepositoryRequest>,
        axum::extract::rejection::JsonRejection,
    >,
) -> ApiResult<RegisterRepositoryResponse> {
    let Json(body) = request_result.map_err(|e| handle_json_parsing_error(e, "repositories"))?;
    if body.path.is_none() && body.git_url.is_none() {
        return Err(handle_validation_error(
            "path|git_url",
            "Provide either local path or git_url",
            "repositories",
        ));
    }

    // For now, support local path; git clone can be added later
    let repo_path = match (&body.path, &body.git_url) {
        (Some(p), _) => PathBuf::from(p.clone()),
        (None, Some(_git)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(StandardApiError {
                    error_type: "git_url_not_supported".into(),
                    message: "git_url support pending; use local path".into(),
                    details: None,
                    suggestions: vec!["Clone locally and provide path".into()],
                    error_code: Some(400),
                }),
            ));
        }
        _ => unreachable!(),
    };

    let repo_name = repo_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("repo")
        .to_string();
    let repository_id = format!("repo_{}", repo_name);
    let job_id = Uuid::new_v4().to_string();
    let job_id_out = job_id.clone();
    let repository_id_out = repository_id.clone();

    // Create job status
    let mut jobs = state.jobs.write().await;
    jobs.insert(
        job_id.clone(),
        JobStatus {
            id: job_id.clone(),
            repo_path: repo_path.to_string_lossy().into(),
            status: "queued".into(),
            progress: None,
            started_at: Some(now_rfc3339()),
            updated_at: Some(now_rfc3339()),
            error: None,
        },
    );
    drop(jobs);

    // Persist repository record if not present
    {
        let mut repos = state.repositories.write().await;
        if !repos.iter().any(|r| r.id == repository_id) {
            repos.push(RepositoryRecord {
                id: repository_id.clone(),
                name: repo_name.clone(),
                path: repo_path.to_string_lossy().into(),
                url: body.git_url.clone(),
                last_indexed: None,
            });
            save_repositories_to_disk(&state, &repos).await;
        }
    }

    // Spawn background indexing
    let state_clone = state.clone();
    tokio::spawn(async move {
        update_job_status(&state_clone, &job_id, |j| {
            j.status = "running".into();
            j.updated_at = Some(now_rfc3339());
        })
        .await;

        // Execute indexing
        let database = Database {
            storage: state_clone.storage.clone(),
            primary_index: state_clone.primary_index.clone(),
            trigram_index: state_clone.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        let indexing = IndexingService::new(&database, state_clone.db_path.clone());
        let options = IndexCodebaseOptions {
            repo_path: repo_path.clone(),
            prefix: "repos".into(),
            include_files: true,
            include_commits: true,
            max_file_size_mb: 10,
            max_memory_mb: None,
            max_parallel_files: None,
            enable_chunking: true,
            extract_symbols: Some(true),
            no_symbols: false,
            quiet: false,
        };
        match indexing.index_codebase(options).await {
            Ok(_res) => {
                update_job_status(&state_clone, &job_id, |j| {
                    j.status = "completed".into();
                    j.updated_at = Some(now_rfc3339());
                })
                .await;
                // Update repository last_indexed
                let mut repos = state_clone.repositories.write().await;
                if let Some(r) = repos.iter_mut().find(|r| r.id == repository_id) {
                    r.last_indexed = Some(now_rfc3339());
                }
                save_repositories_to_disk(&state_clone, &repos).await;
            }
            Err(e) => {
                update_job_status(&state_clone, &job_id, |j| {
                    j.status = "failed".into();
                    j.error = Some(e.to_string());
                    j.updated_at = Some(now_rfc3339());
                })
                .await;
            }
        }
    });

    Ok(Json(RegisterRepositoryResponse {
        job_id: job_id_out,
        repository_id: repository_id_out,
        status: "accepted".into(),
    }))
}

/// GET /api/v1/repositories
async fn list_repositories_v1(
    State(state): State<ServicesAppState>,
) -> ApiResult<ListRepositoriesResponse> {
    let repos = state.repositories.read().await.clone();
    Ok(Json(ListRepositoriesResponse {
        repositories: repos,
    }))
}

/// GET /api/v1/index/status?job_id=...
#[derive(Debug, Deserialize)]
struct IndexStatusQuery {
    job_id: String,
}

async fn index_status_v1(
    State(state): State<ServicesAppState>,
    AxumQuery(q): AxumQuery<IndexStatusQuery>,
) -> ApiResult<IndexStatusResponse> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&q.job_id).cloned();
    Ok(Json(IndexStatusResponse { job }))
}

// Helpers ----------------------------------------------------------------------------------------

fn now_rfc3339() -> String {
    // Use std time to avoid heavy deps; best-effort formatting
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

async fn update_job_status<F: FnOnce(&mut JobStatus)>(
    state: &ServicesAppState,
    job_id: &str,
    f: F,
) {
    let mut jobs = state.jobs.write().await;
    if let Some(j) = jobs.get_mut(job_id) {
        f(j);
    }
}

fn load_repositories_from_disk(db_path: &std::path::Path) -> Vec<RepositoryRecord> {
    let path = db_path.join("repositories.json");
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn save_repositories_to_disk(state: &ServicesAppState, repos: &Vec<RepositoryRecord>) {
    let path = state.repo_registry_path();
    if let Ok(s) = serde_json::to_string_pretty(repos) {
        if let Err(e) = tokio::fs::write(path, s).await {
            warn!("Failed to persist repositories.json: {}", e);
        }
    }
}

/// Validate database via ValidationService
async fn validate_database(
    State(state): State<ServicesAppState>,
    Json(request): Json<ValidationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_validate", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let validation_service = ValidationService::new(&database, state.db_path.clone());

        let options = ValidationOptions {
            check_integrity: request.check_integrity.unwrap_or(true),
            check_consistency: request.check_consistency.unwrap_or(true),
            check_performance: false,
            deep_scan: false,
            repair_issues: request.repair.unwrap_or(false),
            quiet: false,
        };

        validation_service.validate_database(options).await
    })
    .await;

    match result {
        Ok(validation_result) => {
            let json_value = serde_json::to_value(validation_result).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to validate database: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "validation_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Detailed health check via ValidationService
async fn health_check_detailed(
    State(state): State<ServicesAppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_health_check", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let validation_service = ValidationService::new(&database, state.db_path.clone());

        let options = ValidationOptions {
            check_integrity: true,
            check_consistency: true,
            check_performance: false,
            deep_scan: false,
            repair_issues: false,
            quiet: false,
        };

        validation_service.validate_database(options).await
    })
    .await;

    match result {
        Ok(health_result) => {
            let json_value = serde_json::to_value(health_result).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed health check: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "health_check_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Index codebase via IndexingService
async fn index_codebase(
    State(state): State<ServicesAppState>,
    Json(request): Json<IndexCodebaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_index_codebase", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let indexing_service = IndexingService::new(&database, state.db_path.clone());

        let options = IndexCodebaseOptions {
            repo_path: PathBuf::from(request.repo_path),
            prefix: request.prefix.unwrap_or_else(|| "repos".to_string()),
            include_files: request.include_files.unwrap_or(true),
            include_commits: request.include_commits.unwrap_or(true),
            max_file_size_mb: 10,
            max_memory_mb: None,
            max_parallel_files: None,
            enable_chunking: true,
            extract_symbols: request.extract_symbols,
            no_symbols: false,
            quiet: false,
        };

        indexing_service.index_codebase(options).await
    })
    .await;

    match result {
        Ok(index_result) => {
            let json_value = serde_json::to_value(index_result).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to index codebase: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "indexing_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Get codebase overview via AnalysisService
async fn codebase_overview(
    State(state): State<ServicesAppState>,
    AxumQuery(request): AxumQuery<CodebaseOverviewRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_codebase_overview", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let analysis_service = AnalysisService::new(&database, state.db_path.clone());

        let options = OverviewOptions {
            format: request.format.unwrap_or_else(|| "json".to_string()),
            top_symbols_limit: request.top_symbols_limit.unwrap_or(10),
            entry_points_limit: request.entry_points_limit.unwrap_or(10),
            quiet: false,
        };

        analysis_service.generate_overview(options).await
    })
    .await;

    match result {
        Ok(overview_result) => {
            let json_value = serde_json::to_value(overview_result).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "serialization_failed".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to get codebase overview: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "codebase_overview_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

// ================================================================================================
// ENHANCED V2 API ENDPOINTS - Standards Compliant Implementation
// ================================================================================================

/// Search code endpoint with format options and validation
async fn search_code_enhanced(
    State(state): State<ServicesAppState>,
    AxumQuery(request): AxumQuery<SearchRequest>,
) -> ApiResult<serde_json::Value> {
    // Validate query input using validation layer
    if request.query.trim().is_empty() {
        return Err(handle_validation_error(
            "query",
            "Query cannot be empty",
            "search-code",
        ));
    }

    let result = with_trace_id("api_enhanced_search_code", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let search_service = SearchService::new(&database, state.db_path.clone());

        let options = SearchOptions {
            query: request.query,
            limit: request.limit.unwrap_or(10),
            tags: None,
            context: request.search_type.unwrap_or_else(|| "medium".to_string()),
            quiet: false,
        };

        search_service.search_content(options).await
    })
    .await;

    match result {
        Ok(search_result) => {
            let format = request.format.unwrap_or_else(|| "rich".to_string());

            // Convert to appropriate format based on request
            let response_value = match format.as_str() {
                "simple" => {
                    let file_paths: Vec<String> =
                        if let Some(ref llm_response) = search_result.llm_response {
                            // Extract file paths from LLM response
                            llm_response
                                .results
                                .iter()
                                .map(|doc| doc.path.clone())
                                .collect()
                        } else {
                            // Extract file paths from regular documents
                            search_result
                                .documents
                                .iter()
                                .map(|doc| doc.path.to_string())
                                .collect()
                        };

                    serde_json::to_value(SimpleSearchResponse {
                        results: file_paths,
                        total_count: search_result.total_count,
                        query_time_ms: 0, // TODO: Add timing
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?
                }
                "cli" => {
                    let cli_output = format_search_as_cli(&search_result);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?
                }
                _ => {
                    // "rich" format (default)
                    serde_json::to_value(search_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "search_code"))?
                }
            };

            Ok(Json(response_value))
        }
        Err(e) => {
            tracing::warn!("Enhanced search failed: {}", e);
            Err(handle_service_error(e, "search_code"))
        }
    }
}

/// Symbol search endpoint with format options
async fn search_symbols_enhanced(
    State(state): State<ServicesAppState>,
    AxumQuery(request): AxumQuery<SymbolSearchRequest>,
) -> ApiResult<serde_json::Value> {
    // Validate pattern input
    if request.pattern.trim().is_empty() {
        return Err(handle_validation_error(
            "pattern",
            "Pattern cannot be empty",
            "search-symbols",
        ));
    }

    let result = with_trace_id("api_enhanced_search_symbols", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let search_service = SearchService::new(&database, state.db_path.clone());

        let options = SymbolSearchOptions {
            pattern: request.pattern,
            limit: request.limit.unwrap_or(25),
            symbol_type: request.symbol_type,
            quiet: false,
        };

        search_service.search_symbols(options).await
    })
    .await;

    match result {
        Ok(symbol_result) => {
            let format = request.format.unwrap_or_else(|| "rich".to_string());

            // Convert to appropriate format based on request
            let response_value = match format.as_str() {
                "simple" => {
                    let symbol_names: Vec<String> = symbol_result
                        .matches
                        .iter()
                        .map(|m| m.name.clone())
                        .collect();

                    serde_json::to_value(SimpleSymbolResponse {
                        symbols: symbol_names,
                        total_count: symbol_result.total_symbols,
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?
                }
                "cli" => {
                    let cli_output = format_symbols_as_cli(&symbol_result);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?
                }
                _ => {
                    // "rich" format (default)
                    serde_json::to_value(symbol_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "symbol_search"))?
                }
            };

            Ok(Json(response_value))
        }
        Err(e) => {
            tracing::warn!("Enhanced symbol search failed: {}", e);
            Err(handle_service_error(e, "symbol_search"))
        }
    }
}

/// Find callers endpoint with format options and validation
async fn find_callers_enhanced(
    State(state): State<ServicesAppState>,
    request_result: Result<Json<CallersRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<serde_json::Value> {
    // Handle JSON parsing errors using shared error handler
    let Json(request) = request_result.map_err(|e| handle_json_parsing_error(e, "find-callers"))?;

    // Validate symbol input using validation layer
    if request.symbol.trim().is_empty() {
        return Err(handle_validation_error(
            "symbol",
            "Symbol name cannot be empty",
            "find-callers",
        ));
    }

    let result = with_trace_id("api_enhanced_find_callers", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());

        let options = CallersOptions {
            target: request.symbol,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.find_callers(options).await
    })
    .await;

    match result {
        Ok(callers_result) => {
            let format = request.format.unwrap_or_else(|| "rich".to_string());

            // Convert to appropriate format based on request
            let response_value = match format.as_str() {
                "simple" => {
                    // Extract just the relevant caller information
                    let simple_results: Vec<String> =
                        if let Ok(json_val) = serde_json::to_value(&callers_result) {
                            extract_simple_caller_results(&json_val)
                        } else {
                            vec!["Error parsing results".to_string()]
                        };
                    let count = simple_results.len();

                    serde_json::to_value(SimpleAnalysisResponse {
                        results: simple_results,
                        total_count: count,
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "find_callers"))?
                }
                "cli" => {
                    let json_val = serde_json::to_value(&callers_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "find_callers"))?;
                    let cli_output = format_callers_as_cli(&json_val);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "find_callers"))?
                }
                _ => {
                    // "rich" format (default)
                    serde_json::to_value(callers_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "find_callers"))?
                }
            };

            Ok(Json(response_value))
        }
        Err(e) => {
            tracing::warn!("Enhanced find callers failed: {}", e);
            Err(handle_service_error(e, "find_callers"))
        }
    }
}

/// Impact analysis endpoint with format options and validation  
async fn analyze_impact_enhanced(
    State(state): State<ServicesAppState>,
    request_result: Result<Json<ImpactAnalysisRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<serde_json::Value> {
    // Handle JSON parsing errors using shared error handler
    let Json(request) =
        request_result.map_err(|e| handle_json_parsing_error(e, "analyze-impact"))?;

    // Validate symbol input using validation layer
    if request.symbol.trim().is_empty() {
        return Err(handle_validation_error(
            "symbol",
            "Symbol name cannot be empty",
            "analyze-impact",
        ));
    }

    let result = with_trace_id("api_enhanced_analyze_impact", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());

        let options = ImpactOptions {
            target: request.symbol,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.analyze_impact(options).await
    })
    .await;

    match result {
        Ok(impact_result) => {
            let format = request.format.unwrap_or_else(|| "rich".to_string());

            // Convert to appropriate format based on request
            let response_value = match format.as_str() {
                "simple" => {
                    // Extract just the relevant impact information
                    let simple_results: Vec<String> =
                        if let Ok(json_val) = serde_json::to_value(&impact_result) {
                            extract_simple_impact_results(&json_val)
                        } else {
                            vec!["Error parsing results".to_string()]
                        };
                    let count = simple_results.len();

                    serde_json::to_value(SimpleAnalysisResponse {
                        results: simple_results,
                        total_count: count,
                    })
                    .map_err(|e| handle_service_error(anyhow::anyhow!(e), "analyze_impact"))?
                }
                "cli" => {
                    let json_val = serde_json::to_value(&impact_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "analyze_impact"))?;
                    let cli_output = format_impact_as_cli(&json_val);
                    serde_json::to_value(CliFormatResponse { output: cli_output })
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "analyze_impact"))?
                }
                _ => {
                    // "rich" format (default)
                    serde_json::to_value(impact_result)
                        .map_err(|e| handle_service_error(anyhow::anyhow!(e), "analyze_impact"))?
                }
            };

            Ok(Json(response_value))
        }
        Err(e) => {
            tracing::warn!("Enhanced impact analysis failed: {}", e);
            Err(handle_service_error(e, "analyze_impact"))
        }
    }
}

// ================================================================================================
// FORMAT CONVERSION HELPERS - CLI and Simple Format Support
// ================================================================================================

/// Format search results as CLI-style output
fn format_search_as_cli(search_result: &crate::services::search_service::SearchResult) -> String {
    let mut output = String::new();

    if let Some(ref llm_response) = search_result.llm_response {
        output.push_str(&format!("Query: {}\n\n", llm_response.query));

        for doc in &llm_response.results {
            output.push_str(&format!("📄 {}\n", doc.path));
            output.push_str(&format!("   {}\n", doc.content_snippet));
        }
    } else {
        for doc in &search_result.documents {
            output.push_str(&format!("📄 {}\n", doc.path));
        }
    }

    output.push_str(&format!("\nTotal matches: {}", search_result.total_count));
    output
}

/// Format symbol search results as CLI-style output
fn format_symbols_as_cli(symbol_result: &crate::services::search_service::SymbolResult) -> String {
    let mut output = String::new();

    for symbol_match in &symbol_result.matches {
        output.push_str(&format!(
            "🔍 {} ({}:{})\n   Type: {}\n",
            symbol_match.name, symbol_match.file_path, symbol_match.start_line, symbol_match.kind
        ));
    }

    output.push_str(&format!(
        "\nTotal symbols found: {}/{}",
        symbol_result.matches.len(),
        symbol_result.total_symbols
    ));
    output
}

/// Format callers results as CLI-style output
fn format_callers_as_cli(callers_result: &serde_json::Value) -> String {
    // TODO: Extract caller information from service result and format as CLI
    // This will depend on the structure of CallersResult from AnalysisService
    if let Some(callers) = callers_result.get("callers") {
        if let Some(callers_array) = callers.as_array() {
            let mut output = String::new();
            for caller in callers_array {
                if let (Some(name), Some(file), Some(line)) = (
                    caller.get("name").and_then(|v| v.as_str()),
                    caller.get("file").and_then(|v| v.as_str()),
                    caller.get("line").and_then(|v| v.as_u64()),
                ) {
                    output.push_str(&format!("📞 {} ({}:{})\n", name, file, line));
                }
            }
            return output;
        }
    }

    // Fallback to JSON string representation
    serde_json::to_string_pretty(callers_result).unwrap_or_else(|_| "No callers found".to_string())
}

/// Format impact analysis results as CLI-style output
fn format_impact_as_cli(impact_result: &serde_json::Value) -> String {
    // TODO: Extract impact information from service result and format as CLI
    // This will depend on the structure of ImpactResult from AnalysisService
    if let Some(impact) = impact_result.get("impacted_files") {
        if let Some(impact_array) = impact.as_array() {
            let mut output = String::new();
            for file in impact_array {
                if let Some(path) = file.as_str() {
                    output.push_str(&format!("⚡ {}\n", path));
                }
            }
            return output;
        }
    }

    // Fallback to JSON string representation
    serde_json::to_string_pretty(impact_result).unwrap_or_else(|_| "No impact found".to_string())
}

/// Extract simple caller results for simple format
fn extract_simple_caller_results(json_val: &serde_json::Value) -> Vec<String> {
    let mut results = Vec::new();

    if let Some(callers) = json_val.get("callers") {
        if let Some(callers_array) = callers.as_array() {
            for caller in callers_array {
                if let Some(name) = caller.get("name").and_then(|v| v.as_str()) {
                    results.push(name.to_string());
                }
            }
        }
    }

    if results.is_empty() {
        results.push("No callers found".to_string());
    }

    results
}

/// Extract simple impact results for simple format
fn extract_simple_impact_results(json_val: &serde_json::Value) -> Vec<String> {
    let mut results = Vec::new();

    if let Some(impact) = json_val.get("impacted_files") {
        if let Some(impact_array) = impact.as_array() {
            for file in impact_array {
                if let Some(path) = file.as_str() {
                    results.push(path.to_string());
                }
            }
        }
    }

    if results.is_empty() {
        results.push("No impact found".to_string());
    }

    results
}

/// Create API key handler for internal endpoints
async fn create_api_key_handler(
    State(state): State<ServicesAppState>,
    Json(request): Json<crate::api_keys::CreateApiKeyRequest>,
) -> Result<
    Json<crate::api_keys::CreateApiKeyResponse>,
    (StatusCode, Json<crate::http_types::ErrorResponse>),
> {
    use crate::http_types::ErrorResponse;
    use crate::observability::with_trace_id;

    // Extract API key service from state
    let api_key_service = match &state.api_key_service {
        Some(service) => service.clone(),
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_server_error(
                    "API key service not configured",
                )),
            ));
        }
    };

    let result = with_trace_id("create_api_key_internal", async move {
        api_key_service.create_api_key(request).await
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Failed to create API key: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_server_error(e.to_string())),
            ))
        }
    }
}
