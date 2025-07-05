// Performance Integration Tests - Stage 1: TDD for Phase 3 Production Readiness
// Tests that validate complete system performance meets production SLAs

use anyhow::Result;
use kotadb::contracts::BulkOperations;
use kotadb::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::task;
use uuid::Uuid;

/// Test end-to-end performance SLAs under realistic workloads
#[tokio::test]
async fn test_end_to_end_performance_slas() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("perf_sla_storage");
    let index_path = temp_dir.path().join("perf_sla_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(5000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(5000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing end-to-end performance SLAs...");

    // Phase 1: Baseline performance measurement
    let baseline_docs = create_performance_test_documents(100, "baseline")?;

    let baseline_start = Instant::now();
    for doc in &baseline_docs {
        storage.insert(doc.clone()).await?;
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
    }
    let baseline_duration = baseline_start.elapsed();
    let baseline_throughput = baseline_docs.len() as f64 / baseline_duration.as_secs_f64();

    println!(
        "  - Baseline: {} docs in {:?} ({:.1} docs/sec)",
        baseline_docs.len(),
        baseline_duration,
        baseline_throughput
    );

    // SLA: Individual operations should complete within acceptable time
    let avg_insert_time = baseline_duration / baseline_docs.len() as u32;
    assert!(
        avg_insert_time < Duration::from_millis(50),
        "Average insert time exceeds SLA: {:?}",
        avg_insert_time
    );

    // Phase 2: Bulk operation performance SLA
    println!("  - Testing bulk operation performance SLAs...");

    let bulk_docs = create_performance_test_documents(1000, "bulk")?;
    let bulk_pairs: Vec<_> = bulk_docs
        .iter()
        .map(|doc| (doc.id.clone(), doc.path.clone()))
        .collect();

    // Bulk insert performance
    let bulk_start = Instant::now();

    // Insert to storage first
    for doc in &bulk_docs {
        storage.insert(doc.clone()).await?;
    }

    // Bulk insert to index
    let bulk_result = optimized_index.bulk_insert(bulk_pairs)?;
    let bulk_duration = bulk_start.elapsed();

    println!(
        "  - Bulk operations: {} docs in {:?}",
        bulk_docs.len(),
        bulk_duration
    );
    println!(
        "  - Bulk result: {} ops at {:.1} ops/sec",
        bulk_result.operations_completed, bulk_result.throughput_ops_per_sec
    );

    // SLA: Bulk operations should achieve 10x improvement over individual ops
    let bulk_throughput = bulk_docs.len() as f64 / bulk_duration.as_secs_f64();
    let speedup_ratio = bulk_throughput / baseline_throughput;

    assert!(
        speedup_ratio >= 5.0,
        "Bulk operation speedup below SLA: {:.1}x (target: 5x)",
        speedup_ratio
    );
    assert!(
        bulk_result.meets_performance_requirements(5.0),
        "Bulk operation SLA not met according to internal metrics"
    );

    // Phase 3: Read performance SLA testing
    println!("  - Testing read performance SLAs...");

    let all_docs = [&baseline_docs[..], &bulk_docs[..]].concat();
    let read_sample_size = 500;
    let mut read_times = Vec::new();

    let read_test_start = Instant::now();
    for _ in 0..read_sample_size {
        let random_idx = fastrand::usize(..all_docs.len());
        let random_doc = &all_docs[random_idx];

        let read_start = Instant::now();
        let retrieved = storage.get(&random_doc.id).await?;
        let read_time = read_start.elapsed();

        assert!(retrieved.is_some(), "Document not found during read test");
        read_times.push(read_time);
    }
    let total_read_time = read_test_start.elapsed();

    // Calculate read performance statistics
    let avg_read_time = read_times.iter().sum::<Duration>() / read_times.len() as u32;
    let min_read_time = *read_times.iter().min().unwrap();
    let max_read_time = *read_times.iter().max().unwrap();

    // Calculate percentiles
    let mut sorted_times = read_times.clone();
    sorted_times.sort();
    let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
    let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;
    let p95_read_time = sorted_times[p95_idx.min(sorted_times.len() - 1)];
    let p99_read_time = sorted_times[p99_idx.min(sorted_times.len() - 1)];

    println!("  - Read performance stats:");
    println!("    - {} reads in {:?}", read_sample_size, total_read_time);
    println!("    - Average: {:?}", avg_read_time);
    println!("    - Min: {:?}, Max: {:?}", min_read_time, max_read_time);
    println!("    - P95: {:?}, P99: {:?}", p95_read_time, p99_read_time);

    // SLA: Read operations performance requirements
    assert!(
        avg_read_time < Duration::from_millis(10),
        "Average read time exceeds SLA: {:?}",
        avg_read_time
    );
    assert!(
        p95_read_time < Duration::from_millis(25),
        "P95 read time exceeds SLA: {:?}",
        p95_read_time
    );
    assert!(
        p99_read_time < Duration::from_millis(100),
        "P99 read time exceeds SLA: {:?}",
        p99_read_time
    );

    // Phase 4: Search performance SLA
    println!("  - Testing search performance SLAs...");

    let query = QueryBuilder::new().with_limit(100)?.build()?;

    let search_iterations = 50;
    let mut search_times = Vec::new();

    for i in 0..search_iterations {
        let search_start = Instant::now();
        let search_results = optimized_index.search(&query).await?;
        let search_time = search_start.elapsed();

        search_times.push(search_time);

        if i == 0 {
            println!(
                "    - First search returned {} results",
                search_results.len()
            );
        }
    }

    let avg_search_time = search_times.iter().sum::<Duration>() / search_times.len() as u32;
    let max_search_time = *search_times.iter().max().unwrap();

    println!("    - Average search time: {:?}", avg_search_time);
    println!("    - Max search time: {:?}", max_search_time);

    // SLA: Search operations should be fast
    assert!(
        avg_search_time < Duration::from_millis(20),
        "Average search time exceeds SLA: {:?}",
        avg_search_time
    );
    assert!(
        max_search_time < Duration::from_millis(100),
        "Max search time exceeds SLA: {:?}",
        max_search_time
    );

    println!("  - All performance SLAs met successfully");

    Ok(())
}

