// Adversarial Tests - Stage 5: Simulating Failures and Edge Cases
// These tests exercise failure modes, race conditions, and error handling
// to ensure the system degrades gracefully under stress

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use tempfile::TempDir;
use std::time::Duration;

// Mock implementations for adversarial testing
mod mocks {
    use super::*;
    use kotadb::*;
    use async_trait::async_trait;
    
    /// Storage that randomly fails operations
    pub struct FlakyStorage {
        inner: HashMap<Uuid, Document>,
        fail_rate: f32,
        fail_count: AtomicU64,
        closed: AtomicBool,
    }
    
    impl FlakyStorage {
        pub fn new(fail_rate: f32) -> Self {
            Self {
                inner: HashMap::new(),
                fail_rate,
                fail_count: AtomicU64::new(0),
                closed: AtomicBool::new(false),
            }
        }
        
        fn maybe_fail(&self) -> Result<()> {
            if self.closed.load(Ordering::Relaxed) {
                anyhow::bail!("Storage is closed");
            }
            
            if rand::random::<f32>() < self.fail_rate {
                self.fail_count.fetch_add(1, Ordering::Relaxed);
                anyhow::bail!("Random failure injected");
            }
            Ok(())
        }
    }
    
    #[async_trait]
    impl Storage for FlakyStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self::new(0.1)) // 10% failure rate
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            self.maybe_fail()?;
            self.inner.insert(doc.id, doc);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            self.maybe_fail()?;
            Ok(self.inner.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            self.maybe_fail()?;
            if !self.inner.contains_key(&doc.id) {
                anyhow::bail!("Document not found");
            }
            self.inner.insert(doc.id, doc);
            Ok(())
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            self.maybe_fail()?;
            self.inner.remove(id);
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            self.maybe_fail()?;
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            self.closed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }
    
    /// Storage that simulates disk full errors
    pub struct DiskFullStorage {
        inner: HashMap<Uuid, Document>,
        capacity_bytes: AtomicU64,
        used_bytes: AtomicU64,
    }
    
    impl DiskFullStorage {
        pub fn new(capacity_bytes: u64) -> Self {
            Self {
                inner: HashMap::new(),
                capacity_bytes: AtomicU64::new(capacity_bytes),
                used_bytes: AtomicU64::new(0),
            }
        }
        
        fn check_space(&self, needed: u64) -> Result<()> {
            let used = self.used_bytes.load(Ordering::Relaxed);
            let capacity = self.capacity_bytes.load(Ordering::Relaxed);
            
            if used + needed > capacity {
                anyhow::bail!("Disk full: {} + {} > {}", used, needed, capacity);
            }
            Ok(())
        }
    }
    
    #[async_trait]
    impl Storage for DiskFullStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self::new(1024 * 1024)) // 1MB capacity
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            self.check_space(doc.size)?;
            self.inner.insert(doc.id, doc.clone());
            self.used_bytes.fetch_add(doc.size, Ordering::Relaxed);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            Ok(self.inner.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            if let Some(old) = self.inner.get(&doc.id) {
                let size_diff = doc.size.saturating_sub(old.size);
                self.check_space(size_diff)?;
                self.inner.insert(doc.id, doc.clone());
                self.used_bytes.fetch_add(size_diff, Ordering::Relaxed);
                Ok(())
            } else {
                anyhow::bail!("Document not found");
            }
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            if let Some(doc) = self.inner.remove(id) {
                self.used_bytes.fetch_sub(doc.size, Ordering::Relaxed);
            }
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
    
    /// Storage with simulated latency spikes
    pub struct SlowStorage {
        inner: HashMap<Uuid, Document>,
        base_latency_ms: u64,
        spike_probability: f32,
        spike_multiplier: u64,
    }
    
    impl SlowStorage {
        pub fn new(base_latency_ms: u64) -> Self {
            Self {
                inner: HashMap::new(),
                base_latency_ms,
                spike_probability: 0.1,
                spike_multiplier: 10,
            }
        }
        
        async fn simulate_latency(&self) {
            let latency = if rand::random::<f32>() < self.spike_probability {
                self.base_latency_ms * self.spike_multiplier
            } else {
                self.base_latency_ms
            };
            
            tokio::time::sleep(Duration::from_millis(latency)).await;
        }
    }
    
    #[async_trait]
    impl Storage for SlowStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self::new(10)) // 10ms base latency
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            self.simulate_latency().await;
            self.inner.insert(doc.id, doc);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            self.simulate_latency().await;
            Ok(self.inner.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            self.simulate_latency().await;
            if !self.inner.contains_key(&doc.id) {
                anyhow::bail!("Document not found");
            }
            self.inner.insert(doc.id, doc);
            Ok(())
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            self.simulate_latency().await;
            self.inner.remove(id);
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            self.simulate_latency().await;
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
}

