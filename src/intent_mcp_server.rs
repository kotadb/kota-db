//! Intent-Based MCP Server Implementation
//!
//! Transforms natural language queries into orchestrated API calls for AI assistants.
//! Provides a clean, conversational interface over KotaDB's technical capabilities.
//!
//! Issue #645: Intent-Based MCP Server: Transform Raw API Exposure to Natural Language Interface

use anyhow::Result;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use url::Url;

/// Intent categories recognized by the parser
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    /// Search for code, symbols, or content
    Search {
        query: String,
        scope: SearchScope,
        context: SearchContext,
    },
    /// Analyze code impact, dependencies, or relationships
    Analysis {
        target: String,
        analysis_type: AnalysisType,
        depth: u32,
    },
    /// Navigate code structure and relationships
    Navigation {
        path: String,
        context: NavigationContext,
    },
    /// Get overview or summary information
    Overview {
        focus: Option<String>,
        detail_level: DetailLevel,
    },
    /// Debug or troubleshoot issues
    Debugging {
        error: String,
        context: Option<String>,
    },
}

/// Search scope specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchScope {
    Code,
    Symbols,
    Functions,
    Classes,
    Variables,
    Files,
    All,
}

/// Search context for better results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchContext {
    pub language: Option<String>,
    pub file_type: Option<String>,
    pub module: Option<String>,
}

/// Type of analysis to perform
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisType {
    Impact,
    Dependencies,
    Callers,
    Callees,
    Usage,
    Relationships,
}

/// Navigation context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NavigationContext {
    Implementation,
    Definition,
    Usage,
    Related,
}

/// Level of detail for overview responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetailLevel {
    Summary,
    Detailed,
    Comprehensive,
}

/// Configuration for the intent-based MCP server
#[derive(Debug, Clone)]
pub struct IntentMcpConfig {
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub max_results: usize,
    pub default_timeout_ms: u64,
}

impl Default for IntentMcpConfig {
    fn default() -> Self {
        Self {
            api_base_url: "http://localhost:8080".to_string(),
            api_key: None,
            max_results: 20,
            default_timeout_ms: 30000,
        }
    }
}

/// Intent-based MCP server
pub struct IntentMcpServer {
    #[allow(dead_code)]
    config: IntentMcpConfig,
    intent_parser: IntentParser,
    orchestrator: QueryOrchestrator,
    context_manager: ContextManager,
    #[allow(dead_code)]
    http_client: Client,
}

/// Parses natural language queries into structured intents
pub struct IntentParser {
    search_patterns: Vec<(Regex, SearchScope)>,
    analysis_patterns: Vec<(Regex, AnalysisType)>,
    navigation_patterns: Vec<(Regex, NavigationContext)>,
}

/// Orchestrates API calls based on parsed intents
pub struct QueryOrchestrator {
    base_url: Url,
    client: Client,
}

/// Manages conversation context and state
pub struct ContextManager {
    conversations: Arc<RwLock<HashMap<String, ConversationContext>>>,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for a conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub session_id: String,
    pub previous_queries: Vec<String>,
    pub current_focus: Option<String>,
    pub language_hints: Vec<String>,
    pub last_results: Option<serde_json::Value>,
}

/// Response from the intent-based MCP server
#[derive(Debug, Serialize, Deserialize)]
pub struct IntentResponse {
    pub intent: Intent,
    pub results: serde_json::Value,
    pub summary: String,
    pub suggestions: Vec<String>,
    pub query_time_ms: u64,
}

