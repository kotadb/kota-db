// Query Sanitization Module - Enhanced Security for Search Input
// This module provides comprehensive sanitization and validation for search queries
// to prevent injection attacks and ensure safe query processing.

use crate::validation::ValidationContext;
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

/// Maximum length for a search query to prevent resource exhaustion
const MAX_QUERY_LENGTH: usize = 1024;

/// Maximum number of terms in a search query
const MAX_QUERY_TERMS: usize = 50;

/// Maximum length for a single search term
const MAX_TERM_LENGTH: usize = 100;

/// Minimum length for a meaningful search term
const MIN_TERM_LENGTH: usize = 1;

/// Reserved characters that could be used in injection attempts
const RESERVED_CHARS: &[char] = &['<', '>', '&', '"', '\'', '\0', '\r', '\n', '\t'];

/// SQL injection patterns to detect and block
static SQL_INJECTION_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute|script|javascript|eval|onload|onerror|onclick|<script|<iframe|<object|<embed|<link|../|..\\)")
        .expect("Failed to compile SQL injection regex")
});

/// Command injection patterns to detect and block
static COMMAND_INJECTION_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(\||;|&&|\|\||`|\$\(|<\(|>\(|\$\{|%0a|%0d|%00)")
        .expect("Failed to compile command injection regex")
});

/// Path traversal patterns to detect and block
static PATH_TRAVERSAL_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\.\.\/|\.\.\\|\.\.%2f|\.\.%2F|\.\.%5c|\.\.%5C|%2e%2e|%252e%252e)")
        .expect("Failed to compile path traversal regex")
});

/// LDAP injection patterns to detect and block (excluding asterisk for wildcard support)
static LDAP_INJECTION_PATTERNS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[()\\,=]").expect("Failed to compile LDAP injection regex"));

/// Common stop words that might be filtered in certain contexts
static STOP_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    vec![
        "the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for", "not", "on",
        "with", "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we",
        "say", "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their",
        "what", "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "when",
        "make", "can", "like", "time", "no", "just", "him", "know", "take", "people", "into",
        "year", "your", "good", "some", "could", "them", "see", "other", "than", "then", "now",
        "look", "only", "come", "its", "over", "think", "also", "back", "after", "use", "two",
        "how", "our", "work", "first", "well", "way", "even", "new", "want", "because", "any",
        "these", "give", "day", "most", "us",
    ]
    .into_iter()
    .collect()
});

/// Result of query sanitization
#[derive(Debug, Clone)]
pub struct SanitizedQuery {
    /// The sanitized query text
    pub text: String,
    /// Individual sanitized terms
    pub terms: Vec<String>,
    /// Whether the query was modified during sanitization
    pub was_modified: bool,
    /// Warnings generated during sanitization
    pub warnings: Vec<String>,
}

impl SanitizedQuery {
    /// Check if the query is effectively empty after sanitization
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty() || self.text == "*"
    }

    /// Check if this is a wildcard query
    pub fn is_wildcard(&self) -> bool {
        self.text == "*"
    }
}

/// Sanitize a search query for safe processing
pub fn sanitize_search_query(query: &str) -> Result<SanitizedQuery> {
    let ctx =
        ValidationContext::new("sanitize_search_query").with_attribute("original_query", query);

    let mut warnings = Vec::new();
    let original = query.to_string();

    // Step 1: Length validation
    if query.len() > MAX_QUERY_LENGTH {
        bail!(
            "Query exceeds maximum length of {} characters",
            MAX_QUERY_LENGTH
        );
    }

    // Step 2: Check for null bytes and control characters
    if query.contains('\0') {
        bail!("Query contains null bytes");
    }

    // Step 3: Normalize whitespace and trim
    let mut sanitized = query
        .chars()
        .map(|c| if c.is_control() && c != ' ' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Step 4: Check for injection patterns
    if SQL_INJECTION_PATTERNS.is_match(&sanitized) {
        warnings.push("Potentially dangerous SQL patterns detected and removed".to_string());
        sanitized = SQL_INJECTION_PATTERNS
            .replace_all(&sanitized, "")
            .to_string();
    }

    if COMMAND_INJECTION_PATTERNS.is_match(&sanitized) {
        warnings.push("Potentially dangerous command patterns detected and removed".to_string());
        sanitized = COMMAND_INJECTION_PATTERNS
            .replace_all(&sanitized, "")
            .to_string();
    }

    if PATH_TRAVERSAL_PATTERNS.is_match(&sanitized) {
        warnings.push("Path traversal patterns detected and removed".to_string());
        sanitized = PATH_TRAVERSAL_PATTERNS
            .replace_all(&sanitized, "")
            .to_string();
    }

    // Step 5: Remove LDAP injection characters for safety (preserving wildcards)
    // Check if this is a wildcard query pattern before applying LDAP sanitization
    let is_wildcard_query = sanitized.contains('*');
    if LDAP_INJECTION_PATTERNS.is_match(&sanitized) {
        warnings.push("Special LDAP characters sanitized".to_string());
        sanitized = LDAP_INJECTION_PATTERNS
            .replace_all(&sanitized, " ")
            .to_string();
    }

    // Step 6: Remove reserved/dangerous characters
    let mut clean_chars = String::with_capacity(sanitized.len());
    for c in sanitized.chars() {
        if RESERVED_CHARS.contains(&c) {
            clean_chars.push(' ');
            if warnings.is_empty() || !warnings.last().unwrap().contains("Reserved characters") {
                warnings.push("Reserved characters removed from query".to_string());
            }
        } else {
            clean_chars.push(c);
        }
    }
    sanitized = clean_chars;

    // Step 7: Normalize whitespace again after removals
    sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");

    // Step 8: Extract and validate individual terms
    let terms: Vec<String> = sanitized
        .split_whitespace()
        .filter(|term| {
            // Allow wildcard patterns like *, *test, test*, *test*
            let is_wildcard_pattern = term.contains('*');
            let non_wildcard_chars: String = term.chars().filter(|&c| c != '*').collect();

            // For wildcard patterns, check the non-wildcard portion
            if is_wildcard_pattern {
                // Pure wildcard "*" is always valid
                *term == "*"
                    || (!non_wildcard_chars.is_empty()
                        && non_wildcard_chars.len() <= MAX_TERM_LENGTH)
            } else {
                // Regular term validation
                term.len() >= MIN_TERM_LENGTH
                    && term.len() <= MAX_TERM_LENGTH
                    && !term.chars().all(|c| c.is_numeric() || !c.is_alphanumeric())
            }
        })
        .take(MAX_QUERY_TERMS)
        .map(|s| s.to_string())
        .collect();

    // Step 9: Check if we removed too many terms
    let original_term_count = query.split_whitespace().count();
    if terms.len() < original_term_count / 2 && original_term_count > 2 {
        warnings.push(format!(
            "Many terms were filtered out ({} of {} remaining)",
            terms.len(),
            original_term_count
        ));
    }

    // Step 10: Rebuild the sanitized query from valid terms
    let final_text = if terms.is_empty() && query.trim() == "*" {
        "*".to_string() // Preserve wildcard queries
    } else {
        terms.join(" ")
    };

    // Step 11: Final validation
    ctx.validate(
        !final_text.is_empty() || query.trim() == "*",
        "Query became empty after sanitization",
    )?;

    let was_modified = original != final_text;

    Ok(SanitizedQuery {
        text: final_text,
        terms,
        was_modified,
        warnings,
    })
}

/// Sanitize a path pattern for safe file system operations
pub fn sanitize_path_pattern(pattern: &str) -> Result<String> {
    let ctx = ValidationContext::new("sanitize_path_pattern").with_attribute("pattern", pattern);

    // Use existing path validation from validation module
    crate::validation::path::validate_file_path(pattern)?;

    // Additional pattern-specific validation
    ctx.validate(
        !pattern.contains("**/**"),
        "Recursive wildcard patterns are not allowed",
    )?;

    Ok(pattern.to_string())
}

/// Validate and sanitize a tag
pub fn sanitize_tag(tag: &str) -> Result<String> {
    let ctx = ValidationContext::new("sanitize_tag").with_attribute("tag", tag);

    ctx.clone().validate(
        tag.len() <= 50,
        "Tag exceeds maximum length of 50 characters",
    )?;

    ctx.validate(
        tag.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
        "Tag contains invalid characters",
    )?;

    Ok(tag.to_lowercase())
}

/// Check if a term is a common stop word
pub fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(word.to_lowercase().as_str())
}

/// Filter stop words from a list of terms
pub fn filter_stop_words(terms: &[String]) -> Vec<String> {
    terms
        .iter()
        .filter(|term| !is_stop_word(term))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sanitization() {
        let result = sanitize_search_query("hello world").unwrap();
        assert_eq!(result.text, "hello world");
        assert!(!result.was_modified);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_sql_injection_removal() {
        let result = sanitize_search_query("test UNION SELECT * FROM users").unwrap();
        assert!(!result.text.contains("UNION"));
        assert!(!result.text.contains("SELECT"));
        assert!(result.was_modified);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_command_injection_removal() {
        let result = sanitize_search_query("test; rm -rf /").unwrap();
        assert!(!result.text.contains(";"));
        assert!(result.was_modified);
    }

    #[test]
    fn test_path_traversal_removal() {
        let result = sanitize_search_query("../../etc/passwd").unwrap();
        assert!(!result.text.contains(".."));
        assert!(result.was_modified);
    }

    #[test]
    fn test_null_byte_rejection() {
        let result = sanitize_search_query("test\0query");
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_normalization() {
        let result = sanitize_search_query("  hello   \t\n  world  ").unwrap();
        assert_eq!(result.text, "hello world");
        assert!(result.was_modified);
    }

    #[test]
    fn test_wildcard_preservation() {
        let result = sanitize_search_query("*").unwrap();
        assert_eq!(result.text, "*");
        assert!(result.is_wildcard());
        assert!(!result.was_modified);
    }

    #[test]
    fn test_xss_prevention() {
        let result = sanitize_search_query("<script>alert('xss')</script>").unwrap();
        assert!(!result.text.contains("<script"));
        assert!(!result.text.contains("</script>"));
        assert!(result.was_modified);
    }

    #[test]
    fn test_tag_sanitization() {
        assert_eq!(sanitize_tag("Valid-Tag_123").unwrap(), "valid-tag_123");
        assert!(sanitize_tag("invalid!tag").is_err());
        assert!(sanitize_tag(&"x".repeat(51)).is_err());
    }

    #[test]
    fn test_stop_word_detection() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("The"));
        assert!(is_stop_word("AND"));
        assert!(!is_stop_word("rust"));
        assert!(!is_stop_word("database"));
    }

    #[test]
    fn test_term_extraction() {
        let result = sanitize_search_query("find rust async functions in database").unwrap();
        // The term count may vary based on filtering
        assert!(result.terms.len() >= 4); // At least have the main keywords
        assert!(result.terms.contains(&"rust".to_string()));
        assert!(result.terms.contains(&"database".to_string()));
    }

    #[test]
    fn test_max_query_length() {
        let long_query = "x".repeat(MAX_QUERY_LENGTH + 1);
        let result = sanitize_search_query(&long_query);
        assert!(result.is_err());
    }

    #[test]
    fn test_ldap_injection_sanitization() {
        let result = sanitize_search_query("user*(admin)").unwrap();
        // Asterisk should be preserved for wildcard support
        assert!(result.text.contains("*"));
        // But parentheses should still be removed
        assert!(!result.text.contains("("));
        assert!(!result.text.contains(")"));
        assert!(result.was_modified);
    }

    #[test]
    fn test_unicode_handling() {
        let result = sanitize_search_query("hello 世界 мир").unwrap();
        assert_eq!(result.text, "hello 世界 мир");
        assert!(!result.was_modified);
    }

    #[test]
    fn test_empty_after_sanitization() {
        // Query that becomes empty after removing all dangerous content
        // This should fail validation since it becomes empty
        let result = sanitize_search_query("';--");
        assert!(result.is_err() || result.unwrap().text.is_empty());
    }

    #[test]
    fn test_wildcard_patterns_preserved() {
        // Test various wildcard patterns that should be preserved
        let patterns = vec![
            ("*", "*"),
            ("*test", "*test"),
            ("test*", "test*"),
            ("*test*", "*test*"),
            ("*Controller", "*Controller"),
            ("test_*", "test_*"),
            ("prefix*suffix", "prefix*suffix"),
            ("multiple*wild*cards", "multiple*wild*cards"),
        ];

        for (input, expected) in patterns {
            let result = sanitize_search_query(input).unwrap();
            assert_eq!(result.text, expected, "Failed for pattern: {}", input);
            assert!(
                !result.was_modified
                    || result.warnings.is_empty()
                    || !result.warnings.iter().any(|w| w.contains("LDAP")),
                "LDAP warning should not be triggered for wildcards: {}",
                input
            );
        }
    }

    #[test]
    fn test_wildcard_with_dangerous_chars() {
        // Test that wildcards are preserved while dangerous chars are removed
        let result = sanitize_search_query("*test<script>*").unwrap();
        assert!(result.text.contains("*test"));
        assert!(!result.text.contains("<script>"));
        assert!(result.was_modified);
    }

    #[test]
    fn test_wildcard_with_ldap_chars() {
        // Test that asterisks are preserved but other LDAP chars are removed
        let result = sanitize_search_query("test*(admin)=value").unwrap();
        assert!(result.text.contains("*"));
        assert!(!result.text.contains("("));
        assert!(!result.text.contains(")"));
        assert!(!result.text.contains("="));
        assert!(result.was_modified);
    }

    #[test]
    fn test_complex_wildcard_query() {
        // Test a real-world wildcard query pattern
        let result = sanitize_search_query("*Controller OR test_* AND *.rs").unwrap();
        assert!(result.text.contains("*Controller"));
        assert!(result.text.contains("test_*"));
        assert!(result.text.contains("*.rs") || result.text.contains("* rs"));
        // OR and AND should be preserved as regular terms
        assert!(result.text.contains("OR"));
        assert!(result.text.contains("AND"));
    }
}
