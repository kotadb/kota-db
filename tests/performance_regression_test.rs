// Performance Regression Tests - Stage 1: TDD
// These tests ensure B+ tree operations maintain O(log n) performance

use kotadb::{ValidatedDocumentId, ValidatedPath, btree};
use uuid::Uuid;
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Performance thresholds for regression detection
#[derive(Debug, Clone)]
struct PerformanceThresholds {
    /// Maximum allowed growth factor when input size increases 10x
    max_growth_factor: f64,
    /// Maximum time per operation in microseconds
    max_operation_time_us: f64,
    /// Minimum operations per second
    min_operations_per_sec: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_growth_factor: 4.0,  // For O(log n), 10x data = ~3.3x time
            max_operation_time_us: 100.0,  // 100 microseconds max per operation
            min_operations_per_sec: 10_000.0,  // At least 10k ops/sec
        }
    }
}

/// Performance measurement result
#[derive(Debug)]
struct PerformanceResult {
    size: usize,
    total_duration: Duration,
    operations: usize,
    avg_time_per_op: Duration,
    ops_per_second: f64,
}

impl PerformanceResult {
    fn new(size: usize, operations: usize, duration: Duration) -> Self {
        let avg_time_per_op = duration / operations as u32;
        let ops_per_second = operations as f64 / duration.as_secs_f64();
        
        Self {
            size,
            total_duration: duration,
            operations,
            avg_time_per_op,
            ops_per_second,
        }
    }
}

/// Measure insertion performance at different scales
fn measure_insertion_performance(sizes: &[usize]) -> Vec<PerformanceResult> {
    let mut results = Vec::new();
    
    for &size in sizes {
        // Generate test data
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        let paths: Vec<_> = (0..size)
            .map(|i| ValidatedPath::new(&format!("/perf/doc_{}.md", i)).unwrap())
            .collect();
        
        // Measure insertion time
        let start = Instant::now();
        let mut tree = btree::create_empty_tree();
        for i in 0..size {
            tree = btree::insert_into_tree(tree, keys[i].clone(), paths[i].clone())
                .expect("Insertion should succeed");
        }
        let duration = start.elapsed();
        
        results.push(PerformanceResult::new(size, size, duration));
    }
    
    results
}

/// Measure search performance at different scales
fn measure_search_performance(sizes: &[usize]) -> Vec<PerformanceResult> {
    let mut results = Vec::new();
    
    for &size in sizes {
        // Build tree
        let mut tree = btree::create_empty_tree();
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
            
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(&format!("/perf/doc_{}.md", i)).unwrap();
            tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
        }
        
        // Search for random keys
        let search_count = size.min(1000); // Search up to 1000 keys
        let search_indices: Vec<_> = (0..search_count)
            .map(|i| i * size / search_count)
            .collect();
        
        let start = Instant::now();
        for &idx in &search_indices {
            let _ = btree::search_in_tree(&tree, &keys[idx]);
        }
        let duration = start.elapsed();
        
        results.push(PerformanceResult::new(size, search_count, duration));
    }
    
    results
}

/// Measure deletion performance at different scales
fn measure_deletion_performance(sizes: &[usize]) -> Vec<PerformanceResult> {
    let mut results = Vec::new();
    
    for &size in sizes {
        // Build tree
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        
        let mut tree = btree::create_empty_tree();
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(&format!("/perf/doc_{}.md", i)).unwrap();
            tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
        }
        
        // Delete half the keys
        let delete_count = size / 2;
        let start = Instant::now();
        for i in 0..delete_count {
            tree = btree::delete_from_tree(tree, &keys[i]).unwrap();
        }
        let duration = start.elapsed();
        
        results.push(PerformanceResult::new(size, delete_count, duration));
    }
    
    results
}

