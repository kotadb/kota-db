// Query Routing Performance Benchmarks
// Comprehensive benchmarks for routing decision performance and index selection efficiency

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kotadb::{
    contracts::{Index, Query},
    create_optimized_index_with_defaults, create_primary_index_for_tests,
    create_trigram_index_for_tests, QueryBuilder, ValidatedDocumentId, ValidatedPath,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Router simulation for benchmarking
struct BenchmarkRouter {
    primary_index: Arc<Mutex<OptimizedPrimaryIndex>>,
    trigram_index: Arc<Mutex<OptimizedTrigramIndex>>,
}

type OptimizedPrimaryIndex =
    kotadb::wrappers::optimization::OptimizedIndex<kotadb::primary_index::PrimaryIndex>;
type OptimizedTrigramIndex =
    kotadb::wrappers::optimization::OptimizedIndex<kotadb::trigram_index::TrigramIndex>;

impl BenchmarkRouter {
    async fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let primary_path = temp_dir.path().join("bench_primary");
        let trigram_path = temp_dir.path().join("bench_trigram");

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
        })
    }

    async fn populate(&self, count: usize) -> anyhow::Result<()> {
        for i in 0..count {
            let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let path = ValidatedPath::new(format!("/bench/doc_{i:06}.md"))?;

            {
                let mut primary = self.primary_index.lock().await;
                primary.insert(doc_id, path.clone()).await?;
            }
            {
                let mut trigram = self.trigram_index.lock().await;
                trigram.insert(doc_id, path).await?;
            }
        }
        Ok(())
    }

    async fn route_query(&self, query: &Query) -> anyhow::Result<Vec<ValidatedDocumentId>> {
        // Mirror the routing logic from main.rs
        let query_text = query.search_terms.first().map(|t| t.as_str()).unwrap_or("");

        if query_text == "*" || query_text.is_empty() {
            // Route to Primary Index
            let index = self.primary_index.lock().await;
            index.search(query).await
        } else {
            // Route to Trigram Index
            let index = self.trigram_index.lock().await;
            index.search(query).await
        }
    }

    /// Measure only routing decision time (not execution)
    fn route_decision_time(&self, query: &Query) -> std::time::Duration {
        let start = std::time::Instant::now();

        // Routing decision logic (mirrors main.rs)
        let query_text = query.search_terms.first().map(|t| t.as_str()).unwrap_or("");

        let use_primary = query_text == "*" || query_text.is_empty();

        start.elapsed()
    }
}

/// Benchmark routing decision overhead
fn bench_routing_decision_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let router = rt.block_on(BenchmarkRouter::new()).unwrap();

    let queries = vec![
        (
            "wildcard",
            Query::new(Some("*".to_string()), None, None, 10).unwrap(),
        ),
        ("empty", Query::empty()),
        (
            "simple_text",
            Query::new(Some("test".to_string()), None, None, 10).unwrap(),
        ),
        (
            "complex_text",
            QueryBuilder::new()
                .with_text("complex search term")
                .unwrap()
                .with_tag("benchmark")
                .unwrap()
                .with_limit(100)
                .unwrap()
                .build()
                .unwrap(),
        ),
    ];

    let mut group = c.benchmark_group("routing_decision_overhead");

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::new("decision_time", name),
            &query,
            |b, query| {
                b.iter(|| black_box(router.route_decision_time(black_box(query))));
            },
        );
    }

    group.finish();
}

