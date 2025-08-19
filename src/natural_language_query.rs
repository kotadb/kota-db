//! Natural language query processing for code analysis
//!
//! This module provides natural language query capabilities for KotaDB,
//! allowing users to search code using human-readable queries like:
//! - "find all error handling patterns"
//! - "show functions that handle user input"
//! - "what calls the storage layer?"

use crate::{
    dependency_extractor::DependencyGraph,
    parsing::SymbolType,
    relationship_query::{
        parse_natural_language_relationship_query, RelationshipQueryResult, RelationshipStats,
    },
    symbol_index::{
        CodePattern, CodeQuery, DependencyDirection, QueryOperator, SearchScope, SymbolIndex,
        SymbolSearchResult,
    },
    symbol_storage::SymbolStorage,
    types::{ValidatedDocumentId, ValidatedPath},
};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Cached regex patterns for common query patterns
static ERROR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\berror\b.*\b(handling|handle)\b").expect("Invalid error pattern")
});

static ASYNC_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(async|await)\b").expect("Invalid async pattern"));

#[allow(dead_code)]
static DEPENDENCY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(what|who)\s+(calls|depends|imports)\b").expect("Invalid dependency pattern")
});

/// Natural language query intent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryIntent {
    /// Find symbols by name or type
    FindSymbols {
        symbol_types: Option<Vec<SymbolType>>,
        name_pattern: String,
        fuzzy: bool,
    },
    /// Find code patterns (error handling, validation, etc.)
    FindPatterns {
        pattern: CodePattern,
        scope: SearchScope,
    },
    /// Find dependencies and relationships
    FindDependencies {
        target: String,
        direction: DependencyDirection,
    },
    /// Find by function signature
    FindSignature {
        pattern: String,
        language: Option<String>,
    },
    /// Complex multi-part query
    Combined {
        queries: Vec<CodeQuery>,
        operator: QueryOperator,
    },
    /// Find relationships between symbols
    FindRelationships {
        query_result: RelationshipQueryResult,
    },
}

/// Natural language query processor
pub struct NaturalLanguageQueryProcessor {
    symbol_index: Option<SymbolIndex>,
    dependency_graph: Option<DependencyGraph>,
    symbol_storage: Option<SymbolStorage>,
    #[allow(dead_code)]
    pattern_matchers: HashMap<&'static str, PatternMatcher>,
}

/// Pattern matching configuration
#[allow(dead_code)]
struct PatternMatcher {
    keywords: Vec<String>,
    code_pattern: CodePattern,
}

impl Default for NaturalLanguageQueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl NaturalLanguageQueryProcessor {
    /// Create a new natural language query processor
    pub fn new() -> Self {
        let mut pattern_matchers = HashMap::new();

        // Initialize pattern matchers with common patterns
        pattern_matchers.insert(
            "error",
            PatternMatcher {
                keywords: vec![
                    "error".to_string(),
                    "err".to_string(),
                    "result".to_string(),
                    "exception".to_string(),
                    "failure".to_string(),
                ],
                code_pattern: CodePattern::ErrorHandling,
            },
        );

        pattern_matchers.insert(
            "async",
            PatternMatcher {
                keywords: vec![
                    "async".to_string(),
                    "await".to_string(),
                    "future".to_string(),
                    "concurrent".to_string(),
                ],
                code_pattern: CodePattern::AsyncAwait,
            },
        );

        pattern_matchers.insert(
            "test",
            PatternMatcher {
                keywords: vec![
                    "test".to_string(),
                    "assert".to_string(),
                    "expect".to_string(),
                    "should".to_string(),
                ],
                code_pattern: CodePattern::TestCode,
            },
        );

        pattern_matchers.insert(
            "todo",
            PatternMatcher {
                keywords: vec![
                    "todo".to_string(),
                    "fixme".to_string(),
                    "hack".to_string(),
                    "note".to_string(),
                ],
                code_pattern: CodePattern::TodoComments,
            },
        );

        pattern_matchers.insert(
            "security",
            PatternMatcher {
                keywords: vec![
                    "password".to_string(),
                    "secret".to_string(),
                    "key".to_string(),
                    "token".to_string(),
                    "auth".to_string(),
                ],
                code_pattern: CodePattern::SecurityPatterns,
            },
        );

        Self {
            symbol_index: None,
            dependency_graph: None,
            symbol_storage: None,
            pattern_matchers,
        }
    }