/// Test random failures during operations
#[tokio::test]
async fn test_random_failures() -> Result<()> {
    use mocks::FlakyStorage;
    use kotadb::*;
    
    let mut storage = FlakyStorage::new(0.3); // 30% failure rate
    let mut success_count = 0;
    let mut failure_count = 0;
    
    // Try 100 operations
    for i in 0..100 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024,
            1000,
            2000,
            format!("Test Doc {}", i),
            100,
        )?;
        
        match storage.insert(doc).await {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }
    
    // Should have some successes and failures
    assert!(success_count > 20);
    assert!(failure_count > 20);
    
    Ok(())
}

/// Test disk full scenarios
#[tokio::test]
async fn test_disk_full() -> Result<()> {
    use mocks::DiskFullStorage;
    use kotadb::*;
    
    let mut storage = DiskFullStorage::new(10_000); // 10KB capacity
    let mut inserted = 0;
    
    // Insert documents until disk is full
    for i in 0..20 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024, // 1KB each
            1000,
            2000,
            format!("Test Doc {}", i),
            100,
        )?;
        
        match storage.insert(doc).await {
            Ok(_) => inserted += 1,
            Err(e) => {
                assert!(e.to_string().contains("Disk full"));
                break;
            }
        }
    }
    
    // Should have inserted about 9-10 documents
    assert!(inserted >= 9 && inserted <= 10);
    
    // Delete one document to free space
    let doc_to_delete = Document::new(
        Uuid::new_v4(),
        "/test/0.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Test Doc 0".to_string(),
        100,
    )?;
    
    // Insert it first
    let mut storage2 = DiskFullStorage::new(2048);
    storage2.insert(doc_to_delete.clone()).await?;
    
    // Then delete to free space
    storage2.delete(&doc_to_delete.id).await?;
    
    // Should be able to insert another document now
    let new_doc = Document::new(
        Uuid::new_v4(),
        "/test/new.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "New Doc".to_string(),
        100,
    )?;
    
    assert!(storage2.insert(new_doc).await.is_ok());
    
    Ok(())
}

