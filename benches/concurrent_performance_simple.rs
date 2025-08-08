#![allow(clippy::uninlined_format_args)]
// Simplified Concurrent Performance Benchmarks - Phase 2B Benchmark Suite
// Performance testing for concurrent operations beyond 100 user baseline

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use kotadb::*;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::task;
use uuid::Uuid;

/// Benchmark concurrent read operations with different thread counts
fn concurrent_read_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_read_scaling");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    // Pre-populate storage with test data
    let (storage, test_doc_ids) = rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(10000))
            .await
            .unwrap();

        let mut doc_ids = Vec::new();
        for i in 0..1000 {
            let doc = create_benchmark_document(i, "read_scaling").unwrap();
            storage.insert(doc.clone()).await.unwrap();
            doc_ids.push(doc.id);
        }

        (Arc::new(tokio::sync::Mutex::new(storage)), doc_ids)
    });

    // Test different concurrent reader counts
    for &thread_count in &[1, 2, 4, 8, 16, 32] {
        group.throughput(Throughput::Elements(thread_count * 100));

        group.bench_with_input(
            BenchmarkId::new("concurrent_readers", thread_count),
            &thread_count,
            |b, &threads| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        for _ in 0..threads {
                            let storage_ref = Arc::clone(&storage);
                            let doc_ids = test_doc_ids.clone();

                            let handle = task::spawn(async move {
                                let mut read_count = 0;
                                for _ in 0..100 {
                                    let random_id = &doc_ids[fastrand::usize(..doc_ids.len())];
                                    let storage_guard = storage_ref.lock().await;
                                    if let Ok(Some(_)) = storage_guard.get(random_id).await {
                                        read_count += 1;
                                    }
                                }
                                read_count
                            });

                            handles.push(handle);
                        }

                        let mut total_reads = 0;
                        for handle in handles {
                            total_reads += handle.await.unwrap();
                        }

                        black_box(total_reads)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent write operations with lock contention
fn concurrent_write_contention(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_write_contention");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    // Test different concurrent writer counts
    for &writer_count in &[1, 2, 4, 8, 16] {
        group.throughput(Throughput::Elements(writer_count * 50));

        group.bench_with_input(
            BenchmarkId::new("concurrent_writers", writer_count),
            &writer_count,
            |b, &writers| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let storage = Arc::new(tokio::sync::Mutex::new(
                            create_file_storage(temp_dir.path().to_str().unwrap(), Some(5000))
                                .await
                                .unwrap(),
                        ));

                        let mut handles = Vec::new();

                        for writer_id in 0..writers {
                            let storage_ref = Arc::clone(&storage);

                            let handle = task::spawn(async move {
                                let mut write_count = 0;
                                for op_id in 0..50 {
                                    let doc = create_benchmark_document(
                                        (writer_id * 1000 + op_id) as usize,
                                        "write_contention",
                                    )
                                    .unwrap();

                                    let mut storage_guard = storage_ref.lock().await;
                                    if storage_guard.insert(doc).await.is_ok() {
                                        write_count += 1;
                                    }
                                }
                                write_count
                            });

                            handles.push(handle);
                        }

                        let mut total_writes = 0;
                        for handle in handles {
                            total_writes += handle.await.unwrap();
                        }

                        black_box(total_writes)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mixed read-write workloads
fn mixed_workload_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("mixed_workload_performance");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    // Test different read-write ratios
    let workload_patterns = vec![
        ("read_heavy", 80, 20),  // 80% reads, 20% writes
        ("balanced", 50, 50),    // 50% reads, 50% writes
        ("write_heavy", 20, 80), // 20% reads, 80% writes
    ];

    for (pattern_name, read_pct, _write_pct) in workload_patterns {
        group.throughput(Throughput::Elements(200)); // 200 total operations

        group.bench_with_input(
            BenchmarkId::new("mixed_workload", pattern_name),
            &read_pct,
            |b, &read_percentage| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let storage = Arc::new(tokio::sync::Mutex::new(
                            create_file_storage(temp_dir.path().to_str().unwrap(), Some(5000))
                                .await
                                .unwrap(),
                        ));

                        // Pre-populate for reads
                        let mut existing_ids = Vec::new();
                        {
                            let mut storage_guard = storage.lock().await;
                            for i in 0..100 {
                                let doc = create_benchmark_document(i, "mixed_pre").unwrap();
                                storage_guard.insert(doc.clone()).await.unwrap();
                                existing_ids.push(doc.id);
                            }
                        }

                        let shared_ids = Arc::new(existing_ids);
                        let mut handles = Vec::new();

                        // Spawn 8 worker threads
                        for worker_id in 0..8 {
                            let storage_ref = Arc::clone(&storage);
                            let ids_ref = Arc::clone(&shared_ids);

                            let handle = task::spawn(async move {
                                let mut operations = 0;
                                for op_id in 0..25 {
                                    // 25 ops per worker = 200 total
                                    let is_read = (op_id * 100 / 25) < read_percentage;

                                    if is_read && !ids_ref.is_empty() {
                                        // Read operation
                                        let random_id = &ids_ref[fastrand::usize(..ids_ref.len())];
                                        let storage_guard = storage_ref.lock().await;
                                        if storage_guard.get(random_id).await.is_ok() {
                                            operations += 1;
                                        }
                                    } else {
                                        // Write operation
                                        let doc = create_benchmark_document(
                                            1000 + worker_id * 100 + op_id,
                                            "mixed_write",
                                        )
                                        .unwrap();

                                        let mut storage_guard = storage_ref.lock().await;
                                        if storage_guard.insert(doc).await.is_ok() {
                                            operations += 1;
                                        }
                                    }
                                }
                                operations
                            });

                            handles.push(handle);
                        }

                        let mut total_ops = 0;
                        for handle in handles {
                            total_ops += handle.await.unwrap();
                        }

                        black_box(total_ops)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark burst workload patterns
fn burst_workload_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("burst_workload_patterns");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    // Test different burst patterns
    let burst_patterns = vec![
        ("small_bursts", 10, 5),  // 10 ops per burst, 5 bursts
        ("medium_bursts", 25, 4), // 25 ops per burst, 4 bursts
        ("large_bursts", 50, 2),  // 50 ops per burst, 2 bursts
    ];

    for (pattern_name, ops_per_burst, burst_count) in burst_patterns {
        group.throughput(Throughput::Elements(ops_per_burst * burst_count));

        group.bench_with_input(
            BenchmarkId::new("burst_pattern", pattern_name),
            &(ops_per_burst, burst_count),
            |b, &(burst_size, bursts)| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let storage = Arc::new(tokio::sync::Mutex::new(
                            create_file_storage(temp_dir.path().to_str().unwrap(), Some(5000))
                                .await
                                .unwrap(),
                        ));

                        let mut total_ops = 0;

                        for burst_id in 0..bursts {
                            // Burst phase: spawn many concurrent operations
                            let mut burst_handles = Vec::new();

                            for op_id in 0..burst_size {
                                let storage_ref = Arc::clone(&storage);
                                let handle = task::spawn(async move {
                                    let doc_result = create_benchmark_document(
                                        (burst_id * 1000 + op_id) as usize,
                                        "burst_test",
                                    );

                                    if let Ok(doc) = doc_result {
                                        let mut storage_guard = storage_ref.lock().await;
                                        if storage_guard.insert(doc).await.is_ok() {
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
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }

                        black_box(total_ops)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory pressure under concurrent access
fn memory_pressure_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_pressure_concurrent");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(5); // Fewer samples due to memory usage

    // Test with different document sizes to create memory pressure
    let document_sizes = vec![
        ("small_docs", 1024),   // 1KB documents
        ("medium_docs", 10240), // 10KB documents
        ("large_docs", 102400), // 100KB documents
    ];

    for (size_name, doc_size) in document_sizes {
        group.throughput(Throughput::Bytes(doc_size * 100)); // 100 docs per benchmark

        group.bench_with_input(
            BenchmarkId::new("memory_pressure", size_name),
            &doc_size,
            |b, &document_size| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let storage = Arc::new(tokio::sync::Mutex::new(
                            create_file_storage(temp_dir.path().to_str().unwrap(), Some(2000))
                                .await
                                .unwrap(),
                        ));

                        let mut handles = Vec::new();

                        // Spawn 8 concurrent workers
                        for worker_id in 0..8 {
                            let storage_ref = Arc::clone(&storage);

                            let handle = task::spawn(async move {
                                let mut operations = 0;
                                for op_id in 0..12 {
                                    // 12 ops per worker = 96 total (close to 100)
                                    let doc = create_large_benchmark_document(
                                        (worker_id * 100 + op_id) as usize,
                                        "memory_pressure",
                                        document_size as usize,
                                    )
                                    .unwrap();

                                    let mut storage_guard = storage_ref.lock().await;
                                    if storage_guard.insert(doc).await.is_ok() {
                                        operations += 1;
                                    }

                                    // Occasionally read to test memory pressure during mixed workload
                                    if op_id % 3 == 0 {
                                        let _ = storage_guard.list_all().await;
                                    }
                                }
                                operations
                            });

                            handles.push(handle);
                        }

                        let mut total_ops = 0;
                        for handle in handles {
                            total_ops += handle.await.unwrap();
                        }

                        black_box(total_ops)
                    })
                });
            },
        );
    }

    group.finish();
}

// Helper functions

fn create_benchmark_document(index: usize, test_type: &str) -> Result<Document, anyhow::Error> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!("/{test_type}/benchmark_{index:06}.md"))?;
    let title = ValidatedTitle::new(format!("{test_type} Benchmark Document {index}"))?;

    let content = format!(
        "# Benchmark Document {}\n\n\
         Test type: {}\n\
         Index: {}\n\
         Content: This is a benchmark document for concurrent performance testing. \
         It contains realistic text content to test both storage and indexing performance. \
         Keywords: benchmark, performance, concurrent, test, validation, storage, index.\n\n\
         Timestamp: {}\n\
         Random data: {}",
        index,
        test_type,
        index,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        fastrand::u64(..)
    )
    .into_bytes();

    let tags = vec![
        ValidatedTag::new(test_type)?,
        ValidatedTag::new("benchmark")?,
        ValidatedTag::new("performance")?,
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

fn create_large_benchmark_document(
    index: usize,
    test_type: &str,
    target_size: usize,
) -> Result<Document, anyhow::Error> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!("/{test_type}/large_benchmark_{index:06}.md"))?;
    let title = ValidatedTitle::new(format!("{test_type} Large Benchmark Document {index}"))?;

    let base_content = format!(
        "# Large Benchmark Document {index}\n\n\
         Test type: {test_type}\n\
         Index: {index}\n\
         Target size: {target_size} bytes\n\
         Content: This is a large benchmark document for memory pressure testing.\n\n"
    );

    // Fill remaining space with repeated content to reach target size
    let remaining_size = target_size.saturating_sub(base_content.len());
    let filler =
        "This is filler content for memory pressure testing. ".repeat(remaining_size / 50 + 1);
    let content = format!(
        "{}{}",
        base_content,
        &filler[..remaining_size.min(filler.len())]
    )
    .into_bytes();

    let tags = vec![
        ValidatedTag::new(test_type)?,
        ValidatedTag::new("large-benchmark")?,
        ValidatedTag::new("memory-pressure")?,
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

criterion_group!(
    name = concurrent_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5));
    targets =
        concurrent_read_scaling,
        concurrent_write_contention,
        mixed_workload_performance,
        burst_workload_patterns,
        memory_pressure_concurrent
);

criterion_main!(concurrent_benches);