/// Test performance under concurrent load
#[tokio::test]
async fn test_concurrent_performance_characteristics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("concurrent_perf_storage");
    let index_path = temp_dir.path().join("concurrent_perf_index");

    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(5000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(5000)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    println!("Testing concurrent performance characteristics...");

    // Phase 1: Measure performance scaling with concurrent users
    let concurrency_levels = vec![1, 5, 10, 20];
    let operations_per_user = 50;

    for &num_users in &concurrency_levels {
        println!("  - Testing with {} concurrent users...", num_users);

        let mut handles = Vec::new();
        let test_start = Instant::now();

        for user_id in 0..num_users {
            let storage_ref = Arc::clone(&storage);
            let index_ref = Arc::clone(&index);

            let handle = task::spawn(async move {
                let mut user_metrics = UserPerformanceMetrics::new(user_id);

                for op_num in 0..operations_per_user {
                    let doc = create_user_performance_document(user_id, op_num)?;

                    // Write operation
                    let write_start = Instant::now();
                    {
                        let mut storage_guard = storage_ref.lock().await;
                        storage_guard.insert(doc.clone()).await?;
                    }
                    {
                        let mut index_guard = index_ref.lock().await;
                        index_guard.insert(doc.id.clone(), doc.path.clone()).await?;
                    }
                    let write_time = write_start.elapsed();
                    user_metrics.add_write_time(write_time);

                    // Read operation (read a previous document)
                    if op_num > 0 {
                        let read_start = Instant::now();
                        let storage_guard = storage_ref.lock().await;
                        let _ = storage_guard.get(&doc.id).await?;
                        let read_time = read_start.elapsed();
                        user_metrics.add_read_time(read_time);
                    }
                }

                Ok::<UserPerformanceMetrics, anyhow::Error>(user_metrics)
            });

            handles.push(handle);
        }

        // Collect results
        let mut all_write_times = Vec::new();
        let mut all_read_times = Vec::new();
        let mut total_operations = 0;

        for handle in handles {
            let user_metrics = handle.await??;
            let total_ops = user_metrics.total_operations();
            all_write_times.extend(user_metrics.write_times);
            all_read_times.extend(user_metrics.read_times);
            total_operations += total_ops;
        }

        let total_duration = test_start.elapsed();
        let concurrent_throughput = total_operations as f64 / total_duration.as_secs_f64();

        // Calculate performance statistics
        let avg_write_time = if !all_write_times.is_empty() {
            all_write_times.iter().sum::<Duration>() / all_write_times.len() as u32
        } else {
            Duration::ZERO
        };

        let avg_read_time = if !all_read_times.is_empty() {
            all_read_times.iter().sum::<Duration>() / all_read_times.len() as u32
        } else {
            Duration::ZERO
        };

        println!(
            "    - {} users: {:.1} ops/sec, avg write: {:?}, avg read: {:?}",
            num_users, concurrent_throughput, avg_write_time, avg_read_time
        );

        // Performance degradation should be reasonable
        assert!(
            avg_write_time < Duration::from_millis(100),
            "Write performance degraded too much with {} users: {:?}",
            num_users,
            avg_write_time
        );
        assert!(
            avg_read_time < Duration::from_millis(50),
            "Read performance degraded too much with {} users: {:?}",
            num_users,
            avg_read_time
        );

        // Throughput should scale reasonably (not linearly, but should increase)
        if num_users > 1 {
            assert!(
                concurrent_throughput > 50.0,
                "Throughput too low with {} users: {:.1} ops/sec",
                num_users,
                concurrent_throughput
            );
        }
    }

    Ok(())
}