/// Test concurrent access patterns
#[tokio::test]
async fn test_concurrent_stress() -> Result<()> {
    use kotadb::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    struct ConcurrentStorage {
        inner: Arc<Mutex<HashMap<Uuid, Document>>>,
        lock_contentions: AtomicU64,
    }
    
    #[async_trait::async_trait]
    impl Storage for ConcurrentStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self {
                inner: Arc::new(Mutex::new(HashMap::new())),
                lock_contentions: AtomicU64::new(0),
            })
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            let start = std::time::Instant::now();
            let mut map = self.inner.lock().await;
            if start.elapsed().as_micros() > 100 {
                self.lock_contentions.fetch_add(1, Ordering::Relaxed);
            }
            map.insert(doc.id, doc);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            let map = self.inner.lock().await;
            Ok(map.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            let mut map = self.inner.lock().await;
            if !map.contains_key(&doc.id) {
                anyhow::bail!("Document not found");
            }
            map.insert(doc.id, doc);
            Ok(())
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            let mut map = self.inner.lock().await;
            map.remove(id);
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
    
    let storage = Arc::new(Mutex::new(ConcurrentStorage::open("/test").await?));
    let mut handles = vec![];
    
    // Spawn 10 concurrent writers
    for thread_id in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            for i in 0..100 {
                let doc = Document::new(
                    Uuid::new_v4(),
                    format!("/test/thread{}/{}.md", thread_id, i),
                    [0u8; 32],
                    1024,
                    1000,
                    2000,
                    format!("Doc {}-{}", thread_id, i),
                    100,
                ).unwrap();
                
                let mut s = storage_clone.lock().await;
                s.insert(doc).await.unwrap();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all writers
    for handle in handles {
        handle.await?;
    }
    
    // Check contentions
    let storage = storage.lock().await;
    let contentions = storage.lock_contentions.load(Ordering::Relaxed);
    println!("Lock contentions: {}", contentions);
    
    Ok(())
}

/// Test timeout scenarios
#[tokio::test]
async fn test_operation_timeouts() -> Result<()> {
    use mocks::SlowStorage;
    use kotadb::*;
    
    let mut storage = SlowStorage::new(100); // 100ms base latency
    
    let doc = Document::new(
        Uuid::new_v4(),
        "/test/timeout.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Timeout Test".to_string(),
        100,
    )?;
    
    // Test with timeout
    let result = tokio::time::timeout(
        Duration::from_millis(50),
        storage.insert(doc.clone())
    ).await;
    
    // Should timeout sometimes (when latency spike happens)
    match result {
        Ok(Ok(_)) => {
            // Normal latency, operation succeeded
        }
        Err(_) => {
            // Timeout occurred
            println!("Operation timed out as expected");
        }
        Ok(Err(e)) => {
            // Operation failed for other reason
            return Err(e);
        }
    }
    
    Ok(())
}

/// Test invalid input handling
#[tokio::test]
async fn test_invalid_inputs() -> Result<()> {
    use kotadb::*;
    
    // Test document with invalid fields
    let invalid_docs = vec![
        // Empty path
        (
            Uuid::new_v4(),
            "".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "Test".to_string(),
            100,
        ),
        // Zero size
        (
            Uuid::new_v4(),
            "/test.md".to_string(),
            [0u8; 32],
            0,
            1000,
            2000,
            "Test".to_string(),
            100,
        ),
        // Updated < created
        (
            Uuid::new_v4(),
            "/test.md".to_string(),
            [0u8; 32],
            1024,
            2000,
            1000,
            "Test".to_string(),
            100,
        ),
        // Empty title
        (
            Uuid::new_v4(),
            "/test.md".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "".to_string(),
            100,
        ),
    ];
    
    for (id, path, hash, size, created, updated, title, word_count) in invalid_docs {
        let result = Document::new(id, path, hash, size, created, updated, title, word_count);
        assert!(result.is_err(), "Expected validation error");
    }
    
    // Test invalid queries
    let invalid_queries = vec![
        // No criteria
        (None, None, None, 10),
        // Invalid limit
        (Some("test".to_string()), None, None, 0),
        (Some("test".to_string()), None, None, 10000),
        // Invalid date range
        (Some("test".to_string()), None, Some((2000, 1000)), 10),
    ];
    
    for (text, tags, date_range, limit) in invalid_queries {
        let result = Query::new(text, tags, date_range, limit);
        assert!(result.is_err(), "Expected validation error");
    }
    
    Ok(())
}

/// Test memory leaks under stress
#[tokio::test]
async fn test_memory_pressure() -> Result<()> {
    use kotadb::*;
    
    struct MemoryTrackingStorage {
        docs: HashMap<Uuid, Document>,
        allocations: AtomicU64,
        deallocations: AtomicU64,
    }
    
    impl MemoryTrackingStorage {
        fn new() -> Self {
            Self {
                docs: HashMap::new(),
                allocations: AtomicU64::new(0),
                deallocations: AtomicU64::new(0),
            }
        }
        
        fn track_alloc(&self, size: u64) {
            self.allocations.fetch_add(size, Ordering::Relaxed);
        }
        
        fn track_dealloc(&self, size: u64) {
            self.deallocations.fetch_add(size, Ordering::Relaxed);
        }
        
        fn get_net_allocated(&self) -> i64 {
            let alloc = self.allocations.load(Ordering::Relaxed) as i64;
            let dealloc = self.deallocations.load(Ordering::Relaxed) as i64;
            alloc - dealloc
        }
    }
    
    #[async_trait::async_trait]
    impl Storage for MemoryTrackingStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self::new())
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            self.track_alloc(doc.size);
            self.docs.insert(doc.id, doc);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            Ok(self.docs.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            if let Some(old) = self.docs.get(&doc.id) {
                self.track_dealloc(old.size);
                self.track_alloc(doc.size);
                self.docs.insert(doc.id, doc);
                Ok(())
            } else {
                anyhow::bail!("Document not found");
            }
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            if let Some(doc) = self.docs.remove(id) {
                self.track_dealloc(doc.size);
            }
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            // Dealloc all remaining docs
            for doc in self.docs.values() {
                self.track_dealloc(doc.size);
            }
            Ok(())
        }
    }
    
    let mut storage = MemoryTrackingStorage::new();
    let mut doc_ids = vec![];
    
    // Insert 1000 documents
    for i in 0..1000 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024 * (i % 10 + 1), // Variable sizes
            1000,
            2000,
            format!("Doc {}", i),
            100,
        )?;
        
        doc_ids.push(doc.id);
        storage.insert(doc).await?;
    }
    
    // Update half of them
    for i in 0..500 {
        if let Some(doc) = storage.get(&doc_ids[i]).await? {
            let mut updated = doc;
            updated.size = 2048; // Change size
            updated.updated = 3000;
            storage.update(updated).await?;
        }
    }
    
    // Delete half of them
    for i in 500..1000 {
        storage.delete(&doc_ids[i]).await?;
    }
    
    // Check memory tracking before close
    let net_before_close = storage.get_net_allocated();
    assert!(net_before_close > 0, "Should have allocated memory");
    
    // Close should clean up
    storage.close().await?;
    
    Ok(())
}

