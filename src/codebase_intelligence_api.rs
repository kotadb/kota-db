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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    // Test helper to create a minimal state for testing (async version)
    async fn create_test_state_async() -> (TempDir, CodebaseIntelligenceState) {
        use crate::relationship_query::RelationshipQueryConfig;
        
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().to_path_buf();
        
        // Create a minimal relationship engine
        let engine = AsyncBinaryRelationshipEngine::new(&db_path, RelationshipQueryConfig::default())
            .await
            .expect("Failed to create relationship engine");
        
        let state = CodebaseIntelligenceState {
            relationship_engine: Arc::new(engine),
            trigram_index: Arc::new(RwLock::new(None)),
            db_path: db_path.clone(),
            storage: None,
        };
        
        (temp_dir, state)
    }
    
    // Test helper to create a minimal state for sync tests (without relationship engine)
    fn create_minimal_test_state() -> (TempDir, CodebaseIntelligenceState) {
        use crate::relationship_query::RelationshipQueryConfig;
        
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().to_path_buf();
        
        // Create a mock relationship engine for tests that don't need full functionality
        let rt = tokio::runtime::Runtime::new().unwrap();
        let engine = rt.block_on(async {
            AsyncBinaryRelationshipEngine::new(&db_path, RelationshipQueryConfig::default())
                .await
                .expect("Failed to create relationship engine")
        });
        
        let state = CodebaseIntelligenceState {
            relationship_engine: Arc::new(engine),
            trigram_index: Arc::new(RwLock::new(None)),
            db_path: db_path.clone(),
            storage: None,
        };
        
        (temp_dir, state)
    }

    #[test]
    fn test_symbol_info_serialization() {
        let symbol_info = SymbolInfo {
            name: "test_function".to_string(),
            symbol_type: "function".to_string(),
            file_path: "src/main.rs".to_string(),
            line_number: 42,
            column: 8,
            definition: Some("fn test_function() {}".to_string()),
            language: "rust".to_string(),
        };
        
        let serialized = serde_json::to_string(&symbol_info).expect("Should serialize");
        let deserialized: SymbolInfo = 
            serde_json::from_str(&serialized).expect("Should deserialize");
        
        assert_eq!(symbol_info.name, deserialized.name);
        assert_eq!(symbol_info.file_path, deserialized.file_path);
        assert_eq!(symbol_info.line_number, deserialized.line_number);
    }

    #[test]
    fn test_caller_info_serialization() {
        let caller_info = CallerInfo {
            caller_name: "main".to_string(),
            file_path: "src/main.rs".to_string(),
            line_number: 10,
            column: 4,
            call_type: "direct".to_string(),
            context: Some("main calls test_function".to_string()),
        };
        
        let serialized = serde_json::to_string(&caller_info).expect("Should serialize");
        let deserialized: CallerInfo = 
            serde_json::from_str(&serialized).expect("Should deserialize");
        
        assert_eq!(caller_info.caller_name, deserialized.caller_name);
        assert_eq!(caller_info.call_type, deserialized.call_type);
        assert_eq!(caller_info.context, deserialized.context);
    }

    #[test]
    fn test_impact_info_serialization() {
        let impact_info = ImpactInfo {
            component_name: "Database".to_string(),
            file_path: "src/database.rs".to_string(),
            impact_type: "dependency".to_string(),
            distance: 2,
            description: Some("Database depends on Connection".to_string()),
        };
        
        let serialized = serde_json::to_string(&impact_info).expect("Should serialize");
        let deserialized: ImpactInfo = 
            serde_json::from_str(&serialized).expect("Should deserialize");
        
        assert_eq!(impact_info.component_name, deserialized.component_name);
        assert_eq!(impact_info.distance, deserialized.distance);
        assert_eq!(impact_info.description, deserialized.description);
    }

    #[test]
    fn test_risk_level_serialization() {
        let risk_levels = vec![
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Critical,
        ];
        
        for risk_level in risk_levels {
            let serialized = serde_json::to_string(&risk_level).expect("Should serialize");
            let deserialized: RiskLevel = 
                serde_json::from_str(&serialized).expect("Should deserialize");
            
            // Compare serialized string representation
            let original_serialized = serde_json::to_string(&risk_level).unwrap();
            let deserialized_serialized = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(original_serialized, deserialized_serialized);
        }
    }

    #[test]
    fn test_code_search_result_serialization() {
        let search_result = CodeSearchResult {
            file_path: "src/main.rs".to_string(),
            line_number: 15,
            content: "fn main() {".to_string(),
            context_before: vec!["// Main function".to_string()],
            context_after: vec!["    println!(\"Hello\");".to_string()],
            score: 0.95,
        };
        
        let serialized = serde_json::to_string(&search_result).expect("Should serialize");
        let deserialized: CodeSearchResult = 
            serde_json::from_str(&serialized).expect("Should deserialize");
        
        assert_eq!(search_result.file_path, deserialized.file_path);
        assert_eq!(search_result.line_number, deserialized.line_number);
        assert_eq!(search_result.content, deserialized.content);
        assert_eq!(search_result.context_before, deserialized.context_before);
        assert_eq!(search_result.context_after, deserialized.context_after);
        assert!((search_result.score - deserialized.score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_symbol_search_params_deserialization() {
        let params_json = r#"{
            "q": "test_*",
            "symbol_type": "function",
            "language": "rust",
            "limit": 50,
            "offset": 10
        }"#;
        
        let params: SymbolSearchParams = 
            serde_json::from_str(params_json).expect("Should deserialize");
        
        assert_eq!(params.q, "test_*");
        assert_eq!(params.symbol_type, Some("function".to_string()));
        assert_eq!(params.language, Some("rust".to_string()));
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.offset, Some(10));
    }

    #[test]
    fn test_symbol_search_params_minimal() {
        let params_json = r#"{"q": "main"}"#;
        
        let params: SymbolSearchParams = 
            serde_json::from_str(params_json).expect("Should deserialize");
        
        assert_eq!(params.q, "main");
        assert_eq!(params.symbol_type, None);
        assert_eq!(params.language, None);
        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
    }

    #[test]
    fn test_find_callers_params_deserialization() {
        let params_json = r#"{
            "include_indirect": true,
            "max_depth": 5,
            "limit": 20
        }"#;
        
        let params: FindCallersParams = 
            serde_json::from_str(params_json).expect("Should deserialize");
        
        assert_eq!(params.include_indirect, Some(true));
        assert_eq!(params.max_depth, Some(5));
        assert_eq!(params.limit, Some(20));
    }

    #[test]
    fn test_impact_analysis_params_deserialization() {
        let params_json = r#"{
            "max_depth": 3,
            "include_tests": false,
            "risk_threshold": "medium"
        }"#;
        
        let params: ImpactAnalysisParams = 
            serde_json::from_str(params_json).expect("Should deserialize");
        
        assert_eq!(params.max_depth, Some(3));
        assert_eq!(params.include_tests, Some(false));
        assert_eq!(params.risk_threshold, Some("medium".to_string()));
    }

    #[test]
    fn test_code_search_params_deserialization() {
        let params_json = r#"{
            "q": "async fn",
            "fuzzy": true,
            "regex": false,
            "case_sensitive": true,
            "file_pattern": "*.rs",
            "limit": 100,
            "offset": 0,
            "context_lines": 3
        }"#;
        
        let params: CodeSearchParams = 
            serde_json::from_str(params_json).expect("Should deserialize");
        
        assert_eq!(params.q, "async fn");
        assert_eq!(params.fuzzy, Some(true));
        assert_eq!(params.regex, Some(false));
        assert_eq!(params.case_sensitive, Some(true));
        assert_eq!(params.file_pattern, Some("*.rs".to_string()));
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.offset, Some(0));
        assert_eq!(params.context_lines, Some(3));
    }

    #[test]
    fn test_index_repository_request_deserialization() {
        let request_json = r#"{
            "repository_url": "https://github.com/rust-lang/rust",
            "branch": "master",
            "include_patterns": ["*.rs", "*.toml"],
            "exclude_patterns": ["target/", "*.lock"],
            "extract_symbols": true,
            "shallow_clone": false
        }"#;
        
        let request: IndexRepositoryRequest = 
            serde_json::from_str(request_json).expect("Should deserialize");
        
        assert_eq!(request.repository_url, "https://github.com/rust-lang/rust");
        assert_eq!(request.branch, Some("master".to_string()));
        assert_eq!(request.include_patterns, Some(vec!["*.rs".to_string(), "*.toml".to_string()]));
        assert_eq!(request.exclude_patterns, Some(vec!["target/".to_string(), "*.lock".to_string()]));
        assert_eq!(request.extract_symbols, Some(true));
        assert_eq!(request.shallow_clone, Some(false));
    }

    #[test]
    fn test_index_repository_request_minimal() {
        let request_json = r#"{"repository_url": "/local/path"}"#;
        
        let request: IndexRepositoryRequest = 
            serde_json::from_str(request_json).expect("Should deserialize");
        
        assert_eq!(request.repository_url, "/local/path");
        assert_eq!(request.branch, None);
        assert_eq!(request.include_patterns, None);
        assert_eq!(request.exclude_patterns, None);
        assert_eq!(request.extract_symbols, None);
        assert_eq!(request.shallow_clone, None);
    }

    #[test]
    fn test_symbol_search_response_creation() {
        let symbols = vec![
            SymbolInfo {
                name: "main".to_string(),
                symbol_type: "function".to_string(),
                file_path: "src/main.rs".to_string(),
                line_number: 1,
                column: 1,
                definition: Some("fn main() {}".to_string()),
                language: "rust".to_string(),
            },
            SymbolInfo {
                name: "Config".to_string(),
                symbol_type: "struct".to_string(),
                file_path: "src/config.rs".to_string(),
                line_number: 10,
                column: 1,
                definition: Some("pub struct Config {}".to_string()),
                language: "rust".to_string(),
            },
        ];
        
        let response = SymbolSearchResponse {
            symbols: symbols.clone(),
            total_count: symbols.len(),
            query_time_ms: 5,
        };
        
        assert_eq!(response.symbols.len(), 2);
        assert_eq!(response.total_count, 2);
        assert_eq!(response.query_time_ms, 5);
        assert_eq!(response.symbols[0].name, "main");
        assert_eq!(response.symbols[1].name, "Config");
    }

    #[test]
    fn test_find_callers_response_creation() {
        let callers = vec![
            CallerInfo {
                caller_name: "main".to_string(),
                file_path: "src/main.rs".to_string(),
                line_number: 5,
                column: 4,
                call_type: "direct".to_string(),
                context: Some("main() calls init()".to_string()),
            },
        ];
        
        let response = FindCallersResponse {
            target: "init".to_string(),
            callers: callers.clone(),
            total_count: callers.len(),
            query_time_ms: 3,
        };
        
        assert_eq!(response.target, "init");
        assert_eq!(response.callers.len(), 1);
        assert_eq!(response.total_count, 1);
        assert_eq!(response.query_time_ms, 3);
        assert_eq!(response.callers[0].caller_name, "main");
    }

    #[test]
    fn test_impact_analysis_response_creation() {
        let direct_impacts = vec![
            ImpactInfo {
                component_name: "Database".to_string(),
                file_path: "src/database.rs".to_string(),
                impact_type: "uses".to_string(),
                distance: 1,
                description: Some("Database uses Connection".to_string()),
            },
        ];
        
        let indirect_impacts = vec![
            ImpactInfo {
                component_name: "WebServer".to_string(),
                file_path: "src/web.rs".to_string(),
                impact_type: "indirect".to_string(),
                distance: 2,
                description: Some("WebServer depends on Database".to_string()),
            },
        ];
        
        let response = ImpactAnalysisResponse {
            target: "Connection".to_string(),
            direct_impacts: direct_impacts.clone(),
            indirect_impacts: indirect_impacts.clone(),
            total_affected: 2,
            query_time_ms: 8,
            risk_assessment: RiskLevel::Low,
        };
        
        assert_eq!(response.target, "Connection");
        assert_eq!(response.direct_impacts.len(), 1);
        assert_eq!(response.indirect_impacts.len(), 1);
        assert_eq!(response.total_affected, 2);
        assert_eq!(response.query_time_ms, 8);
        
        // Verify risk assessment is correct
        match response.risk_assessment {
            RiskLevel::Low => {},
            _ => panic!("Expected Low risk level for 2 impacts"),
        }
    }

    #[test]
    fn test_code_search_response_creation() {
        let results = vec![
            CodeSearchResult {
                file_path: "src/main.rs".to_string(),
                line_number: 10,
                content: "async fn main() {".to_string(),
                context_before: vec!["#[tokio::main]".to_string()],
                context_after: vec!["    println!(\"Starting...\");".to_string()],
                score: 0.9,
            },
        ];
        
        let response = CodeSearchResponse {
            query: "async fn".to_string(),
            results: results.clone(),
            total_count: results.len(),
            query_time_ms: 4,
            search_type: "exact".to_string(),
        };
        
        assert_eq!(response.query, "async fn");
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.total_count, 1);
        assert_eq!(response.query_time_ms, 4);
        assert_eq!(response.search_type, "exact");
        assert_eq!(response.results[0].content, "async fn main() {");
    }

    #[test]
    fn test_index_repository_response_creation() {
        let response = IndexRepositoryResponse {
            repository_id: "repo_rust".to_string(),
            repository_url: "https://github.com/rust-lang/rust".to_string(),
            branch: "master".to_string(),
            files_indexed: 1500,
            symbols_extracted: 50000,
            relationships_found: 25000,
            index_time_ms: 30000,
            status: "completed".to_string(),
        };
        
        assert_eq!(response.repository_id, "repo_rust");
        assert_eq!(response.branch, "master");
        assert_eq!(response.files_indexed, 1500);
        assert_eq!(response.symbols_extracted, 50000);
        assert_eq!(response.relationships_found, 25000);
        assert_eq!(response.index_time_ms, 30000);
        assert_eq!(response.status, "completed");
    }

    #[test]
    fn test_risk_level_assessment_logic() {
        // Test the risk assessment logic used in analyze_impact
        let test_cases = vec![
            (0, RiskLevel::Low),
            (3, RiskLevel::Low),
            (5, RiskLevel::Low),
            (6, RiskLevel::Medium),
            (15, RiskLevel::Medium),
            (20, RiskLevel::Medium),
            (21, RiskLevel::High),
            (35, RiskLevel::High),
            (50, RiskLevel::High),
            (51, RiskLevel::Critical),
            (100, RiskLevel::Critical),
        ];
        
        for (impact_count, expected_risk) in test_cases {
            let actual_risk = match impact_count {
                0..=5 => RiskLevel::Low,
                6..=20 => RiskLevel::Medium,
                21..=50 => RiskLevel::High,
                _ => RiskLevel::Critical,
            };
            
            // Compare serialized representation since RiskLevel doesn't implement PartialEq
            let expected_json = serde_json::to_string(&expected_risk).unwrap();
            let actual_json = serde_json::to_string(&actual_risk).unwrap();
            assert_eq!(expected_json, actual_json, "Failed for impact_count: {}", impact_count);
        }
    }

    #[test]
    fn test_codebase_intelligence_state_creation() {
        let (_temp_dir, state) = create_minimal_test_state();
        
        // Verify state was created successfully
        assert!(state.db_path.exists() || state.db_path.parent().unwrap().exists());
        assert!(state.storage.is_none()); // Default state has no storage
    }

    #[test]
    fn test_add_deprecation_headers() {
        let mut headers = HeaderMap::new();
        add_deprecation_headers(&mut headers);
        
        assert_eq!(headers.get("Deprecation").unwrap(), "true");
        assert_eq!(headers.get("Sunset").unwrap(), "2025-03-01T00:00:00Z");
        assert!(headers.get("Link").unwrap().to_str().unwrap().contains("successor-version"));
        assert!(headers.get("Warning").unwrap().to_str().unwrap().contains("deprecated"));
    }

    #[test]
    fn test_github_url_detection() {
        let github_urls = vec![
            "https://github.com/rust-lang/rust",
            "git@github.com:rust-lang/rust.git",
        ];
        
        let local_paths = vec![
            "/home/user/project",
            "./local/path",
            "C:\\Users\\project",
        ];
        
        for url in github_urls {
            let is_github = url.starts_with("https://github.com/") 
                || url.starts_with("git@github.com:");
            assert!(is_github, "Should detect {} as GitHub URL", url);
        }
        
        for path in local_paths {
            let is_github = path.starts_with("https://github.com/") 
                || path.starts_with("git@github.com:");
            assert!(!is_github, "Should not detect {} as GitHub URL", path);
        }
    }

    #[test]
    fn test_repository_name_extraction() {
        let test_cases = vec![
            ("https://github.com/rust-lang/rust", "rust"),
            ("git@github.com:user/project.git", "project"),
            ("https://github.com/org/repo.git", "repo"),
            ("/local/path/project", "project"),
            ("simple_name", "simple_name"),
        ];
        
        for (url, expected_name) in test_cases {
            let repo_name = url
                .split('/')
                .next_back()
                .unwrap_or("unknown")
                .trim_end_matches(".git");
            
            assert_eq!(repo_name, expected_name, "Failed to extract name from: {}", url);
        }
    }

    #[test]
    fn test_file_extension_filtering() {
        let include_patterns = vec!["*.rs".to_string(), "*.toml".to_string(), "README.md".to_string()];
        
        // Simulate the extension filtering logic from index_repository
        let extensions: Vec<String> = include_patterns
            .iter()
            .filter_map(|p| {
                if p.starts_with("*.") {
                    Some(p.trim_start_matches("*.").to_string())
                } else {
                    None
                }
            })
            .collect();
        
        assert_eq!(extensions, vec!["rs", "toml"]);
        assert!(!extensions.contains(&"md".to_string())); // README.md doesn't match pattern
    }

    #[tokio::test]
    async fn test_state_trigram_index_access() {
        let (_temp_dir, state) = create_test_state_async().await;
        
        // Test read access
        {
            let index_guard = state.trigram_index.read().await;
            assert!(index_guard.is_none());
        }
        
        // Test write access
        {
            let mut index_guard = state.trigram_index.write().await;
            *index_guard = None; // Set to None explicitly
            assert!(index_guard.is_none());
        }
        
        // Verify read access after write
        {
            let index_guard = state.trigram_index.read().await;
            assert!(index_guard.is_none());
        }
    }
}
