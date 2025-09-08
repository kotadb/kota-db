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
use tracing::{error, info};

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

/// Search request
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub search_type: Option<String>,
    pub format: Option<String>, // "simple" or "rich" (default)
}

/// Symbol search request
#[derive(Debug, Deserialize)]
pub struct SymbolSearchRequest {
    pub pattern: String,
    pub limit: Option<usize>,
    pub symbol_type: Option<String>,
    pub format: Option<String>, // "simple" or "rich" (default)
}

/// Callers request - supports both 'target' and 'symbol' field names for backward compatibility
#[derive(Debug, Deserialize)]
pub struct CallersRequest {
    pub target: Option<String>,
    pub symbol: Option<String>, // More intuitive field name
    pub limit: Option<usize>,
    pub format: Option<String>, // "simple" or "rich" (default)
}

/// Impact analysis request - supports both 'target' and 'symbol' field names for backward compatibility
#[derive(Debug, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub target: Option<String>,
    pub symbol: Option<String>, // More intuitive field name
    pub limit: Option<usize>,
    pub format: Option<String>, // "simple" or "rich" (default)
}

/// Codebase overview request
#[derive(Debug, Deserialize)]
pub struct CodebaseOverviewRequest {
    pub format: Option<String>,
    pub top_symbols_limit: Option<usize>,
    pub entry_points_limit: Option<usize>,
}

/// Simple response for code search - CLI-like format
#[derive(Debug, Serialize)]
pub struct SimpleSearchResponse {
    pub results: Vec<String>, // Just file paths, like CLI
}

/// Simple response for symbol search - CLI-like format
#[derive(Debug, Serialize)]
pub struct SimpleSymbolResponse {
    pub symbols: Vec<String>, // Just symbol names, like CLI
}

/// Simple response for callers/impact - CLI-like format
#[derive(Debug, Serialize)]
pub struct SimpleAnalysisResponse {
    pub results: Vec<String>, // Just the relevant items, like CLI
}

/// Helper function to extract target from request (supports both 'target' and 'symbol' fields)
fn extract_target_from_callers_request(request: &CallersRequest) -> Result<String, String> {
    if let Some(target) = &request.target {
        Ok(target.clone())
    } else if let Some(symbol) = &request.symbol {
        Ok(symbol.clone())
    } else {
        Err("Either 'target' or 'symbol' field must be provided".to_string())
    }
}

/// Helper function to extract target from impact analysis request
fn extract_target_from_impact_request(request: &ImpactAnalysisRequest) -> Result<String, String> {
    if let Some(target) = &request.target {
        Ok(target.clone())
    } else if let Some(symbol) = &request.symbol {
        Ok(symbol.clone())
    } else {
        Err("Either 'target' or 'symbol' field must be provided".to_string())
    }
}

/// Convert rich search result to simple format
fn convert_to_simple_search_response(
    rich_result: crate::services::SearchResult,
) -> SimpleSearchResponse {
    let results = if let Some(llm_response) = &rich_result.llm_response {
        // Extract paths from LLM response results (the actual data)
        tracing::debug!(
            "Converting search result from llm_response.results ({} items)",
            llm_response.results.len()
        );
        llm_response
            .results
            .iter()
            .map(|result| result.path.clone())
            .collect()
    } else if !rich_result.documents.is_empty() {
        // Fallback to documents field if available
        tracing::warn!(
            "API simple format fallback: using documents field instead of llm_response ({} items)",
            rich_result.documents.len()
        );
        rich_result
            .documents
            .into_iter()
            .map(|doc| doc.path.as_str().to_string())
            .collect()
    } else {
        // Return empty results
        tracing::info!("API simple format: no results found in either llm_response or documents");
        vec![]
    };
    SimpleSearchResponse { results }
}

/// Convert rich symbol result to simple format
fn convert_to_simple_symbol_response(
    rich_result: crate::services::SymbolResult,
) -> SimpleSymbolResponse {
    let symbols = rich_result
        .matches
        .into_iter()
        .map(|symbol| symbol.name)
        .collect();
    SimpleSymbolResponse { symbols }
}

