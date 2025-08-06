// Wrapper Components - Stage 6: Component Library
// This module provides high-level wrappers that automatically apply best practices
// like tracing, validation, retries, and caching.

pub mod optimization;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::contracts::{Document, Index, Query, Storage};
use crate::observability::*;
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::validation::{self};

/// Storage wrapper that adds automatic tracing to all operations
pub struct TracedStorage<S: Storage> {
    inner: S,
    trace_id: Uuid,
    operation_count: Arc<Mutex<u64>>,
}

impl<S: Storage> TracedStorage<S> {
    /// Wrap a storage implementation with tracing
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            trace_id: Uuid::new_v4(),
            operation_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the current trace ID
    pub fn trace_id(&self) -> Uuid {
        self.trace_id
    }

    /// Get the number of operations performed
    pub async fn operation_count(&self) -> u64 {
        *self.operation_count.lock().await
    }

    async fn increment_op_count(&self) {
        let mut count = self.operation_count.lock().await;
        *count += 1;
    }
}

#[async_trait]
impl<S: Storage> Storage for TracedStorage<S> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let trace_id = Uuid::new_v4();
        info!("[{}] Opening storage at: {}", trace_id, path);

        let start = Instant::now();
        let inner = S::open(path).await.context("Failed to open storage")?;

        let duration = start.elapsed();
        info!("[{}] Storage opened in {:?}", trace_id, duration);
        record_metric(MetricType::Histogram {
            name: "storage.open.duration",
            value: duration.as_millis() as f64,
            unit: "ms",
        });

        Ok(Self {
            inner,
            trace_id,
            operation_count: Arc::new(Mutex::new(0)),
        })
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        self.increment_op_count().await;

        with_trace_id("storage.insert", async {
            let start = Instant::now();
            info!(
                "[{}] Inserting document: {}",
                self.trace_id,
                doc.id.as_uuid()
            );

            let result = self.inner.insert(doc.clone()).await;

            let _duration = start.elapsed();
            let mut ctx = OperationContext::new("storage.insert");
            ctx.add_attribute("doc_id", doc.id.as_uuid().to_string());
            ctx.add_attribute("size", doc.size.to_string());

            log_operation(
                &ctx,
                &Operation::StorageWrite {
                    doc_id: doc.id.as_uuid(),
                    size_bytes: doc.size,
                },
                &result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|e| anyhow::anyhow!("{}", e)),
            );

            result
        })
        .await
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        with_trace_id("storage.get", async {
            let start = Instant::now();
            debug!("[{}] Getting document: {}", self.trace_id, id.as_uuid());

            let result = self.inner.get(id).await;

            let _duration = start.elapsed();
            let size = result
                .as_ref()
                .ok()
                .and_then(|opt| opt.as_ref())
                .map(|doc| doc.size)
                .unwrap_or(0);

            let mut ctx = OperationContext::new("storage.get");
            ctx.add_attribute("doc_id", id.as_uuid().to_string());
            ctx.add_attribute(
                "found",
                result
                    .as_ref()
                    .map(|opt| opt.is_some().to_string())
                    .unwrap_or_else(|_| "error".to_string()),
            );

            log_operation(
                &ctx,
                &Operation::StorageRead {
                    doc_id: id.as_uuid(),
                    size_bytes: size,
                },
                &result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|e| anyhow::anyhow!("{}", e)),
            );

            result
        })
        .await
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        self.increment_op_count().await;

        with_trace_id("storage.update", async {
            info!(
                "[{}] Updating document: {}",
                self.trace_id,
                doc.id.as_uuid()
            );
            self.inner.update(doc.clone()).await
        })
        .await
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        self.increment_op_count().await;

        with_trace_id("storage.delete", async {
            info!("[{}] Deleting document: {}", self.trace_id, id.as_uuid());
            self.inner.delete(id).await
        })
        .await
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        with_trace_id("storage.list_all", async {
            info!("[{}] Listing all documents", self.trace_id);
            let start = Instant::now();

            let result = self.inner.list_all().await;

            let duration = start.elapsed();
            let count = result.as_ref().map(|docs| docs.len()).unwrap_or(0);

            record_metric(MetricType::Histogram {
                name: "storage.list_all.duration",
                value: duration.as_millis() as f64,
                unit: "ms",
            });

            record_metric(MetricType::Gauge {
                name: "storage.list_all.count",
                value: count as f64,
            });

            result
        })
        .await
    }

    async fn sync(&mut self) -> Result<()> {
        with_trace_id("storage.sync", async {
            info!("[{}] Syncing storage", self.trace_id);
            let start = Instant::now();

            let result = self.inner.sync().await;

            let duration = start.elapsed();
            record_metric(MetricType::Histogram {
                name: "storage.sync.duration",
                value: duration.as_millis() as f64,
                unit: "ms",
            });

            result
        })
        .await
    }

    async fn flush(&mut self) -> Result<()> {
        with_trace_id("storage.flush", async {
            info!("[{}] Flushing storage", self.trace_id);
            let start = Instant::now();

            let result = self.inner.flush().await;

            let duration = start.elapsed();
            record_metric(MetricType::Histogram {
                name: "storage.flush.duration",
                value: duration.as_millis() as f64,
                unit: "ms",
            });

            result
        })
        .await
    }

    async fn close(self) -> Result<()> {
        let op_count = self.operation_count().await;
        info!(
            "[{}] Closing storage after {} operations",
            self.trace_id, op_count
        );
        self.inner.close().await
    }
}

