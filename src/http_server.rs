// HTTP REST API Server Implementation
// Provides health check, statistics, and validation endpoints

use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::{net::TcpListener, sync::Mutex};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    connection_pool::ConnectionPoolImpl,
    contracts::Storage,
    observability::with_trace_id,
    types::{ValidatedDocumentId, ValidatedTitle},
    validation::{index, path},
};

// Constants for default resource statistics
const DEFAULT_MEMORY_USAGE_BYTES: u64 = 32 * 1024 * 1024; // 32MB baseline memory usage
const DEFAULT_MEMORY_USAGE_MB: f64 = 32.0; // 32MB in megabytes
const DEFAULT_CPU_USAGE_PERCENT: f32 = 5.0; // 5% baseline CPU usage
const DEFAULT_CONNECTION_POOL_CAPACITY: f64 = 100.0; // Default max connections if not specified
const HEALTH_THRESHOLD_CPU: f32 = 90.0; // CPU usage threshold for health check
const HEALTH_THRESHOLD_MEMORY_MB: f64 = 1000.0; // Memory threshold in MB for health check
const HEALTH_THRESHOLD_CONNECTION_RATIO: f64 = 0.95; // Connection capacity threshold for health check

// Maximum request body size for validation endpoints
const MAX_REQUEST_SIZE: usize = 1024 * 1024; // 1MB

// Maximum items allowed in bulk validation requests to prevent abuse
const MAX_BULK_VALIDATION_ITEMS: usize = 100;

// Global server start time for uptime tracking
static SERVER_START_TIME: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    storage: Arc<Mutex<dyn Storage>>,
    connection_pool: Option<Arc<tokio::sync::Mutex<ConnectionPoolImpl>>>,
}


/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Connection statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatsResponse {
    pub active_connections: usize,
    pub total_connections: u64,
    pub rejected_connections: u64,
    pub rate_limited_requests: u64,
}

/// Performance metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceStatsResponse {
    pub avg_latency_ms: f64,
    pub total_requests: u64,
    pub requests_per_second: f64,
}

/// Resource usage response
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceStatsResponse {
    pub memory_usage_bytes: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f32,
    pub system_healthy: bool,
}

/// Aggregated stats response combining all statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatedStatsResponse {
    pub connections: ConnectionStatsResponse,
    pub performance: PerformanceStatsResponse,
    pub resources: ResourceStatsResponse,
}

/// Semantic search request
#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub limit: Option<u32>,
    pub threshold: Option<f32>,
}

/// Hybrid search request
#[derive(Debug, Deserialize)]
pub struct HybridSearchRequest {
    pub query: String,
    pub semantic_weight: Option<f32>,
    pub limit: Option<u32>,
}

/// Validation request for path validation
#[derive(Debug, Deserialize)]
pub struct ValidatePathRequest {
    pub path: String,
}

/// Validation request for document ID validation
#[derive(Debug, Deserialize)]
pub struct ValidateDocumentIdRequest {
    pub id: String,
}

/// Validation request for title validation
#[derive(Debug, Deserialize)]
pub struct ValidateTitleRequest {
    pub title: String,
}

/// Validation request for tag validation
#[derive(Debug, Deserialize)]
pub struct ValidateTagRequest {
    pub tag: String,
}

/// Bulk validation request for multiple fields
#[derive(Debug, Deserialize)]
pub struct BulkValidationRequest {
    pub paths: Option<Vec<String>>,
    pub document_ids: Option<Vec<String>>,
    pub titles: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub error: Option<String>,
}

/// Bulk validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkValidationResponse {
    pub paths: Option<Vec<ValidationResponse>>,
    pub document_ids: Option<Vec<ValidationResponse>>,
    pub titles: Option<Vec<ValidationResponse>>,
    pub tags: Option<Vec<ValidationResponse>>,
}