/// Verify performance meets O(log n) characteristics
fn verify_logarithmic_growth(results: &[PerformanceResult], thresholds: &PerformanceThresholds) -> Result<(), String> {
    if results.len() < 2 {
        return Ok(());
    }
    
    for i in 1..results.len() {
        let prev = &results[i - 1];
        let curr = &results[i];
        
        let size_ratio = curr.size as f64 / prev.size as f64;
        let time_ratio = curr.avg_time_per_op.as_secs_f64() / prev.avg_time_per_op.as_secs_f64();
        
        // For O(log n), when size increases by N, time should increase by log(N)
        let expected_ratio = size_ratio.log2();
        
        if time_ratio > thresholds.max_growth_factor {
            return Err(format!(
                "Performance degradation detected: size {}→{} ({}x), time ratio {:.2}x (expected ~{:.2}x)",
                prev.size, curr.size, size_ratio, time_ratio, expected_ratio
            ));
        }
        
        if curr.avg_time_per_op.as_micros() as f64 > thresholds.max_operation_time_us {
            return Err(format!(
                "Operation too slow at size {}: {:.2}μs (max allowed: {:.2}μs)",
                curr.size,
                curr.avg_time_per_op.as_micros() as f64,
                thresholds.max_operation_time_us
            ));
        }
        
        if curr.ops_per_second < thresholds.min_operations_per_sec {
            return Err(format!(
                "Throughput too low at size {}: {:.0} ops/sec (min required: {:.0})",
                curr.size,
                curr.ops_per_second,
                thresholds.min_operations_per_sec
            ));
        }
    }
    
    Ok(())
}

#[test]
fn test_insertion_performance_regression() {
    let sizes = vec![100, 1_000, 10_000, 100_000];
    let thresholds = PerformanceThresholds::default();
    
    println!("\n=== Insertion Performance Regression Test ===");
    let results = measure_insertion_performance(&sizes);
    
    // Print results
    for result in &results {
        println!(
            "Size: {:7} | Total: {:?} | Avg/op: {:?} | Throughput: {:.0} ops/sec",
            result.size,
            result.total_duration,
            result.avg_time_per_op,
            result.ops_per_second
        );
    }
    
    // Verify logarithmic growth
    if let Err(e) = verify_logarithmic_growth(&results, &thresholds) {
        panic!("Insertion performance regression: {}", e);
    }
}

#[test]
fn test_search_performance_regression() {
    let sizes = vec![100, 1_000, 10_000, 100_000];
    let thresholds = PerformanceThresholds {
        max_operation_time_us: 50.0,  // Searches should be faster
        min_operations_per_sec: 20_000.0,
        ..Default::default()
    };
    
    println!("\n=== Search Performance Regression Test ===");
    let results = measure_search_performance(&sizes);
    
    // Print results
    for result in &results {
        println!(
            "Size: {:7} | Searches: {:5} | Avg/op: {:?} | Throughput: {:.0} ops/sec",
            result.size,
            result.operations,
            result.avg_time_per_op,
            result.ops_per_second
        );
    }
    
    // Verify logarithmic growth
    if let Err(e) = verify_logarithmic_growth(&results, &thresholds) {
        panic!("Search performance regression: {}", e);
    }
}

#[test]
fn test_deletion_performance_regression() {
    let sizes = vec![100, 1_000, 10_000];  // Smaller sizes for deletion test
    let thresholds = PerformanceThresholds {
        max_operation_time_us: 200.0,  // Deletions with rebalancing take longer
        min_operations_per_sec: 5_000.0,
        ..Default::default()
    };
    
    println!("\n=== Deletion Performance Regression Test ===");
    let results = measure_deletion_performance(&sizes);
    
    // Print results
    for result in &results {
        println!(
            "Size: {:7} | Deletions: {:5} | Avg/op: {:?} | Throughput: {:.0} ops/sec",
            result.size,
            result.operations,
            result.avg_time_per_op,
            result.ops_per_second
        );
    }
    
    // Verify logarithmic growth
    if let Err(e) = verify_logarithmic_growth(&results, &thresholds) {
        panic!("Deletion performance regression: {}", e);
    }
}

