---
tags:
- file
- kota-db
- ext_rs
---
//! Integration test to verify BinaryRelationshipEngine works correctly with async HTTP handlers
//!
//! This test simulates the scenario where HTTP handlers call the BinaryRelationshipEngine
//! and ensures that the async wrapper properly prevents blocking the tokio runtime.

#[cfg(feature = "tree-sitter-parsing")]
mod async_handler_tests {
    use anyhow::Result;
    use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
    use kotadb::{
        binary_relationship_engine_async::AsyncBinaryRelationshipEngine,
        relationship_query::RelationshipQueryConfig,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::timeout;

    #[derive(Clone)]
    struct AppState {
        engine: Arc<AsyncBinaryRelationshipEngine>,
    }

    #[derive(Serialize, Deserialize)]
    struct FindCallersRequest {
        target: String,
    }

    #[derive(Serialize)]
    struct FindCallersResponse {
        message: String,
        direct_count: usize,
    }

    /// HTTP handler that uses the async engine
    async fn find_callers_handler(
        State(state): State<AppState>,
        Json(request): Json<FindCallersRequest>,
    ) -> Result<Json<FindCallersResponse>, StatusCode> {
        // This should not block the tokio runtime
        match state.engine.find_callers(&request.target).await {
            Ok(result) => Ok(Json(FindCallersResponse {
                message: result.summary,
                direct_count: result.stats.direct_count,
            })),
            Err(_) => Ok(Json(FindCallersResponse {
                message: "No binary symbols available".to_string(),
                direct_count: 0,
            })),
        }
    }

    /// Test that the async engine works correctly in HTTP handlers
    #[tokio::test]
    async fn test_async_engine_in_http_handler() -> Result<()> {
        // Setup
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        // Create async engine
        let engine = AsyncBinaryRelationshipEngine::new(db_path, config).await?;
        let state = AppState {
            engine: Arc::new(engine),
        };

        // Create router
        let app = Router::new()
            .route("/api/find-callers", get(find_callers_handler))
            .with_state(state.clone());

        // Start server in background
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Make concurrent requests to test thread safety
        let client = reqwest::Client::new();
        let base_url = format!("http://{}", addr);

        let mut handles = vec![];
        for i in 0..5 {
            let client = client.clone();
            let url = format!("{}/api/find-callers", base_url);
            let handle = tokio::spawn(async move {
                let response = client
                    .get(&url)
                    .json(&FindCallersRequest {
                        target: format!("TestSymbol{}", i),
                    })
                    .send()
                    .await;
                response.is_ok()
            });
            handles.push(handle);
        }

        // Wait for all requests with timeout
        for handle in handles {
            let result = timeout(Duration::from_secs(5), handle).await;
            assert!(
                result.is_ok(),
                "Request timed out - possible runtime blocking"
            );
            assert!(result.unwrap()?, "Request failed");
        }

        // Cleanup
        server_handle.abort();
        Ok(())
    }

    /// Test that multiple concurrent queries don't block each other
    #[tokio::test]
    async fn test_concurrent_queries_dont_block() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config).await?;
        let engine = Arc::new(engine);

        // Launch multiple concurrent queries
        let mut handles = vec![];
        for i in 0..10 {
            let engine = engine.clone();
            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();
                let _ = engine.find_callers(&format!("Symbol{}", i)).await;
                start.elapsed()
            });
            handles.push(handle);
        }

        // All queries should complete quickly (not serialized)
        let mut total_time = Duration::from_secs(0);
        for handle in handles {
            let elapsed = timeout(Duration::from_secs(1), handle)
                .await?
                .expect("Task panicked");
            total_time += elapsed;
        }

        // If queries were serialized, total time would be much higher
        // With proper async handling, they should run concurrently
        assert!(
            total_time < Duration::from_secs(5),
            "Queries appear to be blocking each other"
        );

        Ok(())
    }

    /// Test that the engine handles errors gracefully in async context
    #[tokio::test]
    async fn test_error_handling_in_async_context() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config).await?;

        // Without binary symbols, queries should fail gracefully
        let result = engine.find_callers("NonExistentSymbol").await;
        assert!(result.is_err());

        // Error should not cause panic or runtime issues
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("Binary symbol reader not available")
                || error_message.contains("not found")
                || error_message.contains("Legacy relationship queries")
                || error_message.contains("binary symbols"),
            "Unexpected error: {}",
            error_message
        );

        Ok(())
    }

    /// Test that engine can be safely shared across tasks
    #[tokio::test]
    async fn test_engine_sharing_across_tasks() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path();
        let config = RelationshipQueryConfig::default();

        let engine = AsyncBinaryRelationshipEngine::new(db_path, config).await?;
        let engine = Arc::new(engine);

        // Share engine across multiple tasks
        let engine1 = engine.clone();
        let task1 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = engine1.find_callers("Task1Symbol").await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        let engine2 = engine.clone();
        let task2 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = engine2.analyze_impact("Task2Symbol").await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        let engine3 = engine.clone();
        let task3 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = engine3.find_callees("Task3Symbol").await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // All tasks should complete without issues
        let results = tokio::try_join!(task1, task2, task3);
        assert!(results.is_ok(), "Tasks failed to complete");

        Ok(())
    }
}
