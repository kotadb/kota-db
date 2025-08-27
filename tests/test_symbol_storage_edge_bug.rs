//! Test to reproduce Issue #341 at the SymbolStorage level
//! This test focuses on the build_dependency_graph -> graph storage persistence bug

use anyhow::Result;
use kotadb::{
    create_file_storage,
    graph_storage::GraphStorageConfig,
    native_graph_storage::NativeGraphStorage,
    parsing::{CodeParser, SupportedLanguage},
    symbol_storage::SymbolStorage,
};
use tempfile::TempDir;

/// Test that reproduces the exact bug: edges lost during symbol storage build_dependency_graph
#[tokio::test]
async fn test_symbol_storage_dependency_graph_edge_loss() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path();
    let storage_path = db_path.join("storage");
    let graph_path = storage_path.join("graph");

    tokio::fs::create_dir_all(&storage_path).await?;
    tokio::fs::create_dir_all(&graph_path).await?;

    println!(
        "Testing symbol storage edge persistence bug at: {:?}",
        graph_path
    );

    // Create test code with clear function call relationship
    let test_code = r#"
pub fn main() {
    helper_function();
}

pub fn helper_function() {
    println!("Called from main");
}
"#;

    // Write test file
    let test_file_path = temp_dir.path().join("test.rs");
    tokio::fs::write(&test_file_path, test_code).await?;

    // Phase 1: Extract symbols and build dependency graph
    {
        let file_storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        let graph_config = GraphStorageConfig::default();
        let graph_storage = NativeGraphStorage::new(&graph_path, graph_config).await?;

        let mut symbol_storage =
            SymbolStorage::with_graph_storage(Box::new(file_storage), Box::new(graph_storage))
                .await?;

        // Extract symbols using the same process as the main CLI
        let mut code_parser = CodeParser::new()?;
        let parsed_code = code_parser.parse_content(test_code, SupportedLanguage::Rust)?;

        let symbol_ids = symbol_storage
            .extract_symbols(&test_file_path, parsed_code, Some("test-repo".to_string()))
            .await?;

        println!("Extracted {} symbols", symbol_ids.len());
        assert!(!symbol_ids.is_empty(), "Should extract symbols");

        // Build dependency graph - THIS IS WHERE THE BUG OCCURS
        println!("Building dependency graph...");
        symbol_storage.build_dependency_graph().await?;

        let stats = symbol_storage.get_dependency_stats();
        println!(
            "Dependency stats: {} total relationships",
            stats.total_relationships
        );

        // Check if any relationships were found at all
        if stats.total_relationships > 0 {
            println!("✅ Found {} relationships", stats.total_relationships);
        } else {
            println!("❌ No relationships found during dependency graph build");
        }

        // CRITICAL: Flush symbol storage (which should flush graph storage)
        println!("Flushing symbol storage...");
        symbol_storage.flush_storage().await?;

        // Check what's actually in the graph storage after flush
        println!("Checking direct graph storage after flush...");

        // Drop symbol storage to free locks
        drop(symbol_storage);
    }

    // Phase 2: Check if edges were persisted by directly accessing graph storage
    {
        println!("Phase 2: Checking edge persistence...");
        let config = GraphStorageConfig::default();
        let graph_storage = NativeGraphStorage::new(&graph_path, config).await?;

        // Check edges directory
        let edges_dir = graph_path.join("edges");
        let edges_exist = edges_dir.exists();
        println!("Edges directory exists: {}", edges_exist);

        let mut total_files = 0;
        let mut total_size = 0;

        if edges_exist {
            let mut entries = tokio::fs::read_dir(&edges_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("page") {
                    total_files += 1;
                    let file_size = entry.metadata().await?.len();
                    total_size += file_size;
                    println!(
                        "Edge file: {:?}, size: {} bytes",
                        entry.file_name(),
                        file_size
                    );
                }
            }
        }

        println!(
            "Total edge files: {}, total size: {} bytes",
            total_files, total_size
        );

        // THIS IS THE KEY BUG: Even if relationships were found during build_dependency_graph,
        // they may not be persisted to the graph storage correctly

        if total_files == 0 {
            println!("🐛 BUG #341 REPRODUCED: No edge files found despite build_dependency_graph completing");
            println!("   This indicates edges are not being transferred from dependency graph to graph storage");
        } else {
            println!("✅ Edge files found - the bug may be fixed or not reproduced in this case");
        }
    }

    Ok(())
}

/// Test to specifically examine the build_dependency_graph -> graph storage transfer
#[tokio::test]
async fn test_dependency_graph_to_graph_storage_transfer() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path();
    let storage_path = db_path.join("storage");
    let graph_path = storage_path.join("graph");

    tokio::fs::create_dir_all(&storage_path).await?;
    tokio::fs::create_dir_all(&graph_path).await?;

    println!("Testing dependency graph -> graph storage transfer");

    let file_storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

    let graph_config = GraphStorageConfig::default();
    let graph_storage = NativeGraphStorage::new(&graph_path, graph_config).await?;

    let mut symbol_storage =
        SymbolStorage::with_graph_storage(Box::new(file_storage), Box::new(graph_storage)).await?;

    // Create a simple test with a function that calls another function
    let test_code = r#"
fn caller() {
    target();
}

fn target() {
    // target function
}
"#;

    let test_file_path = temp_dir.path().join("simple.rs");
    tokio::fs::write(&test_file_path, test_code).await?;

    // Extract symbols
    let mut code_parser = CodeParser::new()?;
    let parsed_code = code_parser.parse_content(test_code, SupportedLanguage::Rust)?;
    let _symbol_ids = symbol_storage
        .extract_symbols(&test_file_path, parsed_code, Some("test-repo".to_string()))
        .await?;

    println!("Step 1: Symbols extracted");

    // Build dependency graph and capture statistics
    println!("Step 2: Building dependency graph...");
    symbol_storage.build_dependency_graph().await?;

    let stats = symbol_storage.get_dependency_stats();
    println!(
        "Step 3: Dependency graph built with {} relationships",
        stats.total_relationships
    );

    // The critical question: are these relationships transferred to graph storage?
    println!("Step 4: Flushing to ensure persistence...");
    symbol_storage.flush_storage().await?;

    println!("Step 5: Checking if edges were persisted...");
    let edges_dir = graph_path.join("edges");

    let file_count = if edges_dir.exists() {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&edges_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("page") {
                count += 1;
                println!("  Found edge file: {:?}", entry.file_name());
            }
        }
        count
    } else {
        0
    };

    println!(
        "RESULT: {} dependency relationships -> {} edge files",
        stats.total_relationships, file_count
    );

    if stats.total_relationships > 0 && file_count == 0 {
        println!("🐛 BUG CONFIRMED: Relationships found but not persisted to graph storage");
    } else if stats.total_relationships == 0 {
        println!(
            "⚠️  No relationships found during dependency analysis - may be a different issue"
        );
    } else {
        println!("✅ Relationships properly persisted");
    }

    Ok(())
}
