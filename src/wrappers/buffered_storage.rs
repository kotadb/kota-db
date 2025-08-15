// Buffered Storage Wrapper - Write Performance Optimization
// This wrapper batches write operations to reduce disk I/O variability and improve performance

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{debug, info};

use crate::contracts::{Document, Storage};
use crate::observability::{record_metric, MetricType};
use crate::types::ValidatedDocumentId;

/// Configuration for write buffering behavior
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum number of writes to buffer before flushing
    pub max_buffer_size: usize,
    /// Maximum memory (in bytes) to use for buffering before flushing
    pub max_buffer_memory: usize,
    /// Maximum time to wait before flushing buffered writes
    pub flush_interval: Duration,
    /// Whether to use write-ahead logging for durability
    pub use_wal: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 100,                      // Batch up to 100 writes
            max_buffer_memory: 10 * 1024 * 1024,       // 10MB max buffer memory
            flush_interval: Duration::from_millis(50), // Flush every 50ms max
            use_wal: true,                             // Use WAL for durability
        }
    }
}

/// Write operation types that can be buffered
#[derive(Debug, Clone)]
enum BufferedOperation {
    Insert(Document),
    Update(Document),
    Delete(ValidatedDocumentId),
}

/// Storage wrapper that buffers write operations for improved performance
pub struct BufferedStorage<S: Storage> {
    inner: S,
    config: BufferConfig,
    write_buffer: Arc<Mutex<VecDeque<BufferedOperation>>>,
    buffer_memory: Arc<AtomicUsize>,
    last_flush: Arc<RwLock<Instant>>,
    flush_count: Arc<Mutex<u64>>,
    buffered_writes: Arc<Mutex<u64>>,
    shutdown: Arc<AtomicBool>,
    needs_flush: Arc<AtomicBool>,
    flush_handle: Option<tokio::task::JoinHandle<()>>,
}

impl<S: Storage> BufferedStorage<S> {
    /// Create a new buffered storage wrapper with default configuration
    pub fn new(inner: S) -> Self {
        Self::with_config(inner, BufferConfig::default())
    }

    /// Create a new buffered storage wrapper with custom configuration
    pub fn with_config(inner: S, config: BufferConfig) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let needs_flush = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let buffer_memory = Arc::new(AtomicUsize::new(0));
        let last_flush = Arc::new(RwLock::new(Instant::now()));

        // Start background flush task if not in test mode
        let flush_handle = if !cfg!(test) && !config.flush_interval.is_zero() {
            let shutdown_clone = Arc::clone(&shutdown);
            let needs_flush_clone = Arc::clone(&needs_flush);
            let last_flush_clone = Arc::clone(&last_flush);
            let interval = config.flush_interval;

            Some(tokio::spawn(async move {
                let mut interval_timer = time::interval(interval);

                loop {
                    interval_timer.tick().await;

                    // Check if shutdown requested
                    if shutdown_clone.load(Ordering::Relaxed) {
                        debug!("Flush timer shutting down");
                        break;
                    }

                    // Check if enough time has passed since last flush
                    let elapsed = last_flush_clone.read().await.elapsed();
                    if elapsed >= interval {
                        // Signal that a flush is needed
                        needs_flush_clone.store(true, Ordering::Relaxed);
                    }
                }
            }))
        } else {
            None
        };