#[test]
fn test_mixed_operations_performance() {
    println!("\n=== Mixed Operations Performance Test ===");
    
    let size = 10_000;
    let keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
    
    // Build initial tree
    let mut tree = btree::create_empty_tree();
    for (i, key) in keys.iter().take(size / 2).enumerate() {
        let path = ValidatedPath::new(&format!("/mixed/doc_{}.md", i)).unwrap();
        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
    }
    
    // Mixed operations
    let operations = size;
    let start = Instant::now();
    
    for i in 0..operations {
        match i % 3 {
            0 => {
                // Insert
                let idx = size / 2 + i / 3;
                if idx < size {
                    let path = ValidatedPath::new(&format!("/mixed/new_{}.md", i)).unwrap();
                    tree = btree::insert_into_tree(tree, keys[idx].clone(), path).unwrap();
                }
            }
            1 => {
                // Search
                let idx = i % (size / 2);
                let _ = btree::search_in_tree(&tree, &keys[idx]);
            }
            2 => {
                // Delete
                let idx = i / 3;
                if idx < size / 4 {
                    tree = btree::delete_from_tree(tree, &keys[idx]).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    
    let duration = start.elapsed();
    let ops_per_second = operations as f64 / duration.as_secs_f64();
    
    println!(
        "Mixed operations: {} ops in {:?} ({:.0} ops/sec)",
        operations, duration, ops_per_second
    );
    
    assert!(ops_per_second > 5_000.0, "Mixed operations too slow: {:.0} ops/sec", ops_per_second);
    assert!(btree::is_valid_btree(&tree), "Tree invariants violated after mixed operations");
}

#[test]
fn test_performance_stability() {
    println!("\n=== Performance Stability Test ===");
    
    let size = 10_000;
    let iterations = 5;
    let mut insertion_times = Vec::new();
    let mut search_times = Vec::new();
    
    for iteration in 0..iterations {
        // Generate fresh data for each iteration
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        
        // Measure insertion
        let start = Instant::now();
        let mut tree = btree::create_empty_tree();
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(&format!("/stable/doc_{}.md", i)).unwrap();
            tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
        }
        let insert_duration = start.elapsed();
        insertion_times.push(insert_duration);
        
        // Measure searches
        let search_count = 1000;
        let start = Instant::now();
        for i in 0..search_count {
            let idx = (i * size) / search_count;
            let _ = btree::search_in_tree(&tree, &keys[idx]);
        }
        let search_duration = start.elapsed();
        search_times.push(search_duration);
        
        println!(
            "Iteration {}: Insert {:?}, Search {:?}",
            iteration + 1, insert_duration, search_duration
        );
    }
    
    // Calculate variance
    let avg_insert = insertion_times.iter().map(|d| d.as_micros()).sum::<u128>() / iterations as u128;
    let avg_search = search_times.iter().map(|d| d.as_micros()).sum::<u128>() / iterations as u128;
    
    let insert_variance = insertion_times.iter()
        .map(|d| {
            let diff = d.as_micros() as i128 - avg_insert as i128;
            (diff * diff) as f64
        })
        .sum::<f64>() / iterations as f64;
    
    let search_variance = search_times.iter()
        .map(|d| {
            let diff = d.as_micros() as i128 - avg_search as i128;
            (diff * diff) as f64
        })
        .sum::<f64>() / iterations as f64;
    
    let insert_std_dev = (insert_variance.sqrt() / avg_insert as f64) * 100.0;
    let search_std_dev = (search_variance.sqrt() / avg_search as f64) * 100.0;
    
    println!("\nPerformance stability:");
    println!("  Insertion std dev: {:.1}%", insert_std_dev);
    println!("  Search std dev: {:.1}%", search_std_dev);
    
    // Performance should be stable (< 20% standard deviation)
    assert!(insert_std_dev < 20.0, "Insertion performance too unstable: {:.1}%", insert_std_dev);
    assert!(search_std_dev < 20.0, "Search performance too unstable: {:.1}%", search_std_dev);
}