/// Benchmark query execution through routing
fn bench_query_execution_routing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let router = rt.block_on(async {
        let router = BenchmarkRouter::new().await.unwrap();
        router.populate(1000).await.unwrap();
        router
    });

    let queries = vec![
        (
            "wildcard_primary",
            Query::new(Some("*".to_string()), None, None, 10).unwrap(),
        ),
        (
            "text_trigram",
            Query::new(Some("benchmark".to_string()), None, None, 10).unwrap(),
        ),
        ("empty_primary", Query::empty()),
        (
            "complex_trigram",
            QueryBuilder::new()
                .with_text("performance test")
                .unwrap()
                .with_limit(50)
                .unwrap()
                .build()
                .unwrap(),
        ),
    ];

    let mut group = c.benchmark_group("query_execution_routing");

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::new("full_execution", name),
            &query,
            |b, query| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(router.route_query(black_box(query)).await.unwrap())
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark routing throughput under varying loads
fn bench_routing_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let router = rt.block_on(async {
        let router = BenchmarkRouter::new().await.unwrap();
        router.populate(5000).await.unwrap();
        router
    });

    let query_counts = [10, 50, 100, 250];

    let mixed_queries = vec![
        Query::new(Some("*".to_string()), None, None, 10).unwrap(),
        Query::new(Some("throughput".to_string()), None, None, 10).unwrap(),
        Query::empty(),
        Query::new(Some("performance".to_string()), None, None, 10).unwrap(),
    ];

    let mut group = c.benchmark_group("routing_throughput");

    for &count in &query_counts {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("mixed_queries", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut results = Vec::new();
                        for i in 0..count {
                            let query = &mixed_queries[i % mixed_queries.len()];
                            let result = router.route_query(query).await.unwrap();
                            results.push(black_box(result));
                        }
                        black_box(results)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark routing accuracy under stress
fn bench_routing_accuracy_stress(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let router = rt.block_on(async {
        let router = BenchmarkRouter::new().await.unwrap();
        router.populate(3000).await.unwrap();
        router
    });

    // Create query sets that should definitely route to specific indices
    let primary_queries = vec![
        Query::new(Some("*".to_string()), None, None, 10).unwrap(),
        Query::new(Some("".to_string()), None, None, 10).unwrap(),
        Query::empty(),
    ];

    let trigram_queries = vec![
        Query::new(Some("specific_text".to_string()), None, None, 10).unwrap(),
        Query::new(Some("accuracy".to_string()), None, None, 10).unwrap(),
        QueryBuilder::new()
            .with_text("complex query")
            .unwrap()
            .with_tag("stress")
            .unwrap()
            .build()
            .unwrap(),
    ];

    let mut group = c.benchmark_group("routing_accuracy");

    // Benchmark primary index routing accuracy
    group.bench_function("primary_routing_batch", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut results = Vec::new();
                for query in &primary_queries {
                    for _ in 0..10 {
                        let result = router.route_query(query).await.unwrap();
                        results.push(black_box(result));
                    }
                }
                black_box(results)
            })
        });
    });

    // Benchmark trigram index routing accuracy
    group.bench_function("trigram_routing_batch", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut results = Vec::new();
                for query in &trigram_queries {
                    for _ in 0..10 {
                        let result = router.route_query(query).await.unwrap();
                        results.push(black_box(result));
                    }
                }
                black_box(results)
            })
        });
    });

    // Benchmark mixed routing under stress
    group.bench_function("mixed_routing_stress", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut results = Vec::new();
                for i in 0..50 {
                    let query = if i % 2 == 0 {
                        &primary_queries[i % primary_queries.len()]
                    } else {
                        &trigram_queries[i % trigram_queries.len()]
                    };
                    let result = router.route_query(query).await.unwrap();
                    results.push(black_box(result));
                }
                black_box(results)
            })
        });
    });

    group.finish();
}

/// Benchmark index selection efficiency
fn bench_index_selection_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let router = rt.block_on(async {
        let router = BenchmarkRouter::new().await.unwrap();
        router.populate(10000).await.unwrap();
        router
    });

    let query_patterns = vec![
        (
            "all_primary",
            vec![
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::empty(),
            ],
        ),
        (
            "all_trigram",
            vec![
                Query::new(Some("efficiency".to_string()), None, None, 10).unwrap(),
                Query::new(Some("selection".to_string()), None, None, 10).unwrap(),
            ],
        ),
        (
            "mixed_50_50",
            vec![
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::new(Some("mixed".to_string()), None, None, 10).unwrap(),
            ],
        ),
        (
            "mixed_80_20",
            vec![
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::new(Some("*".to_string()), None, None, 10).unwrap(),
                Query::new(Some("minority".to_string()), None, None, 10).unwrap(),
            ],
        ),
    ];

    let mut group = c.benchmark_group("index_selection_efficiency");

    for (pattern_name, queries) in query_patterns {
        group.bench_with_input(
            BenchmarkId::new("selection_pattern", pattern_name),
            &queries,
            |b, queries| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut results = Vec::new();
                        for _ in 0..20 {
                            for query in queries {
                                let result = router.route_query(query).await.unwrap();
                                results.push(black_box(result));
                            }
                        }
                        black_box(results)
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_routing_decision_overhead,
    bench_query_execution_routing,
    bench_routing_throughput,
    bench_routing_accuracy_stress,
    bench_index_selection_efficiency
);

criterion_main!(benches);