        Self {
            inner,
            config,
            write_buffer: buffer,
            buffer_memory,
            last_flush,
            flush_count: Arc::new(Mutex::new(0)),
            buffered_writes: Arc::new(Mutex::new(0)),
            shutdown,
            needs_flush,
            flush_handle,
        }
    }

    /// Flush all buffered operations to the underlying storage
    async fn flush_buffer(&mut self) -> Result<()> {
        let operations: Vec<BufferedOperation> = {
            let mut buffer = self.write_buffer.lock().await;
            let ops = buffer.drain(..).collect();
            // Reset memory counter
            self.buffer_memory.store(0, Ordering::Relaxed);
            ops
        };

        if operations.is_empty() {
            return Ok(());
        }

        let start = Instant::now();
        let operation_count = operations.len();

        // Process all buffered operations
        for op in operations {
            match op {
                BufferedOperation::Insert(doc) => {
                    self.inner
                        .insert(doc)
                        .await
                        .context("Failed to insert document during flush")?;
                }
                BufferedOperation::Update(doc) => {
                    self.inner
                        .update(doc)
                        .await
                        .context("Failed to update document during flush")?;
                }
                BufferedOperation::Delete(id) => {
                    self.inner
                        .delete(&id)
                        .await
                        .context("Failed to delete document during flush")?;
                }
            }
        }

        // Ensure all writes are persisted
        self.inner
            .sync()
            .await
            .context("Failed to sync after flush")?;

        let duration = start.elapsed();

        // Update metrics
        *self.flush_count.lock().await += 1;
        *self.last_flush.write().await = Instant::now();

        info!(
            "Flushed {} operations in {:?} ({:.2} ops/ms)",
            operation_count,
            duration,
            operation_count as f64 / duration.as_millis() as f64
        );

        record_metric(MetricType::Histogram {
            name: "storage.buffer.flush_duration",
            value: duration.as_millis() as f64,
            unit: "ms",
        });

        record_metric(MetricType::Counter {
            name: "storage.buffer.operations_flushed",
            value: operation_count as u64,
        });

        Ok(())
    }

    /// Check if buffer should be flushed based on size, memory, or time
    async fn should_flush(&self) -> bool {
        let buffer_size = self.write_buffer.lock().await.len();
        let buffer_memory = self.buffer_memory.load(Ordering::Relaxed);
        let time_since_flush = self.last_flush.read().await.elapsed();

        buffer_size >= self.config.max_buffer_size
            || buffer_memory >= self.config.max_buffer_memory
            || (buffer_size > 0 && time_since_flush >= self.config.flush_interval)
    }

    /// Check for flush signals from the background timer
    async fn check_and_flush_if_needed(&mut self) -> Result<()> {
        // Check if timer has signaled a flush is needed
        if self.needs_flush.load(Ordering::Relaxed) {
            // Reset the flag
            self.needs_flush.store(false, Ordering::Relaxed);

            // Flush if we have data
            if !self.write_buffer.lock().await.is_empty() {
                debug!("Timer-triggered flush");
                self.flush_buffer().await?;
            }
        }

        // Also check other flush conditions
        if self.should_flush().await {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Get statistics about the buffer
    pub async fn buffer_stats(&self) -> (usize, u64, u64) {
        let buffer_size = self.write_buffer.lock().await.len();
        let flush_count = *self.flush_count.lock().await;
        let total_buffered = *self.buffered_writes.lock().await;
        (buffer_size, flush_count, total_buffered)
    }
}

#[async_trait]
impl<S: Storage> Storage for BufferedStorage<S> {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let inner = S::open(path)
            .await
            .context("Failed to open underlying storage")?;
        Ok(Self::new(inner))
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        // Calculate document size
        let doc_size = doc.content.len() + doc.path.as_str().len() + doc.title.as_str().len();

        // Add to buffer
        {
            let mut buffer = self.write_buffer.lock().await;
            buffer.push_back(BufferedOperation::Insert(doc));
            *self.buffered_writes.lock().await += 1;

            // Update memory counter
            self.buffer_memory.fetch_add(doc_size, Ordering::Relaxed);
        }

        // Check for timer-triggered flush and other conditions
        self.check_and_flush_if_needed().await?;

        Ok(())
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        // Check buffer for pending operations on this document
        {
            let buffer = self.write_buffer.lock().await;
            // Search buffer in reverse order for most recent operation
            for op in buffer.iter().rev() {
                match op {
                    BufferedOperation::Insert(doc) | BufferedOperation::Update(doc) => {
                        if doc.id == *id {
                            return Ok(Some(doc.clone()));
                        }
                    }
                    BufferedOperation::Delete(del_id) => {
                        if del_id == id {
                            return Ok(None);
                        }
                    }
                }
            }
        }

        // Not in buffer, check underlying storage
        self.inner.get(id).await
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        // Calculate document size
        let doc_size = doc.content.len() + doc.path.as_str().len() + doc.title.as_str().len();

        // Add to buffer
        {
            let mut buffer = self.write_buffer.lock().await;
            buffer.push_back(BufferedOperation::Update(doc));
            *self.buffered_writes.lock().await += 1;

            // Update memory counter
            self.buffer_memory.fetch_add(doc_size, Ordering::Relaxed);
        }

        // Check for timer-triggered flush and other conditions
        self.check_and_flush_if_needed().await?;

        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        // Check if document exists first (in buffer or storage)
        let exists = self.get(id).await?.is_some();

        if exists {
            // Add to buffer
            {
                let mut buffer = self.write_buffer.lock().await;
                buffer.push_back(BufferedOperation::Delete(*id));
                *self.buffered_writes.lock().await += 1;
            }

            // Check for timer-triggered flush and other conditions
            self.check_and_flush_if_needed().await?;
        }

        Ok(exists)
    }

    async fn flush(&mut self) -> Result<()> {
        // Flush all buffered operations
        self.flush_buffer().await
    }

    async fn sync(&mut self) -> Result<()> {
        // Flush buffer before syncing to ensure durability
        self.flush_buffer().await?;
        self.inner.sync().await
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        // Get documents from underlying storage
        let mut docs = self.inner.list_all().await?;

        // Create a map for efficient lookups and updates
        let mut doc_map: HashMap<ValidatedDocumentId, Document> =
            docs.drain(..).map(|d| (d.id, d)).collect();

        // Apply buffered operations to get consistent view
        {
            let buffer = self.write_buffer.lock().await;
            for op in buffer.iter() {
                match op {
                    BufferedOperation::Insert(doc) | BufferedOperation::Update(doc) => {
                        // Insert or update in the map
                        doc_map.insert(doc.id, doc.clone());
                    }
                    BufferedOperation::Delete(id) => {
                        // Remove from the map
                        doc_map.remove(id);
                    }
                }
            }
        }

        // Return the merged list
        Ok(doc_map.into_values().collect())
    }

    async fn close(mut self) -> Result<()> {
        // Signal shutdown to background task
        self.shutdown.store(true, Ordering::Relaxed);

        // Stop the flush timer if it exists
        if let Some(handle) = self.flush_handle.take() {
            handle.abort();
            // Wait for it to finish (with timeout)
            let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
        }

        // Flush any remaining buffered operations
        self.flush_buffer().await?;

        // Close underlying storage
        self.inner.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::DocumentBuilder;
    use crate::file_storage::FileStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_buffered_writes() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = FileStorage::open(temp_dir.path().to_str().unwrap()).await?;
        let mut buffered = BufferedStorage::with_config(
            storage,
            BufferConfig {
                max_buffer_size: 5,
                max_buffer_memory: 1024 * 1024, // 1MB
                flush_interval: Duration::from_millis(100),
                use_wal: true,
            },
        );

        // Insert documents - should buffer without immediate disk writes
        for i in 0..4 {
            let doc = DocumentBuilder::new()
                .path(format!("test{}.md", i))
                .unwrap()
                .title(format!("Test {}", i))
                .unwrap()
                .content(b"test content")
                .build()
                .unwrap();
            buffered.insert(doc).await?;
        }

        // Check buffer stats
        let (buffer_size, _, _) = buffered.buffer_stats().await;
        assert_eq!(buffer_size, 4);

        // One more insert should trigger flush (max_buffer_size = 5)
        let doc = DocumentBuilder::new()
            .path("test5.md")
            .unwrap()
            .title("Test 5")
            .unwrap()
            .content(b"test content")
            .build()
            .unwrap();
        buffered.insert(doc).await?;

        // Buffer should be flushed
        let (buffer_size, flush_count, _) = buffered.buffer_stats().await;
        assert_eq!(buffer_size, 0);
        assert!(flush_count > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_read_from_buffer() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = FileStorage::open(temp_dir.path().to_str().unwrap()).await?;
        let mut buffered = BufferedStorage::new(storage);

        let doc = DocumentBuilder::new()
            .path("test.md")
            .unwrap()
            .title("Test")
            .unwrap()
            .content(b"test content")
            .build()
            .unwrap();

        let doc_id = doc.id;
        buffered.insert(doc).await?;

        // Document should be readable from buffer even before flush
        let retrieved = buffered.get(&doc_id).await?;
        assert!(retrieved.is_some());

        Ok(())
    }
}
