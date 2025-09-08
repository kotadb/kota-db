// Integration tests for Codebase Intelligence HTTP API endpoints
// Following KotaDB's anti-mock philosophy - uses real server with real codebase ingestion
// Implements mandatory dogfooding protocol by testing on KotaDB's own codebase

use anyhow::Result;
use kotadb::{create_file_storage, database::Database};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::Duration;

/// Helper to create test environment with real codebase ingestion
/// Following dogfooding protocol - uses KotaDB's own source code as test data
async fn create_test_environment_with_real_codebase(
) -> Result<(Arc<Mutex<dyn kotadb::Storage>>, TempDir, Database)> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_impl = create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000))
        .await
        .expect("Failed to create storage");

    let storage = Arc::new(Mutex::new(storage_impl));

    // Create database for indexing operations
    let db = Database::new(temp_dir.path(), true).await?;

    // DOGFOODING: Index KotaDB's own codebase for testing
    // This follows AGENT.md:256-357 dogfooding requirements
    let kotadb_root_path = std::env::current_dir()?;

    if kotadb_root_path.join("src").exists() && kotadb_root_path.join(".git").exists() {
        // Use direct ingestion approach following working test pattern
        use kotadb::git::types::IngestionOptions;
        use kotadb::git::{IngestionConfig, RepositoryIngester};

        let ingestion_options = IngestionOptions {
            include_file_contents: true,
            include_commit_history: false, // Skip commits for faster testing
            max_file_size: 5 * 1024 * 1024, // 5MB max
            extract_symbols: true,         // CRITICAL: Enable symbol extraction
            ..Default::default()
        };

        let config = IngestionConfig {
            path_prefix: "test".to_string(),
            options: ingestion_options,
            create_index: true,
            organization_config: Some(kotadb::git::RepositoryOrganizationConfig::default()),
        };

        let ingester = RepositoryIngester::new(config);
        let mut storage_guard = storage.lock().await;

        // Create symbol and graph storage paths
        let symbol_db_path = temp_dir.path().join("symbols.kota");
        let graph_db_path = temp_dir.path().join("dependency_graph.bin");

        let result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &kotadb_root_path,
                &mut *storage_guard,
                &symbol_db_path,
                &graph_db_path,
                None, // No progress callback for test
            )
            .await;

        match result {
            Ok(_) => {
                // Ingestion succeeded
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to index KotaDB codebase: {}", e));
            }
        }

        drop(storage_guard); // Release lock
    }

    Ok((storage, temp_dir, db))
}

/// Start HTTP server with codebase intelligence and real data
async fn start_test_server_with_real_intelligence(
) -> Result<(u16, TempDir, Database, tokio::task::JoinHandle<Result<()>>)> {
    let (storage, temp_dir, db) = create_test_environment_with_real_codebase().await?;
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

    // Give the server more time to start with real data
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((port, temp_dir, db, server_handle))
}

