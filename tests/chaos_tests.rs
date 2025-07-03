// Chaos Testing - Stage 5: Extreme Failure Scenarios
// These tests simulate catastrophic failures and system-wide issues

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

/// Simulate sudden system shutdown during write
#[tokio::test]
async fn test_sudden_shutdown() -> Result<()> {
    use kotadb::*;

    struct ShutdownStorage {
        docs: Arc<Mutex<HashMap<Uuid, Document>>>,
        shutdown_signal: Arc<AtomicBool>,
        writes_before_shutdown: AtomicU64,
    }

    #[async_trait::async_trait]
    impl Storage for ShutdownStorage {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self {
                docs: Arc::new(Mutex::new(HashMap::new())),
                shutdown_signal: Arc::new(AtomicBool::new(false)),
                writes_before_shutdown: AtomicU64::new(10),
            })
        }

        async fn insert(&mut self, doc: Document) -> Result<()> {
            // Check if we should shutdown
            if self.writes_before_shutdown.fetch_sub(1, Ordering::SeqCst) == 0 {
                self.shutdown_signal.store(true, Ordering::Relaxed);
                anyhow::bail!("System shutdown!");
            }

            if self.shutdown_signal.load(Ordering::Relaxed) {
                anyhow::bail!("System is down");
            }

            let mut docs = self.docs.lock().await;
            docs.insert(doc.id, doc);
            Ok(())
        }

        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            if self.shutdown_signal.load(Ordering::Relaxed) {
                anyhow::bail!("System is down");
            }
            let docs = self.docs.lock().await;
            Ok(docs.get(id).cloned())
        }

        async fn update(&mut self, _doc: Document) -> Result<()> {
            anyhow::bail!("System is down")
        }

        async fn delete(&mut self, _id: &Uuid) -> Result<()> {
            anyhow::bail!("System is down")
        }

        async fn sync(&mut self) -> Result<()> {
            anyhow::bail!("System is down")
        }

        async fn close(self) -> Result<()> {
            Ok(())
        }
    }

    let mut storage = ShutdownStorage::open("/test").await?;
    let mut successful_writes = 0;

    // Try to write 20 documents, but system will shutdown after 10
    for i in 0..20 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024,
            1000,
            2000,
            format!("Doc {}", i),
            100,
        )?;

        match storage.insert(doc).await {
            Ok(_) => successful_writes += 1,
            Err(e) => {
                assert!(e.to_string().contains("shutdown"));
                break;
            }
        }
    }

    assert_eq!(successful_writes, 10);
    Ok(())
}

/// Simulate network partition in distributed scenario
#[tokio::test]
async fn test_network_partition() -> Result<()> {
    use kotadb::*;

    struct PartitionedNode {
        id: usize,
        local_data: Arc<Mutex<HashMap<Uuid, Document>>>,
        peers: Arc<Mutex<Vec<Arc<PartitionedNode>>>>,
        partitioned: Arc<AtomicBool>,
    }

    impl PartitionedNode {
        fn new(id: usize) -> Arc<Self> {
            Arc::new(Self {
                id,
                local_data: Arc::new(Mutex::new(HashMap::new())),
                peers: Arc::new(Mutex::new(Vec::new())),
                partitioned: Arc::new(AtomicBool::new(false)),
            })
        }

        async fn replicate(&self, doc: &Document) -> Result<()> {
            if self.partitioned.load(Ordering::Relaxed) {
                anyhow::bail!("Network partitioned");
            }

            let peers = self.peers.lock().await;
            for peer in peers.iter() {
                if !peer.partitioned.load(Ordering::Relaxed) {
                    let mut peer_data = peer.local_data.lock().await;
                    peer_data.insert(doc.id, doc.clone());
                }
            }
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Storage for Arc<PartitionedNode> {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(PartitionedNode::new(0))
        }

        async fn insert(&mut self, doc: Document) -> Result<()> {
            let mut local = self.local_data.lock().await;
            local.insert(doc.id, doc.clone());
            drop(local);

            // Try to replicate
            if let Err(_) = self.replicate(&doc).await {
                // Continue even if replication fails (eventual consistency)
            }
            Ok(())
        }

        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            let local = self.local_data.lock().await;
            Ok(local.get(id).cloned())
        }

        async fn update(&mut self, doc: Document) -> Result<()> {
            let mut local = self.local_data.lock().await;
            local.insert(doc.id, doc.clone());
            drop(local);

            let _ = self.replicate(&doc).await;
            Ok(())
        }

        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            let mut local = self.local_data.lock().await;
            local.remove(id);
            Ok(())
        }

        async fn sync(&mut self) -> Result<()> {
            Ok(())
        }

        async fn close(self) -> Result<()> {
            Ok(())
        }
    }

    // Create 3 nodes
    let node1 = PartitionedNode::new(1);
    let node2 = PartitionedNode::new(2);
    let node3 = PartitionedNode::new(3);

    // Connect nodes
    {
        let mut peers1 = node1.peers.lock().await;
        peers1.push(node2.clone());
        peers1.push(node3.clone());

        let mut peers2 = node2.peers.lock().await;
        peers2.push(node1.clone());
        peers2.push(node3.clone());

        let mut peers3 = node3.peers.lock().await;
        peers3.push(node1.clone());
        peers3.push(node2.clone());
    }

    // Insert document on node1
    let doc = Document::new(
        Uuid::new_v4(),
        "/test/partition.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Partition Test".to_string(),
        100,
    )?;

    let mut storage1 = node1.clone();
    storage1.insert(doc.clone()).await?;

    // Verify replication
    assert!(node2.get(&doc.id).await?.is_some());
    assert!(node3.get(&doc.id).await?.is_some());

    // Simulate partition (node3 isolated)
    node3.partitioned.store(true, Ordering::Relaxed);

    // Insert new document on node1
    let doc2 = Document::new(
        Uuid::new_v4(),
        "/test/partition2.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Partition Test 2".to_string(),
        100,
    )?;

    storage1.insert(doc2.clone()).await?;

    // Node2 should have it, node3 should not
    assert!(node2.get(&doc2.id).await?.is_some());
    assert!(node3.get(&doc2.id).await?.is_none());

    Ok(())
}

