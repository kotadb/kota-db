// KotaDB - A Custom Database for Distributed Cognition
// Root library module

pub mod observability;
pub mod contracts;
pub mod validation;
pub mod pure;
pub mod types;
pub mod builders;
pub mod wrappers;
pub mod file_storage;
pub mod primary_index;
pub mod metrics;

// Re-export key types
pub use observability::{
    init_logging, 
    Operation, 
    MetricType,
    log_operation,
    record_metric,
    with_trace_id,
};

pub use contracts::{
    Storage,
    Index,
    Document,
    Query,
    StorageMetrics,
    PageId,
    Transaction,
};

// Re-export validated types
pub use types::{
    ValidatedPath,
    ValidatedDocumentId,
    ValidatedTitle,
    NonZeroSize,
    ValidatedTimestamp,
    TimestampPair,
    ValidatedTag,
    ValidatedSearchQuery,
    ValidatedPageId,
    ValidatedLimit,
};

// Re-export builders
pub use builders::{
    DocumentBuilder,
    QueryBuilder,
    StorageConfigBuilder,
    IndexConfigBuilder,
    MetricsBuilder,
};

// Re-export wrappers
pub use wrappers::{
    TracedStorage,
    ValidatedStorage,
    RetryableStorage,
    CachedStorage,
    MeteredIndex,
    SafeTransaction,
    create_wrapped_storage,
};

// Re-export optimization wrappers
pub use wrappers::optimization::{
    OptimizedIndex,
    OptimizationConfig,
    OptimizationReport,
    create_optimized_index,
    create_optimized_index_with_defaults,
};

// Re-export storage implementations
pub use file_storage::{FileStorage, create_file_storage};

// Re-export index implementations
pub use primary_index::{PrimaryIndex, create_primary_index, create_primary_index_for_tests};

// Re-export pure functions
pub use pure::btree;
pub use pure::performance;
// Re-export bulk operations
pub use pure::{
    bulk_insert_into_tree,
    bulk_delete_from_tree,
    count_entries,
    analyze_tree_structure,
};

// Re-export contracts
pub use contracts::performance as performance_contracts;
pub use contracts::optimization as optimization_contracts;

// Re-export metrics
pub use metrics::performance as performance_metrics;
pub use metrics::optimization as optimization_metrics;

// Test modules
#[cfg(test)]
mod btree_test;