// Pure Functions - Stage 3: Pure Function Modularization
// This module contains all pure functions (no side effects, deterministic)
// These functions form the core business logic and can be tested in isolation

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Trigram extraction from text - pure, no I/O
pub mod trigram {
    use super::*;
    
    /// Extract trigrams from text
    /// Returns a vector of 3-byte arrays representing trigrams
    pub fn extract_trigrams(text: &str) -> Vec<[u8; 3]> {
        if text.len() < 3 {
            return Vec::new();
        }
        
        let bytes = text.as_bytes();
        let mut trigrams = Vec::with_capacity(bytes.len().saturating_sub(2));
        
        for i in 0..bytes.len().saturating_sub(2) {
            let trigram: [u8; 3] = [bytes[i], bytes[i + 1], bytes[i + 2]];
            trigrams.push(trigram);
        }
        
        trigrams
    }
    
    /// Extract unique trigrams with positions
    pub fn extract_trigrams_with_positions(text: &str) -> HashMap<[u8; 3], Vec<u32>> {
        let mut trigram_positions: HashMap<[u8; 3], Vec<u32>> = HashMap::new();
        
        if text.len() < 3 {
            return trigram_positions;
        }
        
        let bytes = text.as_bytes();
        for i in 0..bytes.len().saturating_sub(2) {
            let trigram: [u8; 3] = [bytes[i], bytes[i + 1], bytes[i + 2]];
            trigram_positions
                .entry(trigram)
                .or_insert_with(Vec::new)
                .push(i as u32);
        }
        
        trigram_positions
    }
    
    /// Calculate Jaccard similarity between two sets of trigrams
    pub fn jaccard_similarity(trigrams1: &[[u8; 3]], trigrams2: &[[u8; 3]]) -> f32 {
        if trigrams1.is_empty() && trigrams2.is_empty() {
            return 1.0;
        }
        
        let set1: HashSet<_> = trigrams1.iter().collect();
        let set2: HashSet<_> = trigrams2.iter().collect();
        
        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
    
    /// Calculate edit distance between two strings (Levenshtein)
    pub fn edit_distance(s1: &str, s2: &str) -> u32 {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();
        
        let mut dp = vec![vec![0u32; len2 + 1]; len1 + 1];
        
        // Initialize base cases
        for i in 0..=len1 {
            dp[i][0] = i as u32;
        }
        for j in 0..=len2 {
            dp[0][j] = j as u32;
        }
        
        // Fill the matrix
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                dp[i][j] = std::cmp::min(
                    dp[i - 1][j] + 1,           // deletion
                    std::cmp::min(
                        dp[i][j - 1] + 1,       // insertion
                        dp[i - 1][j - 1] + cost // substitution
                    )
                );
            }
        }
        
        dp[len1][len2]
    }
}

/// Text scoring and relevance - pure functions
pub mod scoring {
    use super::*;
    
    /// Calculate TF-IDF score
    pub fn calculate_tf_idf(
        term_frequency: f32,
        document_frequency: usize,
        total_documents: usize,
    ) -> f32 {
        if document_frequency == 0 || total_documents == 0 {
            return 0.0;
        }
        
        let idf = (total_documents as f32 / document_frequency as f32).ln();
        term_frequency * idf
    }
    
    /// Calculate BM25 score (better than TF-IDF for text search)
    pub fn calculate_bm25(
        term_frequency: f32,
        document_length: usize,
        average_document_length: f32,
        document_frequency: usize,
        total_documents: usize,
        k1: f32, // typically 1.2
        b: f32,  // typically 0.75
    ) -> f32 {
        if document_frequency == 0 || total_documents == 0 {
            return 0.0;
        }
        
        let idf = ((total_documents - document_frequency) as f32 + 0.5) /
                  (document_frequency as f32 + 0.5);
        let idf = idf.ln();
        
        let dl_norm = document_length as f32 / average_document_length;
        let tf_component = (term_frequency * (k1 + 1.0)) /
                          (term_frequency + k1 * (1.0 - b + b * dl_norm));
        
        idf * tf_component
    }
    
    /// Score a document based on query match
    #[derive(Debug, Clone)]
    pub struct DocumentScore {
        pub doc_id: Uuid,
        pub text_score: f32,
        pub tag_score: f32,
        pub recency_score: f32,
        pub total_score: f32,
    }
    
    /// Calculate composite relevance score
    pub fn calculate_relevance_score(
        text_score: f32,
        tag_score: f32,
        recency_score: f32,
        weights: (f32, f32, f32), // (text_weight, tag_weight, recency_weight)
    ) -> f32 {
        let (text_w, tag_w, recency_w) = weights;
        text_score * text_w + tag_score * tag_w + recency_score * recency_w
    }
    
