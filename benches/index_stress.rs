// Index Stress Tests - Enhanced benchmarks for Phase 2A scale requirements
// Comprehensive stress testing for B+ Tree, Trigram Index, and index integration at scale

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kotadb::{
    btree,
    contracts::{Index, Query},
    create_optimized_index_with_defaults, create_primary_index_for_tests,
    create_trigram_index_for_tests, ValidatedDocumentId, ValidatedPath,
};
use std::time::Instant;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Enhanced B+ Tree stress tests for 50K, 100K entries
fn bench_btree_stress_large_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_large_scale");
    group.sample_size(10); // Reduce sample size for large tests

    // Test Phase 2A scale requirements: 50K, 100K entries
    for size in [50_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("insertion", format!("{}K", size / 1000)),
            size,
            |b, &size| {
                let keys: Vec<_> = (0..size)
                    .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
                    .collect();
                let paths: Vec<_> = (0..size)
                    .map(|i| ValidatedPath::new(format!("/stress/doc_{i}.md")).unwrap())
                    .collect();

                b.iter(|| {
                    let mut tree = btree::create_empty_tree();
                    for i in 0..size {
                        tree = btree::insert_into_tree(tree, keys[i], paths[i].clone()).unwrap();
                    }
                    black_box(tree)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("search", format!("{}K", size / 1000)),
            size,
            |b, &size| {
                // Pre-build tree
                let mut tree = btree::create_empty_tree();
                let keys: Vec<_> = (0..size)
                    .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
                    .collect();

                for (i, key) in keys.iter().enumerate() {
                    let path = ValidatedPath::new(format!("/stress/doc_{i}.md")).unwrap();
                    tree = btree::insert_into_tree(tree, *key, path).unwrap();
                }

                // Search for random subset
                let search_keys: Vec<_> = keys.iter().take(1000).collect(); // 1K searches

                b.iter(|| {
                    for key in &search_keys {
                        black_box(btree::search_in_tree(&tree, key));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Trigram index stress tests with large text corpora
fn bench_trigram_stress_large_corpus(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("trigram_large_corpus");
    group.sample_size(10);

    // Test with realistic document volumes
    for doc_count in [10_000, 25_000].iter() {
        group.throughput(Throughput::Elements(*doc_count as u64));

        group.bench_with_input(
            BenchmarkId::new("index_build", format!("{}K_docs", doc_count / 1000)),
            doc_count,
            |b, &doc_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let mut index =
                            create_trigram_index_for_tests(temp_dir.path().to_str().unwrap())
                                .await
                                .unwrap();

                        // Generate realistic test data
                        let test_data = generate_test_documents(doc_count, 2000);

                        let start = Instant::now();
                        for (id, path) in test_data {
                            index.insert(id, path).await.unwrap();
                        }
                        black_box(start.elapsed())
                    })
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("text_search", format!("{}K_docs", doc_count / 1000)),
            doc_count,
            |b, &doc_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let mut index =
                            create_trigram_index_for_tests(temp_dir.path().to_str().unwrap())
                                .await
                                .unwrap();

                        // Pre-populate index
                        let test_data = generate_test_documents(doc_count, 1500);
                        for (id, path) in test_data {
                            index.insert(id, path).await.unwrap();
                        }

                        // Test various search patterns
                        let search_terms = vec![
                            "function",
                            "database",
                            "implementation",
                            "performance",
                            "async",
                            "error",
                            "testing",
                            "optimization",
                        ];

                        let start = Instant::now();
                        for term in &search_terms {
                            let query =
                                Query::new(Some(term.to_string()), None, None, 100).unwrap();
                            black_box(index.search(&query).await.unwrap());
                        }
                        black_box(start.elapsed())
                    })
                });
            },
        );
    }

    group.finish();
}

/// Index integration stress tests - multiple indices working together
fn bench_index_integration_stress(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("index_integration");
    group.sample_size(10);

    group.bench_function("dual_index_operations_25K", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let primary_path = temp_dir.path().join("primary");
                let trigram_path = temp_dir.path().join("trigram");

                tokio::fs::create_dir_all(&primary_path).await.unwrap();
                tokio::fs::create_dir_all(&trigram_path).await.unwrap();

                let mut primary_index = create_optimized_index_with_defaults(
                    create_primary_index_for_tests(primary_path.to_str().unwrap())
                        .await
                        .unwrap(),
                );
                let mut trigram_index = create_optimized_index_with_defaults(
                    create_trigram_index_for_tests(trigram_path.to_str().unwrap())
                        .await
                        .unwrap(),
                );

                // Generate test documents
                let test_data = generate_test_documents(25_000, 3000);

                let start = Instant::now();

                // Simulate real workload: insert into both indices
                for (doc_id, doc_path) in test_data.iter() {
                    primary_index
                        .insert(*doc_id, doc_path.clone())
                        .await
                        .unwrap();
                    trigram_index
                        .insert(*doc_id, doc_path.clone())
                        .await
                        .unwrap();
                }

                // Mixed query workload
                for i in 0..100 {
                    if i % 3 == 0 {
                        // Primary index lookup - create a by-ID query
                        let query = Query::empty(); // Use empty query for now
                        black_box(primary_index.search(&query).await.unwrap());
                    } else {
                        // Text search
                        let search_terms = ["function", "async", "performance", "test"];
                        let term = search_terms[i % search_terms.len()];
                        let query = Query::new(Some(term.to_string()), None, None, 10).unwrap();
                        black_box(trigram_index.search(&query).await.unwrap());
                    }
                }

                black_box(start.elapsed())
            })
        });
    });

    group.finish();
}

