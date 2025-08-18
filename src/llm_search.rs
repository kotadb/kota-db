// LLM-Optimized Search Module
// Provides relevance ranking, context optimization, and structured output for LLM consumption

use crate::contracts::{Index, Query, Storage};
use crate::Document;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Configuration for relevance scoring algorithm
#[derive(Debug, Clone)]
pub struct RelevanceConfig {
    /// Weight for exact phrase matches (0.0-1.0)
    pub exact_match_weight: f32,
    /// Weight for term proximity in content (0.0-1.0)
    pub proximity_weight: f32,
    /// Weight for symbol importance (public APIs > private helpers) (0.0-1.0)
    pub symbol_importance_weight: f32,
    /// Weight for content freshness/recency (0.0-1.0)
    pub freshness_weight: f32,
}

impl Default for RelevanceConfig {
    fn default() -> Self {
        Self {
            exact_match_weight: 0.4,
            proximity_weight: 0.3,
            symbol_importance_weight: 0.2,
            freshness_weight: 0.1,
        }
    }
}

/// Configuration for context window optimization
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum tokens available for search results
    pub token_budget: usize,
    /// Include related symbols (callers, callees) when relevant
    pub include_related: bool,
    /// Strip non-essential comments to save space
    pub strip_comments: bool,
    /// Prefer complete functions over partial cuts
    pub prefer_complete_functions: bool,
    /// Maximum characters per content snippet
    pub max_snippet_chars: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            token_budget: 4000, // Conservative default for smaller LLMs
            include_related: true,
            strip_comments: false, // Keep comments by default for context
            prefer_complete_functions: true,
            max_snippet_chars: 500,
        }
    }
}

/// Details about where and how search terms matched in content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetails {
    /// Exact phrase matches found
    pub exact_matches: Vec<MatchLocation>,
    /// Individual term matches
    pub term_matches: Vec<MatchLocation>,
    /// Estimated match quality (0.0-1.0)
    pub match_quality: f32,
    /// Primary match type that contributed most to score
    pub primary_match_type: MatchType,
}

/// Location of a match within document content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchLocation {
    /// Character offset in content where match starts
    pub start_offset: usize,
    /// Character offset where match ends
    pub end_offset: usize,
    /// Context around the match (snippet)
    pub context: String,
    /// Type of content where match occurred
    pub context_type: ContextType,
}

/// Type of match found
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// Exact phrase match
    ExactPhrase,
    /// Multiple terms in proximity
    ProximityMatch,
    /// Single term match
    TermMatch,
    /// Semantic similarity match
    SemanticMatch,
}

/// Type of content context where match occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextType {
    /// Function or method name
    FunctionName,
    /// Type or struct name
    TypeName,
    /// Variable or field name
    VariableName,
    /// Comment or documentation
    Comment,
    /// String literal or text content
    TextContent,
    /// Code body/implementation
    CodeBody,
    /// Unknown or mixed context
    Unknown,
}

/// Information about related code context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInfo {
    /// Functions or methods that call this code
    pub callers: Vec<String>,
    /// Functions or methods called by this code
    pub callees: Vec<String>,
    /// Related types or interfaces
    pub related_types: Vec<String>,
    /// Estimated importance score (0.0-1.0)
    pub importance_score: f32,
}

/// A search result with relevance scoring and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSearchResult {
    /// Document metadata
    pub id: String,
    pub path: String,
    pub title: Option<String>,

    /// Relevance information
    pub relevance_score: f32,
    pub match_details: MatchDetails,

    /// Optimized content for LLM consumption
    pub content_snippet: String,
    pub estimated_tokens: usize,

    /// Context and metadata
    pub context_info: ContextInfo,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Token usage and optimization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Estimated tokens used by results
    pub estimated_tokens: usize,
    /// Token budget available
    pub budget: usize,
    /// Efficiency ratio (used/budget)
    pub efficiency: f32,
    /// Number of results that were truncated
    pub truncated_results: usize,
}

