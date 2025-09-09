// KotaDB - A Custom Database for Distributed Cognition
// Root library module

pub mod api_keys;
pub mod auth_middleware;
pub mod binary_trigram_index;
pub mod builders;
pub mod codebase_intelligence_api;
pub mod connection_pool;
pub mod contracts;
pub mod coordinated_deletion;
pub mod database;
pub mod documentation_verification;
pub mod embedding_transformer;
pub mod embeddings;
pub mod file_storage;
pub mod graph_storage;
pub mod http_server;
pub mod http_types;
pub mod hybrid_storage;
pub mod llm_search;
pub mod memory;
pub mod metrics;
pub mod native_graph_storage;
pub mod observability;
pub mod primary_index;
pub mod pure;
pub mod query_sanitization;
pub mod search_validation;
pub mod semantic_search;
pub mod services;
pub mod services_http_server;
pub mod trigram_index;
pub mod types;
pub mod validation;
pub mod vector_index;
pub mod wrappers;

// Git integration module
#[cfg(feature = "git-integration")]
pub mod git;

// Code parsing module
pub mod parsing;

// Symbol storage and extraction pipeline
#[cfg(feature = "tree-sitter-parsing")]
pub mod symbol_storage;

// Binary format for efficient symbol storage
#[cfg(feature = "tree-sitter-parsing")]
pub mod binary_symbols;

// Binary-to-relationship bridge for dependency graph construction
#[cfg(feature = "tree-sitter-parsing")]
pub mod binary_relationship_bridge;

// Symbol-aware index for code-specific searches
#[cfg(feature = "tree-sitter-parsing")]
pub mod symbol_index;

// Dependency extraction and call graph building
#[cfg(feature = "tree-sitter-parsing")]
pub mod dependency_extractor;

// Relationship query interface for dependency graph navigation
#[cfg(feature = "tree-sitter-parsing")]
pub mod relationship_query;

// Binary relationship engine that provides fast symbol lookup and relationships
#[cfg(feature = "tree-sitter-parsing")]
pub mod binary_relationship_engine;

// Async wrapper for binary relationship engine (thread-safe for HTTP handlers)
#[cfg(feature = "tree-sitter-parsing")]
pub mod binary_relationship_engine_async;

// Path normalization utilities for consistent path handling
pub mod path_utils;

// Factory functions for production-ready components
#[cfg(feature = "tree-sitter-parsing")]
pub mod factory;

// Re-export key types
pub use observability::{
    init_logging, init_logging_with_level, log_operation, record_metric, with_trace_id, MetricType,
    Operation,
};

pub use contracts::{Document, Index, PageId, Query, Storage, StorageMetrics, Transaction};

// Re-export validated types
pub use types::{
    NonZeroSize, TimestampPair, ValidatedDocumentId, ValidatedLimit, ValidatedPageId,
    ValidatedPath, ValidatedSearchQuery, ValidatedTag, ValidatedTimestamp, ValidatedTitle,
};

// Re-export builders
pub use builders::{
    DocumentBuilder, IndexConfigBuilder, MetricsBuilder, QueryBuilder, StorageConfigBuilder,
};

// Re-export wrappers
pub use wrappers::{
    create_wrapped_storage, CachedStorage, MeteredIndex, RetryableStorage, TracedStorage,
    ValidatedStorage,
};

// Re-export optimization wrappers
pub use wrappers::optimization::{
    create_optimized_index, create_optimized_index_with_defaults, OptimizationConfig,
    OptimizationReport, OptimizedIndex,
};

// Re-export storage implementations
pub use file_storage::{create_file_storage, FileStorage};

// Re-export API key management
pub use api_keys::{ApiKeyConfig, ApiKeyService};

// Utility functions for deployment and debugging
use anyhow::Result;

