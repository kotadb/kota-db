//! HTTP API endpoints for codebase intelligence features
//!
//! This module provides RESTful API endpoints for code analysis operations including:
//! - Symbol search and navigation
//! - Find callers (who calls this function)
//! - Impact analysis (what would be affected by changes)
//! - Code search with trigram indexing
//!
//! All endpoints use the BinaryRelationshipEngine for performance (<10ms latency target)

use anyhow::Result;
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

use crate::{
    binary_relationship_engine_async::AsyncBinaryRelationshipEngine,
    contracts::Index,
    git::{IngestionConfig, RepositoryIngester},
    observability::with_trace_id,
    relationship_query::RelationshipQueryType,
    trigram_index::TrigramIndex,
};

/// Response for symbol search operations
#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolSearchResponse {
    pub symbols: Vec<SymbolInfo>,
    pub total_count: usize,
    pub query_time_ms: u64,
}

/// Information about a code symbol
#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: String,
    pub file_path: String,
    pub line_number: u32,
    pub column: u32,
    pub definition: Option<String>,
    pub language: String,
}

/// Response for find callers operations
#[derive(Debug, Serialize, Deserialize)]
pub struct FindCallersResponse {
    pub target: String,
    pub callers: Vec<CallerInfo>,
    pub total_count: usize,
    pub query_time_ms: u64,
}

/// Information about a caller
#[derive(Debug, Serialize, Deserialize)]
pub struct CallerInfo {
    pub caller_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub column: u32,
    pub call_type: String, // "direct" or "indirect"
    pub context: Option<String>,
}

/// Response for impact analysis operations
#[derive(Debug, Serialize, Deserialize)]
pub struct ImpactAnalysisResponse {
    pub target: String,
    pub direct_impacts: Vec<ImpactInfo>,
    pub indirect_impacts: Vec<ImpactInfo>,
    pub total_affected: usize,
    pub query_time_ms: u64,
    pub risk_assessment: RiskLevel,
}

/// Information about an impacted component
#[derive(Debug, Serialize, Deserialize)]
pub struct ImpactInfo {
    pub component_name: String,
    pub file_path: String,
    pub impact_type: String,
    pub distance: u32, // How many hops from the target
    pub description: Option<String>,
}

/// Risk level for impact analysis
#[derive(Debug, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Response for code search operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeSearchResponse {
    pub query: String,
    pub results: Vec<CodeSearchResult>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub search_type: String, // "exact", "fuzzy", "semantic"
}

/// Individual code search result
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub file_path: String,
    pub line_number: u32,
    pub content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub score: f32,
}