    /// Set the symbol index for code-specific searches
    pub fn with_symbol_index(mut self, index: SymbolIndex) -> Self {
        self.symbol_index = Some(index);
        self
    }

    /// Set the dependency graph for relationship queries
    pub fn with_dependency_graph(mut self, graph: DependencyGraph) -> Self {
        self.dependency_graph = Some(graph);
        self
    }

    /// Set the symbol storage for additional metadata
    pub fn with_symbol_storage(mut self, storage: SymbolStorage) -> Self {
        self.symbol_storage = Some(storage);
        self
    }

    /// Parse natural language query into structured intent
    #[instrument(skip(self))]
    pub async fn parse_query(&self, query: &str) -> Result<QueryIntent> {
        let query_lower = query.to_lowercase();
        debug!("Parsing natural language query: {}", query);

        // Try relationship queries first (they're more specific)
        if let Some(relationship_type) = parse_natural_language_relationship_query(query) {
            if self.dependency_graph.is_some() && self.symbol_storage.is_some() {
                // Return a special intent that indicates we need to execute a relationship query
                // The actual execution will happen in execute_intent with proper borrowing
                return Ok(QueryIntent::FindRelationships {
                    query_result: RelationshipQueryResult {
                        query_type: relationship_type,
                        direct_relationships: vec![],
                        indirect_relationships: vec![],
                        stats: RelationshipStats {
                            direct_count: 0,
                            indirect_count: 0,
                            symbols_analyzed: 0,
                            execution_time_ms: 0,
                            truncated: false,
                        },
                        summary: "Relationship query pending execution".to_string(),
                    },
                });
            } else {
                debug!("Relationship query detected but dependency graph or symbol storage not available");
            }
        }

        // Try pattern-based detection
        if let Some(intent) = self.detect_pattern_query(&query_lower) {
            return Ok(intent);
        }

        // Try dependency-based detection
        if let Some(intent) = self.detect_dependency_query(query, &query_lower)? {
            return Ok(intent);
        }

        // Try symbol-based detection
        if let Some(intent) = self.detect_symbol_query(&query_lower)? {
            return Ok(intent);
        }

        // Signature-based queries
        if query_lower.contains("signature") || query_lower.contains("function with") {
            let pattern = self.extract_signature_pattern(&query_lower)?;
            return Ok(QueryIntent::FindSignature {
                pattern,
                language: None,
            });
        }

        // Default to symbol search with the entire query as pattern
        Ok(QueryIntent::FindSymbols {
            symbol_types: None,
            name_pattern: query.to_string(),
            fuzzy: true,
        })
    }

    /// Detect pattern-based queries
    fn detect_pattern_query(&self, query_lower: &str) -> Option<QueryIntent> {
        if ERROR_PATTERN.is_match(query_lower) {
            return Some(QueryIntent::FindPatterns {
                pattern: CodePattern::ErrorHandling,
                scope: SearchScope::All,
            });
        }

        if ASYNC_PATTERN.is_match(query_lower) {
            return Some(QueryIntent::FindPatterns {
                pattern: CodePattern::AsyncAwait,
                scope: SearchScope::All,
            });
        }

        if query_lower.contains("test") {
            return Some(QueryIntent::FindPatterns {
                pattern: CodePattern::TestCode,
                scope: SearchScope::All,
            });
        }

        if query_lower.contains("todo") || query_lower.contains("fixme") {
            return Some(QueryIntent::FindPatterns {
                pattern: CodePattern::TodoComments,
                scope: SearchScope::Comments,
            });
        }

        if query_lower.contains("security")
            || query_lower.contains("password")
            || query_lower.contains("secret")
        {
            return Some(QueryIntent::FindPatterns {
                pattern: CodePattern::SecurityPatterns,
                scope: SearchScope::All,
            });
        }

        None
    }

    /// Detect dependency-based queries
    fn detect_dependency_query(
        &self,
        query: &str,
        query_lower: &str,
    ) -> Result<Option<QueryIntent>> {
        if query_lower.contains("what calls") || query_lower.contains("who calls") {
            let target = self.extract_symbol_name(query, "calls")?;
            return Ok(Some(QueryIntent::FindDependencies {
                target,
                direction: DependencyDirection::Dependents,
            }));
        }

        if query_lower.contains("what does") && query_lower.contains("call") {
            let target = self.extract_symbol_name(query, "does")?;
            return Ok(Some(QueryIntent::FindDependencies {
                target,
                direction: DependencyDirection::Dependencies,
            }));
        }

        if query_lower.contains("depends on") {
            let target = self.extract_symbol_name(query, "on")?;
            return Ok(Some(QueryIntent::FindDependencies {
                target,
                direction: DependencyDirection::Both,
            }));
        }

        Ok(None)
    }

