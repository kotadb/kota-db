//! Async wrapper for BinaryRelationshipEngine to prevent blocking the tokio runtime
//!
//! This module provides an async-safe wrapper around BinaryRelationshipEngine that uses
//! tokio::task::spawn_blocking to ensure CPU-intensive operations don't block the async runtime.
//! This is critical for HTTP API integration where blocking operations would degrade performance.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info, instrument};

use crate::{
    binary_relationship_engine::{BinaryRelationshipEngine, ExtractionConfig},
    relationship_query::{RelationshipQueryConfig, RelationshipQueryResult, RelationshipQueryType},
};

/// Shared runtime for executing blocking operations
/// This avoids the overhead of creating a new runtime for each query
static BLOCKING_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2) // Small pool for blocking operations
        .thread_name("kotadb-blocking")
        .enable_all()
        .build()
        .expect("Failed to create blocking runtime")
});

/// Thread-safe async wrapper for BinaryRelationshipEngine
///
/// This wrapper ensures all potentially blocking operations are executed
/// on a dedicated thread pool via spawn_blocking, preventing them from
/// blocking the tokio runtime when called from async contexts like HTTP handlers.
#[derive(Clone)]
pub struct AsyncBinaryRelationshipEngine {
    /// The underlying engine wrapped in Arc for thread-safe sharing
    engine: Arc<BinaryRelationshipEngine>,
}

impl AsyncBinaryRelationshipEngine {
    /// Create a new async binary engine from database paths
    #[instrument]
    pub async fn new(db_path: &Path, config: RelationshipQueryConfig) -> Result<Self> {
        let engine = BinaryRelationshipEngine::new(db_path, config).await?;
        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Create a new async binary engine with custom extraction configuration
    #[instrument]
    pub async fn with_extraction_config(
        db_path: &Path,
        config: RelationshipQueryConfig,
        extraction_config: ExtractionConfig,
    ) -> Result<Self> {
        let engine =
            BinaryRelationshipEngine::with_extraction_config(db_path, config, extraction_config)
                .await?;
        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Execute a relationship query with proper async/sync boundary handling
    ///
    /// This method uses spawn_blocking to ensure the potentially CPU-intensive
    /// query execution doesn't block the tokio runtime. This is essential for
    /// maintaining responsive HTTP endpoints.
    #[instrument(skip(self))]
    pub async fn execute_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        info!("Executing async relationship query: {:?}", query_type);

        // Clone the Arc to move into the spawn_blocking closure
        let engine = self.engine.clone();

        // Use spawn_blocking to run the query on a dedicated thread pool
        // This prevents blocking the tokio runtime
        let result = tokio::task::spawn_blocking(move || {
            // Use the shared runtime pool instead of creating a new one each time
            // This eliminates the ~1-2ms overhead of runtime creation
            BLOCKING_RUNTIME.block_on(async move { engine.execute_query(query_type).await })
        })
        .await
        .context("Task join error")?;

        debug!("Async query execution completed");
        result
    }

    /// Execute a find callers query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_callers(&self, target: &str) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::FindCallers {
            target: target.to_string(),
        };
        self.execute_query(query_type).await
    }

    /// Execute an impact analysis query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn analyze_impact(&self, target: &str) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::ImpactAnalysis {
            target: target.to_string(),
        };
        self.execute_query(query_type).await
    }

    /// Execute a find callees query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_callees(&self, target: &str) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::FindCallees {
            target: target.to_string(),
        };
        self.execute_query(query_type).await
    }

    /// Execute a call chain query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_call_chain(&self, from: &str, to: &str) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::CallChain {
            from: from.to_string(),
            to: to.to_string(),
        };
        self.execute_query(query_type).await
    }

    /// Execute a circular dependencies query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_circular_dependencies(
        &self,
        target: Option<String>,
    ) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::CircularDependencies { target };
        self.execute_query(query_type).await
    }

    /// Execute an unused symbols query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_unused_symbols(
        &self,
        symbol_type: Option<crate::parsing::SymbolType>,
    ) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::UnusedSymbols { symbol_type };
        self.execute_query(query_type).await
    }

    /// Execute a hot paths query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_hot_paths(&self, limit: Option<usize>) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::HotPaths { limit };
        self.execute_query(query_type).await
    }

    /// Execute a dependencies by type query with async safety
    ///
    /// Convenience method that wraps the query construction and execution
    #[instrument(skip(self))]
    pub async fn find_dependencies_by_type(
        &self,
        target: &str,
        relation_type: crate::types::RelationType,
    ) -> Result<RelationshipQueryResult> {
        let query_type = RelationshipQueryType::DependenciesByType {
            target: target.to_string(),
            relation_type,
        };
        self.execute_query(query_type).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_async_engine_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        // Should succeed even without binary symbols
        let result = AsyncBinaryRelationshipEngine::new(db_path, config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_engine_with_custom_config() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();
        let extraction_config = ExtractionConfig::default();

        let result = AsyncBinaryRelationshipEngine::with_extraction_config(
            db_path,
            config,
            extraction_config,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_query_without_symbols() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config)
            .await
            .expect("Failed to create engine");

        // Query should fail gracefully without symbols
        let result = engine.find_callers("TestSymbol").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_async_queries() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config)
            .await
            .expect("Failed to create engine");

        // Test that multiple concurrent queries don't cause issues
        let engine1 = engine.clone();
        let engine2 = engine.clone();
        let engine3 = engine.clone();

        let handle1 = tokio::spawn(async move {
            let _ = engine1.find_callers("Symbol1").await;
        });

        let handle2 = tokio::spawn(async move {
            let _ = engine2.analyze_impact("Symbol2").await;
        });

        let handle3 = tokio::spawn(async move {
            let _ = engine3.find_callees("Symbol3").await;
        });

        // All handles should complete without panicking
        assert!(handle1.await.is_ok());
        assert!(handle2.await.is_ok());
        assert!(handle3.await.is_ok());
    }

    #[tokio::test]
    async fn test_all_convenience_methods() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config)
            .await
            .expect("Failed to create engine");

        // Test all convenience methods (they should fail gracefully without data)
        let _ = engine.find_callers("test").await;
        let _ = engine.analyze_impact("test").await;
        let _ = engine.find_callees("test").await;
        let _ = engine.find_call_chain("from", "to").await;
        let _ = engine.find_circular_dependencies(None).await;
        let _ = engine.find_unused_symbols(None).await;
        let _ = engine.find_hot_paths(Some(10)).await;
        let _ = engine
            .find_dependencies_by_type("test", crate::types::RelationType::Calls)
            .await;

        // If we get here without panicking, the test passes
    }
}