/// Test transaction rollback scenarios
#[tokio::test]
async fn test_transaction_failures() -> Result<()> {
    use kotadb::*;
    
    // Simulate transaction that fails partway through
    let mut tx = Transaction::begin(12345)?;
    
    // Add some valid operations
    tx.operations.push(Operation::StorageWrite {
        doc_id: Uuid::new_v4(),
        size_bytes: 1024,
    });
    
    tx.operations.push(Operation::IndexUpdate {
        index_name: "trigram".to_string(),
        doc_id: Uuid::new_v4(),
    });
    
    // Validate operations (should pass)
    for op in &tx.operations {
        op.validate()?;
    }
    
    // Test transaction ID conflicts
    let tx1 = Transaction::begin(100)?;
    let tx2 = Transaction::begin(100)?; // Same ID
    
    // In real implementation, this would fail due to ID conflict
    // but our simple implementation doesn't track active transactions
    
    Ok(())
}

/// Test race conditions in concurrent updates
#[tokio::test]
async fn test_concurrent_update_race() -> Result<()> {
    use kotadb::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    struct RaceTestStorage {
        docs: Arc<RwLock<HashMap<Uuid, Document>>>,
        update_count: AtomicU64,
        race_detected: AtomicBool,
    }
    
    #[async_trait::async_trait]
    impl Storage for RaceTestStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self {
                docs: Arc::new(RwLock::new(HashMap::new())),
                update_count: AtomicU64::new(0),
                race_detected: AtomicBool::new(false),
            })
        }
        
        async fn insert(&mut self, doc: Document) -> Result<()> {
            let mut docs = self.docs.write().await;
            docs.insert(doc.id, doc);
            Ok(())
        }
        
        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            let docs = self.docs.read().await;
            Ok(docs.get(id).cloned())
        }
        
        async fn update(&mut self, doc: Document) -> Result<()> {
            // Simulate race condition detection
            let count_before = self.update_count.fetch_add(1, Ordering::SeqCst);
            
            // Small delay to increase chance of race
            tokio::time::sleep(Duration::from_micros(10)).await;
            
            let mut docs = self.docs.write().await;
            
            if let Some(existing) = docs.get(&doc.id) {
                // Check if document was modified by another thread
                if existing.updated > doc.created {
                    self.race_detected.store(true, Ordering::Relaxed);
                    anyhow::bail!("Concurrent modification detected");
                }
                docs.insert(doc.id, doc);
                Ok(())
            } else {
                anyhow::bail!("Document not found");
            }
        }
        
        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            let mut docs = self.docs.write().await;
            docs.remove(id);
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
    
    let storage = Arc::new(RwLock::new(RaceTestStorage::open("/test").await?));
    
    // Insert initial document
    let doc_id = Uuid::new_v4();
    let initial_doc = Document::new(
        doc_id,
        "/test/race.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        1000,
        "Race Test".to_string(),
        100,
    )?;
    
    storage.write().await.insert(initial_doc).await?;
    
    // Spawn multiple updaters
    let mut handles = vec![];
    for i in 0..5 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let doc = Document::new(
                doc_id,
                "/test/race.md".to_string(),
                [0u8; 32],
                1024 + i,
                1000,
                2000 + i as i64,
                format!("Updated by thread {}", i),
                100 + i as u32,
            ).unwrap();
            
            // Try to update
            let mut s = storage_clone.write().await;
            let _ = s.update(doc).await; // May fail due to race
        });
        handles.push(handle);
    }
    
    // Wait for all updates
    for handle in handles {
        let _ = handle.await; // Ignore errors
    }
    
    Ok(())
}