    /// Calculate recency score (exponential decay)
    pub fn calculate_recency_score(
        document_timestamp: i64,
        current_timestamp: i64,
        half_life_days: f32,
    ) -> f32 {
        let age_days = (current_timestamp - document_timestamp) as f32 / 86400.0;
        0.5_f32.powf(age_days / half_life_days)
    }
}

/// Metadata parsing - pure functions
pub mod metadata {
    use super::*;
    
    /// Parse YAML frontmatter from markdown content
    pub fn parse_frontmatter(content: &str) -> Option<HashMap<String, serde_yaml::Value>> {
        if !content.starts_with("---\n") {
            return None;
        }
        
        let end_marker = content[4..].find("\n---\n")?;
        let yaml_content = &content[4..end_marker + 4];
        
        serde_yaml::from_str(yaml_content).ok()
    }
    
    /// Extract tags from frontmatter
    pub fn extract_tags(frontmatter: &HashMap<String, serde_yaml::Value>) -> Vec<String> {
        if let Some(serde_yaml::Value::Sequence(tags)) = frontmatter.get("tags") {
            tags.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        } else if let Some(serde_yaml::Value::String(tag)) = frontmatter.get("tags") {
            vec![tag.clone()]
        } else {
            Vec::new()
        }
    }
    
    /// Extract related documents from frontmatter
    pub fn extract_related(frontmatter: &HashMap<String, serde_yaml::Value>) -> Vec<String> {
        if let Some(serde_yaml::Value::Sequence(related)) = frontmatter.get("related") {
            related.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Calculate content hash (SHA-256)
    pub fn calculate_hash(content: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content);
        hasher.finalize().into()
    }
}

/// Graph algorithms - pure functions
pub mod graph {
    use super::*;
    
    /// Find all nodes reachable within N hops
    pub fn find_reachable(
        start: Uuid,
        edges: &HashMap<Uuid, Vec<Uuid>>,
        max_depth: u32,
    ) -> HashSet<Uuid> {
        let mut visited = HashSet::new();
        let mut current_level = vec![start];
        
        for _depth in 0..max_depth {
            if current_level.is_empty() {
                break;
            }
            
            let mut next_level = Vec::new();
            
            for node in current_level {
                if visited.insert(node) {
                    if let Some(neighbors) = edges.get(&node) {
                        for neighbor in neighbors {
                            if !visited.contains(neighbor) {
                                next_level.push(*neighbor);
                            }
                        }
                    }
                }
            }
            
            current_level = next_level;
        }
        
        visited.remove(&start); // Don't include start node
        visited
    }
    
    /// Detect cycles in graph
    pub fn has_cycle(edges: &HashMap<Uuid, Vec<Uuid>>) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        fn dfs(
            node: &Uuid,
            edges: &HashMap<Uuid, Vec<Uuid>>,
            visited: &mut HashSet<Uuid>,
            rec_stack: &mut HashSet<Uuid>,
        ) -> bool {
            visited.insert(*node);
            rec_stack.insert(*node);
            
            if let Some(neighbors) = edges.get(node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        if dfs(neighbor, edges, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(neighbor) {
                        return true;
                    }
                }
            }
            
            rec_stack.remove(node);
            false
        }
        
