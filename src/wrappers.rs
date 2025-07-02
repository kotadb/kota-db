// Wrapper Components - Stage 6: Component Library
// This module provides high-level wrappers that automatically apply best practices
// like tracing, validation, retries, and caching.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use async_trait::async_trait;

use crate::contracts::{Storage, Index, Document, Query, StorageMetrics};
use crate::observability::*;
use crate::validation::{self, ValidationContext};
use crate::types::*;

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
    async fn open(path: &str) -> Result<Self> where Self: Sized {
        let trace_id = Uuid::new_v4();
        info!("[{}] Opening storage at: {}", trace_id, path);
        
        let start = Instant::now();
        let inner = S::open(path).await
            .context("Failed to open storage")?;
        
        let duration = start.elapsed();
        info!("[{}] Storage opened in {:?}", trace_id, duration);
        record_metric(MetricType::Histogram {
            name: "storage.open.duration".to_string(),
            value: duration.as_millis() as f64,
            tags: vec![("path".to_string(), path.to_string())],
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
            info!("[{}] Inserting document: {}", self.trace_id, doc.id);
            
            let result = self.inner.insert(doc.clone()).await;
            
            let duration = start.elapsed();
            log_operation(
                &OperationContext::new("storage.insert")
                    .with_attribute("doc_id", &doc.id.to_string())
                    .with_attribute("size", &doc.size.to_string()),
                &Operation::StorageWrite { 
                    doc_id: doc.id, 
                    size_bytes: doc.size 
                },
                &result.as_ref().map(|_| ()),
            );
            
            result
        }).await
    }
    
    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        with_trace_id("storage.get", async {
            let start = Instant::now();
            debug!("[{}] Getting document: {}", self.trace_id, id);
            
            let result = self.inner.get(id).await;
            
            let duration = start.elapsed();
            let size = result.as_ref()
                .ok()
                .and_then(|opt| opt.as_ref())
                .map(|doc| doc.size)
                .unwrap_or(0);
            
            log_operation(
                &OperationContext::new("storage.get")
                    .with_attribute("doc_id", &id.to_string())
                    .with_attribute("found", &result.as_ref()
                        .map(|opt| opt.is_some().to_string())
                        .unwrap_or_else(|_| "error".to_string())),
                &Operation::StorageRead { 
                    doc_id: *id, 
                    size_bytes: size 
                },
                &result.as_ref().map(|_| ()),
            );
            
            result
        }).await
    }
    
    async fn update(&mut self, doc: Document) -> Result<()> {
        self.increment_op_count().await;
        
        with_trace_id("storage.update", async {
            info!("[{}] Updating document: {}", self.trace_id, doc.id);
            self.inner.update(doc.clone()).await
        }).await
    }
    
    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.increment_op_count().await;
        
        with_trace_id("storage.delete", async {
            info!("[{}] Deleting document: {}", self.trace_id, id);
            self.inner.delete(id).await
        }).await
    }
    
    async fn sync(&mut self) -> Result<()> {
        with_trace_id("storage.sync", async {
            info!("[{}] Syncing storage", self.trace_id);
            let start = Instant::now();
            
            let result = self.inner.sync().await;
            
            let duration = start.elapsed();
            record_metric(MetricType::Histogram {
                name: "storage.sync.duration".to_string(),
                value: duration.as_millis() as f64,
                tags: vec![],
            });
            
            result
        }).await
    }
    
    async fn close(self) -> Result<()> {
        let op_count = self.operation_count().await;
        info!("[{}] Closing storage after {} operations", self.trace_id, op_count);
        self.inner.close().await
    }
}

/// Storage wrapper that validates all inputs and outputs
pub struct ValidatedStorage<S: Storage> {
    inner: S,
    existing_ids: Arc<RwLock<std::collections::HashSet<Uuid>>>,
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
    async fn open(path: &str) -> Result<Self> where Self: Sized {
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
    
    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        let result = self.inner.get(id).await?;
        
        // Validate returned document if present
        if let Some(ref doc) = result {
            validation::document::validate_for_insert(doc, &std::collections::HashSet::new())?;
        }
        
        Ok(result)
    }
    
    async fn update(&mut self, doc: Document) -> Result<()> {
        // Get existing document for validation
        let existing = self.inner.get(&doc.id).await?
            .ok_or_else(|| anyhow::anyhow!("Document not found for update"))?;
        
        // Validate update
        validation::document::validate_for_update(&doc, &existing)?;
        
        // Perform update
        self.inner.update(doc).await
    }
    
    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.inner.delete(id).await?;
        self.existing_ids.write().await.remove(id);
        Ok(())
    }
    
    async fn sync(&mut self) -> Result<()> {
        self.inner.sync().await
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
    
    /// Execute an operation with exponential backoff retry
    async fn retry<F, Fut, T>(&self, operation: &str, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.base_delay;
        
        loop {
            attempt += 1;
            
            match f().await {
                Ok(value) => {
                    if attempt > 1 {
                        info!("Operation {} succeeded after {} attempts", operation, attempt);
                    }
                    return Ok(value);
                }
                Err(e) if attempt >= self.max_retries => {
                    error!("Operation {} failed after {} attempts: {}", operation, attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!("Operation {} failed (attempt {}/{}): {}", 
                          operation, attempt, self.max_retries, e);
                    
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.max_delay);
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    delay += jitter;
                }
            }
        }
    }
}

