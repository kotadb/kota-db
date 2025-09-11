//! Integration tests for auth middleware on /mcp/* endpoints using ephemeral Postgres.

use anyhow::Result;
use axum::{middleware, routing::get};
use kotadb::{
    api_keys::{ApiKeyConfig, ApiKeyService},
    auth_middleware::auth_middleware,
};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use tokio::net::TcpListener;

#[tokio::test]
#[ignore] // Requires Docker; run with `--ignored` in CI where Docker is available
async fn test_mcp_endpoints_require_auth_and_accept_valid_key() -> Result<()> {
    // 1) Start ephemeral Postgres via testcontainers
    // Start Postgres container
    let pg = Postgres::default()
        .start()
        .await
        .expect("failed to start postgres");
    let mapped = pg
        .get_host_port_ipv4(5432)
        .await
        .expect("failed to map postgres port");
    let db_url = format!("postgresql://postgres:postgres@127.0.0.1:{mapped}/postgres");

    // 2) Prepare minimal schema required by ApiKeyService
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Core tables used by validation and usage recording
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kotadb_api_keys (
            id BIGSERIAL PRIMARY KEY,
            key_hash VARCHAR(64) NOT NULL UNIQUE,
            user_email VARCHAR(255) NOT NULL,
            user_id VARCHAR(255),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_used_at TIMESTAMPTZ,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            rate_limit INTEGER NOT NULL DEFAULT 60,
            monthly_quota BIGINT NOT NULL DEFAULT 1000000,
            monthly_usage BIGINT NOT NULL DEFAULT 0,
            total_usage BIGINT NOT NULL DEFAULT 0,
            expires_at TIMESTAMPTZ,
            description TEXT,
            allowed_ips JSONB
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kotadb_api_key_usage (
            id BIGSERIAL PRIMARY KEY,
            key_id BIGINT NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            endpoint VARCHAR(255) NOT NULL,
            method VARCHAR(10) NOT NULL,
            status_code INTEGER,
            response_time_ms INTEGER,
            ip_address INET,
            user_agent TEXT
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_key_rate_limits (
            key_id BIGINT NOT NULL,
            window_start TIMESTAMPTZ NOT NULL,
            request_count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (key_id, window_start)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // 3) Insert a valid API key directly (hashing matches service behavior)
    let plaintext_key = "kdb_live_test_integration";
    let mut hasher = Sha256::new();
    hasher.update(plaintext_key);
    let key_hash = format!("{:x}", hasher.finalize());

    sqlx::query(
        r#"
        INSERT INTO kotadb_api_keys (key_hash, user_email, is_active, rate_limit, monthly_quota)
        VALUES ($1, $2, TRUE, 100, 1000000)
        "#,
    )
    .bind(&key_hash)
    .bind("tester@example.com")
    .execute(&pool)
    .await?;

    // 4) Build router: MCP bridge + auth middleware with real ApiKeyService
    let api_key_config = ApiKeyConfig {
        database_url: db_url.clone(),
        max_connections: 5,
        connect_timeout_seconds: 5,
        default_rate_limit: 100,
        default_monthly_quota: 1_000_000,
    };
    let api_key_service = Arc::new(ApiKeyService::new(api_key_config).await?);

    // Use MCP bridge router; no need for full server
    let state = {
        #[cfg(feature = "mcp-server")]
        {
            use kotadb::mcp::tools::MCPToolRegistry;
            kotadb::mcp_http_bridge::McpHttpBridgeState::new(Some(std::sync::Arc::new(
                MCPToolRegistry::new(),
            )))
        }
        #[cfg(not(feature = "mcp-server"))]
        {
            kotadb::mcp_http_bridge::McpHttpBridgeState::new()
        }
    };

    let app = kotadb::mcp_http_bridge::create_mcp_bridge_router()
        .layer(middleware::from_fn_with_state(
            api_key_service.clone(),
            auth_middleware,
        ))
        .route("/health", get(|| async { "ok" }))
        .with_state(state);

    // Serve with ConnectInfo so middleware can extract client IP
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // 5) Missing API key → 401
    let resp = client
        .get(format!("http://127.0.0.1:{}/mcp/tools", addr.port()))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 6) Invalid API key → 401
    let resp = client
        .get(format!("http://127.0.0.1:{}/mcp/tools", addr.port()))
        .header("Authorization", "Bearer kdb_live_invalid")
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 7) Valid API key → 200
    let resp = client
        .get(format!("http://127.0.0.1:{}/mcp/tools", addr.port()))
        .header("Authorization", format!("Bearer {}", plaintext_key))
        .send()
        .await?;
    if resp.status() != reqwest::StatusCode::OK {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        panic!("Expected 200, got {} body: {}", status, body);
    }

    server.abort();

    Ok(())
}
