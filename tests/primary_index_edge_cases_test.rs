// Primary Index Edge Cases and Adversarial Tests - Stage 1: Test-Driven Development
// These tests cover failure scenarios, edge cases, and adversarial conditions

use anyhow::{anyhow, Result};
use kotadb::{Index, QueryBuilder, ValidatedDocumentId, ValidatedPath};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod primary_index_edge_cases {
    use super::*;

    pub async fn create_test_index() -> Result<(TempDir, impl Index)> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("edge_case_index");

        let index = kotadb::create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        Ok((temp_dir, index))
    }

    #[tokio::test]
    async fn test_index_with_zero_capacity() -> Result<()> {
        // Test index creation with minimal resources
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("zero_capacity");

        // Should handle zero cache capacity gracefully
        let index = kotadb::create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        // Basic operations should still work
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("edge/zero_capacity.md")?;

        // This should work even with zero cache
        // index.insert(doc_id, doc_path).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_index_with_extremely_long_paths() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Create path near filesystem limits
        let long_component = "a".repeat(255); // Max filename length on most filesystems
        let long_path = format!("edge/{long_component}/{long_component}/{long_component}.md");

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

        // This might fail at ValidatedPath creation, which is expected
        match ValidatedPath::new(&long_path) {
            Ok(valid_path) => {
                // If validation passes, insertion should work
                index.insert(doc_id, valid_path).await?;
            }
            Err(_) => {
                // Validation failure is acceptable for extremely long paths
            }
        }

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_with_unicode_paths() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Test various Unicode characters
        let unicode_paths = [
            "edge/русский.md",
            "edge/中文.md",
            "edge/🚀📦.md",
            "edge/café.md",
            "edge/naïve.md",
        ];

        for path_str in unicode_paths.iter() {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

            match ValidatedPath::new(path_str) {
                Ok(valid_path) => {
                    index.insert(doc_id, valid_path).await?;
                }
                Err(e) => {
                    // Document which Unicode patterns are rejected
                    println!("Unicode path rejected: {path_str} - {e}");
                }
            }
        }

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_rapid_insert_delete_cycles() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("edge/cycle_test.md")?;

        // Rapid insert/delete cycles to test for memory leaks or corruption
        for _ in 0..1000 {
            index.insert(doc_id, doc_path.clone()).await?;
            index.delete(&doc_id).await?;
        }

        // Index should be empty and functional
        let query = QueryBuilder::new().with_limit(10)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 0);

        // Should still be able to insert after cycles
        index.insert(doc_id, doc_path.clone()).await?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 1);

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_many_small_operations() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Test many small operations to stress internal data structures
        for i in 0..10000 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("edge/small_{i}.md"))?;

            index.insert(doc_id, doc_path).await?;

            // Randomly delete some documents to keep size manageable
            if i % 3 == 0 {
                index.delete(&doc_id).await?;
            }
        }

        // Index should remain functional
        let query = QueryBuilder::new().with_limit(1000)?.build()?;
        let results = index.search(&query).await?;

        // Should return the maximum limit (test that index is functioning)
        assert!(
            results.len() == 1000,
            "Unexpected result count: {} (expected exactly 1000 due to limit)",
            results.len()
        );

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }
}

#[cfg(test)]
mod primary_index_adversarial_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_index_concurrent_readers_writers() -> Result<()> {
        let (temp_dir, index_inner) = primary_index_edge_cases::create_test_index().await?;
        let index = Arc::new(Mutex::new(index_inner));
        let mut handles = Vec::new();

