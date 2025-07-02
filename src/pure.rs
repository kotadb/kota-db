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
}