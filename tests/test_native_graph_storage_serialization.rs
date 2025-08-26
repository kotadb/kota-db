#!/bin/false
//! Test to isolate and reproduce the NativeGraphStorage deserialization corruption
//!
//! This test is designed to reproduce issue #329 where node deserialization fails
//! with format mismatch errors causing complete graph storage dysfunction.

use anyhow::Result;
use kotadb::contracts::Storage;
use kotadb::graph_storage::{GraphEdge, GraphNode, GraphStorage, GraphStorageConfig, NodeLocation};
use kotadb::native_graph_storage::NativeGraphStorage;
use kotadb::symbol_storage::RelationType;
use petgraph::Direction;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;

/// Test that reproduces the exact deserialization failure from issue #329
#[tokio::test]
async fn test_node_serialization_deserialization_corruption() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    // Create graph storage instance
    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Create a test node similar to ones that fail in production
    let node_id = Uuid::new_v4();
    let node = GraphNode {
        id: node_id,
        node_type: "function".to_string(),
        qualified_name: "kotadb::FileStorage::insert".to_string(),
        file_path: "src/file_storage.rs".to_string(),
        location: NodeLocation {
            start_line: 100,
            start_column: 4,
            end_line: 105,
            end_column: 5,
        },
        metadata: HashMap::new(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    // Store the node
    storage.store_node(node_id, node.clone()).await?;

    // Force flush to disk to trigger persistence
    storage.sync().await?;

    // Create a new storage instance to trigger loading from disk
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Try to retrieve the node - this should work but currently fails due to corruption
    let retrieved_node = storage2.get_node(node_id).await?;

    assert!(
        retrieved_node.is_some(),
        "Node should be retrievable after persistence and reload"
    );
    let retrieved = retrieved_node.unwrap();
    assert_eq!(retrieved.qualified_name, node.qualified_name);
    assert_eq!(retrieved.node_type, node.node_type);

    Ok(())
}

/// Test the specific page format that's causing the corruption
#[tokio::test]
async fn test_page_format_corruption() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Add multiple nodes to trigger page creation
    let mut node_ids = Vec::new();
    for i in 0..10 {
        let node_id = Uuid::new_v4();
        let node = GraphNode {
            id: node_id,
            node_type: format!("type_{}", i),
            qualified_name: format!("test::symbol_{}", i),
            file_path: format!("test_{}.rs", i),
            location: NodeLocation {
                start_line: i * 10,
                start_column: 0,
                end_line: i * 10 + 5,
                end_column: 10,
            },
            metadata: HashMap::new(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        storage.store_node(node_id, node).await?;
        node_ids.push(node_id);
    }

    // Force persistence
    storage.sync().await?;

    // Check that page files are created
    let nodes_dir = db_path.join("nodes");
    assert!(nodes_dir.exists(), "Nodes directory should be created");

    let mut entries = fs::read_dir(&nodes_dir).await?;
    let mut page_files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("page") {
            page_files.push(entry.path());
        }
    }

    assert!(!page_files.is_empty(), "Page files should be created");

    // Try to create a new storage instance and load the data
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // This should succeed but will fail due to the corruption bug
    for node_id in &node_ids {
        let retrieved = storage2.get_node(*node_id).await?;
        assert!(
            retrieved.is_some(),
            "Each node should be retrievable after reload"
        );
    }

    Ok(())
}

/// Test to examine the raw page data structure and verify fix
#[tokio::test]
async fn test_examine_raw_page_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Create a simple node
    let node_id = Uuid::new_v4();
    let node = GraphNode {
        id: node_id,
        node_type: "test".to_string(),
        qualified_name: "test::simple".to_string(),
        file_path: "test.rs".to_string(),
        location: NodeLocation {
            start_line: 1,
            start_column: 0,
            end_line: 5,
            end_column: 10,
        },
        metadata: HashMap::new(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    storage.store_node(node_id, node.clone()).await?;
    storage.sync().await?;

    // Read the raw page data to examine structure
    let nodes_dir = db_path.join("nodes");
    let mut entries = fs::read_dir(&nodes_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("page") {
            let page_data = fs::read(&path).await?;

            println!("Page file: {:?}", path);
            println!("Page size: {} bytes", page_data.len());
            println!(
                "First 100 bytes: {:?}",
                &page_data[..std::cmp::min(100, page_data.len())]
            );

            // Try to examine the structure
            if page_data.len() >= 8 {
                let magic = &page_data[0..8];
                println!(
                    "Magic bytes: {:?}",
                    std::str::from_utf8(magic).unwrap_or("invalid")
                );
            }
        }
    }

    // Now test that reload works correctly with the fix
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;
    let retrieved = storage2.get_node(node_id).await?;
    assert!(
        retrieved.is_some(),
        "Node should be retrievable with the deserialization fix"
    );
    let retrieved_node = retrieved.unwrap();
    assert_eq!(retrieved_node.qualified_name, node.qualified_name);
    assert_eq!(retrieved_node.node_type, node.node_type);

    Ok(())
}

/// Test the critical serialization round-trip that was broken
#[tokio::test]
async fn test_serialization_round_trip_with_fix() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Create nodes with complex qualified names similar to production data
    let mut node_ids = Vec::new();
    let qualified_names = [
        "kotadb::FileStorage::insert",
        "kotadb::primary_index::BTreeNode::insert_key",
        "kotadb::graph_storage::NativeGraphStorage::load_nodes_from_page",
        "std::collections::HashMap::get",
        "anyhow::Result",
    ];

    for (i, qualified_name) in qualified_names.iter().enumerate() {
        let node_id = Uuid::new_v4();
        let node = GraphNode {
            id: node_id,
            node_type: "function".to_string(),
            qualified_name: qualified_name.to_string(),
            file_path: format!("src/module_{}.rs", i),
            location: NodeLocation {
                start_line: i * 100,
                start_column: 4,
                end_line: i * 100 + 10,
                end_column: 5,
            },
            metadata: HashMap::new(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        storage.store_node(node_id, node).await?;
        node_ids.push((node_id, qualified_name));
    }

    // Force persistence to disk
    storage.sync().await?;
    drop(storage);

    // Create new storage instance to force loading from disk
    // This is where the corruption would happen before the fix
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Verify all nodes can be retrieved correctly
    for (node_id, expected_name) in &node_ids {
        let retrieved = storage2.get_node(*node_id).await?;
        assert!(
            retrieved.is_some(),
            "Node {} should be retrievable",
            expected_name
        );
        let node = retrieved.unwrap();
        assert_eq!(node.qualified_name, **expected_name);
        println!("✅ Successfully retrieved: {}", expected_name);
    }

    Ok(())
}

/// Test both node and edge serialization/deserialization round-trip
#[tokio::test]
async fn test_complete_graph_serialization_round_trip() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Create test nodes
    let node1_id = Uuid::new_v4();
    let node1 = GraphNode {
        id: node1_id,
        node_type: "function".to_string(),
        qualified_name: "kotadb::FileStorage::insert".to_string(),
        file_path: "src/file_storage.rs".to_string(),
        location: NodeLocation {
            start_line: 100,
            start_column: 4,
            end_line: 105,
            end_column: 5,
        },
        metadata: HashMap::new(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let node2_id = Uuid::new_v4();
    let node2 = GraphNode {
        id: node2_id,
        node_type: "function".to_string(),
        qualified_name: "kotadb::FileStorage::get".to_string(),
        file_path: "src/file_storage.rs".to_string(),
        location: NodeLocation {
            start_line: 200,
            start_column: 4,
            end_line: 205,
            end_column: 5,
        },
        metadata: HashMap::new(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    // Store the nodes
    storage.store_node(node1_id, node1.clone()).await?;
    storage.store_node(node2_id, node2.clone()).await?;

    // Create test edge
    let edge = GraphEdge {
        relation_type: RelationType::Calls,
        location: NodeLocation {
            start_line: 100,
            start_column: 8,
            end_line: 100,
            end_column: 12,
        },
        context: Some("FileStorage.insert()".to_string()),
        metadata: HashMap::new(),
        created_at: chrono::Utc::now().timestamp(),
    };

    // Store the edge
    storage.store_edge(node1_id, node2_id, edge.clone()).await?;

    // Force persistence to disk
    storage.sync().await?;
    drop(storage);

    // Create new storage instance to force loading from disk
    // This tests both node and edge deserialization
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Verify nodes can be retrieved correctly
    let retrieved_node1 = storage2.get_node(node1_id).await?;
    assert!(retrieved_node1.is_some(), "Node 1 should be retrievable");
    let node1_retrieved = retrieved_node1.unwrap();
    assert_eq!(node1_retrieved.qualified_name, node1.qualified_name);

    let retrieved_node2 = storage2.get_node(node2_id).await?;
    assert!(retrieved_node2.is_some(), "Node 2 should be retrievable");
    let node2_retrieved = retrieved_node2.unwrap();
    assert_eq!(node2_retrieved.qualified_name, node2.qualified_name);

    // Verify edges can be retrieved correctly
    let edges_from_node1 = storage2.get_edges(node1_id, Direction::Outgoing).await?;
    assert!(!edges_from_node1.is_empty(), "Should have edges from node1");

    let (target_id, retrieved_edge) = &edges_from_node1[0];
    assert_eq!(*target_id, node2_id, "Should have edge to node2");
    assert_eq!(retrieved_edge.relation_type, edge.relation_type);

    println!("✅ Successfully tested complete graph serialization round-trip");
    println!("   - Nodes: {} entries", 2);
    println!("   - Edges: {} entries", 1);

    Ok(())
}

/// Test edge-only serialization to isolate edge persistence issues
#[tokio::test]
async fn test_edge_serialization_only() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let mut storage = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Create nodes first (edges need nodes to exist)
    let nodes: Vec<(Uuid, GraphNode)> = (0..3)
        .map(|i| {
            let id = Uuid::new_v4();
            let node = GraphNode {
                id,
                node_type: "function".to_string(),
                qualified_name: format!("test::function_{}", i),
                file_path: format!("test_{}.rs", i),
                location: NodeLocation {
                    start_line: i * 10,
                    start_column: 0,
                    end_line: i * 10 + 5,
                    end_column: 10,
                },
                metadata: HashMap::new(),
                updated_at: chrono::Utc::now().timestamp(),
            };
            (id, node)
        })
        .collect();

    // Store nodes
    for (id, node) in &nodes {
        storage.store_node(*id, node.clone()).await?;
    }

    // Create multiple edges with different types
    let edge_types = [
        RelationType::Calls,
        RelationType::Imports,
        RelationType::Extends,
        RelationType::Implements,
    ];
    let mut expected_edges = Vec::new();

    for (i, relation_type) in edge_types.iter().enumerate() {
        let from_idx = i % nodes.len();
        let to_idx = (i + 1) % nodes.len();
        let from_id = nodes[from_idx].0;
        let to_id = nodes[to_idx].0;


        let edge = GraphEdge {
            relation_type: relation_type.clone(),
            location: NodeLocation {
                start_line: 100 + i * 10,
                start_column: 8,
                end_line: 100 + i * 10,
                end_column: 12,
            },
            context: Some(format!("test_context_{}", i)),
            metadata: HashMap::from([
                ("weight".to_string(), (i + 1).to_string()),
                ("line".to_string(), (100 + i * 10).to_string()),
            ]),
            created_at: chrono::Utc::now().timestamp(),
        };

        storage.store_edge(from_id, to_id, edge.clone()).await?;
        expected_edges.push((from_id, to_id, edge));
    }

    // Force persistence to disk
    storage.sync().await?;
    drop(storage);

    // Create new storage instance to test edge loading
    let storage2 = NativeGraphStorage::new(&db_path, GraphStorageConfig::default()).await?;

    // Verify all edges can be retrieved correctly
    for (from_id, to_id, expected_edge) in &expected_edges {
        let edges_from = storage2.get_edges(*from_id, Direction::Outgoing).await?;

        // Find edge that matches both target node AND relationship type
        let found_edge = edges_from.iter().find(|(target, edge)| {
            target == to_id && edge.relation_type == expected_edge.relation_type
        });
        assert!(
            found_edge.is_some(),
            "Should have edge from {} to {} with RelationType::{:?}",
            from_id,
            to_id,
            expected_edge.relation_type
        );

        let (_, retrieved_edge) = found_edge.unwrap();
        assert_eq!(retrieved_edge.relation_type, expected_edge.relation_type);
        assert_eq!(retrieved_edge.metadata.len(), expected_edge.metadata.len());

        for (key, expected_value) in &expected_edge.metadata {
            assert_eq!(retrieved_edge.metadata.get(key), Some(expected_value));
        }
    }

    println!(
        "✅ Successfully tested edge serialization with {} different edge types",
        edge_types.len()
    );

    Ok(())
}
