// B+ Tree Implementation - Pure Functions
// This module contains the B+ tree data structures and algorithms
// All functions are pure (no side effects, deterministic)

use crate::types::{ValidatedDocumentId, ValidatedPath};
use anyhow::{bail, Result};
use std::cmp::Ordering;

/// B+ Tree node types
#[derive(Debug, Clone, PartialEq)]
pub enum BTreeNode {
    Leaf {
        keys: Vec<ValidatedDocumentId>,
        values: Vec<ValidatedPath>,
        next_leaf: Option<Box<BTreeNode>>,
    },
    Internal {
        keys: Vec<ValidatedDocumentId>,
        children: Vec<Box<BTreeNode>>,
    },
}

/// B+ Tree configuration constants
pub const MIN_DEGREE: usize = 3; // Minimum degree (t)
pub const MAX_KEYS: usize = 2 * MIN_DEGREE - 1; // Maximum keys per node (2t-1)
pub const MIN_KEYS: usize = MIN_DEGREE - 1; // Minimum keys per non-root node (t-1)

/// Tree root wrapper
#[derive(Debug, Clone)]
pub struct BTreeRoot {
    pub root: Option<Box<BTreeNode>>,
    pub height: u32,
    pub total_keys: usize,
}

impl BTreeNode {
    /// Check if node is a leaf
    pub fn is_leaf(&self) -> bool {
        matches!(self, BTreeNode::Leaf { .. })
    }

    /// Get number of keys in node
    pub fn key_count(&self) -> usize {
        match self {
            BTreeNode::Leaf { keys, .. } => keys.len(),
            BTreeNode::Internal { keys, .. } => keys.len(),
        }
    }

    /// Check if node needs splitting
    pub fn needs_split(&self) -> bool {
        self.key_count() > MAX_KEYS
    }

    /// Get keys as slice for iteration
    pub fn keys(&self) -> &[ValidatedDocumentId] {
        match self {
            BTreeNode::Leaf { keys, .. } => keys,
            BTreeNode::Internal { keys, .. } => keys,
        }
    }
}

/// Create an empty B+ tree
pub fn create_empty_tree() -> BTreeRoot {
    BTreeRoot {
        root: None,
        height: 0,
        total_keys: 0,
    }
}

/// Create a new leaf node
pub fn create_leaf_node() -> BTreeNode {
    BTreeNode::Leaf {
        keys: Vec::with_capacity(MAX_KEYS + 1),
        values: Vec::with_capacity(MAX_KEYS + 1),
        next_leaf: None,
    }
}

/// Create a new internal node
pub fn create_internal_node() -> BTreeNode {
    BTreeNode::Internal {
        keys: Vec::with_capacity(MAX_KEYS + 1),
        children: Vec::with_capacity(MAX_KEYS + 2),
    }
}

/// Binary search for key position in node
pub fn find_key_position(node: &BTreeNode, key: &ValidatedDocumentId) -> Result<usize, usize> {
    node.keys().binary_search(key)
}

/// Search for a key in a single node (leaf only)
pub fn search_in_node<'a>(
    node: &'a BTreeNode,
    key: &ValidatedDocumentId,
) -> Option<(usize, &'a ValidatedPath)> {
    if let BTreeNode::Leaf { keys, values, .. } = node {
        if let Ok(index) = keys.binary_search(key) {
            return Some((index, &values[index]));
        }
    }
    None
}

/// Insert key-value pair into a leaf node (pure function)
pub fn insert_key_value_in_leaf(
    mut node: BTreeNode,
    key: ValidatedDocumentId,
    value: ValidatedPath,
) -> Result<BTreeNode> {
    if let BTreeNode::Leaf {
        ref mut keys,
        ref mut values,
        ..
    } = node
    {
        match keys.binary_search(&key) {
            Ok(index) => {
                // Key exists, update value
                values[index] = value;
            }
            Err(index) => {
                // Key doesn't exist, insert at correct position
                keys.insert(index, key);
                values.insert(index, value);
            }
        }
        Ok(node)
    } else {
        bail!("Cannot insert key-value into non-leaf node")
    }
}

