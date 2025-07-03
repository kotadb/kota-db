// Metadata Processing - Pure Functions
// This module contains functions for parsing and extracting metadata from documents

use std::collections::HashMap;
use serde_yaml;
use sha2::{Sha256, Digest};

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
    let mut hasher = Sha256::new();
    hasher.update(content);
    hasher.finalize().into()
}