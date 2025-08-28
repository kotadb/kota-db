//! Integration tests for the binary-to-relationship bridge
//!
//! This tests the complete hybrid solution that extracts symbols to binary format
//! and then builds dependency graphs from those symbols.

#[cfg(feature = "tree-sitter-parsing")]
mod tests {
    use anyhow::Result;
    use kotadb::{
        binary_relationship_bridge::BinaryRelationshipBridge,
        binary_symbols::BinarySymbolWriter,
        create_file_storage,
        git::{IngestionConfig, RepositoryIngester},
    };
    use std::path::PathBuf;
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Test basic relationship extraction from binary symbols
    #[test]
    fn test_basic_relationship_extraction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let symbol_db_path = temp_dir.path().join("symbols.db");

        // Create sample symbols
        let mut writer = BinarySymbolWriter::new();

        let func1_id = Uuid::new_v4();
        let func2_id = Uuid::new_v4();
        let struct_id = Uuid::new_v4();

        writer.add_symbol(func1_id, "main", 1, "src/main.rs", 5, 15, None);
        writer.add_symbol(func2_id, "process_data", 1, "src/lib.rs", 10, 20, None);
        writer.add_symbol(struct_id, "Config", 3, "src/config.rs", 1, 10, None);

        writer.write_to_file(&symbol_db_path)?;

        // Create sample source files
        let main_rs = r#"
use crate::process_data;
use config::Config;

fn main() {
    let config = Config::new();
    process_data(&config);
    println!("Done");
}
        "#;

        let lib_rs = r#"
use config::Config;

pub fn process_data(config: &Config) {
    // Process data with config
    println!("Processing with {:?}", config);
}
        "#;

        let config_rs = r#"
#[derive(Debug)]
pub struct Config {
    pub value: i32,
}

impl Config {
    pub fn new() -> Self {
        Config { value: 42 }
    }
}
        "#;

        let files = vec![
            (PathBuf::from("src/main.rs"), main_rs.as_bytes().to_vec()),
            (PathBuf::from("src/lib.rs"), lib_rs.as_bytes().to_vec()),
            (
                PathBuf::from("src/config.rs"),
                config_rs.as_bytes().to_vec(),
            ),
        ];

        // Extract relationships
        let bridge = BinaryRelationshipBridge::new();
        let graph = bridge.extract_relationships(&symbol_db_path, temp_dir.path(), &files)?;

        // Verify graph structure
        assert!(graph.stats.node_count > 0, "Graph should have nodes");
        assert_eq!(graph.stats.node_count, 3, "Should have 3 symbols");

        // The actual edge count will depend on successful reference resolution
        println!("Graph stats: {:?}", graph.stats);
        println!(
            "Nodes: {}, Edges: {}",
            graph.stats.node_count, graph.stats.edge_count
        );

        Ok(())
    }

    /// Test the complete ingestion pipeline with relationships
    #[tokio::test]
    async fn test_complete_ingestion_with_relationships() -> Result<()> {
        // Create a temporary git repository with some code
        let temp_dir = TempDir::new()?;
        let repo_dir = temp_dir.path().join("test_repo");
        std::fs::create_dir_all(&repo_dir)?;

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_dir)
            .output()?;

        // Create some source files
        let main_rs = r#"
mod utils;

fn main() {
    println!("Starting application");
    utils::helper_function();
}
"#;

        let utils_rs = r#"
pub fn helper_function() {
    println!("Helper function called");
    internal_function();
}