    /// Detect symbol-based queries
    fn detect_symbol_query(&self, query_lower: &str) -> Result<Option<QueryIntent>> {
        if query_lower.contains("function") || query_lower.contains("fn") {
            let name_pattern = self
                .extract_symbol_pattern(query_lower)?
                .unwrap_or_else(|| "*".to_string());
            let fuzzy = !name_pattern.contains('*');
            return Ok(Some(QueryIntent::FindSymbols {
                symbol_types: Some(vec![SymbolType::Function, SymbolType::Method]),
                name_pattern,
                fuzzy,
            }));
        }

        if query_lower.contains("struct") {
            let name_pattern = self
                .extract_symbol_pattern(query_lower)?
                .unwrap_or_else(|| "*".to_string());
            let fuzzy = !name_pattern.contains('*');
            return Ok(Some(QueryIntent::FindSymbols {
                symbol_types: Some(vec![SymbolType::Struct]),
                name_pattern,
                fuzzy,
            }));
        }

        if query_lower.contains("class") {
            let name_pattern = self
                .extract_symbol_pattern(query_lower)?
                .unwrap_or_else(|| "*".to_string());
            let fuzzy = !name_pattern.contains('*');
            return Ok(Some(QueryIntent::FindSymbols {
                symbol_types: Some(vec![SymbolType::Class]),
                name_pattern,
                fuzzy,
            }));
        }

        if query_lower.contains("interface") {
            let name_pattern = self
                .extract_symbol_pattern(query_lower)?
                .unwrap_or_else(|| "*".to_string());
            let fuzzy = !name_pattern.contains('*');
            return Ok(Some(QueryIntent::FindSymbols {
                symbol_types: Some(vec![SymbolType::Interface]),
                name_pattern,
                fuzzy,
            }));
        }

        Ok(None)
    }

    /// Execute a parsed query intent
    #[instrument(skip(self))]
    pub async fn execute_intent(&self, intent: &QueryIntent) -> Result<NaturalLanguageQueryResult> {
        match intent {
            QueryIntent::FindSymbols {
                symbol_types,
                name_pattern,
                fuzzy,
            } => {
                self.execute_symbol_search(symbol_types.as_ref(), name_pattern, *fuzzy)
                    .await
            }
            QueryIntent::FindPatterns { pattern, scope } => {
                self.execute_pattern_search(pattern, scope).await
            }
            QueryIntent::FindDependencies { target, direction } => {
                self.execute_dependency_search(target, direction).await
            }
            QueryIntent::FindSignature { pattern, language } => {
                self.execute_signature_search(pattern, language.as_deref())
                    .await
            }
            QueryIntent::Combined { queries, operator } => {
                self.execute_combined_query(queries, operator).await
            }
            QueryIntent::FindRelationships { query_result } => {
                // Relationship query was already executed during parsing
                Ok(NaturalLanguageQueryResult {
                    intent: intent.clone(),
                    results: vec![], // We'll return the relationship result directly
                    explanation: query_result.summary.clone(),
                })
            }
        }
    }

    /// Execute a symbol search
    async fn execute_symbol_search(
        &self,
        symbol_types: Option<&Vec<SymbolType>>,
        name_pattern: &str,
        fuzzy: bool,
    ) -> Result<NaturalLanguageQueryResult> {
        let symbol_index = self
            .symbol_index
            .as_ref()
            .context("Symbol index not available")?;

        let code_query = CodeQuery::SymbolSearch {
            name: name_pattern.to_string(),
            symbol_types: symbol_types.cloned(),
            fuzzy,
        };

        let results = symbol_index.search_code(&code_query).await?;
        let result_count = results.len();

        Ok(NaturalLanguageQueryResult {
            intent: QueryIntent::FindSymbols {
                symbol_types: symbol_types.cloned(),
                name_pattern: name_pattern.to_string(),
                fuzzy,
            },
            results: results.into_iter().map(QueryResult::Symbol).collect(),
            explanation: format!(
                "Found {} symbols{} matching '{}'",
                result_count,
                symbol_types
                    .as_ref()
                    .map(|types| format!(" of type {:?}", types))
                    .unwrap_or_default(),
                name_pattern
            ),
        })
    }

