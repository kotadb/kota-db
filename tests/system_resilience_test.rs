// System Resilience Integration Tests - Stage 1: TDD for Phase 3 Production Readiness
// Tests system behavior under stress, resource constraints, and adverse conditions

use anyhow::Result;
use kotadb::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::task;
use uuid::Uuid;

/// Test high-load concurrent operations (100+ concurrent users)
#[tokio::test]
async fn test_high_load_concurrent_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("high_load_storage");
    let index_path = temp_dir.path().join("high_load_index");

    // Create shared system with higher capacity for stress testing
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path, Some(10000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index = create_primary_index(&index_path, 10000)?;
        create_optimized_index_with_defaults(primary_index)
    }));

    let num_concurrent_users = 100;
    let operations_per_user = 25;
    let mut handles = Vec::new();

    println!(
        "Starting high-load test: {} users, {} ops each",
        num_concurrent_users, operations_per_user
    );

    let start = Instant::now();

    // Spawn many concurrent user sessions
    for user_id in 0..num_concurrent_users {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);

        let handle = task::spawn(async move {
            let mut operations_completed = 0;
            let mut read_operations = 0;
            let mut write_operations = 0;
            let mut errors = 0;

            // Mixed read/write workload per user
            for op_num in 0..operations_per_user {
                let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
                let path =
                    ValidatedPath::new(&format!("/load_test/user_{}/doc_{}.md", user_id, op_num))?;
                let title =
                    ValidatedTitle::new(&format!("Load Test Doc U{} O{}", user_id, op_num))?;

                let content = format!(
                    "High load test content for user {} operation {}. \
                     This simulates realistic document sizes under stress.",
                    user_id, op_num
                )
                .into_bytes();

                let tags = vec![
                    ValidatedTag::new(&format!("user-{}", user_id))?,
                    ValidatedTag::new("high-load-test")?,
                ];

                let now = chrono::Utc::now();
                let doc = Document {
                    id: doc_id,
                    path,
                    title,
                    content,
                    tags,
                    created_at: now,
                    updated_at: now,
                    size: content.len(),
                };

                // Write operation
                match (|| async {
                    {
                        let mut storage_guard = storage_ref.lock().await;
                        storage_guard.insert(doc.clone()).await?;
                    }
                    {
                        let mut index_guard = index_ref.lock().await;
                        index_guard.insert(doc.id.clone(), doc.path.clone()).await?;
                    }
                    Ok::<(), anyhow::Error>(())
                })()
                .await
                {
                    Ok(_) => {
                        write_operations += 1;
                        operations_completed += 1;
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }

                // Read operation (attempt to read a previously written doc)
                if op_num > 0 {
                    let prev_doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?; // Simulate random access
                    match storage_ref.lock().await.get(&prev_doc_id).await {
                        Ok(_) => {
                            read_operations += 1;
                            operations_completed += 1;
                        }
                        Err(_) => {
                            errors += 1;
                        }
                    }
                }

                // Small delay to create realistic load patterns
                if op_num % 5 == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }

            Ok::<(usize, usize, usize, usize), anyhow::Error>((
                operations_completed,
                read_operations,
                write_operations,
                errors,
            ))
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    let mut total_ops = 0;
    let mut total_reads = 0;
    let mut total_writes = 0;
    let mut total_errors = 0;

    for handle in handles {
        let (ops, reads, writes, errors) = handle.await??;
        total_ops += ops;
        total_reads += reads;
        total_writes += writes;
        total_errors += errors;
    }

    let total_duration = start.elapsed();
    let throughput = total_ops as f64 / total_duration.as_secs_f64();

    println!("High-load test results:");
    println!("  - Total operations: {}", total_ops);
    println!("  - Read operations: {}", total_reads);
    println!("  - Write operations: {}", total_writes);
    println!("  - Total errors: {}", total_errors);
    println!("  - Duration: {:?}", total_duration);
    println!("  - Throughput: {:.1} ops/sec", throughput);

    // Performance assertions for high-load scenarios
    let error_rate = total_errors as f64 / (total_ops + total_errors) as f64;
    assert!(
        error_rate < 0.05,
        "Error rate too high under load: {:.2}%",
        error_rate * 100.0
    );
    assert!(
        throughput > 100.0,
        "Throughput too low under load: {:.1} ops/sec",
        throughput
    );
    assert!(
        total_duration < Duration::from_secs(60),
        "High-load test took too long: {:?}",
        total_duration
    );

    // Verify system integrity after high load
    let final_storage = storage.lock().await;
    let final_docs = final_storage.list_all().await?;
    assert!(final_docs.len() > 0, "No documents survived high-load test");

    Ok(())
}

/// Test memory pressure scenarios and garbage collection
#[tokio::test]
async fn test_memory_pressure_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("memory_test_storage");
    let index_path = temp_dir.path().join("memory_test_index");

    let mut storage = create_file_storage(&storage_path, Some(5000)).await?;
    let primary_index = create_primary_index(&index_path, 5000)?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing memory pressure handling...");

    // Phase 1: Create many large documents to pressure memory
    let large_doc_count = 1000;
    let large_content_size = 10_000; // 10KB per document = ~10MB total
    let mut inserted_ids = Vec::new();

    let start = Instant::now();

    for i in 0..large_doc_count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(&format!("/memory_test/large_doc_{:04}.md", i))?;
        let title = ValidatedTitle::new(&format!("Large Memory Test Doc {}", i))?;

        // Create large content to pressure memory
        let content = format!(
            "{}{}",
            format!(
                "# Large Document {}\n\nThis is a memory pressure test document. ",
                i
            ),
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
                .repeat(large_content_size / 60)
        )
        .into_bytes();

        let tags = vec![
            ValidatedTag::new("memory-test")?,
            ValidatedTag::new(&format!("batch-{}", i / 100))?,
        ];

        let now = chrono::Utc::now();
        let doc = Document {
            id: doc_id,
            path,
            title,
            content,
            tags,
            created_at: now,
            updated_at: now,
            size: content.len(),
        };

        storage.insert(doc.clone()).await?;
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
        inserted_ids.push(doc.id);

        // Periodic pressure relief test
        if i % 100 == 0 && i > 0 {
            println!(
                "  - Inserted {} large documents ({:.1}MB estimated)",
                i,
                (i * large_content_size) as f64 / 1_000_000.0
            );

            // Test random access under memory pressure
            let random_idx = fastrand::usize(..inserted_ids.len());
            let random_id = &inserted_ids[random_idx];

            let access_start = Instant::now();
            let retrieved = storage.get(random_id).await?;
            let access_time = access_start.elapsed();

            assert!(
                retrieved.is_some(),
                "Failed to retrieve document under memory pressure"
            );
            assert!(
                access_time < Duration::from_millis(100),
                "Access too slow under memory pressure: {:?}",
                access_time
            );
        }
    }

    let insertion_duration = start.elapsed();
    println!(
        "  - Inserted {} documents in {:?}",
        large_doc_count, insertion_duration
    );

    // Phase 2: Test bulk operations under memory pressure
    let query = QueryBuilder::new().limit(ValidatedLimit::new(500)?).build();

    let search_start = Instant::now();
    let search_results = optimized_index.search(&query).await?;
    let search_duration = search_start.elapsed();

    println!(
        "  - Search under memory pressure: {} results in {:?}",
        search_results.len(),
        search_duration
    );

    // Phase 3: Bulk deletion to test memory cleanup
    let delete_batch_size = 300;
    let delete_ids: Vec<_> = inserted_ids[..delete_batch_size].to_vec();

    let delete_start = Instant::now();
    for id in &delete_ids {
        storage.delete(id).await?;
        optimized_index.delete(id).await?;
    }
    let deletion_duration = delete_start.elapsed();

    println!(
        "  - Deleted {} documents in {:?}",
        delete_batch_size, deletion_duration
    );

    // Phase 4: Verify system cleanup and stability
    let remaining_docs = storage.list_all().await?;
    let expected_remaining = large_doc_count - delete_batch_size;

    assert_eq!(
        remaining_docs.len(),
        expected_remaining,
        "Incorrect document count after memory pressure test"
    );

    // Test performance hasn't degraded after memory pressure
    let final_access_start = Instant::now();
    let test_id = &inserted_ids[delete_batch_size]; // Should still exist
    let final_retrieved = storage.get(test_id).await?;
    let final_access_time = final_access_start.elapsed();

    assert!(
        final_retrieved.is_some(),
        "Document missing after memory pressure cleanup"
    );
    assert!(
        final_access_time < Duration::from_millis(50),
        "Performance degraded after memory pressure: {:?}",
        final_access_time
    );

    Ok(())
}

