use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{types::Json, PgPool};
use tracing::instrument;
use uuid::Uuid;

pub mod job_worker;
pub mod task;

/// Row representing a repository in Supabase.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RepositoryRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub git_url: String,
    pub provider: String,
    pub status: String,
    pub sync_state: String,
    pub default_branch: Option<String>,
    pub last_indexed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Row representing an indexing job with repository metadata.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobStatusRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub status: String,
    pub job_type: String,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub git_url: String,
    pub name: String,
}

#[derive(Clone)]
pub struct SupabaseRepositoryStore {
    pool: PgPool,
}

pub struct RepositoryRegistration<'a> {
    pub user_id: Uuid,
    pub name: &'a str,
    pub git_url: &'a str,
    pub provider: &'a str,
    pub default_branch: Option<&'a str>,
    pub settings: &'a JsonValue,
    pub job_payload: &'a JsonValue,
}

impl SupabaseRepositoryStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    #[instrument(skip(self, registration))]
    pub async fn register_repository_and_enqueue_job(
        &self,
        registration: RepositoryRegistration<'_>,
    ) -> Result<(RepositoryRow, Uuid)> {
        let mut tx = self.pool.begin().await?;

        let repository = sqlx::query_as::<_, RepositoryRow>(
            r#"
            INSERT INTO repositories (
                user_id,
                name,
                git_url,
                provider,
                default_branch,
                status,
                sync_state,
                settings
            )
            VALUES ($1, $2, $3, $4, $5, 'queued', 'pending', $6)
            ON CONFLICT (user_id, git_url)
            DO UPDATE
                SET updated_at = NOW(),
                    provider = EXCLUDED.provider,
                    default_branch = COALESCE(EXCLUDED.default_branch, repositories.default_branch),
                    settings = repositories.settings || EXCLUDED.settings,
                    status = 'queued',
                    sync_state = 'pending'
            RETURNING
                id,
                user_id,
                name,
                git_url,
                provider,
                status,
                sync_state,
                default_branch,
                last_indexed_at,
                created_at
            "#,
        )
        .bind(registration.user_id)
        .bind(registration.name)
        .bind(registration.git_url)
        .bind(registration.provider)
        .bind(registration.default_branch)
        .bind(Json(registration.settings.clone()))
        .fetch_one(&mut *tx)
        .await
        .context("failed to upsert repository record")?;

        let job_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO indexing_jobs (
                repository_id,
                job_type,
                payload,
                priority,
                status
            )
            VALUES ($1, $2, $3, $4, 'queued')
            RETURNING id
            "#,
        )
        .bind(repository.id)
        .bind("full_index")
        .bind(Json(registration.job_payload.clone()))
        .bind(0_i32)
        .fetch_one(&mut *tx)
        .await
        .context("failed to create indexing job")?;

        tx.commit().await?;
        Ok((repository, job_id))
    }

    #[instrument(skip(self))]
    pub async fn list_repositories(&self, user_id: Uuid) -> Result<Vec<RepositoryRow>> {
        let rows = sqlx::query_as::<_, RepositoryRow>(
            r#"
            SELECT
                id,
                user_id,
                name,
                git_url,
                provider,
                status,
                sync_state,
                default_branch,
                last_indexed_at,
                created_at
            FROM repositories
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list repositories")?;

        Ok(rows)
    }

    #[instrument(skip(self))]
    pub async fn job_status(&self, job_id: Uuid, user_id: Uuid) -> Result<Option<JobStatusRow>> {
        let row = sqlx::query_as::<_, JobStatusRow>(
            r#"
            SELECT
                j.id,
                j.repository_id,
                j.status,
                j.job_type,
                j.queued_at,
                j.started_at,
                j.finished_at,
                j.error_message,
                r.git_url,
                r.name
            FROM indexing_jobs j
            JOIN repositories r ON r.id = j.repository_id
            WHERE j.id = $1 AND r.user_id = $2
            "#,
        )
        .bind(job_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch job status")?;

        Ok(row)
    }

    #[instrument(skip(self))]
    pub async fn fetch_job_for_worker(&self) -> Result<Option<JobForWorker>> {
        let row = sqlx::query_as::<_, JobForWorker>(
            r#"
            UPDATE indexing_jobs
            SET status = 'in_progress',
                started_at = NOW(),
                attempt = attempt + 1,
                updated_at = NOW()
            WHERE id = (
                SELECT id
                FROM indexing_jobs
                WHERE status = 'queued'
                ORDER BY priority DESC, queued_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING
                id,
                repository_id,
                job_type,
                payload,
                attempt
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch job for worker")?;

        Ok(row)
    }

    #[instrument(skip(self, result))]
    pub async fn complete_job(&self, job_id: Uuid, result: Option<JsonValue>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexing_jobs
            SET status = 'completed',
                finished_at = NOW(),
                result = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(result.map(Json))
        .execute(&self.pool)
        .await
        .context("failed to mark job completed")?;

        Ok(())
    }

    #[instrument(skip(self, error_message))]
    pub async fn fail_job(&self, job_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexing_jobs
            SET status = 'failed',
                error_message = $2,
                finished_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .context("failed to mark job failed")?;

        Ok(())
    }

    #[instrument(skip(self, metadata))]
    pub async fn update_repository_metadata(
        &self,
        repository_id: Uuid,
        last_indexed_at: Option<DateTime<Utc>>,
        metadata: JsonValue,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE repositories
            SET last_indexed_at = COALESCE($2, last_indexed_at),
                metadata = metadata || $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(repository_id)
        .bind(last_indexed_at)
        .bind(Json(metadata))
        .execute(&self.pool)
        .await
        .context("failed to update repository metadata")?;

        Ok(())
    }

    #[instrument(skip(self, event_message, context))]
    pub async fn record_job_event(
        &self,
        job_id: Uuid,
        event_type: &str,
        event_message: &str,
        context: Option<JsonValue>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO indexing_job_events (job_id, event_type, message, context)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(job_id)
        .bind(event_type)
        .bind(event_message)
        .bind(context.map(Json))
        .execute(&self.pool)
        .await
        .context("failed to record job event")?;

        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobForWorker {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub job_type: String,
    pub payload: JsonValue,
    pub attempt: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RepositoryMetaRow {
    pub id: Uuid,
    pub git_url: String,
    pub provider: String,
    pub default_branch: Option<String>,
    pub settings: JsonValue,
}

impl SupabaseRepositoryStore {
    #[instrument(skip(self))]
    pub async fn fetch_repository(&self, repository_id: Uuid) -> Result<Option<RepositoryMetaRow>> {
        let row = sqlx::query_as::<_, RepositoryMetaRow>(
            r#"
            SELECT
                id,
                git_url,
                provider,
                default_branch,
                settings
            FROM repositories
            WHERE id = $1
            "#,
        )
        .bind(repository_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch repository metadata")?;

        Ok(row)
    }
}
