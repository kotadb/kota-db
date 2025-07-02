// Tests for Wrapper Components - Stage 6
// These tests ensure that our wrappers correctly apply automatic best practices

use kotadb::wrappers::*;
use kotadb::{Storage, Index, Document, Query};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;

// Mock storage implementation for testing
#[derive(Clone)]
struct MockStorage {
    docs: Arc<Mutex<HashMap<Uuid, Document>>>,
    call_count: Arc<Mutex<HashMap<String, usize>>>,
    fail_next: Arc<Mutex<bool>>,
}

impl MockStorage {
    fn new() -> Self {
        Self {
            docs: Arc::new(Mutex::new(HashMap::new())),
            call_count: Arc::new(Mutex::new(HashMap::new())),
            fail_next: Arc::new(Mutex::new(false)),
        }
    }
    
    async fn set_fail_next(&self, fail: bool) {
        *self.fail_next.lock().await = fail;
    }
    
    async fn get_call_count(&self, method: &str) -> usize {
        self.call_count.lock().await.get(method).copied().unwrap_or(0)
    }
    
    async fn increment_call(&self, method: &str) {
        let mut counts = self.call_count.lock().await;
        *counts.entry(method.to_string()).or_insert(0) += 1;
    }
}

#[async_trait]
impl Storage for MockStorage {
    async fn open(_path: &str) -> Result<Self> where Self: Sized {
        Ok(Self::new())
    }
    
    async fn insert(&mut self, doc: Document) -> Result<()> {
        self.increment_call("insert").await;
        
        if *self.fail_next.lock().await {
            *self.fail_next.lock().await = false;
            anyhow::bail!("Simulated failure");
        }
        
        self.docs.lock().await.insert(doc.id, doc);
        Ok(())
    }
    
    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        self.increment_call("get").await;
        Ok(self.docs.lock().await.get(id).cloned())
    }
    
    async fn update(&mut self, doc: Document) -> Result<()> {
        self.increment_call("update").await;
        
        if !self.docs.lock().await.contains_key(&doc.id) {
            anyhow::bail!("Document not found");
        }
        
        self.docs.lock().await.insert(doc.id, doc);
        Ok(())
    }
    
    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.increment_call("delete").await;
        self.docs.lock().await.remove(id);
        Ok(())
    }
    
    async fn sync(&mut self) -> Result<()> {
        self.increment_call("sync").await;
        Ok(())
    }
    
    async fn close(self) -> Result<()> {
        Ok(())
    }
}

fn create_test_doc() -> Document {
    Document::new(
        Uuid::new_v4(),
        "/test/doc.md".to_string(),
        [0u8; 32],
        1024,
        1000,
        2000,
        "Test Document".to_string(),
        100,
    ).unwrap()
}

#[tokio::test]
async fn test_traced_storage_operations() -> Result<()> {
    let base = MockStorage::new();
    let mut traced = TracedStorage::new(base.clone());
    
    // Test that operations are traced
    let doc = create_test_doc();
    traced.insert(doc.clone()).await?;
    
    // Operation count should increment
    assert_eq!(traced.operation_count().await, 1);
    
    // Get doesn't increment operation count
    let retrieved = traced.get(&doc.id).await?;
    assert!(retrieved.is_some());
    assert_eq!(traced.operation_count().await, 1);
    
    // Update increments
    traced.update(doc.clone()).await?;
    assert_eq!(traced.operation_count().await, 2);
    
    // Delete increments
    traced.delete(&doc.id).await?;
    assert_eq!(traced.operation_count().await, 3);
    
    Ok(())
}

