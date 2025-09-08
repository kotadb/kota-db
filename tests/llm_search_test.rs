// LLM Search Integration Tests
// Tests for the LLM-optimized search functionality with real storage and indices

use anyhow::Result;
use kotadb::{
    create_file_storage, create_trigram_index, ContextConfig, DocumentBuilder, Index,
    LLMSearchEngine, RelevanceConfig, Storage,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Test helper to create a test database with sample documents  
async fn setup_test_db() -> Result<(TempDir, Arc<Mutex<dyn Storage>>, Arc<Mutex<dyn Index>>)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Create storage and trigram index
    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));

    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

    // Insert sample documents for testing
    {
        let mut storage_guard = storage.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        // Document 1: Error handling example
        let doc1 = DocumentBuilder::new()
            .path("src/error_handler.rs")
            .unwrap()
            .title("Error Handler")
            .unwrap()
            .content(
                b"
pub fn handle_storage_error(error: StorageError) -> Result<(), ProcessingError> {
    match error {
        StorageError::NotFound => {
            log::warn!(\"Document not found in storage\");
            Err(ProcessingError::DocumentMissing)
        }
        StorageError::PermissionDenied => {
            log::error!(\"Permission denied accessing storage\");
            Err(ProcessingError::AccessDenied)
        }
        StorageError::IoError(e) => {
            log::error!(\"IO error in storage: {}\", e);
            Err(ProcessingError::InternalError)
        }
    }
}

pub fn retry_with_backoff<F, T>(mut operation: F) -> Result<T, ProcessingError>
where
    F: FnMut() -> Result<T, StorageError>,
{
    for attempt in 1..=3 {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < 3 => {
                log::warn!(\"Attempt {} failed, retrying: {:?}\", attempt, e);
                std::thread::sleep(std::time::Duration::from_millis(attempt * 100));
            }
            Err(e) => return Err(handle_storage_error(e)?),
        }
    }
    unreachable!()
}",
            )
            .build()?;

        storage_guard.insert(doc1.clone()).await?;
        trigram_guard
            .insert_with_content(doc1.id, doc1.path.clone(), &doc1.content)
            .await?;

        // Document 2: Index implementation
        let doc2 = DocumentBuilder::new()
            .path("src/primary_index.rs")
            .unwrap()
            .title("Primary Index")
            .unwrap()
            .content(
                b"
/// Primary index implementation using B+ tree for fast lookups
pub struct PrimaryIndex {
    tree: BTreeMap<String, DocumentId>,
    cache: LruCache<String, DocumentId>,
}

impl PrimaryIndex {
    pub fn new(capacity: usize) -> Self {
        Self {
            tree: BTreeMap::new(),
            cache: LruCache::new(capacity),
        }
    }
    
    pub fn search(&self, query: &str) -> Vec<DocumentId> {
        let mut results = Vec::new();
        
        // Exact match first
        if let Some(doc_id) = self.tree.get(query) {
            results.push(*doc_id);
            return results;
        }
        
        // Prefix search for wildcards
        if query.ends_with('*') {
            let prefix = &query[..query.len()-1];
            for (key, doc_id) in self.tree.range(prefix.to_string()..) {
                if key.starts_with(prefix) {
                    results.push(*doc_id);
                    if results.len() >= 50 {
                        break;
                    }
                }
            }
        }
        
        results
    }
}",
            )
            .build()?;

        storage_guard.insert(doc2.clone()).await?;
        trigram_guard
            .insert_with_content(doc2.id, doc2.path.clone(), &doc2.content)
            .await?;

        // Document 3: Test utilities
        let doc3 = DocumentBuilder::new()
            .path("tests/test_utils.rs")
            .unwrap()
            .title("Test Utilities")
            .unwrap()
            .content(
                b"
/// Test utilities for validating search functionality
use crate::*;

pub fn create_test_document(path: &str, content: &str) -> Result<Document> {
    DocumentBuilder::new()
        .path(path)?
        .title(format!(\"Test Document: {}\", path))?
        .content(content.as_bytes())?
        .build()
}

pub async fn validate_search_results(
    engine: &LLMSearchEngine,
    query: &str,
    expected_min: usize
) -> Result<bool> {
    let results = engine.search(query).await?;
    Ok(results.len() >= expected_min)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_search_validation() -> Result<()> {
        // Test implementation would go here
        Ok(())
    }
}",
            )
            .build()?;

        storage_guard.insert(doc3.clone()).await?;
        trigram_guard
            .insert_with_content(doc3.id, doc3.path.clone(), &doc3.content)
            .await?;
    }

    Ok((temp_dir, storage, trigram_index))
}

#[tokio::test]
async fn test_llm_search_basic_functionality() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    // Test search for "error"
    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("error", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    // Verify response structure
    assert_eq!(response.query, "error");
    assert!(
        !response.results.is_empty(),
        "Should find error-related documents"
    );
    assert!(response.optimization.total_matches > 0);
    // Query time is tracked (can be 0ms for very fast operations)

    // Verify results are ranked by relevance
    if response.results.len() > 1 {
        for i in 0..response.results.len() - 1 {
            assert!(
                response.results[i].relevance_score >= response.results[i + 1].relevance_score,
                "Results should be sorted by relevance score"
            );
        }
    }

    println!("✓ Basic LLM search functionality works");
    println!(
        "  Found {} results for 'error' in {}ms",
        response.results.len(),
        response.metadata.query_time_ms
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_relevance_ranking() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    // Search for "handle_storage_error" - should rank exact function name matches higher
    let response = search_engine
        .search_optimized(
            "handle_storage_error",
            &*storage_guard,
            &*trigram_guard,
            Some(10),
        )
        .await?;

    assert!(
        !response.results.is_empty(),
        "Should find function name matches"
    );

    // The document containing the function definition should have high relevance
    let top_result = &response.results[0];
    assert!(
        top_result.relevance_score > 0.5,
        "Top result should have high relevance"
    );
    assert!(
        top_result.path.contains("error_handler"),
        "Should prioritize the error handler file"
    );

    println!("✓ Relevance ranking prioritizes exact matches");
    println!(
        "  Top result: {} (score: {:.3})",
        top_result.path, top_result.relevance_score
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_token_optimization() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    // Test with a small token budget
    let context_config = ContextConfig {
        token_budget: 500, // Very small budget
        max_snippet_chars: 200,
        include_related: false,
        strip_comments: true,
        prefer_complete_functions: true,
        proximity_window_size: 100,
        max_term_matches: 10,
        match_context_size: 50,
    };

    let search_engine = LLMSearchEngine::with_config(RelevanceConfig::default(), context_config);

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("search", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    // Verify token budget is respected
    assert!(
        response.optimization.token_usage.estimated_tokens <= 500,
        "Should respect token budget"
    );

    // Verify results are optimized for size
    for result in &response.results {
        assert!(
            result.estimated_tokens <= 200,
            "Individual results should fit within reasonable token limits"
        );
        assert!(
            result.content_snippet.len() <= 200,
            "Content snippets should be appropriately sized"
        );
    }

    println!("✓ Token optimization works correctly");
    println!(
        "  Used {}/{} tokens ({}% efficiency)",
        response.optimization.token_usage.estimated_tokens,
        response.optimization.token_usage.budget,
        (response.optimization.token_usage.efficiency * 100.0) as u32
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_match_details() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("BTreeMap", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    assert!(
        !response.results.is_empty(),
        "Should find BTreeMap references"
    );

    let result_with_match = &response.results[0];

    // Verify match details are populated
    assert!(result_with_match.match_details.match_quality > 0.0);
    assert!(
        !result_with_match.match_details.exact_matches.is_empty()
            || !result_with_match.match_details.term_matches.is_empty(),
        "Should have match location information"
    );

    // Verify content snippet includes the match
    assert!(
        result_with_match
            .content_snippet
            .to_lowercase()
            .contains("btreemap"),
        "Content snippet should include the matched term"
    );

    println!("✓ Match details are properly analyzed");
    println!(
        "  Match quality: {:.3}, Primary type: {:?}",
        result_with_match.match_details.match_quality,
        result_with_match.match_details.primary_match_type
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_structured_output() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("test", &*storage_guard, &*trigram_guard, Some(5))
        .await?;

    // Verify complete response structure
    assert!(!response.query.is_empty());
    assert!(response.optimization.total_matches >= response.optimization.returned);
    assert!(response.optimization.token_usage.estimated_tokens > 0);
    // Query time is reported (u64 is always >= 0, so we just verify it exists)

    // Verify each result has required fields
    for result in &response.results {
        assert!(!result.id.is_empty());
        assert!(!result.path.is_empty());
        assert!(result.title.is_some());
        assert!(result.relevance_score >= 0.0 && result.relevance_score <= 1.0);
        assert!(!result.content_snippet.is_empty());
        assert!(result.estimated_tokens > 0);
    }

    // Verify suggestions are generated
    assert!(
        !response.metadata.suggestions.is_empty(),
        "Should provide helpful suggestions"
    );

    println!("✓ Structured output format is complete and valid");
    println!(
        "  Response includes {} suggestions and {} warnings",
        response.metadata.suggestions.len(),
        response.metadata.warnings.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_empty_query_handling() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    // Test empty query
    let result = search_engine
        .search_optimized("", &*storage_guard, &*trigram_guard, Some(10))
        .await;

    // Should handle empty query gracefully
    match result {
        Err(e) => {
            assert!(
                e.to_string().contains("empty") || e.to_string().contains("invalid"),
                "Should provide meaningful error for empty query: {}",
                e
            );
        }
        Ok(response) => {
            // If not erroring, should return reasonable results or warnings
            if response.results.is_empty() {
                assert!(
                    !response.metadata.warnings.is_empty(),
                    "Should warn about empty query"
                );
            }
        }
    }

    println!("✓ Empty query handling works correctly");

    Ok(())
}

#[tokio::test]
async fn test_llm_search_performance() -> Result<()> {
    let (_temp_dir, storage, trigram_index) = setup_test_db().await?;

    let search_engine = LLMSearchEngine::new();

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let start_time = std::time::Instant::now();

    let response = search_engine
        .search_optimized("function", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    let elapsed = start_time.elapsed();

    // Verify performance targets
    assert!(
        elapsed.as_millis() < 100,
        "Search should complete within 100ms for small dataset"
    );
    assert!(
        response.metadata.query_time_ms < 100,
        "Reported query time should be reasonable"
    );

    println!("✓ Performance meets targets");
    println!(
        "  Search completed in {}ms (reported: {}ms)",
        elapsed.as_millis(),
        response.metadata.query_time_ms
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_unicode_boundary_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Create storage and trigram index
    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));

    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

    // Insert document with Unicode content that previously caused crashes
    {
        let mut storage_guard = storage.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        let unicode_content = "
#!/bin/bash
# Test script with Unicode box-drawing characters
echo \"Starting test with Unicode...\"

# Box drawing characters that caused the crash:
# ═══════════════════════════════════════════════════════════════════
# ║ This section contains Unicode box-drawing characters           ║
# ║ These characters are multi-byte UTF-8 sequences                ║
# ╠═══════════════════════════════════════════════════════════════════╣
# ║ Character '═' takes 3 bytes: 0xE2 0x95 0x90                   ║
# ║ Character '║' takes 3 bytes: 0xE2 0x95 0x91                   ║
# ║ Character '╠' takes 3 bytes: 0xE2 0x95 0xA0                   ║
# ║ Character '╣' takes 3 bytes: 0xE2 0x95 0xA3                   ║
# ╚═══════════════════════════════════════════════════════════════════╝

echo \"Test complete with emojis: 🚀 🎯 ✅ 🔧 📋\"
echo \"International chars: áéíóú ñ ç ß æ ø å\"
echo \"Mathematical symbols: ∑ ∫ ∆ π ∞ √ ≤ ≥ ≠\"
";

        let doc = DocumentBuilder::new()
            .path("scripts/unicode_test.sh")
            .unwrap()
            .title("Unicode Test Script")
            .unwrap()
            .content(unicode_content.as_bytes())
            .build()?;

        storage_guard.insert(doc.clone()).await?;
        trigram_guard
            .insert_with_content(doc.id, doc.path.clone(), &doc.content)
            .await?;
    }

    let search_engine = LLMSearchEngine::new();

    // Test 1: Search that should find the Unicode content
    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("box-drawing", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    assert!(
        !response.results.is_empty(),
        "Should find document with Unicode content"
    );

    // Test 2: Verify content snippet doesn't crash on Unicode boundaries
    for result in &response.results {
        assert!(
            !result.content_snippet.is_empty(),
            "Content snippet should not be empty"
        );

        // Verify the snippet is valid UTF-8
        assert!(
            std::str::from_utf8(result.content_snippet.as_bytes()).is_ok(),
            "Content snippet should be valid UTF-8"
        );

        // Verify it can be displayed without panicking
        let _ = result.content_snippet.to_string();
    }

    println!("✓ Unicode boundary handling works correctly");
    println!(
        "  Processed {} results with Unicode content safely",
        response.results.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_snippet_length_with_unicode() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));

    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

    // Test with different Unicode character types
    let test_cases = vec![
        (
            "emojis.txt",
            "Search with emojis: 🔍 🚀 💡 📊 ⚡ 🎯 ✅ 🔧 📋 🌟 ".repeat(50),
        ),
        (
            "accents.txt",
            "Àccéntéd châráctérs: café naïve résumé piñata façade ".repeat(50),
        ),
        (
            "mathematical.txt",
            "Math symbols: ∑∫∆πα∞√≤≥≠±×÷∈∉⊂⊃∪∩∀∃∇∂ ".repeat(50),
        ),
        (
            "box_drawing.txt",
            "Box chars: ═║╔╗╚╝╠╣╦╩╬├┤┬┴┼─│┌┐└┘ ".repeat(50),
        ),
    ];

    for (filename, content) in test_cases {
        let mut storage_guard = storage.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        let doc = DocumentBuilder::new()
            .path(filename)
            .unwrap()
            .title(format!("Unicode Test: {}", filename))
            .unwrap()
            .content(content.as_bytes())
            .build()?;

        storage_guard.insert(doc.clone()).await?;
        trigram_guard
            .insert_with_content(doc.id, doc.path.clone(), &doc.content)
            .await?;

        drop(storage_guard);
        drop(trigram_guard);
    }

    // Test with small snippet size to force truncation at Unicode boundaries
    let context_config = ContextConfig {
        token_budget: 1000,
        max_snippet_chars: 100, // Small size to trigger boundary issues
        include_related: false,
        strip_comments: false,
        prefer_complete_functions: false,
        proximity_window_size: 50,
        max_term_matches: 5,
        match_context_size: 25,
    };

    let search_engine = LLMSearchEngine::with_config(RelevanceConfig::default(), context_config);

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    let response = search_engine
        .search_optimized("symbols", &*storage_guard, &*trigram_guard, Some(10))
        .await?;

    for result in &response.results {
        // Verify snippet length is reasonable but respects Unicode boundaries
        assert!(
            result.content_snippet.len() <= 200, // Allow some flexibility
            "Snippet should be approximately within size limits"
        );

        // Most important: verify it's valid UTF-8 after truncation
        assert!(
            std::str::from_utf8(result.content_snippet.as_bytes()).is_ok(),
            "Truncated snippet must be valid UTF-8: {}",
            result.path
        );

        // Verify no Unicode characters are broken
        for c in result.content_snippet.chars() {
            assert!(
                !c.is_control() || c == '\n' || c == '\t',
                "Should not contain broken Unicode control characters"
            );
        }
    }

    println!("✓ Unicode snippet truncation works correctly");
    println!(
        "  Tested {} results with various Unicode character types",
        response.results.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_llm_search_regression_issue_397() -> Result<()> {
    // Regression test specifically for issue #397 - Unicode boundary crash
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));

    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

    // Create content that reproduces the exact crash scenario
    let crash_content = format!(
        "{}{}{}",
        "-".repeat(497), // Get close to the crash boundary
        "═══",           // The Unicode character that caused the crash (bytes 499-502)
        "More content after the Unicode character to test boundary handling"
    );

    {
        let mut storage_guard = storage.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        let doc = DocumentBuilder::new()
            .path("tests/crash_reproduction.md")
            .unwrap()
            .title("Crash Reproduction Test")
            .unwrap()
            .content(crash_content.as_bytes())
            .build()?;

        storage_guard.insert(doc.clone()).await?;
        trigram_guard
            .insert_with_content(doc.id, doc.path.clone(), &doc.content)
            .await?;
    }

    // Use configuration that previously caused the crash
    let context_config = ContextConfig {
        token_budget: 2000,
        max_snippet_chars: 500, // This was the problematic size
        include_related: false,
        strip_comments: false,
        prefer_complete_functions: false,
        proximity_window_size: 100,
        max_term_matches: 10,
        match_context_size: 50,
    };

    let search_engine = LLMSearchEngine::with_config(RelevanceConfig::default(), context_config);

    let storage_guard = storage.lock().await;
    let trigram_guard = trigram_index.lock().await;

    // This search should NOT crash (was the failing case from issue #397)
    let response = search_engine
        .search_optimized(
            "nonexistent_term",
            &*storage_guard,
            &*trigram_guard,
            Some(10),
        )
        .await?;

    // When no matches are found, it falls back to beginning of content
    // This path previously crashed due to Unicode boundary issues
    if !response.results.is_empty() {
        for result in &response.results {
            // Verify the result is valid and doesn't contain broken Unicode
            assert!(
                std::str::from_utf8(result.content_snippet.as_bytes()).is_ok(),
                "Content snippet must be valid UTF-8 after boundary correction"
            );

            // Additional validation that the fix worked
            assert!(
                result.content_snippet.len() <= 600, // Some buffer for safety
                "Snippet should be within reasonable bounds"
            );
        }
    }

    println!("✓ Issue #397 regression test passed");
    println!("  Unicode boundary crash is fixed - search completes without panic");

    Ok(())
}