/// Storage wrapper that validates all inputs and outputs
pub struct ValidatedStorage<S: Storage> {
    inner: S,
    existing_ids: Arc<RwLock<std::collections::HashSet<ValidatedDocumentId>>>,
}

impl<S: Storage> ValidatedStorage<S> {
    /// Wrap a storage implementation with validation
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            existing_ids: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }
}

#[async_trait]
impl<S: Storage> Storage for ValidatedStorage<S> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Validate path before opening
        validation::path::validate_directory_path(path)?;

        let inner = S::open(path).await?;
        Ok(Self::new(inner))
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        // Validate document
        let existing = self.existing_ids.read().await;
        validation::document::validate_for_insert(&doc, &existing)?;
        drop(existing);

        // Perform insert
        self.inner.insert(doc.clone()).await?;

        // Update tracking
        self.existing_ids.write().await.insert(doc.id);

        Ok(())
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        let result = self.inner.get(id).await?;

        // Validate returned document if present
        if let Some(ref doc) = result {
            validation::document::validate_for_insert(doc, &std::collections::HashSet::new())?;
        }

        Ok(result)
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        // Get existing document for validation
        let existing = self
            .inner
            .get(&doc.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document not found for update"))?;

        // Validate update
        validation::document::validate_for_update(&doc, &existing)?;

        // Perform update
        self.inner.update(doc).await
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let deleted = self.inner.delete(id).await?;
        if deleted {
            self.existing_ids.write().await.remove(id);
        }
        Ok(deleted)
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        self.inner.list_all().await
    }

    async fn sync(&mut self) -> Result<()> {
        self.inner.sync().await
    }

    async fn flush(&mut self) -> Result<()> {
        self.inner.flush().await
    }

    async fn close(self) -> Result<()> {
        self.inner.close().await
    }
}

/// Storage wrapper that automatically retries failed operations
pub struct RetryableStorage<S: Storage> {
    inner: S,
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl<S: Storage> RetryableStorage<S> {
    /// Create a new retryable storage wrapper
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        }
    }

    /// Configure retry parameters
    pub fn with_retry_config(
        mut self,
        max_retries: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        self.max_retries = max_retries;
        self.base_delay = base_delay;
        self.max_delay = max_delay;
        self
    }
}

