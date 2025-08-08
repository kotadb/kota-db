#![allow(clippy::uninlined_format_args)]
// Phase 2B Concurrent Stress Testing - Simplified Version
// Beyond 100 concurrent user baseline - tests 200+ concurrent operations

use anyhow::Result;
use kotadb::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::task;
use tracing::error;
use uuid::Uuid;

mod test_constants;
use test_constants::performance::SLOW_OPERATION_THRESHOLD;

/// Phase 2B - Enhanced Multi-threaded Stress Testing (200+ concurrent operations)
#[tokio::test]
async fn test_phase2b_enhanced_concurrent_stress() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("phase2b_storage");
    let index_path = temp_dir.path().join("phase2b_index");

    // Scale beyond current 100 user baseline to 250 concurrent operations
    let concurrent_operations = 250;
    let operations_per_task = 30;

    // Create shared system with enhanced capacity for Phase 2B
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(20000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index =
            create_primary_index(&index_path.to_string_lossy(), Some(20000)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    let mut handles = Vec::new();

    println!("🚀 Phase 2B: Starting enhanced stress test with {concurrent_operations} concurrent operations");

    let start = Instant::now();

    // Test different concurrency patterns simultaneously
    for pattern_id in 0..concurrent_operations {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);

        // Determine operation pattern type
        let is_read_heavy = pattern_id % 5 < 3; // 60% read-heavy, 40% write-heavy

        let handle = task::spawn(async move {
            let mut operations_completed = 0;
            let mut reads = 0;
            let mut writes = 0;
            let mut errors = 0;

            for op_num in 0..operations_per_task {
                let operation_start = Instant::now();

                let is_read_operation = if is_read_heavy {
                    op_num % 5 < 4 // 80% reads in read-heavy pattern
                } else {
                    op_num % 5 < 2 // 40% reads in write-heavy pattern
                };

                if is_read_operation {
                    // Read operation - simulate random document access
                    let random_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

                    match async {
                        let storage_guard = storage_ref.lock().await;
                        storage_guard.get(&random_id).await
                    }
                    .await
                    {
                        Ok(_) => reads += 1,
                        Err(_) => errors += 1,
                    }
                } else {
                    // Write operation
                    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
                    let path = ValidatedPath::new(format!(
                        "/phase2b/pattern_{pattern_id}/op_{op_num}.md"
                    ))?;
                    let title =
                        ValidatedTitle::new(format!("Phase2B Doc P{pattern_id} O{op_num}"))?;
                    let content = format!(
                        "Phase 2B enhanced concurrent stress test content for pattern {pattern_id} operation {op_num}. \
                         This tests advanced concurrent access patterns beyond the 100 user baseline."
                    ).into_bytes();
                    let tags = vec![
                        ValidatedTag::new(format!("pattern-{pattern_id}"))?,
                        ValidatedTag::new("phase2b-stress")?,
                    ];
                    let now = chrono::Utc::now();
                    let content_size = content.len();

                    let doc = Document {
                        id: doc_id,
                        path: path.clone(),
                        title,
                        content,
                        tags,
                        created_at: now,
                        updated_at: now,
                        size: content_size,
                        embedding: None,
                    };

                    // Storage write
                    match async {
                        let mut storage_guard = storage_ref.lock().await;
                        storage_guard.insert(doc.clone()).await
                    }
                    .await
                    {
                        Ok(_) => {
                            // Index update
                            match async {
                                let mut index_guard = index_ref.lock().await;
                                index_guard.insert(doc.id, path).await
                            }
                            .await
                            {
                                Ok(_) => writes += 1,
                                Err(_) => errors += 1,
                            }
                        }
                        Err(_) => errors += 1,
                    }
                }

                operations_completed += 1;

                // Track slow operations
                let operation_duration = operation_start.elapsed();
                if operation_duration > SLOW_OPERATION_THRESHOLD {
                    // This is a slow operation but we continue
                }

                // Small delay to simulate realistic load patterns
                if op_num % 10 == 0 {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }

            Ok::<(usize, usize, usize, usize), anyhow::Error>((
                operations_completed,
                reads,
                writes,
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
        match handle.await? {
            Ok((ops, reads, writes, errors)) => {
                total_ops += ops;
                total_reads += reads;
                total_writes += writes;
                total_errors += errors;
            }
            Err(e) => {
                error!("Pattern execution failed: {}", e);
                total_errors += 1;
            }
        }
    }

    let total_duration = start.elapsed();
    let throughput = total_ops as f64 / total_duration.as_secs_f64();
    let error_rate = total_errors as f64 / (total_ops + total_errors) as f64;

    println!("\n🎯 Phase 2B Enhanced Concurrent Stress Test Results:");
    println!("  📊 Total Operations: {total_ops}");
    println!("  📖 Read Operations: {total_reads}");
    println!("  ✏️  Write Operations: {total_writes}");
    println!("  ❌ Total Errors: {total_errors}");
    println!("  ⏱️  Duration: {total_duration:?}");
    println!("  🚀 Throughput: {throughput:.1} ops/sec");
    println!("  📊 Error Rate: {:.2}%", error_rate * 100.0);

    // Performance assertions for Phase 2B
    assert!(
        error_rate < 0.05,
        "Error rate too high for Phase 2B: {:.2}%",
        error_rate * 100.0
    );

    assert!(
        throughput > 150.0,
        "Throughput below Phase 2B requirement: {throughput:.1} ops/sec"
    );

    assert!(
        total_duration < Duration::from_secs(45),
        "Phase 2B test duration too long: {total_duration:?}"
    );

    // Verify system integrity after high load
    let final_storage = storage.lock().await;
    let final_docs = final_storage.list_all().await?;
    assert!(
        !final_docs.is_empty(),
        "No documents survived Phase 2B stress test"
    );

    println!("  ✅ Final document count: {}", final_docs.len());

    Ok(())
}

/// Test concurrent read scaling with 200+ readers
#[tokio::test]
async fn test_phase2b_concurrent_read_scaling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("read_scaling_storage");

    // Pre-populate storage with test data
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?,
    ));

    let mut test_doc_ids = Vec::new();

    // Insert test documents
    {
        let mut storage_guard = storage.lock().await;
        for i in 0..1000 {
            let doc = create_test_document(i, "read_scaling").await?;
            storage_guard.insert(doc.clone()).await?;
            test_doc_ids.push(doc.id);
        }
    }

    // Test with 200 concurrent readers
    let concurrent_readers = 200;
    let reads_per_reader = 50;
    let mut handles = Vec::new();

    println!("📖 Phase 2B: Concurrent Read Scaling with {concurrent_readers} readers");

    let start = Instant::now();

    for _reader_id in 0..concurrent_readers {
        let storage_ref = Arc::clone(&storage);
        let doc_ids = test_doc_ids.clone();

        let handle = task::spawn(async move {
            let mut successful_reads = 0;

            for _ in 0..reads_per_reader {
                let random_id = &doc_ids[fastrand::usize(..doc_ids.len())];

                if let Ok(Some(_)) = async {
                    let storage_guard = storage_ref.lock().await;
                    storage_guard.get(random_id).await
                }
                .await
                {
                    successful_reads += 1;
                }
            }

            successful_reads
        });

        handles.push(handle);
    }

    let mut total_reads = 0;
    for handle in handles {
        total_reads += handle.await?;
    }

    let duration = start.elapsed();
    let read_throughput = total_reads as f64 / duration.as_secs_f64();

    println!("  📊 Total Reads: {total_reads}");
    println!("  🚀 Read Throughput: {read_throughput:.1} reads/sec");
    println!("  ⏱️  Duration: {duration:?}");

    // Performance requirements for concurrent reads
    assert!(
        read_throughput > 1000.0,
        "Read throughput too low: {read_throughput:.1} reads/sec"
    );

    assert!(
        duration < Duration::from_secs(30),
        "Read test took too long: {duration:?}"
    );

    Ok(())
}

/// Test concurrent write contention with 100+ writers
#[tokio::test]
async fn test_phase2b_concurrent_write_contention() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("write_contention_storage");
    let index_path = temp_dir.path().join("write_contention_index");

    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(15000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index =
            create_primary_index(&index_path.to_string_lossy(), Some(15000)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    let concurrent_writers = 100;
    let writes_per_writer = 25;
    let mut handles = Vec::new();

    println!("✏️  Phase 2B: Concurrent Write Contention with {concurrent_writers} writers");

    let start = Instant::now();

    for writer_id in 0..concurrent_writers {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);

        let handle = task::spawn(async move {
            let mut successful_writes = 0;

            for write_id in 0..writes_per_writer {
                let doc =
                    create_test_document(writer_id * 1000 + write_id, "write_contention").await?;

                // Storage insert
                let storage_result = {
                    let mut storage_guard = storage_ref.lock().await;
                    storage_guard.insert(doc.clone()).await
                };

                if storage_result.is_ok() {
                    // Index insert
                    let index_result = {
                        let mut index_guard = index_ref.lock().await;
                        index_guard.insert(doc.id, doc.path.clone()).await
                    };

                    if index_result.is_ok() {
                        successful_writes += 1;
                    }
                }

                // Small delay to increase contention
                tokio::time::sleep(Duration::from_micros(50)).await;
            }

            Ok::<usize, anyhow::Error>(successful_writes)
        });

        handles.push(handle);
    }

    let mut total_writes = 0;
    for handle in handles {
        if let Ok(Ok(writes)) = handle.await {
            total_writes += writes;
        }
    }

    let duration = start.elapsed();
    let write_throughput = total_writes as f64 / duration.as_secs_f64();

    println!("  📊 Total Writes: {total_writes}");
    println!("  🚀 Write Throughput: {write_throughput:.1} writes/sec");
    println!("  ⏱️  Duration: {duration:?}");

    // Performance requirements for concurrent writes
    assert!(
        write_throughput > 100.0,
        "Write throughput too low: {write_throughput:.1} writes/sec"
    );

    assert!(
        total_writes >= concurrent_writers * writes_per_writer / 2,
        "Too many failed writes: {} out of {}",
        total_writes,
        concurrent_writers * writes_per_writer
    );

    // Verify final state
    let final_docs = {
        let storage_guard = storage.lock().await;
        storage_guard.list_all().await?
    };

    println!("  ✅ Final document count: {}", final_docs.len());
    assert!(
        final_docs.len() >= total_writes / 2,
        "Too few documents persisted"
    );

    Ok(())
}

/// Test burst workload patterns
#[tokio::test]
async fn test_phase2b_burst_workload_patterns() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("burst_storage");

    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?,
    ));

    println!("💥 Phase 2B: Burst Workload Patterns");

    // Test different burst patterns
    let burst_configs = vec![
        ("small_bursts", 20, 5),  // 20 ops per burst, 5 bursts
        ("medium_bursts", 50, 3), // 50 ops per burst, 3 bursts
        ("large_bursts", 100, 2), // 100 ops per burst, 2 bursts
    ];

    for (pattern_name, ops_per_burst, burst_count) in burst_configs {
        println!("  🔥 Testing {pattern_name}: {ops_per_burst} ops × {burst_count} bursts");

        let pattern_start = Instant::now();
        let mut total_ops = 0;

        for burst_id in 0..burst_count {
            // Burst phase: spawn many concurrent operations
            let mut burst_handles = Vec::new();

            for op_id in 0..ops_per_burst {
                let storage_ref = Arc::clone(&storage);
                let handle = task::spawn(async move {
                    let doc_result = create_test_document(
                        burst_id * 10000 + op_id,
                        &format!("{pattern_name}_burst"),
                    )
                    .await;

                    if let Ok(doc) = doc_result {
                        let mut storage_guard = storage_ref.lock().await;
                        let result = storage_guard.insert(doc).await;

                        if result.is_ok() {
                            1
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                });
                burst_handles.push(handle);
            }

            // Wait for burst to complete
            for handle in burst_handles {
                total_ops += handle.await.unwrap_or(0);
            }

            // Cool-down period between bursts
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let pattern_duration = pattern_start.elapsed();
        let pattern_throughput = total_ops as f64 / pattern_duration.as_secs_f64();

        println!(
            "    📊 {total_ops} operations in {:?} ({:.1} ops/sec)",
            pattern_duration, pattern_throughput
        );

        // Each pattern should achieve reasonable throughput
        assert!(
            pattern_throughput > 50.0,
            "{pattern_name} throughput too low: {pattern_throughput:.1} ops/sec"
        );
    }

    Ok(())
}

// Helper function to create test documents
async fn create_test_document(index: usize, test_type: &str) -> Result<Document> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!("/{test_type}/doc_{index:06}.md"))?;
    let title = ValidatedTitle::new(format!("{test_type} Test Document {index}"))?;

    let content = format!(
        "# Test Document {}\n\n\
         Test type: {}\n\
         Index: {}\n\
         Content: This is a test document for Phase 2B concurrent stress testing. \
         It contains realistic text content to test both storage and indexing performance under load. \
         Keywords: phase2b, concurrent, stress, test, performance, validation.\n\n\
         Timestamp: {}\n\
         Random data: {}",
        index,
        test_type,
        index,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        fastrand::u64(..)
    ).into_bytes();

    let tags = vec![
        ValidatedTag::new(test_type)?,
        ValidatedTag::new("phase2b")?,
        ValidatedTag::new("concurrent-test")?,
    ];

    let now = chrono::Utc::now();
    let content_size = content.len();

    Ok(Document {
        id: doc_id,
        path,
        title,
        content,
        tags,
        created_at: now,
        updated_at: now,
        size: content_size,
        embedding: None,
    })
}
