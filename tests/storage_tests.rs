// Storage Engine Tests - Stage 1: TDD
// These tests define the expected behavior of the storage engine
// Written BEFORE implementation following 6-stage risk reduction

use anyhow::Result;
use kotadb::*;
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

// Test helper to create a temporary database directory
fn temp_db() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_string_lossy().to_string();
    (dir, path)
}

#[tokio::test]
async fn test_storage_initialization() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    
    // Storage should initialize successfully
    let storage = Storage::open(&path).await?;
    
    // Should create necessary directories
    assert!(Path::new(&path).join("meta.db").exists());
    assert!(Path::new(&path).join("indices").exists());
    assert!(Path::new(&path).join("wal").exists());
    
    // Should be able to close and reopen
    storage.close().await?;
    let storage2 = Storage::open(&path).await?;
    storage2.close().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_document_insert_and_retrieve() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    // Create test document
    let doc_id = Uuid::new_v4();
    let doc = Document {
        id: doc_id,
        path: "/test/doc.md".to_string(),
        hash: [0u8; 32], // SHA-256
        size: 1234,
        created: 1234567890,
        updated: 1234567890,
        title: "Test Document".to_string(),
        word_count: 100,
    };
    
    // Insert should succeed
    storage.insert(doc.clone()).await?;
    
    // Retrieve should return the same document
    let retrieved = storage.get(&doc_id).await?;
    assert_eq!(retrieved.unwrap().id, doc_id);
    assert_eq!(retrieved.unwrap().title, "Test Document");
    
    // Non-existent document should return None
    let missing = storage.get(&Uuid::new_v4()).await?;
    assert!(missing.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_document_update() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    let doc_id = Uuid::new_v4();
    let mut doc = Document {
        id: doc_id,
        path: "/test/doc.md".to_string(),
        hash: [0u8; 32],
        size: 1234,
        created: 1234567890,
        updated: 1234567890,
        title: "Original Title".to_string(),
        word_count: 100,
    };
    
    // Insert original
    storage.insert(doc.clone()).await?;
    
    // Update document
    doc.title = "Updated Title".to_string();
    doc.updated = 1234567899;
    storage.update(doc.clone()).await?;
    
    // Retrieve should return updated version
    let retrieved = storage.get(&doc_id).await?.unwrap();
    assert_eq!(retrieved.title, "Updated Title");
    assert_eq!(retrieved.updated, 1234567899);
    
    Ok(())
}

#[tokio::test]
async fn test_document_delete() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    let doc_id = Uuid::new_v4();
    let doc = Document {
        id: doc_id,
        path: "/test/doc.md".to_string(),
        hash: [0u8; 32],
        size: 1234,
        created: 1234567890,
        updated: 1234567890,
        title: "To Delete".to_string(),
        word_count: 100,
    };
    
    // Insert then delete
    storage.insert(doc).await?;
    assert!(storage.get(&doc_id).await?.is_some());
    
    storage.delete(&doc_id).await?;
    assert!(storage.get(&doc_id).await?.is_none());
    
    // Delete non-existent should not error
    storage.delete(&Uuid::new_v4()).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_persistence_across_restarts() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    
    let doc_id = Uuid::new_v4();
    let doc = Document {
        id: doc_id,
        path: "/test/persist.md".to_string(),
        hash: [1u8; 32],
        size: 5678,
        created: 1234567890,
        updated: 1234567890,
        title: "Persistent Document".to_string(),
        word_count: 200,
    };
    
    // Insert and close
    {
        let mut storage = Storage::open(&path).await?;
        storage.insert(doc.clone()).await?;
        storage.sync().await?; // Force flush to disk
        storage.close().await?;
    }
    
    // Reopen and verify
    {
        let storage = Storage::open(&path).await?;
        let retrieved = storage.get(&doc_id).await?.unwrap();
        assert_eq!(retrieved.title, "Persistent Document");
        assert_eq!(retrieved.size, 5678);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_read_write_safety() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let storage = Arc::new(Mutex::new(Storage::open(&path).await?));
    
    // Spawn multiple concurrent operations
    let mut handles = vec![];
    
    // Writers
    for i in 0..5 {
        let storage = storage.clone();
        let handle = tokio::spawn(async move {
            let doc = Document {
                id: Uuid::new_v4(),
                path: format!("/test/concurrent_{}.md", i),
                hash: [i as u8; 32],
                size: i as u64 * 1000,
                created: 1234567890 + i,
                updated: 1234567890 + i,
                title: format!("Concurrent Doc {}", i),
                word_count: i as u32 * 10,
            };
            
            let mut storage = storage.lock().await;
            storage.insert(doc).await
        });
        handles.push(handle);
    }
    
    // Readers
    for _ in 0..5 {
        let storage = storage.clone();
        let handle = tokio::spawn(async move {
            let storage = storage.lock().await;
            storage.list_all().await
        });
        handles.push(handle);
    }
    
    // All operations should complete without error
    for handle in handles {
        handle.await?.unwrap();
    }
    
    Ok(())
}

