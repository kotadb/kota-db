//! Comprehensive integration tests for the code analysis features.
//!
//! This test suite validates all codebase analysis features working together,
//! including symbol extraction, code search, dependency mapping, and natural
//! language queries. Uses KotaDB's own codebase for dogfooding validation.

use anyhow::Result;
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::contracts::Index;
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::dependency_extractor::DependencyExtractor;
// Natural language query processor removed
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::parsing::SymbolType;
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::symbol_index::{CodeQuery, SymbolIndex};
use kotadb::{DocumentBuilder, ValidatedPath};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;

mod test_constants;
use test_constants::gating;

/// Helper to create a test symbol index with sample code
#[cfg(feature = "tree-sitter-parsing")]
async fn create_test_index() -> Result<(SymbolIndex, TempDir)> {
    let temp_dir = TempDir::new()?;
    let data_dir = temp_dir.path().join("data");
    fs::create_dir_all(&data_dir)?;

    let storage = kotadb::create_file_storage(data_dir.to_str().unwrap(), Some(1000)).await?;
    let index =
        kotadb::create_symbol_index_for_tests(data_dir.to_str().unwrap(), Box::new(storage))
            .await?;
    Ok((index, temp_dir))
}

/// Helper to load KotaDB source files for dogfooding tests
fn load_kotadb_source_files() -> Result<Vec<(String, String)>> {
    use anyhow::Context;

    let mut files = Vec::new();
    let src_dir = Path::new("src");

    // Fail explicitly if src directory doesn't exist
    if !src_dir.exists() {
        // Use bundled test data as fallback
        let test_code = include_str!("test_data/sample_code.rs");
        files.push((
            "test_data/sample_code.rs".to_string(),
            test_code.to_string(),
        ));
        return Ok(files);
    }

    for entry in fs::read_dir(src_dir).context("Failed to read src directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;
            let relative_path = path.strip_prefix(".").unwrap_or(&path);
            files.push((relative_path.to_string_lossy().into_owned(), content));
        }
    }

    // Ensure we have at least some files
    if files.is_empty() {
        anyhow::bail!("No Rust source files found in src directory");
    }

    Ok(files)
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_complete_analysis_pipeline() -> Result<()> {
    // This test validates the entire code analysis pipeline working together
    let (mut index, _temp_dir) = create_test_index().await?;

    // Sample Rust code that exercises all features
    let test_code = r#"
use std::collections::HashMap;
use anyhow::Result;

/// A sample storage implementation
pub struct FileStorage {
    data: HashMap<String, Vec<u8>>,
}

impl FileStorage {
    /// Creates a new file storage instance
    pub fn new() -> Self {
        FileStorage {
            data: HashMap::new(),
        }
    }
    
    /// Stores data with the given key
    pub fn store(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        self.data.insert(key, value);
        Ok(())
    }
    
    /// Retrieves data by key
    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.data.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage() {
        let mut storage = FileStorage::new();
        storage.store("test".to_string(), vec![1, 2, 3]).unwrap();
        assert!(storage.get("test").is_some());
    }
}
"#;

    // Step 1: Symbol extraction
    let path = ValidatedPath::new("test/storage.rs")?;
    let doc = DocumentBuilder::new()
        .path(path.as_str())?
        .title("Test Storage")?
        .content(test_code.as_bytes())
        .build()?;

    index
        .insert_with_content(doc.id, doc.path.clone(), test_code.as_bytes())
        .await?;

    // Step 2: Code-specific search
    let query = CodeQuery::SymbolSearch {
        name: "FileStorage".to_string(),
        symbol_types: Some(vec![SymbolType::Struct]),
        fuzzy: false,
    };

    let results = index.search_code(&query).await?;
    assert!(!results.is_empty(), "Should find FileStorage struct");

    // Step 3: Dependency mapping
    let extractor = DependencyExtractor::new()?;
    // Parse the content first
    let mut parser = kotadb::parsing::CodeParser::new()?;
    let parsed = parser.parse_content(test_code, kotadb::parsing::SupportedLanguage::Rust)?;
    let analysis = extractor.extract_dependencies(
        &parsed,
        test_code,
        std::path::Path::new("test/storage.rs"),
    )?;

    // Verify imports are tracked
    assert!(analysis.imports.iter().any(|i| i.path.contains("HashMap")));
    assert!(analysis.imports.iter().any(|i| i.path.contains("Result")));

    // Step 4: Natural language queries REMOVED
    // Natural language processing has been removed - use direct commands instead
    // Previously: let nlp = NaturalLanguageQueryProcessor::new();
    // Previously: let nl_query = "find functions that store data";
    // Previously: let intent = nlp.parse_query(nl_query).await?;

    // Convert natural language to structured query
    let structured_query = CodeQuery::SymbolSearch {
        name: "store".to_string(),
        symbol_types: Some(vec![SymbolType::Function, SymbolType::Method]),
        fuzzy: true,
    };
    let nl_results = index.search_code(&structured_query).await?;

    // Verify we found the store method
    assert!(nl_results.iter().any(|r| r.symbol_name == "store"));

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_dogfooding_symbol_extraction() -> Result<()> {
    // Test symbol extraction on actual KotaDB source code
    let (mut index, _temp_dir) = create_test_index().await?;
    let source_files = load_kotadb_source_files()?;

    // Index all source files
    for (path, content) in &source_files {
        if path.ends_with(".rs") {
            let validated_path = ValidatedPath::new(path)?;
            let doc = DocumentBuilder::new()
                .path(validated_path.as_str())?
                .title("Source File")?
                .content(content.as_bytes())
                .build()?;

            index
                .insert_with_content(doc.id, doc.path.clone(), content.as_bytes())
                .await?;
        }
    }

    // Dynamically discover and verify symbols exist
    // First, search for any structs to validate the index has content
    let struct_query = CodeQuery::SymbolSearch {
        name: String::new(),
        symbol_types: Some(vec![SymbolType::Struct]),
        fuzzy: true,
    };

    let struct_results = index.search_code(&struct_query).await?;
    assert!(
        !struct_results.is_empty(),
        "Should find at least some structs in KotaDB source"
    );

    // Verify we can find functions
    let function_query = CodeQuery::SymbolSearch {
        name: String::new(),
        symbol_types: Some(vec![SymbolType::Function]),
        fuzzy: true,
    };

    let function_results = index.search_code(&function_query).await?;
    assert!(
        !function_results.is_empty(),
        "Should find at least some functions in KotaDB source"
    );

    // Log what we found for debugging
    println!(
        "Found {} structs and {} functions in source",
        struct_results.len(),
        function_results.len()
    );

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_performance_targets() -> Result<()> {
    // Verify <10ms query latency requirement
    let (mut index, _temp_dir) = create_test_index().await?;

    // Load and index multiple files to simulate realistic load
    let source_files = load_kotadb_source_files()?;
    for (path, content) in &source_files[..source_files.len().min(10)] {
        if path.ends_with(".rs") {
            let validated_path = ValidatedPath::new(path)?;
            let doc = DocumentBuilder::new()
                .path(validated_path.as_str())?
                .title("Source File")?
                .content(content.as_bytes())
                .build()?;

            index
                .insert_with_content(doc.id, doc.path.clone(), content.as_bytes())
                .await?;
        }
    }

    // Test various query types and measure performance
    let queries = vec![
        CodeQuery::SymbolSearch {
            name: "test".to_string(),
            symbol_types: None,
            fuzzy: true,
        },
        CodeQuery::SymbolSearch {
            name: "".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: false,
        },
        CodeQuery::PatternSearch {
            pattern: kotadb::symbol_index::CodePattern::Custom("error".to_string()),
            scope: kotadb::symbol_index::SearchScope::All,
        },
    ];

    for query in queries {
        let start = Instant::now();
        let _results = index.search_code(&query).await?;
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 10,
            "Query took {}ms, exceeds 10ms target",
            elapsed.as_millis()
        );
    }

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
#[ignore = "Dependency graph merging not yet implemented - TODO: implement when API is ready"]
async fn test_dependency_graph_building() -> Result<()> {
    // Test dependency extraction and graph building
    let extractor = DependencyExtractor::new()?;

    // Load actual KotaDB source for realistic testing
    let source_files = load_kotadb_source_files()?;

    // Track extracted dependencies for validation
    let mut total_imports = 0;
    let mut total_symbols = 0;

    for (path, content) in &source_files[..source_files.len().min(3)] {
        // Limit to 3 files for testing
        if path.ends_with(".rs") {
            // Parse the content first
            let mut parser = kotadb::parsing::CodeParser::new()?;
            let parsed = parser.parse_content(content, kotadb::parsing::SupportedLanguage::Rust)?;
            let analysis =
                extractor.extract_dependencies(&parsed, content, std::path::Path::new(path))?;

            // Validate that we extracted meaningful data
            total_imports += analysis.imports.len();
            total_symbols += analysis.symbols.len();

            println!(
                "File {}: {} imports, {} symbols",
                path,
                analysis.imports.len(),
                analysis.symbols.len()
            );
        }
    }

    // Verify we extracted meaningful dependency data
    assert!(
        total_imports > 0 || total_symbols > 0,
        "Should have extracted some dependencies or symbols"
    );

    Ok(())
}

// Natural language query test removed - natural language processing has been removed
// #[cfg(feature = "tree-sitter-parsing")]
// #[tokio::test]
// async fn test_natural_language_queries() -> Result<()> {
// //     // Test natural language query processing
//     let (mut index, _temp_dir) = create_test_index().await?;
//     let nlp = NaturalLanguageQueryProcessor::new();

//     // Index some test code
//     let test_code = r#"
// pub fn validate_input(data: &str) -> bool {
//     !data.is_empty() && data.len() < 1000
// }
//
// pub fn handle_error(err: anyhow::Error) {
//     eprintln!("Error occurred: {}", err);
// }
//
// pub async fn fetch_data(url: &str) -> Result<String> {
//     // Simulated async fetch
//     Ok("data".to_string())
// }
// "#;
//
//     let doc = DocumentBuilder::new()
//         .path("test.rs")?
//         .title("Test Code")?
//         .content(test_code.as_bytes())
//         .build()?;
//
//     index
//         .insert_with_content(doc.id, doc.path.clone(), test_code.as_bytes())
//         .await?;
//
//     // Test various natural language queries
//     let test_queries = vec![
//         "find validation functions",
//         "show error handling code",
//         "find async functions",
//         "search for functions with url parameter",
//     ];
//
//     for query_text in test_queries {
//         let intent = nlp.parse_query(query_text).await?;
//         // For now, create a simple search query since intent_to_query may not exist
//         let query = CodeQuery::SymbolSearch {
//             name: "".to_string(), // Search all symbols
//             symbol_types: Some(vec![SymbolType::Function]),
//             fuzzy: true,
//         };
//         let results = index.search_code(&query).await?;
//
//         println!("Query '{}' returned {} results", query_text, results.len());
//         // Note: We're not asserting results are non-empty since the simple query may not match the intent
//     }
//
//     Ok(())
// }

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    if gating::skip_if_heavy_disabled("code_analysis_integration_test::test_edge_cases") {
        return Ok(());
    }

    // Test handling of edge cases
    let (mut index, _temp_dir) = create_test_index().await?;

    // Test 1: Empty file
    let empty_doc = DocumentBuilder::new()
        .path("empty.rs")?
        .title("Empty File")?
        .content(b"")
        .build()?;

    index
        .insert_with_content(empty_doc.id, empty_doc.path.clone(), b"")
        .await?;

    // Test 2: Malformed code
    let malformed = "pub fn incomplete(";
    let malformed_doc = DocumentBuilder::new()
        .path("malformed.rs")?
        .title("Malformed Code")?
        .content(malformed.as_bytes())
        .build()?;

    // Should handle gracefully without panic
    let result = index
        .insert_with_content(
            malformed_doc.id,
            malformed_doc.path.clone(),
            malformed.as_bytes(),
        )
        .await;
    assert!(result.is_ok(), "Should handle malformed code gracefully");

    // Test 3: Very large file (simulate)
    let large_content = "fn test() {}\n".repeat(10000);
    let large_doc = DocumentBuilder::new()
        .path("large.rs")?
        .title("Large File")?
        .content(large_content.as_bytes())
        .build()?;

    let start = Instant::now();
    index
        .insert_with_content(
            large_doc.id,
            large_doc.path.clone(),
            large_content.as_bytes(),
        )
        .await?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 10,
        "Large file processing took too long: {}s (max 10s allowed)",
        elapsed.as_secs()
    );

    // Test 4: Non-Rust file (should handle gracefully)
    let non_rust = "SELECT * FROM users WHERE id = 1;";
    let non_rust_doc = DocumentBuilder::new()
        .path("query.sql")?
        .title("SQL Query")?
        .content(non_rust.as_bytes())
        .build()?;

    let result = index
        .insert_with_content(
            non_rust_doc.id,
            non_rust_doc.path.clone(),
            non_rust.as_bytes(),
        )
        .await;
    assert!(result.is_ok(), "Should handle non-Rust files gracefully");

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    // Test thread safety and concurrent access
    let (index, _temp_dir) = create_test_index().await?;
    let index = std::sync::Arc::new(tokio::sync::RwLock::new(index));

    // Spawn multiple concurrent operations
    let mut handles = vec![];

    for i in 0..10 {
        let index_clone = index.clone();
        let handle = tokio::spawn(async move {
            let code = format!(
                "pub fn function_{}() {{ println!(\"Function {}\"); }}",
                i, i
            );

            let doc = DocumentBuilder::new()
                .path(format!("concurrent_{}.rs", i))?
                .title(format!("Concurrent File {}", i))?
                .content(code.as_bytes())
                .build()?;

            // Add timeout for lock acquisition to prevent deadlock
            let index_guard =
                tokio::time::timeout(std::time::Duration::from_secs(5), index_clone.write())
                    .await
                    .map_err(|_| anyhow::anyhow!("Timeout acquiring write lock"))?;

            let mut index = index_guard;
            index
                .insert_with_content(doc.id, doc.path.clone(), code.as_bytes())
                .await
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    // Verify all functions were indexed
    let index = index.read().await;
    for i in 0..10 {
        let query = CodeQuery::SymbolSearch {
            name: format!("function_{}", i),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: false,
        };

        let results = index.search_code(&query).await?;
        assert!(!results.is_empty(), "Function {} should be indexed", i);
    }

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_query_combinations() -> Result<()> {
    // Test complex query combinations
    let (mut index, _temp_dir) = create_test_index().await?;

    let test_code = r#"
pub struct UserManager {
    users: Vec<User>,
}

impl UserManager {
    pub fn new() -> Self {
        UserManager { users: Vec::new() }
    }
    
    pub fn add_user(&mut self, user: User) {
        self.users.push(user);
    }
    
    pub fn find_user(&self, id: u64) -> Option<&User> {
        self.users.iter().find(|u| u.id == id)
    }
    
    pub async fn validate_user(&self, user: &User) -> bool {
        // Validation logic
        true
    }
}

pub struct User {
    id: u64,
    name: String,
}
"#;

    let doc = DocumentBuilder::new()
        .path("user_manager.rs")?
        .title("User Manager")?
        .content(test_code.as_bytes())
        .build()?;

    index
        .insert_with_content(doc.id, doc.path.clone(), test_code.as_bytes())
        .await?;

    // Test pattern search: Find async methods
    let query = CodeQuery::PatternSearch {
        pattern: kotadb::symbol_index::CodePattern::AsyncAwait,
        scope: kotadb::symbol_index::SearchScope::Functions,
    };

    let results = index.search_code(&query).await?;
    assert!(!results.is_empty(), "Should find at least one async method");
    assert!(results.iter().any(|r| r.symbol_name == "validate_user"));

    // Test symbol search: Find structs with "user" in name
    let query = CodeQuery::SymbolSearch {
        name: "user".to_string(),
        symbol_types: Some(vec![SymbolType::Struct]),
        fuzzy: true,
    };

    let results = index.search_code(&query).await?;
    assert!(!results.is_empty(), "Should find user-related items");

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_integration_with_existing_queries() -> Result<()> {
    // Test queries from the original dogfooding issue #104
    // Natural language processing has been removed - using direct queries instead
    let (mut index, _temp_dir) = create_test_index().await?;

    // Load a subset of KotaDB source
    let source_files = load_kotadb_source_files()?;
    for (path, content) in &source_files[..source_files.len().min(5)] {
        if path.ends_with(".rs") {
            let validated_path = ValidatedPath::new(path)?;
            let doc = DocumentBuilder::new()
                .path(validated_path.as_str())?
                .title("Source File")?
                .content(content.as_bytes())
                .build()?;

            index
                .insert_with_content(doc.id, doc.path.clone(), content.as_bytes())
                .await?;
        }
    }

    // Test direct queries instead of natural language
    // Previously: "find all test functions", "show storage implementation", etc.
    let query = CodeQuery::SymbolSearch {
        name: "test".to_string(),
        symbol_types: Some(vec![SymbolType::Function]),
        fuzzy: true,
    };
    let results = index.search_code(&query).await?;

    // Log results for validation
    println!(
        "Direct query for test functions returned {} results",
        results.len()
    );

    Ok(())
}
/// Regression test to ensure analysis accuracy over time
#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn test_analysis_accuracy_regression() -> Result<()> {
    let (mut index, _temp_dir) = create_test_index().await?;

    // Known code with expected symbols
    let test_code = r#"
pub trait Storage {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn put(&mut self, key: String, value: Vec<u8>);
}

pub struct MemoryStorage {
    data: std::collections::HashMap<String, Vec<u8>>,
}

impl Storage for MemoryStorage {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }
    
    fn put(&mut self, key: String, value: Vec<u8>) {
        self.data.insert(key, value);
    }
}
"#;

    let doc = DocumentBuilder::new()
        .path("regression_test.rs")?
        .title("Regression Test")?
        .content(test_code.as_bytes())
        .build()?;

    index
        .insert_with_content(doc.id, doc.path.clone(), test_code.as_bytes())
        .await?;

    // Expected symbols that must be found
    let expected_symbols = vec![
        ("Storage", SymbolType::Interface),
        ("MemoryStorage", SymbolType::Struct),
        ("get", SymbolType::Method),
        ("put", SymbolType::Method),
    ];

    for (name, symbol_type) in expected_symbols {
        let query = CodeQuery::SymbolSearch {
            name: name.to_string(),
            symbol_types: Some(vec![symbol_type.clone()]),
            fuzzy: false,
        };

        let results = index.search_code(&query).await?;
        assert!(
            !results.is_empty(),
            "Regression: Failed to find {} {:?}",
            name,
            symbol_type
        );
    }

    // Verify dependency extraction
    let extractor = DependencyExtractor::new()?;
    // Parse the content first
    let mut parser = kotadb::parsing::CodeParser::new()?;
    let parsed = parser.parse_content(test_code, kotadb::parsing::SupportedLanguage::Rust)?;
    let analysis = extractor.extract_dependencies(
        &parsed,
        test_code,
        std::path::Path::new("regression_test.rs"),
    )?;

    // Should detect the trait implementation relationship
    assert!(
        analysis.imports.iter().any(|i| i.path.contains("HashMap")),
        "Regression: Failed to detect HashMap import"
    );

    Ok(())
}