        for node in edges.keys() {
            if !visited.contains(node) {
                if dfs(node, edges, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Calculate PageRank scores
    pub fn calculate_pagerank(
        edges: &HashMap<Uuid, Vec<Uuid>>,
        damping_factor: f32,
        iterations: usize,
    ) -> HashMap<Uuid, f32> {
        let nodes: Vec<Uuid> = edges.keys().cloned().collect();
        let n = nodes.len() as f32;
        
        // Initialize scores
        let mut scores: HashMap<Uuid, f32> = nodes.iter()
            .map(|&id| (id, 1.0 / n))
            .collect();
        
        // Build reverse edges (incoming links)
        let mut incoming: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (&from, tos) in edges {
            for &to in tos {
                incoming.entry(to).or_insert_with(Vec::new).push(from);
            }
        }
        
        // Iterate
        for _ in 0..iterations {
            let mut new_scores = HashMap::new();
            
            for &node in &nodes {
                let mut score = (1.0 - damping_factor) / n;
                
                if let Some(inbound) = incoming.get(&node) {
                    for &from in inbound {
                        let from_score = scores.get(&from).unwrap_or(&0.0);
                        let out_degree = edges.get(&from).map(|v| v.len()).unwrap_or(0) as f32;
                        if out_degree > 0.0 {
                            score += damping_factor * from_score / out_degree;
                        }
                    }
                }
                
                new_scores.insert(node, score);
            }
            
            scores = new_scores;
        }
        
        scores
    }
}

/// Compression helpers - pure functions
pub mod compression {
    use super::*;
    
    /// Estimate compression ratio for text
    pub fn estimate_compression_ratio(text: &str) -> f32 {
        if text.is_empty() {
            return 1.0;
        }
        
        // Count character frequencies
        let mut freq = HashMap::new();
        for ch in text.chars() {
            *freq.entry(ch).or_insert(0) += 1;
        }
        
        // Calculate entropy
        let total = text.len() as f32;
        let entropy: f32 = freq.values()
            .map(|&count| {
                let p = count as f32 / total;
                -p * p.log2()
            })
            .sum();
        
        // Estimate compression ratio (higher entropy = less compressible)
        1.0 / (1.0 + entropy / 8.0)
    }
    
    /// Determine compression level based on access pattern
    pub fn determine_compression_level(
        access_count: u32,
        last_access_days_ago: u32,
        size_bytes: u64,
    ) -> i32 {
        // Hot data: low compression for fast access
        if access_count > 100 && last_access_days_ago < 7 {
            return 1;
        }
        
        // Warm data: moderate compression
        if access_count > 10 && last_access_days_ago < 30 {
            return 3;
        }
        
        // Cold data: high compression
        if last_access_days_ago > 90 {
            return 9;
        }
        
        // Large files: higher compression
        if size_bytes > 1_000_000 {
            return 6;
        }
        
        // Default
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trigram_extraction() {
        let trigrams = trigram::extract_trigrams("hello");
        assert_eq!(trigrams.len(), 3);
        assert_eq!(trigrams[0], [b'h', b'e', b'l']);
        assert_eq!(trigrams[1], [b'e', b'l', b'l']);
        assert_eq!(trigrams[2], [b'l', b'l', b'o']);
        
        // Edge cases
        assert_eq!(trigram::extract_trigrams("").len(), 0);
        assert_eq!(trigram::extract_trigrams("hi").len(), 0);
        assert_eq!(trigram::extract_trigrams("abc").len(), 1);
    }
    
    #[test]
    fn test_edit_distance() {
        assert_eq!(trigram::edit_distance("hello", "hello"), 0);
        assert_eq!(trigram::edit_distance("hello", "hallo"), 1);
        assert_eq!(trigram::edit_distance("hello", "help"), 2);
        assert_eq!(trigram::edit_distance("", "hello"), 5);
        assert_eq!(trigram::edit_distance("hello", ""), 5);
    }
    
    #[test]
    fn test_bm25_scoring() {
        let score = scoring::calculate_bm25(
            3.0,  // term frequency
            100,  // document length
            150.0, // average doc length
            10,   // document frequency
            1000, // total documents
            1.2,  // k1
            0.75, // b
        );
        assert!(score > 0.0);
    }
    
    #[test]
    fn test_graph_reachability() {
        let mut edges = HashMap::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        
        edges.insert(a, vec![b, c]);
        edges.insert(b, vec![d]);
        edges.insert(c, vec![d]);
        
        let reachable = graph::find_reachable(a, &edges, 1);
        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&c));
        
        let reachable = graph::find_reachable(a, &edges, 2);
        assert_eq!(reachable.len(), 3);
        assert!(reachable.contains(&d));
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut edges = HashMap::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        
        // No cycle
        edges.insert(a, vec![b]);
        edges.insert(b, vec![c]);
        assert!(!graph::has_cycle(&edges));
        
        // Add cycle
        edges.insert(c, vec![a]);
        assert!(graph::has_cycle(&edges));
    }
    
    #[test]
    fn test_btree_basic_operations() -> Result<(), anyhow::Error> {
        use crate::types::{ValidatedDocumentId, ValidatedPath};
        
        // Test node creation
        let leaf_node = btree::create_leaf_node();
        assert!(leaf_node.is_leaf());
        assert_eq!(leaf_node.key_count(), 0);
        
        let internal_node = btree::create_internal_node();
        assert!(!internal_node.is_leaf());
        assert_eq!(internal_node.key_count(), 0);
        
        // Test tree operations
        let mut tree = btree::create_empty_tree();
        assert_eq!(tree.total_keys, 0);
        assert_eq!(tree.height, 0);
        
        // Test insertion
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new("/test/doc.md")?;
        
        tree = btree::insert_into_tree(tree, doc_id.clone(), path.clone())?;
        assert_eq!(tree.total_keys, 1);
        assert_eq!(tree.height, 1);
        
        // Test search
        let result = btree::search_in_tree(&tree, &doc_id);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), path);
        
        // Test tree validation
        assert!(btree::is_valid_btree(&tree));
        assert!(btree::all_leaves_at_same_level(&tree));
        assert!(btree::is_properly_ordered(&tree));
        
        Ok(())
    }
}

/// B+ Tree algorithms - pure functions for indexed operations
/// Stage 3: Pure Function Modularization
pub mod btree {
    use super::*;
    use crate::types::{ValidatedDocumentId, ValidatedPath};
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
    pub const MIN_DEGREE: usize = 3;  // Minimum degree (t)
    pub const MAX_KEYS: usize = 2 * MIN_DEGREE - 1;  // Maximum keys per node (2t-1)
    pub const MIN_KEYS: usize = MIN_DEGREE - 1;      // Minimum keys per non-root node (t-1)
    
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
    pub fn search_in_node<'a>(node: &'a BTreeNode, key: &ValidatedDocumentId) -> Option<(usize, &'a ValidatedPath)> {
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
    ) -> Result<BTreeNode, anyhow::Error> {
        if let BTreeNode::Leaf { ref mut keys, ref mut values, .. } = node {
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
            anyhow::bail!("Cannot insert key-value into non-leaf node")
        }
    }
    
