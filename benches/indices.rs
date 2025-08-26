// Index benchmarks - Stage 1: TDD Performance Benchmarks for B+ Tree

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kotadb::{btree, ValidatedDocumentId, ValidatedPath};
use uuid::Uuid;

/// Benchmark B+ tree insertion performance
fn bench_btree_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insertion");

    // Test different tree sizes to verify O(log n) behavior
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup: Create keys to insert
            let keys: Vec<_> = (0..size)
                .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
                .collect();
            let paths: Vec<_> = (0..size)
                .map(|i| ValidatedPath::new(format!("bench/doc_{i}.md")).unwrap())
                .collect();

            b.iter(|| {
                let mut tree = btree::create_empty_tree();
                for i in 0..size {
                    tree = btree::insert_into_tree(tree, keys[i], paths[i].clone()).unwrap();
                }
                black_box(tree)
            });
        });
    }

    group.finish();
}

/// Benchmark B+ tree search performance
fn bench_btree_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_search");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup: Build tree
            let mut tree = btree::create_empty_tree();
            let keys: Vec<_> = (0..size)
                .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
                .collect();

            for (i, key) in keys.iter().enumerate() {
                let path = ValidatedPath::new(format!("bench/doc_{i}.md")).unwrap();
                tree = btree::insert_into_tree(tree, *key, path).unwrap();
            }

            // Benchmark searching for middle elements
            let search_keys: Vec<_> = keys.iter().skip(size / 4).take(size / 2).collect();

            b.iter(|| {
                for key in &search_keys {
                    black_box(btree::search_in_tree(&tree, key));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark B+ tree deletion performance (once implemented)
fn bench_btree_deletion(c: &mut Criterion) {
    let group = c.benchmark_group("btree_deletion");

    // This will be uncommented once delete_from_tree is implemented
    /*
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup: Build tree
            let keys: Vec<_> = (0..size)
                .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
                .collect();

            b.iter_batched(
                || {
                    // Setup for each iteration
                    let mut tree = btree::create_empty_tree();
                    for (i, key) in keys.iter().enumerate() {
                        let path = ValidatedPath::new(&format!("/bench/doc_{i}.md")).unwrap();
                        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
                    }
                    (tree, keys.clone())
                },
                |(mut tree, keys)| {
                    // Delete half the keys
                    for key in keys.iter().take(size / 2) {
                        tree = btree::delete_from_tree(tree, key).unwrap();
                    }
                    black_box(tree)
                },
                criterion::BatchSize::SmallInput
            );
        });
    }
    */

    group.finish();
}

/// Compare O(n) vs O(log n) performance characteristics
fn bench_complexity_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("complexity_comparison");

    // Linear search simulation (what we're avoiding)
    group.bench_function("linear_search_10k", |b| {
        let keys: Vec<_> = (0..10000)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        let target = &keys[5000]; // Middle element

        b.iter(|| {
            // Simulate O(n) linear search
            for key in &keys {
                if key == target {
                    black_box(true);
                    break;
                }
            }
        });
    });

    // B+ tree search (should be much faster)
    group.bench_function("btree_search_10k", |b| {
        let mut tree = btree::create_empty_tree();
        let keys: Vec<_> = (0..10000)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();

        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("bench/doc_{i}.md")).unwrap();
            tree = btree::insert_into_tree(tree, *key, path).unwrap();
        }

        let target = &keys[5000];

        b.iter(|| {
            black_box(btree::search_in_tree(&tree, target));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_btree_insertion,
    bench_btree_search,
    bench_btree_deletion,
    bench_complexity_comparison
);
criterion_main!(benches);
