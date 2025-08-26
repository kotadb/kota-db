//! Test that symbol relationships are properly routed to graph storage backend

use anyhow::Result;
use kotadb::factory::create_symbol_storage_with_graph;
use kotadb::graph_storage::{GraphStorage, GraphStorageConfig};
use kotadb::native_graph_storage::NativeGraphStorage;
use kotadb::parsing::{CodeParser, SupportedLanguage};
use kotadb::symbol_storage::SymbolStorage;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_relationships_routed_to_graph_storage() -> Result<()> {
    // Create temporary directory for test
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Create document storage
    let document_storage = kotadb::file_storage::create_file_storage(db_path, Some(100)).await?;
    
    // Create graph storage for relationships
    let graph_path = Path::new(db_path).join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;
    let graph_config = GraphStorageConfig::default();
    let graph_storage = NativeGraphStorage::new(graph_path.clone(), graph_config.clone()).await?;
    
    // Create symbol storage with both backends
    let mut symbol_storage = SymbolStorage::with_graph_storage(
        Box::new(document_storage),
        Box::new(graph_storage),
    ).await?;
    
    // Create test code with clear relationships
    let rust_code = r#"
use std::collections::HashMap;

struct Calculator {
    memory: f64,
}

impl Calculator {
    fn new() -> Self {
        Self { memory: 0.0 }
    }
    
    fn add(&mut self, x: f64, y: f64) -> f64 {
        let result = x + y;
        self.store_memory(result);
        result
    }
    
    fn store_memory(&mut self, value: f64) {
        self.memory = value;
    }
}

fn main() {
    let mut calc = Calculator::new();
    let sum = calc.add(5.0, 3.0);
    println!("Sum: {}", sum);
}
"#;

    // Parse the code
    let mut parser = CodeParser::new()?;
    let parsed_code = parser.parse_content(rust_code, SupportedLanguage::Rust)?;
    
    // Extract symbols
    let symbol_ids = symbol_storage.extract_symbols(
        Path::new("test_calculator.rs"),
        parsed_code,
        Some("test_repo".to_string()),
    ).await?;
    
    assert!(!symbol_ids.is_empty(), "Should extract symbols");
    
    // Build dependency graph - this should route relationships to graph storage
    symbol_storage.build_dependency_graph().await?;
    
    // Verify relationships were created
    let relationship_count = symbol_storage.get_relationships_count();
    println!("Created {} relationships", relationship_count);
    
    // Now verify the relationships are actually in the graph storage
    // by creating a new instance and checking the stats
    let graph_storage_check = NativeGraphStorage::new(graph_path, graph_config).await?;
    let graph_stats = graph_storage_check.get_graph_stats().await?;
    
    // The graph storage should have nodes and edges
    assert!(
        graph_stats.node_count > 0,
        "Graph storage should have nodes. Found: {}",
        graph_stats.node_count
    );
    
    assert!(
        graph_stats.edge_count > 0,
        "Graph storage should have edges (relationships). Found: {}",
        graph_stats.edge_count
    );
    
    println!("✅ Graph storage stats:");
    println!("   - Nodes: {}", graph_stats.node_count);
    println!("   - Edges: {}", graph_stats.edge_count);
    println!("   - Nodes by type: {:?}", graph_stats.nodes_by_type);
    println!("   - Edges by type: {:?}", graph_stats.edges_by_type);
    
    // Test that we can query relationships through graph storage
    if !symbol_ids.is_empty() {
        let test_symbol_id = symbol_ids[0];
        let edges = graph_storage_check.get_edges(
            test_symbol_id, 
            petgraph::Direction::Outgoing
        ).await?;
        
        println!("   - Outgoing edges from first symbol: {}", edges.len());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_graph_storage_factory_function() -> Result<()> {
    // Create temporary directory
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Use factory function to create symbol storage with graph
    let symbol_storage = create_symbol_storage_with_graph(db_path, Some(100)).await?;
    
    // Verify it was created successfully
    let storage = symbol_storage.lock().await;
    let stats = storage.get_stats();
    
    // Initially should be empty
    assert_eq!(stats.total_symbols, 0);
    assert_eq!(stats.relationship_count, 0);
    
    // Drop lock before directory cleanup
    drop(storage);
    
    // Verify graph directory was created
    let graph_path = Path::new(db_path).join("graph");
    assert!(graph_path.exists(), "Graph directory should be created");
    
    Ok(())
}

#[tokio::test] 
async fn test_batch_relationship_insertion() -> Result<()> {
    // Create temporary directory
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Create storages
    let document_storage = kotadb::file_storage::create_file_storage(db_path, Some(100)).await?;
    let graph_path = Path::new(db_path).join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;
    let graph_config = GraphStorageConfig::default();
    let graph_storage = NativeGraphStorage::new(graph_path.clone(), graph_config.clone()).await?;
    
    // Create symbol storage with both backends
    let mut symbol_storage = SymbolStorage::with_graph_storage(
        Box::new(document_storage),
        Box::new(graph_storage),
    ).await?;
    
    // Create code with many relationships to test batching
    let rust_code = r#"
mod module_a {
    pub fn func_a() -> i32 { 1 }
    pub fn func_b() -> i32 { func_a() + 2 }
    pub fn func_c() -> i32 { func_b() + func_a() }
}

mod module_b {
    use super::module_a;
    
    pub fn func_d() -> i32 { 
        module_a::func_a() + module_a::func_b() 
    }
    pub fn func_e() -> i32 { 
        func_d() + module_a::func_c() 
    }
}

fn main() {
    let x = module_a::func_c();
    let y = module_b::func_e();
    println!("{} {}", x, y);
}
"#;

    // Parse and extract
    let mut parser = CodeParser::new()?;
    let parsed_code = parser.parse_content(rust_code, SupportedLanguage::Rust)?;
    
    let symbol_ids = symbol_storage.extract_symbols(
        Path::new("test_modules.rs"),
        parsed_code,
        Some("test_repo".to_string()),
    ).await?;
    
    assert!(!symbol_ids.is_empty(), "Should extract symbols");
    
    // Build dependency graph - should batch insert
    symbol_storage.build_dependency_graph().await?;
    
    // Check graph storage directly
    let graph_storage_check = NativeGraphStorage::new(graph_path, graph_config).await?;
    let stats = graph_storage_check.get_graph_stats().await?;
    
    assert!(stats.node_count > 5, "Should have multiple nodes");
    assert!(stats.edge_count > 0, "Should have edges from batch insertion");
    
    println!("✅ Batch insertion successful:");
    println!("   - Batched {} nodes", stats.node_count);
    println!("   - Batched {} edges", stats.edge_count);
    
    Ok(())
}