/// Split a full leaf node into two nodes
pub fn split_leaf_node(node: BTreeNode) -> Result<(BTreeNode, ValidatedDocumentId, BTreeNode)> {
    if let BTreeNode::Leaf {
        keys,
        values,
        next_leaf,
    } = node
    {
        if keys.len() <= MAX_KEYS {
            bail!("Node does not need splitting");
        }

        let mid_index = keys.len() / 2;

        // Split keys and values
        let mut left_keys = keys;
        let mut left_values = values;

        let right_keys = left_keys.split_off(mid_index);
        let right_values = left_values.split_off(mid_index);

        // The median key (first key of right node)
        let median_key = right_keys[0].clone();

        // Create new nodes
        let left_node = BTreeNode::Leaf {
            keys: left_keys,
            values: left_values,
            next_leaf: None, // Will be updated when inserted into tree
        };

        let right_node = BTreeNode::Leaf {
            keys: right_keys,
            values: right_values,
            next_leaf,
        };

        Ok((left_node, median_key, right_node))
    } else {
        bail!("Cannot split non-leaf node with leaf split function")
    }
}

/// Split a full internal node
pub fn split_internal_node(node: BTreeNode) -> Result<(BTreeNode, ValidatedDocumentId, BTreeNode)> {
    if let BTreeNode::Internal { keys, children } = node {
        if keys.len() <= MAX_KEYS {
            bail!("Node does not need splitting");
        }

        let mid_index = keys.len() / 2;

        // Split keys and children
        let mut left_keys = keys;
        let mut left_children = children;

        let right_keys = left_keys.split_off(mid_index + 1);
        let right_children = left_children.split_off(mid_index + 1);

        // The median key moves up to parent
        let median_key = left_keys.pop().unwrap();

        // Create new nodes
        let left_node = BTreeNode::Internal {
            keys: left_keys,
            children: left_children,
        };

        let right_node = BTreeNode::Internal {
            keys: right_keys,
            children: right_children,
        };

        Ok((left_node, median_key, right_node))
    } else {
        bail!("Cannot split leaf node with internal split function")
    }
}

/// Search for a value in the entire tree
pub fn search_in_tree(root: &BTreeRoot, key: &ValidatedDocumentId) -> Option<ValidatedPath> {
    let mut current = root.root.as_ref()?;

    loop {
        match current.as_ref() {
            BTreeNode::Leaf { keys, values, .. } => {
                // Search in leaf
                if let Ok(index) = keys.binary_search(key) {
                    return Some(values[index].clone());
                }
                return None;
            }
            BTreeNode::Internal { keys, children } => {
                // Find child to search
                let child_index = match keys.binary_search(key) {
                    Ok(index) => index + 1, // Key found, go to right child
                    Err(index) => index,    // Key not found, go to appropriate child
                };

                if child_index >= children.len() {
                    return None;
                }

                current = &children[child_index];
            }
        }
    }
}

/// Insert a key-value pair into the tree
pub fn insert_into_tree(
    mut root: BTreeRoot,
    key: ValidatedDocumentId,
    value: ValidatedPath,
) -> Result<BTreeRoot> {
    // Special case: empty tree
    if root.root.is_none() {
        let mut leaf = create_leaf_node();
        leaf = insert_key_value_in_leaf(leaf, key, value)?;
        root.root = Some(Box::new(leaf));
        root.height = 1;
        root.total_keys = 1;
        return Ok(root);
    }

    // Check if key already exists
    let exists = search_in_tree(&root, &key).is_some();

    // Insert into tree
    let root_node = root
        .root
        .take()
        .ok_or_else(|| anyhow::anyhow!("Root node is missing"))?;
    let (new_root, needs_new_root) = insert_recursive(root_node, key, value)?;

    if let Some((left_child, median_key, right_child)) = needs_new_root {
        // Root split, create new root
        let mut new_internal_root = create_internal_node();
        if let BTreeNode::Internal {
            ref mut keys,
            ref mut children,
        } = new_internal_root
        {
            keys.push(median_key);
            children.push(left_child);
            children.push(right_child);
        }
        root.root = Some(Box::new(new_internal_root));
        root.height += 1;
    } else {
        root.root = Some(new_root);
    }

    if !exists {
        root.total_keys += 1;
    }

    Ok(root)
}