/// Simulate resource exhaustion
#[tokio::test]
async fn test_resource_exhaustion() -> Result<()> {
    use kotadb::*;

    struct ResourceLimitedStorage {
        docs: Arc<Mutex<HashMap<Uuid, Document>>>,
        file_handles: Arc<Semaphore>,
        memory_limit_bytes: Arc<AtomicU64>,
        memory_used_bytes: Arc<AtomicU64>,
    }

    #[async_trait::async_trait]
    impl Storage for ResourceLimitedStorage {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self {
                docs: Arc::new(Mutex::new(HashMap::new())),
                file_handles: Arc::new(Semaphore::new(10)), // Max 10 concurrent operations
                memory_limit_bytes: Arc::new(AtomicU64::new(1_000_000)), // 1MB limit
                memory_used_bytes: Arc::new(AtomicU64::new(0)),
            })
        }

        async fn insert(&mut self, doc: Document) -> Result<()> {
            // Try to acquire file handle
            let permit = self
                .file_handles
                .try_acquire()
                .map_err(|_| anyhow::anyhow!("Too many open files"))?;

            // Check memory limit
            let current_mem = self.memory_used_bytes.load(Ordering::Relaxed);
            let limit = self.memory_limit_bytes.load(Ordering::Relaxed);

            if current_mem + doc.size > limit {
                anyhow::bail!("Out of memory");
            }

            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;

            let mut docs = self.docs.lock().await;
            docs.insert(doc.id, doc.clone());
            self.memory_used_bytes
                .fetch_add(doc.size, Ordering::Relaxed);

            drop(permit); // Release file handle
            Ok(())
        }

        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            let _permit = self
                .file_handles
                .try_acquire()
                .map_err(|_| anyhow::anyhow!("Too many open files"))?;

            let docs = self.docs.lock().await;
            Ok(docs.get(id).cloned())
        }

        async fn update(&mut self, doc: Document) -> Result<()> {
            let _permit = self
                .file_handles
                .try_acquire()
                .map_err(|_| anyhow::anyhow!("Too many open files"))?;

            let mut docs = self.docs.lock().await;
            if let Some(old) = docs.get(&doc.id) {
                let size_diff = doc.size.saturating_sub(old.size);

                let current_mem = self.memory_used_bytes.load(Ordering::Relaxed);
                let limit = self.memory_limit_bytes.load(Ordering::Relaxed);

                if current_mem + size_diff > limit {
                    anyhow::bail!("Out of memory");
                }

                docs.insert(doc.id, doc.clone());
                self.memory_used_bytes
                    .fetch_add(size_diff, Ordering::Relaxed);
                Ok(())
            } else {
                anyhow::bail!("Document not found");
            }
        }

        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            let _permit = self
                .file_handles
                .try_acquire()
                .map_err(|_| anyhow::anyhow!("Too many open files"))?;

            let mut docs = self.docs.lock().await;
            if let Some(doc) = docs.remove(id) {
                self.memory_used_bytes
                    .fetch_sub(doc.size, Ordering::Relaxed);
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

    let storage = Arc::new(Mutex::new(ResourceLimitedStorage::open("/test").await?));
    let mut handles = vec![];

    // Spawn many concurrent operations to exhaust file handles
    for i in 0..20 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let doc = Document::new(
                Uuid::new_v4(),
                format!("/test/{}.md", i),
                [0u8; 32],
                50_000, // 50KB each
                1000,
                2000,
                format!("Doc {}", i),
                100,
            )
            .unwrap();

            let mut s = storage_clone.lock().await;
            s.insert(doc).await
        });
        handles.push(handle);
    }

    let mut successes = 0;
    let mut file_handle_errors = 0;
    let mut memory_errors = 0;

    for handle in handles {
        match handle.await? {
            Ok(_) => successes += 1,
            Err(e) => {
                if e.to_string().contains("Too many open files") {
                    file_handle_errors += 1;
                } else if e.to_string().contains("Out of memory") {
                    memory_errors += 1;
                }
            }
        }
    }

    println!(
        "Successes: {}, File handle errors: {}, Memory errors: {}",
        successes, file_handle_errors, memory_errors
    );

    // Should have some of each type of error
    assert!(file_handle_errors > 0 || memory_errors > 0);

    Ok(())
}