/// Test database connection for deployment troubleshooting
pub async fn test_database_connection(config: &ApiKeyConfig) -> Result<()> {
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use tracing::info;

    info!("Testing PostgreSQL connection...");
    info!(
        "Database URL prefix: {}...",
        &config.database_url.chars().take(20).collect::<String>()
    );

    let pool = PgPoolOptions::new()
        .max_connections(1) // Just for testing
        .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
        .connect(&config.database_url)
        .await?;

    // Test basic query
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;

    if row.0 != 1 {
        return Err(anyhow::anyhow!("Database test query failed"));
    }

    pool.close().await;
    info!("Database connection test successful");
    Ok(())
}
// Re-export coordinated deletion service
pub use coordinated_deletion::CoordinatedDeletionService;

// Re-export HTTP server and connection pool
pub use connection_pool::{
    create_connection_pool, create_rate_limiter, ConnectionPoolImpl, SystemResourceMonitor,
    TokenBucketRateLimiter,
};
pub use http_server::{
    create_server, create_server_with_intelligence, create_server_with_pool, start_saas_server,
    start_server, start_server_with_intelligence,
};

// Re-export services HTTP server (clean architecture)
pub use services_http_server::{
    create_services_saas_server, create_services_server, start_services_saas_server,
    start_services_server,
};

// Re-export shared HTTP types
pub use http_types::ErrorResponse;

// Re-export index implementations
pub use binary_trigram_index::{create_binary_trigram_index, BinaryTrigramIndex};
pub use primary_index::{create_primary_index, create_primary_index_for_tests, PrimaryIndex};
#[cfg(feature = "tree-sitter-parsing")]
pub use symbol_index::{create_symbol_index, create_symbol_index_for_tests, SymbolIndex};
pub use trigram_index::{create_trigram_index, create_trigram_index_for_tests, TrigramIndex};
pub use vector_index::{DistanceMetric, SemanticQuery, VectorIndex};

// Re-export embedding providers
pub use embeddings::models;
pub use embeddings::{
    EmbeddingConfig, EmbeddingProvider, EmbeddingProviderType, EmbeddingResult, EmbeddingService,
    ProviderConfig,
}; // Predefined model configurations

// Re-export semantic search
pub use semantic_search::{
    EmbeddingStats, HybridSearchConfig, ScoredDocument, SemanticSearchEngine,
};

// Re-export pure functions
pub use pure::btree;
pub use pure::performance;

// Re-export search validation
pub use search_validation::{
    quick_search_validation, quick_search_validation_bool, validate_post_ingestion_search,
    validate_post_ingestion_search_with_config, QuickValidationResult, ValidationCheck,
    ValidationConfig, ValidationReport, ValidationStatus,
};

// Re-export documentation verification
pub use documentation_verification::{
    DocumentationVerificationReport, DocumentationVerifier, Severity, VerificationCheck,
    VerificationStatus,
};

// Re-export LLM search functionality
pub use llm_search::{
    ContextConfig, ContextInfo, ContextType, LLMSearchEngine, LLMSearchResponse, LLMSearchResult,
    MatchDetails, MatchLocation, MatchType, OptimizationInfo, RelevanceConfig, SelectionStrategy,
    TokenUsage,
};
// Re-export bulk operations
pub use pure::{
    analyze_tree_structure, bulk_delete_from_tree, bulk_insert_into_tree, count_entries,
};

// Re-export contracts
pub use contracts::optimization as optimization_contracts;
pub use contracts::performance as performance_contracts;

// Re-export metrics
pub use metrics::optimization as optimization_metrics;
pub use metrics::performance as performance_metrics;

// Re-export symbol storage and extraction
#[cfg(feature = "tree-sitter-parsing")]
pub use symbol_storage::{
    SearchThresholds, SymbolEntry, SymbolIndexStats, SymbolRelation, SymbolStorage,
    SymbolStorageConfig,
};

// Re-export RelationType from types (always available)
pub use types::RelationType;

// Re-export symbol factory functions
// NOTE: These are deprecated in favor of binary symbol format
#[cfg(feature = "tree-sitter-parsing")]
#[allow(deprecated)]
pub use factory::{
    create_symbol_storage, create_symbol_storage_with_storage, create_test_symbol_storage,
};

/// Model Context Protocol (MCP) Server
#[cfg(feature = "mcp-server")]
pub mod mcp;

// Test modules
#[cfg(test)]
mod btree_test;
