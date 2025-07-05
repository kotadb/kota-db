// Tests for Wrapper Components - Stage 6
// These tests ensure that our wrappers correctly apply automatic best practices

use anyhow::Result;
use async_trait::async_trait;
use kotadb::types::{ValidatedDocumentId, ValidatedPath, ValidatedTag, ValidatedTitle};
use kotadb::wrappers::*;
use kotadb::{Document, DocumentBuilder, Index, Query, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// Test helper functions
#[cfg(test)]
mod test_helpers {
    use super::*;

    pub fn test_doc_id() -> ValidatedDocumentId {
        ValidatedDocumentId::new()
    }

    pub fn test_path(path: &str) -> ValidatedPath {
        ValidatedPath::new(path).expect("Test path should be valid")
    }

    pub fn test_title(title: &str) -> ValidatedTitle {
        ValidatedTitle::new(title).expect("Test title should be valid")
    }

    pub fn test_document() -> Document {
        DocumentBuilder::new()
            .path("/test/doc.md")
            .unwrap()
            .title("Test Document")
            .unwrap()
            .content(b"Test content".to_vec())
            .build()
            .expect("Test document should build")
    }
}

use test_helpers::*;

// Mock storage implementation for testing
#[derive(Clone)]
struct MockStorage {
    docs: Arc<Mutex<HashMap<ValidatedDocumentId, Document>>>,
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
        self.call_count
            .lock()
            .await
            .get(method)
            .copied()
            .unwrap_or(0)
    }

    async fn increment_call(&self, method: &str) {
        let mut counts = self.call_count.lock().await;
        *counts.entry(method.to_string()).or_insert(0) += 1;
    }
}

#[async_trait]
impl Storage for MockStorage {
    async fn open(_path: &str) -> Result<Self>
    where
        Self: Sized,
    {
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

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
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

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        self.increment_call("delete").await;
        Ok(self.docs.lock().await.remove(id).is_some())
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        self.increment_call("list_all").await;
        Ok(self.docs.lock().await.values().cloned().collect())
    }

    async fn sync(&mut self) -> Result<()> {
        self.increment_call("sync").await;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.increment_call("flush").await;
        Ok(())
    }

    async fn close(self) -> Result<()> {
        Ok(())
    }
}

fn create_test_doc() -> Document {
    test_document()
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

    // Invalid document should fail - try to create one with empty path
    // The ValidatedPath::new() should fail with empty path, preventing invalid document creation
    let invalid_path_result = ValidatedPath::new("");
    assert!(invalid_path_result.is_err());

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
    updated_doc.updated_at = chrono::Utc::now();
    updated_doc.title = ValidatedTitle::new("Updated Title").unwrap();
    validated.update(updated_doc.clone()).await?;

    // Update with changed created timestamp should fail
    let mut invalid_update = updated_doc.clone();
    invalid_update.created_at = chrono::DateTime::from_timestamp(500, 0).unwrap().into();
    let result = validated.update(invalid_update).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Created timestamp cannot change"));

    // Update with earlier timestamp should fail
    let mut invalid_update = updated_doc.clone();
    invalid_update.updated_at = chrono::DateTime::from_timestamp(1500, 0).unwrap().into(); // Earlier than current
    let result = validated.update(invalid_update).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Updated timestamp must increase"));

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
    let mut retryable = RetryableStorage::new(base.clone()).with_retry_config(
        3,
        std::time::Duration::from_millis(10),
        std::time::Duration::from_millis(100),
    );

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
        async fn open(_path: &str) -> Result<Self>
        where
            Self: Sized,
        {
            Ok(Self)
        }

        async fn insert(&mut self, _doc: Document) -> Result<()> {
            anyhow::bail!("Always fails")
        }

        async fn get(&self, _id: &ValidatedDocumentId) -> Result<Option<Document>> {
            anyhow::bail!("Always fails")
        }

        async fn update(&mut self, _doc: Document) -> Result<()> {
            anyhow::bail!("Always fails")
        }

        async fn delete(&mut self, _id: &ValidatedDocumentId) -> Result<bool> {
            anyhow::bail!("Always fails")
        }

        async fn list_all(&self) -> Result<Vec<Document>> {
            anyhow::bail!("Always fails")
        }

        async fn flush(&mut self) -> Result<()> {
            anyhow::bail!("Always fails")
        }

        async fn sync(&mut self) -> Result<()> {
            anyhow::bail!("Always fails")
        }

        async fn close(self) -> Result<()> {
            Ok(())
        }
    }

    let mut retryable = RetryableStorage::new(AlwaysFailStorage).with_retry_config(
        2,
        std::time::Duration::from_millis(10),
        std::time::Duration::from_millis(50),
    );

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
    updated_doc.title = ValidatedTitle::new("Updated Title").unwrap();
    updated_doc.updated_at = chrono::Utc::now();
    cached.update(updated_doc.clone()).await?;

    // Get should return updated version from cache
    let retrieved = cached.get(&doc.id).await?.unwrap();
    assert_eq!(retrieved.title.as_str(), "Updated Title");

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

// TODO: Implement SafeTransaction test when Transaction trait is implemented as a concrete type
#[tokio::test]
async fn test_safe_transaction() -> Result<()> {
    // SafeTransaction is currently commented out in wrappers.rs
    // because Transaction is a trait, not a concrete type.
    // This test will be enabled when we have a concrete Transaction implementation.

    // For now, just test that we can create transaction-like IDs
    let tx_id = 1u64;
    assert!(tx_id > 0);

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
    entries: Arc<Mutex<HashMap<ValidatedDocumentId, ValidatedPath>>>,
}

impl MockIndex {
    fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Index for MockIndex {
    async fn open(_path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new())
    }

    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        self.entries.lock().await.insert(id, path);
        Ok(())
    }

    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        self.entries.lock().await.insert(id, path);
        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        Ok(self.entries.lock().await.remove(id).is_some())
    }

    async fn search(&self, _query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        // Simple mock - return all IDs
        Ok(self.entries.lock().await.keys().cloned().collect())
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
async fn test_metered_index() -> Result<()> {
    let base_index = MockIndex::new();
    let mut metered = MeteredIndex::new(base_index, "test_index".to_string());

    // Perform some operations
    let id1 = test_doc_id();
    let id2 = test_doc_id();
    let path1 = test_path("/test/doc1.md");
    let path2 = test_path("/test/doc2.md");

    metered.insert(id1, path1).await?;
    metered.insert(id2, path2).await?;
    metered.delete(&id1).await?;

    let query = Query::new(Some("test search".to_string()), None, None, 10)?;
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