/// Strategy used for result selection and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// Return highest relevance scores
    HighestRelevance,
    /// Maximize diversity of result types
    MaximizeDiversity,
    /// Balance relevance with token efficiency
    BalancedOptimal,
}

/// Optimization metadata for the search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    /// Total matches found before filtering
    pub total_matches: usize,
    /// Number of results returned after optimization
    pub returned: usize,
    /// Strategy used for result selection
    pub selection_strategy: SelectionStrategy,
    /// Token usage statistics
    pub token_usage: TokenUsage,
}

/// Complete LLM-optimized search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSearchResponse {
    /// Original search query
    pub query: String,
    /// Optimization and selection information
    pub optimization: OptimizationInfo,
    /// Ranked and optimized search results
    pub results: Vec<LLMSearchResult>,
    /// Additional metadata and suggestions
    pub metadata: LLMResponseMetadata,
}

/// Additional response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponseMetadata {
    /// Query execution time in milliseconds
    pub query_time_ms: u64,
    /// Suggested follow-up queries
    pub suggestions: Vec<String>,
    /// Warnings or notices for the user
    pub warnings: Vec<String>,
}

/// Main LLM-optimized search engine
pub struct LLMSearchEngine {
    relevance_config: RelevanceConfig,
    context_config: ContextConfig,
}

