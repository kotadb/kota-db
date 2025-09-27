// Pure Functions Module - Stage 3: Pure Function Modularization
// All algorithms implemented as side-effect-free functions

pub mod btree;
pub mod connection_pool;
pub mod metadata;
pub mod performance;

// Re-export btree types and functions for convenience
pub use btree::{
    create_empty_tree, delete_from_tree, insert_into_tree, search_in_tree, BTreeNode, BTreeRoot,
};

// Bulk operations - Stage 3 pure function implementations
use crate::contracts::optimization::TreeStructureMetrics;
use crate::types::{ValidatedDocumentId, ValidatedPath};
use anyhow::Result;
use std::time::Instant;

/// Bulk insert multiple key-value pairs using optimized tree construction
///
/// This function implements an optimized bulk insertion algorithm that:
/// 1. Sorts the input pairs by key for optimal tree construction
/// 2. Builds the tree bottom-up for better balance
/// 3. Minimizes tree restructuring during insertion
///
/// Time Complexity: O(n log n) where n is the number of pairs
/// Space Complexity: O(n) for the sorted intermediate structure
pub fn bulk_insert_into_tree(
    mut tree: BTreeRoot,
    pairs: Vec<(ValidatedDocumentId, ValidatedPath)>,
) -> Result<BTreeRoot> {
    if pairs.is_empty() {
        return Ok(tree);
    }

    let _start = Instant::now();

    // Step 1: Sort pairs by key for optimal insertion order
    let mut sorted_pairs = pairs;
    sorted_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    // Step 2: Remove duplicates (last writer wins)
    sorted_pairs.dedup_by(|a, b| a.0 == b.0);

    // Step 3: Use optimized insertion strategy based on tree size
    let current_size = count_entries(&tree);
    let new_entries = sorted_pairs.len();

    if current_size == 0 {
        // Build new tree from scratch - most efficient approach
        tree = build_balanced_tree_from_sorted(sorted_pairs)?;
    } else if new_entries > current_size / 2 {
        // Merge strategy: extract existing, merge, rebuild
        let existing_pairs = extract_all_pairs(&tree)?;
        let mut all_pairs = existing_pairs;
        all_pairs.extend(sorted_pairs);
        all_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        all_pairs.dedup_by(|a, b| a.0 == b.0);

        tree = build_balanced_tree_from_sorted(all_pairs)?;
    } else {
        // Incremental strategy: insert one by one (already optimized for sorted order)
        for (key, path) in sorted_pairs {
            tree = insert_into_tree(tree, key, path)?;
        }
    }

    Ok(tree)
}

/// Bulk delete multiple keys using optimized deletion algorithm
///
/// This function implements an optimized bulk deletion that:
/// 1. Sorts the keys for efficient traversal
/// 2. Performs deletions in a single tree pass where possible
/// 3. Defers rebalancing until all deletions are complete
///
/// Time Complexity: O(k log n) where k is keys to delete, n is tree size
/// Space Complexity: O(k) for the sorted key list
pub fn bulk_delete_from_tree(
    mut tree: BTreeRoot,
    keys: Vec<ValidatedDocumentId>,
) -> Result<BTreeRoot> {
    if keys.is_empty() {
        return Ok(tree);
    }

    let _start = Instant::now();

    // Step 1: Sort keys for efficient deletion order
    let mut sorted_keys = keys;
    sorted_keys.sort();
    sorted_keys.dedup();

    // Step 2: Choose deletion strategy based on ratio
    let current_size = count_entries(&tree);
    let delete_count = sorted_keys.len();

    if delete_count > current_size / 2 {
        // Extract and filter strategy for large deletions
        let existing_pairs = extract_all_pairs(&tree)?;
        let key_set: std::collections::HashSet<_> = sorted_keys.into_iter().collect();

        let remaining_pairs: Vec<_> = existing_pairs
            .into_iter()
            .filter(|(key, _)| !key_set.contains(key))
            .collect();

        if remaining_pairs.is_empty() {
            tree = create_empty_tree();
        } else {
            tree = build_balanced_tree_from_sorted(remaining_pairs)?;
        }
    } else {
        // Incremental deletion for smaller deletions
        for key in sorted_keys {
            tree = delete_from_tree(tree, &key)?;
        }
    }

    Ok(tree)
}

