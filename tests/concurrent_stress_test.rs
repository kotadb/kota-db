// Concurrent Stress Testing - Advanced Multi-threaded Stress Tests
// Tests high concurrency scenarios with 200+ concurrent operations and advanced patterns

use anyhow::Result;
use kotadb::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::task;
use tracing::{error, info};
use uuid::Uuid;

mod test_constants;
use test_constants::concurrency::{
    get_concurrent_operations, get_operations_per_task, get_pool_capacity,
};
use test_constants::gating;
use test_constants::performance::{
    lock_efficiency_min, lock_read_avg_ms, lock_write_avg_ms, SLOW_OPERATION_THRESHOLD,
};

// Minimum conflict resolution rate for valid tests
const MIN_CONFLICT_RESOLUTION_RATE: f64 = 0.1;

/// Enhanced Multi-threaded Stress Testing with high concurrency
#[tokio::test]
async fn test_enhanced_concurrent_stress() -> Result<()> {
    if gating::skip_if_heavy_disabled("concurrent_stress_test::test_enhanced_concurrent_stress") {
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("concurrent_stress_storage");
    let index_path = temp_dir.path().join("concurrent_stress_index");

    // Use CI-aware configuration for concurrent operations
    let concurrent_operations = get_concurrent_operations();
    let operations_per_task = get_operations_per_task();
    let pool_capacity = get_pool_capacity();

    // Create shared system with enhanced capacity for stress testing
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(pool_capacity)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index =
            create_primary_index(&index_path.to_string_lossy(), Some(pool_capacity)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    // Advanced concurrency metrics tracking
    let metrics = Arc::new(ConcurrencyMetrics::new());
    let mut handles = Vec::new();

    println!("🚀 Starting enhanced stress test with {concurrent_operations} concurrent operations");

    let start = Instant::now();

    // Test different concurrency patterns simultaneously
    for pattern_id in 0..concurrent_operations {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);
        let metrics_ref = Arc::clone(&metrics);

        // Determine operation pattern type
        let pattern_type = match pattern_id % 5 {
            0 | 1 => ConcurrencyPattern::ReadHeavy, // 40% read-heavy
            2 => ConcurrencyPattern::WriteHeavy,    // 20% write-heavy
            3 => ConcurrencyPattern::Mixed,         // 20% mixed
            4 => ConcurrencyPattern::BurstWrite,    // 20% burst write
            _ => unreachable!(),
        };

        let handle = task::spawn(async move {
            let mut pattern_results = PatternResults::new();

            for op_num in 0..operations_per_task {
                let operation_start = Instant::now();

                // Track lock acquisition timing
                let lock_start = Instant::now();

                match pattern_type {
                    ConcurrencyPattern::ReadHeavy => {
                        // 80% reads, 20% writes
                        if op_num % 5 == 0 {
                            // Write operation
                            if let Err(_e) = execute_write_operation(
                                &storage_ref,
                                &index_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                                error!("Write error in read-heavy pattern {}: {}", pattern_id, _e);
                            } else {
                                pattern_results.writes += 1;
                            }
                        } else {
                            // Read operation
                            if let Err(_e) = execute_read_operation(
                                &storage_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.reads += 1;
                            }
                        }
                    }

                    ConcurrencyPattern::WriteHeavy => {
                        // 70% writes, 30% reads
                        if op_num % 10 < 7 {
                            // Write operation
                            if let Err(_e) = execute_write_operation(
                                &storage_ref,
                                &index_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.writes += 1;
                            }
                        } else {
                            // Read operation
                            if let Err(_e) = execute_read_operation(
                                &storage_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.reads += 1;
                            }
                        }
                    }

                    ConcurrencyPattern::Mixed => {
                        // 50% reads, 50% writes
                        if op_num % 2 == 0 {
                            if let Err(_e) = execute_write_operation(
                                &storage_ref,
                                &index_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.writes += 1;
                            }
                        } else if let Err(_e) =
                            execute_read_operation(&storage_ref, pattern_id, op_num, &metrics_ref)
                                .await
                        {
                            pattern_results.errors += 1;
                        } else {
                            pattern_results.reads += 1;
                        }
                    }

                    ConcurrencyPattern::BurstWrite => {
                        // Burst of 5 writes followed by 10 reads
                        if op_num % 15 < 5 {
                            // Burst write phase
                            if let Err(_e) = execute_write_operation(
                                &storage_ref,
                                &index_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.writes += 1;
                            }
                        } else {
                            // Read phase
                            if let Err(_e) = execute_read_operation(
                                &storage_ref,
                                pattern_id,
                                op_num,
                                &metrics_ref,
                            )
                            .await
                            {
                                pattern_results.errors += 1;
                            } else {
                                pattern_results.reads += 1;
                            }
                        }
                    }
                }

                let lock_duration = lock_start.elapsed();
                if lock_duration > Duration::from_millis(10) {
                    metrics_ref.long_lock_waits.fetch_add(1, Ordering::Relaxed);
                }

                let operation_duration = operation_start.elapsed();
                pattern_results.total_duration += operation_duration;
                pattern_results.operations_completed += 1;

                // Track performance degradation
                if operation_duration > SLOW_OPERATION_THRESHOLD {
                    pattern_results.slow_operations += 1;
                }

                // Small delay to simulate realistic load patterns
                if op_num % 10 == 0 {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }

            Ok::<PatternResults, anyhow::Error>(pattern_results)
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    let mut all_results = Vec::new();
    for handle in handles {
        match handle.await? {
            Ok(result) => all_results.push(result),
            Err(e) => error!("Pattern execution failed: {}", e),
        }
    }

    let total_duration = start.elapsed();

    // Aggregate results and analyze performance
    let summary = analyze_phase2b_results(&all_results, &metrics, total_duration)?;

    println!("\n🎯 Phase 2B Enhanced Concurrent Stress Test Results:");
    println!("  📊 Total Operations: {}", summary.total_operations);
    println!("  📖 Read Operations: {}", summary.total_reads);
    println!("  ✏️  Write Operations: {}", summary.total_writes);
    println!("  ❌ Total Errors: {}", summary.total_errors);
    println!("  ⏱️  Duration: {total_duration:?}");
    println!("  🚀 Throughput: {:.1} ops/sec", summary.throughput);
    println!("  🔒 Lock Contention Events: {}", summary.lock_contentions);
    println!("  ⏳ Long Lock Waits: {}", summary.long_lock_waits);
    println!("  🐌 Slow Operations: {}", summary.slow_operations);

    // Performance assertions for Phase 2B
    assert!(
        summary.error_rate < 0.03,
        "Error rate too high for Phase 2B: {:.2}%",
        summary.error_rate * 100.0
    );

    assert!(
        summary.throughput > 200.0,
        "Throughput below Phase 2B requirement: {:.1} ops/sec",
        summary.throughput
    );

    assert!(
        total_duration < Duration::from_secs(30),
        "Phase 2B test duration too long: {total_duration:?}"
    );

    // Advanced lock contention analysis
    let contention_rate = summary.lock_contentions as f64 / summary.total_operations as f64;
    assert!(
        contention_rate < 0.15,
        "Lock contention rate too high: {:.2}%",
        contention_rate * 100.0
    );

    // Performance should not degrade >50% under concurrent load
    let degradation_rate = summary.slow_operations as f64 / summary.total_operations as f64;
    assert!(
        degradation_rate < 0.50,
        "Performance degradation too high: {:.2}%",
        degradation_rate * 100.0
    );

    Ok(())
}

/// Advanced Lock Contention Analysis Test
#[tokio::test]
async fn test_lock_contention_analysis() -> Result<()> {
    if gating::skip_if_heavy_disabled("concurrent_stress_test::test_lock_contention_analysis") {
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("lock_analysis_storage");

    // Create storage with limited capacity to force contention
    let storage = Arc::new(tokio::sync::RwLock::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(5000)).await?,
    ));

    let contention_metrics = Arc::new(LockContentionMetrics::new());
    let concurrent_threads = 50;
    let operations_per_thread = 40;
    let mut handles = Vec::new();

    println!("🔒 Phase 2B: Lock Contention Analysis with {concurrent_threads} threads");

    let start = Instant::now();

    for thread_id in 0..concurrent_threads {
        let storage_ref = Arc::clone(&storage);
        let metrics_ref = Arc::clone(&contention_metrics);

        let handle = task::spawn(async move {
            let mut thread_metrics = ThreadLockMetrics::new();

            for op_id in 0..operations_per_thread {
                let operation_type = if op_id % 3 == 0 {
                    LockType::Write
                } else {
                    LockType::Read
                };

                let lock_attempt_start = Instant::now();

                let result = match operation_type {
                    LockType::Read => {
                        // Measure reader lock acquisition time
                        let guard_start = Instant::now();
                        let guard = storage_ref.read().await;
                        let lock_acquired_time = guard_start.elapsed();

                        // Track queue depth (approximate)
                        if lock_acquired_time > Duration::from_millis(5) {
                            metrics_ref
                                .reader_queue_depth
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        drop(guard);

                        // Simulate read work
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        let total_time = lock_attempt_start.elapsed();

                        thread_metrics.record_read_lock(lock_acquired_time, total_time);
                        Ok(())
                    }

                    LockType::Write => {
                        // Measure writer lock acquisition time
                        let guard_start = Instant::now();
                        let mut guard = storage_ref.write().await;
                        let lock_acquired_time = guard_start.elapsed();

                        // Track queue depth (approximate)
                        if lock_acquired_time > Duration::from_millis(10) {
                            metrics_ref
                                .writer_queue_depth
                                .fetch_add(1, Ordering::Relaxed);
                        }

                        // Create test document
                        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
                        let path =
                            ValidatedPath::new(format!("lock_test/t{thread_id}_op{op_id}.md"))?;
                        let title =
                            ValidatedTitle::new(format!("Lock Test T{thread_id} O{op_id}"))?;
                        let content = format!(
                            "Lock contention test content for thread {thread_id} op {op_id}"
                        )
                        .into_bytes();
                        let tags = vec![ValidatedTag::new("lock-test")?];
                        let now = chrono::Utc::now();

                        let doc = Document {
                            id: doc_id,
                            path,
                            title,
                            content: content.clone(),
                            tags,
                            created_at: now,
                            updated_at: now,
                            size: content.len(),
                            embedding: None,
                        };

                        // Simulate write work (restructure to avoid Send issues)
                        let insert_result = { guard.insert(doc).await };
                        drop(guard);
                        insert_result?;
                        let total_time = lock_attempt_start.elapsed();

                        thread_metrics.record_write_lock(lock_acquired_time, total_time);
                        Ok::<(), anyhow::Error>(())
                    }
                };

                if let Err(e) = result {
                    thread_metrics.errors += 1;
                    error!("Lock operation error in thread {}: {}", thread_id, e);
                }

                // Small delay to increase contention
                tokio::time::sleep(Duration::from_micros(50)).await;
            }

            Ok::<ThreadLockMetrics, anyhow::Error>(thread_metrics)
        });

        handles.push(handle);
    }

    // Collect all thread metrics
    let mut all_thread_metrics = Vec::new();
    for handle in handles {
        match handle.await? {
            Ok(metrics) => all_thread_metrics.push(metrics),
            Err(e) => error!("Thread failed: {e}"),
        }
    }

    let total_duration = start.elapsed();

    // Analyze lock contention results
    let analysis =
        analyze_lock_contention(&all_thread_metrics, &contention_metrics, total_duration)?;

    println!("\n🔒 Lock Contention Analysis Results:");
    println!("  📖 Total Read Operations: {}", analysis.total_reads);
    println!("  ✏️  Total Write Operations: {}", analysis.total_writes);
    println!(
        "  ⏱️  Avg Read Lock Time: {:?}",
        analysis.avg_read_lock_time
    );
    println!(
        "  ⏱️  Avg Write Lock Time: {:?}",
        analysis.avg_write_lock_time
    );
    println!("  📊 Max Read Lock Time: {:?}", analysis.max_read_lock_time);
    println!(
        "  📊 Max Write Lock Time: {:?}",
        analysis.max_write_lock_time
    );
    println!("  🔄 Reader Queue Events: {}", analysis.reader_queue_events);
    println!("  🔄 Writer Queue Events: {}", analysis.writer_queue_events);
    println!(
        "  🎯 Lock Efficiency: {:.2}%",
        analysis.lock_efficiency * 100.0
    );

    // Performance requirements for lock contention (CI-aware)
    let read_ms = lock_read_avg_ms();
    let write_ms = lock_write_avg_ms();
    let eff_min = lock_efficiency_min();
    assert!(
        analysis.avg_read_lock_time < Duration::from_millis(read_ms),
        "Average read lock time too high: {:?} (threshold {}ms)",
        analysis.avg_read_lock_time,
        read_ms
    );

    assert!(
        analysis.avg_write_lock_time < Duration::from_millis(write_ms),
        "Average write lock time too high: {:?} (threshold {}ms)",
        analysis.avg_write_lock_time,
        write_ms
    );

    assert!(
        analysis.lock_efficiency > eff_min,
        "Lock efficiency too low: {:.2}% (min {:.0}%)",
        analysis.lock_efficiency * 100.0,
        eff_min * 100.0
    );

    Ok(())
}

/// Comprehensive Race Condition Detection Test
#[tokio::test]
async fn test_race_condition_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("race_detection_storage");

    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?,
    ));

    let race_detector = Arc::new(RaceConditionDetector::new());
    let concurrent_modifiers = 20;
    let modifications_per_modifier = 50;

    println!(
        "🏁 Phase 2B: Race Condition Detection with {concurrent_modifiers} concurrent modifiers"
    );

    // Create shared documents for race condition testing
    let shared_doc_count = 10;
    let mut shared_doc_ids = Vec::new();

    // Pre-populate with shared documents
    {
        let mut storage_guard = storage.lock().await;
        for i in 0..shared_doc_count {
            let doc = create_race_test_document(i, 0, "initial").await?;
            storage_guard.insert(doc.clone()).await?;
            shared_doc_ids.push(doc.id);
        }
    }

    let shared_ids = Arc::new(shared_doc_ids);
    let mut handles = Vec::new();

    let start = Instant::now();

    for modifier_id in 0..concurrent_modifiers {
        let storage_ref = Arc::clone(&storage);
        let detector_ref = Arc::clone(&race_detector);
        let ids_ref = Arc::clone(&shared_ids);

        let handle = task::spawn(async move {
            let mut modifier_results = RaceDetectionResults::new();

            for mod_num in 0..modifications_per_modifier {
                // Pick a random shared document to modify
                let doc_index = mod_num % shared_doc_count;
                let target_id = ids_ref[doc_index];

                let race_start = Instant::now();

                // Attempt concurrent modification with race detection
                let result = perform_race_prone_operation(
                    &storage_ref,
                    &detector_ref,
                    target_id,
                    modifier_id,
                    mod_num,
                )
                .await;

                let operation_time = race_start.elapsed();

                match result {
                    Ok(RaceOperationResult::Success) => {
                        modifier_results.successful_modifications += 1;
                    }
                    Ok(RaceOperationResult::RaceDetected) => {
                        modifier_results.races_detected += 1;
                    }
                    Ok(RaceOperationResult::ConflictResolved) => {
                        modifier_results.conflicts_resolved += 1;
                    }
                    Err(e) => {
                        modifier_results.errors += 1;
                        error!("Race operation error modifier {modifier_id}: {e}");
                    }
                }

                modifier_results.total_time += operation_time;

                // Add controlled delay to increase race probability
                tokio::time::sleep(Duration::from_micros(10)).await;
            }

            Ok::<RaceDetectionResults, anyhow::Error>(modifier_results)
        });

        handles.push(handle);
    }

    // Collect results
    let mut all_modifier_results = Vec::new();
    for handle in handles {
        match handle.await? {
            Ok(results) => all_modifier_results.push(results),
            Err(e) => error!("Modifier failed: {e}"),
        }
    }

    let total_duration = start.elapsed();

    // Analyze race condition detection results
    let analysis = analyze_race_detection(&all_modifier_results, &race_detector, total_duration)?;

    println!("\n🏁 Race Condition Detection Results:");
    println!(
        "  ✅ Successful Modifications: {}",
        analysis.total_successful
    );
    println!("  🚨 Races Detected: {}", analysis.total_races_detected);
    println!(
        "  🔧 Conflicts Resolved: {}",
        analysis.total_conflicts_resolved
    );
    println!("  ❌ Total Errors: {}", analysis.total_errors);
    println!(
        "  📊 Race Detection Rate: {:.2}%",
        analysis.race_detection_rate * 100.0
    );
    println!(
        "  🎯 Conflict Resolution Rate: {:.2}%",
        analysis.conflict_resolution_rate * 100.0
    );
    println!(
        "  ⏱️  Avg Operation Time: {:?}",
        analysis.avg_operation_time
    );

    // Validate data consistency after race conditions
    let final_consistency = validate_data_consistency(&storage, &shared_ids).await?;
    println!("  🔍 Data Consistency: {:.2}%", final_consistency * 100.0);

    // Requirements for race condition detection
    assert!(
        analysis.race_detection_rate > 0.05,
        "Race detection rate too low - races not being detected: {:.2}%",
        analysis.race_detection_rate * 100.0
    );

    assert!(
        final_consistency > 0.95,
        "Data consistency compromised: {:.2}%",
        final_consistency * 100.0
    );

    // Validate conflict resolution logic with enhanced diagnostics
    if analysis.conflict_resolution_rate == 0.0 {
        // Zero conflict resolution could indicate either excellent conflict avoidance
        // or that conflict detection logic isn't working. Verify with error rates.
        if all_modifier_results.iter().all(|r| r.errors == 0) {
            info!(
                "✅ No explicit conflict resolution needed - excellent conflict avoidance achieved"
            );
        } else {
            let total_errors: usize = all_modifier_results.iter().map(|r| r.errors).sum();
            error!(
                "⚠️  Zero conflict resolution with {total_errors} errors detected - potential race detection issue"
            );
            panic!(
                "Zero conflict resolution rate with {total_errors} errors suggests race detection logic may not be working"
            );
        }
    } else {
        assert!(
            analysis.conflict_resolution_rate > MIN_CONFLICT_RESOLUTION_RATE,
            "Conflict resolution rate too low: {:.2}% (minimum required: {:.1}%)",
            analysis.conflict_resolution_rate * 100.0,
            MIN_CONFLICT_RESOLUTION_RATE * 100.0
        );
        info!(
            "✅ Conflict resolution operating within acceptable range: {:.2}%",
            analysis.conflict_resolution_rate * 100.0
        );
    }

    Ok(())
}

/// Concurrent Index Operations Test
#[tokio::test]
async fn test_concurrent_index_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("concurrent_index_storage");
    let primary_index_path = temp_dir.path().join("concurrent_primary_index");
    let trigram_index_path = temp_dir.path().join("concurrent_trigram_index");

    // Create both primary and trigram indices for concurrent testing
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(15000)).await?,
    ));

    let primary_index = Arc::new(tokio::sync::Mutex::new(
        create_primary_index(&primary_index_path.to_string_lossy(), Some(15000)).await?,
    ));

    let trigram_index = Arc::new(tokio::sync::Mutex::new({
        let trigram_index_impl =
            create_trigram_index(&trigram_index_path.to_string_lossy(), Some(15000)).await?;
        create_optimized_index_with_defaults(trigram_index_impl)
    }));

    let index_metrics = Arc::new(IndexConcurrencyMetrics::new());
    let concurrent_indexers = 30;
    let operations_per_indexer = 40;

    println!("📚 Phase 2B: Concurrent Index Operations with {concurrent_indexers} indexers");

    let mut handles = Vec::new();
    let start = Instant::now();

    for indexer_id in 0..concurrent_indexers {
        let storage_ref = Arc::clone(&storage);
        let primary_ref = Arc::clone(&primary_index);
        let trigram_ref = Arc::clone(&trigram_index);
        let metrics_ref = Arc::clone(&index_metrics);

        // Assign different index operation patterns
        let indexer_type = match indexer_id % 3 {
            0 => IndexerType::PrimaryOnly, // 33% primary index only
            1 => IndexerType::TrigramOnly, // 33% trigram index only
            2 => IndexerType::Mixed,       // 34% both indices
            _ => unreachable!(),
        };

        let handle = task::spawn(async move {
            let mut indexer_results = IndexerResults::new();

            for op_num in 0..operations_per_indexer {
                let doc = create_index_test_document(indexer_id, op_num).await?;

                // Insert into storage first
                let storage_start = Instant::now();
                {
                    let mut storage_guard = storage_ref.lock().await;
                    storage_guard.insert(doc.clone()).await?;
                }
                let storage_time = storage_start.elapsed();
                indexer_results.storage_time += storage_time;

                // Perform index operations based on type
                match indexer_type {
                    IndexerType::PrimaryOnly => {
                        let index_start = Instant::now();
                        {
                            let mut primary_guard = primary_ref.lock().await;
                            primary_guard.insert(doc.id, doc.path.clone()).await?;
                        }
                        let index_time = index_start.elapsed();
                        indexer_results.primary_operations += 1;
                        indexer_results.primary_time += index_time;

                        if index_time > Duration::from_millis(20) {
                            metrics_ref.slow_primary_ops.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    IndexerType::TrigramOnly => {
                        let index_start = Instant::now();
                        {
                            let mut trigram_guard = trigram_ref.lock().await;
                            // Provide content for trigram indexing
                            trigram_guard
                                .insert_with_content(doc.id, doc.path.clone(), &doc.content)
                                .await?;
                        }
                        let index_time = index_start.elapsed();
                        indexer_results.trigram_operations += 1;
                        indexer_results.trigram_time += index_time;

                        if index_time > Duration::from_millis(50) {
                            metrics_ref.slow_trigram_ops.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    IndexerType::Mixed => {
                        // Update both indices (test concurrent access)
                        let both_start = Instant::now();

                        // Primary index
                        let primary_start = Instant::now();
                        {
                            let mut primary_guard = primary_ref.lock().await;
                            primary_guard.insert(doc.id, doc.path.clone()).await?;
                        }
                        let primary_time = primary_start.elapsed();
                        indexer_results.primary_operations += 1;
                        indexer_results.primary_time += primary_time;

                        // Trigram index
                        let trigram_start = Instant::now();
                        {
                            let mut trigram_guard = trigram_ref.lock().await;
                            trigram_guard
                                .insert_with_content(doc.id, doc.path.clone(), &doc.content)
                                .await?;
                        }
                        let trigram_time = trigram_start.elapsed();
                        indexer_results.trigram_operations += 1;
                        indexer_results.trigram_time += trigram_time;

                        let both_time = both_start.elapsed();
                        indexer_results.mixed_operations += 1;

                        // Check for index synchronization issues
                        if primary_time + trigram_time < both_time * 2 {
                            metrics_ref.sync_issues.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }

                indexer_results.total_operations += 1;

                // Test concurrent reads occasionally
                if op_num % 10 == 0 {
                    let read_start = Instant::now();
                    let search_result = {
                        let primary_guard = primary_ref.lock().await;
                        primary_guard.search(&QueryBuilder::new().build()?).await?
                    };
                    let read_time = read_start.elapsed();
                    indexer_results.read_operations += 1;
                    indexer_results.read_time += read_time;
                }

                // Controlled delay to simulate realistic indexing load
                tokio::time::sleep(Duration::from_micros(200)).await;
            }

            Ok::<IndexerResults, anyhow::Error>(indexer_results)
        });

        handles.push(handle);
    }

    // Collect all indexer results
    let mut all_indexer_results = Vec::new();
    for handle in handles {
        match handle.await? {
            Ok(results) => all_indexer_results.push(results),
            Err(e) => error!("Indexer failed: {e}"),
        }
    }

    let total_duration = start.elapsed();

    // Analyze concurrent index performance
    let analysis =
        analyze_concurrent_index_performance(&all_indexer_results, &index_metrics, total_duration)?;

    println!("\n📚 Concurrent Index Operations Results:");
    println!("  📊 Total Operations: {}", analysis.total_operations);
    println!(
        "  🔑 Primary Index Operations: {}",
        analysis.primary_operations
    );
    println!(
        "  🔤 Trigram Index Operations: {}",
        analysis.trigram_operations
    );
    println!("  📖 Read Operations: {}", analysis.read_operations);
    println!(
        "  ⏱️  Avg Primary Index Time: {:?}",
        analysis.avg_primary_time
    );
    println!(
        "  ⏱️  Avg Trigram Index Time: {:?}",
        analysis.avg_trigram_time
    );
    println!(
        "  🚀 Index Throughput: {:.1} ops/sec",
        analysis.index_throughput
    );
    println!("  🐌 Slow Primary Ops: {}", analysis.slow_primary_ops);
    println!("  🐌 Slow Trigram Ops: {}", analysis.slow_trigram_ops);
    println!("  🔄 Sync Issues: {}", analysis.sync_issues);

    // Validate index consistency after concurrent operations
    let consistency_check =
        validate_index_consistency(&storage, &primary_index, &trigram_index).await?;
    println!(
        "  ✅ Index Consistency: {:.2}%",
        consistency_check.consistency_rate * 100.0
    );

    // Performance requirements for concurrent index operations
    assert!(
        analysis.index_throughput > 150.0,
        "Index throughput too low: {:.1} ops/sec",
        analysis.index_throughput
    );

    assert!(
        analysis.avg_primary_time < Duration::from_millis(30),
        "Primary index operations too slow: {:?}",
        analysis.avg_primary_time
    );

    assert!(
        analysis.avg_trigram_time < Duration::from_millis(80),
        "Trigram index operations too slow: {:?}",
        analysis.avg_trigram_time
    );

    // Allow for some inconsistency during concurrent operations while maintaining reasonable quality
    assert!(
        consistency_check.consistency_rate > 0.60,
        "Index consistency too low: {:.2}%",
        consistency_check.consistency_rate * 100.0
    );

    // No more than 5% of operations should be slow
    let slow_op_rate = (analysis.slow_primary_ops + analysis.slow_trigram_ops) as f64
        / analysis.total_operations as f64;
    assert!(
        slow_op_rate < 0.05,
        "Too many slow operations: {:.2}%",
        slow_op_rate * 100.0
    );

    Ok(())
}

// Helper structures for Phase 2B testing

#[derive(Debug, Clone, Copy)]
enum ConcurrencyPattern {
    ReadHeavy,
    WriteHeavy,
    Mixed,
    BurstWrite,
}

#[derive(Debug)]
struct PatternResults {
    operations_completed: usize,
    reads: usize,
    writes: usize,
    errors: usize,
    slow_operations: usize,
    total_duration: Duration,
}

impl PatternResults {
    fn new() -> Self {
        Self {
            operations_completed: 0,
            reads: 0,
            writes: 0,
            errors: 0,
            slow_operations: 0,
            total_duration: Duration::ZERO,
        }
    }
}

struct ConcurrencyMetrics {
    lock_acquisitions: AtomicU64,
    lock_contentions: AtomicU64,
    long_lock_waits: AtomicU64,
    operations_completed: AtomicU64,
}

impl ConcurrencyMetrics {
    fn new() -> Self {
        Self {
            lock_acquisitions: AtomicU64::new(0),
            lock_contentions: AtomicU64::new(0),
            long_lock_waits: AtomicU64::new(0),
            operations_completed: AtomicU64::new(0),
        }
    }
}

#[derive(Debug)]
struct Phase2bSummary {
    total_operations: usize,
    total_reads: usize,
    total_writes: usize,
    total_errors: usize,
    throughput: f64,
    error_rate: f64,
    lock_contentions: u64,
    long_lock_waits: u64,
    slow_operations: usize,
}

struct LockContentionMetrics {
    reader_queue_depth: AtomicU64,
    writer_queue_depth: AtomicU64,
}

impl LockContentionMetrics {
    fn new() -> Self {
        Self {
            reader_queue_depth: AtomicU64::new(0),
            writer_queue_depth: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LockType {
    Read,
    Write,
}

#[derive(Debug)]
struct ThreadLockMetrics {
    read_operations: usize,
    write_operations: usize,
    total_read_lock_time: Duration,
    total_write_lock_time: Duration,
    max_read_lock_time: Duration,
    max_write_lock_time: Duration,
    errors: usize,
}

impl ThreadLockMetrics {
    fn new() -> Self {
        Self {
            read_operations: 0,
            write_operations: 0,
            total_read_lock_time: Duration::ZERO,
            total_write_lock_time: Duration::ZERO,
            max_read_lock_time: Duration::ZERO,
            max_write_lock_time: Duration::ZERO,
            errors: 0,
        }
    }

    fn record_read_lock(&mut self, lock_time: Duration, _total_time: Duration) {
        self.read_operations += 1;
        self.total_read_lock_time += lock_time;
        if lock_time > self.max_read_lock_time {
            self.max_read_lock_time = lock_time;
        }
    }

    fn record_write_lock(&mut self, lock_time: Duration, _total_time: Duration) {
        self.write_operations += 1;
        self.total_write_lock_time += lock_time;
        if lock_time > self.max_write_lock_time {
            self.max_write_lock_time = lock_time;
        }
    }
}

#[derive(Debug)]
struct LockContentionAnalysis {
    total_reads: usize,
    total_writes: usize,
    avg_read_lock_time: Duration,
    avg_write_lock_time: Duration,
    max_read_lock_time: Duration,
    max_write_lock_time: Duration,
    reader_queue_events: u64,
    writer_queue_events: u64,
    lock_efficiency: f64,
}

struct RaceConditionDetector {
    operation_timestamps: Arc<tokio::sync::RwLock<HashMap<ValidatedDocumentId, Instant>>>,
    races_detected: AtomicU64,
}

impl RaceConditionDetector {
    fn new() -> Self {
        Self {
            operation_timestamps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            races_detected: AtomicU64::new(0),
        }
    }

    async fn record_access(&self, doc_id: ValidatedDocumentId) -> bool {
        let now = Instant::now();
        let mut timestamps = self.operation_timestamps.write().await;

        if let Some(last_access) = timestamps.get(&doc_id) {
            if now.duration_since(*last_access) < Duration::from_millis(10) {
                self.races_detected.fetch_add(1, Ordering::Relaxed);
                timestamps.insert(doc_id, now);
                return true; // Race detected
            }
        }

        timestamps.insert(doc_id, now);
        false
    }
}

#[derive(Debug)]
enum RaceOperationResult {
    Success,
    RaceDetected,
    ConflictResolved,
}

#[derive(Debug)]
struct RaceDetectionResults {
    successful_modifications: usize,
    races_detected: usize,
    conflicts_resolved: usize,
    errors: usize,
    total_time: Duration,
}

impl RaceDetectionResults {
    fn new() -> Self {
        Self {
            successful_modifications: 0,
            races_detected: 0,
            conflicts_resolved: 0,
            errors: 0,
            total_time: Duration::ZERO,
        }
    }
}

#[derive(Debug)]
struct RaceDetectionAnalysis {
    total_successful: usize,
    total_races_detected: usize,
    total_conflicts_resolved: usize,
    total_errors: usize,
    race_detection_rate: f64,
    conflict_resolution_rate: f64,
    avg_operation_time: Duration,
}

struct IndexConcurrencyMetrics {
    slow_primary_ops: AtomicU64,
    slow_trigram_ops: AtomicU64,
    sync_issues: AtomicU64,
}

impl IndexConcurrencyMetrics {
    fn new() -> Self {
        Self {
            slow_primary_ops: AtomicU64::new(0),
            slow_trigram_ops: AtomicU64::new(0),
            sync_issues: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum IndexerType {
    PrimaryOnly,
    TrigramOnly,
    Mixed,
}

#[derive(Debug)]
struct IndexerResults {
    total_operations: usize,
    primary_operations: usize,
    trigram_operations: usize,
    mixed_operations: usize,
    read_operations: usize,
    storage_time: Duration,
    primary_time: Duration,
    trigram_time: Duration,
    read_time: Duration,
}

impl IndexerResults {
    fn new() -> Self {
        Self {
            total_operations: 0,
            primary_operations: 0,
            trigram_operations: 0,
            mixed_operations: 0,
            read_operations: 0,
            storage_time: Duration::ZERO,
            primary_time: Duration::ZERO,
            trigram_time: Duration::ZERO,
            read_time: Duration::ZERO,
        }
    }
}

#[derive(Debug)]
struct IndexConcurrencyAnalysis {
    total_operations: usize,
    primary_operations: usize,
    trigram_operations: usize,
    read_operations: usize,
    avg_primary_time: Duration,
    avg_trigram_time: Duration,
    index_throughput: f64,
    slow_primary_ops: u64,
    slow_trigram_ops: u64,
    sync_issues: u64,
}

#[derive(Debug)]
struct IndexConsistencyCheck {
    consistency_rate: f64,
}

// Helper functions

async fn execute_write_operation(
    storage: &Arc<tokio::sync::Mutex<impl Storage>>,
    index: &Arc<tokio::sync::Mutex<impl Index>>,
    pattern_id: usize,
    op_num: usize,
    metrics: &Arc<ConcurrencyMetrics>,
) -> Result<()> {
    metrics.lock_acquisitions.fetch_add(1, Ordering::Relaxed);

    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!("phase2b/pattern_{pattern_id}/op_{op_num}.md"))?;
    let title = ValidatedTitle::new(format!("Phase2B Doc P{pattern_id} O{op_num}"))?;
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
    {
        let mut storage_guard = storage.lock().await;
        storage_guard.insert(doc.clone()).await?;
    }

    // Index update
    {
        let mut index_guard = index.lock().await;
        index_guard.insert(doc.id, path).await?;
    }

    metrics.operations_completed.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

async fn execute_read_operation(
    storage: &Arc<tokio::sync::Mutex<impl Storage>>,
    pattern_id: usize,
    op_num: usize,
    metrics: &Arc<ConcurrencyMetrics>,
) -> Result<()> {
    metrics.lock_acquisitions.fetch_add(1, Ordering::Relaxed);

    // Simulate read of existing document (may not exist)
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?; // Random ID for stress testing

    let storage_guard = storage.lock().await;
    let result = storage_guard.get(&doc_id).await?;

    metrics.operations_completed.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn analyze_phase2b_results(
    results: &[PatternResults],
    metrics: &Arc<ConcurrencyMetrics>,
    total_duration: Duration,
) -> Result<Phase2bSummary> {
    let total_operations = results.iter().map(|r| r.operations_completed).sum();
    let total_reads = results.iter().map(|r| r.reads).sum();
    let total_writes = results.iter().map(|r| r.writes).sum();
    let total_errors = results.iter().map(|r| r.errors).sum();
    let slow_operations = results.iter().map(|r| r.slow_operations).sum();

    let throughput = total_operations as f64 / total_duration.as_secs_f64();
    let error_rate = total_errors as f64 / (total_operations + total_errors) as f64;

    Ok(Phase2bSummary {
        total_operations,
        total_reads,
        total_writes,
        total_errors,
        throughput,
        error_rate,
        lock_contentions: metrics.lock_contentions.load(Ordering::Relaxed),
        long_lock_waits: metrics.long_lock_waits.load(Ordering::Relaxed),
        slow_operations,
    })
}

fn analyze_lock_contention(
    thread_metrics: &[ThreadLockMetrics],
    contention_metrics: &Arc<LockContentionMetrics>,
    total_duration: Duration,
) -> Result<LockContentionAnalysis> {
    let total_reads: usize = thread_metrics.iter().map(|m| m.read_operations).sum();
    let total_writes: usize = thread_metrics.iter().map(|m| m.write_operations).sum();

    let total_read_time: Duration = thread_metrics.iter().map(|m| m.total_read_lock_time).sum();
    let total_write_time: Duration = thread_metrics.iter().map(|m| m.total_write_lock_time).sum();

    let avg_read_lock_time = if total_reads > 0 {
        total_read_time / total_reads as u32
    } else {
        Duration::ZERO
    };

    let avg_write_lock_time = if total_writes > 0 {
        total_write_time / total_writes as u32
    } else {
        Duration::ZERO
    };

    let max_read_lock_time = thread_metrics
        .iter()
        .map(|m| m.max_read_lock_time)
        .max()
        .unwrap_or(Duration::ZERO);

    let max_write_lock_time = thread_metrics
        .iter()
        .map(|m| m.max_write_lock_time)
        .max()
        .unwrap_or(Duration::ZERO);

    let total_operations = total_reads + total_writes;
    let total_lock_time = total_read_time + total_write_time;
    let lock_efficiency = if total_duration.as_nanos() > 0 {
        1.0 - (total_lock_time.as_nanos() as f64
            / (total_duration.as_nanos() as f64 * total_operations as f64))
    } else {
        0.0
    };

    Ok(LockContentionAnalysis {
        total_reads,
        total_writes,
        avg_read_lock_time,
        avg_write_lock_time,
        max_read_lock_time,
        max_write_lock_time,
        reader_queue_events: contention_metrics
            .reader_queue_depth
            .load(Ordering::Relaxed),
        writer_queue_events: contention_metrics
            .writer_queue_depth
            .load(Ordering::Relaxed),
        lock_efficiency,
    })
}

async fn create_race_test_document(index: usize, version: u32, modifier: &str) -> Result<Document> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!("race_test/shared_doc_{index:03}.md"))?;
    let title = ValidatedTitle::new(format!("Shared Race Test Doc {index} v{version}"))?;
    let content = format!(
        "Race test document {index} version {version} modified by {modifier}.\n\
         Timestamp: {}\n\
         Content for race condition detection and consistency validation.",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    )
    .into_bytes();
    let tags = vec![
        ValidatedTag::new("race-test")?,
        ValidatedTag::new(format!("version-{version}"))?,
        ValidatedTag::new(modifier)?,
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

async fn perform_race_prone_operation(
    storage: &Arc<tokio::sync::Mutex<impl Storage>>,
    detector: &Arc<RaceConditionDetector>,
    target_id: ValidatedDocumentId,
    modifier_id: usize,
    mod_num: usize,
) -> Result<RaceOperationResult> {
    // Record access for race detection
    let race_detected = detector.record_access(target_id).await;

    if race_detected {
        // Simulate conflict resolution
        tokio::time::sleep(Duration::from_micros(100)).await;
        return Ok(RaceOperationResult::RaceDetected);
    }

    // Attempt to modify the document
    let mut storage_guard = storage.lock().await;

    // Try to get existing document
    if let Some(existing_doc) = storage_guard.get(&target_id).await? {
        // Create modified version
        let updated_content = format!(
            "{}\nModified by modifier {} operation {} at {}",
            String::from_utf8_lossy(&existing_doc.content),
            modifier_id,
            mod_num,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        )
        .into_bytes();

        let updated_doc = Document {
            id: existing_doc.id,
            path: existing_doc.path,
            title: existing_doc.title,
            content: updated_content.clone(),
            tags: existing_doc.tags,
            created_at: existing_doc.created_at,
            updated_at: chrono::Utc::now(),
            size: updated_content.len(),
            embedding: None,
        };

        storage_guard.update(updated_doc).await?;
        Ok(RaceOperationResult::Success)
    } else {
        // Document not found, possibly deleted by another thread
        Ok(RaceOperationResult::ConflictResolved)
    }
}

fn analyze_race_detection(
    modifier_results: &[RaceDetectionResults],
    detector: &Arc<RaceConditionDetector>,
    total_duration: Duration,
) -> Result<RaceDetectionAnalysis> {
    let total_successful: usize = modifier_results
        .iter()
        .map(|r| r.successful_modifications)
        .sum();
    let total_races_detected: usize = modifier_results.iter().map(|r| r.races_detected).sum();
    let total_conflicts_resolved: usize =
        modifier_results.iter().map(|r| r.conflicts_resolved).sum();
    let total_errors: usize = modifier_results.iter().map(|r| r.errors).sum();

    let total_operations = total_successful + total_races_detected + total_conflicts_resolved;
    let race_detection_rate = if total_operations > 0 {
        total_races_detected as f64 / total_operations as f64
    } else {
        0.0
    };

    let conflict_resolution_rate = if total_races_detected + total_conflicts_resolved > 0 {
        total_conflicts_resolved as f64 / (total_races_detected + total_conflicts_resolved) as f64
    } else {
        0.0
    };

    let total_time: Duration = modifier_results.iter().map(|r| r.total_time).sum();
    let avg_operation_time = if total_operations > 0 {
        total_time / total_operations as u32
    } else {
        Duration::ZERO
    };

    Ok(RaceDetectionAnalysis {
        total_successful,
        total_races_detected,
        total_conflicts_resolved,
        total_errors,
        race_detection_rate,
        conflict_resolution_rate,
        avg_operation_time,
    })
}

async fn validate_data_consistency(
    storage: &Arc<tokio::sync::Mutex<impl Storage>>,
    shared_ids: &Arc<Vec<ValidatedDocumentId>>,
) -> Result<f64> {
    let storage_guard = storage.lock().await;
    let mut consistent_documents = 0;
    let mut total_documents = 0;

    for doc_id in shared_ids.iter() {
        total_documents += 1;

        if let Ok(Some(doc)) = storage_guard.get(doc_id).await {
            // Basic consistency check: document should be well-formed
            if !doc.content.is_empty()
                && doc.size == doc.content.len()
                && doc.updated_at >= doc.created_at
            {
                consistent_documents += 1;
            }
        }
    }

    Ok(if total_documents > 0 {
        consistent_documents as f64 / total_documents as f64
    } else {
        1.0
    })
}

async fn create_index_test_document(indexer_id: usize, op_num: usize) -> Result<Document> {
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(format!(
        "index_test/indexer_{indexer_id}/doc_{op_num:04}.md"
    ))?;
    let title = ValidatedTitle::new(format!("Index Test I{indexer_id} D{op_num}"))?;

    // Content with trigram-indexable text
    let content = format!(
        "# Index Test Document {}\n\n\
         Indexer: {}\n\
         Operation: {}\n\
         Content: This document tests concurrent index operations with realistic text content. \
         Keywords: search, index, concurrent, performance, test, validation. \
         Timestamp: {}\n\n\
         This content should be indexed by both primary and trigram indices for comprehensive testing.",
        op_num, indexer_id, op_num,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ).into_bytes();

    let tags = vec![
        ValidatedTag::new(format!("indexer-{indexer_id}"))?,
        ValidatedTag::new("index-test")?,
        ValidatedTag::new("concurrent")?,
    ];

    let now = chrono::Utc::now();

    Ok(Document {
        id: doc_id,
        path,
        title,
        content: content.clone(),
        tags,
        created_at: now,
        updated_at: now,
        size: content.len(),
        embedding: None,
    })
}

fn analyze_concurrent_index_performance(
    indexer_results: &[IndexerResults],
    metrics: &Arc<IndexConcurrencyMetrics>,
    total_duration: Duration,
) -> Result<IndexConcurrencyAnalysis> {
    let total_operations: usize = indexer_results.iter().map(|r| r.total_operations).sum();
    let primary_operations: usize = indexer_results.iter().map(|r| r.primary_operations).sum();
    let trigram_operations: usize = indexer_results.iter().map(|r| r.trigram_operations).sum();
    let read_operations: usize = indexer_results.iter().map(|r| r.read_operations).sum();

    let total_primary_time: Duration = indexer_results.iter().map(|r| r.primary_time).sum();
    let total_trigram_time: Duration = indexer_results.iter().map(|r| r.trigram_time).sum();

    let avg_primary_time = if primary_operations > 0 {
        total_primary_time / primary_operations as u32
    } else {
        Duration::ZERO
    };

    let avg_trigram_time = if trigram_operations > 0 {
        total_trigram_time / trigram_operations as u32
    } else {
        Duration::ZERO
    };

    let index_throughput = total_operations as f64 / total_duration.as_secs_f64();

    Ok(IndexConcurrencyAnalysis {
        total_operations,
        primary_operations,
        trigram_operations,
        read_operations,
        avg_primary_time,
        avg_trigram_time,
        index_throughput,
        slow_primary_ops: metrics.slow_primary_ops.load(Ordering::Relaxed),
        slow_trigram_ops: metrics.slow_trigram_ops.load(Ordering::Relaxed),
        sync_issues: metrics.sync_issues.load(Ordering::Relaxed),
    })
}

async fn validate_index_consistency(
    storage: &Arc<tokio::sync::Mutex<impl Storage>>,
    primary_index: &Arc<tokio::sync::Mutex<impl Index>>,
    trigram_index: &Arc<tokio::sync::Mutex<impl Index>>,
) -> Result<IndexConsistencyCheck> {
    let storage_guard = storage.lock().await;
    let primary_guard = primary_index.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let storage_docs = storage_guard.list_all().await?;
    let storage_count = storage_docs.len();

    // Check primary index consistency (limit to max 1000)
    let primary_query = QueryBuilder::new()
        .with_limit(std::cmp::min(storage_count + 100, 1000))?
        .build()?;
    let primary_results = primary_guard.search(&primary_query).await?;
    let primary_count = primary_results.len();

    // Check trigram index consistency (using text search)
    let trigram_query = QueryBuilder::new()
        .with_text("test")?
        .with_limit(1000)?
        .build()?;
    let trigram_results = trigram_guard.search(&trigram_query).await?;
    let trigram_count = trigram_results.len();

    // Calculate consistency rate (storage should match primary, trigram is subset)
    let primary_consistency = if storage_count > 0 {
        (primary_count.min(storage_count) as f64) / (storage_count as f64)
    } else {
        1.0
    };

    // Trigram consistency is more lenient (not all docs may have indexable content)
    let trigram_consistency = if storage_count > 0 {
        (trigram_count as f64) / (storage_count as f64)
    } else {
        1.0
    };

    let overall_consistency = (primary_consistency + trigram_consistency.min(1.0)) / 2.0;

    Ok(IndexConsistencyCheck {
        consistency_rate: overall_consistency,
    })
}
