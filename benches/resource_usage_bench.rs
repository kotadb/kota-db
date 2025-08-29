//! Resource Usage Benchmarks for Codebase Intelligence Platform
//!
//! Validates memory usage, disk usage, and concurrent operation performance
//! to ensure KotaDB scales efficiently with large codebases and multiple queries.

use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const CONCURRENT_QUERY_COUNT: usize = 10;
const TARGET_MEMORY_OVERHEAD: f64 = 2.5; // <2.5x raw data size

/// Benchmark concurrent query performance
fn benchmark_concurrent_operations(c: &mut Criterion) {
    let temp_db = TempDir::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    // Index current repository once for all concurrent tests
    let index_output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "kotadb",
            "--",
            "-d",
            db_path,
            "index-codebase",
            ".",
        ])
        .output()
        .expect("Failed to index repository");

    if !index_output.status.success() {
        eprintln!(
            "Failed to set up concurrent benchmark database: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );
        return;
    }

    let mut group = c.benchmark_group("concurrent_operations");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(5); // Fewer samples for concurrent tests

    // Test concurrent code searches
    group.bench_function("concurrent_code_search", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(CONCURRENT_QUERY_COUNT));
            let mut handles = Vec::new();

            let search_terms = [
                "Storage", "Index", "Document", "Query", "async", "Result", "impl", "struct", "fn",
                "pub",
            ];

            let start = Instant::now();

            for i in 0..CONCURRENT_QUERY_COUNT {
                let barrier = Arc::clone(&barrier);
                let db_path = db_path.to_string();
                let search_term = search_terms[i % search_terms.len()].to_string();

                let handle = thread::spawn(move || {
                    barrier.wait(); // Synchronize start time

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            &db_path,
                            "search-code",
                            &search_term,
                        ])
                        .output()
                        .expect("Failed to run concurrent search-code");

                    (output.status.success(), output.stdout.len())
                });

                handles.push(handle);
            }

            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.join().unwrap());
            }

            let duration = start.elapsed();
            let successful_queries = results.iter().filter(|(success, _)| *success).count();

            black_box((duration, successful_queries, results.len()))
        });
    });

    // Test concurrent symbol searches
    group.bench_function("concurrent_symbol_search", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(CONCURRENT_QUERY_COUNT));
            let mut handles = Vec::new();

            let symbol_patterns = ["*Storage*", "*Index*", "*Builder*", "*Query*", "*Error*"];

            let start = Instant::now();

            for i in 0..CONCURRENT_QUERY_COUNT {
                let barrier = Arc::clone(&barrier);
                let db_path = db_path.to_string();
                let pattern = symbol_patterns[i % symbol_patterns.len()].to_string();

                let handle = thread::spawn(move || {
                    barrier.wait();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            &db_path,
                            "search-symbols",
                            &pattern,
                        ])
                        .output()
                        .expect("Failed to run concurrent search-symbols");

                    (output.status.success(), output.stdout.len())
                });

                handles.push(handle);
            }

            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.join().unwrap());
            }

            let duration = start.elapsed();
            let successful_queries = results.iter().filter(|(success, _)| *success).count();

            black_box((duration, successful_queries))
        });
    });

    // Test concurrent relationship queries
    group.bench_function("concurrent_relationship_queries", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(CONCURRENT_QUERY_COUNT));
            let mut handles = Vec::new();

            let symbols = ["Storage", "Document", "Index", "Query", "Result"];
            let operations = ["find-callers", "analyze-impact"];

            let start = Instant::now();

            for i in 0..CONCURRENT_QUERY_COUNT {
                let barrier = Arc::clone(&barrier);
                let db_path = db_path.to_string();
                let symbol = symbols[i % symbols.len()].to_string();
                let operation = operations[i % operations.len()].to_string();

                let handle = thread::spawn(move || {
                    barrier.wait();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            &db_path,
                            &operation,
                            &symbol,
                        ])
                        .output()
                        .expect("Failed to run concurrent relationship query");

                    (output.status.success(), output.stdout.len())
                });

                handles.push(handle);
            }

            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.join().unwrap());
            }

            let duration = start.elapsed();
            let successful_queries = results.iter().filter(|(success, _)| *success).count();

            black_box((duration, successful_queries))
        });
    });

    group.finish();
}