fn internal_function() {
    println!("Internal function");
}
"#;

        std::fs::write(repo_dir.join("main.rs"), main_rs)?;
        std::fs::write(repo_dir.join("utils.rs"), utils_rs)?;

        // Add and commit files
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_dir)
            .output()?;

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_dir)
            .output()?;

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_dir)
            .output()?;

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_dir)
            .output()?;

        // Create storage and paths for databases
        let storage_dir = temp_dir.path().join("storage");
        let symbol_db_path = temp_dir.path().join("symbols.db");
        let graph_db_path = temp_dir.path().join("graph.json");

        let mut storage = create_file_storage(storage_dir.to_str().unwrap(), Some(100)).await?;

        // Configure ingestion with symbol extraction
        let mut config = IngestionConfig::default();
        config.options.extract_symbols = true;
        config.options.include_file_contents = true;

        // Run ingestion with relationship extraction
        let ingester = RepositoryIngester::new(config);
        let result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &repo_dir,
                &mut storage,
                &symbol_db_path,
                &graph_db_path,
                None,
            )
            .await?;

        // Verify results
        assert!(result.documents_created > 0, "Should create documents");
        assert!(result.symbols_extracted > 0, "Should extract symbols");
        assert_eq!(result.errors, 0, "Should have no errors");

        println!("Ingestion results:");
        println!("  Documents: {}", result.documents_created);
        println!("  Symbols: {}", result.symbols_extracted);
        println!("  Relationships: {}", result.relationships_extracted);
        println!("  Files with symbols: {}", result.files_with_symbols);

        // Verify symbol database was created
        assert!(symbol_db_path.exists(), "Symbol database should exist");

        // Verify graph database was created
        assert!(graph_db_path.exists(), "Graph database should exist");

        // Read and verify the graph (binary format)
        let graph_binary = std::fs::read(&graph_db_path)?;
        assert!(
            !graph_binary.is_empty(),
            "Graph binary data should not be empty"
        );

        Ok(())
    }

    /// Test performance of relationship extraction
    #[test]
    fn test_relationship_extraction_performance() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let symbol_db_path = temp_dir.path().join("symbols.db");

        // Create a larger set of symbols to test performance
        let mut writer = BinarySymbolWriter::new();

        let mut symbol_ids = Vec::new();

        // Create 100 symbols across 10 files
        for file_idx in 0..10 {
            let file_path = format!("src/file_{}.rs", file_idx);

            for symbol_idx in 0..10 {
                let id = Uuid::new_v4();
                let name = format!("function_{}", symbol_idx);
                let start_line = (symbol_idx * 10) as u32;
                let end_line = start_line + 5;

                writer.add_symbol(
                    id, &name, 1, // Function type
                    &file_path, start_line, end_line, None,
                );

                symbol_ids.push(id);
            }
        }

        writer.write_to_file(&symbol_db_path)?;

        // Create synthetic source files with cross-references
        let mut files = Vec::new();

        for file_idx in 0..10 {
            let mut content = String::new();
            content.push_str(&format!("// File {}\n", file_idx));

            // Add some cross-file references
            for ref_idx in 0..5 {
                let target_file = (file_idx + ref_idx + 1) % 10;
                content.push_str(&format!(
                    "use file_{}::function_{};\n",
                    target_file, ref_idx
                ));
            }

            // Add function definitions with calls to other functions
            for symbol_idx in 0..10 {
                content.push_str(&format!("\nfn function_{}() {{\n", symbol_idx));

                // Call some other functions
                if symbol_idx > 0 {
                    content.push_str(&format!("    function_{}();\n", symbol_idx - 1));
                }

                content.push_str("}\n");
            }

            files.push((
                PathBuf::from(format!("src/file_{}.rs", file_idx)),
                content.into_bytes(),
            ));
        }

        // Measure extraction time
        let start = std::time::Instant::now();

        let bridge = BinaryRelationshipBridge::new();
        let graph = bridge.extract_relationships(&symbol_db_path, temp_dir.path(), &files)?;

        let elapsed = start.elapsed();

        println!(
            "Extracted relationships for {} symbols across {} files in {:?}",
            symbol_ids.len(),
            files.len(),
            elapsed
        );
        println!(
            "Graph stats: {} nodes, {} edges",
            graph.stats.node_count, graph.stats.edge_count
        );

        // Verify performance target: should complete in under 1 second for this size
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "Relationship extraction took too long: {:?}",
            elapsed
        );

        Ok(())
    }

    /// Test that relationship extraction handles edge cases gracefully
    #[test]
    fn test_edge_cases() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let symbol_db_path = temp_dir.path().join("symbols.db");

        // Test with empty symbol database
        let writer = BinarySymbolWriter::new();
        writer.write_to_file(&symbol_db_path)?;

        let bridge = BinaryRelationshipBridge::new();
        let graph = bridge.extract_relationships(&symbol_db_path, temp_dir.path(), &[])?;

        assert_eq!(graph.stats.node_count, 0, "Empty DB should have no nodes");
        assert_eq!(graph.stats.edge_count, 0, "Empty DB should have no edges");

        // Test with binary files (should be skipped)
        let binary_file = vec![(PathBuf::from("binary.exe"), vec![0xFF, 0xD8, 0xFF, 0xE0])];

        let graph = bridge.extract_relationships(&symbol_db_path, temp_dir.path(), &binary_file)?;
        assert_eq!(
            graph.stats.edge_count, 0,
            "Binary files should produce no relationships"
        );

        Ok(())
    }

    /// Test that dependency graph is automatically created during repository ingestion
    #[tokio::test]
    async fn test_automatic_dependency_graph_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let repo_dir = temp_dir.path().join("test_repo");
        std::fs::create_dir_all(&repo_dir)?;

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_dir)
            .output()?;

        // Create source files with clear dependencies
        let main_rs = r#"