/// Recursive helper for insertion
fn insert_recursive(
    mut node: Box<BTreeNode>,
    key: ValidatedDocumentId,
    value: ValidatedPath,
) -> Result<(
    Box<BTreeNode>,
    Option<(Box<BTreeNode>, ValidatedDocumentId, Box<BTreeNode>)>,
)> {
    match node.as_mut() {
        BTreeNode::Leaf { .. } => {
            // Insert into leaf
            *node = insert_key_value_in_leaf(*node, key, value)?;

            // Check if split needed
            if node.needs_split() {
                let (left, median, right) = split_leaf_node(*node)?;
                let left_clone = left.clone();
                Ok((
                    Box::new(left_clone),
                    Some((Box::new(left), median, Box::new(right))),
                ))
            } else {
                Ok((node, None))
            }
        }
        BTreeNode::Internal { keys, children } => {
            // Find child to insert into
            let child_index = match keys.binary_search(&key) {
                Ok(index) => index + 1,
                Err(index) => index,
            };

            // Recursively insert into child
            let child = children.remove(child_index);
            let (new_child, split_info) = insert_recursive(child, key.clone(), value)?;

            // Handle child split
            if let Some((left_child, median_key, right_child)) = split_info {
                keys.insert(child_index, median_key);
                children.insert(child_index, left_child);
                children.insert(child_index + 1, right_child);

                // Check if this node needs splitting
                if keys.len() > MAX_KEYS {
                    let (left, median, right) = split_internal_node(*node)?;
                    let left_clone = left.clone();
                    Ok((
                        Box::new(left_clone),
                        Some((Box::new(left), median, Box::new(right))),
                    ))
                } else {
                    Ok((node, None))
                }
            } else {
                children.insert(child_index, new_child);
                Ok((node, None))
            }
        }
    }
}

/// Delete a key from the tree
pub fn delete_from_tree(mut root: BTreeRoot, key: &ValidatedDocumentId) -> Result<BTreeRoot> {
    // Check if key exists
    if search_in_tree(&root, key).is_none() {
        return Ok(root); // Key doesn't exist, nothing to delete
    }

    // Delete from tree
    let root_node = root
        .root
        .take()
        .ok_or_else(|| anyhow::anyhow!("Root node is missing during deletion"))?;
    let new_root = delete_recursive(root_node, key)?;

    // Handle empty root
    if let Some(new_root_node) = new_root {
        // Check if root is now an empty internal node with one child
        if let BTreeNode::Internal { keys, children } = new_root_node.as_ref() {
            if keys.is_empty() && children.len() == 1 {
                // Collapse tree height
                root.root = Some(children[0].clone());
                root.height = root.height.saturating_sub(1);
            } else {
                root.root = Some(new_root_node);
            }
        } else {
            root.root = Some(new_root_node);
        }
    } else {
        // Tree is now empty
        root.root = None;
        root.height = 0;
    }

    root.total_keys = root.total_keys.saturating_sub(1);
    Ok(root)
}

