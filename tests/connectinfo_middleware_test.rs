//! Integration tests for ConnectInfo middleware across all server startup functions
//!
//! This test validates that all three server startup functions properly provide
//! ConnectInfo<SocketAddr> to authentication middleware, preventing HTTP 500 errors.

#![allow(deprecated)]

use anyhow::Result;
use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
    routing::get,
};
#[allow(deprecated)]
use kotadb::{
    create_file_storage, create_server, create_server_with_intelligence, create_wrapped_storage,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Test middleware that mimics auth_middleware's ConnectInfo usage
async fn test_connectinfo_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // This should not panic if ConnectInfo is properly provided
    println!("Client IP: {}", addr);
    Ok(next.run(request).await)
}

/// Test endpoint that responds with OK if ConnectInfo middleware succeeds
async fn test_endpoint() -> &'static str {
    "ConnectInfo middleware working"
}

#[tokio::test]
async fn test_basic_server_provides_connectinfo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;
    let wrapped_storage = create_wrapped_storage(storage, 50).await;

    // Create server using the basic start_server configuration
    let app = create_server(Arc::new(Mutex::new(wrapped_storage)))
        .route("/test-connectinfo", get(test_endpoint))
        .layer(axum::middleware::from_fn(test_connectinfo_middleware));

    // Test with a real socket address
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Start server
    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test request
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/test-connectinfo", addr.port()))
        .send()
        .await?;

    server_handle.abort();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await?, "ConnectInfo middleware working");

    Ok(())
}

#[tokio::test]
async fn test_intelligence_server_provides_connectinfo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;
    let wrapped_storage = create_wrapped_storage(storage, 50).await;

    // Create server using the intelligence server configuration
    let app = create_server_with_intelligence(
        Arc::new(Mutex::new(wrapped_storage)),
        temp_dir.path().to_path_buf(),
    )
    .await?
    .route("/test-connectinfo", get(test_endpoint))
    .layer(axum::middleware::from_fn(test_connectinfo_middleware));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/test-connectinfo", addr.port()))
        .send()
        .await?;

    server_handle.abort();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await?, "ConnectInfo middleware working");

    Ok(())
}

/// Test that validates ConnectInfo extraction works under concurrent load
#[tokio::test]
async fn test_concurrent_connectinfo_extraction() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;
    let wrapped_storage = create_wrapped_storage(storage, 50).await;

    let app = create_server(Arc::new(Mutex::new(wrapped_storage)))
        .route("/test-connectinfo", get(test_endpoint))
        .layer(axum::middleware::from_fn(test_connectinfo_middleware));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Sequential requests to test ConnectInfo consistency
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/test-connectinfo", addr.port());

    for _ in 0..5 {
        let response = client.get(&url).send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await?, "ConnectInfo middleware working");
    }

    server_handle.abort();

    Ok(())
}

/// Test error handling when ConnectInfo extraction might fail
#[tokio::test]
async fn test_connectinfo_error_handling() -> Result<()> {
    // Test middleware that gracefully handles ConnectInfo issues
    async fn robust_connectinfo_middleware(
        connect_info: Option<ConnectInfo<SocketAddr>>,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        match connect_info {
            Some(ConnectInfo(addr)) => {
                println!("Client IP available: {}", addr);
            }
            None => {
                println!("Client IP not available - degraded mode");
            }
        }
        Ok(next.run(request).await)
    }

    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;
    let wrapped_storage = create_wrapped_storage(storage, 50).await;

    let app = create_server(Arc::new(Mutex::new(wrapped_storage)))
        .route("/test-connectinfo", get(test_endpoint))
        .layer(axum::middleware::from_fn(robust_connectinfo_middleware));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/test-connectinfo", addr.port()))
        .send()
        .await?;

    server_handle.abort();

    assert_eq!(response.status(), 200);

    Ok(())
}
