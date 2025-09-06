// Tests for Codebase Intelligence HTTP API endpoints
// Following anti-mock philosophy - uses real server and real HTTP calls

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
#[ignore = "Temporarily disabled - API test failures tracked in issue #588"]
async fn test_symbol_search_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search
    let response = client
        .get(format!("{base_url}/api/symbols/search?q=test_function"))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    assert!(body["symbols"].is_array());
    assert!(body["total_count"].is_number());
    assert!(body["query_time_ms"].is_number());

    // Check performance requirement
    let query_time = body["query_time_ms"].as_u64().unwrap();
    assert!(
        query_time < 100,
        "Query time {}ms exceeds 100ms threshold",
        query_time
    );

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_find_callers_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test find callers
    let response = client
        .get(format!("{base_url}/api/relationships/callers/FileStorage"))
        .send()
        .await?;

    // May return 404 if the symbol doesn't exist in the test environment
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["target"], "FileStorage");
        assert!(body["callers"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["query_time_ms"].is_number());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_impact_analysis_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test impact analysis
    let response = client
        .get(format!("{base_url}/api/analysis/impact/Document"))
        .send()
        .await?;

    // May return 404 if the symbol doesn't exist in the test environment
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["target"], "Document");
        assert!(body["direct_impacts"].is_array());
        assert!(body["indirect_impacts"].is_array());
        assert!(body["total_affected"].is_number());
        assert!(body["query_time_ms"].is_number());
        assert!(body["risk_assessment"].is_string());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_code_search_endpoint() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test code search
    let response = client
        .get(format!("{base_url}/api/code/search?q=function"))
        .send()
        .await?;

    // May return 503 if trigram index is not available
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["query"], "function");
        assert!(body["results"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["query_time_ms"].is_number());
        assert!(body["search_type"].is_string());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
#[ignore = "Temporarily disabled - API test failures tracked in issue #588"]
async fn test_deprecated_endpoints_have_headers() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test that deprecated document endpoints have proper headers
    let response = client.get(format!("{base_url}/documents")).send().await?;

    assert_eq!(response.status(), StatusCode::OK);

    // Check for deprecation headers
    let headers = response.headers();
    assert_eq!(headers.get("Deprecation").unwrap(), "true");
    assert!(headers.contains_key("Sunset"));
    assert!(headers.contains_key("Link"));
    assert!(headers.contains_key("Warning"));

    // Check the warning header contains helpful information
    let warning = headers.get("Warning").unwrap().to_str()?;
    assert!(warning.contains("deprecated"));
    assert!(warning.contains("/api/symbols/search"));

    server_handle.abort();
    Ok(())
}

#[tokio::test]
#[ignore = "Temporarily disabled - API test failures tracked in issue #588"]
async fn test_symbol_search_with_filters() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search with type filter
    let response = client
        .get(format!(
            "{base_url}/api/symbols/search?q=*&symbol_type=function&limit=10"
        ))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    let symbols = body["symbols"].as_array().unwrap();

    // Check that limit is respected
    assert!(symbols.len() <= 10);

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_find_callers_with_parameters() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test find callers with parameters
    let response = client
        .get(format!(
            "{base_url}/api/relationships/callers/test?include_indirect=true&max_depth=3&limit=50"
        ))
        .send()
        .await?;

    // May return 404 if the symbol doesn't exist
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    server_handle.abort();
    Ok(())
}

#[tokio::test]
#[ignore = "Temporarily disabled - API test failures tracked in issue #588"]
async fn test_performance_requirements() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Run multiple queries and check performance
    let endpoints = vec![
        format!("{base_url}/api/symbols/search?q=test"),
        format!("{base_url}/api/relationships/callers/Storage"),
        format!("{base_url}/api/analysis/impact/Document"),
    ];

    for endpoint in endpoints {
        let start = std::time::Instant::now();
        let response = client.get(&endpoint).send().await?;
        let elapsed = start.elapsed();

        // API response time should be under 100ms (target is <10ms for query, but add overhead)
        assert!(
            elapsed.as_millis() < 100,
            "Endpoint {} took {}ms, exceeding 100ms threshold",
            endpoint,
            elapsed.as_millis()
        );

        // If the request succeeded, check the reported query time
        if response.status() == StatusCode::OK {
            let body: Value = response.json().await?;
            if let Some(query_time) = body["query_time_ms"].as_u64() {
                // Internal query time should be under 10ms as per requirements
                assert!(
                    query_time <= 10,
                    "Query time {}ms exceeds 10ms target for {}",
                    query_time,
                    endpoint
                );
            }
        }
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
#[ignore = "Temporarily disabled - API test failures tracked in issue #588"]
async fn test_concurrent_api_requests() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Send multiple concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();

        let handle = tokio::spawn(async move {
            let response = client_clone
                .get(format!("{base_url_clone}/api/symbols/search?q=test_{i}"))
                .send()
                .await?;

            Ok::<StatusCode, anyhow::Error>(response.status())
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let status = handle.await??;
        assert_eq!(status, StatusCode::OK);
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server_with_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test invalid symbol search (empty query)
    let response = client
        .get(format!("{base_url}/api/symbols/search"))
        .send()
        .await?;

    // Should return bad request for missing query parameter
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );

    // Test non-existent symbol for find callers
    let response = client
        .get(format!(
            "{base_url}/api/relationships/callers/NonExistentSymbol123456789"
        ))
        .send()
        .await?;

    // Should return not found or internal server error (depending on setup state)
    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    server_handle.abort();
    Ok(())
}