    /// Execute a pattern search
    async fn execute_pattern_search(
        &self,
        pattern: &CodePattern,
        scope: &SearchScope,
    ) -> Result<NaturalLanguageQueryResult> {
        let symbol_index = self
            .symbol_index
            .as_ref()
            .context("Symbol index not available")?;

        let code_query = CodeQuery::PatternSearch {
            pattern: pattern.clone(),
            scope: scope.clone(),
        };

        let results = symbol_index.search_code(&code_query).await?;
        let result_count = results.len();

        Ok(NaturalLanguageQueryResult {
            intent: QueryIntent::FindPatterns {
                pattern: pattern.clone(),
                scope: scope.clone(),
            },
            results: results.into_iter().map(QueryResult::Symbol).collect(),
            explanation: format!(
                "Found {} instances of {:?} patterns in {:?}",
                result_count, pattern, scope
            ),
        })
    }

    /// Execute a dependency search
    async fn execute_dependency_search(
        &self,
        target: &str,
        direction: &DependencyDirection,
    ) -> Result<NaturalLanguageQueryResult> {
        let symbol_index = self
            .symbol_index
            .as_ref()
            .context("Symbol index not available")?;

        let code_query = CodeQuery::DependencySearch {
            target: target.to_string(),
            direction: direction.clone(),
        };

        let results = symbol_index.search_code(&code_query).await?;
        let result_count = results.len();

        Ok(NaturalLanguageQueryResult {
            intent: QueryIntent::FindDependencies {
                target: target.to_string(),
                direction: direction.clone(),
            },
            results: results.into_iter().map(QueryResult::Symbol).collect(),
            explanation: format!("Found {} {:?} for '{}'", result_count, direction, target),
        })
    }

    /// Execute a signature search
    async fn execute_signature_search(
        &self,
        pattern: &str,
        language: Option<&str>,
    ) -> Result<NaturalLanguageQueryResult> {
        let symbol_index = self
            .symbol_index
            .as_ref()
            .context("Symbol index not available")?;

        let code_query = CodeQuery::SignatureSearch {
            pattern: pattern.to_string(),
            language: language.map(String::from),
        };

        let results = symbol_index.search_code(&code_query).await?;
        let result_count = results.len();

        Ok(NaturalLanguageQueryResult {
            intent: QueryIntent::FindSignature {
                pattern: pattern.to_string(),
                language: language.map(String::from),
            },
            results: results.into_iter().map(QueryResult::Symbol).collect(),
            explanation: format!("Found {} signatures matching '{}'", result_count, pattern),
        })
    }

    /// Execute a combined query
    async fn execute_combined_query(
        &self,
        queries: &[CodeQuery],
        operator: &QueryOperator,
    ) -> Result<NaturalLanguageQueryResult> {
        let symbol_index = self
            .symbol_index
            .as_ref()
            .context("Symbol index not available")?;

        let code_query = CodeQuery::Combined {
            queries: queries.to_vec(),
            operator: operator.clone(),
        };

        let results = symbol_index.search_code(&code_query).await?;
        let result_count = results.len();

        Ok(NaturalLanguageQueryResult {
            intent: QueryIntent::Combined {
                queries: queries.to_vec(),
                operator: operator.clone(),
            },
            results: results.into_iter().map(QueryResult::Symbol).collect(),
            explanation: format!(
                "Combined {} queries with {:?} operator, found {} results",
                queries.len(),
                operator,
                result_count
            ),
        })
    }