/// Create HTTP server with all routes configured
pub fn create_server(storage: Arc<Mutex<dyn Storage>>) -> Router {
    let state = AppState {
        storage,
        connection_pool: None,
    };

    Router::new()
        .route("/health", get(health_check))
        // New search endpoints for client compatibility
        .route("/search/semantic", post(semantic_search))
        .route("/search/hybrid", post(hybrid_search))
        // Monitoring endpoints
        .route("/stats", get(get_aggregated_stats))
        .route("/stats/connections", get(get_connection_stats))
        .route("/stats/performance", get(get_performance_stats))
        .route("/stats/resources", get(get_resource_stats))
        // Validation endpoints
        .route("/validate/path", post(validate_path))
        .route("/validate/document-id", post(validate_document_id))
        .route("/validate/title", post(validate_title))
        .route("/validate/tag", post(validate_tag))
        .route("/validate/bulk", post(validate_bulk))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
}

/// Create HTTP server with connection pool integration
pub fn create_server_with_pool(
    storage: Arc<Mutex<dyn Storage>>,
    connection_pool: Arc<tokio::sync::Mutex<ConnectionPoolImpl>>,
) -> Router {
    let state = AppState {
        storage,
        connection_pool: Some(connection_pool),
    };

    Router::new()
        .route("/health", get(health_check))
        // New search endpoints for client compatibility
        .route("/search/semantic", post(semantic_search))
        .route("/search/hybrid", post(hybrid_search))
        // Monitoring endpoints
        .route("/stats", get(get_aggregated_stats))
        .route("/stats/connections", get(get_connection_stats))
        .route("/stats/performance", get(get_performance_stats))
        .route("/stats/resources", get(get_resource_stats))
        // Validation endpoints
        .route("/validate/path", post(validate_path))
        .route("/validate/document-id", post(validate_document_id))
        .route("/validate/title", post(validate_title))
        .route("/validate/tag", post(validate_tag))
        .route("/validate/bulk", post(validate_bulk))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
}

/// Start the HTTP server on the specified port
pub async fn start_server(storage: Arc<Mutex<dyn Storage>>, port: u16) -> Result<()> {
    let app = create_server(storage);
    let listener = TcpListener::bind(&format!("0.0.0.0:{port}")).await?;

    info!("KotaDB HTTP server starting on port {}", port);
    info!(
        "Maximum request size: {}MB",
        MAX_REQUEST_SIZE / (1024 * 1024)
    );

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    let uptime_seconds = SERVER_START_TIME.elapsed().as_secs();

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
    })
}