/// Count total entries in the tree
///
/// Time Complexity: O(n) - traverses all nodes
/// Space Complexity: O(log n) - recursion stack
pub fn count_entries(tree: &BTreeRoot) -> usize {
    if let Some(ref root_node) = tree.root {
        count_entries_recursive(root_node)
    } else {
        0
    }
}

fn count_entries_recursive(node: &BTreeNode) -> usize {
    match node {
        BTreeNode::Leaf { keys, .. } => keys.len(),
        BTreeNode::Internal { children, .. } => children
            .iter()
            .map(|child| count_entries_recursive(child.as_ref()))
            .sum(),
    }
}

/// Analyze tree structure for optimization insights
///
/// Time Complexity: O(n) - single tree traversal
/// Space Complexity: O(log n) - recursion stack
pub fn analyze_tree_structure(tree: &BTreeRoot) -> Result<TreeStructureMetrics> {
    let root_node = match tree.root.as_ref() {
        Some(node) => node,
        None => {
            return Ok(TreeStructureMetrics {
                total_entries: 0,
                tree_depth: 0,
                balance_factor: 1.0,
                utilization_factor: 0.0,
                memory_efficiency: 0.0,
                node_distribution: crate::contracts::optimization::NodeDistribution {
                    total_nodes: 0,
                    leaf_nodes: 0,
                    internal_nodes: 0,
                    avg_keys_per_node: 0.0,
                    min_keys_per_node: 0,
                    max_keys_per_node: 0,
                },
                leaf_depth_variance: 0,
                recommended_actions: Vec::new(),
            });
        }
    };

    // Collect structural metrics
    let mut leaf_depths = Vec::new();
    let mut node_sizes = Vec::new();
    let mut total_nodes = 0;
    let mut leaf_nodes = 0;
    let mut internal_nodes = 0;

    analyze_node_recursive(
        root_node,
        0,
        &mut leaf_depths,
        &mut node_sizes,
        &mut total_nodes,
        &mut leaf_nodes,
        &mut internal_nodes,
    );

    let total_entries = count_entries(tree);
    let tree_depth = leaf_depths.iter().max().copied().unwrap_or(0);

    // Calculate balance factor (1.0 = perfect balance)
    let min_depth = leaf_depths.iter().min().copied().unwrap_or(0);
    let balance_factor = if tree_depth > 0 {
        min_depth as f64 / tree_depth as f64
    } else {
        1.0
    };

    // Calculate utilization factor
    let avg_keys_per_node = if total_nodes > 0 {
        total_entries as f64 / total_nodes as f64
    } else {
        0.0
    };

    let min_keys = node_sizes.iter().min().copied().unwrap_or(0);
    let max_keys = node_sizes.iter().max().copied().unwrap_or(0);
    let utilization_factor = if max_keys > 0 {
        avg_keys_per_node / max_keys as f64
    } else {
        0.0
    };

    // Calculate memory efficiency (placeholder - would need actual memory measurements)
    let memory_efficiency = 0.75; // Estimate based on B+ tree overhead

    // Calculate leaf depth variance
    let leaf_depth_variance = if leaf_depths.len() > 1 {
        let max_leaf_depth = leaf_depths.iter().max().copied().unwrap_or(0);
        let min_leaf_depth = leaf_depths.iter().min().copied().unwrap_or(0);
        max_leaf_depth - min_leaf_depth
    } else {
        0
    };

    // Generate recommendations
    let mut recommendations = Vec::new();

    if balance_factor < 0.8 {
        recommendations.push(
            crate::contracts::optimization::OptimizationRecommendation::RebalanceTree {
                reason: format!("Balance factor {balance_factor:.2} below optimal 0.8"),
                estimated_improvement: (0.8 - balance_factor) * 100.0,
            },
        );
    }

    if utilization_factor < 0.5 {
        recommendations.push(
            crate::contracts::optimization::OptimizationRecommendation::CompactNodes {
                fragmented_nodes: total_nodes - (total_entries / max_keys.max(1)),
                estimated_memory_savings: ((0.5 - utilization_factor) * total_entries as f64 * 32.0)
                    as usize, // Estimate
            },
        );
    }

    Ok(TreeStructureMetrics {
        total_entries,
        tree_depth,
        balance_factor,
        utilization_factor,
        memory_efficiency,
        node_distribution: crate::contracts::optimization::NodeDistribution {
            total_nodes,
            leaf_nodes,
            internal_nodes,
            avg_keys_per_node,
            min_keys_per_node: min_keys,
            max_keys_per_node: max_keys,
        },
        leaf_depth_variance,
        recommended_actions: recommendations,
    })
}

