// Complexity Comparison Tests - Stage 1: TDD
// Compare B+ tree O(log n) performance against O(n) baseline implementations

use kotadb::{btree, ValidatedDocumentId, ValidatedPath};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Linear search baseline implementation
struct LinearIndex {
    entries: Vec<(ValidatedDocumentId, ValidatedPath)>,
}

impl LinearIndex {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// O(n) insertion - append to end
    fn insert(&mut self, key: ValidatedDocumentId, value: ValidatedPath) {
        // Remove existing entry if present (O(n))
        self.entries.retain(|(k, _)| k != &key);
        self.entries.push((key, value));
    }

    /// O(n) search - linear scan
    fn search(&self, key: &ValidatedDocumentId) -> Option<&ValidatedPath> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// O(n) deletion - find and remove
    #[allow(dead_code)]
    fn delete(&mut self, key: &ValidatedDocumentId) {
        self.entries.retain(|(k, _)| k != key);
    }
}

/// HashMap baseline implementation
struct HashMapIndex {
    map: HashMap<ValidatedDocumentId, ValidatedPath>,
}

impl HashMapIndex {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// O(1) average insertion
    fn insert(&mut self, key: ValidatedDocumentId, value: ValidatedPath) {
        self.map.insert(key, value);
    }

    /// O(1) average search
    fn search(&self, key: &ValidatedDocumentId) -> Option<&ValidatedPath> {
        self.map.get(key)
    }

    /// O(1) average deletion
    #[allow(dead_code)]
    fn delete(&mut self, key: &ValidatedDocumentId) {
        self.map.remove(key);
    }
}

/// Measure and compare insertion performance
fn compare_insertion_performance(sizes: &[usize]) {
    println!("\n=== Insertion Performance Comparison ===");
    println!("Size      | Linear (O(n)) | HashMap (O(1)) | B+ Tree (O(log n)) | Linear/B+Tree | HashMap/B+Tree");
    println!("----------|---------------|----------------|--------------------|--------------|--------------");

    for &size in sizes {
        // Generate test data
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        let paths: Vec<_> = (0..size)
            .map(|i| ValidatedPath::new(format!("/cmp/doc_{i}.md")).unwrap())
            .collect();

        // Linear insertion
        let start = Instant::now();
        let mut linear = LinearIndex::new();
        for i in 0..size {
            linear.insert(keys[i], paths[i].clone());
        }
        let linear_time = start.elapsed();

        // HashMap insertion
        let start = Instant::now();
        let mut hashmap = HashMapIndex::new();
        for i in 0..size {
            hashmap.insert(keys[i], paths[i].clone());
        }
        let hashmap_time = start.elapsed();

        // B+ tree insertion
        let start = Instant::now();
        let mut btree_idx = btree::create_empty_tree();
        for i in 0..size {
            btree_idx = btree::insert_into_tree(btree_idx, keys[i], paths[i].clone()).unwrap();
        }
        let btree_time = start.elapsed();

        // Calculate ratios
        let linear_ratio = linear_time.as_micros() as f64 / btree_time.as_micros() as f64;
        let hashmap_ratio = hashmap_time.as_micros() as f64 / btree_time.as_micros() as f64;

        println!(
            "{:9} | {:>13} | {:>14} | {:>18} | {:>12.2}x | {:>13.2}x",
            size,
            format!("{:?}", linear_time),
            format!("{:?}", hashmap_time),
            format!("{:?}", btree_time),
            linear_ratio,
            hashmap_ratio
        );
    }
}

