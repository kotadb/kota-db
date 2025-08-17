//! Performance benchmarks for symbol index

#[cfg(feature = "tree-sitter-parsing")]
mod symbol_index_performance_tests {
    use anyhow::Result;
    use std::time::Instant;
    use tempfile::TempDir;

    use kotadb::contracts::Index;
    use kotadb::parsing::SymbolType;
    use kotadb::symbol_index::{CodeQuery, SymbolIndex};
    use kotadb::types::{ValidatedDocumentId, ValidatedPath};

    #[tokio::test]
    async fn test_symbol_search_performance() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(1000))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        // Create a moderately sized test dataset
        for i in 0..100 {
            let rust_code = format!(
                r#"
use std::collections::HashMap;

fn function_{i}() -> i32 {{
    {i}
}}

fn calculate_something_{i}(input: i32) -> i32 {{
    input * {i}
}}

struct DataStruct{i} {{
    field: i32,
    name: String,
}}

impl DataStruct{i} {{
    fn new() -> Self {{
        Self {{
            field: {i},
            name: "test".to_string(),
        }}
    }}
    
    fn process(&self, value: i32) -> i32 {{
        self.field + value
    }}
}}
"#,
                i = i
            );

            let doc_id = ValidatedDocumentId::new();
            let path = ValidatedPath::new(format!("test_{}.rs", i))?;
            index
                .insert_with_content(doc_id, path, rust_code.as_bytes())
                .await?;
        }

        // Warm up the index
        let warmup_query = CodeQuery::SymbolSearch {
            name: "function".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: true,
        };
        let _ = index.search_code(&warmup_query).await?;

        // Test 1: Symbol search performance
        let start = Instant::now();
        let query = CodeQuery::SymbolSearch {
            name: "calculate".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: true,
        };
        let results = index.search_code(&query).await?;
        let duration = start.elapsed();

        println!(
            "Symbol search took: {:?} for {} results",
            duration,
            results.len()
        );
        assert!(
            duration.as_millis() < 10,
            "Symbol search should complete in under 10ms, took: {:?}",
            duration
        );
        assert!(!results.is_empty(), "Should find some results");

        // Test 2: Struct type search performance
        let start = Instant::now();
        let struct_query = CodeQuery::SymbolSearch {
            name: "DataStruct".to_string(),
            symbol_types: Some(vec![SymbolType::Struct]),
            fuzzy: true,
        };
        let struct_results = index.search_code(&struct_query).await?;
        let duration = start.elapsed();

        println!(
            "Struct search took: {:?} for {} results",
            duration,
            struct_results.len()
        );
        assert!(
            duration.as_millis() < 10,
            "Struct search should complete in under 10ms, took: {:?}",
            duration
        );

        // Test 3: Pattern search performance
        let start = Instant::now();
        let pattern_query = CodeQuery::PatternSearch {
            pattern: kotadb::symbol_index::CodePattern::ErrorHandling,
            scope: kotadb::symbol_index::SearchScope::All,
        };
        let pattern_results = index.search_code(&pattern_query).await?;
        let duration = start.elapsed();

        println!(
            "Pattern search took: {:?} for {} results",
            duration,
            pattern_results.len()
        );
        assert!(
            duration.as_millis() < 50,
            "Pattern search should complete in under 50ms, took: {:?}",
            duration
        );

        // Test 4: Signature search performance
        let start = Instant::now();
        let sig_query = CodeQuery::SignatureSearch {
            pattern: "i32".to_string(),
            language: Some("rust".to_string()),
        };
        let sig_results = index.search_code(&sig_query).await?;
        let duration = start.elapsed();

        println!(
            "Signature search took: {:?} for {} results",
            duration,
            sig_results.len()
        );
        assert!(
            duration.as_millis() < 20,
            "Signature search should complete in under 20ms, took: {:?}",
            duration
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_concurrent_searches() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(1000))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        // Add test data
        for i in 0..50 {
            let rust_code = format!(
                r#"
fn test_function_{i}() -> i32 {{ {i} }}
fn helper_function_{i}(x: i32) -> i32 {{ x + {i} }}
struct TestStruct{i} {{ value: i32 }}
"#,
                i = i
            );

            let doc_id = ValidatedDocumentId::new();
            let path = ValidatedPath::new(format!("file_{}.rs", i))?;
            index
                .insert_with_content(doc_id, path, rust_code.as_bytes())
                .await?;
        }

        // Test concurrent searches
        let start = Instant::now();

        let queries = vec![
            CodeQuery::SymbolSearch {
                name: "test_function".to_string(),
                symbol_types: Some(vec![SymbolType::Function]),
                fuzzy: true,
            },
            CodeQuery::SymbolSearch {
                name: "helper".to_string(),
                symbol_types: Some(vec![SymbolType::Function]),
                fuzzy: true,
            },
            CodeQuery::SymbolSearch {
                name: "TestStruct".to_string(),
                symbol_types: Some(vec![SymbolType::Struct]),
                fuzzy: true,
            },
        ];

        // Run searches sequentially (simulating concurrent usage pattern)
        let mut total_results = 0;
        for query in queries {
            let results = index.search_code(&query).await?;
            total_results += results.len();
        }

        let duration = start.elapsed();
        println!(
            "Sequential searches took: {:?} for {} total results",
            duration, total_results
        );
        assert!(
            duration.as_millis() < 30,
            "Sequential searches should complete in under 30ms, took: {:?}",
            duration
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_large_symbol_dataset_performance() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let storage =
            kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(10000))
                .await?;

        let mut index = SymbolIndex::new(temp_path, Box::new(storage)).await?;

        println!("Building large symbol dataset...");
        let start_insert = Instant::now();

        // Create a larger dataset to test scaling
        for i in 0..200 {
            let rust_code = format!(
                r#"
mod module_{i} {{
    use std::collections::HashMap;
    
    pub fn public_function_{i}() -> i32 {{ {i} }}
    fn private_function_{i}() -> String {{ "test{i}".to_string() }}
    
    pub struct PublicStruct{i} {{
        pub field: i32,
        private_field: String,
    }}
    
    impl PublicStruct{i} {{
        pub fn new(value: i32) -> Self {{
            Self {{
                field: value,
                private_field: format!("value_{{}}", value),
            }}
        }}
        
        pub fn get_value(&self) -> i32 {{
            self.field
        }}
        
        fn private_helper(&self) -> String {{
            self.private_field.clone()
        }}
    }}
    
    #[derive(Debug)]
    enum DataType{i} {{
        Integer(i32),
        Text(String),
        Boolean(bool),
    }}
}}
"#,
                i = i
            );

            let doc_id = ValidatedDocumentId::new();
            let path = ValidatedPath::new(format!("large_file_{}.rs", i))?;
            index
                .insert_with_content(doc_id, path, rust_code.as_bytes())
                .await?;

            if i % 50 == 0 {
                println!("Inserted {} files", i + 1);
            }
        }

        let insert_duration = start_insert.elapsed();
        println!("Dataset insertion took: {:?}", insert_duration);

        // Test search performance on large dataset
        let start = Instant::now();
        let query = CodeQuery::SymbolSearch {
            name: "public_function".to_string(),
            symbol_types: Some(vec![SymbolType::Function]),
            fuzzy: true,
        };
        let results = index.search_code(&query).await?;
        let duration = start.elapsed();

        println!(
            "Search on large dataset took: {:?} for {} results",
            duration,
            results.len()
        );
        assert!(
            duration.as_millis() < 15,
            "Large dataset search should complete in under 15ms, took: {:?}",
            duration
        );
        assert!(results.len() >= 200, "Should find all public functions");

        Ok(())
    }
}