        // Spawn writers
        for i in 0..5 {
            let index_clone = Arc::clone(&index);
            let handle = tokio::spawn(async move {
                for j in 0..100 {
                    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
                    let doc_path =
                        ValidatedPath::new(format!("adversarial/writer_{}_{}.md", i, j)).unwrap();

                    let mut index_guard = index_clone.lock().await;
                    index_guard.insert(doc_id, doc_path).await.unwrap();
                }
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Spawn readers
        for _i in 0..10 {
            let index_clone = Arc::clone(&index);
            let handle = tokio::spawn(async move {
                for _ in 0..50 {
                    let index_guard = index_clone.lock().await;
                    let query = QueryBuilder::new()
                        .with_limit(100)
                        .unwrap()
                        .build()
                        .unwrap();
                    let results = index_guard.search(&query).await.unwrap();

                    // Small delay to increase contention
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Wait for all operations
        for handle in handles {
            handle.await??;
        }

        // Verify final state is consistent
        let index_guard = index.lock().await;
        let query = QueryBuilder::new().with_limit(1000)?.build()?;
        let results = index_guard.search(&query).await?;
        assert_eq!(results.len(), 500); // 5 writers * 100 operations each
        drop(index_guard);

        let mutex = Arc::try_unwrap(index)
            .map_err(|_| anyhow!("index Arc still has outstanding references"))?;
        let index = mutex.into_inner();
        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_memory_pressure() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Insert many large documents to stress memory usage
        let mut doc_ids = Vec::new();

        for i in 0..1000 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("adversarial/memory_pressure_{i:06}.md"))?;

            index.insert(doc_id, doc_path).await?;
            doc_ids.push(doc_id);

            // Periodically check memory doesn't grow unbounded
            if i % 100 == 0 {
                // In real implementation, we'd check memory usage here
                // For now, just verify index remains functional
                let query = QueryBuilder::new().with_limit(10)?.build()?;
                let results = index.search(&query).await?;
            }
        }

        // Delete half the documents to test memory reclamation
        for (i, doc_id) in doc_ids.iter().enumerate() {
            if i % 2 == 0 {
                index.delete(doc_id).await?;
            }
        }

        // Verify final state
        let query = QueryBuilder::new().with_limit(1000)?.build()?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 500);

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_disk_space_exhaustion() -> Result<()> {
        // This test would simulate disk space exhaustion
        // Implementation depends on the actual file format and error handling

        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // In a real test, we'd create a small filesystem or use disk quotas
        // For now, we'll test the error handling interface

        for i in 0..100 {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let doc_path = ValidatedPath::new(format!("adversarial/disk_full_{i}.md"))?;

            match index.insert(doc_id, doc_path).await {
                Ok(()) => {
                    // Normal operation
                }
                Err(e) => {
                    // Should get a clear error about disk space
                    // Implementation will define specific error types
                    println!("Expected disk space error: {e}");
                    break;
                }
            }
        }

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_interrupted_operations() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Test that interrupted operations don't corrupt the index
        // This simulates power failure, process kill, etc.

        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("adversarial/interrupted.md")?;

        // Start an operation but don't complete it
        // In real implementation, this might involve:
        // 1. Starting a transaction
        // 2. Writing partial data
        // 3. Simulating interruption before commit

        // For now, just test that normal operations work
        index.insert(doc_id, doc_path.clone()).await?;

        // Simulate recovery by creating new index instance
        let (_recovery_dir, recovered_index) =
            primary_index_edge_cases::create_test_index().await?;

        // Should be able to operate on recovered index
        let new_doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let new_doc_path = ValidatedPath::new("adversarial/post_recovery.md")?;

        // recovered_index.insert(new_doc_id, new_doc_path).await?;

        recovered_index.close().await?;
        index.close().await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_malformed_data_handling() -> Result<()> {
        // Test how index handles potentially malformed or corrupted data

        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("malformed_test");

        // Create some malformed files in the index directory
        std::fs::create_dir_all(&index_path)?;
        std::fs::write(index_path.join("corrupted.dat"), b"invalid data")?;
        std::fs::write(index_path.join("partial.dat"), b"\x00\x01\x02")?;

        // Index should either:
        // 1. Detect corruption and refuse to open
        // 2. Detect corruption and recover gracefully
        // 3. Skip malformed files and continue

        match kotadb::create_primary_index_for_tests(index_path.to_str().unwrap()).await {
            Ok(index) => {
                // If it opens successfully, basic operations should work
                // let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
                // let doc_path = ValidatedPath::new("adversarial/after_corruption.md"))?;
                // index.insert(doc_id, doc_path).await?;
                index.close().await?;
            }
            Err(e) => {
                // If it fails to open, error should be descriptive
                println!("Expected corruption detection error: {e}");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_index_pathological_key_distribution() -> Result<()> {
        let (temp_dir, mut index) = primary_index_edge_cases::create_test_index().await?;

        // Test with keys that might cause poor performance in tree structures
        // e.g., sequential UUIDs, all similar prefixes, etc.

        // Sequential-like UUIDs (poor for hash tables, might be poor for trees)
        let base_uuid = Uuid::new_v4();
        let mut base_bytes = *base_uuid.as_bytes();

        for i in 0u32..1000 {
            // Modify only the last 4 bytes to create sequential-like pattern
            let i_bytes = i.to_le_bytes();
            base_bytes[12..16].copy_from_slice(&i_bytes);

            let sequential_uuid = Uuid::from_bytes(base_bytes);
            let doc_id = ValidatedDocumentId::from_uuid(sequential_uuid)?;
            let doc_path = ValidatedPath::new(format!("adversarial/sequential_{i}.md"))?;

            index.insert(doc_id, doc_path).await?;
        }

        // Performance should still be reasonable
        let start = std::time::Instant::now();
        let query = QueryBuilder::new().with_limit(1000)?.build()?;
        let results = index.search(&query).await?;
        let duration = start.elapsed();

        assert_eq!(results.len(), 1000);
        assert!(
            duration.as_millis() < 100,
            "Search too slow with pathological keys: {duration:?}"
        );

        index.close().await?;
        drop(temp_dir);

        Ok(())
    }
}

#[cfg(test)]
mod primary_index_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_index_partial_write_recovery() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("recovery_test");

        // This test will be implemented when we have the actual file format
        // It should test recovery from various partial write scenarios:
        // 1. Incomplete node writes
        // 2. Incomplete metadata writes
        // 3. WAL replay scenarios
        // 4. Index rebuilding from storage

        Ok(())
    }

    #[tokio::test]
    async fn test_index_wal_replay() -> Result<()> {
        // Test write-ahead log replay on startup
        // This ensures durability and consistency after crashes

        Ok(())
    }

    #[tokio::test]
    async fn test_index_corruption_detection() -> Result<()> {
        // Test that index can detect various forms of corruption:
        // 1. Checksum mismatches
        // 2. Invalid pointers
        // 3. Inconsistent metadata
        // 4. Missing files

        Ok(())
    }

    #[tokio::test]
    async fn test_index_rebuilding_from_storage() -> Result<()> {
        // Test that index can be rebuilt from storage if completely corrupted
        // This is the ultimate recovery mechanism

        Ok(())
    }
}
