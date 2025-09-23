use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD as BASE64_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use hex::encode;
use rand::{rngs::OsRng, RngCore};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use sqlx::{types::Json, PgPool};
use std::time::Duration;
use tracing::{instrument, warn};
use uuid::Uuid;

pub mod job_worker;
pub mod task;
use self::task::merge_settings;

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
    pub metadata: JsonValue,
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

/// Row representing a webhook delivery entry for deduplication and status tracking.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookDeliveryRow {
    pub id: i64,
    pub repository_id: Uuid,
    pub provider: String,
    pub delivery_id: Option<String>,
    pub event_type: Option<String>,
    pub status: String,
    pub job_id: Option<Uuid>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RecoveredJobRow {
    id: Uuid,
    repository_id: Uuid,
}

#[derive(Clone)]
pub struct SupabaseRepositoryStore {
    pool: PgPool,
}

pub struct RepositoryRegistration<'a> {
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
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
    ) -> Result<(RepositoryRow, Uuid, Option<String>)> {
        #[derive(sqlx::FromRow)]
        struct ExistingRepoRow {
            settings: JsonValue,
            metadata: JsonValue,
        }

        let mut tx = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, ExistingRepoRow>(
            r#"
            SELECT settings, metadata
            FROM repositories
            WHERE user_id = $1 AND git_url = $2
            FOR UPDATE
            "#,
        )
        .bind(registration.user_id)
        .bind(registration.git_url)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to fetch existing repository metadata")?;

        let (base_settings, mut metadata) = existing
            .map(|row| (row.settings, row.metadata))
            .unwrap_or_else(|| (json!({}), json!({})));
        let (secret, secret_hash, secret_created) =
            ensure_webhook_secret(&mut metadata, registration.provider);
        let merged_settings = merge_settings(&base_settings, Some(registration.settings));

        let repository = sqlx::query_as::<_, RepositoryRow>(
            r#"
            INSERT INTO repositories (
                user_id,
                api_key_id,
                name,
                git_url,
                provider,
                default_branch,
                status,
                sync_state,
                settings,
                metadata,
                webhook_secret_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'queued', 'pending', $7, $8, $9)
            ON CONFLICT (user_id, git_url)
            DO UPDATE
                SET updated_at = NOW(),
                    provider = EXCLUDED.provider,
                    default_branch = COALESCE(EXCLUDED.default_branch, repositories.default_branch),
                    api_key_id = COALESCE(EXCLUDED.api_key_id, repositories.api_key_id),
                    settings = EXCLUDED.settings,
                    metadata = repositories.metadata || EXCLUDED.metadata,
                    status = 'queued',
                    sync_state = 'pending',
                    webhook_secret_hash = COALESCE(repositories.webhook_secret_hash, EXCLUDED.webhook_secret_hash)
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
                created_at,
                metadata
            "#,
        )
        .bind(registration.user_id)
        .bind(registration.api_key_id)
        .bind(registration.name)
        .bind(registration.git_url)
        .bind(registration.provider)
        .bind(registration.default_branch)
        .bind(Json(merged_settings))
        .bind(Json(metadata.clone()))
        .bind(secret_hash)
        .fetch_one(&mut *tx)
        .await
        .context("failed to upsert repository record")?;

        let job_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO indexing_jobs (
                repository_id,
                requested_by,
                job_type,
                payload,
                priority,
                status
            )
            VALUES ($1, $2, $3, $4, $5, 'queued')
            RETURNING id
            "#,
        )
        .bind(repository.id)
        .bind(registration.api_key_id)
        .bind("full_index")
        .bind(Json(registration.job_payload.clone()))
        .bind(0_i32)
        .fetch_one(&mut *tx)
        .await
        .context("failed to create indexing job")?;

        tx.commit().await?;
        Ok((
            repository,
            job_id,
            if secret_created { Some(secret) } else { None },
        ))
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
                created_at,
                metadata
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

    /// Requeue jobs that were marked `in_progress` but never completed.
    #[instrument(skip(self))]
    pub async fn recover_stale_jobs(&self, max_age: Duration) -> Result<Vec<(Uuid, Uuid)>> {
        let max_age_secs = max_age.as_secs();
        if max_age_secs == 0 {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, RecoveredJobRow>(
            r#"
            WITH updated AS (
                UPDATE indexing_jobs
                SET status = 'queued',
                    started_at = NULL,
                    updated_at = NOW()
                WHERE status = 'in_progress'
                    AND started_at IS NOT NULL
                    AND started_at < NOW() - ($1 * INTERVAL '1 second')
                RETURNING id, repository_id
            )
            SELECT id, repository_id FROM updated
            "#,
        )
        .bind(max_age_secs as i64)
        .fetch_all(&self.pool)
        .await
        .context("failed to recover stale jobs")?;

        for row in &rows {
            if let Err(e) = self
                .update_repository_state(row.repository_id, "queued", "pending")
                .await
            {
                warn!(
                    "Failed to update repository {} state during job recovery: {}",
                    row.repository_id, e
                );
            }

            if let Err(e) = self
                .record_job_event(
                    row.id,
                    "requeued",
                    "Recovered stale in-progress job",
                    Some(json!({ "max_age_seconds": max_age_secs })),
                )
                .await
            {
                warn!("Failed to record recovery event for job {}: {}", row.id, e);
            }
        }

        Ok(rows
            .into_iter()
            .map(|row| (row.id, row.repository_id))
            .collect())
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

        if let Some(job) = &row {
            if let Err(e) = self
                .update_repository_state(job.repository_id, "syncing", "in_progress")
                .await
            {
                warn!(
                    "Failed to update repository {} state to syncing: {}",
                    job.repository_id, e
                );
            }
        }

        Ok(row)
    }

    #[instrument(skip(self, result))]
    pub async fn complete_job(&self, job_id: Uuid, result: Option<JsonValue>) -> Result<()> {
        let repository_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE indexing_jobs
            SET status = 'completed',
                finished_at = NOW(),
                result = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING repository_id
            "#,
        )
        .bind(job_id)
        .bind(result.map(Json))
        .fetch_one(&self.pool)
        .await
        .context("failed to mark job completed")?;

        self.update_repository_state(repository_id, "ready", "synced")
            .await?;

        Ok(())
    }

    #[instrument(skip(self, payload, headers, signature))]
    pub async fn refresh_webhook_delivery(
        &self,
        record_id: i64,
        payload: &JsonValue,
        headers: &JsonValue,
        signature: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET payload = $2,
                headers = $3,
                signature = $4,
                received_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(Json(payload.clone()))
        .bind(Json(headers.clone()))
        .bind(signature)
        .execute(&self.pool)
        .await
        .context("failed to refresh webhook delivery payload")?;

        Ok(())
    }

    #[instrument(skip(self, payload, headers, signature))]
    pub async fn reset_failed_webhook_delivery(
        &self,
        record_id: i64,
        event_type: &str,
        payload: &JsonValue,
        headers: &JsonValue,
        signature: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = 'received',
                event_type = $2,
                payload = $3,
                headers = $4,
                signature = $5,
                error_message = NULL,
                processed_at = NULL,
                received_at = NOW(),
                job_id = NULL
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(event_type)
        .bind(Json(payload.clone()))
        .bind(Json(headers.clone()))
        .bind(signature)
        .execute(&self.pool)
        .await
        .context("failed to reset webhook delivery")?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn find_webhook_delivery(
        &self,
        repository_id: Uuid,
        provider: &str,
        delivery_id: &str,
    ) -> Result<Option<WebhookDeliveryRow>> {
        let row = sqlx::query_as::<_, WebhookDeliveryRow>(
            r#"
            SELECT
                id,
                repository_id,
                provider,
                delivery_id,
                event_type,
                status,
                job_id,
                processed_at
            FROM webhook_deliveries
            WHERE repository_id = $1 AND provider = $2 AND delivery_id = $3
            "#,
        )
        .bind(repository_id)
        .bind(provider)
        .bind(delivery_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query webhook delivery")?;

        Ok(row)
    }

    #[instrument(skip(self, error_message))]
    pub async fn fail_job(&self, job_id: Uuid, error_message: &str) -> Result<()> {
        let repository_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE indexing_jobs
            SET status = 'failed',
                error_message = $2,
                finished_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING repository_id
            "#,
        )
        .bind(job_id)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await
        .context("failed to mark job failed")?;

        self.update_repository_state(repository_id, "error", "error")
            .await?;

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

    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self, payload, headers, signature, error_message))]
    pub async fn record_webhook_delivery(
        &self,
        repository_id: Uuid,
        provider: &str,
        delivery_id: Option<&str>,
        event_type: &str,
        status: &str,
        payload: &JsonValue,
        headers: &JsonValue,
        signature: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<i64> {
        let record_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO webhook_deliveries (
                repository_id,
                provider,
                delivery_id,
                event_type,
                status,
                payload,
                headers,
                signature,
                error_message,
                job_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)
            RETURNING id
            "#,
        )
        .bind(repository_id)
        .bind(provider)
        .bind(delivery_id)
        .bind(event_type)
        .bind(status)
        .bind(Json(payload.clone()))
        .bind(Json(headers.clone()))
        .bind(signature)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await
        .context("failed to record webhook delivery")?;

        Ok(record_id)
    }

    #[instrument(skip(self))]
    pub async fn update_webhook_delivery_status(
        &self,
        record_id: i64,
        status: &str,
        processed: bool,
        error_message: Option<&str>,
        job_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $2,
                processed_at = CASE WHEN $3 THEN NOW() ELSE processed_at END,
                error_message = $4,
                job_id = COALESCE($5, job_id)
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(status)
        .bind(processed)
        .bind(error_message)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .context("failed to update webhook delivery")?;

        Ok(())
    }

    #[instrument(skip(self, payload))]
    pub async fn enqueue_job(
        &self,
        repository_id: Uuid,
        requested_by: Option<Uuid>,
        job_type: &str,
        payload: &JsonValue,
        priority: i32,
    ) -> Result<Uuid> {
        let job_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO indexing_jobs (
                repository_id,
                requested_by,
                job_type,
                payload,
                priority,
                status
            )
            VALUES ($1, $2, $3, $4, $5, 'queued')
            RETURNING id
            "#,
        )
        .bind(repository_id)
        .bind(requested_by)
        .bind(job_type)
        .bind(Json(payload.clone()))
        .bind(priority)
        .fetch_one(&self.pool)
        .await
        .context("failed to enqueue job")?;

        self.update_repository_state(repository_id, "queued", "pending")
            .await?;

        Ok(job_id)
    }

    #[instrument(skip(self))]
    pub async fn lookup_primary_api_key(&self, user_id: Uuid) -> Result<Option<Uuid>> {
        let api_key_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to lookup Supabase API key")?;

        Ok(api_key_id)
    }

    async fn update_repository_state(
        &self,
        repository_id: Uuid,
        status: &str,
        sync_state: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE repositories
            SET status = $2,
                sync_state = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(repository_id)
        .bind(status)
        .bind(sync_state)
        .execute(&self.pool)
        .await
        .context("failed to update repository state")?;

        Ok(())
    }
}

fn ensure_webhook_secret(metadata: &mut JsonValue, provider: &str) -> (String, String, bool) {
    use serde_json::Map;

    if !metadata.is_object() {
        *metadata = json!({});
    }

    let obj = metadata
        .as_object_mut()
        .expect("metadata coerced to object");
    let webhook_entry = obj
        .entry("webhook".to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));

    if !webhook_entry.is_object() {
        *webhook_entry = JsonValue::Object(Map::new());
    }

    let webhook = webhook_entry
        .as_object_mut()
        .expect("webhook coerced to object");

    let mut created = false;
    let secret = webhook
        .get("secret")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            created = true;
            let secret = generate_webhook_secret();
            webhook.insert("secret".into(), JsonValue::String(secret.clone()));
            secret
        });

    webhook.insert("provider".into(), JsonValue::String(provider.to_string()));

    let hash = hash_secret(&secret);
    webhook.insert("secret_hash".into(), JsonValue::String(hash.clone()));

    (secret, hash, created)
}

fn generate_webhook_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    BASE64_NO_PAD.encode(bytes)
}

fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    encode(hasher.finalize())
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
    pub api_key_id: Option<Uuid>,
    pub metadata: JsonValue,
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
                settings,
                api_key_id,
                metadata
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
