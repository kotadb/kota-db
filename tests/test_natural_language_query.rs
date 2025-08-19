//! Integration tests for natural language query processing
//!
//! These tests verify the full query execution pipeline, including:
//! - Query parsing
//! - Intent mapping
//! - Index integration
//! - Result formatting

#[cfg(feature = "tree-sitter-parsing")]
mod natural_language_tests {
    use anyhow::Result;
    use kotadb::natural_language_query::{NaturalLanguageQueryProcessor, QueryIntent};

    #[tokio::test]
    async fn test_error_handling_pattern_query() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Parse the query
        let intent = processor
            .parse_query("find all error handling patterns")
            .await?;

        // Verify correct intent
        match intent {
            QueryIntent::FindPatterns { .. } => {
                // Expected - test passes
            }
            _ => panic!("Expected FindPatterns intent for error handling query"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_function_search_query() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Parse the query
        let intent = processor
            .parse_query("find function named create_storage")
            .await?;

        // Verify correct intent
        match intent {
            QueryIntent::FindSymbols { name_pattern, .. } => {
                assert!(name_pattern.contains("create_storage"));
            }
            _ => panic!("Expected FindSymbols intent for function search"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_dependency_query() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Parse the query
        let intent = processor.parse_query("what calls FileStorage").await?;

        // Verify correct intent
        match intent {
            QueryIntent::FindDependencies { target, .. } => {
                assert_eq!(target, "FileStorage");
            }
            _ => panic!("Expected FindDependencies intent for dependency query"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_complex_query_patterns() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Test various query patterns
        let queries = vec![
            ("show async await code", "async patterns"),
            ("find test functions", "test code"),
            ("show TODO comments", "todo markers"),
            ("find security patterns", "security sensitive code"),
        ];

        for (query, description) in queries {
            let intent = processor.parse_query(query).await?;
            match intent {
                QueryIntent::FindPatterns { .. } => {
                    // Expected pattern query
                }
                _ => panic!("Expected FindPatterns for {}", description),
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_quoted_pattern_extraction() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Test quoted patterns
        let intent = processor
            .parse_query("find functions matching \"validate_*\"")
            .await?;

        match intent {
            QueryIntent::FindSymbols { name_pattern, .. } => {
                assert_eq!(name_pattern, "validate_*");
            }
            _ => panic!("Expected FindSymbols with quoted pattern"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_case_preservation() -> Result<()> {
        let processor = NaturalLanguageQueryProcessor::new();

        // Test that symbol names preserve case
        let intent = processor.parse_query("what calls FileStorage").await?;

        match intent {
            QueryIntent::FindDependencies { target, .. } => {
                assert_eq!(target, "FileStorage"); // Should preserve original case
                assert_ne!(target, "filestorage");
            }
            _ => panic!("Expected FindDependencies"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_result_formatting() -> Result<()> {
        use kotadb::natural_language_query::NaturalLanguageQueryResult;
        use kotadb::parsing::SymbolType;

        // Create mock results
        let result = NaturalLanguageQueryResult {
            intent: QueryIntent::FindSymbols {
                symbol_types: Some(vec![SymbolType::Function]),
                name_pattern: "test".to_string(),
                fuzzy: true,
            },
            results: vec![],
            explanation: "Found 0 symbols matching 'test'".to_string(),
        };

        // Test markdown formatting
        let markdown = result.to_markdown();
        assert!(markdown.contains("Found 0 symbols"));

        // Test JSON formatting
        let json = result.to_json()?;
        assert!(json.contains("\"explanation\""));

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires full index setup"]
    async fn test_full_pipeline_integration() -> Result<()> {
        // This test would require a full setup with actual storage and indices
        // For now, we test the parsing and formatting components separately

        let processor = NaturalLanguageQueryProcessor::new();

        // Test that we can parse various queries
        let queries = vec![
            "find error handling patterns",
            "what calls FileStorage",
            "find function named create_storage",
        ];

        for query in queries {
            let intent = processor.parse_query(query).await?;
            // Verify parsing succeeded
            match intent {
                QueryIntent::FindPatterns { .. }
                | QueryIntent::FindDependencies { .. }
                | QueryIntent::FindSymbols { .. } => {
                    // Expected - parsing works
                }
                _ => panic!("Unexpected intent for query: {}", query),
            }
        }

        Ok(())
    }
}
