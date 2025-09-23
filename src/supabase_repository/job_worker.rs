use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
#[cfg(feature = "git-integration")]
use git2::build::RepoBuilder;
use serde_json::{json, Value as JsonValue};
#[cfg(feature = "git-integration")]
use tokio::task;
use tokio::time::sleep;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::services::{DatabaseAccess, IndexCodebaseOptions, IndexingService};

use super::{
    task::{merge_settings, option_bool, option_usize, SupabaseJobPayload},
    JobForWorker, SupabaseRepositoryStore,
};

#[derive(Clone)]
pub struct SupabaseJobWorker<D>
where
    D: DatabaseAccess + Send + Sync + 'static,
{
    pub store: SupabaseRepositoryStore,
    pub database: Arc<D>,
    pub db_path: PathBuf,
    pub poll_interval: Duration,
}

impl<D> SupabaseJobWorker<D>
where
    D: DatabaseAccess + Send + Sync + 'static,
{
    pub fn new(store: SupabaseRepositoryStore, database: Arc<D>, db_path: PathBuf) -> Self {
        Self {
            store,
            database,
            db_path,
            poll_interval: Duration::from_secs(5),
        }
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    #[instrument(skip_all)]
    pub async fn run(&self) -> Result<()> {
        loop {
            match self.tick().await {
                Ok(should_continue) => {
                    if !should_continue {
                        sleep(self.poll_interval).await;
                    }
                }
                Err(e) => {
                    error!("Worker tick failed: {}", e);
                    sleep(self.poll_interval).await;
                }
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn tick(&self) -> Result<bool> {
        let Some(job) = self.store.fetch_job_for_worker().await? else {
            return Ok(false);
        };

        let job_id = job.id;
        self.store
            .record_job_event(job_id, "started", "Job picked up by worker", None)
            .await?;

        match self.process_job(job).await {
            Ok(result) => {
                self.store.complete_job(job_id, result).await?;
                self.store
                    .record_job_event(job_id, "completed", "Job completed", None)
                    .await?;
                Ok(true)
            }
            Err(e) => {
                error!("Job {} failed: {}", job_id, e);
                self.store
                    .fail_job(job_id, &e.to_string())
                    .await
                    .context("failed to mark job failed")?;
                self.store
                    .record_job_event(
                        job_id,
                        "failed",
                        "Job failed",
                        Some(json!({ "error": e.to_string() })),
                    )
                    .await?;
                Ok(true)
            }
        }
    }

    async fn process_job(&self, job: JobForWorker) -> Result<Option<JsonValue>> {
        match job.job_type.as_str() {
            "full_index" => self.process_full_index(job).await,
            other => {
                self.store
                    .record_job_event(
                        job.id,
                        "skipped",
                        "Unsupported job type",
                        Some(json!({ "job_type": other })),
                    )
                    .await?;
                Ok(None)
            }
        }
    }

    async fn process_full_index(&self, job: JobForWorker) -> Result<Option<JsonValue>> {
        let mut payload = SupabaseJobPayload::parse(job.payload.clone());

        let repo_meta = self
            .store
            .fetch_repository(job.repository_id)
            .await?
            .ok_or_else(|| anyhow!("repository metadata missing for job"))?;

        if payload.git_url.is_empty() {
            payload.git_url = repo_meta.git_url.clone();
        }
        if payload.git_url.is_empty() {
            return Err(anyhow!("job payload missing git_url"));
        }

        payload.provider = payload.provider.or(Some(repo_meta.provider.clone()));
        payload.branch = payload.branch.or(repo_meta.default_branch.clone());

        let merged_settings = merge_settings(&repo_meta.settings, payload.settings.as_ref());
        payload.settings = Some(merged_settings);

        info!(
            repository = %repo_meta.git_url,
            job_id = %job.id,
            "Processing Supabase indexing job"
        );

        let repo_path = self.prepare_repository(job.repository_id, &payload).await?;
        let index_result = self.index_repository(&repo_path, &payload).await?;

        self.store
            .update_repository_metadata(
                job.repository_id,
                Some(Utc::now()),
                json!({ "last_success": Utc::now().to_rfc3339() }),
            )
            .await?;

        Ok(Some(json!({
            "files_processed": index_result.files_processed,
            "symbols_extracted": index_result.symbols_extracted,
            "relationships_found": index_result.relationships_found,
        })))
    }

    async fn prepare_repository(
        &self,
        repository_id: Uuid,
        payload: &SupabaseJobPayload,
    ) -> Result<PathBuf> {
        #[cfg(not(feature = "git-integration"))]
        {
            let _ = repository_id;
            let _ = payload;
            return Err(anyhow!(
                "git-integration feature is required for SaaS ingestion worker"
            ));
        }

        #[cfg(feature = "git-integration")]
        {
            let repo_dir = self.db_path.join("repos").join(repository_id.to_string());
            if repo_dir.exists() {
                tokio::fs::remove_dir_all(&repo_dir).await.ok();
            }
            tokio::fs::create_dir_all(&repo_dir).await?;

            let git_url = payload.git_url.clone();
            let branch = payload.branch.clone();
            let repo_dir_clone = repo_dir.clone();

            task::spawn_blocking(move || -> Result<()> {
                let mut builder = RepoBuilder::new();
                if let Some(branch) = branch.as_deref() {
                    builder.branch(branch);
                }

                builder
                    .clone(&git_url, repo_dir_clone.as_path())
                    .with_context(|| format!("Failed to clone repository: {}", git_url))?;
                Ok(())
            })
            .await??;

            Ok(repo_dir)
        }
    }

    async fn index_repository(
        &self,
        repo_path: &Path,
        payload: &SupabaseJobPayload,
    ) -> Result<crate::services::IndexResult> {
        let indexing = IndexingService::new(self.database.as_ref(), self.db_path.clone());
        let mut options = IndexCodebaseOptions {
            repo_path: repo_path.to_path_buf(),
            ..IndexCodebaseOptions::default()
        };

        if let Some(settings) = &payload.settings {
            if let Some(value) = option_bool(settings, "include_files") {
                options.include_files = value;
            }
            if let Some(value) = option_bool(settings, "include_commits") {
                options.include_commits = value;
            }
            if let Some(value) = option_usize(settings, "max_file_size_mb") {
                options.max_file_size_mb = value;
            }
            if let Some(value) = option_usize(settings, "max_parallel_files") {
                options.max_parallel_files = Some(value);
            }
            if let Some(value) = option_bool(settings, "enable_chunking") {
                options.enable_chunking = value;
            }
            if let Some(value) = option_bool(settings, "extract_symbols") {
                options.extract_symbols = Some(value);
            }
        }

        indexing.index_codebase(options).await
    }
}