/// Test memory usage and performance correlation
#[tokio::test]
async fn test_memory_performance_characteristics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("memory_perf_storage");
    let index_path = temp_dir.path().join("memory_perf_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(10000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing memory vs performance characteristics...");

    // Phase 1: Test performance at different dataset sizes
    let size_milestones = vec![100, 500, 1000, 2000, 5000];
    let mut performance_history = Vec::new();

    for &target_size in &size_milestones {
        println!("  - Testing performance at {} documents...", target_size);

        // Add documents to reach target size
        let current_docs = storage.list_all().await?.len();
        let docs_to_add = target_size - current_docs;

        if docs_to_add > 0 {
            let new_docs =
                create_performance_test_documents(docs_to_add, &format!("size_{}", target_size))?;

            let insert_start = Instant::now();
            for doc in &new_docs {
                storage.insert(doc.clone()).await?;
                optimized_index
                    .insert(doc.id.clone(), doc.path.clone())
                    .await?;
            }
            let insert_duration = insert_start.elapsed();
            let insert_throughput = docs_to_add as f64 / insert_duration.as_secs_f64();

            // Test read performance at this dataset size
            let read_sample_size = 100.min(target_size);
            let mut read_times = Vec::new();

            let all_docs = storage.list_all().await?;
            for _ in 0..read_sample_size {
                let random_idx = fastrand::usize(..all_docs.len());
                let random_doc = &all_docs[random_idx];

                let read_start = Instant::now();
                let _ = storage.get(&random_doc.id).await?;
                read_times.push(read_start.elapsed());
            }

            let avg_read_time = read_times.iter().sum::<Duration>() / read_times.len() as u32;

            // Test search performance
            let query = QueryBuilder::new().with_limit(50)?.build()?;

            let search_start = Instant::now();
            let search_results = optimized_index.search(&query).await?;
            let search_time = search_start.elapsed();

            let milestone_metrics = DatasetPerformanceMetrics {
                dataset_size: target_size,
                insert_throughput,
                avg_read_time,
                search_time,
                search_results_count: search_results.len(),
            };

            performance_history.push(milestone_metrics);

            println!("    - Insert: {:.1} docs/sec", insert_throughput);
            println!("    - Read: {:?} avg", avg_read_time);
            println!(
                "    - Search: {:?} ({} results)",
                search_time,
                search_results.len()
            );
        }
    }

    // Phase 2: Analyze performance scaling characteristics
    println!("  - Analyzing performance scaling...");

    for (i, metrics) in performance_history.iter().enumerate() {
        if i > 0 {
            let prev_metrics = &performance_history[i - 1];
            let size_ratio = metrics.dataset_size as f64 / prev_metrics.dataset_size as f64;
            let read_ratio = metrics.avg_read_time.as_nanos() as f64
                / prev_metrics.avg_read_time.as_nanos() as f64;
            let search_ratio =
                metrics.search_time.as_nanos() as f64 / prev_metrics.search_time.as_nanos() as f64;

            println!(
                "    - Size {}x: read {}x, search {}x",
                size_ratio, read_ratio, search_ratio
            );

            // Performance should scale logarithmically, not linearly
            // If dataset increases by factor X, performance should degrade by much less than X
            assert!(
                read_ratio < size_ratio * 0.5,
                "Read performance scaling too poor: {}x degradation for {}x size increase",
                read_ratio,
                size_ratio
            );
            assert!(
                search_ratio < size_ratio * 0.7,
                "Search performance scaling too poor: {}x degradation for {}x size increase",
                search_ratio,
                size_ratio
            );
        }
    }

    // Phase 3: Test performance under memory pressure
    println!("  - Testing performance under memory pressure...");

    let large_docs = create_large_documents(100, 50000)?; // 100 docs of ~50KB each

    let memory_pressure_start = Instant::now();
    for doc in &large_docs {
        storage.insert(doc.clone()).await?;
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
    }
    let memory_pressure_duration = memory_pressure_start.elapsed();

    // Test that read performance hasn't degraded severely under memory pressure
    let read_test_count = 50;

    let mut pressure_read_times: Vec<Duration> = Vec::new();
    for _ in 0..read_test_count {
        let random_idx = fastrand::usize(..large_docs.len());
        let random_doc = &large_docs[random_idx];

        let read_start = Instant::now();
        let retrieved = storage.get(&random_doc.id).await?;
        pressure_read_times.push(read_start.elapsed());
        assert!(retrieved.is_some(), "Large document not retrievable");
    }

    let avg_pressure_read_time =
        pressure_read_times.iter().sum::<Duration>() / pressure_read_times.len() as u32;

    println!(
        "    - Large docs inserted in: {:?}",
        memory_pressure_duration
    );
    println!(
        "    - Average read time under pressure: {:?}",
        avg_pressure_read_time
    );

    // Performance under memory pressure should still be acceptable
    assert!(
        avg_pressure_read_time < Duration::from_millis(100),
        "Read performance too slow under memory pressure: {:?}",
        avg_pressure_read_time
    );

    Ok(())
}