/// Recursive helper for deletion
fn delete_recursive(
    mut node: Box<BTreeNode>,
    key: &ValidatedDocumentId,
) -> Result<Option<Box<BTreeNode>>> {
    match node.as_mut() {
        BTreeNode::Leaf { keys, values, .. } => {
            // Delete from leaf
            if let Ok(index) = keys.binary_search(key) {
                keys.remove(index);
                values.remove(index);
            }

            // Check if node is now underfull (but root can have 0 keys)
            if keys.is_empty() {
                Ok(None) // Leaf is empty
            } else {
                Ok(Some(node))
            }
        }
        BTreeNode::Internal { keys, children } => {
            // Find child that contains the key
            let child_index = match keys.binary_search(key) {
                Ok(index) => index + 1, // Key might be in right subtree
                Err(index) => index,    // Key is in this subtree
            };

            // Recursively delete from child
            let child = children.remove(child_index);
            let new_child = delete_recursive(child, key)?;

            // Reinsert child if it still exists
            if let Some(new_child_node) = new_child {
                children.insert(child_index, new_child_node);

                // Check if rebalancing is needed
                if child_index < children.len() {
                    let child_keys = children[child_index].key_count();
                    if child_keys < MIN_KEYS {
                        rebalance_after_deletion(keys, children, child_index)?;
                    }
                }
            } else {
                // Child was deleted entirely, adjust keys
                if child_index > 0 && child_index <= keys.len() {
                    keys.remove(child_index - 1);
                }
            }

            // Return the node (might be underfull, handled by parent)
            Ok(Some(node))
        }
    }
}

/// Rebalance tree after deletion
fn rebalance_after_deletion(
    keys: &mut Vec<ValidatedDocumentId>,
    children: &mut Vec<Box<BTreeNode>>,
    child_index: usize,
) -> Result<()> {
    let child_keys = children[child_index].key_count();

    // Try to borrow from left sibling
    if child_index > 0 {
        let left_sibling_keys = children[child_index - 1].key_count();
        if left_sibling_keys > MIN_KEYS {
            borrow_from_left_sibling(keys, children, child_index)?;
            return Ok(());
        }
    }

    // Try to borrow from right sibling
    if child_index < children.len() - 1 {
        let right_sibling_keys = children[child_index + 1].key_count();
        if right_sibling_keys > MIN_KEYS {
            borrow_from_right_sibling(keys, children, child_index)?;
            return Ok(());
        }
    }

    // Merge with a sibling
    if child_index > 0 {
        merge_with_left_sibling(keys, children, child_index)?;
    } else if child_index < children.len() - 1 {
        merge_with_right_sibling(keys, children, child_index)?;
    }

    Ok(())
}

/// Borrow a key from left sibling
fn borrow_from_left_sibling(
    parent_keys: &mut Vec<ValidatedDocumentId>,
    children: &mut Vec<Box<BTreeNode>>,
    child_index: usize,
) -> Result<()> {
    let separator_index = child_index - 1;
    let separator_key = parent_keys[separator_index].clone();

    let (left_children, right_children) = children.split_at_mut(child_index);
    let left_child = left_children
        .last_mut()
        .ok_or_else(|| anyhow::anyhow!("No left sibling available for borrowing"))?;
    let right_child = right_children
        .first_mut()
        .ok_or_else(|| anyhow::anyhow!("No right child available for borrowing"))?;

    match (left_child.as_mut(), right_child.as_mut()) {
        (
            BTreeNode::Leaf {
                keys: left_keys,
                values: left_values,
                ..
            },
            BTreeNode::Leaf {
                keys: right_keys,
                values: right_values,
                ..
            },
        ) => {
            // Move last key-value from left sibling to beginning of right sibling
            let borrowed_key = left_keys
                .pop()
                .ok_or_else(|| anyhow::anyhow!("Left sibling has no keys to borrow"))?;
            let borrowed_value = left_values
                .pop()
                .ok_or_else(|| anyhow::anyhow!("Left sibling has no values to borrow"))?;

            right_keys.insert(0, borrowed_key.clone());
            right_values.insert(0, borrowed_value);

            // Update parent separator
            parent_keys[separator_index] = borrowed_key;
        }
        (
            BTreeNode::Internal {
                keys: left_keys,
                children: left_children,
            },
            BTreeNode::Internal {
                keys: right_keys,
                children: right_children,
            },
        ) => {
            // Move separator down to right node
            right_keys.insert(0, separator_key);

            // Move last key from left sibling up to parent
            let new_separator = left_keys
                .pop()
                .ok_or_else(|| anyhow::anyhow!("Left sibling has no keys to promote"))?;
            parent_keys[separator_index] = new_separator;

            // Move last child from left sibling to right sibling
            let borrowed_child = left_children
                .pop()
                .ok_or_else(|| anyhow::anyhow!("Left sibling has no children to borrow"))?;
            right_children.insert(0, borrowed_child);
        }
        _ => bail!("Sibling nodes must be of same type"),
    }

    Ok(())
}

