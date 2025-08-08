#![allow(clippy::uninlined_format_args)]
// HTTP Server Integration Tests
// Tests the complete HTTP REST API with real HTTP requests
// Following anti-mock philosophy - uses real server and real HTTP calls

use anyhow::Result;
use kotadb::{create_file_storage, create_wrapped_storage, start_server};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::{sync::Mutex, time::Duration};
use uuid::Uuid;

/// Helper to create a test storage instance
async fn create_test_storage() -> (Arc<Mutex<dyn kotadb::Storage>>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create storage");
    let wrapped = create_wrapped_storage(storage, 100).await;
    (Arc::new(Mutex::new(wrapped)), temp_dir)
}

/// Start HTTP server on a random available port for testing
async fn start_test_server() -> (u16, TempDir, tokio::task::JoinHandle<Result<()>>) {
    let (storage, temp_dir) = create_test_storage().await;

    // Use port 0 to get an available port automatically
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Close the listener so the server can bind to it

    let storage_clone = storage.clone();
    let server_handle = tokio::spawn(async move { start_server(storage_clone, port).await });

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
    assert!(body["version"].is_string());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_document_lifecycle() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // 1. Create a document
    let create_payload = json!({
        "path": "/test-doc.md",
        "title": "Test Document",
        "content": [72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100], // "Hello World"
        "tags": ["test", "integration"]
    });

    let create_response = client
        .post(format!("{base_url}/documents"))
        .json(&create_payload)
        .send()
        .await?;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let created_doc: Value = create_response.json().await?;
    let doc_id = created_doc["id"].as_str().unwrap();

    // Validate created document structure
    assert_eq!(created_doc["path"], "/test-doc.md");
    assert_eq!(created_doc["title"], "Test Document");
    assert_eq!(
        created_doc["content"],
        json!([72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100])
    );
    assert_eq!(created_doc["tags"], json!(["test", "integration"]));
    assert!(created_doc["content_hash"].is_string());
    assert_eq!(created_doc["size_bytes"], 11);

    // 2. Retrieve the document
    let get_response = client
        .get(format!("{base_url}/documents/{doc_id}"))
        .send()
        .await?;

    assert_eq!(get_response.status(), StatusCode::OK);

    let retrieved_doc: Value = get_response.json().await?;
    assert_eq!(retrieved_doc["id"], doc_id);
    assert_eq!(retrieved_doc["path"], "/test-doc.md");
    assert_eq!(retrieved_doc["title"], "Test Document");

    // 3. Update the document
    let update_payload = json!({
        "title": "Updated Test Document",
        "content": [72, 101, 108, 108, 111, 33], // "Hello!"
        "tags": ["test", "integration", "updated"]
    });

    let update_response = client
        .put(format!("{base_url}/documents/{doc_id}"))
        .json(&update_payload)
        .send()
        .await?;

    assert_eq!(update_response.status(), StatusCode::OK);

    let updated_doc: Value = update_response.json().await?;
    assert_eq!(updated_doc["id"], doc_id);
    assert_eq!(updated_doc["title"], "Updated Test Document");
    assert_eq!(updated_doc["content"], json!([72, 101, 108, 108, 111, 33]));
    assert_eq!(
        updated_doc["tags"],
        json!(["test", "integration", "updated"])
    );
    assert_eq!(updated_doc["size_bytes"], 6);

    // 4. Search for documents
    let search_response = client
        .get(format!("{base_url}/documents/search?q=Updated"))
        .send()
        .await?;

    assert_eq!(search_response.status(), StatusCode::OK);

    let search_results: Value = search_response.json().await?;
    assert!(search_results["total_count"].as_u64().unwrap() >= 1);
    assert!(!search_results["documents"].as_array().unwrap().is_empty());

    // 5. Delete the document
    let delete_response = client
        .delete(format!("{base_url}/documents/{doc_id}"))
        .send()
        .await?;

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // 6. Verify deletion - should return 404
    let get_deleted_response = client
        .get(format!("{base_url}/documents/{doc_id}"))
        .send()
        .await?;

    assert_eq!(get_deleted_response.status(), StatusCode::NOT_FOUND);

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Test invalid document ID format
    let invalid_id_response = client
        .get(format!("{base_url}/documents/invalid-uuid"))
        .send()
        .await?;

    assert_eq!(invalid_id_response.status(), StatusCode::BAD_REQUEST);
    let error_body: Value = invalid_id_response.json().await?;
    assert_eq!(error_body["error"], "invalid_id");

    // Test non-existent document
    let fake_uuid = Uuid::new_v4();
    let not_found_response = client
        .get(format!("{base_url}/documents/{fake_uuid}"))
        .send()
        .await?;

    assert_eq!(not_found_response.status(), StatusCode::NOT_FOUND);
    let error_body: Value = not_found_response.json().await?;
    assert_eq!(error_body["error"], "document_not_found");

    // Test invalid JSON in request body
    let invalid_json_response = client
        .post(format!("{base_url}/documents"))
        .header("Content-Type", "application/json")
        .body("invalid json")
        .send()
        .await?;

    assert_eq!(invalid_json_response.status(), StatusCode::BAD_REQUEST);

    // Test missing required fields
    let incomplete_payload = json!({
        "title": "Missing Path"
        // missing "path" and "content"
    });

    let incomplete_response = client
        .post(format!("{base_url}/documents"))
        .json(&incomplete_payload)
        .send()
        .await?;

    assert_eq!(
        incomplete_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Create multiple test documents
    let docs = vec![
        json!({
            "path": "/rust-guide.md",
            "title": "Rust Programming Guide",
            "content": [82, 117, 115, 116, 32, 105, 115, 32, 97, 119, 101, 115, 111, 109, 101], // "Rust is awesome"
            "tags": ["rust", "programming"]
        }),
        json!({
            "path": "/python-tutorial.py",
            "title": "Python Tutorial",
            "content": [80, 121, 116, 104, 111, 110, 32, 105, 115, 32, 101, 97, 115, 121], // "Python is easy"
            "tags": ["python", "tutorial"]
        }),
        json!({
            "path": "/javascript-notes.js",
            "title": "JavaScript Notes",
            "content": [74, 83, 32, 105, 115, 32, 101, 118, 101, 114, 121, 119, 104, 101, 114, 101], // "JS is everywhere"
            "tags": ["javascript", "web"]
        }),
    ];

    // Create all documents
    for doc in &docs {
        let response = client
            .post(format!("{base_url}/documents"))
            .json(doc)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Test search by title
    let rust_search = client
        .get(format!("{base_url}/documents/search?q=Rust"))
        .send()
        .await?;

    assert_eq!(rust_search.status(), StatusCode::OK);
    let rust_results: Value = rust_search.json().await?;
    assert!(rust_results["total_count"].as_u64().unwrap() >= 1);

    // Test search by content
    let content_search = client
        .get(format!("{base_url}/documents/search?q=awesome"))
        .send()
        .await?;

    assert_eq!(content_search.status(), StatusCode::OK);
    let content_results: Value = content_search.json().await?;
    assert!(content_results["total_count"].as_u64().unwrap() >= 1);

    // Test search with limit
    let limited_search = client
        .get(format!("{base_url}/documents/search?limit=2"))
        .send()
        .await?;

    assert_eq!(limited_search.status(), StatusCode::OK);
    let limited_results: Value = limited_search.json().await?;
    let documents = limited_results["documents"].as_array().unwrap();
    assert!(documents.len() <= 2);

    // Test empty search (should return all documents)
    let all_search = client
        .get(format!("{base_url}/documents/search"))
        .send()
        .await?;

    assert_eq!(all_search.status(), StatusCode::OK);
    let all_results: Value = all_search.json().await?;
    assert!(all_results["total_count"].as_u64().unwrap() >= 3);

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Create multiple documents concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();

        let handle = tokio::spawn(async move {
            let payload = json!({
                "path": format!("/test-doc-{i}.md"),
                "title": format!("Test Document {i}"),
                "content": format!("Content for concurrent document {i}").into_bytes(),
                "tags": ["test", "concurrent"]
            });

            let response = client_clone
                .post(format!("{base_url_clone}/documents"))
                .json(&payload)
                .send()
                .await?;

            Ok::<StatusCode, anyhow::Error>(response.status())
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let status = handle.await??;
        assert_eq!(status, StatusCode::CREATED);
    }

    // Add a small delay to ensure documents are fully persisted
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify all documents were created by searching
    let search_response = client
        .get(format!("{base_url}/documents/search?q=concurrent"))
        .send()
        .await?;

    assert_eq!(search_response.status(), StatusCode::OK);
    let search_results: Value = search_response.json().await?;
    assert!(search_results["total_count"].as_u64().unwrap() >= 10);

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_performance_response_times() -> Result<()> {
    let (port, _temp_dir, server_handle) = start_test_server().await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // Create a test document
    let create_payload = json!({
        "path": "/performance-test.md",
        "title": "Performance Test Document",
        "content": [80, 101, 114, 102, 111, 114, 109, 97, 110, 99, 101], // "Performance"
        "tags": ["performance", "test"]
    });

    let create_response = client
        .post(format!("{base_url}/documents"))
        .json(&create_payload)
        .send()
        .await?;

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let doc: Value = create_response.json().await?;
    let doc_id = doc["id"].as_str().unwrap();

    // Test response times for various operations
    let operations = vec![
        ("health", format!("{base_url}/health")),
        ("get_document", format!("{base_url}/documents/{doc_id}")),
        (
            "search",
            format!("{base_url}/documents/search?q=performance"),
        ),
    ];

    for (operation_name, url) in operations {
        let start = std::time::Instant::now();

        let response = client.get(&url).send().await?;
        let elapsed = start.elapsed();

        assert_eq!(response.status(), StatusCode::OK);

        // Performance requirement: <10ms for simple operations
        println!("{operation_name} response time: {elapsed:?}");

        // Note: In real tests, we might want to be more lenient due to test environment variability
        // For now, just ensure it's under a reasonable threshold (100ms)
        assert!(
            elapsed < Duration::from_millis(100),
            "{operation_name} took too long: {elapsed:?}"
        );
    }

    server_handle.abort();
    Ok(())
}
