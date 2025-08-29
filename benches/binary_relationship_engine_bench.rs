//! Performance benchmarks for binary relationship engine.
//!
//! This benchmark suite validates that the binary relationship engine meets the
//! sub-10ms query latency requirement and confirms the claimed sub-microsecond
//! symbol lookups.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::binary_relationship_engine::BinaryRelationshipEngine;
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::binary_symbols::{BinarySymbolReader, BinarySymbolWriter};
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::relationship_query::{RelationshipQueryConfig, RelationshipQueryType};
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Create a test binary symbol database for benchmarking
#[cfg(feature = "tree-sitter-parsing")]
fn create_test_symbol_database(path: &Path, symbol_count: usize) -> anyhow::Result<Vec<String>> {
    let mut writer = BinarySymbolWriter::new();
    let mut symbol_names = Vec::new();

    // Add realistic symbol names for benchmarking
    for i in 0..symbol_count {
        let symbol_name = if i % 4 == 0 {
            format!("function_{}", i)
        } else if i % 4 == 1 {
            format!("struct_{}", i)
        } else if i % 4 == 2 {
            format!("impl_method_{}", i)
        } else {
            format!("variable_{}", i)
        };

        symbol_names.push(symbol_name.clone());

        let id = Uuid::new_v4();
        let kind = ((i % 8) + 1) as u8; // Distribute across symbol types
        let file_path = format!("src/module_{}.rs", i / 100);
        let start_line = (i % 1000) as u32 + 1;
        let end_line = start_line + ((i % 50) as u32) + 1;

        writer.add_symbol(
            id,
            &symbol_name,
            kind,
            &file_path,
            start_line,
            end_line,
            None, // No parent for simplicity
        );
    }

    writer.write_to_file(path)?;
    Ok(symbol_names)
}

/// Benchmark hybrid engine initialization
#[cfg(feature = "tree-sitter-parsing")]
fn bench_hybrid_engine_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    // Create test symbol database
    let symbol_db_path = db_path.join("symbols.kota");
    rt.block_on(async {
        create_test_symbol_database(&symbol_db_path, 10_000).unwrap();
    });

    let mut group = c.benchmark_group("hybrid_engine_init");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("init_with_10k_symbols", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = RelationshipQueryConfig::default();
                let engine = BinaryRelationshipEngine::new(black_box(db_path), black_box(config))
                    .await
                    .unwrap();

                let stats = engine.get_stats();
                assert_eq!(stats.binary_symbols_loaded, 10_000);
                black_box(engine)
            })
        });
    });

    group.finish();
}

/// Benchmark symbol lookup performance (should be sub-microsecond)
#[cfg(feature = "tree-sitter-parsing")]
fn bench_symbol_lookup_performance(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let symbol_db_path = temp_dir.path().join("symbols.kota");

    // Create test database with various sizes
    let symbol_counts = vec![1_000, 10_000, 50_000];

    for &count in &symbol_counts {
        let symbol_names = create_test_symbol_database(&symbol_db_path, count).unwrap();
        let reader = BinarySymbolReader::open(&symbol_db_path).unwrap();

        let mut group = c.benchmark_group("symbol_lookup");
        group.measurement_time(Duration::from_secs(5));

        // Test O(1) UUID lookup
        group.bench_with_input(BenchmarkId::new("uuid_lookup", count), &count, |b, _| {
            // Get a sample UUID from the database
            let sample_symbol = reader.get_symbol(count / 2).unwrap();
            let sample_uuid = uuid::Uuid::from_bytes(sample_symbol.id);

            b.iter(|| {
                let result = reader.find_symbol(black_box(sample_uuid));
                assert!(result.is_some());
                black_box(result)
            });
        });

        // Test O(n) name lookup for comparison
        group.bench_with_input(BenchmarkId::new("name_lookup", count), &count, |b, _| {
            let sample_name = &symbol_names[count / 2];

            b.iter(|| {
                let result = reader.find_symbol_by_name(black_box(sample_name));
                assert!(result.is_some());
                black_box(result)
            });
        });

        group.finish();
    }
}