/// Measure and compare search performance
fn compare_search_performance(sizes: &[usize]) {
    println!("\n=== Search Performance Comparison ===");
    println!("Size      | Searches | Linear (O(n)) | HashMap (O(1)) | B+ Tree (O(log n)) | Linear/B+Tree | HashMap/B+Tree");
    println!("----------|----------|---------------|----------------|--------------------|--------------|--------------");

    for &size in sizes {
        // Build indices
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();

        let mut linear = LinearIndex::new();
        let mut hashmap = HashMapIndex::new();
        let mut btree_idx = btree::create_empty_tree();

        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("/cmp/doc_{i}.md")).unwrap();
            linear.insert(*key, path.clone());
            hashmap.insert(*key, path.clone());
            btree_idx = btree::insert_into_tree(btree_idx, *key, path).unwrap();
        }

        // Search for random keys
        let search_count = (size / 10).clamp(100, 1000);
        let search_indices: Vec<_> = (0..search_count)
            .map(|i| (i * size) / search_count)
            .collect();

        // Linear search
        let start = Instant::now();
        for &idx in &search_indices {
            let _ = linear.search(&keys[idx]);
        }
        let linear_time = start.elapsed();

        // HashMap search
        let start = Instant::now();
        for &idx in &search_indices {
            let _ = hashmap.search(&keys[idx]);
        }
        let hashmap_time = start.elapsed();

        // B+ tree search
        let start = Instant::now();
        for &idx in &search_indices {
            let _ = btree::search_in_tree(&btree_idx, &keys[idx]);
        }
        let btree_time = start.elapsed();

        // Calculate ratios
        let linear_ratio = linear_time.as_micros() as f64 / btree_time.as_micros() as f64;
        let hashmap_ratio = hashmap_time.as_micros() as f64 / btree_time.as_micros() as f64;

        println!(
            "{:9} | {:8} | {:>13} | {:>14} | {:>18} | {:>12.2}x | {:>13.2}x",
            size,
            search_count,
            format!("{:?}", linear_time),
            format!("{:?}", hashmap_time),
            format!("{:?}", btree_time),
            linear_ratio,
            hashmap_ratio
        );
    }
}

/// Demonstrate O(n) vs O(log n) growth
fn demonstrate_complexity_growth() {
    println!("\n=== Complexity Growth Demonstration ===");
    println!("Testing how performance scales with data size...\n");

    let sizes = vec![100, 1_000, 10_000, 100_000];
    let mut linear_times = Vec::new();
    let mut btree_times = Vec::new();

    for &size in &sizes {
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();

        // Build linear index and measure worst-case search
        let mut linear = LinearIndex::new();
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("/growth/doc_{i}.md")).unwrap();
            linear.insert(*key, path);
        }

        // Search for last element (worst case for linear)
        let target = &keys[size - 1];
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = linear.search(target);
        }
        let linear_avg = start.elapsed() / 1000;
        linear_times.push((size, linear_avg));

        // Build B+ tree and measure search
        let mut btree_idx = btree::create_empty_tree();
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("/growth/doc_{i}.md")).unwrap();
            btree_idx = btree::insert_into_tree(btree_idx, *key, path).unwrap();
        }

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = btree::search_in_tree(&btree_idx, target);
        }
        let btree_avg = start.elapsed() / 1000;
        btree_times.push((size, btree_avg));
    }

    // Print results and growth factors
    println!("Linear Search (O(n)):");
    for i in 0..linear_times.len() {
        let (size, time) = linear_times[i];
        print!("  Size {size:7}: {time:?}");

        if i > 0 {
            let size_growth = size as f64 / linear_times[i - 1].0 as f64;
            let time_growth = time.as_nanos() as f64 / linear_times[i - 1].1.as_nanos() as f64;
            print!(" ({size_growth}x size = {time_growth:.2}x time)");
        }
        println!();
    }

    println!("\nB+ Tree Search (O(log n)):");
    for i in 0..btree_times.len() {
        let (size, time) = btree_times[i];
        print!("  Size {size:7}: {time:?}");

        if i > 0 {
            let size_growth = size as f64 / btree_times[i - 1].0 as f64;
            let time_growth = time.as_nanos() as f64 / btree_times[i - 1].1.as_nanos() as f64;
            print!(" ({size_growth}x size = {time_growth:.2}x time)");
        }
        println!();
    }

    println!("\nAnalysis:");
    println!("- Linear search: Time grows proportionally with size (O(n))");
    println!("- B+ tree search: Time grows logarithmically with size (O(log n))");
    println!("- As data grows, B+ tree advantage increases dramatically");
}

