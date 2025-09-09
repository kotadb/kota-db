// Services HTTP Server Integration Tests
// Tests the complete Services API with real HTTP requests
// Following anti-mock philosophy - uses real server and real HTTP calls

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, create_wrapped_storage,
    start_services_server,
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tokio::{sync::Mutex, time::Duration};

/// Helper to create a test storage instance
async fn create_test_storage() -> (Arc<Mutex<dyn kotadb::Storage>>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create storage");
    let wrapped = create_wrapped_storage(storage, 100).await;
    (Arc::new(Mutex::new(wrapped)), temp_dir)
}

/// Start HTTP services server on a random available port for testing
async fn start_test_server() -> (u16, TempDir, tokio::task::JoinHandle<Result<()>>) {
    let (storage, temp_dir) = create_test_storage().await;

    // Create indices for services server
    let db_path = PathBuf::from(temp_dir.path());
    let primary_index_path = db_path.join("primary_index");
    let trigram_index_path = db_path.join("trigram_index");

    let primary_index = create_primary_index(primary_index_path.to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create primary index");
    let primary_index = Arc::new(tokio::sync::Mutex::new(primary_index));

    let trigram_index = create_trigram_index(trigram_index_path.to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create trigram index");
    let trigram_index = Arc::new(tokio::sync::Mutex::new(trigram_index));

    // Use port 0 to get an available port automatically
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Close the listener so the server can bind to it

    let storage_clone = storage.clone();
    let primary_clone = primary_index.clone();
    let trigram_clone = trigram_index.clone();
    let db_path_clone = db_path.clone();

    let server_handle = tokio::spawn(async move {
        start_services_server(
            storage_clone,
            primary_clone,
            trigram_clone,
            db_path_clone,
            port,
        )
        .await
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    (port, temp_dir, server_handle)
}

#[tokio::test]
async fn test_health_check_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["version"], "0.6.0");
    assert!(body["services_enabled"].is_array());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_stats_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/api/v1/stats"))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    assert!(body.get("symbol_stats").is_some());
    assert!(body.get("relationship_stats").is_some());
    assert!(body.get("formatted_output").is_some());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_search_code_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let response = client
        .get(format!(
            "http://127.0.0.1:{port}/api/v1/search-code?query=test&limit=5"
        ))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    assert!(body.get("documents").is_some());
    assert!(body.get("llm_response").is_some());
    assert!(body.get("search_type").is_some());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_search_symbols_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let response = client
        .get(format!(
            "http://127.0.0.1:{port}/api/v1/search-symbols?pattern=test&limit=5"
        ))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    // Just verify we get a valid JSON response - structure may vary for empty DB
    assert!(body.is_object());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_find_callers_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let payload = json!({
        "symbol": "test_symbol",
        "limit": 5
    });

    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/find-callers"))
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    // Just verify we get a valid JSON response
    assert!(body.is_object());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_analyze_impact_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let payload = json!({
        "symbol": "test_symbol",
        "limit": 5
    });

    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/analyze-impact"))
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    // Impact analysis may return different fields based on results
    assert!(body.is_object());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_benchmark_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let payload = json!({
        "operations": 10,
        "benchmark_type": "storage",
        "max_search_queries": 5
    });

    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/benchmark"))
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    // Benchmark results structure may vary
    assert!(body.is_object());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_validate_endpoint_v1() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let payload = json!({
        "validation_type": "search",
        "test_queries": ["test"]
    });

    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/validate"))
        .json(&payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    // Validation results structure may vary
    assert!(body.is_object());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();

    // Test invalid JSON payload
    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/find-callers"))
        .header("Content-Type", "application/json")
        .body("invalid json")
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test missing required field
    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/find-callers"))
        .json(&json!({}))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Create multiple concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();

        let handle = tokio::spawn(async move {
            let response = client_clone
                .get(format!(
                    "{base_url_clone}/api/v1/search-symbols?pattern=test_{i}&limit=1"
                ))
                .send()
                .await?;

            assert_eq!(response.status(), StatusCode::OK);
            Result::<()>::Ok(())
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await??;
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_endpoint_performance() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;

    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test common endpoints for reasonable response times
    let endpoints = vec![
        ("health", format!("{base_url}/health")),
        ("stats", format!("{base_url}/api/v1/stats")),
        (
            "search_symbols",
            format!("{base_url}/api/v1/search-symbols?pattern=test&limit=5"),
        ),
    ];

    for (name, url) in endpoints {
        let start = std::time::Instant::now();
        let response = client.get(&url).send().await?;
        let duration = start.elapsed();

        assert_eq!(response.status(), StatusCode::OK);

        // Services should respond within reasonable time (1 second for tests)
        assert!(
            duration.as_secs() < 1,
            "Endpoint {} took {}ms, should be under 1000ms",
            name,
            duration.as_millis()
        );
    }

    server_handle.abort();
    Ok(())
}