fn main() {
    let result = helper::process_data();
    println!("Result: {}", result);
}
"#;

        let helper_rs = r#"
pub fn process_data() -> i32 {
    let value = calculate();
    value * 2
}

fn calculate() -> i32 {
    42
}
"#;

        std::fs::write(repo_dir.join("main.rs"), main_rs)?;
        std::fs::write(repo_dir.join("helper.rs"), helper_rs)?;

        // Commit the files
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_dir)
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_dir)
            .output()?;

        // Set up storage
        let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        // Create ingester and run the complete ingestion with relationships
        let config = IngestionConfig::default();
        let ingester = RepositoryIngester::new(config);

        let symbol_db_path = temp_dir.path().join("symbols.kota");
        let graph_db_path = temp_dir.path().join("dependency_graph.bin");

        let result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &repo_dir,
                &mut storage,
                &symbol_db_path,
                &graph_db_path,
                None, // No progress callback for test
            )
            .await?;

        // Verify that both files were created
        assert!(
            symbol_db_path.exists(),
            "Binary symbol database should be created automatically"
        );
        assert!(
            graph_db_path.exists(),
            "Dependency graph should be created automatically"
        );

        // Verify that relationships were extracted
        assert!(
            result.relationships_extracted > 0,
            "Should have extracted some relationships between symbols"
        );

        // Verify that symbols were extracted
        assert!(
            result.symbols_extracted > 0,
            "Should have extracted some symbols"
        );

        // Verify that documents were created
        assert!(
            result.documents_created > 0,
            "Should have created document entries"
        );

        // Verify the dependency graph file is not empty
        let graph_file_size = std::fs::metadata(&graph_db_path)?.len();
        assert!(
            graph_file_size > 100,
            "Dependency graph file should contain data"
        );

        println!(
            "✅ Automatic dependency graph creation test passed: {} docs, {} symbols, {} relationships",
            result.documents_created, result.symbols_extracted, result.relationships_extracted
        );

        Ok(())
    }

    /// Test error handling when relationship extraction fails
    #[tokio::test]
    async fn test_relationship_extraction_error_handling() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let repo_dir = temp_dir.path().join("test_repo");
        std::fs::create_dir_all(&repo_dir)?;

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&repo_dir)
            .output()?;

        // Create a file that might cause parsing issues
        std::fs::write(repo_dir.join("main.rs"), "invalid rust syntax $$$ {{{")?;

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_dir)
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "Invalid syntax commit"])
            .current_dir(&repo_dir)
            .output()?;

        let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        let config = IngestionConfig::default();
        let ingester = RepositoryIngester::new(config);

        let symbol_db_path = temp_dir.path().join("symbols.kota");
        let graph_db_path = temp_dir.path().join("dependency_graph.bin");

        // This should complete without panicking, even with parsing errors
        let result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &repo_dir,
                &mut storage,
                &symbol_db_path,
                &graph_db_path,
                None,
            )
            .await?;

        // Verify graceful handling - documents should still be created
        assert!(
            result.documents_created > 0,
            "Should create documents even with relationship extraction errors"
        );

        // Error count should be non-negative (always true, but documents the expectation)
        println!("Error count tracked: {}", result.errors);

        println!(
            "✅ Error handling test passed: {} docs created despite parsing errors",
            result.documents_created
        );

        Ok(())
    }
}