    /// Split a full leaf node into two nodes
    pub fn split_leaf_node(node: BTreeNode) -> Result<(BTreeNode, ValidatedDocumentId, BTreeNode), anyhow::Error> {
        if let BTreeNode::Leaf { keys, values, next_leaf } = node {
            if keys.len() <= MAX_KEYS {
                anyhow::bail!("Node does not need splitting");
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
            anyhow::bail!("Cannot split non-leaf node with leaf split function")
        }
    }
    
    /// Split a full internal node
    pub fn split_internal_node(node: BTreeNode) -> Result<(BTreeNode, ValidatedDocumentId, BTreeNode), anyhow::Error> {
        if let BTreeNode::Internal { keys, children } = node {
            if keys.len() <= MAX_KEYS {
                anyhow::bail!("Node does not need splitting");
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
            anyhow::bail!("Cannot split leaf node with internal split function")
        }
    }
    
    /// Search for a value in the entire tree
    pub fn search_in_tree(root: &BTreeRoot, key: &ValidatedDocumentId) -> Option<ValidatedPath> {
        if let Some(ref root_node) = root.root {
            search_in_subtree(root_node, key)
        } else {
            None
        }
    }
    
    /// Helper function to search in a subtree
    fn search_in_subtree(node: &BTreeNode, key: &ValidatedDocumentId) -> Option<ValidatedPath> {
        match node {
            BTreeNode::Leaf { keys, values, .. } => {
                if let Ok(index) = keys.binary_search(key) {
                    Some(values[index].clone())
                } else {
                    None
                }
            }
            BTreeNode::Internal { keys, children, .. } => {
                let child_index = match keys.binary_search(key) {
                    Ok(index) => index + 1,  // Key found, go to right child
                    Err(index) => index,     // Key not found, go to appropriate child
                };
                
                if child_index < children.len() {
                    search_in_subtree(&children[child_index], key)
                } else {
                    None
                }
            }
        }
    }
    
    /// Insert a key-value pair into the tree
    pub fn insert_into_tree(
        mut root: BTreeRoot,
        key: ValidatedDocumentId,
        value: ValidatedPath,
    ) -> Result<BTreeRoot, anyhow::Error> {
        if root.root.is_none() {
            // Empty tree, create first leaf node
            let leaf = BTreeNode::Leaf {
                keys: vec![key],
                values: vec![value],
                next_leaf: None,
            };
            root.root = Some(Box::new(leaf));
            root.height = 1;
            root.total_keys = 1;
            return Ok(root);
        }
        
        let root_node = root.root.unwrap();
        let (new_root, split_info) = insert_into_node(*root_node, key, value)?;
        
        root.total_keys += 1; // Always increment for successful insertion
        
        // Check if root was split
        if let Some((left, median, right)) = split_info {
            // Root was split, create new root
            let new_root_node = BTreeNode::Internal {
                keys: vec![median],
                children: vec![Box::new(left), Box::new(right)],
            };
            root.root = Some(Box::new(new_root_node));
            root.height += 1;
        } else {
            root.root = Some(Box::new(new_root));
        }
        
        Ok(root)
    }
    
    /// Insert into a node, returning (new_node, optional_split)
    fn insert_into_node(
        node: BTreeNode,
        key: ValidatedDocumentId,
        value: ValidatedPath,
    ) -> Result<(BTreeNode, Option<(BTreeNode, ValidatedDocumentId, BTreeNode)>), anyhow::Error> {
        match node {
            BTreeNode::Leaf { .. } => {
                let new_node = insert_key_value_in_leaf(node, key, value)?;
                
                if new_node.needs_split() {
                    let split_result = split_leaf_node(new_node)?;
                    // Clone the left node since we need it in two places
                    let left_clone = split_result.0.clone();
                    Ok((left_clone, Some(split_result)))
                } else {
                    Ok((new_node, None))
                }
            }
            BTreeNode::Internal { mut keys, mut children } => {
                // Find the child to insert into
                let child_index = match keys.binary_search(&key) {
                    Ok(index) => index + 1,
                    Err(index) => index,
                };
                
                let child = children.remove(child_index);
                let (new_child, split_info) = insert_into_node(*child, key, value)?;
                children.insert(child_index, Box::new(new_child));
                
                // Handle child split
                if let Some((left_child, median_key, right_child)) = split_info {
                    // Insert median key and right child
                    children[child_index] = Box::new(left_child);
                    keys.insert(child_index, median_key);
                    children.insert(child_index + 1, Box::new(right_child));
                }
                
                let new_internal = BTreeNode::Internal { keys, children };
                
                if new_internal.needs_split() {
                    let split_result = split_internal_node(new_internal)?;
                    let left_clone = split_result.0.clone();
                    Ok((left_clone, Some(split_result)))
                } else {
                    Ok((new_internal, None))
                }
            }
        }
    }
    
    /// Delete a key from the B+ tree
    /// Stage 3: Pure function for O(log n) deletion with redistribution and merging
    pub fn delete_from_tree(mut root: BTreeRoot, key: &ValidatedDocumentId) -> Result<BTreeRoot> {
        if root.root.is_none() {
            return Ok(root); // Empty tree, nothing to delete
        }
        
        let (new_root_node, _) = delete_from_node(root.root.take().unwrap(), key)?;
        
        // Handle root node changes
        match new_root_node {
            Some(node) => {
                // Check if root became empty internal node
                if let BTreeNode::Internal { keys, children } = &*node {
                    if keys.is_empty() && children.len() == 1 {
                        // Root has only one child, make child the new root
                        root.root = Some(children[0].clone());
                        root.height = root.height.saturating_sub(1);
                    } else {
                        root.root = Some(node);
                    }
                } else {
                    root.root = Some(node);
                }
            }
            None => {
                // Tree became empty
                root.root = None;
                root.height = 0;
            }
        }
        
        Ok(root)
    }
    
    /// Delete from a node, returning (new_node, was_deleted)
    fn delete_from_node(
        node: Box<BTreeNode>,
        key: &ValidatedDocumentId,
    ) -> Result<(Option<Box<BTreeNode>>, bool)> {
        match *node {
            BTreeNode::Leaf { mut keys, mut values, .. } => {
                // Find and remove key from leaf
                match keys.binary_search(key) {
                    Ok(index) => {
                        keys.remove(index);
                        values.remove(index);
                        
                        if keys.is_empty() {
                            Ok((None, true)) // Leaf became empty
                        } else {
                            Ok((Some(Box::new(BTreeNode::Leaf { keys, values })), true))
                        }
                    }
                    Err(_) => {
                        // Key not found in this leaf
                        Ok((Some(Box::new(BTreeNode::Leaf { keys, values })), false))
                    }
                }
            }
            BTreeNode::Internal { mut keys, mut children } => {
                // Find child that might contain the key
                let child_index = match keys.binary_search(key) {
                    Ok(index) => index + 1,
                    Err(index) => index,
                };
                
                let child = children.remove(child_index);
                let (new_child, was_deleted) = delete_from_node(child, key)?;
                
                if !was_deleted {
                    // Key not found, restore child and return
                    children.insert(child_index, child);
                    return Ok((Some(Box::new(BTreeNode::Internal { keys, children })), false));
                }
                
                // Handle child modifications
                match new_child {
                    Some(child) => {
                        children.insert(child_index, child);
                        
                        // Check if child needs rebalancing (has too few keys)
                        if !is_root_node(&children[child_index]) && child_needs_rebalancing(&children[child_index]) {
                            rebalance_after_deletion(&mut keys, &mut children, child_index)?;
                        }
                        
                        Ok((Some(Box::new(BTreeNode::Internal { keys, children })), true))
                    }
                    None => {
                        // Child was deleted entirely
                        if child_index > 0 {
                            keys.remove(child_index - 1);
                        } else if !keys.is_empty() {
                            keys.remove(0);
                        }
                        
                        if children.is_empty() {
                            Ok((None, true))
                        } else {
                            Ok((Some(Box::new(BTreeNode::Internal { keys, children })), true))
                        }
                    }
                }
            }
        }
    }
    
    /// Check if a node needs rebalancing (has too few keys)
    fn child_needs_rebalancing(node: &BTreeNode) -> bool {
        match node {
            BTreeNode::Leaf { keys, .. } => keys.len() < MIN_KEYS,
            BTreeNode::Internal { keys, .. } => keys.len() < MIN_KEYS,
        }
    }
    
    /// Check if this could be a root node (for special handling)
    fn is_root_node(node: &BTreeNode) -> bool {
        // This is a heuristic - in practice, we'd need to track this explicitly
        false
    }
    
    /// Rebalance a child node that has too few keys
    fn rebalance_after_deletion(
        parent_keys: &mut Vec<ValidatedDocumentId>,
        children: &mut Vec<Box<BTreeNode>>,
        child_index: usize,
    ) -> Result<()> {
        // Try to borrow from left sibling
        if child_index > 0 && can_borrow_from(&children[child_index - 1]) {
            borrow_from_left_sibling(parent_keys, children, child_index)?;
        }
        // Try to borrow from right sibling
        else if child_index < children.len() - 1 && can_borrow_from(&children[child_index + 1]) {
            borrow_from_right_sibling(parent_keys, children, child_index)?;
        }
        // Merge with a sibling
        else if child_index > 0 {
            merge_with_left_sibling(parent_keys, children, child_index)?;
        } else if child_index < children.len() - 1 {
            merge_with_right_sibling(parent_keys, children, child_index)?;
        }
        
        Ok(())
    }
    
    /// Check if a node has enough keys to lend one
    fn can_borrow_from(node: &BTreeNode) -> bool {
        match node {
            BTreeNode::Leaf { keys, .. } => keys.len() > MIN_KEYS,
            BTreeNode::Internal { keys, .. } => keys.len() > MIN_KEYS,
        }
    }
    
    /// Borrow a key from the left sibling
    fn borrow_from_left_sibling(
        parent_keys: &mut Vec<ValidatedDocumentId>,
        children: &mut Vec<Box<BTreeNode>>,
        child_index: usize,
    ) -> Result<()> {
        let separator_index = child_index - 1;
        
        match (children[child_index - 1].as_mut(), children[child_index].as_mut()) {
            (BTreeNode::Leaf { keys: left_keys, values: left_values, .. },
             BTreeNode::Leaf { keys: right_keys, values: right_values, .. }) => {
                // Move the last key-value from left to right
                let borrowed_key = left_keys.pop().unwrap();
                let borrowed_value = left_values.pop().unwrap();
                
                right_keys.insert(0, borrowed_key.clone());
                right_values.insert(0, borrowed_value);
                
                // Update parent separator
                parent_keys[separator_index] = borrowed_key;
            }
            (BTreeNode::Internal { keys: left_keys, children: left_children },
             BTreeNode::Internal { keys: right_keys, children: right_children }) => {
                // Move separator down to right, and rightmost key from left up
                let separator = parent_keys[separator_index].clone();
                let borrowed_key = left_keys.pop().unwrap();
                let borrowed_child = left_children.pop().unwrap();
                
                right_keys.insert(0, separator);
                right_children.insert(0, borrowed_child);
                parent_keys[separator_index] = borrowed_key;
            }
            _ => bail!("Sibling type mismatch during borrow"),
        }
        
        Ok(())
    }
    
    /// Borrow a key from the right sibling
    fn borrow_from_right_sibling(
        parent_keys: &mut Vec<ValidatedDocumentId>,
        children: &mut Vec<Box<BTreeNode>>,
        child_index: usize,
    ) -> Result<()> {
        let separator_index = child_index;
        
        match (children[child_index].as_mut(), children[child_index + 1].as_mut()) {
            (BTreeNode::Leaf { keys: left_keys, values: left_values, .. },
             BTreeNode::Leaf { keys: right_keys, values: right_values, .. }) => {
                // Move the first key-value from right to left
                let borrowed_key = right_keys.remove(0);
                let borrowed_value = right_values.remove(0);
                
                left_keys.push(borrowed_key);
                left_values.push(borrowed_value);
                
                // Update parent separator
                parent_keys[separator_index] = right_keys[0].clone();
            }
            (BTreeNode::Internal { keys: left_keys, children: left_children },
             BTreeNode::Internal { keys: right_keys, children: right_children }) => {
                // Move separator down to left, and leftmost key from right up
                let separator = parent_keys[separator_index].clone();
                let borrowed_key = right_keys.remove(0);
                let borrowed_child = right_children.remove(0);
                
                left_keys.push(separator);
                left_children.push(borrowed_child);
                parent_keys[separator_index] = borrowed_key;
            }
            _ => bail!("Sibling type mismatch during borrow"),
        }
        
        Ok(())
    }
    
    /// Merge with left sibling
    fn merge_with_left_sibling(
        parent_keys: &mut Vec<ValidatedDocumentId>,
        children: &mut Vec<Box<BTreeNode>>,
        child_index: usize,
    ) -> Result<()> {
        let separator_index = child_index - 1;
        let separator = parent_keys.remove(separator_index);
        
        let right_node = children.remove(child_index);
        let left_node = &mut children[child_index - 1];
        
        match (left_node.as_mut(), right_node.as_ref()) {
            (BTreeNode::Leaf { keys: left_keys, values: left_values, .. },
             BTreeNode::Leaf { keys: right_keys, values: right_values, .. }) => {
                // Merge leaf nodes
                left_keys.extend(right_keys.iter().cloned());
                left_values.extend(right_values.iter().cloned());
            }
            (BTreeNode::Internal { keys: left_keys, children: left_children },
             BTreeNode::Internal { keys: right_keys, children: right_children }) => {
                // Merge internal nodes with separator
                left_keys.push(separator);
                left_keys.extend(right_keys.iter().cloned());
                left_children.extend(right_children.iter().cloned());
            }
            _ => bail!("Node type mismatch during merge"),
        }
        
        Ok(())
    }
    
    /// Merge with right sibling
    fn merge_with_right_sibling(
        parent_keys: &mut Vec<ValidatedDocumentId>,
        children: &mut Vec<Box<BTreeNode>>,
        child_index: usize,
    ) -> Result<()> {
        let separator_index = child_index;
        let separator = parent_keys.remove(separator_index);
        
        let left_node = children.remove(child_index);
        let right_node = &mut children[child_index];
        
        match (left_node.as_ref(), right_node.as_mut()) {
            (BTreeNode::Leaf { keys: left_keys, values: left_values, .. },
             BTreeNode::Leaf { keys: right_keys, values: right_values, .. }) => {
                // Merge leaf nodes
                let mut new_keys = left_keys.clone();
                new_keys.extend(right_keys.iter().cloned());
                let mut new_values = left_values.clone();
                new_values.extend(right_values.iter().cloned());
                
                *right_keys = new_keys;
                *right_values = new_values;
            }
            (BTreeNode::Internal { keys: left_keys, children: left_children },
             BTreeNode::Internal { keys: right_keys, children: right_children }) => {
                // Merge internal nodes with separator
                let mut new_keys = left_keys.clone();
                new_keys.push(separator);
                new_keys.extend(right_keys.iter().cloned());
                
                let mut new_children = left_children.clone();
                new_children.extend(right_children.iter().cloned());
                
                *right_keys = new_keys;
                *right_children = new_children;
            }
            _ => bail!("Node type mismatch during merge"),
        }
        
        Ok(())
    }
    
    /// Validate B+ tree invariants
    pub fn is_valid_btree(root: &BTreeRoot) -> bool {
        if let Some(ref root_node) = root.root {
            validate_node_invariants(root_node, true) && 
            all_leaves_at_same_level(root) &&
            is_properly_ordered(root)
        } else {
            true // Empty tree is valid
        }
    }
    
    /// Validate individual node invariants
    fn validate_node_invariants(node: &BTreeNode, is_root: bool) -> bool {
        match node {
            BTreeNode::Leaf { keys, values, .. } => {
                // Leaf node checks
                keys.len() == values.len() &&
                (is_root || keys.len() >= MIN_KEYS) &&
                keys.len() <= MAX_KEYS &&
                is_sorted(&keys)
            }
            BTreeNode::Internal { keys, children } => {
                // Internal node checks
                children.len() == keys.len() + 1 &&
                (is_root || keys.len() >= MIN_KEYS) &&
                keys.len() <= MAX_KEYS &&
                is_sorted(&keys) &&
                children.iter().all(|child| validate_node_invariants(child, false))
            }
        }
    }
    
    /// Check if keys are sorted
    fn is_sorted(keys: &[ValidatedDocumentId]) -> bool {
        keys.windows(2).all(|w| w[0] <= w[1])
    }
    
    /// Check if all leaves are at the same level
    pub fn all_leaves_at_same_level(root: &BTreeRoot) -> bool {
        if let Some(ref root_node) = root.root {
            let leaf_level = find_leaf_level(root_node, 0);
            check_all_leaves_at_level(root_node, 0, leaf_level)
        } else {
            true
        }
    }
    
    /// Find the level of the first leaf encountered
    fn find_leaf_level(node: &BTreeNode, current_level: u32) -> u32 {
        match node {
            BTreeNode::Leaf { .. } => current_level,
            BTreeNode::Internal { children, .. } => {
                if let Some(first_child) = children.first() {
                    find_leaf_level(first_child, current_level + 1)
                } else {
                    current_level
                }
            }
        }
    }
    
    /// Check if all leaves are at the expected level
    fn check_all_leaves_at_level(node: &BTreeNode, current_level: u32, expected_leaf_level: u32) -> bool {
        match node {
            BTreeNode::Leaf { .. } => current_level == expected_leaf_level,
            BTreeNode::Internal { children, .. } => {
                children.iter().all(|child| {
                    check_all_leaves_at_level(child, current_level + 1, expected_leaf_level)
                })
            }
        }
    }
    
    /// Check if tree maintains proper key ordering
    pub fn is_properly_ordered(root: &BTreeRoot) -> bool {
        if let Some(ref root_node) = root.root {
            check_ordering(root_node, None, None)
        } else {
            true
        }
    }
    
    /// Helper to check ordering constraints
    fn check_ordering(
        node: &BTreeNode,
        min_bound: Option<&ValidatedDocumentId>,
        max_bound: Option<&ValidatedDocumentId>,
    ) -> bool {
        match node {
            BTreeNode::Leaf { keys, .. } => {
                // Check bounds
                if let Some(min) = min_bound {
                    if keys.first().map_or(false, |k| k < min) {
                        return false;
                    }
                }
                if let Some(max) = max_bound {
                    if keys.last().map_or(false, |k| k >= max) {
                        return false;
                    }
                }
                is_sorted(keys)
            }
            BTreeNode::Internal { keys, children } => {
                if !is_sorted(keys) {
                    return false;
                }
                
                // Check each child with appropriate bounds
                for (i, child) in children.iter().enumerate() {
                    let child_min = if i == 0 { min_bound } else { keys.get(i - 1) };
                    let child_max = keys.get(i);
                    
                    if !check_ordering(child, child_min, child_max) {
                        return false;
                    }
                }
                true
            }
        }
    }
    
    /// Count total keys in tree
    pub fn count_total_keys(root: &BTreeRoot) -> usize {
        if let Some(ref root_node) = root.root {
            count_keys_in_subtree(root_node)
        } else {
            0
        }
    }
    
    /// Extract all key-value pairs from the tree (ordered traversal)
    pub fn extract_all_pairs(root: &BTreeRoot) -> Vec<(ValidatedDocumentId, ValidatedPath)> {
        let mut results = Vec::new();
        if let Some(ref root_node) = root.root {
            extract_pairs_from_subtree(root_node, &mut results);
        }
        results
    }
    
    /// Helper to count keys recursively
    fn count_keys_in_subtree(node: &BTreeNode) -> usize {
        match node {
            BTreeNode::Leaf { keys, .. } => keys.len(),
            BTreeNode::Internal { children, .. } => {
                children.iter().map(|child| count_keys_in_subtree(child)).sum()
            }
        }
    }
    
    /// Helper to extract all key-value pairs in order
    fn extract_pairs_from_subtree(
        node: &BTreeNode,
        results: &mut Vec<(ValidatedDocumentId, ValidatedPath)>,
    ) {
        match node {
            BTreeNode::Leaf { keys, values, .. } => {
                for (key, value) in keys.iter().zip(values.iter()) {
                    results.push((key.clone(), value.clone()));
                }
            }
            BTreeNode::Internal { children, .. } => {
                for child in children {
                    extract_pairs_from_subtree(child, results);
                }
            }
        }
    }
    
    /// Range search: find all keys between start and end (inclusive)
    pub fn range_search(
        root: &BTreeRoot,
        start_key: &ValidatedDocumentId,
        end_key: &ValidatedDocumentId,
    ) -> Vec<(ValidatedDocumentId, ValidatedPath)> {
        let mut results = Vec::new();
        if let Some(ref root_node) = root.root {
            range_search_in_subtree(root_node, start_key, end_key, &mut results);
        }
        results
    }
    
    /// Helper for range search
    fn range_search_in_subtree(
        node: &BTreeNode,
        start_key: &ValidatedDocumentId,
        end_key: &ValidatedDocumentId,
        results: &mut Vec<(ValidatedDocumentId, ValidatedPath)>,
    ) {
        match node {
            BTreeNode::Leaf { keys, values, .. } => {
                for (key, value) in keys.iter().zip(values.iter()) {
                    if key >= start_key && key <= end_key {
                        results.push((key.clone(), value.clone()));
                    }
                }
            }
            BTreeNode::Internal { keys, children } => {
                for (i, child) in children.iter().enumerate() {
                    // Determine if this child might contain relevant keys
                    let child_min = keys.get(i.wrapping_sub(1));
                    let child_max = keys.get(i);
                    
                    let should_search = match (child_min, child_max) {
                        (Some(min), Some(max)) => start_key <= max && end_key >= min,
                        (Some(min), None) => end_key >= min,
                        (None, Some(max)) => start_key <= max,
                        (None, None) => true,
                    };
                    
                    if should_search {
                        range_search_in_subtree(child, start_key, end_key, results);
                    }
                }
            }
        }
    }
}