/// Create clean services-only HTTP server
pub fn create_services_server(
    storage: Arc<tokio::sync::Mutex<dyn Storage>>,
    primary_index: Arc<tokio::sync::Mutex<dyn Index>>,
    trigram_index: Arc<tokio::sync::Mutex<dyn Index>>,
    db_path: PathBuf,
) -> Router {
    let state = ServicesAppState {
        storage,
        primary_index,
        trigram_index,
        db_path,
    };

    Router::new()
        // Health endpoint
        .route("/health", get(health_check))
        // Statistics Service endpoints
        .route("/api/stats", get(get_stats))
        // Benchmark Service endpoints
        .route("/api/benchmark", post(run_benchmark))
        // Validation Service endpoints
        .route("/api/validate", post(validate_database))
        .route("/api/health-check", get(health_check_detailed))
        // Indexing Service endpoints
        .route("/api/index-codebase", post(index_codebase))
        // Search Service endpoints
        .route("/api/search-code", get(search_code))
        .route("/api/search-symbols", get(search_symbols))
        // Analysis Service endpoints
        .route("/api/find-callers", post(find_callers))
        .route("/api/analyze-impact", post(analyze_impact))
        .route("/api/codebase-overview", get(codebase_overview))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
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

/// Validate database via ValidationService
async fn validate_database(
    State(state): State<ServicesAppState>,
    request: Result<Json<ValidationRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Handle JSON parsing errors with helpful messages
    let Json(request) = match request {
        Ok(json_request) => json_request,
        Err(json_error) => {
            let error_msg = match json_error {
                axum::extract::rejection::JsonRejection::JsonDataError(_) => {
                    "Invalid JSON format. Please check your request body syntax."
                }
                axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => {
                    "Missing or invalid Content-Type header. Please set 'Content-Type: application/json'."
                }
                axum::extract::rejection::JsonRejection::JsonSyntaxError(_) => {
                    "JSON syntax error. Please validate your JSON structure."
                }
                _ => "Request body parsing failed. Ensure valid JSON with Content-Type: application/json."
            };

            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "request_parsing_error".to_string(),
                    message: format!("{error_msg} For validation endpoint, you can send an empty JSON object: {{}}"),
                }),
            ));
        }
    };
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