/// Borrow a key from right sibling
fn borrow_from_right_sibling(
    parent_keys: &mut Vec<ValidatedDocumentId>,
    children: &mut Vec<Box<BTreeNode>>,
    child_index: usize,
) -> Result<()> {
    let separator_index = child_index;
    let separator_key = parent_keys[separator_index].clone();

    let (left_children, right_children) = children.split_at_mut(child_index + 1);
    let left_child = left_children
        .last_mut()
        .ok_or_else(|| anyhow::anyhow!("No left child available for right borrowing"))?;
    let right_child = right_children
        .first_mut()
        .ok_or_else(|| anyhow::anyhow!("No right sibling available for borrowing"))?;

    match (left_child.as_mut(), right_child.as_mut()) {
        (
            BTreeNode::Leaf {
                keys: left_keys,
                values: left_values,
                ..
            },
            BTreeNode::Leaf {
                keys: right_keys,
                values: right_values,
                ..
            },
        ) => {
            // Move first key-value from right sibling to end of left sibling
            let borrowed_key = right_keys.remove(0);
            let borrowed_value = right_values.remove(0);

            left_keys.push(borrowed_key);
            left_values.push(borrowed_value);

            // Update parent separator
            parent_keys[separator_index] = right_keys[0].clone();
        }
        (
            BTreeNode::Internal {
                keys: left_keys,
                children: left_children,
            },
            BTreeNode::Internal {
                keys: right_keys,
                children: right_children,
            },
        ) => {
            // Move separator down to left node
            left_keys.push(separator_key);

            // Move first key from right sibling up to parent
            let new_separator = right_keys.remove(0);
            parent_keys[separator_index] = new_separator;

            // Move first child from right sibling to left sibling
            let borrowed_child = right_children.remove(0);
            left_children.push(borrowed_child);
        }
        _ => bail!("Sibling nodes must be of same type"),
    }

    Ok(())
}

/// Merge node with left sibling
fn merge_with_left_sibling(
    parent_keys: &mut Vec<ValidatedDocumentId>,
    children: &mut Vec<Box<BTreeNode>>,
    child_index: usize,
) -> Result<()> {
    let separator_index = child_index - 1;
    let separator_key = parent_keys.remove(separator_index);

    let right_node = children.remove(child_index);
    let left_node = &mut children[child_index - 1];

    match (left_node.as_mut(), right_node.as_ref()) {
        (
            BTreeNode::Leaf {
                keys: left_keys,
                values: left_values,
                next_leaf,
            },
            BTreeNode::Leaf {
                keys: right_keys,
                values: right_values,
                next_leaf: right_next,
            },
        ) => {
            // Merge right node into left node
            left_keys.extend(right_keys.iter().cloned());
            left_values.extend(right_values.iter().cloned());
            *next_leaf = right_next.clone();
        }
        (
            BTreeNode::Internal {
                keys: left_keys,
                children: left_children,
            },
            BTreeNode::Internal {
                keys: right_keys,
                children: right_children,
            },
        ) => {
            // Add separator key
            left_keys.push(separator_key);

            // Merge right node into left node
            left_keys.extend(right_keys.iter().cloned());
            left_children.extend(right_children.iter().cloned());
        }
        _ => bail!("Cannot merge nodes of different types"),
    }

    Ok(())
}

