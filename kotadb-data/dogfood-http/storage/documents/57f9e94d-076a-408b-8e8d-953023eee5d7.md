---
tags:
- file
- kota-db
- ext_rs
---
//! Integration tests for symbol index functionality

#[cfg(feature = "tree-sitter-parsing")]
mod symbol_index_tests {
    use anyhow::Result;
    use tempfile::TempDir;

    use kotadb::contracts::{Index, Query};
    use kotadb::parsing::SymbolType;
    use kotadb::symbol_index::{CodeQuery, SymbolIndex};
    use kotadb::types::{ValidatedDocumentId, ValidatedPath};

    #[tokio::test]
    async fn test_symbol_index_basic_functionality() -> Result<()> {
        // Create temporary directory
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        // Create symbol index
        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        // Test Rust code
        let rust_code = r#"
use std::collections::HashMap;

/// Calculate the total of a vector
fn calculate_total(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

/// Calculate the average of numbers
fn calculate_average(numbers: &[i32]) -> f64 {
    if numbers.is_empty() {
        0.0
    } else {
        calculate_total(numbers) as f64 / numbers.len() as f64
    }
}

struct Calculator {
    history: Vec<i32>,
}

impl Calculator {
    fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
    
    fn add(&mut self, value: i32) -> i32 {
        self.history.push(value);
        value
    }
}
"#;

        // Insert code into index
        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("test.rs")?;
        println!("Inserting code into symbol index...");
        index
            .insert_with_content(doc_id, path, rust_code.as_bytes())
            .await?;
        println!("Code inserted successfully");

        // Debug: Try searching without type filter to see what's stored
        let all_query = CodeQuery::SymbolSearch {
            name: "Calculator".to_string(),
            symbol_types: None, // No filter
            fuzzy: false,
        };
        let all_results = index.search_code(&all_query).await?;
        println!(
            "Search for 'Calculator' (no type filter): {:?}",
            all_results
        );

        // Test symbol search
        let query = CodeQuery::SymbolSearch {
            name: "calculate".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: true,
        };

        let results = index.search_code(&query).await?;
        println!(
            "Function search results: {:?}",
            results.iter().map(|r| &r.symbol_name).collect::<Vec<_>>()
        );
        assert!(
            results.len() >= 2,
            "Should find calculate_total and calculate_average functions"
        );

        // Check that we found functions with "calculate" in the name
        let function_names: Vec<String> = results.iter().map(|r| r.symbol_name.clone()).collect();
        let has_calculate_function = function_names.iter().any(|name| name.contains("calculate"));
        assert!(
            has_calculate_function,
            "Should find at least one function with 'calculate' in the name"
        );

        // Test struct search - search for actual struct name, not the keyword
        let struct_query = CodeQuery::SymbolSearch {
            name: "Calc".to_string(), // Search for part of "Calculator" with fuzzy matching
            symbol_types: Some(vec![SymbolType::Struct]),
            fuzzy: true,
        };

        let struct_results = index.search_code(&struct_query).await?;
        println!("Struct search results: {:?}", struct_results);
        assert!(
            !struct_results.is_empty(),
            "Should find at least one struct. Found: {:?}",
            struct_results
        );
        println!(
            "Found structs: {:?}",
            struct_results
                .iter()
                .map(|r| &r.symbol_name)
                .collect::<Vec<_>>()
        );

        // Test dependency search
        let dep_query = CodeQuery::DependencySearch {
            target: "std".to_string(),
            direction: kotadb::symbol_index::DependencyDirection::Dependencies,
        };

        let dep_results = index.search_code(&dep_query).await?;
        // Note: This will depend on how well the import parsing works
        println!("Dependency results: {:?}", dep_results);

        Ok(())
    }

    #[tokio::test]
    async fn test_pattern_search() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        // Code with error handling patterns
        let rust_code = r#"
fn risky_operation() -> Result<i32, String> {
    if true {
        Ok(42)
    } else {
        Err("Something went wrong".to_string())
    }
}

fn handle_errors() {
    match risky_operation() {
        Ok(value) => println!("Got: {}", value),
        Err(e) => panic!("Error: {}", e),
    }
    
    let result = risky_operation().unwrap();
    println!("Result: {}", result);
}

#[test]
fn test_something() {
    assert_eq!(2 + 2, 4);
}
"#;

        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("error_handling.rs")?;
        index
            .insert_with_content(doc_id, path, rust_code.as_bytes())
            .await?;

        // Test error handling pattern search
        let error_query = CodeQuery::PatternSearch {
            pattern: kotadb::symbol_index::CodePattern::ErrorHandling,
            scope: kotadb::symbol_index::SearchScope::All,
        };

        let error_results = index.search_code(&error_query).await?;
        assert!(
            !error_results.is_empty(),
            "Should find error handling patterns"
        );

        // Test test pattern search
        let test_query = CodeQuery::PatternSearch {
            pattern: kotadb::symbol_index::CodePattern::TestCode,
            scope: kotadb::symbol_index::SearchScope::All,
        };

        let test_results = index.search_code(&test_query).await?;
        assert!(!test_results.is_empty(), "Should find test patterns");

        Ok(())
    }