#[async_trait]
impl<S: Storage> Storage for RetryableStorage<S> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let inner = S::open(path).await?;
        Ok(Self::new(inner))
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.insert(doc.clone()).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Operation insert succeeded after {} attempts", attempt);
                    }
                    return Ok(());
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation insert failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation insert failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.get(id).await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Operation get succeeded after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation get failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation get failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.update(doc.clone()).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Operation update succeeded after {} attempts", attempt);
                    }
                    return Ok(());
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation update failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation update failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.delete(id).await {
                Ok(deleted) => {
                    if attempt > 1 {
                        info!("Operation delete succeeded after {} attempts", attempt);
                    }
                    return Ok(deleted);
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation delete failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation delete failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.list_all().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Operation list_all succeeded after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(e) if attempt >= self.max_retries => {
                    error!(
                        "Operation list_all failed after {} attempts: {}",
                        attempt, e
                    );
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation list_all failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn sync(&mut self) -> Result<()> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.sync().await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Operation sync succeeded after {} attempts", attempt);
                    }
                    return Ok(());
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation sync failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation sync failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn flush(&mut self) -> Result<()> {
        let mut attempt = 0;
        let mut delay = self.base_delay;

        loop {
            attempt += 1;

            match self.inner.flush().await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Operation flush succeeded after {} attempts", attempt);
                    }
                    return Ok(());
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation flush failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        "Operation flush failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }

    async fn close(self) -> Result<()> {
        self.inner.close().await
    }
}

/// Simple LRU cache for documents
struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    access_order: Vec<K>,
}

impl<K: Clone + Eq + std::hash::Hash, V> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            access_order: Vec::with_capacity(capacity),
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move to end (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            // Evict least recently used
            if let Some(lru_key) = self.access_order.first().cloned() {
                self.access_order.remove(0);
                self.map.remove(&lru_key);
            }
        }

        self.map.insert(key.clone(), value);
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key);
    }

    fn remove(&mut self, key: &K) {
        self.map.remove(key);
        self.access_order.retain(|k| k != key);
    }
}

/// Storage wrapper with built-in caching
pub struct CachedStorage<S: Storage> {
    inner: S,
    cache: Arc<Mutex<LruCache<Uuid, Document>>>,
    cache_hits: Arc<Mutex<u64>>,
    cache_misses: Arc<Mutex<u64>>,
}

impl<S: Storage> CachedStorage<S> {
    /// Create a cached storage with specified capacity
    pub fn new(inner: S, capacity: usize) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            cache_hits: Arc::new(Mutex::new(0)),
            cache_misses: Arc::new(Mutex::new(0)),
        }
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (u64, u64) {
        let hits = *self.cache_hits.lock().await;
        let misses = *self.cache_misses.lock().await;
        (hits, misses)
    }
}

#[async_trait]
impl<S: Storage> Storage for CachedStorage<S> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let inner = S::open(path).await?;
        Ok(Self::new(inner, 1000)) // Default 1000 document cache
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        self.inner.insert(doc.clone()).await?;

        // Update cache
        self.cache.lock().await.insert(doc.id.as_uuid(), doc);

        Ok(())
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        // Check cache first
        {
            let mut cache = self.cache.lock().await;
            if let Some(doc) = cache.get(&id.as_uuid()) {
                *self.cache_hits.lock().await += 1;
                return Ok(Some(doc.clone()));
            }
        }

        // Cache miss
        *self.cache_misses.lock().await += 1;

        // Fetch from storage
        let result = self.inner.get(id).await?;

        // Update cache if found
        if let Some(ref doc) = result {
            self.cache.lock().await.insert(id.as_uuid(), doc.clone());
        }

        Ok(result)
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        self.inner.update(doc.clone()).await?;

        // Update cache
        self.cache.lock().await.insert(doc.id.as_uuid(), doc);

        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let deleted = self.inner.delete(id).await?;

        // Remove from cache
        self.cache.lock().await.remove(&id.as_uuid());

        Ok(deleted)
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        // For cached storage, we still need to get all from the underlying storage
        // We could potentially cache the list_all result, but that could be memory intensive
        self.inner.list_all().await
    }

    async fn sync(&mut self) -> Result<()> {
        self.inner.sync().await
    }

    async fn flush(&mut self) -> Result<()> {
        self.inner.flush().await
    }

    async fn close(self) -> Result<()> {
        let (hits, misses) = self.cache_stats().await;
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "Cache statistics: {} hits, {} misses ({:.1}% hit rate)",
            hits, misses, hit_rate
        );

        self.inner.close().await
    }
}

