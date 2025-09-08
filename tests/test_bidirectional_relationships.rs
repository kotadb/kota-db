//! Test bidirectional relationship graph construction (Issue #291)

use anyhow::Result;
use kotadb::create_file_storage;
use kotadb::parsing::CodeParser;
use kotadb::symbol_storage::SymbolStorage;
use tempfile::TempDir;
use tracing::info;

#[tokio::test]
async fn test_bidirectional_relationships() -> Result<()> {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    info!("=== Testing Bidirectional Relationship Graph (Issue #291) ===");

    // Create test environment
    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000)).await?;
    let mut symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    // Create test code with clear dependencies
    let test_code = r#"
// Test code for relationship extraction
use std::collections::HashMap;

fn main() {
    let data = process_data();
    print_result(data);
    let validated = validate_input(data);
    if validated {
        finalize();
    }
}

fn process_data() -> String {
    format_output("processed")
}

fn print_result(result: String) {
    println!("{}", result);
}

fn validate_input(input: String) -> bool {
    !input.is_empty()
}

fn format_output(msg: &str) -> String {
    msg.to_string()
}

fn finalize() {
    println!("Done");
}
"#;

    // Store the test code
    // Write the test code to a real file so DependencyExtractor can analyze it
    let test_file_path = temp_dir.path().join("test_relationships.rs");
    tokio::fs::write(&test_file_path, test_code).await?;

    // Extract symbols
    let mut parser = CodeParser::new()?;
    let parsed = parser.parse_content(test_code, kotadb::parsing::SupportedLanguage::Rust)?;

    // Add symbols to storage
    symbol_storage
        .extract_symbols(&test_file_path, parsed, None, None)
        .await?;

    info!(
        "Extracted {} symbols",
        symbol_storage.get_stats().total_symbols
    );

    // Build dependency graph using the new DependencyExtractor approach
    symbol_storage.build_dependency_graph().await?;

    info!(
        "Built {} relationships",
        symbol_storage.get_relationships_count()
    );

    // Convert to dependency graph for queries
    let dep_graph = symbol_storage.to_dependency_graph().await?;

    // Test: Find callers of process_data (should find main)
    let process_data_symbol = dep_graph
        .name_to_symbol
        .iter()
        .find(|(name, _)| name.contains("process_data"))
        .map(|(_, id)| *id);

    if let Some(process_data_id) = process_data_symbol {
        let dependents = dep_graph.find_dependents(process_data_id);
        info!(
            "Functions that call process_data: {} dependents found",
            dependents.len()
        );

        // We expect main to be a dependent of process_data
        assert!(
            !dependents.is_empty(),
            "process_data should have dependents (main calls it)"
        );

        // Check if main is among the dependents
        let has_main_dependent = dependents.iter().any(|(id, _)| {
            dep_graph
                .symbol_to_node
                .get(id)
                .map(|node_idx| &dep_graph.graph[*node_idx])
                .map(|node| node.qualified_name.contains("main"))
                .unwrap_or(false)
        });

        assert!(
            has_main_dependent,
            "main should be a dependent of process_data"
        );
    } else {
        panic!("Could not find process_data symbol");
    }

    // Test: Find callers of format_output (should find process_data)
    let format_output_symbol = dep_graph
        .name_to_symbol
        .iter()
        .find(|(name, _)| name.contains("format_output"))
        .map(|(_, id)| *id);

    if let Some(format_output_id) = format_output_symbol {
        let dependents = dep_graph.find_dependents(format_output_id);
        info!(
            "Functions that call format_output: {} dependents found",
            dependents.len()
        );

        assert!(
            !dependents.is_empty(),
            "format_output should have dependents (process_data calls it)"
        );
    }

    // Test: Verify bidirectional nature
    // If A depends on B, then B should have A as a dependent
    let main_symbol = dep_graph
        .name_to_symbol
        .iter()
        .find(|(name, _)| name.contains("main"))
        .map(|(_, id)| *id);

    if let Some(main_id) = main_symbol {
        let main_dependencies = dep_graph.find_dependencies(main_id);
        info!("main has {} dependencies", main_dependencies.len());

        // For each dependency of main, main should be in their dependents
        for (dep_id, _) in &main_dependencies {
            let dep_dependents = dep_graph.find_dependents(*dep_id);
            let main_is_dependent = dep_dependents.iter().any(|(id, _)| id == &main_id);

            assert!(
                main_is_dependent,
                "Bidirectional relationship broken: main depends on a function, but that function doesn't list main as a dependent"
            );
        }
    }

    info!("✅ All bidirectional relationship tests passed!");
    info!("✅ Issue #291 fix verified: Dependents arrays are properly populated");

    Ok(())
}
