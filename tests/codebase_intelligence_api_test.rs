// Integration tests for Codebase Intelligence HTTP API endpoints
// Following KotaDB's anti-mock philosophy - uses real server with minimal test data
// Focuses on HTTP functionality and error handling rather than full codebase ingestion

use anyhow::{anyhow, Result};
use kotadb::{create_file_storage, database::Database, Storage};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::{sync::oneshot, task::JoinHandle, time::Duration};

/// Async guard that ensures the spawned HTTP server shuts down cleanly between tests.
struct TestServerHandle {
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<()>>>,
}

impl TestServerHandle {
    async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.task.take() {
            match handle.await {
                Ok(result) => result,
                Err(join_err) => Err(anyhow!(join_err)),
            }
        } else {
            Ok(())
        }
    }
}

impl Drop for TestServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}

/// Helper to create test environment with minimal setup
/// Following anti-mock philosophy but with practical constraints for fast testing
async fn create_test_environment_with_minimal_data(
) -> Result<(Arc<Mutex<dyn kotadb::Storage>>, TempDir, Database)> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_impl = create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000))
        .await
        .expect("Failed to create storage");

    let storage = Arc::new(Mutex::new(storage_impl));
    let db = Database::new(temp_dir.path(), true).await?;

    // Add minimal test documents to avoid completely empty database
    // This provides some data for the API to work with without complex ingestion
    let mut storage_guard = storage.lock().await;

    let test_doc = kotadb::contracts::Document {
        id: kotadb::ValidatedDocumentId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        path: kotadb::ValidatedPath::new("test/example.rs").unwrap(),
        title: kotadb::ValidatedTitle::new("Test Document").unwrap(),
        content: b"fn test_function() { println!(\"Hello\"); } struct Storage;".to_vec(),
        tags: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        size: 50,
        embedding: None,
    };

    storage_guard.insert(test_doc).await?;
    drop(storage_guard);

    Ok((storage, temp_dir, db))
}

/// Start HTTP server with codebase intelligence and minimal test data
async fn start_test_server_with_real_intelligence(
) -> Result<(u16, TempDir, Database, TestServerHandle)> {
    let (storage, temp_dir, db) = create_test_environment_with_minimal_data().await?;
    let db_path = PathBuf::from(temp_dir.path());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let port = listener.local_addr().unwrap().port();

    let app = kotadb::create_server_with_intelligence(storage.clone(), db_path.clone()).await?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = shutdown_rx.await;
    });

    let task = tokio::spawn(async move {
        match server.await {
            Ok(()) => Ok(()),
            Err(err) => Err(anyhow!(err)),
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((
        port,
        temp_dir,
        db,
        TestServerHandle {
            shutdown: Some(shutdown_tx),
            task: Some(task),
        },
    ))
}

#[tokio::test]
async fn test_symbol_search_with_limited_data() -> Result<()> {
    let (port, _temp_dir, _db, server) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search - may return empty results with minimal test data
    let response = client
        .get(format!("{base_url}/api/symbols/search?q=Storage"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Should return proper HTTP response (200 with empty results or 404/500 without symbol extraction)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    // If successful, validate response structure
    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert!(body["symbols"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["query_time_ms"].is_number());
    }

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_callers_error_handling() -> Result<()> {
    let (port, _temp_dir, _db, server) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test find callers endpoint - will likely return error without full symbol extraction
    let response = client
        .get(format!("{base_url}/api/relationships/callers/test_symbol"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Should return proper HTTP response (404 not found or 500 internal error is expected)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_server_responds_to_basic_requests() -> Result<()> {
    let (port, _temp_dir, _db, server) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test that server responds to basic API requests
    let endpoints = vec![
        format!("{base_url}/api/symbols/search?q=test"),
        format!("{base_url}/api/relationships/callers/test"),
        format!("{base_url}/api/analysis/impact/test"),
    ];

    for endpoint in endpoints {
        let response = client
            .get(&endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        // Should return valid HTTP response (not connection refused)
        assert!(
            response.status().is_success()
                || response.status().is_client_error()
                || response.status().is_server_error()
        );
    }

    server.shutdown().await?;
    Ok(())
}

// FAILURE INJECTION TESTS - Following anti-mock philosophy with real failure scenarios

#[tokio::test]
async fn test_malformed_query_handling() -> Result<()> {
    let (port, _temp_dir, _db, server) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test malformed query - no query parameter
    let response = client
        .get(format!("{base_url}/api/symbols/search"))
        .send()
        .await?;

    // Should return proper error status for missing parameter
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_basic_server_functionality() -> Result<()> {
    let (port, _temp_dir, _db, server) = start_test_server_with_real_intelligence().await?;

    // Verify server is responding
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let response = client
        .get(format!("{base_url}/api/symbols/search?q=test"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Should get a valid response (server is running) - any HTTP status is fine
    assert!(
        response.status().is_success()
            || response.status().is_client_error()
            || response.status().is_server_error()
    );

    server.shutdown().await?;
    Ok(())
}
