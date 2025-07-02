// Memory Usage and Tree Balance Tests - Stage 1: TDD
// Tests memory efficiency and tree structure properties

use kotadb::{ValidatedDocumentId, ValidatedPath, btree};
use uuid::Uuid;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

/// Custom allocator to track memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

/// Reset memory tracking counters
fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    DEALLOCATED.store(0, Ordering::SeqCst);
}

/// Get current memory usage
fn get_memory_usage() -> (usize, usize) {
    (
        ALLOCATED.load(Ordering::SeqCst),
        DEALLOCATED.load(Ordering::SeqCst)
    )
}

/// Calculate net memory used
fn net_memory_used() -> isize {
    let (allocated, deallocated) = get_memory_usage();
    allocated as isize - deallocated as isize
}

/// Tree statistics for balance analysis
#[derive(Debug)]
struct TreeStats {
    depth: usize,
    total_nodes: usize,
    leaf_nodes: usize,
    internal_nodes: usize,
    total_keys: usize,
    min_keys_per_node: usize,
    max_keys_per_node: usize,
    avg_keys_per_node: f64,
    balance_factor: f64,  // Ratio of min to max path length
}

/// Analyze tree structure and balance
fn analyze_tree_structure(root: &btree::BTreeRoot) -> TreeStats {
    use btree::{BTreeNode, MIN_KEYS, MAX_KEYS};
    
    let mut stats = TreeStats {
        depth: 0,
        total_nodes: 0,
        leaf_nodes: 0,
        internal_nodes: 0,
        total_keys: 0,
        min_keys_per_node: MAX_KEYS + 1,
        max_keys_per_node: 0,
        avg_keys_per_node: 0.0,
        balance_factor: 1.0,
    };
    
    if root.root.is_none() {
        return stats;
    }
    
    let mut min_leaf_depth = usize::MAX;
    let mut max_leaf_depth = 0;
    
    fn analyze_node(
        node: &BTreeNode,
        depth: usize,
        stats: &mut TreeStats,
        min_depth: &mut usize,
        max_depth: &mut usize,
    ) {
        stats.total_nodes += 1;
        
        match node {
            BTreeNode::Leaf { keys, .. } => {
                stats.leaf_nodes += 1;
                stats.total_keys += keys.len();
                stats.min_keys_per_node = stats.min_keys_per_node.min(keys.len());
                stats.max_keys_per_node = stats.max_keys_per_node.max(keys.len());
                
                *min_depth = (*min_depth).min(depth);
                *max_depth = (*max_depth).max(depth);
            }
            BTreeNode::Internal { keys, children } => {
                stats.internal_nodes += 1;
                stats.total_keys += keys.len();
                stats.min_keys_per_node = stats.min_keys_per_node.min(keys.len());
                stats.max_keys_per_node = stats.max_keys_per_node.max(keys.len());
                
                for child in children {
                    analyze_node(child, depth + 1, stats, min_depth, max_depth);
                }
            }
        }
    }
    
    analyze_node(&root.root.as_ref().unwrap(), 0, &mut stats, &mut min_leaf_depth, &mut max_leaf_depth);
    
    stats.depth = max_leaf_depth;
    stats.avg_keys_per_node = stats.total_keys as f64 / stats.total_nodes as f64;
    
    // Balance factor: 1.0 means perfectly balanced (all leaves at same depth)
    if min_leaf_depth > 0 {
        stats.balance_factor = min_leaf_depth as f64 / max_leaf_depth as f64;
    }
    
    stats
}

