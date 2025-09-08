// Tests for Codebase Intelligence HTTP API endpoints
// Following anti-mock philosophy - uses real server and real HTTP calls
// NOTE: Tests validate API structure and error handling, not full functionality
// Full functionality requires proper codebase ingestion and symbol extraction

use anyhow::Result;
use kotadb::create_file_storage;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::Duration;

/// Helper to create test storage and database path
async fn create_test_environment() -> (Arc<Mutex<dyn kotadb::Storage>>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create storage");
    (Arc::new(Mutex::new(storage)), temp_dir)
}

/// Start HTTP server with codebase intelligence on a random port
async fn start_test_server_with_intelligence(
) -> Result<(u16, TempDir, tokio::task::JoinHandle<Result<()>>)> {
    let (storage, temp_dir) = create_test_environment().await;
    let db_path = PathBuf::from(temp_dir.path());

    // Use port 0 to get an available port automatically
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Close the listener so the server can bind to it

    let storage_clone = storage.clone();
    let db_path_clone = db_path.clone();
    let server_handle = tokio::spawn(async move {
        kotadb::start_server_with_intelligence(storage_clone, db_path_clone, port).await
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    Ok((port, temp_dir, server_handle))
}

#[tokio::test]
async fn test_server_starts_successfully() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;

    // Just verify the server started and is listening
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test a simple endpoint that should respond (even if with an error)
    let response = client
        .get(format!("{base_url}/api/symbols/search?q=test"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Server should respond (not connection refused)
    // Response could be 500 (no data) or other status, but should not timeout
    assert!(response.status().is_client_error() || response.status().is_server_error());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_symbol_search_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search with empty database - should return proper error
    let response = client
        .get(format!("{base_url}/api/symbols/search?q=test_function"))
        .send()
        .await?;

    // Should return 500 with proper error message about missing data
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body: Value = response.json().await?;
    assert!(body["error"].is_string());
    assert!(body["message"].is_string());

    let error_message = body["message"].as_str().unwrap();
    assert!(error_message.contains("symbol") || error_message.contains("relationship"));

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_missing_query_parameter() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search without query parameter
    let response = client
        .get(format!("{base_url}/api/symbols/search"))
        .send()
        .await?;

    // Should return bad request for missing query parameter
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_find_callers_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test find callers with empty database
    let response = client
        .get(format!("{base_url}/api/relationships/callers/TestSymbol"))
        .send()
        .await?;

    // Should return error (500, 404, etc.) with proper error structure
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::NOT_FOUND
    );

    if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
        let body: Value = response.json().await?;
        assert!(body["error"].is_string());
        assert!(body["message"].is_string());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_impact_analysis_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test impact analysis with empty database
    let response = client
        .get(format!("{base_url}/api/analysis/impact/TestSymbol"))
        .send()
        .await?;

    // Should return error with proper error structure
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::NOT_FOUND
    );

    if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
        let body: Value = response.json().await?;
        assert!(body["error"].is_string());
        assert!(body["message"].is_string());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_code_search_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test code search - may work even without full setup depending on trigram index
    let response = client
        .get(format!("{base_url}/api/code/search?q=function"))
        .send()
        .await?;

    // May return 503 if trigram index is not available, or other error codes
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    // If it succeeds, validate response structure
    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert!(body["query"].is_string());
        assert!(body["results"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["search_type"].is_string());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_concurrent_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Send multiple concurrent requests that should all fail gracefully
    let mut handles = vec![];

    for i in 0..5 {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();

        let handle = tokio::spawn(async move {
            let response = client_clone
                .get(format!("{base_url_clone}/api/symbols/search?q=test_{i}"))
                .timeout(Duration::from_secs(10))
                .send()
                .await?;

            Ok::<StatusCode, anyhow::Error>(response.status())
        });

        handles.push(handle);
    }

    // Wait for all requests to complete - they should all return errors but not hang
    for handle in handles {
        let status = handle.await??;
        // Should return some error status, not OK since no data is ingested
        assert!(status.is_client_error() || status.is_server_error());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_server_cleanup() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;

    // Test that server can be properly shut down
    server_handle.abort();

    // Wait a bit for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect again - should fail
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let result = client
        .get(format!("{base_url}/api/symbols/search?q=test"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    // Should fail to connect
    assert!(result.is_err());

    Ok(())
}
