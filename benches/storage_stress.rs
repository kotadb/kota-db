#![allow(clippy::uninlined_format_args)]
// Storage Stress Test - Simplified Version for Initial Testing
// Tests storage engine performance with realistic document loads

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use kotadb::{create_file_storage, Storage};
use std::time::Instant;
use tempfile::TempDir;
use tokio::runtime::Runtime;

mod stress_data_generator;
use stress_data_generator::{DataGenConfig, StressDocumentGenerator};

/// Storage stress test with different dataset sizes
fn storage_stress_test(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("storage_stress");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    // Test different scales
    for &size in &[1_000, 5_000] {
        let config = DataGenConfig {
            count: size,
            base_size_bytes: 3_000, // 3KB average documents
            size_variation: 0.6,
            ..Default::default()
        };

        let mut generator = StressDocumentGenerator::new(config);
        let documents = rt.block_on(async {
            generator
                .generate_documents()
                .expect("Failed to generate documents")
        });

        println!("📊 Testing {} documents: {}", size, generator.get_stats());

        group.bench_with_input(
            BenchmarkId::new("bulk_operations", size),
            &documents,
            |b, docs| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let mut storage =
                            create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000))
                                .await
                                .unwrap();

                        let start = Instant::now();

                        // Insert all documents
                        for doc in docs.iter().take(100) {
                            // Limit to 100 for quick test
                            storage.insert(doc.clone()).await.unwrap();
                        }

                        // Test some reads
                        let mut read_count = 0;
                        for doc in docs.iter().take(50) {
                            if let Ok(Some(_)) = storage.get(&doc.id).await {
                                read_count += 1;
                            }
                        }

                        let duration = start.elapsed();
                        black_box((read_count, duration))
                    })
                });
            },
        );
    }

    group.finish();
}

/// Test realistic mixed workload
fn mixed_workload_test(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("mixed_workload");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    let config = DataGenConfig {
        count: 1_000,
        base_size_bytes: 2_000,
        ..Default::default()
    };

    let mut generator = StressDocumentGenerator::new(config);
    let documents = rt.block_on(async {
        generator
            .generate_documents()
            .expect("Failed to generate documents")
    });

    group.bench_function("realistic_workload", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let mut storage =
                    create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000))
                        .await
                        .unwrap();

                // Pre-populate with some documents
                for doc in documents.iter().take(50) {
                    storage.insert(doc.clone()).await.unwrap();
                }

                let start = Instant::now();
                let mut operations = 0;

                // Simulate realistic workload: 70% reads, 30% writes
                for i in 0..100 {
                    if i % 10 < 7 {
                        // 70% reads
                        if let Some(doc) = documents.get(i % 50) {
                            let _ = storage.get(&doc.id).await;
                            operations += 1;
                        }
                    } else {
                        // 30% writes
                        if let Some(doc) = documents.get(50 + (i % 20)) {
                            let _ = storage.insert(doc.clone()).await;
                            operations += 1;
                        }
                    }
                }

                let duration = start.elapsed();
                black_box((operations, duration))
            })
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = storage_stress_test, mixed_workload_test
);
criterion_main!(benches);