#[test]
fn test_memory_usage_insertion() {
    println!("\n=== Memory Usage Test: Insertion ===");
    
    let sizes = vec![100, 1_000, 10_000];
    let mut memory_results = Vec::new();
    
    for size in sizes {
        reset_memory_tracking();
        let initial_memory = net_memory_used();
        
        // Create tree and insert elements
        let mut tree = btree::create_empty_tree();
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
            
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(&format!("/mem/doc_{}.md", i)).unwrap();
            tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
        }
        
        let final_memory = net_memory_used();
        let memory_per_entry = (final_memory - initial_memory) as f64 / size as f64;
        
        memory_results.push((size, final_memory - initial_memory, memory_per_entry));
        
        println!("Size: {:7} | Total memory: {:8} bytes | Per entry: {:.1} bytes",
                 size, final_memory - initial_memory, memory_per_entry);
    }
    
    // Verify memory usage scales appropriately
    for i in 1..memory_results.len() {
        let (prev_size, _, prev_per_entry) = memory_results[i-1];
        let (curr_size, _, curr_per_entry) = memory_results[i];
        
        // Memory per entry should remain relatively constant
        let ratio = curr_per_entry / prev_per_entry;
        assert!(ratio < 1.5, 
                "Memory usage per entry increased too much: {}→{} entries caused {:.2}x increase",
                prev_size, curr_size, ratio);
    }
}

#[test]
fn test_memory_cleanup_after_deletion() {
    println!("\n=== Memory Cleanup Test ===");
    
    reset_memory_tracking();
    let initial = net_memory_used();
    
    // Build tree
    let mut tree = btree::create_empty_tree();
    let keys: Vec<_> = (0..1000)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
        
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(&format!("/cleanup/doc_{}.md", i)).unwrap();
        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
    }
    
    let after_insert = net_memory_used();
    println!("After inserting 1000 entries: {} bytes", after_insert - initial);
    
    // Delete half the entries
    for key in keys.iter().take(500) {
        tree = btree::delete_from_tree(tree, key).unwrap();
    }
    
    let after_delete = net_memory_used();
    println!("After deleting 500 entries: {} bytes", after_delete - initial);
    
    // Memory should decrease after deletions
    assert!(after_delete < after_insert, 
            "Memory not released after deletion: {} → {} bytes", 
            after_insert - initial, after_delete - initial);
}

#[test]
fn test_tree_balance_properties() {
    println!("\n=== Tree Balance Properties Test ===");
    
    let sizes = vec![100, 1_000, 10_000];
    
    for size in sizes {
        let mut tree = btree::create_empty_tree();
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
            
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(&format!("/balance/doc_{}.md", i)).unwrap();
            tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
        }
        
        let stats = analyze_tree_structure(&tree);
        
        println!("\nTree with {} entries:", size);
        println!("  Depth: {}", stats.depth);
        println!("  Total nodes: {} (leaf: {}, internal: {})", 
                 stats.total_nodes, stats.leaf_nodes, stats.internal_nodes);
        println!("  Keys per node: min={}, max={}, avg={:.1}", 
                 stats.min_keys_per_node, stats.max_keys_per_node, stats.avg_keys_per_node);
        println!("  Balance factor: {:.3}", stats.balance_factor);
        
        // Verify tree properties
        assert_eq!(stats.balance_factor, 1.0, "Tree not perfectly balanced");
        
        // Theoretical depth for B+ tree
        let theoretical_depth = (size as f64).log(btree::MAX_KEYS as f64).ceil() as usize;
        assert!(stats.depth <= theoretical_depth + 1, 
                "Tree too deep: {} vs theoretical {}", stats.depth, theoretical_depth);
        
        // Node utilization (except root)
        if stats.total_nodes > 1 {
            assert!(stats.min_keys_per_node >= btree::MIN_KEYS,
                    "Node underutilized: {} keys (min: {})", 
                    stats.min_keys_per_node, btree::MIN_KEYS);
        }
    }
}