/// Index wrapper with automatic metrics collection
pub struct MeteredIndex<I: Index> {
    inner: I,
    #[allow(dead_code)]
    name: String,
    operation_timings: Arc<Mutex<HashMap<String, Vec<Duration>>>>,
}

impl<I: Index> MeteredIndex<I> {
    /// Create a new metered index
    pub fn new(inner: I, name: String) -> Self {
        Self {
            inner,
            name,
            operation_timings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record operation timing
    async fn record_timing(&self, operation: &str, duration: Duration) {
        let mut timings = self.operation_timings.lock().await;
        timings
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        // Emit metric
        record_metric(MetricType::Histogram {
            name: "index.operation.duration",
            value: duration.as_millis() as f64,
            unit: "ms",
        });
    }

    /// Get timing statistics
    pub async fn timing_stats(&self) -> HashMap<String, (Duration, Duration, Duration)> {
        let timings = self.operation_timings.lock().await;
        let mut stats = HashMap::new();

        for (op, durations) in timings.iter() {
            if !durations.is_empty() {
                let sum: Duration = durations.iter().sum();
                let avg = sum / durations.len() as u32;
                let min = *durations.iter().min().unwrap();
                let max = *durations.iter().max().unwrap();
                stats.insert(op.clone(), (min, avg, max));
            }
        }

        stats
    }
}

#[async_trait]
impl<I: Index> Index for MeteredIndex<I> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let inner = I::open(path).await?;
        Ok(Self::new(inner, format!("metered_index_{path}")))
    }

    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.insert(id, path).await;
        self.record_timing("insert", start.elapsed()).await;
        result
    }

    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.update(id, path).await;
        self.record_timing("update", start.elapsed()).await;
        result
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        let start = Instant::now();
        let result = self.inner.delete(id).await;
        self.record_timing("delete", start.elapsed()).await;
        result
    }

    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        let start = Instant::now();
        let result = self.inner.search(query).await;
        self.record_timing("search", start.elapsed()).await;
        result
    }

    async fn sync(&mut self) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.sync().await;
        self.record_timing("sync", start.elapsed()).await;
        result
    }

    async fn flush(&mut self) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.flush().await;
        self.record_timing("flush", start.elapsed()).await;
        result
    }

    async fn close(self) -> Result<()> {
        let timing_stats = self.timing_stats().await;
        for (op, (min, avg, max)) in timing_stats {
            info!(
                "Index timing for {}: min={:?}, avg={:?}, max={:?}",
                op, min, avg, max
            );
        }
        self.inner.close().await
    }
}

// TODO: SafeTransaction implementation needs a concrete Transaction type
// Currently commented out as Transaction is a trait, not a struct
/*
/// Transaction wrapper with automatic rollback on drop
pub struct SafeTransaction {
    inner: crate::contracts::Transaction,
    committed: bool,
}

impl SafeTransaction {
    /// Create a new safe transaction
    pub fn begin(id: u64) -> Result<Self> {
        let inner = crate::contracts::Transaction::begin(id)?;
        Ok(Self {
            inner,
            committed: false,
        })
    }

    /// Add an operation to the transaction
    pub fn add_operation(&mut self, op: Operation) {
        self.inner.operations.push(op);
    }

    /// Commit the transaction
    pub async fn commit(mut self) -> Result<()> {
        self.inner.clone().commit().await?;
        self.committed = true;
        Ok(())
    }

    /// Get the transaction ID
    pub fn id(&self) -> u64 {
        self.inner.id
    }
}

impl Drop for SafeTransaction {
    fn drop(&mut self) {
        if !self.committed {
            warn!("Transaction {} dropped without commit - automatic rollback", self.inner.id);
            // In a real implementation, this would trigger rollback
        }
    }
}
*/