#[tokio::test]
async fn test_validated_storage_insert_validation() -> Result<()> {
    let base = MockStorage::new();
    let mut validated = ValidatedStorage::new(base);
    
    // Valid document should pass
    let doc = create_test_doc();
    validated.insert(doc.clone()).await?;
    
    // Duplicate ID should fail
    let result = validated.insert(doc.clone()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
    
    // Invalid document should fail
    let invalid_doc = Document {
        id: Uuid::new_v4(),
        path: "".to_string(), // Invalid empty path
        hash: [0u8; 32],
        size: 1024,
        created: 1000,
        updated: 2000,
        title: "Test".to_string(),
        word_count: 100,
    };
    
    let result = validated.insert(invalid_doc).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_validated_storage_update_validation() -> Result<()> {
    let base = MockStorage::new();
    let mut validated = ValidatedStorage::new(base);
    
    // Insert a document
    let doc = create_test_doc();
    validated.insert(doc.clone()).await?;
    
    // Valid update should pass
    let mut updated_doc = doc.clone();
    updated_doc.updated = 3000;
    updated_doc.title = "Updated Title".to_string();
    validated.update(updated_doc.clone()).await?;
    
    // Update with changed created timestamp should fail
    let mut invalid_update = updated_doc.clone();
    invalid_update.created = 500;
    let result = validated.update(invalid_update).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Created timestamp cannot change"));
    
    // Update with earlier timestamp should fail
    let mut invalid_update = updated_doc.clone();
    invalid_update.updated = 1500; // Earlier than current
    let result = validated.update(invalid_update).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Updated timestamp must increase"));
    
    Ok(())
}

#[tokio::test]
async fn test_retryable_storage_success() -> Result<()> {
    let base = MockStorage::new();
    let mut retryable = RetryableStorage::new(base.clone());
    
    // Successful operation should not retry
    let doc = create_test_doc();
    retryable.insert(doc.clone()).await?;
    
    assert_eq!(base.get_call_count("insert").await, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_retryable_storage_transient_failure() -> Result<()> {
    let base = MockStorage::new();
    let mut retryable = RetryableStorage::new(base.clone())
        .with_retry_config(3, std::time::Duration::from_millis(10), std::time::Duration::from_millis(100));
    
    // Set up to fail first attempt
    base.set_fail_next(true).await;
    
    // Should succeed after retry
    let doc = create_test_doc();
    retryable.insert(doc.clone()).await?;
    
    // Should have been called twice (1 failure + 1 success)
    assert_eq!(base.get_call_count("insert").await, 2);
    
    Ok(())
}

#[tokio::test]
async fn test_retryable_storage_permanent_failure() -> Result<()> {
    // Create a storage that always fails
    struct AlwaysFailStorage;
    
    #[async_trait]
    impl Storage for AlwaysFailStorage {
        async fn open(_path: &str) -> Result<Self> where Self: Sized {
            Ok(Self)
        }
        
        async fn insert(&mut self, _doc: Document) -> Result<()> {
            anyhow::bail!("Always fails")
        }
        
        async fn get(&self, _id: &Uuid) -> Result<Option<Document>> {
            anyhow::bail!("Always fails")
        }
        
        async fn update(&mut self, _doc: Document) -> Result<()> {
            anyhow::bail!("Always fails")
        }
        
        async fn delete(&mut self, _id: &Uuid) -> Result<()> {
            anyhow::bail!("Always fails")
        }
        
        async fn sync(&mut self) -> Result<()> {
            anyhow::bail!("Always fails")
        }
        
        async fn close(self) -> Result<()> {
            Ok(())
        }
    }
    
    let mut retryable = RetryableStorage::new(AlwaysFailStorage)
        .with_retry_config(2, std::time::Duration::from_millis(10), std::time::Duration::from_millis(50));
    
    let doc = create_test_doc();
    let result = retryable.insert(doc).await;
    
    // Should fail after max retries
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Always fails"));
    
    Ok(())
}

#[tokio::test]
async fn test_cached_storage_hit_miss() -> Result<()> {
    let base = MockStorage::new();
    let mut cached = CachedStorage::new(base.clone(), 10);
    
    let doc = create_test_doc();
    cached.insert(doc.clone()).await?;
    
    // First get - cache miss (needs to fetch from storage)
    let retrieved = cached.get(&doc.id).await?;
    assert!(retrieved.is_some());
    let (hits, misses) = cached.cache_stats().await;
    assert_eq!(hits, 0);
    assert_eq!(misses, 1);
    
    // Second get - cache hit
    let retrieved = cached.get(&doc.id).await?;
    assert!(retrieved.is_some());
    let (hits, misses) = cached.cache_stats().await;
    assert_eq!(hits, 1);
    assert_eq!(misses, 1);
    
    // Third get - another cache hit
    let retrieved = cached.get(&doc.id).await?;
    assert!(retrieved.is_some());
    let (hits, misses) = cached.cache_stats().await;
    assert_eq!(hits, 2);
    assert_eq!(misses, 1);
    
    // Base storage should only have been called once
    assert_eq!(base.get_call_count("get").await, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_cached_storage_update_invalidation() -> Result<()> {
    let base = MockStorage::new();
    let mut cached = CachedStorage::new(base, 10);
    
    let doc = create_test_doc();
    cached.insert(doc.clone()).await?;
    
    // Cache the document
    let _ = cached.get(&doc.id).await?;
    
    // Update the document
    let mut updated_doc = doc.clone();
    updated_doc.title = "Updated Title".to_string();
    updated_doc.updated = 3000;
    cached.update(updated_doc.clone()).await?;
    
    // Get should return updated version from cache
    let retrieved = cached.get(&doc.id).await?.unwrap();
    assert_eq!(retrieved.title, "Updated Title");
    
    // Should be a cache hit
    let (hits, _) = cached.cache_stats().await;
    assert!(hits > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_cached_storage_delete_invalidation() -> Result<()> {
    let base = MockStorage::new();
    let mut cached = CachedStorage::new(base, 10);
    
    let doc = create_test_doc();
    cached.insert(doc.clone()).await?;
    
    // Cache the document
    let _ = cached.get(&doc.id).await?;
    
    // Delete the document
    cached.delete(&doc.id).await?;
    
    // Get should return None
    let retrieved = cached.get(&doc.id).await?;
    assert!(retrieved.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_safe_transaction() -> Result<()> {
    // Test automatic commit
    let mut tx = SafeTransaction::begin(1)?;
    tx.add_operation(kotadb::Operation::StorageWrite {
        doc_id: Uuid::new_v4(),
        size_bytes: 1024,
    });
    assert_eq!(tx.id(), 1);
    tx.commit().await?;
    
    // Test automatic rollback on drop
    {
        let mut tx = SafeTransaction::begin(2)?;
        tx.add_operation(kotadb::Operation::StorageWrite {
            doc_id: Uuid::new_v4(),
            size_bytes: 1024,
        });
        // Transaction dropped without commit - should log warning
    }
    
    Ok(())
}

#[tokio::test]
async fn test_composed_wrappers() -> Result<()> {
    // Test that wrappers can be composed together
    let base = MockStorage::new();
    let cached = CachedStorage::new(base, 10);
    let retryable = RetryableStorage::new(cached);
    let validated = ValidatedStorage::new(retryable);
    let mut traced = TracedStorage::new(validated);
    
    // All wrappers should work together
    let doc = create_test_doc();
    traced.insert(doc.clone()).await?;
    
    // Traced wrapper tracks operations
    assert_eq!(traced.operation_count().await, 1);
    
    // Can retrieve through all layers
    let retrieved = traced.get(&doc.id).await?;
    assert!(retrieved.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_create_wrapped_storage() -> Result<()> {
    let base = MockStorage::new();
    let mut wrapped = create_wrapped_storage(base, 100).await;
    
    // Should have all wrapper functionality
    let doc = create_test_doc();
    wrapped.insert(doc.clone()).await?;
    
    // Tracing works
    assert_eq!(wrapped.operation_count().await, 1);
    
    // Can retrieve
    let retrieved = wrapped.get(&doc.id).await?;
    assert!(retrieved.is_some());
    
    Ok(())
}

// Mock index for testing
struct MockIndex {
    entries: Arc<Mutex<HashMap<String, Vec<Uuid>>>>,
}

#[async_trait]
impl Index for MockIndex {
    type Key = String;
    type Value = Uuid;
    
    async fn insert(&mut self, key: Self::Key, value: Self::Value) -> Result<()> {
        let mut entries = self.entries.lock().await;
        entries.entry(key).or_insert_with(Vec::new).push(value);
        Ok(())
    }
    
    async fn delete(&mut self, key: &Self::Key) -> Result<()> {
        self.entries.lock().await.remove(key);
        Ok(())
    }
    
    async fn search(&self, query: &Query) -> Result<Vec<Self::Value>> {
        if let Some(text) = &query.text {
            let entries = self.entries.lock().await;
            Ok(entries.get(text).cloned().unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_metered_index() -> Result<()> {
    let base_index = MockIndex {
        entries: Arc::new(Mutex::new(HashMap::new())),
    };
    let mut metered = MeteredIndex::new(base_index, "test_index".to_string());
    
    // Perform some operations
    metered.insert("key1".to_string(), Uuid::new_v4()).await?;
    metered.insert("key2".to_string(), Uuid::new_v4()).await?;
    metered.delete(&"key1".to_string()).await?;
    
    let query = Query::new(Some("key2".to_string()), None, None, 10)?;
    let _ = metered.search(&query).await?;
    
    metered.flush().await?;
    
    // Check timing statistics
    let stats = metered.timing_stats().await;
    assert!(stats.contains_key("insert"));
    assert!(stats.contains_key("delete"));
    assert!(stats.contains_key("search"));
    assert!(stats.contains_key("flush"));
    
    // Each operation should have min, avg, max timings
    for (_op, (min, avg, max)) in stats {
        assert!(min <= avg);
        assert!(avg <= max);
    }
    
    Ok(())
}