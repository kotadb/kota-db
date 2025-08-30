//! Codebase Intelligence Platform Benchmarks
//!
//! This benchmark suite validates KotaDB's core value proposition:
//! enabling AI assistants to understand code relationships and structure
//! with sub-10ms query latency and efficient indexing performance.
//!
//! ## Benchmark Limitations
//!
//! **Process Spawning Overhead**: These benchmarks use `cargo run` to test end-to-end
//! CLI performance, which adds ~100ms+ process spawn overhead per measurement. This
//! affects sub-10ms latency validation accuracy. For pure library performance,
//! consider in-process benchmarking.
//!
//! **Platform Dependencies**: Benchmarks assume Unix-like systems for some operations.
//! Cross-platform compatibility has been improved but some edge cases may remain.
//!
//! **Concurrency Testing**: Current concurrent tests spawn separate processes rather
//! than testing true concurrent access to shared database instances, which may not
//! fully reflect real-world AI assistant usage patterns.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

// Performance targets and configuration
const TARGET_QUERY_LATENCY_MS: u64 = 10;
const INDEXING_SAMPLE_SIZE: usize = 5; // Fewer samples for long-running repository indexing operations
const QUERY_SAMPLE_SIZE: usize = 10; // Standard sample size for query operations
const MEASUREMENT_TIME_SECS: u64 = 30; // Measurement time for query benchmarks

