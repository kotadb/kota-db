//! Test that manually created relationships are properly routed to graph storage

use anyhow::Result;
use kotadb::graph_storage::{GraphStorage, GraphStorageConfig};
use kotadb::native_graph_storage::NativeGraphStorage;
use kotadb::symbol_storage::{SymbolStorage, SymbolRelation, RelationType};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_manual_relationships_routed_to_graph() -> Result<()> {
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
    
    // Create some test symbol IDs
    let symbol_a = Uuid::new_v4();
    let symbol_b = Uuid::new_v4();
    let symbol_c = Uuid::new_v4();
    
    // Create relationships manually
    let relation_1 = SymbolRelation {
        from_id: symbol_a,
        to_id: symbol_b,
        relation_type: RelationType::Calls,
        metadata: HashMap::new(),
    };
    
    let relation_2 = SymbolRelation {
        from_id: symbol_b,
        to_id: symbol_c,
        relation_type: RelationType::Imports,
        metadata: HashMap::new(),
    };
    
    let relation_3 = SymbolRelation {
        from_id: symbol_a,
        to_id: symbol_c,
        relation_type: RelationType::Extends,
        metadata: HashMap::new(),
    };
    
    // Add relationships - these should be routed to graph storage
    symbol_storage.add_relationship(relation_1).await?;
    symbol_storage.add_relationship(relation_2).await?;
    symbol_storage.add_relationship(relation_3).await?;
    
    // Verify in-memory storage has the relationships
    let relationship_count = symbol_storage.get_relationships_count();
    assert_eq!(relationship_count, 3, "Should have 3 relationships in memory");
    
    // Now verify the relationships are in graph storage
    let graph_storage_check = NativeGraphStorage::new(graph_path, graph_config).await?;
    
    // Check edges from symbol_a
    let edges_from_a = graph_storage_check.get_edges(
        symbol_a,
        petgraph::Direction::Outgoing
    ).await?;
    
    assert_eq!(
        edges_from_a.len(), 
        2, 
        "Symbol A should have 2 outgoing edges (to B and C)"
    );
    
    // Check edges from symbol_b
    let edges_from_b = graph_storage_check.get_edges(
        symbol_b,
        petgraph::Direction::Outgoing
    ).await?;
    
    assert_eq!(
        edges_from_b.len(),
        1,
        "Symbol B should have 1 outgoing edge (to C)"
    );
    
    // Check incoming edges to symbol_c
    let edges_to_c = graph_storage_check.get_edges(
        symbol_c,
        petgraph::Direction::Incoming
    ).await?;
    
    assert_eq!(
        edges_to_c.len(),
        2,
        "Symbol C should have 2 incoming edges (from A and B)"
    );
    
    println!("✅ Manual relationship routing test passed!");
    println!("   - Created 3 relationships");
    println!("   - All relationships successfully routed to graph storage");
    println!("   - Graph queries returning correct edge counts");
    
    Ok(())
}