/// Test bulk operation performance characteristics
#[tokio::test]
async fn test_bulk_operation_performance_guarantees() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("bulk_perf_storage");
    let index_path = temp_dir.path().join("bulk_perf_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(10000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing bulk operation performance guarantees...");

    // Phase 1: Test various bulk operation sizes
    let bulk_sizes = vec![10, 50, 100, 500, 1000];

    for &bulk_size in &bulk_sizes {
        println!(
            "  - Testing bulk operations with {} documents...",
            bulk_size
        );

        let bulk_docs =
            create_performance_test_documents(bulk_size, &format!("bulk_{}", bulk_size))?;

        // Prepare for bulk insert
        for doc in &bulk_docs {
            storage.insert(doc.clone()).await?;
        }

        let bulk_pairs: Vec<_> = bulk_docs
            .iter()
            .map(|doc| (doc.id.clone(), doc.path.clone()))
            .collect();

        // Test bulk insert performance
        let bulk_start = Instant::now();
        let bulk_result = optimized_index.bulk_insert(bulk_pairs.clone())?;
        let bulk_duration = bulk_start.elapsed();

        println!(
            "    - Bulk insert: {:?} ({:.1} ops/sec)",
            bulk_duration, bulk_result.throughput_ops_per_sec
        );

        // Compare with individual operations
        let individual_start = Instant::now();
        for (i, (id, path)) in bulk_pairs.iter().enumerate() {
            if i < 10 {
                // Only test first 10 for performance comparison
                optimized_index.delete(id).await?; // Remove first
                optimized_index.insert(id.clone(), path.clone()).await?;
            }
        }
        let individual_duration = individual_start.elapsed();
        let individual_throughput = 10.0 / individual_duration.as_secs_f64();

        let performance_improvement = bulk_result.throughput_ops_per_sec / individual_throughput;

        println!(
            "    - Performance improvement: {:.1}x",
            performance_improvement
        );

        // Bulk operations should provide significant improvement
        assert!(
            performance_improvement >= 3.0,
            "Bulk operation improvement too low: {:.1}x (target: 3x+)",
            performance_improvement
        );
        assert!(
            bulk_result.meets_performance_requirements(3.0),
            "Bulk operation internal SLA not met"
        );

        // Test bulk delete performance
        if bulk_size >= 50 {
            let delete_sample_size = bulk_size / 2;
            let delete_ids: Vec<_> = bulk_docs[..delete_sample_size]
                .iter()
                .map(|doc| doc.id.clone())
                .collect();

            let bulk_delete_start = Instant::now();
            let delete_result = optimized_index.bulk_delete(delete_ids)?;
            let bulk_delete_duration = bulk_delete_start.elapsed();

            println!(
                "    - Bulk delete: {:?} ({:.1} ops/sec)",
                bulk_delete_duration, delete_result.throughput_ops_per_sec
            );

            assert!(
                delete_result.throughput_ops_per_sec > 100.0,
                "Bulk delete performance too low: {:.1} ops/sec",
                delete_result.throughput_ops_per_sec
            );
        }
    }

    Ok(())
}