#[test]
fn test_tree_rebalancing_efficiency() {
    println!("\n=== Tree Rebalancing Efficiency Test ===");
    
    // Build a tree and then delete many keys to trigger rebalancing
    let mut tree = btree::create_empty_tree();
    let keys: Vec<_> = (0..5000)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
        
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(&format!("/rebalance/doc_{}.md", i)).unwrap();
        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
    }
    
    let initial_stats = analyze_tree_structure(&tree);
    println!("Initial tree: {} nodes, depth {}", initial_stats.total_nodes, initial_stats.depth);
    
    // Delete 80% of keys
    for key in keys.iter().take(4000) {
        tree = btree::delete_from_tree(tree, key).unwrap();
    }
    
    let final_stats = analyze_tree_structure(&tree);
    println!("After deletions: {} nodes, depth {}", final_stats.total_nodes, final_stats.depth);
    
    // Tree should shrink appropriately
    assert!(final_stats.total_nodes < initial_stats.total_nodes / 2,
            "Tree didn't shrink enough after deletions");
    assert!(final_stats.depth <= initial_stats.depth,
            "Tree depth increased after deletions");
    assert_eq!(final_stats.balance_factor, 1.0,
            "Tree lost balance after deletions");
}

#[test]
fn test_memory_overhead_analysis() {
    println!("\n=== Memory Overhead Analysis ===");
    
    // Measure overhead of tree structure vs raw data
    let size = 1000;
    
    // Measure raw data size
    let keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
    let paths: Vec<_> = (0..size)
        .map(|i| ValidatedPath::new(&format!("/overhead/doc_{}.md", i)).unwrap())
        .collect();
    
    let key_size = std::mem::size_of::<ValidatedDocumentId>();
    let path_size = std::mem::size_of::<ValidatedPath>() + 20; // Approximate string size
    let raw_data_size = size * (key_size + path_size);
    
    // Measure tree size
    reset_memory_tracking();
    let before = net_memory_used();
    
    let mut tree = btree::create_empty_tree();
    for i in 0..size {
        tree = btree::insert_into_tree(tree, keys[i].clone(), paths[i].clone()).unwrap();
    }
    
    let after = net_memory_used();
    let tree_size = (after - before) as usize;
    
    let overhead = tree_size as f64 / raw_data_size as f64;
    
    println!("Raw data size: {} bytes", raw_data_size);
    println!("Tree structure size: {} bytes", tree_size);
    println!("Overhead factor: {:.2}x", overhead);
    
    // B+ tree should have reasonable overhead (typically 1.5-3x)
    assert!(overhead < 3.0, "Tree overhead too high: {:.2}x", overhead);
}

#[test]
fn test_worst_case_memory_usage() {
    println!("\n=== Worst Case Memory Usage Test ===");
    
    // Test with pathological insertion order (sorted)
    let size = 1000;
    let mut keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
    keys.sort_by_key(|k| k.as_uuid());
    
    reset_memory_tracking();
    let before = net_memory_used();
    
    let mut tree = btree::create_empty_tree();
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(&format!("/worst/doc_{}.md", i)).unwrap();
        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
    }
    
    let after = net_memory_used();
    let sorted_memory = (after - before) as usize;
    
    // Compare with random insertion
    let mut random_keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();
    
    reset_memory_tracking();
    let before = net_memory_used();
    
    let mut tree = btree::create_empty_tree();
    for (i, key) in random_keys.iter().enumerate() {
        let path = ValidatedPath::new(&format!("/random/doc_{}.md", i)).unwrap();
        tree = btree::insert_into_tree(tree, key.clone(), path).unwrap();
    }
    
    let after = net_memory_used();
    let random_memory = (after - before) as usize;
    
    println!("Sorted insertion memory: {} bytes", sorted_memory);
    println!("Random insertion memory: {} bytes", random_memory);
    println!("Difference: {:.1}%", 
             (sorted_memory as f64 - random_memory as f64) / random_memory as f64 * 100.0);
    
    // Memory usage should be similar regardless of insertion order
    let ratio = sorted_memory as f64 / random_memory as f64;
    assert!(ratio < 1.2, "Sorted insertion uses too much memory: {:.2}x", ratio);
}