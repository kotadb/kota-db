// Services-Only HTTP Server - Clean Implementation for Interface Parity
//
// This module provides a clean HTTP API that exposes KotaDB functionality exclusively
// through the services layer, ensuring complete interface parity with the CLI.
//
// No legacy code, no deprecated endpoints, no document CRUD - pure services architecture.

use anyhow::Result;
use axum::{
    extract::{Path, Query as AxumQuery, State},
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
use tracing::info;

use crate::{
    database::Database,
    services::{
        AnalysisService, AnalysisServiceDatabase, BenchmarkOptions, BenchmarkService,
        CallersOptions, HealthCheckOptions, ImpactOptions, IndexCodebaseOptions, IndexingService,
        OverviewOptions, SearchOptions, SearchService, StatsOptions, StatsService,
        SymbolSearchOptions, ValidationOptions, ValidationService,
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
}

/// Symbol search request
#[derive(Debug, Deserialize)]
pub struct SymbolSearchRequest {
    pub pattern: String,
    pub limit: Option<usize>,
    pub symbol_type: Option<String>,
}

/// Callers request
#[derive(Debug, Deserialize)]
pub struct CallersRequest {
    pub target: String,
    pub limit: Option<usize>,
}

/// Impact analysis request
#[derive(Debug, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub target: String,
    pub limit: Option<usize>,
}

/// Codebase overview request
#[derive(Debug, Deserialize)]
pub struct CodebaseOverviewRequest {
    pub format: Option<String>,
    pub top_symbols_limit: Option<usize>,
    pub entry_points_limit: Option<usize>,
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
    let listener = TcpListener::bind(&format!("0.0.0.0:{port}")).await?;

    info!("🚀 KotaDB Services HTTP Server starting on port {}", port);
    info!("🎯 Clean services-only architecture - no legacy endpoints");
    info!("Available endpoints:");
    info!("  - GET  /health - Server health check");
    info!("  - GET  /api/stats - Database statistics (StatsService)");
    info!("  - POST /api/benchmark - Performance benchmarks (BenchmarkService)");
    info!("  - POST /api/validate - Database validation (ValidationService)");
    info!("  - GET  /api/health-check - Detailed health check (ValidationService)");
    info!("  - POST /api/index-codebase - Index repository (IndexingService)");
    info!("  - GET  /api/search-code - Search code content (SearchService)");
    info!("  - GET  /api/search-symbols - Search symbols (SearchService)");
    info!("  - POST /api/find-callers - Find callers (AnalysisService)");
    info!("  - POST /api/analyze-impact - Impact analysis (AnalysisService)");
    info!("  - GET  /api/codebase-overview - Codebase overview (AnalysisService)");

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
            let json_value = serde_json::to_value(search_result).map_err(|e| {
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
            let json_value = serde_json::to_value(symbol_result).map_err(|e| {
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
    Json(request): Json<CallersRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
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
            target: request.target,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.find_callers(options).await
    })
    .await;

    match result {
        Ok(callers_result) => {
            let json_value = serde_json::to_value(callers_result).map_err(|e| {
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
    Json(request): Json<ImpactAnalysisRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
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
            target: request.target,
            limit: request.limit,
            quiet: false,
        };

        analysis_service.analyze_impact(options).await
    })
    .await;

    match result {
        Ok(impact_result) => {
            let json_value = serde_json::to_value(impact_result).map_err(|e| {
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

        let mut analysis_service = AnalysisService::new(&database, state.db_path.clone());

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