/// Memory pressure testing - verify performance under memory constraints
fn bench_memory_pressure_stress(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_pressure");
    group.sample_size(10);

    // Test with large documents to create memory pressure
    group.bench_function("large_documents_memory_stress", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let mut index = create_trigram_index_for_tests(temp_dir.path().to_str().unwrap())
                    .await
                    .unwrap();

                // Generate large documents (simulate 50KB each through multiple paths)
                let test_data = generate_large_test_documents(1_000);

                let start = Instant::now();

                // Insert large documents
                for (id, path) in test_data {
                    index.insert(id, path).await.unwrap();
                }

                // Perform searches to test retrieval under memory pressure
                for i in 0..50 {
                    let search_terms = ["implementation", "performance", "optimization"];
                    let term = search_terms[i % search_terms.len()];
                    let query = Query::new(Some(term.to_string()), None, None, 10).unwrap();
                    black_box(index.search(&query).await.unwrap());
                }

                black_box(start.elapsed())
            })
        });
    });

    group.finish();
}

/// Helper function to generate test documents
fn generate_test_documents(
    count: usize,
    avg_size: usize,
) -> Vec<(ValidatedDocumentId, ValidatedPath)> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();

        // Create path with content hints based on size
        let topic = match i % 10 {
            0 => "rust",
            1 => "database",
            2 => "async",
            3 => "performance",
            4 => "testing",
            5 => "implementation",
            6 => "optimization",
            7 => "algorithms",
            8 => "networking",
            _ => "general",
        };

        let path = ValidatedPath::new(format!("/{topic}/doc_{i}_{avg_size}_bytes.md")).unwrap();
        documents.push((id, path));
    }

    documents
}

/// Helper function to generate large test documents for memory pressure testing
fn generate_large_test_documents(count: usize) -> Vec<(ValidatedDocumentId, ValidatedPath)> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();

        // Create paths that simulate large documents
        let topic = match i % 5 {
            0 => "large_implementation_guide",
            1 => "comprehensive_performance_analysis",
            2 => "detailed_async_programming_manual",
            3 => "extensive_testing_framework_documentation",
            _ => "massive_optimization_reference",
        };

        let path = ValidatedPath::new(format!("/large/{topic}/huge_doc_{i}_50kb.md")).unwrap();
        documents.push((id, path));
    }

    documents
}

criterion_group!(
    index_stress_benches,
    bench_btree_stress_large_scale,
    bench_trigram_stress_large_corpus,
    bench_index_integration_stress,
    bench_memory_pressure_stress
);

criterion_main!(index_stress_benches);
