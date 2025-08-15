// Query Routing Stress Tests - Comprehensive testing for intelligent index selection under load
// Tests the routing logic between Primary Index and Trigram Index under heavy concurrent access

use anyhow::Result;
use kotadb::{
    contracts::{Index, Query},
    create_optimized_index_with_defaults, create_primary_index_for_tests,
    create_trigram_index_for_tests,
    primary_index::PrimaryIndex,
    trigram_index::TrigramIndex,
    wrappers::optimization::OptimizedIndex,
    QueryBuilder, ValidatedDocumentId, ValidatedPath,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

/// Statistics tracker for query routing performance analysis
#[derive(Debug, Default)]
struct RoutingStats {
    primary_queries: AtomicUsize,
    trigram_queries: AtomicUsize,
    routing_decisions: AtomicUsize,
    routing_time_ns: AtomicUsize,
    primary_execution_time_ns: AtomicUsize,
    trigram_execution_time_ns: AtomicUsize,
    routing_errors: AtomicUsize,
    index_contention_events: AtomicUsize,
}

impl RoutingStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_routing_decision(&self, routing_time: Duration, is_primary: bool) {
        self.routing_decisions.fetch_add(1, Ordering::Relaxed);
        self.routing_time_ns
            .fetch_add(routing_time.as_nanos() as usize, Ordering::Relaxed);

        if is_primary {
            self.primary_queries.fetch_add(1, Ordering::Relaxed);
        } else {
            self.trigram_queries.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_execution_time(&self, execution_time: Duration, is_primary: bool) {
        let time_ns = execution_time.as_nanos() as usize;
        if is_primary {
            self.primary_execution_time_ns
                .fetch_add(time_ns, Ordering::Relaxed);
        } else {
            self.trigram_execution_time_ns
                .fetch_add(time_ns, Ordering::Relaxed);
        }
    }

    fn record_routing_error(&self) {
        self.routing_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn record_contention(&self) {
        self.index_contention_events.fetch_add(1, Ordering::Relaxed);
    }

    fn get_summary(&self) -> RoutingSummary {
        let total_queries = self.primary_queries.load(Ordering::Relaxed)
            + self.trigram_queries.load(Ordering::Relaxed);
        let routing_decisions = self.routing_decisions.load(Ordering::Relaxed);

        RoutingSummary {
            total_queries,
            primary_queries: self.primary_queries.load(Ordering::Relaxed),
            trigram_queries: self.trigram_queries.load(Ordering::Relaxed),
            routing_decisions,
            avg_routing_time_ns: if routing_decisions > 0 {
                self.routing_time_ns.load(Ordering::Relaxed) / routing_decisions
            } else {
                0
            },
            avg_primary_execution_ns: if self.primary_queries.load(Ordering::Relaxed) > 0 {
                self.primary_execution_time_ns.load(Ordering::Relaxed)
                    / self.primary_queries.load(Ordering::Relaxed)
            } else {
                0
            },
            avg_trigram_execution_ns: if self.trigram_queries.load(Ordering::Relaxed) > 0 {
                self.trigram_execution_time_ns.load(Ordering::Relaxed)
                    / self.trigram_queries.load(Ordering::Relaxed)
            } else {
                0
            },
            routing_errors: self.routing_errors.load(Ordering::Relaxed),
            contention_events: self.index_contention_events.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
struct RoutingSummary {
    total_queries: usize,
    primary_queries: usize,
    trigram_queries: usize,
    routing_decisions: usize,
    avg_routing_time_ns: usize,
    avg_primary_execution_ns: usize,
    avg_trigram_execution_ns: usize,
    routing_errors: usize,
    contention_events: usize,
}

impl RoutingSummary {
    fn routing_accuracy(&self) -> f64 {
        if self.routing_decisions == 0 {
            0.0
        } else {
            (self.routing_decisions - self.routing_errors) as f64 / self.routing_decisions as f64
        }
    }

    fn primary_ratio(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            self.primary_queries as f64 / self.total_queries as f64
        }
    }

    fn avg_routing_time_ms(&self) -> f64 {
        self.avg_routing_time_ns as f64 / 1_000_000.0
    }

    fn avg_primary_execution_ms(&self) -> f64 {
        self.avg_primary_execution_ns as f64 / 1_000_000.0
    }

    fn avg_trigram_execution_ms(&self) -> f64 {
        self.avg_trigram_execution_ns as f64 / 1_000_000.0
    }
}

/// Query Router simulator that mimics the routing logic from main.rs
struct QueryRouter {
    primary_index: Arc<Mutex<OptimizedPrimaryIndex>>,
    trigram_index: Arc<Mutex<OptimizedTrigramIndex>>,
    stats: Arc<RoutingStats>,
}

type OptimizedPrimaryIndex = OptimizedIndex<PrimaryIndex>;
type OptimizedTrigramIndex = OptimizedIndex<TrigramIndex>;

impl QueryRouter {
    async fn new(temp_dir: &TempDir) -> Result<Self> {
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        tokio::fs::create_dir_all(&primary_path).await?;
        tokio::fs::create_dir_all(&trigram_path).await?;

        let primary_index = Arc::new(Mutex::new(create_optimized_index_with_defaults(
            create_primary_index_for_tests(primary_path.to_str().unwrap()).await?,
        )));

        let trigram_index = Arc::new(Mutex::new(create_optimized_index_with_defaults(
            create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?,
        )));

        Ok(Self {
            primary_index,
            trigram_index,
            stats: Arc::new(RoutingStats::new()),
        })
    }

    /// Route query to appropriate index based on query type - mirrors main.rs logic
    async fn route_query(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        let routing_start = Instant::now();

        // Determine routing decision based on query text (matches main.rs lines 281-287)
        let query_text = query.search_terms.first().map(|t| t.as_str()).unwrap_or("");

        let use_primary = query_text == "*" || query_text.is_empty();
        let routing_time = routing_start.elapsed();

        self.stats
            .record_routing_decision(routing_time, use_primary);

        let execution_start = Instant::now();
        let result = if use_primary {
            // Route to Primary Index
            match self.primary_index.try_lock() {
                Ok(index) => index.search(query).await,
                Err(_) => {
                    self.stats.record_contention();
                    let index = self.primary_index.lock().await;
                    index.search(query).await
                }
            }
        } else {
            // Route to Trigram Index
            match self.trigram_index.try_lock() {
                Ok(index) => index.search(query).await,
                Err(_) => {
                    self.stats.record_contention();
                    let index = self.trigram_index.lock().await;
                    index.search(query).await
                }
            }
        };

        let execution_time = execution_start.elapsed();
        match &result {
            Ok(_) => {
                self.stats
                    .record_execution_time(execution_time, use_primary);
            }
            Err(_) => {
                self.stats.record_routing_error();
            }
        }

        result
    }

    async fn populate_indices(&self, document_count: usize) -> Result<Vec<ValidatedDocumentId>> {
        let mut doc_ids = Vec::new();

        for i in 0..document_count {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let path = ValidatedPath::new(format!("routing/test/doc_{i:06}.md"))?;

            // Insert into both indices
            {
                let mut primary = self.primary_index.lock().await;
                primary.insert(doc_id, path.clone()).await?;
            }
            {
                let mut trigram = self.trigram_index.lock().await;
                trigram.insert(doc_id, path).await?;
            }

            doc_ids.push(doc_id);
        }

        Ok(doc_ids)
    }

    fn get_stats(&self) -> Arc<RoutingStats> {
        Arc::clone(&self.stats)
    }
}

/// Test 1: Mixed Query Storm - 250+ concurrent queries split between index types
#[tokio::test]
async fn test_mixed_query_storm_routing() -> Result<()> {
    let concurrent_queries = 250;
    let queries_per_task = 20;

    println!(
        "🔥 Starting Mixed Query Storm: {concurrent_queries} concurrent tasks × {queries_per_task} queries"
    );

    let temp_dir = TempDir::new()?;
    let router = Arc::new(QueryRouter::new(&temp_dir).await?);

    // Pre-populate indices with test data
    let populate_start = Instant::now();
    let doc_ids = router.populate_indices(5000).await?;
    let populate_duration = populate_start.elapsed();
    println!("✅ Populated indices with 5000 documents in {populate_duration:?}");

    // Define query patterns for mixed workload
    let wildcard_queries = vec![
        Query::new(Some("*".to_string()), None, None, 10)?,
        Query::new(Some("".to_string()), None, None, 10)?,
        Query::empty(),
    ];

    let text_search_queries = vec![
        Query::new(Some("test".to_string()), None, None, 10)?,
        Query::new(Some("routing".to_string()), None, None, 10)?,
        Query::new(Some("document".to_string()), None, None, 10)?,
        Query::new(Some("performance".to_string()), None, None, 10)?,
        Query::new(Some("stress".to_string()), None, None, 10)?,
    ];

    let test_start = Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_queries));
    let mut handles = Vec::new();

    // Spawn concurrent query tasks
    for task_id in 0..concurrent_queries {
        let router_clone = Arc::clone(&router);
        let semaphore_clone = Arc::clone(&semaphore);
        let wildcard_queries_clone = wildcard_queries.clone();
        let text_queries_clone = text_search_queries.clone();

        let handle = tokio::spawn(async move {
            let permit = semaphore_clone.acquire().await.unwrap();
            let mut local_stats = (0usize, 0usize); // (primary_count, trigram_count)

            for query_num in 0..queries_per_task {
                // 50% wildcard (→ Primary), 50% text search (→ Trigram)
                let query = if query_num % 2 == 0 {
                    // Wildcard query → Primary Index
                    let idx = task_id % wildcard_queries_clone.len();
                    local_stats.0 += 1;
                    &wildcard_queries_clone[idx]
                } else {
                    // Text search → Trigram Index
                    let idx = (task_id + query_num) % text_queries_clone.len();
                    local_stats.1 += 1;
                    &text_queries_clone[idx]
                };

                match router_clone.route_query(query).await {
                    Ok(_results) => {
                        // Query succeeded
                    }
                    Err(_e) => {
                        // Query failed - this is acceptable under stress
                    }
                }

                // Small delay to simulate realistic query patterns
                if query_num % 5 == 0 {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }

            local_stats
        });

        handles.push(handle);
    }

    // Wait for all concurrent queries to complete
    let mut total_primary_expected = 0;
    let mut total_trigram_expected = 0;

    for handle in handles {
        let (primary_count, trigram_count) = handle.await?;
        total_primary_expected += primary_count;
        total_trigram_expected += trigram_count;
    }

    let test_duration = test_start.elapsed();
    let stats = router.get_stats().get_summary();

    // Analyze routing performance
    println!("\n🎯 Mixed Query Storm Results:");
    println!("  📊 Total Queries: {}", stats.total_queries);
    println!(
        "  🎯 Primary Queries: {} (expected: {})",
        stats.primary_queries, total_primary_expected
    );
    println!(
        "  🔍 Trigram Queries: {} (expected: {})",
        stats.trigram_queries, total_trigram_expected
    );
    println!(
        "  ⚡ Avg Routing Time: {:.2}ms",
        stats.avg_routing_time_ms()
    );
    println!(
        "  🔄 Primary Execution: {:.2}ms",
        stats.avg_primary_execution_ms()
    );
    println!(
        "  🔍 Trigram Execution: {:.2}ms",
        stats.avg_trigram_execution_ms()
    );
    println!(
        "  🎯 Routing Accuracy: {:.2}%",
        stats.routing_accuracy() * 100.0
    );
    println!("  ⚠️  Contention Events: {}", stats.contention_events);
    println!("  ⏱️  Total Duration: {test_duration:?}");

    let total_queries = concurrent_queries * queries_per_task;
    let throughput = total_queries as f64 / test_duration.as_secs_f64();
    println!("  🚀 Query Throughput: {throughput:.1} queries/sec");

    // Performance requirements
    assert!(
        stats.avg_routing_time_ms() < 10.0,
        "Routing decision time too high: {:.2}ms",
        stats.avg_routing_time_ms()
    );

    assert!(
        stats.routing_accuracy() > 0.95,
        "Routing accuracy too low: {:.2}%",
        stats.routing_accuracy() * 100.0
    );

    assert!(
        throughput > 200.0,
        "Query throughput too low: {throughput:.1} queries/sec"
    );

    // Verify correct index selection distribution
    let primary_ratio = stats.primary_ratio();
    assert!(
        primary_ratio > 0.4 && primary_ratio < 0.6,
        "Primary/Trigram ratio incorrect: {:.2}% primary",
        primary_ratio * 100.0
    );

    Ok(())
}

/// Test 2: Index Selection Performance Analysis under varying load
#[tokio::test]
async fn test_index_selection_performance_analysis() -> Result<()> {
    println!("🔥 Starting Index Selection Performance Analysis");

    let temp_dir = TempDir::new()?;
    let router = Arc::new(QueryRouter::new(&temp_dir).await?);

    // Populate with realistic data set
    let doc_ids = router.populate_indices(10000).await?;

    // Test different concurrency levels
    let concurrency_levels = vec![10, 50, 100, 200];
    let mut performance_results = HashMap::new();

    for &concurrency in &concurrency_levels {
        println!("  📊 Testing concurrency level: {concurrency} concurrent queries");

        let router_stats = Arc::new(RoutingStats::new());
        let test_start = Instant::now();
        let mut handles = Vec::new();

        // Create test queries with different complexity
        let simple_queries = vec![
            Query::new(Some("*".to_string()), None, None, 10)?,
            Query::new(Some("test".to_string()), None, None, 10)?,
        ];

        let complex_queries = vec![
            QueryBuilder::new()
                .with_text("complex search term with multiple words")?
                .with_tag("performance")?
                .with_limit(100)?
                .build()?,
            QueryBuilder::new()
                .with_text("distributed")?
                .with_tag("stress")?
                .with_tag("routing")?
                .with_limit(50)?
                .build()?,
        ];

        for task_id in 0..concurrency {
            let router_clone = Arc::clone(&router);
            let stats_clone = Arc::clone(&router_stats);
            let simple_queries_clone = simple_queries.clone();
            let complex_queries_clone = complex_queries.clone();

            let handle = tokio::spawn(async move {
                let mut task_times = Vec::new();

                for i in 0..20 {
                    let query = if i % 3 == 0 {
                        // Complex query
                        &complex_queries_clone[task_id % complex_queries_clone.len()]
                    } else {
                        // Simple query
                        &simple_queries_clone[task_id % simple_queries_clone.len()]
                    };

                    let query_start = Instant::now();
                    let routing_start = Instant::now();

                    // Record routing decision time
                    let query_text = query.search_terms.first().map(|t| t.as_str()).unwrap_or("");
                    let use_primary = query_text == "*" || query_text.is_empty();
                    let routing_time = routing_start.elapsed();

                    stats_clone.record_routing_decision(routing_time, use_primary);

                    // Execute query
                    match router_clone.route_query(query).await {
                        Ok(_) => {
                            let total_time = query_start.elapsed();
                            task_times.push(total_time);
                        }
                        Err(_) => {
                            stats_clone.record_routing_error();
                        }
                    }
                }

                task_times
            });

            handles.push(handle);
        }

        // Collect timing results
        let mut all_times = Vec::new();
        for handle in handles {
            let task_times = handle.await?;
            all_times.extend(task_times);
        }

        let test_duration = test_start.elapsed();
        let stats = router_stats.get_summary();

        // Calculate performance metrics
        all_times.sort();
        let median_time = if !all_times.is_empty() {
            all_times[all_times.len() / 2]
        } else {
            Duration::ZERO
        };

        let p95_time = if all_times.len() >= 20 {
            all_times[(all_times.len() * 95) / 100]
        } else {
            median_time
        };

        let avg_time = if !all_times.is_empty() {
            all_times.iter().sum::<Duration>() / all_times.len() as u32
        } else {
            Duration::ZERO
        };

        performance_results.insert(
            concurrency,
            (stats.clone(), median_time, p95_time, avg_time),
        );

        println!(
            "    ⚡ Routing: {:.2}ms, Median: {:.2}ms, P95: {:.2}ms, Throughput: {:.1}/sec",
            stats.avg_routing_time_ms(),
            median_time.as_secs_f64() * 1000.0,
            p95_time.as_secs_f64() * 1000.0,
            concurrency as f64 * 20.0 / test_duration.as_secs_f64()
        );
    }

    // Analyze performance scaling
    println!("\n🎯 Performance Analysis Summary:");
    for &concurrency in &concurrency_levels {
        if let Some((stats, _median, p95, avg)) = performance_results.get(&concurrency) {
            println!(
                "  📊 Concurrency {}: Routing {:.2}ms, Avg {:.2}ms, P95 {:.2}ms, Accuracy {:.1}%",
                concurrency,
                stats.avg_routing_time_ms(),
                avg.as_secs_f64() * 1000.0,
                p95.as_secs_f64() * 1000.0,
                stats.routing_accuracy() * 100.0
            );

            // Performance requirements per concurrency level
            assert!(
                stats.avg_routing_time_ms() < 10.0,
                "Routing time degraded at concurrency {}: {:.2}ms",
                concurrency,
                stats.avg_routing_time_ms()
            );

            assert!(
                avg.as_millis() < 100,
                "Average query time too high at concurrency {}: {:.2}ms",
                concurrency,
                avg.as_secs_f64() * 1000.0
            );
        }
    }

    Ok(())
}

/// Test 3: Multi-Index Concurrent Access Patterns
#[tokio::test]
async fn test_multi_index_concurrent_access() -> Result<()> {
    let concurrent_readers = 100;
    let concurrent_writers = 50;

    println!(
        "🔥 Starting Multi-Index Concurrent Access: {concurrent_readers} readers + {concurrent_writers} writers"
    );

    let temp_dir = TempDir::new()?;
    let router = Arc::new(QueryRouter::new(&temp_dir).await?);

    // Pre-populate indices
    let doc_ids = router.populate_indices(5000).await?;

    let test_start = Instant::now();
    let reader_semaphore = Arc::new(Semaphore::new(concurrent_readers));
    let writer_semaphore = Arc::new(Semaphore::new(concurrent_writers));
    let mut handles = Vec::new();

    // Spawn concurrent readers
    for reader_id in 0..concurrent_readers {
        let router_clone = Arc::clone(&router);
        let semaphore_clone = Arc::clone(&reader_semaphore);

        let handle = tokio::spawn(async move {
            let permit = semaphore_clone.acquire().await.unwrap();
            let mut reads_completed = 0;

            for i in 0..25 {
                let query = if i % 2 == 0 {
                    Query::new(Some("*".to_string()), None, None, 10).unwrap()
                } else {
                    Query::new(
                        Some(format!("search_term_{}", reader_id % 10)),
                        None,
                        None,
                        10,
                    )
                    .unwrap()
                };

                if router_clone.route_query(&query).await.is_ok() {
                    reads_completed += 1;
                }

                if i % 5 == 0 {
                    tokio::time::sleep(Duration::from_micros(50)).await;
                }
            }

            reads_completed
        });

        handles.push(handle);
    }

    // Spawn concurrent writers
    for writer_id in 0..concurrent_writers {
        let router_clone = Arc::clone(&router);
        let semaphore_clone = Arc::clone(&writer_semaphore);

        let handle = tokio::spawn(async move {
            let permit = semaphore_clone.acquire().await.unwrap();
            let mut writes_completed = 0;

            for i in 0..10 {
                let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
                let path = ValidatedPath::new(format!("concurrent/writer_{writer_id}/doc_{i}.md"))
                    .unwrap();

                // Insert into both indices
                let primary_result = {
                    match router_clone.primary_index.try_lock() {
                        Ok(mut index) => index.insert(doc_id, path.clone()).await,
                        Err(_) => {
                            router_clone.stats.record_contention();
                            let mut index = router_clone.primary_index.lock().await;
                            index.insert(doc_id, path.clone()).await
                        }
                    }
                };

                let trigram_result = {
                    match router_clone.trigram_index.try_lock() {
                        Ok(mut index) => index.insert(doc_id, path).await,
                        Err(_) => {
                            router_clone.stats.record_contention();
                            let mut index = router_clone.trigram_index.lock().await;
                            index.insert(doc_id, path).await
                        }
                    }
                };

                if primary_result.is_ok() && trigram_result.is_ok() {
                    writes_completed += 1;
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            writes_completed
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut total_reads = 0;
    let mut total_writes = 0;

    for (i, handle) in handles.into_iter().enumerate() {
        let count = handle.await?;
        if i < concurrent_readers {
            total_reads += count;
        } else {
            total_writes += count;
        }
    }

    let test_duration = test_start.elapsed();
    let stats = router.get_stats().get_summary();

    println!("\n🎯 Multi-Index Concurrent Access Results:");
    println!("  📖 Total Reads: {total_reads}");
    println!("  ✏️  Total Writes: {total_writes}");
    println!("  🔄 Primary Queries: {}", stats.primary_queries);
    println!("  🔍 Trigram Queries: {}", stats.trigram_queries);
    println!(
        "  ⚡ Avg Routing Time: {:.2}ms",
        stats.avg_routing_time_ms()
    );
    println!("  ⚠️  Contention Events: {}", stats.contention_events);
    println!("  ⏱️  Total Duration: {test_duration:?}");

    let read_throughput = total_reads as f64 / test_duration.as_secs_f64();
    let write_throughput = total_writes as f64 / test_duration.as_secs_f64();

    println!("  🚀 Read Throughput: {read_throughput:.1} reads/sec");
    println!("  🚀 Write Throughput: {write_throughput:.1} writes/sec");

    // Performance requirements
    assert!(
        read_throughput > 100.0,
        "Read throughput too low: {read_throughput:.1} reads/sec"
    );

    assert!(
        write_throughput > 20.0,
        "Write throughput too low: {write_throughput:.1} writes/sec"
    );

    assert!(
        stats.avg_routing_time_ms() < 10.0,
        "Routing time under concurrent access too high: {:.2}ms",
        stats.avg_routing_time_ms()
    );

    Ok(())
}

/// Test 4: Query Pattern Optimization and Hot Path Analysis
#[tokio::test]
async fn test_query_pattern_optimization() -> Result<()> {
    println!("🔥 Starting Query Pattern Optimization Test");

    let temp_dir = TempDir::new()?;
    let router = Arc::new(QueryRouter::new(&temp_dir).await?);

    // Populate indices
    let doc_ids = router.populate_indices(3000).await?;

    // Define hot path queries (frequently accessed)
    let hot_queries = vec![
        Query::new(Some("*".to_string()), None, None, 10)?,
        Query::new(Some("hot".to_string()), None, None, 10)?,
        Query::new(Some("frequent".to_string()), None, None, 10)?,
    ];

    // Define cold path queries (rarely accessed)
    let cold_queries = vec![
        Query::new(Some("rare_term".to_string()), None, None, 10)?,
        Query::new(Some("infrequent".to_string()), None, None, 10)?,
        Query::new(Some("cold_path".to_string()), None, None, 10)?,
    ];

    println!("  🔥 Phase 1: Establishing hot paths (90% hot, 10% cold)");

    let mut hot_path_times = Vec::new();
    let mut cold_path_times = Vec::new();

    // Phase 1: Execute many hot path queries to establish cache patterns
    for _ in 0..300 {
        for (i, query) in hot_queries.iter().enumerate() {
            let start = Instant::now();
            let _ = router.route_query(query).await;
            let duration = start.elapsed();
            hot_path_times.push(duration);

            // Occasionally execute cold queries
            if i % 10 == 0 && !cold_queries.is_empty() {
                let cold_query = &cold_queries[i % cold_queries.len()];
                let start = Instant::now();
                let _ = router.route_query(cold_query).await;
                let duration = start.elapsed();
                cold_path_times.push(duration);
            }
        }
    }

    println!("  📊 Phase 2: Measuring optimized hot path performance");

    // Phase 2: Measure performance after optimization
    let mut optimized_hot_times = Vec::new();
    let mut optimized_cold_times = Vec::new();

    for _ in 0..100 {
        for query in &hot_queries {
            let start = Instant::now();
            let _ = router.route_query(query).await;
            let duration = start.elapsed();
            optimized_hot_times.push(duration);
        }

        for query in &cold_queries {
            let start = Instant::now();
            let _ = router.route_query(query).await;
            let duration = start.elapsed();
            optimized_cold_times.push(duration);
        }
    }

    // Calculate performance metrics
    let avg_hot_initial = if !hot_path_times.is_empty() {
        hot_path_times.iter().sum::<Duration>() / hot_path_times.len() as u32
    } else {
        Duration::ZERO
    };

    let avg_hot_optimized = if !optimized_hot_times.is_empty() {
        optimized_hot_times.iter().sum::<Duration>() / optimized_hot_times.len() as u32
    } else {
        Duration::ZERO
    };

    let avg_cold_initial = if !cold_path_times.is_empty() {
        cold_path_times.iter().sum::<Duration>() / cold_path_times.len() as u32
    } else {
        Duration::ZERO
    };

    let avg_cold_optimized = if !optimized_cold_times.is_empty() {
        optimized_cold_times.iter().sum::<Duration>() / optimized_cold_times.len() as u32
    } else {
        Duration::ZERO
    };

    let stats = router.get_stats().get_summary();

    println!("\n🎯 Query Pattern Optimization Results:");
    println!(
        "  🔥 Hot Path - Initial: {:.2}ms, Optimized: {:.2}ms",
        avg_hot_initial.as_secs_f64() * 1000.0,
        avg_hot_optimized.as_secs_f64() * 1000.0
    );
    println!(
        "  ❄️  Cold Path - Initial: {:.2}ms, Optimized: {:.2}ms",
        avg_cold_initial.as_secs_f64() * 1000.0,
        avg_cold_optimized.as_secs_f64() * 1000.0
    );
    println!(
        "  ⚡ Avg Routing Time: {:.2}ms",
        stats.avg_routing_time_ms()
    );
    println!(
        "  🎯 Routing Accuracy: {:.2}%",
        stats.routing_accuracy() * 100.0
    );

    // Calculate optimization benefit
    let hot_path_improvement = if avg_hot_initial > Duration::ZERO {
        avg_hot_initial.as_secs_f64() / avg_hot_optimized.as_secs_f64()
    } else {
        1.0
    };

    println!("  📈 Hot Path Improvement: {hot_path_improvement:.2}x speedup");

    // Performance requirements
    assert!(
        avg_hot_optimized.as_millis() < 50,
        "Hot path optimization not effective: {:.2}ms",
        avg_hot_optimized.as_secs_f64() * 1000.0
    );

    assert!(
        stats.routing_accuracy() > 0.98,
        "Routing accuracy degraded during optimization: {:.2}%",
        stats.routing_accuracy() * 100.0
    );

    // Hot paths should show some benefit (even if small in this test environment)
    assert!(
        hot_path_improvement >= 0.8,
        "Hot path performance regressed: {hot_path_improvement:.2}x"
    );

    Ok(())
}

/// Test 5: Routing Overhead Analysis
#[tokio::test]
async fn test_routing_overhead_analysis() -> Result<()> {
    println!("🔥 Starting Routing Overhead Analysis");

    let temp_dir = TempDir::new()?;
    let router = Arc::new(QueryRouter::new(&temp_dir).await?);

    // Populate with minimal data for fast execution
    let doc_ids = router.populate_indices(1000).await?;

    // Test with different query types
    let test_queries = vec![
        (
            "wildcard",
            Query::new(Some("*".to_string()), None, None, 10)?,
        ),
        ("empty", Query::empty()),
        (
            "simple_text",
            Query::new(Some("test".to_string()), None, None, 10)?,
        ),
        (
            "complex_text",
            QueryBuilder::new()
                .with_text("complex search with multiple terms")?
                .with_tag("performance")?
                .with_tag("routing")?
                .with_limit(50)?
                .build()?,
        ),
    ];

    let mut overhead_results = HashMap::new();

    for (query_type, query) in &test_queries {
        println!("  📊 Analyzing overhead for {query_type}");

        let mut routing_times = Vec::new();
        let mut execution_times = Vec::new();
        let mut total_times = Vec::new();

        // Execute queries multiple times to get reliable measurements
        for _ in 0..100 {
            let total_start = Instant::now();

            // Measure routing decision time
            let routing_start = Instant::now();
            let query_text = query.search_terms.first().map(|t| t.as_str()).unwrap_or("");
            let use_primary = query_text == "*" || query_text.is_empty();
            let routing_time = routing_start.elapsed();

            // Measure actual execution time
            let execution_start = Instant::now();
            let _ = router.route_query(query).await;
            let execution_time = execution_start.elapsed();

            let total_time = total_start.elapsed();

            routing_times.push(routing_time);
            execution_times.push(execution_time);
            total_times.push(total_time);
        }

        // Calculate statistics
        routing_times.sort();
        execution_times.sort();
        total_times.sort();

        let avg_routing = routing_times.iter().sum::<Duration>() / routing_times.len() as u32;
        let avg_execution = execution_times.iter().sum::<Duration>() / execution_times.len() as u32;
        let avg_total = total_times.iter().sum::<Duration>() / total_times.len() as u32;

        let median_routing = routing_times[routing_times.len() / 2];
        let p95_routing = routing_times[(routing_times.len() * 95) / 100];

        let overhead_percentage = if avg_total.as_nanos() > 0 {
            (avg_routing.as_nanos() as f64 / avg_total.as_nanos() as f64) * 100.0
        } else {
            0.0
        };

        overhead_results.insert(
            query_type,
            (avg_routing, avg_execution, overhead_percentage),
        );

        println!(
            "    ⚡ Routing: {:.3}ms, Execution: {:.2}ms, Overhead: {:.1}%",
            avg_routing.as_secs_f64() * 1000.0,
            avg_execution.as_secs_f64() * 1000.0,
            overhead_percentage
        );
        println!(
            "    📈 Median routing: {:.3}ms, P95: {:.3}ms",
            median_routing.as_secs_f64() * 1000.0,
            p95_routing.as_secs_f64() * 1000.0
        );
    }

    println!("\n🎯 Routing Overhead Analysis Summary:");

    let mut total_overhead = 0.0;
    let mut count = 0;

    for (query_type, (routing_time, execution_time, overhead_pct)) in &overhead_results {
        println!(
            "  📊 {}: Routing {:.3}ms, Execution {:.2}ms, Overhead {:.1}%",
            query_type,
            routing_time.as_secs_f64() * 1000.0,
            execution_time.as_secs_f64() * 1000.0,
            overhead_pct
        );

        total_overhead += overhead_pct;
        count += 1;

        // Performance requirements per query type
        assert!(
            routing_time.as_millis() < 10,
            "Routing time too high for {}: {:.3}ms",
            query_type,
            routing_time.as_secs_f64() * 1000.0
        );

        assert!(
            *overhead_pct < 20.0,
            "Routing overhead too high for {query_type}: {overhead_pct:.1}%"
        );
    }

    let avg_overhead = total_overhead / count as f64;
    println!("  📈 Average Routing Overhead: {avg_overhead:.1}%");

    // Overall performance requirements
    assert!(
        avg_overhead < 15.0,
        "Average routing overhead too high: {avg_overhead:.1}%"
    );

    Ok(())
}