/// Search code via SearchService
async fn search_code(
    State(state): State<ServicesAppState>,
    AxumQuery(request): AxumQuery<SearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_search_code", async move {
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
            context: "medium".to_string(),
            quiet: false,
        };

        search_service.search_content(options).await
    })
    .await;

    match result {
        Ok(search_result) => {
            // Check if simple format is requested
            let use_simple_format = request.format.as_deref() == Some("simple");

            let json_value = if use_simple_format {
                // Convert to simple format for developer consumption
                let simple_response = convert_to_simple_search_response(search_result);
                serde_json::to_value(simple_response).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            } else {
                // Use rich format by default (optimized for LLM consumption)
                serde_json::to_value(search_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            };
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to search code: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "search_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Search symbols via SearchService
async fn search_symbols(
    State(state): State<ServicesAppState>,
    AxumQuery(request): AxumQuery<SymbolSearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("api_search_symbols", async move {
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
            // Check if simple format is requested
            let use_simple_format = request.format.as_deref() == Some("simple");

            let json_value = if use_simple_format {
                // Convert to simple format for developer consumption
                let simple_response = convert_to_simple_symbol_response(symbol_result);
                serde_json::to_value(simple_response).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            } else {
                // Use rich format by default (optimized for LLM consumption)
                serde_json::to_value(symbol_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            };
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to search symbols: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "symbol_search_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Find callers via AnalysisService
async fn find_callers(
    State(state): State<ServicesAppState>,
    request: Result<Json<CallersRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Handle JSON parsing errors with helpful messages
    let Json(request) = match request {
        Ok(json_request) => json_request,
        Err(json_error) => {
            let error_msg = match json_error {
                axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => {
                    "Missing or invalid Content-Type header. Please set 'Content-Type: application/json'."
                }
                axum::extract::rejection::JsonRejection::JsonDataError(_) => {
                    "Invalid JSON format. Please check your request body syntax."
                }
                axum::extract::rejection::JsonRejection::JsonSyntaxError(_) => {
                    "JSON syntax error. Please validate your JSON structure."
                }
                _ => "Request body parsing failed. Ensure valid JSON with Content-Type: application/json."
            };

            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "request_parsing_error".to_string(),
                    message: format!("{error_msg} Expected: {{\"symbol\": \"YourSymbolName\"}} or {{\"target\": \"YourTargetName\"}}"),
                }),
            ));
        }
    };
    // Extract target from request with better error handling
    let target = match extract_target_from_callers_request(&request) {
        Ok(target) => target,
        Err(error_msg) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "validation_error".to_string(),
                    message: format!("{error_msg}. Suggestion: Use 'symbol' field name instead of 'target' for better readability."),
                }),
            ));
        }
    };

    let result = with_trace_id("api_find_callers", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());

        let options = CallersOptions {
            target,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.find_callers(options).await
    })
    .await;

    match result {
        Ok(callers_result) => {
            // Check if simple format is requested
            let use_simple_format = request.format.as_deref() == Some("simple");

            let json_value = if use_simple_format {
                // For simple format, convert analysis results to simple strings
                // Note: This is a placeholder - we'd need to see the actual structure of callers_result
                // For now, just pass through the rich format since we prioritize LLM consumption
                serde_json::to_value(callers_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            } else {
                // Use rich format by default (optimized for LLM consumption)
                serde_json::to_value(callers_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            };
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to find callers: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "find_callers_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Analyze impact via AnalysisService
async fn analyze_impact(
    State(state): State<ServicesAppState>,
    request: Result<Json<ImpactAnalysisRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Handle JSON parsing errors with helpful messages
    let Json(request) = match request {
        Ok(json_request) => json_request,
        Err(json_error) => {
            let error_msg = match json_error {
                axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => {
                    "Missing or invalid Content-Type header. Please set 'Content-Type: application/json'."
                }
                axum::extract::rejection::JsonRejection::JsonDataError(_) => {
                    "Invalid JSON format. Please check your request body syntax."
                }
                axum::extract::rejection::JsonRejection::JsonSyntaxError(_) => {
                    "JSON syntax error. Please validate your JSON structure."
                }
                _ => "Request body parsing failed. Ensure valid JSON with Content-Type: application/json."
            };

            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "request_parsing_error".to_string(),
                    message: format!("{error_msg} Expected: {{\"symbol\": \"YourSymbolName\"}} or {{\"target\": \"YourTargetName\"}}"),
                }),
            ));
        }
    };
    // Extract target from request with better error handling
    let target = match extract_target_from_impact_request(&request) {
        Ok(target) => target,
        Err(error_msg) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "validation_error".to_string(),
                    message: format!("{error_msg}. Suggestion: Use 'symbol' field name instead of 'target' for better readability."),
                }),
            ));
        }
    };

    let result = with_trace_id("api_analyze_impact", async move {
        // Create Database instance to implement DatabaseAccess
        let database = Database {
            storage: state.storage.clone(),
            primary_index: state.primary_index.clone(),
            trigram_index: state.trigram_index.clone(),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());

        let options = ImpactOptions {
            target,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.analyze_impact(options).await
    })
    .await;

    match result {
        Ok(impact_result) => {
            // Check if simple format is requested
            let use_simple_format = request.format.as_deref() == Some("simple");

            let json_value = if use_simple_format {
                // For simple format, prioritize LLM consumption but provide option
                serde_json::to_value(impact_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            } else {
                // Use rich format by default (optimized for LLM consumption)
                serde_json::to_value(impact_result).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "serialization_failed".to_string(),
                            message: e.to_string(),
                        }),
                    )
                })?
            };
            Ok(Json(json_value))
        }
        Err(e) => {
            tracing::warn!("Failed to analyze impact: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "impact_analysis_failed".to_string(),
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