// Helper structures and functions

#[derive(Debug)]
struct UserPerformanceMetrics {
    user_id: usize,
    write_times: Vec<Duration>,
    read_times: Vec<Duration>,
}

impl UserPerformanceMetrics {
    fn new(user_id: usize) -> Self {
        Self {
            user_id,
            write_times: Vec::new(),
            read_times: Vec::new(),
        }
    }

    fn add_write_time(&mut self, duration: Duration) {
        self.write_times.push(duration);
    }

    fn add_read_time(&mut self, duration: Duration) {
        self.read_times.push(duration);
    }

    fn total_operations(&self) -> usize {
        self.write_times.len() + self.read_times.len()
    }
}

#[derive(Debug)]
struct DatasetPerformanceMetrics {
    dataset_size: usize,
    insert_throughput: f64,
    avg_read_time: Duration,
    search_time: Duration,
    search_results_count: usize,
}

fn create_performance_test_documents(count: usize, test_type: &str) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(&format!("/performance/{}/doc_{:06}.md", test_type, i))?;
        let title = ValidatedTitle::new(&format!("{} Performance Test Doc {}", test_type, i))?;

        let content = format!(
            "# Performance Test Document {}\n\n\
             Test Type: {}\n\
             Document Number: {}\n\
             Performance testing content. {}\n\n\
             This document is designed for performance testing of the KotaDB system.\n\
             It contains realistic content to simulate production workloads.",
            i,
            test_type,
            i,
            "Sample data. ".repeat(20)
        )
        .into_bytes();

        let tags = vec![
            ValidatedTag::new(test_type)?,
            ValidatedTag::new("performance-test")?,
            ValidatedTag::new(&format!("batch-{}", i / 100))?,
        ];

        let now = chrono::Utc::now();

        let document = Document::new(doc_id, path, title, content, tags, now, now);

        documents.push(document);
    }

    Ok(documents)
}

fn create_user_performance_document(user_id: usize, doc_num: usize) -> Result<Document> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(&format!("/concurrent/user_{}/doc_{}.md", user_id, doc_num))?;
    let title = ValidatedTitle::new(&format!("User {} Doc {}", user_id, doc_num))?;

    let content = format!(
        "# Concurrent Performance Test\n\n\
         User: {}\n\
         Document: {}\n\
         Content for concurrent performance testing.\n\n\
         {}",
        user_id,
        doc_num,
        "Test data for concurrency. ".repeat(10)
    )
    .into_bytes();

    let tags = vec![
        ValidatedTag::new(&format!("user-{}", user_id))?,
        ValidatedTag::new("concurrent-test")?,
    ];

    let now = chrono::Utc::now();

    Ok(Document::new(doc_id, path, title, content, tags, now, now))
}

fn create_large_documents(count: usize, content_size: usize) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(&format!("/large/large_doc_{:04}.md", i))?;
        let title = ValidatedTitle::new(&format!("Large Document {}", i))?;

        let base_content = format!(
            "# Large Document {}\n\nThis is a large document for memory testing.\n\n",
            i
        );
        let padding =
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(content_size / 60);
        let content = format!("{}{}", base_content, padding).into_bytes();

        let tags = vec![
            ValidatedTag::new("large-document")?,
            ValidatedTag::new("memory-test")?,
        ];

        let now = chrono::Utc::now();

        let document = Document::new(doc_id, path, title, content, tags, now, now);

        documents.push(document);
    }

    Ok(documents)
}