/// Simulate cascading failures
#[tokio::test]
async fn test_cascading_failure() -> Result<()> {
    use kotadb::*;

    struct CascadingSystem {
        primary: Arc<AtomicBool>,
        secondary: Arc<AtomicBool>,
        tertiary: Arc<AtomicBool>,
        failure_count: Arc<AtomicU64>,
    }

    impl CascadingSystem {
        fn new() -> Self {
            Self {
                primary: Arc::new(AtomicBool::new(true)),
                secondary: Arc::new(AtomicBool::new(true)),
                tertiary: Arc::new(AtomicBool::new(true)),
                failure_count: Arc::new(AtomicU64::new(0)),
            }
        }

        fn check_health(&self) -> Result<()> {
            if !self.primary.load(Ordering::Relaxed) {
                // Primary failure cascades to secondary
                self.secondary.store(false, Ordering::Relaxed);

                if !self.secondary.load(Ordering::Relaxed) {
                    // Secondary failure cascades to tertiary
                    self.tertiary.store(false, Ordering::Relaxed);
                }

                anyhow::bail!("System failure cascade");
            }
            Ok(())
        }

        fn inject_failure(&self) {
            let count = self.failure_count.fetch_add(1, Ordering::SeqCst);

            // Inject failures at specific counts
            match count {
                5 => self.primary.store(false, Ordering::Relaxed),
                10 => self.secondary.store(false, Ordering::Relaxed),
                15 => self.tertiary.store(false, Ordering::Relaxed),
                _ => {}
            }
        }
    }

    #[async_trait::async_trait]
    impl Storage for CascadingSystem {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self::new())
        }

        async fn insert(&mut self, _doc: Document) -> Result<()> {
            self.inject_failure();
            self.check_health()?;
            Ok(())
        }

        async fn get(&self, _id: &Uuid) -> Result<Option<Document>> {
            self.check_health()?;
            Ok(None)
        }

        async fn update(&mut self, _doc: Document) -> Result<()> {
            self.inject_failure();
            self.check_health()?;
            Ok(())
        }

        async fn delete(&mut self, _id: &Uuid) -> Result<()> {
            self.inject_failure();
            self.check_health()?;
            Ok(())
        }

        async fn sync(&mut self) -> Result<()> {
            self.check_health()?;
            Ok(())
        }

        async fn close(self) -> Result<()> {
            Ok(())
        }
    }

    let mut storage = CascadingSystem::new();
    let mut operations = 0;

    // Perform operations until cascade failure
    for i in 0..20 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024,
            1000,
            2000,
            format!("Doc {}", i),
            100,
        )?;

        match storage.insert(doc).await {
            Ok(_) => operations += 1,
            Err(e) => {
                assert!(e.to_string().contains("System failure cascade"));
                break;
            }
        }
    }

    // Should have failed after primary failure at operation 6
    assert!(operations >= 5 && operations < 10);

    // All subsystems should be down
    assert!(!storage.primary.load(Ordering::Relaxed));
    assert!(!storage.secondary.load(Ordering::Relaxed));

    Ok(())
}