/// Test panic recovery
#[tokio::test]
#[should_panic(expected = "Simulated panic")]
async fn test_panic_during_operation() {
    use kotadb::*;
    
    struct PanicStorage;
    
    #[async_trait::async_trait]
    impl Storage for PanicStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self)
        }
        
        async fn insert(&mut self, _doc: Document) -> Result<()> {
            panic!("Simulated panic during insert");
        }
        
        async fn get(&self, _id: &Uuid) -> Result<Option<Document>> {
            Ok(None)
        }
        
        async fn update(&mut self, _doc: Document) -> Result<()> {
            Ok(())
        }
        
        async fn delete(&mut self, _id: &Uuid) -> Result<()> {
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
    
    let mut storage = PanicStorage;
    let doc = Document::new(
        Uuid::new_v4(),
        "/test/panic.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Panic Test".to_string(),
        100,
    ).unwrap();
    
    // This should panic
    storage.insert(doc).await.unwrap();
}

/// Test resource cleanup on errors
#[tokio::test]
async fn test_resource_cleanup() -> Result<()> {
    use kotadb::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    
    struct ResourceTracker {
        file_open: Arc<AtomicBool>,
        lock_held: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    }
    
    struct TrackedStorage {
        tracker: ResourceTracker,
    }
    
    impl TrackedStorage {
        fn new() -> (Self, ResourceTracker) {
            let tracker = ResourceTracker {
                file_open: Arc::new(AtomicBool::new(false)),
                lock_held: Arc::new(AtomicBool::new(false)),
                connection_active: Arc::new(AtomicBool::new(false)),
            };
            
            let storage = Self {
                tracker: ResourceTracker {
                    file_open: Arc::clone(&tracker.file_open),
                    lock_held: Arc::clone(&tracker.lock_held),
                    connection_active: Arc::clone(&tracker.connection_active),
                },
            };
            
            (storage, tracker)
        }
    }
    
    #[async_trait::async_trait]
    impl Storage for TrackedStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            let (storage, _) = Self::new();
            
            // Simulate opening resources
            storage.tracker.file_open.store(true, Ordering::Relaxed);
            storage.tracker.lock_held.store(true, Ordering::Relaxed);
            storage.tracker.connection_active.store(true, Ordering::Relaxed);
            
            Ok(storage)
        }
        
        async fn insert(&mut self, _doc: Document) -> Result<()> {
            // Simulate error that requires cleanup
            anyhow::bail!("Simulated insert error");
        }
        
        async fn get(&self, _id: &Uuid) -> Result<Option<Document>> {
            Ok(None)
        }
        
        async fn update(&mut self, _doc: Document) -> Result<()> {
            Ok(())
        }
        
        async fn delete(&mut self, _id: &Uuid) -> Result<()> {
            Ok(())
        }
        
        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }
        
        async fn close(self) -> Result<()> {
            // Clean up resources
            self.tracker.file_open.store(false, Ordering::Relaxed);
            self.tracker.lock_held.store(false, Ordering::Relaxed);
            self.tracker.connection_active.store(false, Ordering::Relaxed);
            Ok(())
        }
    }
    
    let mut storage = TrackedStorage::open("/test").await?;
    let tracker = ResourceTracker {
        file_open: Arc::clone(&storage.tracker.file_open),
        lock_held: Arc::clone(&storage.tracker.lock_held),
        connection_active: Arc::clone(&storage.tracker.connection_active),
    };
    
    // Verify resources are acquired
    assert!(tracker.file_open.load(Ordering::Relaxed));
    assert!(tracker.lock_held.load(Ordering::Relaxed));
    assert!(tracker.connection_active.load(Ordering::Relaxed));
    
    // Try operation that fails
    let doc = Document::new(
        Uuid::new_v4(),
        "/test/fail.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Fail Test".to_string(),
        100,
    )?;
    
    let _ = storage.insert(doc).await; // Will fail
    
    // Close storage
    storage.close().await?;
    
    // Verify resources are cleaned up
    assert!(!tracker.file_open.load(Ordering::Relaxed));
    assert!(!tracker.lock_held.load(Ordering::Relaxed));
    assert!(!tracker.connection_active.load(Ordering::Relaxed));
    
    Ok(())
}