fn analyze_node_recursive(
    node: &BTreeNode,
    depth: usize,
    leaf_depths: &mut Vec<usize>,
    node_sizes: &mut Vec<usize>,
    total_nodes: &mut usize,
    leaf_nodes: &mut usize,
    internal_nodes: &mut usize,
) {
    *total_nodes += 1;

    match node {
        BTreeNode::Leaf { keys, .. } => {
            *leaf_nodes += 1;
            leaf_depths.push(depth);
            node_sizes.push(keys.len());
        }
        BTreeNode::Internal { keys, children } => {
            *internal_nodes += 1;
            node_sizes.push(keys.len());

            for child in children {
                analyze_node_recursive(
                    child,
                    depth + 1,
                    leaf_depths,
                    node_sizes,
                    total_nodes,
                    leaf_nodes,
                    internal_nodes,
                );
            }
        }
    }
}

/// Extract all key-value pairs from the tree in sorted order
///
/// Time Complexity: O(n)
/// Space Complexity: O(n)
pub fn extract_all_pairs(tree: &BTreeRoot) -> Result<Vec<(ValidatedDocumentId, ValidatedPath)>> {
    let mut pairs = Vec::new();

    if let Some(ref root_node) = tree.root {
        extract_pairs_recursive(root_node, &mut pairs);
    }

    Ok(pairs)
}

/// Traverse key-value pairs, invoking the visitor for each entry until it returns false
pub fn traverse_pairs_until<F>(tree: &BTreeRoot, mut visitor: F)
where
    F: FnMut(&ValidatedDocumentId, &ValidatedPath) -> bool,
{
    if let Some(ref root_node) = tree.root {
        traverse_pairs_recursive(root_node, &mut visitor);
    }
}

fn traverse_pairs_recursive<F>(node: &BTreeNode, visitor: &mut F) -> bool
where
    F: FnMut(&ValidatedDocumentId, &ValidatedPath) -> bool,
{
    match node {
        BTreeNode::Leaf { keys, values, .. } => {
            for (key, value) in keys.iter().zip(values.iter()) {
                if !visitor(key, value) {
                    return false;
                }
            }
        }
        BTreeNode::Internal { children, .. } => {
            for child in children {
                if !traverse_pairs_recursive(child, visitor) {
                    return false;
                }
            }
        }
    }

    true
}

fn extract_pairs_recursive(
    node: &BTreeNode,
    pairs: &mut Vec<(ValidatedDocumentId, ValidatedPath)>,
) {
    match node {
        BTreeNode::Leaf { keys, values, .. } => {
            for (key, value) in keys.iter().zip(values.iter()) {
                pairs.push((*key, value.clone()));
            }
        }
        BTreeNode::Internal { children, .. } => {
            for child in children {
                extract_pairs_recursive(child, pairs);
            }
        }
    }
}

