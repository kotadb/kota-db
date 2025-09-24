#![cfg(feature = "docker-tests")]

//! Integration smoke test for the Supabase-backed repository store + job lifecycle.

use anyhow::Result;
use kotadb::supabase_repository::{RepositoryRegistration, SupabaseRepositoryStore};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires Docker; run via `cargo test --features docker-tests -- --ignored`
async fn supabase_repository_store_happy_path() -> Result<()> {
    let pg = Postgres::default()
        .start()
        .await
        .expect("failed to start postgres");
    let mapped_port = pg
        .get_host_port_ipv4(5432)
        .await
        .expect("failed to map postgres port");
    let db_url = format!("postgresql://postgres:postgres@127.0.0.1:{mapped_port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    bootstrap_schema(&pool).await?;

    let store = SupabaseRepositoryStore::new(pool.clone());
    let user_id = Uuid::new_v4();

    let registration = RepositoryRegistration {
        user_id,
        api_key_id: None,
        name: "kota-db",
        git_url: "https://github.com/kotadb/kota-db.git",
        provider: "github",
        default_branch: Some("main"),
        settings: &json!({"branch": "main"}),
        job_payload: &json!({
            "git_url": "https://github.com/kotadb/kota-db.git",
            "provider": "github",
            "branch": "main",
            "requested_at": chrono::Utc::now(),
        }),
    };

    let (repository, job_id, webhook_secret) = store
        .register_repository_and_enqueue_job(registration)
        .await?;

    assert_eq!(repository.name, "kota-db");
    assert_eq!(repository.status, "queued");
    assert!(webhook_secret.is_some());

    // First poll should grab the queued job and move it in-flight
    let job = store
        .fetch_job_for_worker()
        .await?
        .expect("job should be available");
    assert_eq!(job.id, job_id);
    assert_eq!(job.job_type, "full_index");

    // Age the job artificially so recovery path re-queues it
    sqlx::query(
        "UPDATE indexing_jobs SET started_at = NOW() - INTERVAL '20 minutes' WHERE id = $1",
    )
    .bind(job_id)
    .execute(&pool)
    .await?;

    let recovered = store.recover_stale_jobs(Duration::from_secs(60)).await?;
    assert_eq!(recovered.len(), 1, "stale job should have been re-queued");

    // Grab the job again after recovery
    let job = store
        .fetch_job_for_worker()
        .await?
        .expect("job should be available after recovery");
    assert_eq!(job.attempt, 2);

    store
        .record_job_event(job.id, "started", "Job picked up", None)
        .await?;

    // Simulate a successful run
    store
        .complete_job(job.id, Some(json!({ "files_processed": 123 })))
        .await?;

    // Ensure repository list and job status reflect completion
    let repositories = store.list_repositories(user_id).await?;
    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].status, "ready");

    let status = store
        .job_status(job.id, user_id)
        .await?
        .expect("job status should exist");
    assert_eq!(status.status, "completed");

    // Allow async clean-up before container teardown
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

async fn bootstrap_schema(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto;")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_updated_at_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ language 'plpgsql';
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id UUID PRIMARY KEY,
            user_id UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS repositories (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            api_key_id UUID,
            name TEXT NOT NULL,
            git_url TEXT NOT NULL,
            provider TEXT NOT NULL,
            status TEXT NOT NULL,
            sync_state TEXT NOT NULL,
            default_branch TEXT,
            last_indexed_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            webhook_secret_hash TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS indexing_jobs (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            requested_by UUID,
            job_type TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            attempt INTEGER NOT NULL DEFAULT 0,
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            result JSONB,
            error_message TEXT,
            queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            finished_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS indexing_job_events (
            id BIGSERIAL PRIMARY KEY,
            job_id UUID NOT NULL REFERENCES indexing_jobs(id) ON DELETE CASCADE,
            event_type TEXT NOT NULL,
            message TEXT,
            context JSONB,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS webhook_deliveries (
            id BIGSERIAL PRIMARY KEY,
            repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            provider TEXT NOT NULL,
            delivery_id TEXT,
            event_type TEXT,
            status TEXT NOT NULL,
            received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            processed_at TIMESTAMPTZ,
            payload JSONB,
            headers JSONB,
            signature TEXT,
            error_message TEXT,
            job_id UUID REFERENCES indexing_jobs(id) ON DELETE SET NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_indexing_jobs_repository_id ON indexing_jobs(repository_id);",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_repo ON webhook_deliveries(repository_id);",
    )
    .execute(pool)
    .await?;

    Ok(())
}