/// Query parameters for symbol search
#[derive(Debug, Deserialize)]
pub struct SymbolSearchParams {
    pub q: String,                   // Search query (supports wildcards)
    pub symbol_type: Option<String>, // Filter by symbol type
    pub language: Option<String>,    // Filter by language
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Query parameters for find callers
#[derive(Debug, Deserialize)]
pub struct FindCallersParams {
    pub include_indirect: Option<bool>, // Include indirect callers
    pub max_depth: Option<u32>,         // Maximum depth for indirect callers
    pub limit: Option<u32>,
}

/// Query parameters for impact analysis
#[derive(Debug, Deserialize)]
pub struct ImpactAnalysisParams {
    pub max_depth: Option<u32>,         // Maximum depth for impact analysis
    pub include_tests: Option<bool>,    // Include test files in analysis
    pub risk_threshold: Option<String>, // Minimum risk level to include
}

/// Query parameters for code search
#[derive(Debug, Deserialize)]
pub struct CodeSearchParams {
    pub q: String,                    // Search query
    pub fuzzy: Option<bool>,          // Enable fuzzy matching
    pub regex: Option<bool>,          // Treat query as regex
    pub case_sensitive: Option<bool>, // Case-sensitive search
    pub file_pattern: Option<String>, // Filter by file pattern
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub context_lines: Option<u32>, // Number of context lines
}

/// Request body for repository indexing
#[derive(Debug, Deserialize)]
pub struct IndexRepositoryRequest {
    pub repository_url: String, // GitHub repository URL or local path
    pub branch: Option<String>, // Branch to index (default: main/master)
    pub include_patterns: Option<Vec<String>>, // File patterns to include
    pub exclude_patterns: Option<Vec<String>>, // File patterns to exclude
    pub extract_symbols: Option<bool>, // Extract code symbols (default: true)
    pub shallow_clone: Option<bool>, // Use shallow clone for faster indexing
}

/// Response for repository indexing operation
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexRepositoryResponse {
    pub repository_id: String,
    pub repository_url: String,
    pub branch: String,
    pub files_indexed: usize,
    pub symbols_extracted: usize,
    pub relationships_found: usize,
    pub index_time_ms: u64,
    pub status: String,
}

/// Shared state for codebase intelligence endpoints
#[derive(Clone)]
pub struct CodebaseIntelligenceState {
    pub relationship_engine: Arc<AsyncBinaryRelationshipEngine>,
    pub trigram_index: Arc<RwLock<Option<TrigramIndex>>>,
    pub db_path: std::path::PathBuf,
    pub storage: Option<Arc<tokio::sync::Mutex<dyn crate::contracts::Storage>>>,
}

/// Search for symbols in the codebase
#[instrument(skip(state))]
pub async fn search_symbols(
    State(state): State<CodebaseIntelligenceState>,
    AxumQuery(params): AxumQuery<SymbolSearchParams>,
) -> Result<Json<SymbolSearchResponse>, (StatusCode, Json<crate::http_server::ErrorResponse>)> {
    let start = Instant::now();

    let result = with_trace_id("search_symbols", async move {
        info!(
            "Searching for symbols: query='{}', type={:?}",
            params.q, params.symbol_type
        );

        // Use FindCallers as a workaround since SymbolSearch doesn't exist
        // We'll search for symbols that match the pattern
        // TODO: Implement proper symbol search in RelationshipQueryEngine
        let query_type = RelationshipQueryType::FindCallers {
            target: params.q.clone(),
        };

        let query_result = state.relationship_engine.execute_query(query_type).await?;

        // Convert results to API response format
        let symbols: Vec<SymbolInfo> = query_result
            .direct_relationships
            .into_iter()
            .skip(params.offset.unwrap_or(0) as usize)
            .take(params.limit.unwrap_or(100) as usize)
            .map(|m| SymbolInfo {
                name: m.symbol_name,
                symbol_type: format!("{:?}", m.symbol_type),
                file_path: m.location.file_path,
                line_number: m.location.line_number as u32,
                column: m.location.column_number as u32,
                definition: Some(m.context),
                language: "rust".to_string(), // Default to rust since language isn't in RelationshipMatch
            })
            .collect();

        let total_count = symbols.len();
        let query_time_ms = start.elapsed().as_millis() as u64;

        if query_time_ms > 10 {
            warn!("Symbol search exceeded 10ms target: {}ms", query_time_ms);
        }

        Ok(SymbolSearchResponse {
            symbols,
            total_count,
            query_time_ms,
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Symbol search failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::http_server::ErrorResponse {
                    error: "symbol_search_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Find all callers of a specific function or symbol
#[instrument(skip(state))]
pub async fn find_callers(
    State(state): State<CodebaseIntelligenceState>,
    Path(target): Path<String>,
    AxumQuery(params): AxumQuery<FindCallersParams>,
) -> Result<Json<FindCallersResponse>, (StatusCode, Json<crate::http_server::ErrorResponse>)> {
    let start = Instant::now();

    let result = with_trace_id("find_callers", async move {
        info!("Finding callers for: {}", target);

        let query_type = RelationshipQueryType::FindCallers {
            target: target.clone(),
        };

        let query_result = state.relationship_engine.execute_query(query_type).await?;

        // Convert results to API response format
        let callers: Vec<CallerInfo> = query_result
            .direct_relationships
            .into_iter()
            .take(params.limit.unwrap_or(100) as usize)
            .map(|m| CallerInfo {
                caller_name: m.symbol_name,
                file_path: m.location.file_path,
                line_number: m.location.line_number as u32,
                column: m.location.column_number as u32,
                call_type: "direct".to_string(),
                context: Some(m.context),
            })
            .collect();

        let total_count = callers.len();
        let query_time_ms = start.elapsed().as_millis() as u64;

        if query_time_ms > 10 {
            warn!("Find callers exceeded 10ms target: {}ms", query_time_ms);
        }

        Ok(FindCallersResponse {
            target,
            callers,
            total_count,
            query_time_ms,
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Find callers failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::http_server::ErrorResponse {
                    error: "find_callers_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Analyze the impact of changes to a specific component
#[instrument(skip(state))]
pub async fn analyze_impact(
    State(state): State<CodebaseIntelligenceState>,
    Path(target): Path<String>,
    AxumQuery(params): AxumQuery<ImpactAnalysisParams>,
) -> Result<Json<ImpactAnalysisResponse>, (StatusCode, Json<crate::http_server::ErrorResponse>)> {
    let start = Instant::now();

    let result = with_trace_id("analyze_impact", async move {
        info!("Analyzing impact for: {}", target);

        let query_type = RelationshipQueryType::ImpactAnalysis {
            target: target.clone(),
        };

        let query_result = state.relationship_engine.execute_query(query_type).await?;

        // Separate direct and indirect impacts
        let direct_impacts: Vec<ImpactInfo> = query_result
            .direct_relationships
            .into_iter()
            .map(|m| ImpactInfo {
                component_name: m.symbol_name,
                file_path: m.location.file_path,
                impact_type: format!("{:?}", m.relation_type),
                distance: 1,
                description: Some(m.context),
            })
            .collect();

        let indirect_impacts: Vec<ImpactInfo> = query_result
            .indirect_relationships
            .into_iter()
            .map(|path| ImpactInfo {
                component_name: path.symbol_names.last().cloned().unwrap_or_default(),
                file_path: String::new(), // Call paths don't have file info
                impact_type: "indirect".to_string(),
                distance: path.distance as u32,
                description: Some(path.description),
            })
            .collect();

        let total_affected = direct_impacts.len() + indirect_impacts.len();

        // Assess risk level based on impact count
        let risk_assessment = match total_affected {
            0..=5 => RiskLevel::Low,
            6..=20 => RiskLevel::Medium,
            21..=50 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        let query_time_ms = start.elapsed().as_millis() as u64;

        if query_time_ms > 10 {
            warn!("Impact analysis exceeded 10ms target: {}ms", query_time_ms);
        }

        Ok(ImpactAnalysisResponse {
            target,
            direct_impacts,
            indirect_impacts,
            total_affected,
            query_time_ms,
            risk_assessment,
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Impact analysis failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::http_server::ErrorResponse {
                    error: "impact_analysis_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Search for code content using trigram indexing
#[instrument(skip(state))]
pub async fn search_code(
    State(state): State<CodebaseIntelligenceState>,
    AxumQuery(params): AxumQuery<CodeSearchParams>,
) -> Result<Json<CodeSearchResponse>, (StatusCode, Json<crate::http_server::ErrorResponse>)> {
    let start = Instant::now();

    let result = with_trace_id("search_code", async move {
        info!("Searching code for: {}", params.q);

        // Check if trigram index is available
        let index_guard = state.trigram_index.read().await;

        if let Some(index) = index_guard.as_ref() {
            // Build a Query object for the trigram index
            use crate::builders::QueryBuilder;
            let query = QueryBuilder::new()
                .with_text(&params.q)?
                .with_limit(params.limit.unwrap_or(100) as usize)?
                .build()?;

            // Use trigram index for fast text search
            let search_results = index.search(&query).await?;

            // Convert to API response format
            let results: Vec<CodeSearchResult> = search_results
                .into_iter()
                .skip(params.offset.unwrap_or(0) as usize)
                .take(params.limit.unwrap_or(100) as usize)
                .map(|doc_id| {
                    // Note: This is a simplified version. In production, you'd fetch
                    // the actual document content and extract context lines
                    CodeSearchResult {
                        file_path: doc_id.to_string(),
                        line_number: 0, // Would need to extract from document
                        content: params.q.clone(), // Placeholder - would show actual match
                        context_before: vec![],
                        context_after: vec![],
                        score: 1.0, // Default score since we don't have ranking yet
                    }
                })
                .collect();

            let total_count = results.len();
            let query_time_ms = start.elapsed().as_millis() as u64;

            if query_time_ms > 10 {
                warn!("Code search exceeded 10ms target: {}ms", query_time_ms);
            }

            Ok(CodeSearchResponse {
                query: params.q,
                results,
                total_count,
                query_time_ms,
                search_type: if params.fuzzy.unwrap_or(false) {
                    "fuzzy"
                } else {
                    "exact"
                }
                .to_string(),
            })
        } else {
            Err(anyhow::anyhow!(
                "Trigram index not available. Please ensure the codebase has been indexed."
            ))
        }
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Code search failed: {}", e);
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(crate::http_server::ErrorResponse {
                    error: "code_search_unavailable".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Add deprecation headers to a response
pub fn add_deprecation_headers(headers: &mut HeaderMap) {
    headers.insert("Deprecation", "true".parse().unwrap());
    headers.insert(
        "Sunset",
        "2025-03-01T00:00:00Z".parse().unwrap(), // 3 months from now
    );
    headers.insert(
        "Link",
        "</api/symbols/search>; rel=\"successor-version\""
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Warning",
        "299 - \"This endpoint is deprecated. Please use /api/symbols/search instead.\""
            .parse()
            .unwrap(),
    );
}

/// Index a GitHub repository or local codebase
#[instrument(skip(state))]
pub async fn index_repository(
    State(state): State<CodebaseIntelligenceState>,
    axum::extract::Json(request): axum::extract::Json<IndexRepositoryRequest>,
) -> Result<Json<IndexRepositoryResponse>, (StatusCode, Json<crate::http_server::ErrorResponse>)> {
    let start = Instant::now();

    let result = with_trace_id("index_repository", async move {
        info!("Indexing repository: {}", request.repository_url);

        // Validate that we have storage available
        let storage = state
            .storage
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Storage not configured for indexing operations"))?;

        // Parse repository URL to determine if it's GitHub or local
        let is_github = request.repository_url.starts_with("https://github.com/")
            || request.repository_url.starts_with("git@github.com:");

        let repo_path = if is_github {
            // Clone the repository to a temporary directory
            let temp_dir = tempfile::tempdir()?;
            let repo_path = temp_dir.path().to_path_buf();

            // Build git clone command
            let mut clone_cmd = std::process::Command::new("git");
            clone_cmd.arg("clone");

            if request.shallow_clone.unwrap_or(true) {
                clone_cmd.arg("--depth").arg("1");
            }

            if let Some(ref branch) = request.branch {
                clone_cmd.arg("--branch").arg(branch);
            }

            clone_cmd.arg(&request.repository_url).arg(&repo_path);

            let output = clone_cmd.output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to clone repository: {}", stderr));
            }

            repo_path
        } else {
            // Local repository path
            std::path::PathBuf::from(&request.repository_url)
        };

        // Configure ingestion
        let mut ingestion_config = IngestionConfig::default();

        // Map include_patterns to include_extensions (if they are file extensions)
        if let Some(include) = request.include_patterns {
            // Filter patterns that look like extensions (e.g., "*.rs" -> "rs")
            let extensions: Vec<String> = include
                .iter()
                .filter_map(|p| {
                    if p.starts_with("*.") {
                        Some(p.trim_start_matches("*.").to_string())
                    } else {
                        None
                    }
                })
                .collect();
            if !extensions.is_empty() {
                ingestion_config.options.include_extensions = extensions;
            }
        }

        if let Some(exclude) = request.exclude_patterns {
            ingestion_config.options.exclude_patterns = exclude;
        }

        ingestion_config.options.extract_symbols = request.extract_symbols.unwrap_or(true);

        // Create the ingester and ingest the repository
        let ingester = RepositoryIngester::new(ingestion_config);

        // Lock storage for ingestion
        let mut storage_guard = storage.lock().await;
        let ingest_result = ingester.ingest(&repo_path, &mut *storage_guard).await?;
        drop(storage_guard);

        // Extract repository name for ID
        let repo_name = request
            .repository_url
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .trim_end_matches(".git");

        let repository_id = format!("repo_{}", repo_name);
        let branch = request.branch.unwrap_or_else(|| "main".to_string());

        // Rebuild trigram index after ingestion
        info!("Rebuilding trigram index after repository ingestion");
        let mut trigram_guard = state.trigram_index.write().await;
        if trigram_guard.is_some() {
            // Re-index with new documents
            let storage_guard = storage.lock().await;
            let all_docs = storage_guard.list_all().await?;
            drop(storage_guard);

            if let Some(ref mut index) = *trigram_guard {
                for doc in all_docs {
                    // Use insert_with_content which is designed for trigram indexing
                    index
                        .insert_with_content(doc.id, doc.path.clone(), &doc.content)
                        .await?;
                }
            }
        }
        drop(trigram_guard);

        let query_time_ms = start.elapsed().as_millis() as u64;

        Ok(IndexRepositoryResponse {
            repository_id,
            repository_url: request.repository_url,
            branch,
            files_indexed: ingest_result.files_ingested,
            symbols_extracted: ingest_result.symbols_extracted,
            relationships_found: ingest_result.relationships_extracted,
            index_time_ms: query_time_ms,
            status: "completed".to_string(),
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Repository indexing failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::http_server::ErrorResponse {
                    error: "indexing_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}
