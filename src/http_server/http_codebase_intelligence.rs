// HTTP API endpoints for codebase intelligence features
// Provides REST API access to code search, symbol analysis, and relationship queries

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    binary_relationship_engine::BinaryRelationshipEngine,
    contracts::{Index, Storage},
    git::{IngestionConfig, IngestionOptions, RepositoryIngester, RepositoryOrganizationConfig},
    observability::with_trace_id,
    relationship_query::RelationshipQueryType,
};

/// State for codebase intelligence endpoints
#[derive(Clone)]
pub struct CodeIntelligenceState {
    pub storage: Arc<Mutex<dyn Storage>>,
    pub trigram_index: Arc<dyn Index>,
    pub db_path: std::path::PathBuf,
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to index a repository
#[derive(Debug, Deserialize)]
pub struct IndexRepositoryRequest {
    /// Path to the repository to index
    pub path: String,
    /// Optional prefix for document paths
    pub prefix: Option<String>,
    /// Include file contents (default: true)
    pub include_files: Option<bool>,
    /// Include commit history (default: false)
    pub include_commits: Option<bool>,
    /// Maximum file size in MB (default: 10)
    pub max_file_size_mb: Option<usize>,
    /// Extract symbols using tree-sitter (default: true)
    #[cfg(feature = "tree-sitter-parsing")]
    pub extract_symbols: Option<bool>,
}

/// Response from repository indexing
#[derive(Debug, Serialize)]
pub struct IndexRepositoryResponse {
    /// Unique ID for this indexing operation
    pub id: Uuid,
    /// Status of the operation
    pub status: String,
    /// Number of files processed
    pub files_processed: usize,
    /// Number of symbols extracted
    pub symbols_extracted: usize,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u128,
}

/// Request for code search
#[derive(Debug, Deserialize)]
pub struct CodeSearchRequest {
    /// Search query
    pub query: String,
    /// Maximum number of results (default: 50)
    pub limit: Option<usize>,
    /// Include context lines (default: 2)
    pub context_lines: Option<usize>,
}

/// Response from code search
#[derive(Debug, Serialize)]
pub struct CodeSearchResponse {
    /// Search results
    pub results: Vec<CodeSearchResult>,
    /// Total number of matches
    pub total_matches: usize,
    /// Search time in milliseconds
    pub search_time_ms: u128,
}

/// Individual code search result
#[derive(Debug, Serialize)]
pub struct CodeSearchResult {
    /// File path
    pub file: String,
    /// Line number
    pub line: usize,
    /// Matching content
    pub content: String,
    /// Context before the match
    pub context_before: Vec<String>,
    /// Context after the match
    pub context_after: Vec<String>,
}

/// Request for symbol search
#[derive(Debug, Deserialize)]
pub struct SymbolSearchRequest {
    /// Symbol pattern (supports wildcards)
    pub pattern: String,
    /// Maximum number of results (default: 50)
    pub limit: Option<usize>,
    /// Filter by symbol type (function, class, etc.)
    pub symbol_type: Option<String>,
}

/// Response from symbol search
#[derive(Debug, Serialize)]
pub struct SymbolSearchResponse {
    /// Symbol matches
    pub symbols: Vec<SymbolInfo>,
    /// Total number of matches
    pub total_matches: usize,
    /// Search time in milliseconds
    pub search_time_ms: u128,
}

/// Information about a symbol
#[derive(Debug, Serialize)]
pub struct SymbolInfo {
    /// Symbol name
    pub name: String,
    /// Symbol type (function, class, etc.)
    pub symbol_type: String,
    /// File containing the symbol
    pub file: String,
    /// Line number
    pub line: usize,
    /// Symbol signature or definition
    pub signature: Option<String>,
}

/// Response for callers analysis
#[derive(Debug, Serialize)]
pub struct CallersResponse {
    /// Target symbol
    pub symbol: String,
    /// Total number of callers
    pub total_callers: usize,
    /// Caller information
    pub callers: Vec<CallerInfo>,
    /// Query time in milliseconds
    pub query_time_ms: u128,
}

/// Information about a caller
#[derive(Debug, Serialize)]
pub struct CallerInfo {
    /// File containing the caller
    pub file: String,
    /// Line number
    pub line: usize,
    /// Code context
    pub context: String,
    /// Type of usage (call, reference, import, etc.)
    pub usage_type: String,
}

/// Response for impact analysis
#[derive(Debug, Serialize)]
pub struct ImpactAnalysisResponse {
    /// Target symbol
    pub symbol: String,
    /// Direct impacts
    pub direct_impacts: Vec<ImpactInfo>,
    /// Indirect impacts
    pub indirect_impacts: Vec<ImpactInfo>,
    /// Total affected files
    pub affected_files: usize,
    /// Query time in milliseconds
    pub query_time_ms: u128,
}

/// Information about an impact
#[derive(Debug, Serialize)]
pub struct ImpactInfo {
    /// File affected
    pub file: String,
    /// Symbol affected
    pub symbol: String,
    /// Impact type
    pub impact_type: String,
    /// Severity (high, medium, low)
    pub severity: String,
}

/// Response for codebase statistics
#[derive(Debug, Serialize)]
pub struct CodebaseStatsResponse {
    /// Total number of documents
    pub total_documents: usize,
    /// Total number of symbols
    pub total_symbols: usize,
    /// Symbol breakdown by type
    pub symbols_by_type: std::collections::HashMap<String, usize>,
    /// Total relationships
    pub total_relationships: usize,
    /// Database size in bytes
    pub database_size_bytes: u64,
    /// Index statistics
    pub index_stats: IndexStats,
}

/// Index statistics
#[derive(Debug, Serialize)]
pub struct IndexStats {
    /// Trigram index size
    pub trigram_index_size: usize,
    /// Primary index size
    pub primary_index_size: usize,
    /// Binary symbols loaded
    pub binary_symbols_loaded: usize,
}

// ============================================================================
// API Handlers
// ============================================================================

/// Index a repository
pub async fn index_repository(
    State(state): State<CodeIntelligenceState>,
    Json(request): Json<IndexRepositoryRequest>,
) -> Result<Json<IndexRepositoryResponse>, StatusCode> {
    let start = std::time::Instant::now();
    with_trace_id("codebase_intelligence", async {
        info!("Indexing repository: {}", request.path);
        debug!("Index options: {:?}", request);
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // Validate repository path
    let repo_path = std::path::Path::new(&request.path);
    if !repo_path.exists() {
        error!("Repository path does not exist: {}", request.path);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Configure ingestion options
    let options = IngestionOptions {
        include_file_contents: request.include_files.unwrap_or(true),
        include_commit_history: request.include_commits.unwrap_or(false),
        max_file_size: request.max_file_size_mb.unwrap_or(10) * 1024 * 1024,
        #[cfg(feature = "tree-sitter-parsing")]
        extract_symbols: request.extract_symbols.unwrap_or(true),
        ..Default::default()
    };

    let config = IngestionConfig {
        path_prefix: request.prefix.unwrap_or_else(String::new),
        options,
        create_index: true,
        organization_config: Some(RepositoryOrganizationConfig::default()),
    };

    // Create ingester and run ingestion
    let ingester = RepositoryIngester::new(config.clone());
    let mut storage = state.storage.lock().await;

    let result = ingester
        .ingest_with_progress(repo_path, &mut *storage, None)
        .await
        .map_err(|e| {
            error!("Failed to index repository: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(
        "Repository indexed successfully: {} files, {} symbols",
        result.files_ingested, result.symbols_extracted
    );

    Ok(Json(IndexRepositoryResponse {
        id: Uuid::new_v4(),
        status: "completed".to_string(),
        files_processed: result.files_ingested,
        symbols_extracted: result.symbols_extracted,
        elapsed_ms: start.elapsed().as_millis(),
    }))
}

/// Search code using trigram index
pub async fn search_code(
    State(state): State<CodeIntelligenceState>,
    Json(request): Json<CodeSearchRequest>,
) -> Result<Json<CodeSearchResponse>, StatusCode> {
    let start = std::time::Instant::now();
    with_trace_id("codebase_intelligence", async {
        info!("Code search query: {}", request.query);
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    let limit = request.limit.unwrap_or(50);
    let context_lines = request.context_lines.unwrap_or(2);

    // Build query
    let query = crate::QueryBuilder::new()
        .with_text(&request.query)
        .and_then(|q| q.with_limit(limit))
        .and_then(|q| q.build())
        .map_err(|e| {
            error!("Failed to build query: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Perform trigram search
    let results = state.trigram_index.search(&query).await.map_err(|e| {
        error!("Code search failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert results to response format
    let mut search_results = Vec::new();
    let storage = state.storage.lock().await;

    for doc_id in results.iter().take(limit) {
        if let Ok(Some(doc)) = storage.get(doc_id).await {
            // Find matching lines in document
            let content = String::from_utf8_lossy(&doc.content);
            for (line_num, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&request.query.to_lowercase()) {
                    // Get context lines
                    let lines: Vec<_> = content.lines().collect();
                    let start_ctx = line_num.saturating_sub(context_lines);
                    let end_ctx = (line_num + context_lines + 1).min(lines.len());

                    let context_before = lines[start_ctx..line_num]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    let context_after = lines[(line_num + 1)..end_ctx]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    search_results.push(CodeSearchResult {
                        file: doc.path.to_string(),
                        line: line_num + 1,
                        content: line.to_string(),
                        context_before,
                        context_after,
                    });
                }
            }
        }
    }

    Ok(Json(CodeSearchResponse {
        total_matches: search_results.len(),
        results: search_results,
        search_time_ms: start.elapsed().as_millis(),
    }))
}

/// Search for symbols
pub async fn search_symbols(
    State(state): State<CodeIntelligenceState>,
    Json(request): Json<SymbolSearchRequest>,
) -> Result<Json<SymbolSearchResponse>, StatusCode> {
    let start = std::time::Instant::now();
    with_trace_id("codebase_intelligence", async {
        info!("Symbol search query: {}", request.pattern);
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // For now, return empty results - this would need proper symbol index implementation
    // TODO: Implement actual symbol search using symbol index
    Ok(Json(SymbolSearchResponse {
        symbols: vec![],
        total_matches: 0,
        search_time_ms: start.elapsed().as_millis(),
    }))
}

/// Find callers of a symbol
pub async fn find_callers(
    State(state): State<CodeIntelligenceState>,
    Path(symbol): Path<String>,
) -> Result<Json<CallersResponse>, StatusCode> {
    let start = std::time::Instant::now();
    with_trace_id("codebase_intelligence", async {
        info!("Finding callers of: {}", symbol);
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // Clone the path for use in blocking task
    let db_path = state.db_path.clone();
    let target_symbol = symbol.clone();

    // Run the relationship engine operations in a blocking task
    let result = tokio::task::spawn_blocking(move || {
        // Create a runtime for the blocking task
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            // Create relationship engine
            let engine = BinaryRelationshipEngine::new(
                &db_path,
                crate::relationship_query::RelationshipQueryConfig::default(),
            )
            .await?;

            // Execute find callers query
            let query = RelationshipQueryType::FindCallers {
                target: target_symbol,
            };

            engine.execute_query(query).await
        })
    })
    .await
    .map_err(|e| {
        error!("Task join error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|e| {
        error!("Failed to find callers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert to response format
    let callers: Vec<CallerInfo> = result
        .direct_relationships
        .iter()
        .map(|r| CallerInfo {
            file: r.file_path.clone(),
            line: r.location.line_number,
            context: String::new(), // TODO: Get actual context from file
            usage_type: format!("{:?}", r.relation_type),
        })
        .collect();

    Ok(Json(CallersResponse {
        symbol,
        total_callers: callers.len(),
        callers,
        query_time_ms: start.elapsed().as_millis(),
    }))
}

/// Analyze impact of changes to a symbol
pub async fn analyze_impact(
    State(state): State<CodeIntelligenceState>,
    Path(symbol): Path<String>,
) -> Result<Json<ImpactAnalysisResponse>, StatusCode> {
    let start = std::time::Instant::now();
    with_trace_id("codebase_intelligence", async {
        info!("Analyzing impact of: {}", symbol);
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // Clone the path for use in blocking task
    let db_path = state.db_path.clone();
    let target_symbol = symbol.clone();

    // Run the relationship engine operations in a blocking task
    let result = tokio::task::spawn_blocking(move || {
        // Create a runtime for the blocking task
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            // Create relationship engine
            let engine = BinaryRelationshipEngine::new(
                &db_path,
                crate::relationship_query::RelationshipQueryConfig::default(),
            )
            .await?;

            // Execute impact analysis query
            let query = RelationshipQueryType::ImpactAnalysis {
                target: target_symbol,
            };

            engine.execute_query(query).await
        })
    })
    .await
    .map_err(|e| {
        error!("Task join error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|e| {
        error!("Failed to analyze impact: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert to response format
    let direct_impacts: Vec<ImpactInfo> = result
        .direct_relationships
        .iter()
        .map(|r| ImpactInfo {
            file: r.file_path.clone(),
            symbol: r.symbol_name.clone(),
            impact_type: format!("{:?}", r.relation_type),
            severity: "medium".to_string(), // TODO: Calculate actual severity
        })
        .collect();

    let indirect_impacts: Vec<ImpactInfo> = result
        .indirect_relationships
        .iter()
        .enumerate()
        .map(|(i, path)| ImpactInfo {
            file: String::new(), // TODO: Get file path for symbols in path
            symbol: path.symbol_names.get(i).cloned().unwrap_or_default(),
            impact_type: "indirect".to_string(),
            severity: "low".to_string(), // TODO: Calculate actual severity
        })
        .collect();

    let affected_files = result.direct_relationships.len() + result.indirect_relationships.len();

    Ok(Json(ImpactAnalysisResponse {
        symbol,
        direct_impacts,
        indirect_impacts,
        affected_files,
        query_time_ms: start.elapsed().as_millis(),
    }))
}

/// Get codebase statistics
pub async fn get_codebase_stats(
    State(state): State<CodeIntelligenceState>,
) -> Result<Json<CodebaseStatsResponse>, StatusCode> {
    with_trace_id("codebase_intelligence", async {
        info!("Getting codebase statistics");
        Ok::<(), anyhow::Error>(())
    })
    .await
    .ok();

    // Get storage stats
    let storage = state.storage.lock().await;
    let docs = storage
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc_count = docs.len();

    // Get symbol stats if available
    let (total_symbols, symbols_by_type, total_relationships, binary_symbols) =
        if state.db_path.join("symbols.kota").exists() {
            // Create relationship engine to get symbol stats
            match BinaryRelationshipEngine::new(
                &state.db_path,
                crate::relationship_query::RelationshipQueryConfig::default(),
            )
            .await
            {
                Ok(engine) => {
                    let stats = engine.get_stats();
                    (
                        stats.binary_symbols_loaded,
                        std::collections::HashMap::new(), // TODO: Get actual symbols by type
                        stats.graph_nodes_loaded,
                        stats.binary_symbols_loaded,
                    )
                }
                Err(_) => (0, std::collections::HashMap::new(), 0, 0),
            }
        } else {
            (0, std::collections::HashMap::new(), 0, 0)
        };

    // Calculate database size
    let db_size = calculate_database_size(&state.db_path).await;

    Ok(Json(CodebaseStatsResponse {
        total_documents: doc_count,
        total_symbols,
        symbols_by_type,
        total_relationships,
        database_size_bytes: db_size,
        index_stats: IndexStats {
            trigram_index_size: 0, // TODO: Get actual size from trigram index
            primary_index_size: 0, // TODO: Get actual size from primary index
            binary_symbols_loaded: binary_symbols,
        },
    }))
}

/// Wrapper for find_callers to help with type inference
#[axum::debug_handler]
async fn find_callers_handler(
    State(state): State<CodeIntelligenceState>,
    Path(symbol): Path<String>,
) -> Result<Json<CallersResponse>, StatusCode> {
    find_callers(State(state), Path(symbol)).await
}

/// Wrapper for analyze_impact to help with type inference
#[axum::debug_handler]
async fn analyze_impact_handler(
    State(state): State<CodeIntelligenceState>,
    Path(symbol): Path<String>,
) -> Result<Json<ImpactAnalysisResponse>, StatusCode> {
    analyze_impact(State(state), Path(symbol)).await
}

/// Register codebase intelligence routes
pub fn register_routes(state: CodeIntelligenceState) -> Router<CodeIntelligenceState> {
    Router::new()
        // Repository management
        .route("/api/repositories", post(index_repository))
        // Code search
        .route("/api/search/code", post(search_code))
        .route("/api/search/symbols", post(search_symbols))
        // Symbol analysis
        .route("/api/symbols/:symbol/callers", get(find_callers_handler))
        .route("/api/symbols/:symbol/impact", get(analyze_impact_handler))
        // Statistics
        .route("/api/analysis/stats", get(get_codebase_stats))
        .with_state(state)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate total database size
async fn calculate_database_size(db_path: &std::path::Path) -> u64 {
    let mut total_size = 0u64;

    // Check main database files
    let files = [
        "documents.kota",
        "documents.wal",
        "index.kota",
        "trigram_index.kota",
        "symbols.kota",
        "dependency_graph.bin",
    ];

    for file in &files {
        let path = db_path.join(file);
        if let Ok(metadata) = tokio::fs::metadata(&path).await {
            total_size += metadata.len();
        }
    }

    total_size
}