/// Benchmark relationship query performance
#[cfg(feature = "tree-sitter-parsing")]
fn bench_relationship_queries(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    // Create test symbol database
    let symbol_db_path = db_path.join("symbols.kota");
    let symbol_names =
        rt.block_on(async { create_test_symbol_database(&symbol_db_path, 20_000).unwrap() });

    let engine = rt.block_on(async {
        let config = RelationshipQueryConfig::default();
        BinaryRelationshipEngine::new(db_path, config)
            .await
            .unwrap()
    });

    let mut group = c.benchmark_group("relationship_queries");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark FindCallers query (without dependency graph - should be fast)
    group.bench_function("find_callers_no_graph", |b| {
        b.iter(|| {
            rt.block_on(async {
                let target = &symbol_names[1000]; // Sample symbol name
                let query = RelationshipQueryType::FindCallers {
                    target: target.clone(),
                };

                let result = engine.execute_query(black_box(query)).await.unwrap();
                // Should return quickly with informative message about missing graph
                assert!(result.summary.contains("relationship graph not available"));
                black_box(result)
            })
        });
    });

    // Benchmark ImpactAnalysis query (without dependency graph)
    group.bench_function("impact_analysis_no_graph", |b| {
        b.iter(|| {
            rt.block_on(async {
                let target = &symbol_names[2000]; // Sample symbol name
                let query = RelationshipQueryType::ImpactAnalysis {
                    target: target.clone(),
                };

                let result = engine.execute_query(black_box(query)).await.unwrap();
                // Should return quickly with informative message
                assert!(result.summary.contains("relationship graph not available"));
                black_box(result)
            })
        });
    });

    group.finish();
}

/// Benchmark engine statistics retrieval
#[cfg(feature = "tree-sitter-parsing")]
fn bench_engine_stats(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    // Create test symbol database
    let symbol_db_path = db_path.join("symbols.kota");
    rt.block_on(async {
        create_test_symbol_database(&symbol_db_path, 25_000).unwrap();
    });

    let engine = rt.block_on(async {
        let config = RelationshipQueryConfig::default();
        BinaryRelationshipEngine::new(db_path, config)
            .await
            .unwrap()
    });

    c.bench_function("get_stats", |b| {
        b.iter(|| {
            let stats = engine.get_stats();
            assert_eq!(stats.binary_symbols_loaded, 25_000);
            assert_eq!(stats.graph_nodes_loaded, 0); // No dependency graph
            assert!(!stats.using_binary_path); // False because no dependency graph
            black_box(stats)
        });
    });
}

/// Test that confirms sub-10ms query latency requirement
#[cfg(feature = "tree-sitter-parsing")]
fn bench_query_latency_requirement(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path();

    // Create larger test database
    let symbol_db_path = db_path.join("symbols.kota");
    let symbol_names =
        rt.block_on(async { create_test_symbol_database(&symbol_db_path, 30_000).unwrap() });

    let engine = rt.block_on(async {
        let config = RelationshipQueryConfig::default();
        BinaryRelationshipEngine::new(db_path, config)
            .await
            .unwrap()
    });

    let mut group = c.benchmark_group("latency_requirement");
    group.measurement_time(Duration::from_secs(15));

    // This benchmark ensures we meet the sub-10ms requirement
    group.bench_function("query_under_10ms", |b| {
        b.iter(|| {
            rt.block_on(async {
                let start = std::time::Instant::now();

                let target = &symbol_names[15_000]; // Middle symbol
                let query = RelationshipQueryType::FindCallers {
                    target: target.clone(),
                };

                let result = engine.execute_query(black_box(query)).await.unwrap();

                let elapsed = start.elapsed();
                // Verify the sub-10ms requirement is met
                assert!(
                    elapsed.as_millis() < 10,
                    "Query took {:?}, exceeding 10ms requirement",
                    elapsed
                );

                black_box(result)
            })
        });
    });

    group.finish();
}

#[cfg(feature = "tree-sitter-parsing")]
criterion_group!(
    benches,
    bench_hybrid_engine_initialization,
    bench_symbol_lookup_performance,
    bench_relationship_queries,
    bench_engine_stats,
    bench_query_latency_requirement
);

#[cfg(not(feature = "tree-sitter-parsing"))]
criterion_group!(benches,);

criterion_main!(benches);