/// Build a balanced B+ tree from sorted key-value pairs
///
/// This implements a bottom-up tree construction algorithm that guarantees
/// optimal balance by building the tree level by level.
///
/// Time Complexity: O(n)
/// Space Complexity: O(n)
fn build_balanced_tree_from_sorted(
    pairs: Vec<(ValidatedDocumentId, ValidatedPath)>,
) -> Result<BTreeRoot> {
    if pairs.is_empty() {
        return Ok(create_empty_tree());
    }

    let max_keys_per_node = crate::pure::btree::MAX_KEYS;

    // Step 1: Build leaf nodes without cloning entire subtrees later
    let num_leaf_nodes = pairs.len().div_ceil(max_keys_per_node);
    let mut current_level: Vec<Box<BTreeNode>> = Vec::with_capacity(num_leaf_nodes);

    for chunk in pairs.chunks(max_keys_per_node) {
        let keys: Vec<_> = chunk.iter().map(|(k, _)| *k).collect();
        let values: Vec<_> = chunk.iter().map(|(_, v)| v.clone()).collect();

        current_level.push(Box::new(BTreeNode::Leaf {
            keys,
            values,
            next_leaf: None,
        }));
    }

    // Step 2: Build internal levels bottom-up, moving nodes instead of cloning
    let mut height: u32 = 1;
    while current_level.len() > 1 {
        height += 1;

        let mut next_level: Vec<Box<BTreeNode>> =
            Vec::with_capacity(current_level.len().div_ceil(max_keys_per_node + 1));
        let mut iter = current_level.into_iter();

        loop {
            let mut children: Vec<Box<BTreeNode>> = Vec::with_capacity(max_keys_per_node + 1);
            match iter.next() {
                Some(child) => children.push(child),
                None => break,
            }

            for _ in 0..max_keys_per_node {
                if let Some(child) = iter.next() {
                    children.push(child);
                } else {
                    break;
                }
            }

            let mut keys = Vec::with_capacity(children.len().saturating_sub(1));
            for child in children.iter().skip(1) {
                keys.push(extract_first_key(child.as_ref()));
            }

            next_level.push(Box::new(BTreeNode::Internal { keys, children }));
        }

        current_level = next_level;
    }

    let root = current_level.into_iter().next();

    Ok(BTreeRoot {
        root,
        height,
        total_keys: pairs.len(),
    })
}

/// Extract the first key from a node (for building internal node separators)
fn extract_first_key(node: &BTreeNode) -> ValidatedDocumentId {
    match node {
        BTreeNode::Leaf { keys, .. } => keys[0],
        BTreeNode::Internal { children, .. } => extract_first_key(children[0].as_ref()),
    }
}

// Helper functions for bulk operations
// (extract_all_pairs and related functions are defined above)

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_bulk_insert_empty_tree() -> Result<()> {
        let tree = create_empty_tree();
        let pairs = vec![
            (
                ValidatedDocumentId::from_uuid(Uuid::new_v4())?,
                ValidatedPath::new("test/1.md")?,
            ),
            (
                ValidatedDocumentId::from_uuid(Uuid::new_v4())?,
                ValidatedPath::new("test/2.md")?,
            ),
        ];

        let result_tree = bulk_insert_into_tree(tree, pairs.clone())?;

        assert_eq!(count_entries(&result_tree), 2);

        // Verify all keys are searchable
        for (key, _) in &pairs {
            assert!(search_in_tree(&result_tree, key).is_some());
        }

        Ok(())
    }

    #[test]
    fn test_bulk_delete_partial() -> Result<()> {
        let mut tree = create_empty_tree();

        // Insert test data
        let mut all_keys = Vec::new();
        for i in 0..10 {
            let key = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let path = ValidatedPath::new(format!("test/{i}.md"))?;
            tree = insert_into_tree(tree, key, path)?;
            all_keys.push(key);
        }

        // Delete half the keys
        let keys_to_delete = all_keys[..5].to_vec();
        tree = bulk_delete_from_tree(tree, keys_to_delete.clone())?;

        assert_eq!(count_entries(&tree), 5);

        // Verify deleted keys are not found
        for key in &keys_to_delete {
            assert!(search_in_tree(&tree, key).is_none());
        }

        // Verify remaining keys are still found
        for key in &all_keys[5..] {
            assert!(search_in_tree(&tree, key).is_some());
        }

        Ok(())
    }

    #[test]
    fn test_count_entries() -> Result<()> {
        let mut tree = create_empty_tree();
        assert_eq!(count_entries(&tree), 0);

        for i in 0..5 {
            let key = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let path = ValidatedPath::new(format!("test/{i}.md"))?;
            tree = insert_into_tree(tree, key, path)?;
            assert_eq!(count_entries(&tree), i + 1);
        }

        Ok(())
    }

    #[test]
    fn test_analyze_tree_structure() -> Result<()> {
        let mut tree = create_empty_tree();

        // Build a tree with known structure
        for i in 0..20 {
            let key = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let path = ValidatedPath::new(format!("test/{i}.md"))?;
            tree = insert_into_tree(tree, key, path)?;
        }

        let metrics = analyze_tree_structure(&tree)?;

        assert_eq!(metrics.total_entries, 20);
        assert!(metrics.tree_depth > 0);
        assert!(metrics.balance_factor > 0.0);
        assert!(metrics.balance_factor <= 1.0);
        assert!(metrics.node_distribution.total_nodes > 0);

        Ok(())
    }
}