impl IntentMcpServer {
    /// Create a new intent-based MCP server
    pub fn new(config: IntentMcpConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.default_timeout_ms))
            .build()?;

        let base_url = Url::parse(&config.api_base_url)?;

        Ok(Self {
            intent_parser: IntentParser::new(),
            orchestrator: QueryOrchestrator::new(base_url, http_client.clone()),
            context_manager: ContextManager::new(),
            http_client,
            config,
        })
    }

    /// Process a natural language query
    #[instrument(skip(self, session_id))]
    pub async fn process_query(&self, query: &str, session_id: &str) -> Result<IntentResponse> {
        let start_time = std::time::Instant::now();

        // Parse the natural language query into intent
        let intent = self.intent_parser.parse(query).await?;
        info!("Parsed intent: {:?}", intent);

        // Get conversation context
        let context = self.context_manager.get_context(session_id).await;

        // Orchestrate API calls based on intent
        let results = self.orchestrator.execute_intent(&intent, &context).await?;

        // Generate summary and suggestions
        let summary = self.generate_summary(&intent, &results);
        let suggestions = self.generate_suggestions(&intent, &context);

        // Update conversation context
        self.context_manager
            .update_context(session_id, query, &intent, &results)
            .await;

        Ok(IntentResponse {
            intent,
            results,
            summary,
            suggestions,
            query_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Generate a human-readable summary of results
    fn generate_summary(&self, intent: &Intent, results: &serde_json::Value) -> String {
        match intent {
            Intent::Search { query, scope, .. } => {
                let count = results.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                format!(
                    "Found {} results for {} search: \"{}\"",
                    count,
                    format!("{:?}", scope).to_lowercase(),
                    query
                )
            }
            Intent::Analysis {
                target,
                analysis_type,
                ..
            } => {
                format!(
                    "Completed {} analysis for \"{}\"",
                    format!("{:?}", analysis_type).to_lowercase(),
                    target
                )
            }
            Intent::Navigation { path, .. } => {
                format!("Navigated to \"{}\"", path)
            }
            Intent::Overview { focus, .. } => match focus {
                Some(f) => format!("Generated overview for \"{}\"", f),
                None => "Generated codebase overview".to_string(),
            },
            Intent::Debugging { error, .. } => {
                format!("Analyzed debugging context for \"{}\"", error)
            }
        }
    }

    /// Generate follow-up suggestions based on current intent and context
    fn generate_suggestions(
        &self,
        intent: &Intent,
        context: &Option<ConversationContext>,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        match intent {
            Intent::Search { query, scope, .. } => {
                suggestions.push("Show me how this is implemented".to_string());
                suggestions.push("Find who calls this function".to_string());
                if *scope != SearchScope::Symbols {
                    suggestions.push("Search for related symbols".to_string());
                }
            }
            Intent::Analysis {
                target,
                analysis_type,
                ..
            } => match analysis_type {
                AnalysisType::Impact => {
                    suggestions.push(format!("Show the implementation of {}", target));
                    suggestions.push("What are the dependencies?".to_string());
                }
                AnalysisType::Callers => {
                    suggestions.push("Show me the implementation".to_string());
                    suggestions.push("What does this function do?".to_string());
                }
                _ => {
                    suggestions.push("Analyze the impact of changes".to_string());
                    suggestions.push("Show related functions".to_string());
                }
            },
            Intent::Overview { .. } => {
                suggestions.push("Search for specific functionality".to_string());
                suggestions.push("Analyze key components".to_string());
                suggestions.push("Show me the main entry points".to_string());
            }
            _ => {
                suggestions.push("Search the codebase".to_string());
                suggestions.push("Get an overview".to_string());
            }
        }

        // Add context-aware suggestions
        if let Some(ctx) = context {
            if let Some(focus) = &ctx.current_focus {
                suggestions.push(format!("Learn more about {}", focus));
            }
        }

        suggestions
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentParser {
    pub fn new() -> Self {
        let search_patterns = vec![
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*function").unwrap(),
                SearchScope::Functions,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*class").unwrap(),
                SearchScope::Classes,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*variable").unwrap(),
                SearchScope::Variables,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*symbol").unwrap(),
                SearchScope::Symbols,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*file").unwrap(),
                SearchScope::Files,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)\s+.*code").unwrap(),
                SearchScope::Code,
            ),
            (
                Regex::new(r"(?i)\b(find|search|look\s+for|locate)").unwrap(),
                SearchScope::All,
            ),
        ];

        let analysis_patterns = vec![
            (
                Regex::new(r"(?i)\b(impact|affect|change|break)").unwrap(),
                AnalysisType::Impact,
            ),
            (
                Regex::new(r"(?i)\b(who\s+calls|what\s+calls|callers)").unwrap(),
                AnalysisType::Callers,
            ),
            (
                Regex::new(r"(?i)\b(calls\s+what|what.*calls|callees)").unwrap(),
                AnalysisType::Callees,
            ),
            (
                Regex::new(r"(?i)\b(depend|dependenc)").unwrap(),
                AnalysisType::Dependencies,
            ),
            (
                Regex::new(r"(?i)\b(usage|used|how.*used)").unwrap(),
                AnalysisType::Usage,
            ),
            (
                Regex::new(r"(?i)\b(relation|connect|link)").unwrap(),
                AnalysisType::Relationships,
            ),
        ];

        let navigation_patterns = vec![
            (
                Regex::new(r"(?i)\b(show.*implement|implementation)").unwrap(),
                NavigationContext::Implementation,
            ),
            (
                Regex::new(r"(?i)\b(definition|define|declared)").unwrap(),
                NavigationContext::Definition,
            ),
            (
                Regex::new(r"(?i)\b(usage|used|example)").unwrap(),
                NavigationContext::Usage,
            ),
            (
                Regex::new(r"(?i)\b(related|similar|connect)").unwrap(),
                NavigationContext::Related,
            ),
        ];

        Self {
            search_patterns,
            analysis_patterns,
            navigation_patterns,
        }
    }

    /// Parse natural language query into structured intent
    pub async fn parse(&self, query: &str) -> Result<Intent> {
        let query_lower = query.to_lowercase();

        // Check for overview requests
        if query_lower.contains("overview")
            || query_lower.contains("summary")
            || query_lower.contains("structure")
            || query_lower.contains("architecture")
        {
            let focus = self.extract_focus_from_query(query);
            return Ok(Intent::Overview {
                focus,
                detail_level: if query_lower.contains("detail") {
                    DetailLevel::Detailed
                } else if query_lower.contains("comprehensive") || query_lower.contains("complete")
                {
                    DetailLevel::Comprehensive
                } else {
                    DetailLevel::Summary
                },
            });
        }

        // Check for debugging requests
        if query_lower.contains("debug")
            || query_lower.contains("error")
            || query_lower.contains("problem")
            || query_lower.contains("issue")
        {
            let error = self.extract_error_from_query(query);
            let context = self.extract_context_from_query(query);
            return Ok(Intent::Debugging { error, context });
        }

        // Check for analysis patterns
        for (pattern, analysis_type) in &self.analysis_patterns {
            if pattern.is_match(&query_lower) {
                let target = self.extract_target_from_query(query);
                return Ok(Intent::Analysis {
                    target,
                    analysis_type: analysis_type.clone(),
                    depth: if query_lower.contains("deep") || query_lower.contains("detailed") {
                        3
                    } else {
                        1
                    },
                });
            }
        }

        // Check for navigation patterns
        for (pattern, nav_context) in &self.navigation_patterns {
            if pattern.is_match(&query_lower) {
                let path = self.extract_path_from_query(query);
                return Ok(Intent::Navigation {
                    path,
                    context: nav_context.clone(),
                });
            }
        }

        // Check for search patterns (default)
        for (pattern, scope) in &self.search_patterns {
            if pattern.is_match(&query_lower) {
                let search_query = self.extract_search_query(query);
                let context = self.extract_search_context(query);
                return Ok(Intent::Search {
                    query: search_query,
                    scope: scope.clone(),
                    context,
                });
            }
        }

        // Default to general search
        Ok(Intent::Search {
            query: query.to_string(),
            scope: SearchScope::All,
            context: SearchContext {
                language: None,
                file_type: None,
                module: None,
            },
        })
    }

    fn extract_search_query(&self, query: &str) -> String {
        // Remove common search prefixes and extract the actual search term
        let cleaned = Regex::new(r"(?i)^(find|search|look\s+for|locate)\s+")
            .unwrap()
            .replace(query, "");

        cleaned.trim().to_string()
    }

    fn extract_target_from_query(&self, query: &str) -> String {
        // Extract the target function/symbol name from queries like "who calls validate_path"
        let words: Vec<&str> = query.split_whitespace().collect();

        // Look for function-like patterns
        for (i, word) in words.iter().enumerate() {
            if word.contains("(")
                || (i > 0 && (words[i - 1] == "function" || words[i - 1] == "calls"))
                || word.chars().any(|c| c.is_uppercase())
            {
                return word
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_string();
            }
        }

        // Fallback: take the last meaningful word
        words
            .last()
            .unwrap_or(&"")
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_string()
    }

    fn extract_path_from_query(&self, query: &str) -> String {
        // Extract file paths or symbol names for navigation
        self.extract_target_from_query(query)
    }

    fn extract_focus_from_query(&self, query: &str) -> Option<String> {
        // Extract specific focus area for overview requests
        let words: Vec<&str> = query.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            if *word == "of" && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }

        None
    }

    fn extract_error_from_query(&self, query: &str) -> String {
        // Extract error message or problem description
        if let Some(start) = query.find("error") {
            query[start..].to_string()
        } else {
            query.to_string()
        }
    }

    fn extract_context_from_query(&self, query: &str) -> Option<String> {
        // Extract additional context for debugging
        if query.contains("in") {
            let parts: Vec<&str> = query.split("in").collect();
            if parts.len() > 1 {
                return Some(parts[1].trim().to_string());
            }
        }
        None
    }

    fn extract_search_context(&self, query: &str) -> SearchContext {
        let query_lower = query.to_lowercase();

        let language = if query_lower.contains("rust") {
            Some("rust".to_string())
        } else if query_lower.contains("python") {
            Some("python".to_string())
        } else if query_lower.contains("javascript") || query_lower.contains("js") {
            Some("javascript".to_string())
        } else {
            None
        };

        let file_type = if query_lower.contains(".rs") {
            Some("rs".to_string())
        } else if query_lower.contains(".py") {
            Some("py".to_string())
        } else if query_lower.contains(".js") {
            Some("js".to_string())
        } else {
            None
        };

        SearchContext {
            language,
            file_type,
            module: None, // TODO: Extract module hints
        }
    }
}

impl QueryOrchestrator {
    pub fn new(base_url: Url, client: Client) -> Self {
        Self { base_url, client }
    }

    /// Execute an intent by making appropriate API calls
    #[instrument(skip(self, context))]
    pub async fn execute_intent(
        &self,
        intent: &Intent,
        context: &Option<ConversationContext>,
    ) -> Result<serde_json::Value> {
        match intent {
            Intent::Search { query, scope, .. } => self.execute_search(query, scope).await,
            Intent::Analysis {
                target,
                analysis_type,
                ..
            } => self.execute_analysis(target, analysis_type).await,
            Intent::Navigation {
                path,
                context: nav_context,
            } => self.execute_navigation(path, nav_context).await,
            Intent::Overview {
                focus,
                detail_level,
            } => self.execute_overview(focus.as_deref(), detail_level).await,
            Intent::Debugging {
                error,
                context: debug_context,
            } => {
                self.execute_debugging(error, debug_context.as_deref())
                    .await
            }
        }
    }

    async fn execute_search(&self, query: &str, scope: &SearchScope) -> Result<serde_json::Value> {
        let endpoint = match scope {
            SearchScope::Code => "/api/code/search",
            SearchScope::Symbols
            | SearchScope::Functions
            | SearchScope::Classes
            | SearchScope::Variables => "/api/symbols/search",
            SearchScope::Files => "/api/code/search", // Files are found through code search
            SearchScope::All => "/api/code/search",   // Default to code search for "all"
        };

        let mut url = self.base_url.join(endpoint)?;
        url.query_pairs_mut().append_pair("q", query);

        let response = self.client.get(url).send().await?;
        let result: serde_json::Value = response.json().await?;

        Ok(result)
    }

    async fn execute_analysis(
        &self,
        target: &str,
        analysis_type: &AnalysisType,
    ) -> Result<serde_json::Value> {
        let endpoint = match analysis_type {
            AnalysisType::Impact => format!("/api/analysis/impact/{}", target),
            AnalysisType::Callers => format!("/api/relationships/callers/{}", target),
            AnalysisType::Dependencies => format!("/api/analysis/dependencies/{}", target),
            _ => format!("/api/relationships/callers/{}", target), // Default to callers
        };

        let url = self.base_url.join(&endpoint)?;
        let response = self.client.get(url).send().await?;
        let result: serde_json::Value = response.json().await?;

        Ok(result)
    }

    async fn execute_navigation(
        &self,
        path: &str,
        _context: &NavigationContext,
    ) -> Result<serde_json::Value> {
        // For navigation, we'll search for the symbol and return its definition
        let mut url = self.base_url.join("/api/symbols/search")?;
        url.query_pairs_mut().append_pair("q", path);

        let response = self.client.get(url).send().await?;
        let result: serde_json::Value = response.json().await?;

        Ok(result)
    }

    async fn execute_overview(
        &self,
        focus: Option<&str>,
        _detail_level: &DetailLevel,
    ) -> Result<serde_json::Value> {
        // Get overall codebase statistics and structure
        let url = self.base_url.join("/stats")?;
        let response = self.client.get(url).send().await?;
        let mut result: serde_json::Value = response.json().await?;

        // If focus is specified, add focused search results
        if let Some(focus_term) = focus {
            let mut search_url = self.base_url.join("/api/code/search")?;
            search_url.query_pairs_mut().append_pair("q", focus_term);

            if let Ok(search_response) = self.client.get(search_url).send().await {
                if let Ok(search_result) = search_response.json::<serde_json::Value>().await {
                    result["focused_results"] = search_result;
                }
            }
        }

        Ok(result)
    }

    async fn execute_debugging(
        &self,
        error: &str,
        context: Option<&str>,
    ) -> Result<serde_json::Value> {
        // Search for error-related code and patterns
        let search_query = if let Some(ctx) = context {
            format!("{} {}", error, ctx)
        } else {
            error.to_string()
        };

        let mut url = self.base_url.join("/api/code/search")?;
        url.query_pairs_mut().append_pair("q", &search_query);

        let response = self.client.get(url).send().await?;
        let result: serde_json::Value = response.json().await?;

        Ok(serde_json::json!({
            "debug_search_results": result,
            "suggestions": [
                "Check error handling patterns",
                "Look for similar error cases",
                "Analyze the error context"
            ]
        }))
    }
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_context(&self, session_id: &str) -> Option<ConversationContext> {
        self.conversations.read().await.get(session_id).cloned()
    }

    pub async fn update_context(
        &self,
        session_id: &str,
        query: &str,
        intent: &Intent,
        results: &serde_json::Value,
    ) {
        let mut conversations = self.conversations.write().await;

        let context = conversations
            .entry(session_id.to_string())
            .or_insert_with(|| ConversationContext {
                session_id: session_id.to_string(),
                previous_queries: Vec::new(),
                current_focus: None,
                language_hints: Vec::new(),
                last_results: None,
            });

        context.previous_queries.push(query.to_string());

        // Update focus based on intent
        match intent {
            Intent::Search { query, .. } => {
                context.current_focus = Some(query.clone());
            }
            Intent::Analysis { target, .. } => {
                context.current_focus = Some(target.clone());
            }
            Intent::Navigation { path, .. } => {
                context.current_focus = Some(path.clone());
            }
            _ => {}
        }

        context.last_results = Some(results.clone());

        // Keep only last 10 queries for memory efficiency
        if context.previous_queries.len() > 10 {
            context.previous_queries.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_parser_search() -> Result<()> {
        let parser = IntentParser::new();

        let intent = parser.parse("find function validate_path").await?;
        assert!(matches!(
            intent,
            Intent::Search {
                scope: SearchScope::Functions,
                ..
            }
        ));

        let intent = parser.parse("search for class FileStorage").await?;
        assert!(matches!(
            intent,
            Intent::Search {
                scope: SearchScope::Classes,
                ..
            }
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_intent_parser_analysis() -> Result<()> {
        let parser = IntentParser::new();

        let intent = parser.parse("who calls validate_path").await?;
        assert!(matches!(
            intent,
            Intent::Analysis {
                analysis_type: AnalysisType::Callers,
                ..
            }
        ));

        let intent = parser
            .parse("what would break if I change FileStorage")
            .await?;
        assert!(matches!(
            intent,
            Intent::Analysis {
                analysis_type: AnalysisType::Impact,
                ..
            }
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_intent_parser_overview() -> Result<()> {
        let parser = IntentParser::new();

        let intent = parser.parse("give me an overview of the codebase").await?;
        assert!(matches!(intent, Intent::Overview { .. }));

        let intent = parser.parse("show me the architecture").await?;
        assert!(matches!(intent, Intent::Overview { .. }));

        Ok(())
    }

    #[test]
    fn test_config_default() {
        let config = IntentMcpConfig::default();
        assert_eq!(config.api_base_url, "http://localhost:8080");
        assert_eq!(config.max_results, 20);
    }
}