    /// Extract symbol name from query
    fn extract_symbol_name(&self, query: &str, keyword: &str) -> Result<String> {
        // Find keyword case-insensitively but preserve original case
        let query_lower = query.to_lowercase();
        if let Some(pos) = query_lower.find(keyword) {
            let after_keyword = &query[pos + keyword.len()..];
            let name = after_keyword
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
        anyhow::bail!(
            "Could not extract symbol name from query '{}' after keyword '{}'",
            query,
            keyword
        )
    }

    /// Extract symbol pattern from query
    fn extract_symbol_pattern(&self, query: &str) -> Result<Option<String>> {
        // Look for quoted strings first
        if let Some(start) = query.find('"') {
            if let Some(end) = query[start + 1..].find('"') {
                let pattern = &query[start + 1..start + 1 + end];
                return Ok(Some(pattern.to_string()));
            }
        }

        // Look for backtick-quoted strings
        if let Some(start) = query.find('`') {
            if let Some(end) = query[start + 1..].find('`') {
                let pattern = &query[start + 1..start + 1 + end];
                return Ok(Some(pattern.to_string()));
            }
        }

        // Try to extract pattern after keywords
        for keyword in &["named", "called", "matching"] {
            if let Some(pos) = query.find(keyword) {
                let after_keyword = &query[pos + keyword.len()..];
                let pattern = after_keyword.split_whitespace().next().map(|s| {
                    s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '*')
                });
                if let Some(p) = pattern {
                    if !p.is_empty() {
                        return Ok(Some(p.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract signature pattern from query
    fn extract_signature_pattern(&self, query: &str) -> Result<String> {
        // Look for quoted strings or patterns after "with"
        if let Some(pos) = query.find("with") {
            let pattern = query[pos + "with".len()..].trim();
            if !pattern.is_empty() {
                return Ok(pattern.to_string());
            }
        }

        // Look for quoted patterns
        if let Some(start) = query.find('"') {
            if let Some(end) = query[start + 1..].find('"') {
                let pattern = &query[start + 1..start + 1 + end];
                return Ok(pattern.to_string());
            }
        }

        anyhow::bail!(
            "Could not extract signature pattern from query: '{}'",
            query
        )
    }
}

/// Result of a natural language query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageQueryResult {
    /// The parsed intent
    pub intent: QueryIntent,
    /// The results
    pub results: Vec<QueryResult>,
    /// Human-readable explanation
    pub explanation: String,
}

/// Individual query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    /// Symbol search result
    Symbol(SymbolSearchResult),
    /// Document result
    Document {
        id: ValidatedDocumentId,
        path: ValidatedPath,
        relevance: f32,
    },
}

/// Format results for LLM consumption
impl NaturalLanguageQueryResult {
    /// Format as markdown for display
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("## {}\n\n", self.explanation));

        for (i, result) in self.results.iter().enumerate() {
            match result {
                QueryResult::Symbol(symbol) => {
                    output.push_str(&format!(
                        "{}. **{}** (`{:?}`)\n",
                        i + 1,
                        symbol.symbol_name,
                        symbol.symbol_type
                    ));
                    output.push_str(&format!("   - File: `{}`\n", symbol.file_path.display()));
                    output.push_str(&format!("   - Relevance: {:.2}\n", symbol.relevance));
                    if !symbol.qualified_name.is_empty() {
                        output.push_str(&format!("   - Qualified: `{}`\n", symbol.qualified_name));
                    }
                    output.push('\n');
                }
                QueryResult::Document {
                    path, relevance, ..
                } => {
                    output.push_str(&format!("{}. **{}**\n", i + 1, path));
                    output.push_str(&format!("   - Relevance: {:.2}\n", relevance));
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Format as JSON for programmatic consumption
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize query result to JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_error_handling_query() {
        let processor = NaturalLanguageQueryProcessor::new();
        let result = processor
            .parse_query("find all error handling patterns")
            .await
            .unwrap();

        match result {
            QueryIntent::FindPatterns { .. } => {
                // Test passes if we get FindPatterns intent
            }
            _ => panic!("Expected FindPatterns intent"),
        }
    }

    #[tokio::test]
    async fn test_parse_dependency_query() {
        let processor = NaturalLanguageQueryProcessor::new();

        let result = processor
            .parse_query("what calls FileStorage")
            .await
            .unwrap();
        match result {
            QueryIntent::FindDependencies { target, .. } => {
                assert_eq!(target, "FileStorage".to_string());
            }
            _ => panic!("Expected FindDependencies intent"),
        }
    }

    #[tokio::test]
    async fn test_parse_symbol_query() {
        let processor = NaturalLanguageQueryProcessor::new();

        let result = processor
            .parse_query("find function named create_storage")
            .await
            .unwrap();
        match result {
            QueryIntent::FindSymbols {
                symbol_types,
                name_pattern,
                ..
            } => {
                assert!(symbol_types.is_some());
                // Check that we have at least one symbol type
                assert!(!symbol_types.as_ref().unwrap().is_empty());
                assert!(!name_pattern.is_empty());
            }
            _ => panic!("Expected FindSymbols intent"),
        }
    }

    #[tokio::test]
    async fn test_extract_quoted_pattern() {
        let processor = NaturalLanguageQueryProcessor::new();

        let result = processor
            .parse_query("find functions matching \"validate_*\"")
            .await
            .unwrap();
        match result {
            QueryIntent::FindSymbols { name_pattern, .. } => {
                assert_eq!(name_pattern, "validate_*".to_string());
            }
            _ => panic!("Expected FindSymbols intent"),
        }
    }
}
