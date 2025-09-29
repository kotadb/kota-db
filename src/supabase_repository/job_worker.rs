use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
#[cfg(feature = "git-integration")]
use git2::build::RepoBuilder;
use serde_json::{json, Value as JsonValue};
#[cfg(feature = "git-integration")]
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, MissedTickBehavior};
use tracing::{debug, error, info, instrument, warn};
use url::Url;
use uuid::Uuid;

use crate::services::{DatabaseAccess, IndexCodebaseOptions, IndexResult, IndexingService};

use super::{
    task::{merge_settings, option_bool, option_usize, SupabaseJobPayload},
    JobForWorker, RepositoryMetaRow, SupabaseRepositoryStore,
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

// Jobs are considered stale after 45 minutes of inactivity. A periodic heartbeat keeps
// genuinely long-running indexing work alive without extending this window.
const STALE_JOB_MAX_AGE: Duration = Duration::from_secs(45 * 60);

fn delivery_id_from_payload(payload: &JsonValue) -> Option<i64> {
    match payload.get("webhook_delivery_id") {
        Some(JsonValue::Number(num)) => num.as_i64(),
        Some(JsonValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

#[derive(Debug, Default, Clone)]
struct IncrementalWork {
    paths_to_index: Vec<String>,
    paths_to_remove: Vec<String>,
    skip_index_document: bool,
}

struct JobHeartbeat {
    handle: JoinHandle<()>,
}

impl JobHeartbeat {
    fn start(store: SupabaseRepositoryStore, job_id: Uuid) -> Self {
        let mut ticker = interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let handle = tokio::spawn(async move {
            loop {
                ticker.tick().await;
                match store.heartbeat_job(job_id).await {
                    Ok(_) => {
                        debug!(job_id = %job_id, "Job heartbeat ticked");
                    }
                    Err(err) => {
                        warn!(job_id = %job_id, "Job heartbeat failed: {}", err);
                        break;
                    }
                }
            }
        });

        Self { handle }
    }
}

impl Drop for JobHeartbeat {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn infer_repository_identifier(git_url: &str) -> String {
    if let Ok(parsed) = Url::parse(git_url) {
        if let Some(segment) = parsed
            .path()
            .trim_matches('/')
            .split('/')
            .filter(|seg| !seg.is_empty())
            .next_back()
        {
            return segment.trim_end_matches(".git").to_string();
        }
    }

    if let Some(stripped) = git_url.strip_prefix("git@") {
        if let Some((_, path)) = stripped.split_once(':') {
            if let Some(segment) = path
                .trim_matches('/')
                .split('/')
                .filter(|seg| !seg.is_empty())
                .next_back()
            {
                return segment.trim_end_matches(".git").to_string();
            }
        }
    }

    "repository".to_string()
}

fn sanitize_repository_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase();

    if sanitized.is_empty() {
        "repository".to_string()
    } else {
        sanitized
    }
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
        let recovered = self.store.recover_stale_jobs(STALE_JOB_MAX_AGE).await?;
        if !recovered.is_empty() {
            warn!(
                count = recovered.len(),
                "Recovered stale Supabase jobs before polling"
            );
        }

        let Some(job) = self.store.fetch_job_for_worker().await? else {
            return Ok(false);
        };

        let job_id = job.id;
        let job_clone = job.clone();
        self.store
            .record_job_event(job_id, "started", "Job picked up by worker", None)
            .await?;

        match self.process_job(job).await {
            Ok((result, webhook_delivery_id)) => {
                self.store.complete_job(job_id, result).await?;
                self.store
                    .record_job_event(job_id, "completed", "Job completed", None)
                    .await?;
                if let Some(delivery_id) = webhook_delivery_id {
                    if let Err(err) = self
                        .store
                        .update_webhook_delivery_status(delivery_id, "processed", true, None, None)
                        .await
                    {
                        error!(
                            "Failed to update webhook delivery {} for job {}: {}",
                            delivery_id, job_id, err
                        );
                    }
                }
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
                if let Some(delivery_id) = delivery_id_from_payload(&job_clone.payload) {
                    if let Err(err) = self
                        .store
                        .update_webhook_delivery_status(
                            delivery_id,
                            "failed",
                            true,
                            Some(&e.to_string()),
                            None,
                        )
                        .await
                    {
                        error!(
                            "Failed to mark webhook delivery {} as failed for job {}: {}",
                            delivery_id, job_id, err
                        );
                    }
                }
                Ok(true)
            }
        }
    }

    async fn process_job(&self, job: JobForWorker) -> Result<(Option<JsonValue>, Option<i64>)> {
        match job.job_type.as_str() {
            "webhook_update" => {
                let payload = SupabaseJobPayload::parse(job.payload.clone());
                let plan = self.prepare_incremental_work(&payload);
                self.process_indexing_job(job, payload, Some(plan)).await
            }
            "full_index" | "incremental_update" => {
                let payload = SupabaseJobPayload::parse(job.payload.clone());
                self.process_indexing_job(job, payload, None).await
            }
            other => {
                self.store
                    .record_job_event(
                        job.id,
                        "skipped",
                        "Unsupported job type",
                        Some(json!({ "job_type": other })),
                    )
                    .await?;
                Ok((None, None))
            }
        }
    }

    fn prepare_incremental_work(&self, payload: &SupabaseJobPayload) -> IncrementalWork {
        let mut work = IncrementalWork {
            skip_index_document: true,
            ..Default::default()
        };

        if let Some(changes) = payload.changes.as_ref() {
            let mut to_index = HashSet::new();
            let mut to_remove = HashSet::new();

            if let Some(added) = changes.get("added").and_then(|v| v.as_array()) {
                for value in added.iter().filter_map(|v| v.as_str()) {
                    to_index.insert(value.trim_start_matches("./").to_string());
                }
            }

            if let Some(modified) = changes.get("modified").and_then(|v| v.as_array()) {
                for value in modified.iter().filter_map(|v| v.as_str()) {
                    to_index.insert(value.trim_start_matches("./").to_string());
                }
            }

            if let Some(removed) = changes.get("removed").and_then(|v| v.as_array()) {
                for value in removed.iter().filter_map(|v| v.as_str()) {
                    to_remove.insert(value.trim_start_matches("./").to_string());
                }
            }

            work.paths_to_index = to_index.into_iter().collect();
            work.paths_to_remove = to_remove.into_iter().collect();
        }

        work
    }

    async fn process_indexing_job(
        &self,
        job: JobForWorker,
        mut payload: SupabaseJobPayload,
        incremental: Option<IncrementalWork>,
    ) -> Result<(Option<JsonValue>, Option<i64>)> {
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

        if let Some(delivery_id) = payload.webhook_delivery_id {
            if let Err(err) = self
                .store
                .update_webhook_delivery_status(delivery_id, "processing", false, None, None)
                .await
            {
                error!(
                    "Failed to mark webhook delivery {} as processing for job {}: {}",
                    delivery_id, job.id, err
                );
            }
        }

        let event_type = payload.event_type.clone();
        info!(
            repository = %repo_meta.git_url,
            job_id = %job.id,
            job_type = %job.job_type,
            event = event_type.as_deref().unwrap_or("unknown"),
            "Processing Supabase indexing job"
        );

        // Keep the job marked as active while long-running phases execute.
        let _heartbeat_guard = JobHeartbeat::start(self.store.clone(), job.id);

        let repo_identifier = infer_repository_identifier(&payload.git_url);
        let safe_repo_name = sanitize_repository_name(&repo_identifier);

        let include_paths = incremental
            .as_ref()
            .map(|work| work.paths_to_index.clone())
            .filter(|paths| !paths.is_empty());
        let removed_paths = incremental
            .as_ref()
            .map(|work| work.paths_to_remove.clone())
            .unwrap_or_default();
        let skip_index_document = incremental
            .as_ref()
            .map(|work| work.skip_index_document)
            .unwrap_or(false);

        if !removed_paths.is_empty() {
            self.purge_removed_paths(&repo_meta, &payload, &safe_repo_name, &removed_paths)
                .await?;
        }

        let should_clone_repo = include_paths
            .as_ref()
            .map(|paths| !paths.is_empty())
            .unwrap_or(true);

        if !should_clone_repo && removed_paths.is_empty() {
            self.store
                .record_job_event(
                    job.id,
                    "no_changes",
                    "Webhook delivered no actionable changes",
                    None,
                )
                .await
                .ok();
        }

        let index_result = if should_clone_repo {
            warn!(
                repository = %repo_meta.git_url,
                branch = payload.branch.as_deref().unwrap_or("unknown"),
                include_paths = include_paths.as_ref().map(|paths| paths.len()),
                "Preparing repository workspace for indexing"
            );
            if let Err(err) = self
                .store
                .record_job_event(
                    job.id,
                    "cloning",
                    "Preparing repository workspace",
                    Some(json!({
                        "branch": payload.branch.clone(),
                        "include_paths": include_paths.as_ref().map(|paths| paths.len()),
                    })),
                )
                .await
            {
                warn!("Failed to record cloning event for job {}: {}", job.id, err);
            }
            let repo_path = self.prepare_repository(job.repository_id, &payload).await?;
            warn!(
                repository = %repo_meta.git_url,
                path = %repo_path.display(),
                "Repository workspace ready"
            );
            if let Err(err) = self
                .store
                .record_job_event(
                    job.id,
                    "clone_completed",
                    "Repository clone completed",
                    Some(json!({ "path": repo_path.display().to_string() })),
                )
                .await
            {
                warn!(
                    "Failed to record clone_completed event for job {}: {}",
                    job.id, err
                );
            }
            if let Err(err) = self
                .store
                .record_job_event(
                    job.id,
                    "indexing",
                    "Starting repository indexing",
                    Some(json!({
                        "create_index_doc": !skip_index_document,
                        "include_paths": include_paths.as_ref().map(|paths| paths.len()),
                    })),
                )
                .await
            {
                warn!(
                    "Failed to record indexing event for job {}: {}",
                    job.id, err
                );
            }
            warn!(
                repository = %repo_meta.git_url,
                path = %repo_path.display(),
                include_paths = include_paths.as_ref().map(|paths| paths.len()),
                "Starting IndexingService::index_codebase"
            );
            let index_start = Instant::now();
            let result = self
                .index_repository(
                    &repo_path,
                    &payload,
                    include_paths.as_deref(),
                    !skip_index_document,
                )
                .await?;
            warn!(
                repository = %repo_meta.git_url,
                elapsed_ms = index_start.elapsed().as_millis() as u64,
                "Completed IndexingService::index_codebase invocation"
            );

            if result.files_processed == 0 {
                warn!(
                    repository = %repo_meta.git_url,
                    branch = payload.branch.as_deref().unwrap_or("unknown"),
                    "Indexing completed without processing any files"
                );
            }

            info!(
                repository = %repo_meta.git_url,
                files_processed = result.files_processed,
                symbols_extracted = result.symbols_extracted,
                relationships_found = result.relationships_found,
                elapsed_ms = index_start.elapsed().as_millis() as u64,
                "Indexing finished"
            );
            if let Err(err) = self
                .store
                .record_job_event(
                    job.id,
                    "indexing_completed",
                    "Repository indexing completed",
                    Some(json!({
                        "files_processed": result.files_processed,
                        "symbols_extracted": result.symbols_extracted,
                        "relationships_found": result.relationships_found,
                        "elapsed_ms": index_start.elapsed().as_millis() as u64,
                    })),
                )
                .await
            {
                warn!(
                    "Failed to record indexing_completed event for job {}: {}",
                    job.id, err
                );
            }
            result
        } else {
            IndexResult {
                files_processed: 0,
                symbols_extracted: 0,
                relationships_found: 0,
                total_time_ms: 0,
                success: true,
                formatted_output: String::from("No files to index"),
                errors: Vec::new(),
            }
        };

        let now = Utc::now();
        self.store
            .update_repository_metadata(
                job.repository_id,
                Some(now),
                json!({
                    "jobs": {
                        "last": {
                            "job_id": job.id,
                            "job_type": job.job_type,
                            "completed_at": now.to_rfc3339(),
                            "event_type": event_type,
                        }
                    }
                }),
            )
            .await?;

        Ok((
            Some(json!({
                "files_processed": index_result.files_processed,
                "symbols_extracted": index_result.symbols_extracted,
                "relationships_found": index_result.relationships_found,
                "files_deleted": removed_paths.len(),
            })),
            payload.webhook_delivery_id,
        ))
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
                warn!(
                    repo = %payload.git_url,
                    path = %repo_dir.display(),
                    "Removing existing repository workspace"
                );
                tokio::fs::remove_dir_all(&repo_dir).await.ok();
            }
            tokio::fs::create_dir_all(&repo_dir).await?;
            warn!(
                repo = %payload.git_url,
                path = %repo_dir.display(),
                "Created repository workspace directory"
            );

            let git_url = payload.git_url.clone();
            let branch = payload.branch.clone();
            let repo_dir_clone = repo_dir.clone();

            task::spawn_blocking(move || -> Result<()> {
                warn!(
                    repo = %git_url,
                    path = %repo_dir_clone.display(),
                    branch = branch.as_deref().unwrap_or("default"),
                    "Cloning repository"
                );
                let mut builder = RepoBuilder::new();
                if let Some(branch) = branch.as_deref() {
                    builder.branch(branch);
                }

                builder
                    .clone(&git_url, repo_dir_clone.as_path())
                    .with_context(|| format!("Failed to clone repository: {}", git_url))?;
                warn!(
                    repo = %git_url,
                    path = %repo_dir_clone.display(),
                    "Repository clone finished"
                );
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
        include_paths: Option<&[String]>,
        create_index_doc: bool,
    ) -> Result<crate::services::IndexResult> {
        let indexing = IndexingService::new(self.database.as_ref(), self.db_path.clone());
        let mut options = IndexCodebaseOptions {
            repo_path: repo_path.to_path_buf(),
            ..IndexCodebaseOptions::default()
        };

        if let Some(paths) = include_paths {
            options.include_paths = Some(
                paths
                    .iter()
                    .map(|p| p.trim_start_matches("./").to_string())
                    .collect(),
            );
            options.create_index = create_index_doc;
        } else {
            options.create_index = create_index_doc;
        }

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
            if let Some(value) = option_usize(settings, "max_memory_mb") {
                options.max_memory_mb = Some(value as u64);
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

    async fn purge_removed_paths(
        &self,
        repo_meta: &RepositoryMetaRow,
        payload: &SupabaseJobPayload,
        safe_repo_name: &str,
        removed_paths: &[String],
    ) -> Result<()> {
        if removed_paths.is_empty() {
            return Ok(());
        }

        let prefix = payload
            .settings
            .as_ref()
            .and_then(|settings| settings.get("prefix"))
            .and_then(|value| value.as_str())
            .unwrap_or("repos");

        let prefix = prefix.trim_start_matches('/');
        let storage_arc = self.database.storage();
        let mut storage = storage_arc.lock().await;
        let primary_index_arc = self.database.primary_index();
        let mut primary_index = primary_index_arc.lock().await;
        let trigram_index_arc = self.database.trigram_index();
        let mut trigram_index = trigram_index_arc.lock().await;
        let path_cache = self.database.path_cache();
        let mut cache = path_cache.write().await;

        for rel_path in removed_paths {
            let document_path = format!(
                "{}/{}/files/{}",
                prefix,
                safe_repo_name,
                rel_path.trim_start_matches('/')
            );

            if let Some(doc_id) = cache.remove(&document_path) {
                if let Err(err) = storage.delete(&doc_id).await {
                    warn!(
                        "Failed to delete document {} for repository {}: {}",
                        document_path, repo_meta.git_url, err
                    );
                }

                if let Err(err) = primary_index.delete(&doc_id).await {
                    warn!(
                        "Failed to delete primary index entry {}: {}",
                        document_path, err
                    );
                }

                if let Err(err) = trigram_index.delete(&doc_id).await {
                    warn!(
                        "Failed to delete trigram index entry {}: {}",
                        document_path, err
                    );
                }
            } else {
                warn!(
                    "Document path {} not found during removal for repository {}",
                    document_path, repo_meta.git_url
                );
            }
        }

        storage.flush().await.ok();
        primary_index.flush().await.ok();
        trigram_index.flush().await.ok();

        Ok(())
    }
}