/// Create a new document
async fn create_document(
    State(state): State<AppState>,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("create_document", async move {
        // Build document using DocumentBuilder
        let mut builder = DocumentBuilder::new()
            .path(&request.path)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?
            .title(request.title.unwrap_or_else(|| "Untitled".to_string()))
            .map_err(|e| anyhow::anyhow!("Invalid title: {}", e))?
            .content(request.content);

        // Add tags if provided
        if let Some(tags) = request.tags {
            for tag in tags {
                builder = builder
                    .tag(&tag)
                    .map_err(|e| anyhow::anyhow!("Invalid tag: {}", e))?;
            }
        }

        let doc = builder.build()?;

        // Store document
        state.storage.lock().await.insert(doc.clone()).await?;

        Ok(DocumentResponse::from(doc))
    })
    .await;

    match result {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => {
            warn!("Failed to create document: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "creation_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Get document by ID
async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("get_document", async move {
        state.storage.lock().await.get(&validated_id).await
    })
    .await;

    match result {
        Ok(Some(doc)) => Ok(Json(DocumentResponse::from(doc))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "document_not_found".to_string(),
                message: format!("Document with ID {doc_id} not found"),
            }),
        )),
        Err(e) => {
            warn!("Failed to get document {}: {}", doc_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "retrieval_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Update document by ID
async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("update_document", async move {
        // Get existing document
        let doc = match state.storage.lock().await.get(&validated_id).await? {
            Some(doc) => doc,
            None => return Err(anyhow::anyhow!("Document not found")),
        };

        // Build updated document using DocumentBuilder
        let mut builder = DocumentBuilder::new()
            .path(
                request
                    .path
                    .as_ref()
                    .unwrap_or(&doc.path.as_str().to_string()),
            )
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?
            .title(
                request
                    .title
                    .as_ref()
                    .unwrap_or(&doc.title.as_str().to_string()),
            )
            .map_err(|e| anyhow::anyhow!("Invalid title: {}", e))?
            .content(request.content.unwrap_or_else(|| doc.content.clone()));

        // Handle tags: use new tags if provided, otherwise keep existing ones
        if let Some(new_tags) = request.tags {
            // Use new tags only
            for tag in new_tags {
                builder = builder
                    .tag(&tag)
                    .map_err(|e| anyhow::anyhow!("Invalid tag: {}", e))?;
            }
        } else {
            // Keep existing tags
            for tag in &doc.tags {
                builder = builder
                    .tag(tag.as_str())
                    .map_err(|e| anyhow::anyhow!("Failed to add existing tag: {}", e))?;
            }
        }

        let mut updated_doc = builder.build()?;
        // Keep the same ID and adjust timestamps
        updated_doc.id = doc.id;
        updated_doc.created_at = doc.created_at;
        // Ensure updated_at is later than the original
        if updated_doc.updated_at <= doc.updated_at {
            updated_doc.updated_at = doc.updated_at + chrono::Duration::milliseconds(1);
        }

        // Update the document
        state
            .storage
            .lock()
            .await
            .update(updated_doc.clone())
            .await?;

        Ok(DocumentResponse::from(updated_doc))
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Failed to update document {}: {}", doc_id, e);
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: "update_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Delete document by ID
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("delete_document", async move {
        // Check if document exists first
        let mut storage = state.storage.lock().await;
        match storage.get(&validated_id).await? {
            Some(_) => {
                storage.delete(&validated_id).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Document not found")),
        }
    })
    .await;

    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            warn!("Failed to delete document {}: {}", doc_id, e);
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: "deletion_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Search documents
async fn search_documents(
    State(state): State<AppState>,
    AxumQuery(params): AxumQuery<SearchParams>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("search_documents", async move {
        // For now, implement a simple search that lists all documents
        // This is a placeholder implementation since we need to integrate with indices
        let limit = params.limit.unwrap_or(50) as usize;
        let _offset = params.offset.unwrap_or(0) as usize;

        // Get all documents and filter by search query if provided
        let all_docs = state.storage.lock().await.list_all().await?;
        let mut filtered_docs = Vec::new();

        // Prepare tag filter - support both 'tag' and 'tags' parameters
        let tag_filter = params.tag.as_ref().or(params.tags.as_ref());

        for doc in all_docs {
            let mut matches = true;

            // Apply text search filter
            if let Some(ref query) = params.q {
                if !query.is_empty() {
                    let content_str = String::from_utf8_lossy(&doc.content);
                    let title_str = doc.title.as_str();
                    let path_str = doc.path.as_str();

                    matches = content_str.to_lowercase().contains(&query.to_lowercase())
                        || title_str.to_lowercase().contains(&query.to_lowercase())
                        || path_str.to_lowercase().contains(&query.to_lowercase());
                }
            }

            // Apply tag filter if specified
            if matches {
                if let Some(tag) = tag_filter {
                    // Check if document has the specified tag
                    matches = doc.tags.iter().any(|t| t.as_str() == tag.as_str());
                }
            }

            // Apply path filter if specified
            if matches {
                if let Some(ref path_pattern) = params.path {
                    // Simple pattern matching - support wildcards
                    if path_pattern.contains('*') {
                        // Convert wildcard pattern to simple prefix/suffix matching
                        let pattern = path_pattern.replace("*", "");
                        if path_pattern.starts_with('*') && path_pattern.ends_with('*') {
                            matches = doc.path.as_str().contains(&pattern);
                        } else if path_pattern.starts_with('*') {
                            matches = doc.path.as_str().ends_with(&pattern);
                        } else if path_pattern.ends_with('*') {
                            matches = doc.path.as_str().starts_with(&pattern);
                        } else {
                            // Pattern has * in the middle - just check contains for now
                            matches = doc.path.as_str().contains(&pattern);
                        }
                    } else {
                        // Exact path match
                        matches = doc.path.as_str() == path_pattern;
                    }
                }
            }

            if matches {
                filtered_docs.push(doc);
            }
        }

        // Apply limit
        let total_count = filtered_docs.len();
        filtered_docs.truncate(limit);

        let documents: Vec<DocumentResponse> = filtered_docs
            .into_iter()
            .map(DocumentResponse::from)
            .collect();

        Ok(SearchResponse {
            documents,
            total_count,
            search_type: Some("text".to_string()),
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Search failed: {}", e);
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

/// Get connection statistics
async fn get_connection_stats(
    State(state): State<AppState>,
) -> Result<Json<ConnectionStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => Ok(Json(ConnectionStatsResponse {
                active_connections: stats.active_connections,
                total_connections: stats.total_connections,
                rejected_connections: stats.rejected_connections,
                rate_limited_requests: stats.rate_limited_requests,
            })),
            Err(e) => {
                warn!("Failed to get connection stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Connection statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return empty stats
        Ok(Json(ConnectionStatsResponse {
            active_connections: 0,
            total_connections: 0,
            rejected_connections: 0,
            rate_limited_requests: 0,
        }))
    }
}

/// Get performance statistics
async fn get_performance_stats(
    State(state): State<AppState>,
) -> Result<Json<PerformanceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => {
                // Calculate approximate requests per second
                // NOTE: This is a rough approximation based on average latency
                // For accurate RPS, implement proper request counting with time windows
                let requests_per_second = if stats.avg_latency_ms > 0.0 {
                    1000.0 / stats.avg_latency_ms
                } else {
                    0.0
                };

                Ok(Json(PerformanceStatsResponse {
                    avg_latency_ms: stats.avg_latency_ms,
                    total_requests: stats.total_connections, // Proxy for total requests
                    requests_per_second,
                }))
            }
            Err(e) => {
                warn!("Failed to get performance stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Performance statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return empty stats
        Ok(Json(PerformanceStatsResponse {
            avg_latency_ms: 0.0,
            total_requests: 0,
            requests_per_second: 0.0,
        }))
    }
}

/// Get resource usage statistics
async fn get_resource_stats(
    State(state): State<AppState>,
) -> Result<Json<ResourceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => {
                let memory_mb = stats.memory_usage_bytes as f64 / (1024.0 * 1024.0);

                // Determine system health based on various factors
                // Note: Using default capacity as actual capacity is not exposed in stats
                // TODO: Consider adding max_connections to ConnectionStats for accurate calculation
                let system_healthy = stats.cpu_usage_percent < HEALTH_THRESHOLD_CPU
                    && memory_mb < HEALTH_THRESHOLD_MEMORY_MB
                    && (stats.active_connections as f64 / DEFAULT_CONNECTION_POOL_CAPACITY)
                        < HEALTH_THRESHOLD_CONNECTION_RATIO;

                Ok(Json(ResourceStatsResponse {
                    memory_usage_bytes: stats.memory_usage_bytes,
                    memory_usage_mb: memory_mb,
                    cpu_usage_percent: stats.cpu_usage_percent,
                    system_healthy,
                }))
            }
            Err(e) => {
                warn!("Failed to get resource stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Resource statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return basic system stats
        Ok(Json(ResourceStatsResponse {
            memory_usage_bytes: DEFAULT_MEMORY_USAGE_BYTES,
            memory_usage_mb: DEFAULT_MEMORY_USAGE_MB,
            cpu_usage_percent: DEFAULT_CPU_USAGE_PERCENT,
            system_healthy: true,
        }))
    }
}

/// Get aggregated statistics (for Python client compatibility)
async fn get_aggregated_stats(
    State(state): State<AppState>,
) -> Result<Json<AggregatedStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Directly gather stats without calling other endpoints for better performance
    let (connections, performance, resources) = if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => {
                // Connection stats
                let connections = ConnectionStatsResponse {
                    active_connections: stats.active_connections,
                    total_connections: stats.total_connections,
                    rejected_connections: stats.rejected_connections,
                    rate_limited_requests: stats.rate_limited_requests,
                };

                // Performance stats
                let requests_per_second = if stats.avg_latency_ms > 0.0 {
                    1000.0 / stats.avg_latency_ms
                } else {
                    0.0
                };
                let performance = PerformanceStatsResponse {
                    avg_latency_ms: stats.avg_latency_ms,
                    total_requests: stats.total_connections,
                    requests_per_second,
                };

                // Resource stats
                let memory_mb = stats.memory_usage_bytes as f64 / (1024.0 * 1024.0);
                // Note: Using default capacity as actual capacity is not exposed in stats
                // TODO: Consider adding max_connections to ConnectionStats for accurate calculation
                let system_healthy = stats.cpu_usage_percent < HEALTH_THRESHOLD_CPU
                    && memory_mb < HEALTH_THRESHOLD_MEMORY_MB
                    && (stats.active_connections as f64 / DEFAULT_CONNECTION_POOL_CAPACITY)
                        < HEALTH_THRESHOLD_CONNECTION_RATIO;

                let resources = ResourceStatsResponse {
                    memory_usage_bytes: stats.memory_usage_bytes,
                    memory_usage_mb: memory_mb,
                    cpu_usage_percent: stats.cpu_usage_percent,
                    system_healthy,
                };

                (connections, performance, resources)
            }
            Err(_) => {
                // Return default stats if error occurs
                (
                    ConnectionStatsResponse {
                        active_connections: 0,
                        total_connections: 0,
                        rejected_connections: 0,
                        rate_limited_requests: 0,
                    },
                    PerformanceStatsResponse {
                        avg_latency_ms: 0.0,
                        total_requests: 0,
                        requests_per_second: 0.0,
                    },
                    ResourceStatsResponse {
                        memory_usage_bytes: DEFAULT_MEMORY_USAGE_BYTES,
                        memory_usage_mb: DEFAULT_MEMORY_USAGE_MB,
                        cpu_usage_percent: DEFAULT_CPU_USAGE_PERCENT,
                        system_healthy: true,
                    },
                )
            }
        }
    } else {
        // No connection pool configured - return default stats
        (
            ConnectionStatsResponse {
                active_connections: 0,
                total_connections: 0,
                rejected_connections: 0,
                rate_limited_requests: 0,
            },
            PerformanceStatsResponse {
                avg_latency_ms: 0.0,
                total_requests: 0,
                requests_per_second: 0.0,
            },
            ResourceStatsResponse {
                memory_usage_bytes: DEFAULT_MEMORY_USAGE_BYTES,
                memory_usage_mb: DEFAULT_MEMORY_USAGE_MB,
                cpu_usage_percent: DEFAULT_CPU_USAGE_PERCENT,
                system_healthy: true,
            },
        )
    };

    Ok(Json(AggregatedStatsResponse {
        connections,
        performance,
        resources,
    }))
}

/// Semantic search (for Python client compatibility)
async fn semantic_search(
    State(_state): State<AppState>,
    Json(request): Json<SemanticSearchRequest>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    info!("Semantic search requested for query: {}", request.query);

    // Document CRUD endpoints have been removed. 
    // Semantic search functionality should be implemented through the new codebase intelligence API.
    Err((
        StatusCode::GONE,
        Json(ErrorResponse {
            error: "endpoint_removed".to_string(),
            message: "Document CRUD endpoints have been removed. Please use the codebase intelligence API for search functionality.".to_string(),
        }),
    ))
}

/// Hybrid search (for Python client compatibility)
async fn hybrid_search(
    State(_state): State<AppState>,
    Json(request): Json<HybridSearchRequest>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Hybrid search requested for query: {} with semantic weight: {:?}",
        request.query, request.semantic_weight
    );

    // Document CRUD endpoints have been removed. 
    // Hybrid search functionality should be implemented through the new codebase intelligence API.
    Err((
        StatusCode::GONE,
        Json(ErrorResponse {
            error: "endpoint_removed".to_string(),
            message: "Document CRUD endpoints have been removed. Please use the codebase intelligence API for search functionality.".to_string(),
        }),
    ))
}

/// Validate a path
async fn validate_path(Json(request): Json<ValidatePathRequest>) -> Json<ValidationResponse> {
    let result = with_trace_id("validate_path", async move {
        match path::validate_file_path(&request.path) {
            Ok(_) => Ok(ValidationResponse {
                valid: true,
                error: None,
            }),
            Err(e) => Ok(ValidationResponse {
                valid: false,
                error: Some(e.to_string()),
            }),
        }
    })
    .await;

    Json(result.unwrap_or_else(|_| ValidationResponse {
        valid: false,
        error: Some("Internal validation error".to_string()),
    }))
}

/// Validate a document ID
async fn validate_document_id(
    Json(request): Json<ValidateDocumentIdRequest>,
) -> Json<ValidationResponse> {
    let result = with_trace_id("validate_document_id", async move {
        // First check UUID format
        match Uuid::parse_str(&request.id) {
            Ok(uuid) => {
                // Then check with ValidatedDocumentId validation
                match ValidatedDocumentId::from_uuid(uuid) {
                    Ok(_) => Ok(ValidationResponse {
                        valid: true,
                        error: None,
                    }),
                    Err(e) => Ok(ValidationResponse {
                        valid: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
            Err(e) => Ok(ValidationResponse {
                valid: false,
                error: Some(format!("Invalid UUID format: {}", e)),
            }),
        }
    })
    .await;

    Json(result.unwrap_or_else(|_| ValidationResponse {
        valid: false,
        error: Some("Internal validation error".to_string()),
    }))
}

/// Validate a title
async fn validate_title(Json(request): Json<ValidateTitleRequest>) -> Json<ValidationResponse> {
    let result = with_trace_id("validate_title", async move {
        match ValidatedTitle::new(&request.title) {
            Ok(_) => Ok(ValidationResponse {
                valid: true,
                error: None,
            }),
            Err(e) => Ok(ValidationResponse {
                valid: false,
                error: Some(e.to_string()),
            }),
        }
    })
    .await;

    Json(result.unwrap_or_else(|_| ValidationResponse {
        valid: false,
        error: Some("Internal validation error".to_string()),
    }))
}

/// Validate a tag
async fn validate_tag(Json(request): Json<ValidateTagRequest>) -> Json<ValidationResponse> {
    let result = with_trace_id("validate_tag", async move {
        match index::validate_tag(&request.tag) {
            Ok(_) => Ok(ValidationResponse {
                valid: true,
                error: None,
            }),
            Err(e) => Ok(ValidationResponse {
                valid: false,
                error: Some(e.to_string()),
            }),
        }
    })
    .await;

    Json(result.unwrap_or_else(|_| ValidationResponse {
        valid: false,
        error: Some("Internal validation error".to_string()),
    }))
}

/// Bulk validation endpoint
async fn validate_bulk(Json(request): Json<BulkValidationRequest>) -> Json<BulkValidationResponse> {
    let result = with_trace_id("validate_bulk", async move {
        let mut response = BulkValidationResponse {
            paths: None,
            document_ids: None,
            titles: None,
            tags: None,
        };

        // Check request limits to prevent abuse
        let total_items = request.paths.as_ref().map(|p| p.len()).unwrap_or(0)
            + request.document_ids.as_ref().map(|d| d.len()).unwrap_or(0)
            + request.titles.as_ref().map(|t| t.len()).unwrap_or(0)
            + request.tags.as_ref().map(|t| t.len()).unwrap_or(0);

        if total_items > MAX_BULK_VALIDATION_ITEMS {
            return Ok(BulkValidationResponse {
                paths: Some(vec![ValidationResponse {
                    valid: false,
                    error: Some(format!(
                        "Too many items in bulk request: {} (max: {})",
                        total_items, MAX_BULK_VALIDATION_ITEMS
                    )),
                }]),
                document_ids: None,
                titles: None,
                tags: None,
            });
        }

        // Validate paths if provided
        if let Some(paths) = request.paths {
            if paths.len() > MAX_BULK_VALIDATION_ITEMS {
                response.paths = Some(vec![ValidationResponse {
                    valid: false,
                    error: Some(format!(
                        "Too many paths: {} (max: {})",
                        paths.len(),
                        MAX_BULK_VALIDATION_ITEMS
                    )),
                }]);
            } else {
                let path_results: Vec<ValidationResponse> = paths
                    .iter()
                    .map(|path| match path::validate_file_path(path) {
                        Ok(_) => ValidationResponse {
                            valid: true,
                            error: None,
                        },
                        Err(e) => ValidationResponse {
                            valid: false,
                            error: Some(e.to_string()),
                        },
                    })
                    .collect();
                response.paths = Some(path_results);
            }
        }

        // Validate document IDs if provided
        if let Some(document_ids) = request.document_ids {
            if document_ids.len() > MAX_BULK_VALIDATION_ITEMS {
                response.document_ids = Some(vec![ValidationResponse {
                    valid: false,
                    error: Some(format!(
                        "Too many document IDs: {} (max: {})",
                        document_ids.len(),
                        MAX_BULK_VALIDATION_ITEMS
                    )),
                }]);
            } else {
                let id_results: Vec<ValidationResponse> = document_ids
                    .iter()
                    .map(|id| match Uuid::parse_str(id) {
                        Ok(uuid) => match ValidatedDocumentId::from_uuid(uuid) {
                            Ok(_) => ValidationResponse {
                                valid: true,
                                error: None,
                            },
                            Err(e) => ValidationResponse {
                                valid: false,
                                error: Some(e.to_string()),
                            },
                        },
                        Err(e) => ValidationResponse {
                            valid: false,
                            error: Some(format!("Invalid UUID format: {}", e)),
                        },
                    })
                    .collect();
                response.document_ids = Some(id_results);
            }
        }

        // Validate titles if provided
        if let Some(titles) = request.titles {
            if titles.len() > MAX_BULK_VALIDATION_ITEMS {
                response.titles = Some(vec![ValidationResponse {
                    valid: false,
                    error: Some(format!(
                        "Too many titles: {} (max: {})",
                        titles.len(),
                        MAX_BULK_VALIDATION_ITEMS
                    )),
                }]);
            } else {
                let title_results: Vec<ValidationResponse> = titles
                    .iter()
                    .map(|title| match ValidatedTitle::new(title) {
                        Ok(_) => ValidationResponse {
                            valid: true,
                            error: None,
                        },
                        Err(e) => ValidationResponse {
                            valid: false,
                            error: Some(e.to_string()),
                        },
                    })
                    .collect();
                response.titles = Some(title_results);
            }
        }

        // Validate tags if provided
        if let Some(tags) = request.tags {
            if tags.len() > MAX_BULK_VALIDATION_ITEMS {
                response.tags = Some(vec![ValidationResponse {
                    valid: false,
                    error: Some(format!(
                        "Too many tags: {} (max: {})",
                        tags.len(),
                        MAX_BULK_VALIDATION_ITEMS
                    )),
                }]);
            } else {
                let tag_results: Vec<ValidationResponse> = tags
                    .iter()
                    .map(|tag| match index::validate_tag(tag) {
                        Ok(_) => ValidationResponse {
                            valid: true,
                            error: None,
                        },
                        Err(e) => ValidationResponse {
                            valid: false,
                            error: Some(e.to_string()),
                        },
                    })
                    .collect();
                response.tags = Some(tag_results);
            }
        }

        Ok(response)
    })
    .await;

    Json(result.unwrap_or_else(|_| BulkValidationResponse {
        paths: Some(vec![ValidationResponse {
            valid: false,
            error: Some("Internal validation error".to_string()),
        }]),
        document_ids: None,
        titles: None,
        tags: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_file_storage, wrappers::create_wrapped_storage};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::util::ServiceExt;

    // Test directory that cleans up on drop
    struct TestDir {
        path: String,
    }

    impl TestDir {
        async fn new() -> Self {
            let path = format!("test_data/http_test_{}", uuid::Uuid::new_v4());
            tokio::fs::create_dir_all(&path)
                .await
                .expect("Failed to create test directory");
            Self { path }
        }

        fn path(&self) -> &str {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            // Clean up test directory
            let path = self.path.clone();
            std::thread::spawn(move || {
                let _ = std::fs::remove_dir_all(path);
            });
        }
    }

    async fn create_test_storage() -> (Arc<Mutex<dyn Storage>>, TestDir) {
        let test_dir = TestDir::new().await;

        let storage = create_file_storage(test_dir.path(), Some(1000))
            .await
            .expect("Failed to create storage");
        let wrapped = create_wrapped_storage(storage, 100).await;

        (Arc::new(Mutex::new(wrapped)), test_dir)
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }




    #[tokio::test]
    async fn test_monitoring_endpoints() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test connection stats endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/connections")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        // Since we're using create_server (not create_server_with_pool),
        // it should return empty stats
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let stats: ConnectionStatsResponse = serde_json::from_slice(&body)?;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_performance_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/performance")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_resource_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/resources")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let stats: ResourceStatsResponse = serde_json::from_slice(&body)?;
        assert!(stats.system_healthy);
        assert!(stats.memory_usage_mb > 0.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_aggregated_stats_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(Request::builder().uri("/stats").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let stats: AggregatedStatsResponse = serde_json::from_slice(&body)?;
        assert_eq!(stats.connections.active_connections, 0);
        assert_eq!(stats.performance.total_requests, 0);
        assert!(stats.resources.system_healthy);

        Ok(())
    }

    #[tokio::test]
    async fn test_semantic_search_endpoint_returns_gone() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let request_body = json!({
            "query": "test query",
            "limit": 10,
            "threshold": 0.8
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search/semantic")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        // Should return 410 Gone since document endpoints were removed
        assert_eq!(response.status(), StatusCode::GONE);

        Ok(())
    }

    #[tokio::test]
    async fn test_hybrid_search_endpoint_returns_gone() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        let request_body = json!({
            "query": "test query",
            "semantic_weight": 0.7,
            "limit": 10
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search/hybrid")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        // Should return 410 Gone since document endpoints were removed
        assert_eq!(response.status(), StatusCode::GONE);

        Ok(())
    }




    #[tokio::test]
    async fn test_validate_path_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test valid path
        let request_body = json!({
            "path": "test/document.md"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/path")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(validation_response.valid);
        assert!(validation_response.error.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_path_endpoint_invalid() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test invalid path (contains parent directory reference)
        let request_body = json!({
            "path": "../../../etc/passwd"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/path")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(!validation_response.valid);
        assert!(validation_response.error.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_document_id_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test valid document ID
        let valid_uuid = Uuid::new_v4();
        let request_body = json!({
            "id": valid_uuid.to_string()
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/document-id")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(validation_response.valid);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_document_id_endpoint_invalid() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test invalid document ID
        let request_body = json!({
            "id": "not-a-valid-uuid"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/document-id")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(!validation_response.valid);
        assert!(validation_response.error.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_title_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test valid title
        let request_body = json!({
            "title": "Valid Document Title"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/title")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(validation_response.valid);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_tag_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test valid tag
        let request_body = json!({
            "tag": "rust-programming"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/tag")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let validation_response: ValidationResponse = serde_json::from_slice(&body)?;
        assert!(validation_response.valid);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_bulk_endpoint() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test bulk validation with mixed valid/invalid data
        let request_body = json!({
            "paths": ["valid/path.md", "../invalid/path"],
            "document_ids": [Uuid::new_v4().to_string(), "invalid-uuid"],
            "titles": ["Valid Title", ""],
            "tags": ["valid-tag", "invalid@tag"]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/bulk")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let bulk_response: BulkValidationResponse = serde_json::from_slice(&body)?;

        // Check that we got responses for all fields
        assert!(bulk_response.paths.is_some());
        assert!(bulk_response.document_ids.is_some());
        assert!(bulk_response.titles.is_some());
        assert!(bulk_response.tags.is_some());

        // Check path validations
        let path_results = bulk_response.paths.unwrap();
        assert_eq!(path_results.len(), 2);
        assert!(path_results[0].valid); // valid/path.md
        assert!(!path_results[1].valid); // ../invalid/path

        // Check document ID validations
        let id_results = bulk_response.document_ids.unwrap();
        assert_eq!(id_results.len(), 2);
        assert!(id_results[0].valid); // valid UUID
        assert!(!id_results[1].valid); // invalid-uuid

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_bulk_request_limits() -> Result<()> {
        let (storage, _test_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Create a request with too many items
        let too_many_paths: Vec<String> = (0..150).map(|i| format!("path_{}.md", i)).collect();
        let request_body = json!({
            "paths": too_many_paths
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate/bulk")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let bulk_response: BulkValidationResponse = serde_json::from_slice(&body)?;

        // Should get an error response about too many items
        assert!(bulk_response.paths.is_some());
        let path_results = bulk_response.paths.unwrap();
        assert_eq!(path_results.len(), 1);
        assert!(!path_results[0].valid);
        assert!(path_results[0].error.as_ref().unwrap().contains("Too many"));

        Ok(())
    }
}