impl LLMSearchEngine {
    /// Create a new LLM search engine with default configuration
    pub fn new() -> Self {
        Self {
            relevance_config: RelevanceConfig::default(),
            context_config: ContextConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(relevance_config: RelevanceConfig, context_config: ContextConfig) -> Self {
        Self {
            relevance_config,
            context_config,
        }
    }

    /// Perform LLM-optimized search
    pub async fn search_optimized(
        &self,
        query: &str,
        storage: &dyn Storage,
        trigram_index: &dyn Index,
        limit: Option<usize>,
    ) -> Result<LLMSearchResponse> {
        let start_time = Instant::now();
        let limit = limit.unwrap_or(10);

        // Validate query - handle empty queries with warnings instead of errors
        let query_trimmed = query.trim();
        let is_empty_query = query_trimmed.is_empty();

        info!("Starting LLM-optimized search for query: '{}'", query);

        // 1. Perform initial search using existing infrastructure
        let search_query = Query::new(Some(query.to_string()), None, None, limit * 3)?; // Get more for ranking
        let doc_ids = trigram_index
            .search(&search_query)
            .await
            .context("Failed to perform trigram search")?;

        debug!("Initial search found {} potential matches", doc_ids.len());

        // 2. Fetch documents and calculate relevance scores
        let mut scored_results = Vec::new();
        for doc_id in doc_ids {
            match storage.get(&doc_id).await? {
                Some(document) => {
                    let scored_result = self.score_document(&document, query).await?;
                    scored_results.push(scored_result);
                }
                None => {
                    warn!("Document {} not found in storage", doc_id.as_uuid());
                    continue;
                }
            }
        }

        // 3. Rank results by relevance score
        scored_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        // 4. Apply context optimization and token budgeting
        let optimized_results = self.optimize_for_context(&scored_results, limit).await?;

        // 5. Generate response metadata
        let query_time_ms = start_time.elapsed().as_millis() as u64;
        let suggestions = self.generate_suggestions(query, &optimized_results);
        let warnings = self.generate_warnings(&optimized_results, is_empty_query);

        let token_usage = self.calculate_token_usage(&optimized_results);

        let response = LLMSearchResponse {
            query: query.to_string(),
            optimization: OptimizationInfo {
                total_matches: scored_results.len(),
                returned: optimized_results.len(),
                selection_strategy: SelectionStrategy::HighestRelevance, // TODO: Make configurable
                token_usage,
            },
            results: optimized_results,
            metadata: LLMResponseMetadata {
                query_time_ms,
                suggestions,
                warnings,
            },
        };

        info!(
            "LLM search completed: {} results in {}ms ({}% token efficiency)",
            response.results.len(),
            query_time_ms,
            (response.optimization.token_usage.efficiency * 100.0) as u32
        );

        Ok(response)
    }

    /// Score a document for relevance to the search query
    async fn score_document(&self, document: &Document, query: &str) -> Result<LLMSearchResult> {
        // Convert document content to string for analysis
        let content = String::from_utf8_lossy(&document.content);
        let query_lower = query.to_lowercase();

        // Calculate different relevance factors
        let exact_match_score = self.calculate_exact_match_score(&content, &query_lower);
        let proximity_score = self.calculate_proximity_score(&content, &query_lower);
        let symbol_score = self.calculate_symbol_importance_score(&content, &query_lower);
        let freshness_score = 0.5; // TODO: Implement based on document metadata

        // Combine scores using weighted average
        let relevance_score = (exact_match_score * self.relevance_config.exact_match_weight)
            + (proximity_score * self.relevance_config.proximity_weight)
            + (symbol_score * self.relevance_config.symbol_importance_weight)
            + (freshness_score * self.relevance_config.freshness_weight);

        // Generate match details
        let match_details = self.analyze_matches(&content, &query_lower)?;

        // Extract context information
        let context_info = self.extract_context_info(&content)?;

        // Create optimized content snippet
        let content_snippet = self.create_optimized_snippet(&content, &query_lower)?;
        let estimated_tokens = self.estimate_token_count(&content_snippet);

        Ok(LLMSearchResult {
            id: document.id.as_uuid().to_string(),
            path: document.path.to_string(),
            title: Some(document.title.to_string()),
            relevance_score: relevance_score.clamp(0.0, 1.0),
            match_details,
            content_snippet,
            estimated_tokens,
            context_info,
            metadata: HashMap::new(),
        })
    }

    /// Calculate exact match score for the query
    fn calculate_exact_match_score(&self, content: &str, query: &str) -> f32 {
        let content_lower = content.to_lowercase();
        if content_lower.contains(query) {
            // Count occurrences and normalize by content length
            let matches = content_lower.matches(query).count();
            let content_len = content.len() as f32;
            (matches as f32 / content_len * 1000.0).min(1.0) // Scale appropriately
        } else {
            0.0
        }
    }

    /// Calculate proximity score (how close query terms are to each other)
    fn calculate_proximity_score(&self, content: &str, query: &str) -> f32 {
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.len() < 2 {
            return 0.0; // No proximity for single terms
        }

        let content_lower = content.to_lowercase();
        let mut best_proximity: f32 = 0.0;

        // Find the closest occurrence of all terms
        for (i, term) in terms.iter().enumerate() {
            if let Some(pos) = content_lower.find(term) {
                // Look for other terms nearby
                let mut proximity_score = 1.0;
                for (j, other_term) in terms.iter().enumerate() {
                    if i == j {
                        continue;
                    }

                    // Search in a window around this term
                    let window_start = pos.saturating_sub(100);
                    let window_end = (pos + 200).min(content_lower.len());
                    let window = &content_lower[window_start..window_end];

                    if window.contains(other_term) {
                        proximity_score += 0.5;
                    }
                }
                best_proximity = best_proximity.max(proximity_score);
            }
        }

        (best_proximity / terms.len() as f32).min(1.0)
    }

    /// Calculate symbol importance score (public APIs > private helpers)
    fn calculate_symbol_importance_score(&self, content: &str, query: &str) -> f32 {
        // Simple heuristic: look for query terms in important contexts
        let content_lower = content.to_lowercase();
        let mut importance: f32 = 0.0;

        // Higher score for matches in function signatures
        if content_lower.contains(&format!("fn {}", query))
            || content_lower.contains(&format!("pub fn {}", query))
        {
            importance += 0.8;
        }

        // Higher score for pub items
        if content_lower.contains("pub") && content_lower.contains(query) {
            importance += 0.6;
        }

        // Medium score for struct/enum names
        if content_lower.contains(&format!("struct {}", query))
            || content_lower.contains(&format!("enum {}", query))
        {
            importance += 0.4;
        }

        importance.min(1.0)
    }

    /// Analyze match details for the query in content
    fn analyze_matches(&self, content: &str, query: &str) -> Result<MatchDetails> {
        let content_lower = content.to_lowercase();
        let mut exact_matches = Vec::new();
        let mut term_matches = Vec::new();

        // Find exact phrase matches
        let mut start = 0;
        while let Some(pos) = content_lower[start..].find(query) {
            let absolute_pos = start + pos;
            let match_location = MatchLocation {
                start_offset: absolute_pos,
                end_offset: absolute_pos + query.len(),
                context: self.extract_match_context(content, absolute_pos, query.len())?,
                context_type: self.determine_context_type(content, absolute_pos)?,
            };
            exact_matches.push(match_location);
            start = absolute_pos + query.len();
        }

        // Find individual term matches
        for term in query.split_whitespace() {
            let mut start = 0;
            while let Some(pos) = content_lower[start..].find(term) {
                let absolute_pos = start + pos;
                let match_location = MatchLocation {
                    start_offset: absolute_pos,
                    end_offset: absolute_pos + term.len(),
                    context: self.extract_match_context(content, absolute_pos, term.len())?,
                    context_type: self.determine_context_type(content, absolute_pos)?,
                };
                term_matches.push(match_location);
                start = absolute_pos + term.len();

                // Limit term matches to avoid excessive data
                if term_matches.len() >= 10 {
                    break;
                }
            }
        }

        let match_quality = if !exact_matches.is_empty() {
            0.9 // High quality for exact matches
        } else if !term_matches.is_empty() {
            0.6 // Medium quality for term matches
        } else {
            0.1 // Low quality fallback
        };

        let primary_match_type = if !exact_matches.is_empty() {
            MatchType::ExactPhrase
        } else if term_matches.len() > 1 {
            MatchType::ProximityMatch
        } else {
            MatchType::TermMatch
        };

        Ok(MatchDetails {
            exact_matches,
            term_matches,
            match_quality,
            primary_match_type,
        })
    }

    /// Extract context around a match
    fn extract_match_context(&self, content: &str, pos: usize, len: usize) -> Result<String> {
        let context_size = 50; // Characters before and after
        let start = pos.saturating_sub(context_size);
        let end = (pos + len + context_size).min(content.len());

        let context = &content[start..end];
        Ok(format!("...{}...", context.trim()))
    }

    /// Determine the context type where a match occurred
    fn determine_context_type(&self, _content: &str, _pos: usize) -> Result<ContextType> {
        // TODO: Implement proper context analysis
        // For now, return unknown - this would analyze surrounding code structure
        Ok(ContextType::Unknown)
    }

    /// Extract context information about related code
    fn extract_context_info(&self, _content: &str) -> Result<ContextInfo> {
        // TODO: Implement proper code analysis
        // For now, return empty context info
        Ok(ContextInfo {
            callers: Vec::new(),
            callees: Vec::new(),
            related_types: Vec::new(),
            importance_score: 0.5,
        })
    }

    /// Create an optimized content snippet for LLM consumption
    fn create_optimized_snippet(&self, content: &str, query: &str) -> Result<String> {
        let max_chars = self.context_config.max_snippet_chars;

        // If content is short enough, return as-is
        if content.len() <= max_chars {
            return Ok(content.to_string());
        }

        // Find the best section that includes query matches
        let content_lower = content.to_lowercase();
        if let Some(match_pos) = content_lower.find(query) {
            // Center the snippet around the first match
            let start = match_pos.saturating_sub(max_chars / 2);
            let end = (start + max_chars).min(content.len());

            let snippet = &content[start..end];

            // Try to break at word boundaries
            let trimmed = if start > 0 && end < content.len() {
                format!("...{}...", snippet.trim())
            } else if start > 0 {
                format!("...{}", snippet.trim())
            } else if end < content.len() {
                format!("{}...", snippet.trim())
            } else {
                snippet.to_string()
            };

            Ok(trimmed)
        } else {
            // No match found, return beginning of content
            let snippet = &content[..max_chars.min(content.len())];
            Ok(format!("{}...", snippet.trim()))
        }
    }

    /// Estimate token count for content (rough approximation)
    fn estimate_token_count(&self, content: &str) -> usize {
        // Rough approximation: 1 token ≈ 4 characters for English text
        // More sophisticated tokenization could be added later
        content.len().div_ceil(4)
    }

    /// Optimize results for context window constraints
    async fn optimize_for_context(
        &self,
        results: &[LLMSearchResult],
        limit: usize,
    ) -> Result<Vec<LLMSearchResult>> {
        let mut optimized = Vec::new();
        let mut total_tokens = 0;

        for result in results.iter().take(limit * 2) {
            // Consider more than limit for selection
            if total_tokens + result.estimated_tokens <= self.context_config.token_budget {
                total_tokens += result.estimated_tokens;
                optimized.push(result.clone());

                if optimized.len() >= limit {
                    break;
                }
            } else if optimized.len() < limit / 2 {
                // If we haven't gotten enough results, try to compress this one
                let compressed = self.compress_result(result).await?;
                if total_tokens + compressed.estimated_tokens <= self.context_config.token_budget {
                    total_tokens += compressed.estimated_tokens;
                    optimized.push(compressed);
                }
            }
        }

        Ok(optimized)
    }

    /// Compress a search result to fit within token constraints
    async fn compress_result(&self, result: &LLMSearchResult) -> Result<LLMSearchResult> {
        // Create a compressed version with shorter snippet
        let compressed_snippet = if result.content_snippet.len() > 200 {
            format!("{}...", &result.content_snippet[..200])
        } else {
            result.content_snippet.clone()
        };

        let estimated_tokens = self.estimate_token_count(&compressed_snippet);

        let mut compressed = result.clone();
        compressed.content_snippet = compressed_snippet;
        compressed.estimated_tokens = estimated_tokens;

        Ok(compressed)
    }

    /// Calculate total token usage for results
    fn calculate_token_usage(&self, results: &[LLMSearchResult]) -> TokenUsage {
        let estimated_tokens: usize = results.iter().map(|r| r.estimated_tokens).sum();
        let budget = self.context_config.token_budget;
        let efficiency = estimated_tokens as f32 / budget as f32;

        TokenUsage {
            estimated_tokens,
            budget,
            efficiency: efficiency.min(1.0),
            truncated_results: 0, // TODO: Track actual truncations
        }
    }

    /// Generate helpful suggestions based on query and results
    fn generate_suggestions(&self, query: &str, results: &[LLMSearchResult]) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Basic suggestions based on query content
        if query.contains("error") || query.contains("Error") {
            suggestions.push("Show error recovery patterns".to_string());
            suggestions.push("Find error handling best practices".to_string());
        }

        if query.contains("test") || query.contains("Test") {
            suggestions.push("Show related test patterns".to_string());
            suggestions.push("Find testing utilities".to_string());
        }

        // Suggestions based on results
        if results.len() > 10 {
            suggestions.push("Narrow search with more specific terms".to_string());
        } else if results.is_empty() {
            suggestions.push("Try broader search terms".to_string());
            suggestions.push("Check for typos in query".to_string());
        }

        suggestions
    }

    /// Generate warnings for the user
    fn generate_warnings(&self, results: &[LLMSearchResult], is_empty_query: bool) -> Vec<String> {
        let mut warnings = Vec::new();

        // Warning for empty queries
        if is_empty_query {
            warnings.push(
                "Search query is empty. Consider providing specific search terms.".to_string(),
            );
        }

        let total_tokens: usize = results.iter().map(|r| r.estimated_tokens).sum();
        if total_tokens > self.context_config.token_budget {
            warnings.push(format!(
                "Results exceed token budget ({} > {}). Some content may be truncated.",
                total_tokens, self.context_config.token_budget
            ));
        }

        if results.len() >= 50 {
            warnings.push("Large result set returned. Consider refining query.".to_string());
        }

        warnings
    }
}

impl Default for LLMSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests are in tests/llm_search_test.rs following KotaDB's integration test pattern
    // This module is reserved for unit tests of internal functions
}