    #[tokio::test]
    async fn test_signature_search() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        let rust_code = r#"
fn process_string(input: &str) -> String {
    input.to_uppercase()
}

fn process_numbers(numbers: Vec<i32>) -> i32 {
    numbers.iter().sum()
}

fn process_option(maybe_value: Option<String>) -> String {
    maybe_value.unwrap_or_default()
}
"#;

        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("signatures.rs")?;
        index
            .insert_with_content(doc_id, path, rust_code.as_bytes())
            .await?;

        // Search for functions that work with strings
        let sig_query = CodeQuery::SignatureSearch {
            pattern: "String".to_string(),
            language: Some("rust".to_string()),
        };

        let sig_results = index.search_code(&sig_query).await?;
        assert!(
            !sig_results.is_empty(),
            "Should find functions with String in signature"
        );

        // Verify we found the right functions
        let function_names: Vec<String> =
            sig_results.iter().map(|r| r.symbol_name.clone()).collect();

        // Should find functions that have String in their signature
        println!("Functions with String signatures: {:?}", function_names);

        Ok(())
    }

    #[tokio::test]
    async fn test_combined_query() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        let rust_code = r#"
fn calculate_sum(numbers: &[i32]) -> Result<i32, String> {
    Ok(numbers.iter().sum())
}

fn calculate_product(numbers: &[i32]) -> Result<i32, String> {
    Ok(numbers.iter().product())
}

fn format_result(value: i32) -> String {
    format!("Result: {}", value)
}
"#;

        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("combined.rs")?;
        index
            .insert_with_content(doc_id, path, rust_code.as_bytes())
            .await?;

        // Combined query: functions that have "calculate" in name AND use Result type
        let combined_query = CodeQuery::Combined {
            queries: vec![
                CodeQuery::SymbolSearch {
                    name: "calculate".to_string(),
                    symbol_types: Some(vec![SymbolType::Function]),
                    fuzzy: true,
                },
                CodeQuery::SignatureSearch {
                    pattern: "Result".to_string(),
                    language: Some("rust".to_string()),
                },
            ],
            operator: kotadb::symbol_index::QueryOperator::And,
        };

        let combined_results = index.search_code(&combined_query).await?;

        // Should find calculate_sum and calculate_product (both have "calculate" and return Result)
        assert!(
            combined_results.len() >= 2,
            "Should find functions matching both criteria"
        );

        let function_names: Vec<String> = combined_results
            .iter()
            .map(|r| r.symbol_name.clone())
            .collect();

        println!("Combined query results: {:?}", function_names);

        Ok(())
    }

    #[tokio::test]
    async fn test_index_trait_compatibility() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        // Test that it works as a standard Index trait
        let doc_id = ValidatedDocumentId::new();
        let path = ValidatedPath::new("std_test.rs")?;
        let content = "fn hello() { println!(\"Hello, world!\"); }";

        index
            .insert_with_content(doc_id, path.clone(), content.as_bytes())
            .await?;

        // Test standard query interface
        let mut query = Query::empty();
        query
            .search_terms
            .push(kotadb::types::ValidatedSearchQuery::new("hello", 1)?);

        let results = index.search(&query).await?;
        assert!(
            !results.is_empty(),
            "Should find documents through standard interface"
        );

        // Test sync/flush operations
        index.sync().await?;
        index.flush().await?;

        Ok(())
    }
}