/// Benchmark codebase indexing performance across different repository sizes
fn benchmark_codebase_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("codebase_indexing");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(INDEXING_SAMPLE_SIZE);

    // Validate that we have a source directory to work with
    let current_repo = Path::new(".");
    let src_dir = Path::new("src");

    if !current_repo.exists() {
        panic!("Current directory does not exist - cannot run repository indexing benchmarks");
    }

    if !src_dir.exists() {
        panic!(
            "Source directory 'src' not found - cannot run meaningful codebase indexing benchmarks"
        );
    }

    // Test indexing on the current KotaDB repository (dogfooding approach)
    if current_repo.exists() {
        group.bench_function("kotadb_self_index", |b| {
            b.iter(|| {
                let temp_db =
                    TempDir::new().expect("Failed to create temporary directory for benchmark");
                let start = Instant::now();

                // NOTE: This benchmark measures end-to-end CLI performance including process spawn overhead (~100ms+)
                // For pure indexing performance measurement, consider in-process benchmarking
                let db_path = temp_db
                    .path()
                    .to_str()
                    .expect("Temporary directory path contains invalid UTF-8");
                let output = Command::new("cargo")
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
                    .expect("Failed to run index-codebase command");

                let duration = start.elapsed();

                if !output.status.success() {
                    eprintln!(
                        "Index command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                black_box(duration)
            });
        });
    }

    group.finish();
}

/// Benchmark code search performance (<10ms target)
fn benchmark_code_search(c: &mut Criterion) {
    // Set up indexed database first
    let temp_db = TempDir::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    // Index current repository for testing
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
        .expect("Failed to index repository for benchmarking");

    if !index_output.status.success() {
        panic!(
            "Failed to set up code search benchmark database - cannot proceed with benchmarks: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );
    }

    let mut group = c.benchmark_group("code_search");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(QUERY_SAMPLE_SIZE);
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));

    // Test common code search patterns
    let search_terms = [
        "Storage", "Index", "Document", "Query", "async", "Result", "impl", "struct", "fn", "pub",
        "use", "mod",
    ];

    for term in &search_terms {
        group.bench_with_input(
            BenchmarkId::new("search_code", term),
            term,
            |b, &search_term| {
                b.iter(|| {
                    let start = Instant::now();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            db_path,
                            "search-code",
                            search_term,
                        ])
                        .output()
                        .expect("Failed to run search-code command");

                    let duration = start.elapsed();

                    // Validate that query meets latency target
                    if duration.as_millis() > TARGET_QUERY_LATENCY_MS as u128 {
                        eprintln!(
                            "WARNING: Query '{}' took {}ms (target: {}ms)",
                            search_term,
                            duration.as_millis(),
                            TARGET_QUERY_LATENCY_MS
                        );
                    }

                    black_box((duration, output.stdout.len()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark symbol search performance
fn benchmark_symbol_search(c: &mut Criterion) {
    let temp_db =
        TempDir::new().expect("Failed to create temporary directory for symbol search benchmark");
    let db_path = temp_db
        .path()
        .to_str()
        .expect("Temporary directory path contains invalid UTF-8");

    // Index current repository
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
        .expect("Failed to execute symbol search indexing command");

    if !index_output.status.success() {
        panic!(
            "Failed to set up symbol search benchmark database: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );
    }

    let mut group = c.benchmark_group("symbol_search");
    group.sampling_mode(SamplingMode::Flat);

    // Test symbol patterns common in Rust codebases
    let symbol_patterns = [
        "*Storage*",
        "*Index*",
        "*Builder*",
        "*Query*",
        "*Error*",
        "create_*",
        "validate_*",
        "process_*",
        "*_test",
        "test_*",
    ];

    for pattern in &symbol_patterns {
        group.bench_with_input(
            BenchmarkId::new("search_symbols", pattern),
            pattern,
            |b, &symbol_pattern| {
                b.iter(|| {
                    let start = Instant::now();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            db_path,
                            "search-symbols",
                            symbol_pattern,
                        ])
                        .output()
                        .expect("Failed to run search-symbols command");

                    let duration = start.elapsed();

                    if duration.as_millis() > TARGET_QUERY_LATENCY_MS as u128 {
                        eprintln!(
                            "WARNING: Symbol search '{}' took {}ms (target: {}ms)",
                            symbol_pattern,
                            duration.as_millis(),
                            TARGET_QUERY_LATENCY_MS
                        );
                    }

                    black_box((duration, output.stdout.len()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark relationship queries (find-callers, analyze-impact)
fn benchmark_relationship_queries(c: &mut Criterion) {
    let temp_db = TempDir::new()
        .expect("Failed to create temporary directory for relationship query benchmark");
    let db_path = temp_db
        .path()
        .to_str()
        .expect("Temporary directory path contains invalid UTF-8");

    // Index current repository
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
        .expect("Failed to execute relationship query indexing command");

    if !index_output.status.success() {
        panic!(
            "Failed to set up relationship query benchmark database: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );
    }

    let mut group = c.benchmark_group("relationship_queries");
    group.sampling_mode(SamplingMode::Flat);

    // Test relationship queries on common symbols in KotaDB
    let symbols = [
        "Storage",
        "Document",
        "Index",
        "Query",
        "create_file_storage",
        "DocumentBuilder",
        "ValidatedPath",
        "Result",
    ];

    for symbol in &symbols {
        // Benchmark find-callers
        group.bench_with_input(
            BenchmarkId::new("find_callers", symbol),
            symbol,
            |b, &symbol_name| {
                b.iter(|| {
                    let start = Instant::now();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            db_path,
                            "find-callers",
                            symbol_name,
                        ])
                        .output()
                        .expect("Failed to run find-callers command");

                    let duration = start.elapsed();

                    if duration.as_millis() > TARGET_QUERY_LATENCY_MS as u128 {
                        eprintln!(
                            "WARNING: find-callers '{}' took {}ms (target: {}ms)",
                            symbol_name,
                            duration.as_millis(),
                            TARGET_QUERY_LATENCY_MS
                        );
                    }

                    black_box((duration, output.stdout.len()))
                });
            },
        );

        // Benchmark analyze-impact
        group.bench_with_input(
            BenchmarkId::new("analyze_impact", symbol),
            symbol,
            |b, &symbol_name| {
                b.iter(|| {
                    let start = Instant::now();

                    let output = Command::new("cargo")
                        .args([
                            "run",
                            "--release",
                            "--bin",
                            "kotadb",
                            "--",
                            "-d",
                            db_path,
                            "analyze-impact",
                            symbol_name,
                        ])
                        .output()
                        .expect("Failed to run analyze-impact command");

                    let duration = start.elapsed();

                    if duration.as_millis() > TARGET_QUERY_LATENCY_MS as u128 {
                        eprintln!(
                            "WARNING: analyze-impact '{}' took {}ms (target: {}ms)",
                            symbol_name,
                            duration.as_millis(),
                            TARGET_QUERY_LATENCY_MS
                        );
                    }

                    black_box((duration, output.stdout.len()))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark database statistics performance
fn benchmark_stats_queries(c: &mut Criterion) {
    let temp_db = TempDir::new().expect("Failed to create temporary directory for stats benchmark");
    let db_path = temp_db
        .path()
        .to_str()
        .expect("Temporary directory path contains invalid UTF-8");

    // Index current repository
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
        .expect("Failed to execute stats benchmark indexing command");

    if !index_output.status.success() {
        panic!(
            "Failed to set up stats benchmark database: {}",
            String::from_utf8_lossy(&index_output.stderr)
        );
    }

    let mut group = c.benchmark_group("stats_queries");

    group.bench_function("database_stats", |b| {
        b.iter(|| {
            let start = Instant::now();

            let output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kotadb",
                    "--",
                    "-d",
                    db_path,
                    "stats",
                ])
                .output()
                .expect("Failed to run stats command");

            let duration = start.elapsed();

            if duration.as_millis() > TARGET_QUERY_LATENCY_MS as u128 {
                eprintln!(
                    "WARNING: stats query took {}ms (target: {}ms)",
                    duration.as_millis(),
                    TARGET_QUERY_LATENCY_MS
                );
            }

            black_box((duration, output.stdout.len()))
        });
    });

    group.bench_function("symbol_stats", |b| {
        b.iter(|| {
            let start = Instant::now();

            let output = Command::new("cargo")
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
                .expect("Failed to run stats --symbols command");

            let duration = start.elapsed();

            black_box((duration, output.stdout.len()))
        });
    });

    group.finish();
}

criterion_group!(
    name = codebase_intelligence_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(60))
        .warm_up_time(Duration::from_secs(10))
        .significance_level(0.1)
        .noise_threshold(0.02);
    targets =
        benchmark_codebase_indexing,
        benchmark_code_search,
        benchmark_symbol_search,
        benchmark_relationship_queries,
        benchmark_stats_queries
);
criterion_main!(codebase_intelligence_benches);