/// Simulate Byzantine failures (inconsistent behavior)
#[tokio::test]
async fn test_byzantine_failures() -> Result<()> {
    use kotadb::*;
    use rand::Rng;

    struct ByzantineStorage {
        docs: Arc<Mutex<HashMap<Uuid, Document>>>,
        corruption_rate: f32,
    }

    impl ByzantineStorage {
        fn corrupt_document(&self, mut doc: Document) -> Document {
            let mut rng = rand::thread_rng();

            // Randomly corrupt different fields
            match rng.gen_range(0..5) {
                0 => doc.size = rng.gen::<u64>(),
                1 => doc.word_count = rng.gen::<u32>(),
                2 => doc.updated = doc.created - 1000, // Invalid timestamp
                3 => doc.title = "".to_string(),       // Empty title
                4 => {
                    // Corrupt hash
                    for i in 0..32 {
                        doc.hash[i] = rng.gen::<u8>();
                    }
                }
                _ => {}
            }

            doc
        }
    }

    #[async_trait::async_trait]
    impl Storage for ByzantineStorage {
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self {
                docs: Arc::new(Mutex::new(HashMap::new())),
                corruption_rate: 0.3,
            })
        }

        async fn insert(&mut self, mut doc: Document) -> Result<()> {
            // Sometimes corrupt the document
            if rand::random::<f32>() < self.corruption_rate {
                doc = self.corrupt_document(doc);
            }

            let mut docs = self.docs.lock().await;
            docs.insert(doc.id, doc);

            // Sometimes claim failure when it succeeded
            if rand::random::<f32>() < 0.1 {
                anyhow::bail!("False failure");
            }

            Ok(())
        }

        async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
            let docs = self.docs.lock().await;

            if let Some(doc) = docs.get(id) {
                // Sometimes return corrupted data
                if rand::random::<f32>() < self.corruption_rate {
                    Ok(Some(self.corrupt_document(doc.clone())))
                } else {
                    Ok(Some(doc.clone()))
                }
            } else {
                // Sometimes claim document exists when it doesn't
                if rand::random::<f32>() < 0.1 {
                    Ok(Some(Document::new(
                        *id,
                        "/fake/doc.md".to_string(),
                        [0u8; 32],
                        1024,
                        1000,
                        2000,
                        "Fake Doc".to_string(),
                        100,
                    )?))
                } else {
                    Ok(None)
                }
            }
        }

        async fn update(&mut self, doc: Document) -> Result<()> {
            // Sometimes update the wrong document
            let mut target_id = doc.id;
            if rand::random::<f32>() < 0.1 {
                target_id = Uuid::new_v4();
            }

            let mut docs = self.docs.lock().await;
            docs.insert(target_id, doc);
            Ok(())
        }

        async fn delete(&mut self, id: &Uuid) -> Result<()> {
            let mut docs = self.docs.lock().await;

            // Sometimes delete a random document instead
            if rand::random::<f32>() < 0.1 && !docs.is_empty() {
                let random_key = docs.keys().next().cloned().unwrap();
                docs.remove(&random_key);
            } else {
                docs.remove(id);
            }

            Ok(())
        }

        async fn sync(&mut self) -> Result<()> {
            // Sometimes don't actually sync
            if rand::random::<f32>() < 0.5 {
                return Ok(());
            }

            // Simulate actual sync
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(())
        }

        async fn close(self) -> Result<()> {
            // Sometimes fail to close properly
            if rand::random::<f32>() < 0.2 {
                anyhow::bail!("Failed to close");
            }
            Ok(())
        }
    }

    let mut storage = ByzantineStorage::open("/test").await?;
    let mut inconsistencies = 0;

    // Insert documents and check for Byzantine behavior
    for i in 0..10 {
        let doc = Document::new(
            Uuid::new_v4(),
            format!("/test/{}.md", i),
            [0u8; 32],
            1024,
            1000,
            2000,
            format!("Doc {}", i),
            100,
        )?;

        let doc_id = doc.id;

        // Insert document
        let _ = storage.insert(doc.clone()).await;

        // Try to retrieve it
        if let Ok(Some(retrieved)) = storage.get(&doc_id).await {
            // Check for corruption
            if retrieved.updated < retrieved.created
                || retrieved.title.is_empty()
                || retrieved.id != doc_id
            {
                inconsistencies += 1;
            }
        }
    }

    // Should have detected some Byzantine behavior
    assert!(inconsistencies > 0);

    Ok(())
}