/// Test disk space exhaustion scenarios (simulated)
#[tokio::test]
async fn test_disk_space_exhaustion_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("disk_test_storage");
    let index_path = temp_dir.path().join("disk_test_index");

    let mut storage = create_file_storage(&storage_path, Some(1000)).await?;
    let primary_index = create_primary_index(&index_path, 1000)?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing disk space exhaustion handling...");

    // Phase 1: Fill up storage with documents
    let mut total_size = 0;
    let mut doc_count = 0;
    let max_test_size = 50_000_000; // 50MB limit for test
    let mut inserted_ids = Vec::new();

    while total_size < max_test_size {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(&format!("/disk_test/doc_{:06}.md", doc_count))?;
        let title = ValidatedTitle::new(&format!("Disk Test Doc {}", doc_count))?;

        // Variable size content
        let content_size = 5000 + (doc_count % 10000); // 5KB to 15KB
        let content = format!(
            "# Disk Space Test Document {}\n\n{}",
            doc_count,
            "Test content for disk space exhaustion simulation. ".repeat(content_size / 50)
        )
        .into_bytes();

        let tags = vec![
            ValidatedTag::new("disk-test")?,
            ValidatedTag::new(&format!("size-{}", content.len() / 1000))?,
        ];

        let now = chrono::Utc::now();
        let doc = Document {
            id: doc_id,
            path,
            title,
            content,
            tags,
            created_at: now,
            updated_at: now,
            size: content.len(),
        };

        // Attempt insertion with graceful failure handling
        match storage.insert(doc.clone()).await {
            Ok(()) => {
                match optimized_index
                    .insert(doc.id.clone(), doc.path.clone())
                    .await
                {
                    Ok(()) => {
                        total_size += doc.size;
                        inserted_ids.push(doc.id);
                        doc_count += 1;

                        if doc_count % 500 == 0 {
                            println!(
                                "  - Inserted {} docs, {:.1}MB total",
                                doc_count,
                                total_size as f64 / 1_000_000.0
                            );
                        }
                    }
                    Err(e) => {
                        println!("  - Index insertion failed at doc {}: {}", doc_count, e);
                        break; // Simulate index space exhaustion
                    }
                }
            }
            Err(e) => {
                println!("  - Storage insertion failed at doc {}: {}", doc_count, e);
                break; // Simulate storage space exhaustion
            }
        }
    }

    println!(
        "  - Reached capacity: {} docs, {:.1}MB",
        doc_count,
        total_size as f64 / 1_000_000.0
    );

    // Phase 2: Test read operations during space pressure
    let read_test_count = 50;
    let mut successful_reads = 0;

    for _ in 0..read_test_count {
        let random_idx = fastrand::usize(..inserted_ids.len());
        let random_id = &inserted_ids[random_idx];

        match storage.get(random_id).await {
            Ok(Some(_)) => successful_reads += 1,
            Ok(None) => {} // Document not found (acceptable)
            Err(_) => {}   // Read failure under pressure (acceptable)
        }
    }

    let read_success_rate = successful_reads as f64 / read_test_count as f64;
    println!(
        "  - Read success rate under pressure: {:.1}%",
        read_success_rate * 100.0
    );

    // Should maintain reasonable read capability even under pressure
    assert!(
        read_success_rate > 0.8,
        "Read success rate too low under disk pressure: {:.1}%",
        read_success_rate * 100.0
    );

    // Phase 3: Test cleanup and space recovery
    let cleanup_count = doc_count / 4; // Delete 25% of documents
    let cleanup_ids: Vec<_> = inserted_ids[..cleanup_count].to_vec();

    let cleanup_start = Instant::now();
    let mut cleanup_successes = 0;

    for id in &cleanup_ids {
        match storage.delete(id).await {
            Ok(true) => {
                if optimized_index.delete(id).await.unwrap_or(false) {
                    cleanup_successes += 1;
                }
            }
            _ => {}
        }
    }

    let cleanup_duration = cleanup_start.elapsed();
    println!(
        "  - Cleaned up {}/{} docs in {:?}",
        cleanup_successes, cleanup_count, cleanup_duration
    );

    // Phase 4: Verify system recovery
    let remaining_docs = storage.list_all().await?;
    assert!(
        remaining_docs.len() > 0,
        "All documents lost during cleanup"
    );
    assert!(
        remaining_docs.len() <= doc_count,
        "Document count increased unexpectedly"
    );

    // Test that new insertions work after cleanup
    let recovery_doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let recovery_path = ValidatedPath::new("/disk_test/recovery_test.md")?;
    let recovery_title = ValidatedTitle::new("Recovery Test Document")?;
    let recovery_content = "Recovery test after disk cleanup.".as_bytes().to_vec();

    let recovery_tags = vec![ValidatedTag::new("recovery-test")?];
    let now = chrono::Utc::now();

    let recovery_doc = Document {
        id: recovery_doc_id,
        path: recovery_path,
        title: recovery_title,
        content: recovery_content,
        tags: recovery_tags,
        created_at: now,
        updated_at: now,
        size: recovery_content.len(),
    };

    let recovery_result = storage.insert(recovery_doc.clone()).await;
    assert!(
        recovery_result.is_ok(),
        "Failed to insert after cleanup - system not recovered"
    );

    Ok(())
}