#[test]
fn test_insertion_complexity_comparison() {
    let sizes = vec![100, 1_000, 10_000];
    compare_insertion_performance(&sizes);
}

#[test]
fn test_search_complexity_comparison() {
    let sizes = vec![100, 1_000, 10_000, 100_000];
    compare_search_performance(&sizes);
}

#[test]
fn test_complexity_growth_demonstration() {
    demonstrate_complexity_growth();
}

#[test]
fn test_worst_case_scenarios() {
    println!("\n=== Worst Case Scenario Comparison ===");

    let size = 10_000;
    let keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();

    // Build indices
    let mut linear = LinearIndex::new();
    let mut btree_idx = btree::create_empty_tree();

    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(format!("/worst/doc_{i}.md")).unwrap();
        linear.insert(*key, path.clone());
        btree_idx = btree::insert_into_tree(btree_idx, *key, path).unwrap();
    }

    // Test 1: Search for non-existent key (worst case for both)
    let non_existent = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = linear.search(&non_existent);
    }
    let linear_worst = start.elapsed() / 100;

    let start = Instant::now();
    for _ in 0..100 {
        let _ = btree::search_in_tree(&btree_idx, &non_existent);
    }
    let btree_worst = start.elapsed() / 100;

    println!("Non-existent key search (worst case):");
    println!("  Linear: {linear_worst:?} (scans all {size} elements)");
    println!(
        "  B+Tree: {:?} (traverses ~{} levels)",
        btree_worst,
        (size as f64).log2() as u32
    );
    println!(
        "  Speedup: {:.2}x",
        linear_worst.as_nanos() as f64 / btree_worst.as_nanos() as f64
    );

    // Test 2: Sequential access pattern
    println!("\nSequential access pattern:");

    let start = Instant::now();
    for key in keys.iter().take(100) {
        let _ = linear.search(key);
    }
    let linear_seq = start.elapsed();

    let start = Instant::now();
    for key in keys.iter().take(100) {
        let _ = btree::search_in_tree(&btree_idx, key);
    }
    let btree_seq = start.elapsed();

    println!("  Linear: {linear_seq:?}");
    println!("  B+Tree: {btree_seq:?}");
    println!("  Note: B+ tree maintains good cache locality in leaf nodes");
}

#[test]
fn test_performance_at_scale() {
    println!("\n=== Performance at Scale Test ===");
    println!("Demonstrating B+ tree advantage at large scale...\n");

    let size = 1_000_000; // 1 million entries
    let sample_size = 1000; // Sample searches

    println!("Building B+ tree with {size} entries...");
    let keys: Vec<_> = (0..sample_size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();

    let mut btree_idx = btree::create_empty_tree();
    let start = Instant::now();

    // Insert sample keys
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(format!("/scale/doc_{i}.md")).unwrap();
        btree_idx = btree::insert_into_tree(btree_idx, *key, path).unwrap();
    }

    // Insert many more keys
    for i in sample_size..size {
        if i % 100_000 == 0 {
            println!("  Inserted {i} entries...");
        }
        let key = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
        let path = ValidatedPath::new(format!("/scale/doc_{i}.md")).unwrap();
        btree_idx = btree::insert_into_tree(btree_idx, key, path).unwrap();
    }

    let build_time = start.elapsed();
    println!("Built tree in {build_time:?}");

    // Search performance at scale
    let start = Instant::now();
    for key in &keys {
        let _ = btree::search_in_tree(&btree_idx, key);
    }
    let search_time = start.elapsed();
    let avg_search = search_time / sample_size as u32;

    println!("\nSearch performance with {size} entries:");
    println!("  {sample_size} searches in {search_time:?}");
    println!("  Average: {avg_search:?} per search");
    println!("  Tree depth: ~{} levels", (size as f64).log2() as u32);

    // Compare with theoretical linear search time
    let theoretical_linear_us = (size as f64 / 2.0) * 0.01; // Assume 0.01μs per comparison
    println!("\nTheoretical linear search would take ~{theoretical_linear_us:.0}μs average");
    println!(
        "B+ tree is ~{:.0}x faster!",
        theoretical_linear_us / avg_search.as_micros() as f64
    );
}