/// Benchmark memory and disk usage efficiency
fn benchmark_resource_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_efficiency");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(3); // Very few samples for resource-intensive tests

    group.bench_function("memory_disk_usage", |b| {
        b.iter(|| {
            let temp_db = TempDir::new().unwrap();
            let db_path = temp_db.path().to_str().unwrap();

            // Measure source code size
            let source_size_output = Command::new("du")
                .args(["-sb", "src"])
                .output()
                .expect("Failed to measure source directory size");

            let source_size_str = String::from_utf8_lossy(&source_size_output.stdout);
            let source_size_bytes: u64 = source_size_str
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            let start = Instant::now();

            // Index the repository
            let index_output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kotadb",
                    "--",
                    "-d",
                    db_path,
                    "index-codebase",
                    ".",
                ])
                .output()
                .expect("Failed to index for resource measurement");

            let index_duration = start.elapsed();

            if !index_output.status.success() {
                eprintln!("Failed to index for resource measurement");
                return black_box((0, 0, index_duration));
            }

            // Measure database size
            let db_size_output = Command::new("du")
                .args(["-sb", db_path])
                .output()
                .expect("Failed to measure database size");

            let db_size_str = String::from_utf8_lossy(&db_size_output.stdout);
            let db_size_bytes: u64 = db_size_str
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            let size_ratio = if source_size_bytes > 0 {
                db_size_bytes as f64 / source_size_bytes as f64
            } else {
                0.0
            };

            if size_ratio > TARGET_MEMORY_OVERHEAD {
                eprintln!(
                    "WARNING: Database size overhead: {:.2}x (target: <{:.1}x)",
                    size_ratio, TARGET_MEMORY_OVERHEAD
                );
            } else {
                println!("Database size overhead: {:.2}x (within target)", size_ratio);
            }

            // Get symbol statistics
            let stats_output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kotadb",
                    "--",
                    "-d",
                    db_path,
                    "stats",
                    "--symbols",
                ])
                .output()
                .expect("Failed to get symbol stats");

            let stats_str = String::from_utf8_lossy(&stats_output.stdout);
            let symbol_count = stats_str
                .lines()
                .filter_map(|line| {
                    if line.contains("Total symbols:") {
                        line.split(':').nth(1)?.trim().parse().ok()
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or(0);

            black_box((symbol_count, size_ratio as u64, index_duration))
        });
    });

    group.finish();
}

/// Benchmark indexing throughput (symbols per second)
fn benchmark_indexing_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing_throughput");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(3);

    group.bench_function("symbols_per_second", |b| {
        b.iter(|| {
            let temp_db = TempDir::new().unwrap();
            let db_path = temp_db.path().to_str().unwrap();

            let start = Instant::now();

            // Index the repository
            let index_output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kotadb",
                    "--",
                    "-d",
                    db_path,
                    "index-codebase",
                    ".",
                ])
                .output()
                .expect("Failed to run indexing throughput test");

            let duration = start.elapsed();

            if !index_output.status.success() {
                return black_box((0, 0.0, duration));
            }

            // Get symbol count
            let stats_output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kotadb",
                    "--",
                    "-d",
                    db_path,
                    "stats",
                    "--symbols",
                ])
                .output()
                .expect("Failed to get symbol stats for throughput");

            let stats_str = String::from_utf8_lossy(&stats_output.stdout);
            let symbol_count: u64 = stats_str
                .lines()
                .filter_map(|line| {
                    if line.contains("Total symbols:") {
                        line.split(':').nth(1)?.trim().parse().ok()
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or(0);

            let symbols_per_second = if duration.as_secs_f64() > 0.0 {
                symbol_count as f64 / duration.as_secs_f64()
            } else {
                0.0
            };

            println!(
                "Indexed {} symbols in {:.2}s ({:.0} symbols/sec)",
                symbol_count,
                duration.as_secs_f64(),
                symbols_per_second
            );

            black_box((symbol_count, symbols_per_second, duration))
        });
    });

    group.finish();
}

criterion_group!(
    name = resource_usage_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(120)) // Longer measurement for resource tests
        .warm_up_time(Duration::from_secs(15))
        .significance_level(0.1);
    targets =
        benchmark_concurrent_operations,
        benchmark_resource_efficiency,
        benchmark_indexing_throughput
);
criterion_main!(resource_usage_benches);