/// Test graceful degradation under resource constraints
#[tokio::test]
async fn test_graceful_degradation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("degradation_storage");
    let index_path = temp_dir.path().join("degradation_index");

    // Start with limited capacity to force degradation
    let mut storage = create_file_storage(&storage_path, Some(100)).await?;
    let primary_index = create_primary_index(&index_path, 100)?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing graceful degradation under resource constraints...");

    // Phase 1: Normal operation baseline
    let baseline_docs = 50;
    let mut baseline_ids = Vec::new();

    let baseline_start = Instant::now();
    for i in 0..baseline_docs {
        let doc = create_test_document(i, "baseline")?;
        storage.insert(doc.clone()).await?;
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
        baseline_ids.push(doc.id);
    }
    let baseline_duration = baseline_start.elapsed();

    println!(
        "  - Baseline: {} docs in {:?}",
        baseline_docs, baseline_duration
    );

    // Phase 2: Approach capacity limits
    let stress_docs = 75; // Approaching the 100 limit
    let mut stress_ids = Vec::new();
    let mut degradation_detected = false;

    let stress_start = Instant::now();
    for i in 0..stress_docs {
        let doc = create_test_document(baseline_docs + i, "stress")?;

        let insert_start = Instant::now();
        match storage.insert(doc.clone()).await {
            Ok(()) => {
                match optimized_index
                    .insert(doc.id.clone(), doc.path.clone())
                    .await
                {
                    Ok(()) => {
                        stress_ids.push(doc.id);

                        let insert_duration = insert_start.elapsed();
                        // Detect performance degradation
                        if insert_duration > Duration::from_millis(100) {
                            degradation_detected = true;
                            println!(
                                "  - Degradation detected at doc {}: {:?}",
                                baseline_docs + i,
                                insert_duration
                            );
                        }
                    }
                    Err(e) => {
                        println!(
                            "  - Index capacity reached at doc {}: {}",
                            baseline_docs + i,
                            e
                        );
                        break;
                    }
                }
            }
            Err(e) => {
                println!(
                    "  - Storage capacity reached at doc {}: {}",
                    baseline_docs + i,
                    e
                );
                break;
            }
        }
    }
    let stress_duration = stress_start.elapsed();

    println!(
        "  - Stress phase: {} additional docs in {:?}",
        stress_ids.len(),
        stress_duration
    );

    // Phase 3: Test read performance under stress
    let read_test_samples = 20;
    let mut read_times = Vec::new();
    let mut read_failures = 0;

    for _ in 0..read_test_samples {
        let all_ids: Vec<_> = baseline_ids.iter().chain(stress_ids.iter()).collect();
        if !all_ids.is_empty() {
            let random_idx = fastrand::usize(..all_ids.len());
            let random_id = all_ids[random_idx];

            let read_start = Instant::now();
            match storage.get(random_id).await {
                Ok(Some(_)) => {
                    read_times.push(read_start.elapsed());
                }
                _ => {
                    read_failures += 1;
                }
            }
        }
    }

    if !read_times.is_empty() {
        let avg_read_time = read_times.iter().sum::<Duration>() / read_times.len() as u32;
        let max_read_time = read_times.iter().max().unwrap();

        println!(
            "  - Read performance under stress: avg {:?}, max {:?}, failures {}",
            avg_read_time, max_read_time, read_failures
        );

        // Graceful degradation acceptance criteria
        assert!(
            avg_read_time < Duration::from_millis(50),
            "Average read time too slow under stress: {:?}",
            avg_read_time
        );
        assert!(
            *max_read_time < Duration::from_millis(200),
            "Maximum read time too slow under stress: {:?}",
            max_read_time
        );
    }

    let read_failure_rate = read_failures as f64 / read_test_samples as f64;
    assert!(
        read_failure_rate < 0.2,
        "Read failure rate too high under stress: {:.1}%",
        read_failure_rate * 100.0
    );

    // Phase 4: Test recovery after load reduction
    println!("  - Testing recovery after load reduction...");

    // Remove some documents to reduce load
    let cleanup_count = stress_ids.len() / 2;
    for id in &stress_ids[..cleanup_count] {
        storage.delete(id).await?;
        optimized_index.delete(id).await?;
    }

    // Test that performance improves
    let recovery_doc = create_test_document(999, "recovery")?;
    let recovery_start = Instant::now();
    let recovery_result = storage.insert(recovery_doc.clone()).await;
    let recovery_duration = recovery_start.elapsed();

    println!("  - Recovery insert time: {:?}", recovery_duration);

    // Should perform better after load reduction
    assert!(recovery_result.is_ok(), "Failed to insert during recovery");
    assert!(
        recovery_duration < Duration::from_millis(50),
        "Recovery performance not improved: {:?}",
        recovery_duration
    );

    Ok(())
}

// Helper function to create test documents
fn create_test_document(index: usize, test_type: &str) -> Result<Document> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(&format!("/{}/doc_{:04}.md", test_type, index))?;
    let title = ValidatedTitle::new(&format!("{} Test Document {}", test_type, index))?;

    let content =
        format!(
        "# {} Test Document {}\n\nThis is a test document for resilience testing.\n\nContent: {}",
        test_type, index, "Test data. ".repeat(50)
    )
        .into_bytes();

    let tags = vec![
        ValidatedTag::new(test_type)?,
        ValidatedTag::new("resilience-test")?,
    ];

    let now = chrono::Utc::now();

    Ok(Document {
        id: doc_id,
        path,
        title,
        content,
        tags,
        created_at: now,
        updated_at: now,
        size: content.len(),
    })
}