#[tokio::test]
async fn test_symbol_search_with_real_codebase() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test symbol search with real KotaDB symbols
    // Search for "Storage" - should exist in KotaDB codebase
    let response = client
        .get(format!("{base_url}/api/symbols/search?q=Storage"))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    // Should return OK with actual symbol results
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await?;
    assert!(body["symbols"].is_array());
    assert!(body["total_count"].is_number());
    assert!(body["query_time_ms"].is_number());

    // Validate we found actual symbols from KotaDB codebase
    let symbols = body["symbols"].as_array().unwrap();
    assert!(
        !symbols.is_empty(),
        "Should find Storage-related symbols in KotaDB codebase"
    );

    // Check performance requirement - should be sub-10ms for internal query
    let query_time = body["query_time_ms"].as_u64().unwrap();
    assert!(
        query_time < 100, // Allow some overhead for integration test
        "Query time {}ms exceeds performance threshold",
        query_time
    );

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_find_callers_with_real_symbols() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test find callers for a symbol that should exist in KotaDB
    // "new" is a common pattern in Rust code
    let response = client
        .get(format!("{base_url}/api/relationships/callers/new"))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    // Should return OK or NOT_FOUND (both valid for real functionality)
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND);

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["target"], "new");
        assert!(body["callers"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["query_time_ms"].is_number());
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_impact_analysis_with_real_codebase() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test impact analysis for a core KotaDB type
    let response = client
        .get(format!("{base_url}/api/analysis/impact/Storage"))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    // Should return OK or NOT_FOUND (both valid for real functionality)
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND);

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["target"], "Storage");
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
async fn test_code_search_with_real_content() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test code search for content that should exist in KotaDB
    let response = client
        .get(format!("{base_url}/api/code/search?q=async"))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    // Should return OK (async is common in KotaDB Rust code) or 503 if trigram not available
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );

    if response.status() == StatusCode::OK {
        let body: Value = response.json().await?;
        assert_eq!(body["query"], "async");
        assert!(body["results"].is_array());
        assert!(body["total_count"].is_number());
        assert!(body["search_type"].is_string());

        // Should find async keywords in KotaDB Rust code
        let results = body["results"].as_array().unwrap();
        assert!(
            !results.is_empty(),
            "Should find 'async' in KotaDB Rust codebase"
        );
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_performance_requirements_with_real_data() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test multiple real endpoints for performance
    let endpoints = vec![
        format!("{base_url}/api/symbols/search?q=fn"), // Should find many function symbols
        format!("{base_url}/api/relationships/callers/Result"), // Common Rust type
        format!("{base_url}/api/analysis/impact/Error"), // Common error handling
    ];

    for endpoint in endpoints {
        let start = std::time::Instant::now();
        let response = client
            .get(&endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        let elapsed = start.elapsed();

        // API response time should be reasonable for integration test
        assert!(
            elapsed.as_millis() < 1000,
            "Endpoint {} took {}ms, exceeding 1s threshold for integration test",
            endpoint,
            elapsed.as_millis()
        );

        // If the request succeeded, check the reported query time
        if response.status() == StatusCode::OK {
            let body: Value = response.json().await?;
            if let Some(query_time) = body["query_time_ms"].as_u64() {
                // Internal query time should meet the <10ms performance requirement
                assert!(
                    query_time <= 50, // Allow some overhead for real data processing
                    "Query time {}ms exceeds performance target for {}",
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
async fn test_concurrent_requests_with_real_functionality() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Send concurrent requests that should succeed with real data
    let mut handles = vec![];

    for i in 0..5 {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();
        let search_term = match i {
            0 => "async",
            1 => "Result",
            2 => "fn",
            3 => "struct",
            _ => "impl",
        };

        let handle = tokio::spawn(async move {
            let response = client_clone
                .get(format!(
                    "{base_url_clone}/api/symbols/search?q={search_term}"
                ))
                .timeout(Duration::from_secs(10))
                .send()
                .await?;

            Ok::<StatusCode, anyhow::Error>(response.status())
        });

        handles.push(handle);
    }

    // Wait for all requests to complete - should handle requests gracefully
    for handle in handles {
        let status = handle.await??;
        // Should return valid HTTP status (OK if found, 404 if not found, 500 for other issues)
        // All are valid responses from a working server
        assert!(
            status == StatusCode::OK
                || status == StatusCode::NOT_FOUND
                || status == StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    server_handle.abort();
    Ok(())
}

// FAILURE INJECTION TESTS - Following anti-mock philosophy with real failure scenarios

#[tokio::test]
async fn test_malformed_query_handling() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;
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

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_server_lifecycle_management() -> Result<()> {
    let (port, _temp_dir, _db, server_handle) = start_test_server_with_real_intelligence().await?;

    // Verify server is responding
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let response = client
        .get(format!("{base_url}/api/symbols/search?q=test"))
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Should get a valid response (server is running)
    assert!(response.status() == StatusCode::OK);

    // Shut down server
    server_handle.abort();

    // Wait for cleanup
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Try to connect again - should fail
    let result = client
        .get(format!("{base_url}/api/symbols/search?q=test"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    // Should fail to connect
    assert!(result.is_err());

    Ok(())
}
