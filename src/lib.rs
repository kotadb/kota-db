// KotaDB - A Custom Database for Distributed Cognition
// Root library module

pub mod builders;
pub mod connection_pool;
pub mod contracts;
pub mod embeddings;
pub mod file_storage;
pub mod http_server;
pub mod metrics;
pub mod observability;
pub mod primary_index;
pub mod pure;
pub mod semantic_search;
pub mod trigram_index;
pub mod types;
pub mod validation;
pub mod vector_index;
pub mod wrappers;

// Re-export key types
pub use observability::{
    init_logging, log_operation, record_metric, with_trace_id, MetricType, Operation,
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

// Re-export HTTP server and connection pool
pub use connection_pool::{
    create_connection_pool, create_rate_limiter, ConnectionPoolImpl, SystemResourceMonitor,
    TokenBucketRateLimiter,
};
pub use http_server::{create_server, create_server_with_pool, start_server};

// Re-export index implementations
pub use primary_index::{create_primary_index, create_primary_index_for_tests, PrimaryIndex};
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

/// Model Context Protocol (MCP) Server
#[cfg(feature = "mcp-server")]
pub mod mcp;

// Test modules
#[cfg(test)]
mod btree_test;