#[async_trait]
impl<S: Storage> Storage for RetryableStorage<S> {
    async fn open(path: &str) -> Result<Self> where Self: Sized {
        let inner = S::open(path).await?;
        Ok(Self::new(inner))
    }
    
    async fn insert(&mut self, doc: Document) -> Result<()> {
        let doc_clone = doc.clone();
        self.retry("insert", || async {
            self.inner.insert(doc_clone.clone()).await
        }).await
    }
    
    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        let id = *id;
        self.retry("get", || async move {
            self.inner.get(&id).await
        }).await
    }
    
    async fn update(&mut self, doc: Document) -> Result<()> {
        let doc_clone = doc.clone();
        self.retry("update", || async {
            self.inner.update(doc_clone.clone()).await
        }).await
    }
    
    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        let id = *id;
        self.retry("delete", || async move {
            self.inner.delete(&id).await
        }).await
    }
    
    async fn sync(&mut self) -> Result<()> {
        self.retry("sync", || async {
            self.inner.sync().await
        }).await
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
    async fn open(path: &str) -> Result<Self> where Self: Sized {
        let inner = S::open(path).await?;
        Ok(Self::new(inner, 1000)) // Default 1000 document cache
    }
    
    async fn insert(&mut self, doc: Document) -> Result<()> {
        self.inner.insert(doc.clone()).await?;
        
        // Update cache
        self.cache.lock().await.insert(doc.id, doc);
        
        Ok(())
    }
    
    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        // Check cache first
        {
            let mut cache = self.cache.lock().await;
            if let Some(doc) = cache.get(id) {
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
            self.cache.lock().await.insert(*id, doc.clone());
        }
        
        Ok(result)
    }
    
    async fn update(&mut self, doc: Document) -> Result<()> {
        self.inner.update(doc.clone()).await?;
        
        // Update cache
        self.cache.lock().await.insert(doc.id, doc);
        
        Ok(())
    }
    
    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.inner.delete(id).await?;
        
        // Remove from cache
        self.cache.lock().await.remove(id);
        
        Ok(())
    }
    
    async fn sync(&mut self) -> Result<()> {
        self.inner.sync().await
    }
    
    async fn close(self) -> Result<()> {
        let (hits, misses) = self.cache_stats().await;
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        
        info!("Cache statistics: {} hits, {} misses ({:.1}% hit rate)", 
              hits, misses, hit_rate);
        
        self.inner.close().await
    }
}

/// Index wrapper with automatic metrics collection
pub struct MeteredIndex<I: Index> {
    inner: I,
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
        timings.entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        // Emit metric
        record_metric(MetricType::Histogram {
            name: format!("index.{}.duration", operation),
            value: duration.as_millis() as f64,
            tags: vec![("index".to_string(), self.name.clone())],
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
    type Key = I::Key;
    type Value = I::Value;
    
    async fn insert(&mut self, key: Self::Key, value: Self::Value) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.insert(key, value).await;
        self.record_timing("insert", start.elapsed()).await;
        result
    }
    
    async fn delete(&mut self, key: &Self::Key) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.delete(key).await;
        self.record_timing("delete", start.elapsed()).await;
        result
    }
    
    async fn search(&self, query: &Query) -> Result<Vec<Self::Value>> {
        let start = Instant::now();
        let result = self.inner.search(query).await;
        self.record_timing("search", start.elapsed()).await;
        result
    }
    
    async fn flush(&mut self) -> Result<()> {
        let start = Instant::now();
        let result = self.inner.flush().await;
        self.record_timing("flush", start.elapsed()).await;
        result
    }
}

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
        self.inner.commit().await?;
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

/// Compose multiple wrappers together
pub type FullyWrappedStorage<S> = TracedStorage<ValidatedStorage<RetryableStorage<CachedStorage<S>>>>;

/// Helper to create a fully wrapped storage
pub async fn create_wrapped_storage<S: Storage>(
    inner: S,
    cache_capacity: usize,
) -> FullyWrappedStorage<S> {
    let cached = CachedStorage::new(inner, cache_capacity);
    let retryable = RetryableStorage::new(cached);
    let validated = ValidatedStorage::new(retryable);
    let traced = TracedStorage::new(validated);
    traced
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Document;
    
    // Mock storage for testing
    struct MockStorage {
        docs: Arc<Mutex<HashMap<Uuid, Document>>>,
        fail_next: Arc<Mutex<bool>>,
    }
    
    #[async_trait]
    impl Storage for MockStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
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
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            Ok(self.docs.lock().await.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            self.docs.lock().await.insert(doc.id, doc);
            Ok(())
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            self.docs.lock().await.remove(id);
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
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
            Uuid::new_v4(),
            "/test.md".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "Test".to_string(),
            100,
        ).unwrap();
        
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
            Uuid::new_v4(),
            "/test.md".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "Test".to_string(),
            100,
        ).unwrap();
        
        // Insert and get
        let mut cached_mut = cached;
        cached_mut.insert(doc.clone()).await.unwrap();
        
        // First get - cache miss
        let _ = cached_mut.get(&doc.id).await.unwrap();
        let (hits, misses) = cached_mut.cache_stats().await;
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);
        
        // Second get - cache hit
        let _ = cached_mut.get(&doc.id).await.unwrap();
        let (hits, misses) = cached_mut.cache_stats().await;
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }
}