#[tokio::test]
async fn test_wal_crash_recovery() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    
    let doc_id = Uuid::new_v4();
    let doc = Document {
        id: doc_id,
        path: "/test/wal.md".to_string(),
        hash: [2u8; 32],
        size: 9999,
        created: 1234567890,
        updated: 1234567890,
        title: "WAL Test Document".to_string(),
        word_count: 500,
    };
    
    // Simulate crash during write
    {
        let mut storage = Storage::open(&path).await?;
        storage.begin_transaction().await?;
        storage.insert(doc.clone()).await?;
        // Don't commit - simulate crash
        // storage.commit_transaction().await?;
        drop(storage); // Force drop without proper close
    }
    
    // Reopen - should recover from WAL
    {
        let storage = Storage::open(&path).await?;
        // Document should either be fully present or fully absent
        // (depending on whether WAL was flushed)
        let result = storage.get(&doc_id).await?;
        if let Some(retrieved) = result {
            // If present, must be complete
            assert_eq!(retrieved.title, "WAL Test Document");
            assert_eq!(retrieved.size, 9999);
        }
        // If absent, that's also valid (transaction not committed)
    }
    
    Ok(())
}

#[tokio::test]
async fn test_storage_metrics() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    // Get initial metrics
    let metrics1 = storage.get_metrics().await?;
    assert_eq!(metrics1.document_count, 0);
    assert_eq!(metrics1.total_size_bytes, 0);
    
    // Insert some documents
    for i in 0..3 {
        let doc = Document {
            id: Uuid::new_v4(),
            path: format!("/test/metric_{}.md", i),
            hash: [i as u8; 32],
            size: 1000 * (i + 1) as u64,
            created: 1234567890,
            updated: 1234567890,
            title: format!("Metric Doc {}", i),
            word_count: 100,
        };
        storage.insert(doc).await?;
    }
    
    // Check updated metrics
    let metrics2 = storage.get_metrics().await?;
    assert_eq!(metrics2.document_count, 3);
    assert_eq!(metrics2.total_size_bytes, 6000); // 1000 + 2000 + 3000
    assert!(metrics2.index_sizes.contains_key("primary"));
    
    Ok(())
}

#[tokio::test]
async fn test_page_allocation() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    // Test internal page allocation
    let page1 = storage.allocate_page().await?;
    let page2 = storage.allocate_page().await?;
    let page3 = storage.allocate_page().await?;
    
    // Pages should be unique
    assert_ne!(page1, page2);
    assert_ne!(page2, page3);
    assert_ne!(page1, page3);
    
    // Free a page
    storage.free_page(page2).await?;
    
    // Next allocation might reuse freed page
    let page4 = storage.allocate_page().await?;
    // page4 could be page2 (reused) or a new page
    
    Ok(())
}

#[tokio::test]
async fn test_memory_mapped_performance() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_db();
    let mut storage = Storage::open(&path).await?;
    
    // Insert a "hot" document that should be memory-mapped
    let doc_id = Uuid::new_v4();
    let doc = Document {
        id: doc_id,
        path: "/test/hot.md".to_string(),
        hash: [3u8; 32],
        size: 10000,
        created: 1234567890,
        updated: 1234567890,
        title: "Hot Document".to_string(),
        word_count: 1000,
    };
    
    storage.insert(doc).await?;
    storage.mark_hot(&doc_id).await?;
    
    // Multiple rapid reads should be fast
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = storage.get(&doc_id).await?;
    }
    let elapsed = start.elapsed();
    
    // Should complete in under 100ms (0.1ms per read)
    assert!(elapsed.as_millis() < 100, "Memory-mapped reads too slow: {:?}", elapsed);
    
    Ok(())
}

// Helper types that will be implemented later
// These are here to make the tests compile

use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
struct Document {
    id: Uuid,
    path: String,
    hash: [u8; 32],
    size: u64,
    created: i64,
    updated: i64,
    title: String,
    word_count: u32,
}

struct Storage;

impl Storage {
    async fn open(_path: &str) -> Result<Self> {
        todo!("Implement after contracts are defined")
    }
    
    async fn close(self) -> Result<()> {
        todo!()
    }
    
    async fn insert(&mut self, _doc: Document) -> Result<()> {
        todo!()
    }
    
    async fn get(&self, _id: &Uuid) -> Result<Option<Document>> {
        todo!()
    }
    
    async fn update(&mut self, _doc: Document) -> Result<()> {
        todo!()
    }
    
    async fn delete(&mut self, _id: &Uuid) -> Result<()> {
        todo!()
    }
    
    async fn sync(&mut self) -> Result<()> {
        todo!()
    }
    
    async fn list_all(&self) -> Result<Vec<Document>> {
        todo!()
    }
    
    async fn begin_transaction(&mut self) -> Result<()> {
        todo!()
    }
    
    async fn get_metrics(&self) -> Result<StorageMetrics> {
        todo!()
    }
    
    async fn allocate_page(&mut self) -> Result<PageId> {
        todo!()
    }
    
    async fn free_page(&mut self, _page: PageId) -> Result<()> {
        todo!()
    }
    
    async fn mark_hot(&mut self, _id: &Uuid) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
struct StorageMetrics {
    document_count: usize,
    total_size_bytes: u64,
    index_sizes: std::collections::HashMap<String, usize>,
}

type PageId = u64;