/// Compose multiple wrappers together
pub type FullyWrappedStorage<S> =
    TracedStorage<ValidatedStorage<RetryableStorage<CachedStorage<S>>>>;

/// Helper to create a fully wrapped storage
pub async fn create_wrapped_storage<S: Storage>(
    inner: S,
    cache_capacity: usize,
) -> FullyWrappedStorage<S> {
    let cached = CachedStorage::new(inner, cache_capacity);
    let retryable = RetryableStorage::new(cached);
    let validated = ValidatedStorage::new(retryable);

    TracedStorage::new(validated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Document;
    use crate::ValidatedTitle;

    // Mock storage for testing
    struct MockStorage {
        docs: Arc<Mutex<HashMap<ValidatedDocumentId, Document>>>,
        fail_next: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self {
                docs: Arc::new(Mutex::new(HashMap::new())),
                fail_next: Arc::new(Mutex::new(false)),
            })
        }

        async fn insert(&mut self, doc: Document) -> Result<()> {
            if *self.fail_next.lock().await {
                *self.fail_next.lock().await = false;
                anyhow::bail!("Simulated failure");
            }
            self.docs.lock().await.insert(doc.id, doc);
            Ok(())
        }

        async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
            Ok(self.docs.lock().await.get(id).cloned())
        }

        async fn update(&mut self, doc: Document) -> Result<()> {
            self.docs.lock().await.insert(doc.id, doc);
            Ok(())
        }

        async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
            let was_present = self.docs.lock().await.remove(id).is_some();
            Ok(was_present)
        }

        async fn list_all(&self) -> Result<Vec<Document>> {
            Ok(self.docs.lock().await.values().cloned().collect())
        }

        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }

        async fn flush(&mut self) -> Result<()> {
            Ok(())
        }

        async fn close(self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_traced_storage() {
        let storage = MockStorage::open("/test").await.unwrap();
        let mut traced = TracedStorage::new(storage);

        let doc = Document::new(
            ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap(),
            ValidatedPath::new("/test.md").unwrap(),
            ValidatedTitle::new("Test Document").unwrap(),
            b"test content".to_vec(),
            vec![],
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        traced.insert(doc.clone()).await.unwrap();
        assert_eq!(traced.operation_count().await, 1);

        let retrieved = traced.get(&doc.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(traced.operation_count().await, 1); // get doesn't increment
    }

    #[tokio::test]
    async fn test_cached_storage() {
        let storage = MockStorage::open("/test").await.unwrap();
        let cached = CachedStorage::new(storage, 10);

        let doc = Document::new(
            ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap(),
            ValidatedPath::new("/test.md").unwrap(),
            ValidatedTitle::new("Test Document").unwrap(),
            b"test content".to_vec(),
            vec![],
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        // Insert and get
        let mut cached_mut = cached;
        cached_mut.insert(doc.clone()).await.unwrap();

        // First get - cache hit (document was cached during insert)
        let _ = cached_mut.get(&doc.id).await.unwrap();
        let (hits, misses) = cached_mut.cache_stats().await;
        assert_eq!(hits, 1);
        assert_eq!(misses, 0);

        // Second get - cache hit again
        let _ = cached_mut.get(&doc.id).await.unwrap();
        let (hits, misses) = cached_mut.cache_stats().await;
        assert_eq!(hits, 2);
        assert_eq!(misses, 0);
    }
}