/// Merge node with right sibling
fn merge_with_right_sibling(
    parent_keys: &mut Vec<ValidatedDocumentId>,
    children: &mut Vec<Box<BTreeNode>>,
    child_index: usize,
) -> Result<()> {
    let separator_index = child_index;
    let separator_key = parent_keys.remove(separator_index);

    let left_node = children.remove(child_index);
    let right_node = &mut children[child_index]; // After removal, this is the old child_index + 1

    match (left_node.as_ref(), right_node.as_mut()) {
        (
            BTreeNode::Leaf {
                keys: left_keys,
                values: left_values,
                ..
            },
            BTreeNode::Leaf {
                keys: right_keys,
                values: right_values,
                ..
            },
        ) => {
            // Prepend left node to right node
            let mut new_keys = left_keys.clone();
            new_keys.extend(right_keys.iter().cloned());
            *right_keys = new_keys;

            let mut new_values = left_values.clone();
            new_values.extend(right_values.iter().cloned());
            *right_values = new_values;
        }
        (
            BTreeNode::Internal {
                keys: left_keys,
                children: left_children,
            },
            BTreeNode::Internal {
                keys: right_keys,
                children: right_children,
            },
        ) => {
            // Prepend left node to right node with separator
            let mut new_keys = left_keys.clone();
            new_keys.push(separator_key);
            new_keys.extend(right_keys.iter().cloned());
            *right_keys = new_keys;

            let mut new_children = left_children.clone();
            new_children.extend(right_children.iter().cloned());
            *right_children = new_children;
        }
        _ => bail!("Cannot merge nodes of different types"),
    }

    Ok(())
}

/// Count total keys in tree (for testing)
pub fn count_total_keys(root: &BTreeRoot) -> usize {
    root.total_keys
}

/// Check if tree maintains B+ tree invariants (for testing)
pub fn is_valid_btree(root: &BTreeRoot) -> bool {
    if let Some(ref root_node) = root.root {
        check_btree_invariants(root_node.as_ref(), true).is_ok()
    } else {
        true // Empty tree is valid
    }
}

/// Check B+ tree invariants recursively
fn check_btree_invariants(node: &BTreeNode, is_root: bool) -> Result<()> {
    let key_count = node.key_count();

    // Check key count constraints
    if !is_root && key_count < MIN_KEYS {
        bail!("Non-root node has too few keys: {}", key_count);
    }
    if key_count > MAX_KEYS {
        bail!("Node has too many keys: {}", key_count);
    }

    // Check keys are sorted
    let keys = node.keys();
    for i in 1..keys.len() {
        if keys[i - 1] >= keys[i] {
            bail!("Keys not in sorted order");
        }
    }

    // Check node-specific invariants
    match node {
        BTreeNode::Leaf { keys, values, .. } => {
            if keys.len() != values.len() {
                bail!("Leaf node key-value count mismatch");
            }
        }
        BTreeNode::Internal { keys, children } => {
            if children.len() != keys.len() + 1 {
                bail!("Internal node key-child count mismatch");
            }

            // Recursively check children
            for child in children {
                check_btree_invariants(child.as_ref(), false)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_empty_tree_creation() {
        let tree = create_empty_tree();
        assert!(tree.root.is_none());
        assert_eq!(tree.height, 0);
        assert_eq!(tree.total_keys, 0);
    }

    #[test]
    fn test_single_insertion() -> Result<()> {
        let mut tree = create_empty_tree();
        let key = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let value = ValidatedPath::new("/test.md")?;

        tree = insert_into_tree(tree, key.clone(), value.clone())?;

        assert_eq!(tree.total_keys, 1);
        assert_eq!(search_in_tree(&tree, &key), Some(value));
        Ok(())
    }

    #[test]
    fn test_multiple_insertions() -> Result<()> {
        let mut tree = create_empty_tree();
        let mut keys = Vec::new();

        for i in 0..10 {
            let key = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
            let value = ValidatedPath::new(&format!("/test{}.md", i))?;
            keys.push((key.clone(), value.clone()));
            tree = insert_into_tree(tree, key, value)?;
        }

        assert_eq!(tree.total_keys, 10);

        // Verify all keys can be found
        for (key, value) in &keys {
            assert_eq!(search_in_tree(&tree, key), Some(value.clone()));
        }

        Ok(())
    }
}
