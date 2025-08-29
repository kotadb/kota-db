use anyhow::Result;
use kotadb::binary_relationship_engine::BinaryRelationshipEngine;
use kotadb::relationship_query::RelationshipQueryConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let db_path = PathBuf::from("./test-data");

    println!("=== Testing File Collection ===\n");

    // Create an engine to test file collection
    let config = RelationshipQueryConfig::default();
    let engine = BinaryRelationshipEngine::new(&db_path, config).await?;

    // This would normally be private, so let's test indirectly
    // Try to trigger on-demand extraction to see what happens

    let current_dir = std::env::current_dir()?;
    println!("Current directory: {:?}", current_dir);

    // List Rust files in current directory
    println!("\nRust files in current directory:");
    let mut count = 0;
    for entry in std::fs::read_dir("src")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            println!("  {}", path.display());
            count += 1;
            if count >= 5 {
                println!("  ... and more");
                break;
            }
        }
    }

    // Check if the engine can find files
    use kotadb::relationship_query::RelationshipQueryType;

    println!("\n=== Attempting Query (will trigger extraction) ===");

    // Remove cached graph to force extraction
    let graph_path = db_path.join("dependency_graph.bin");
    if graph_path.exists() {
        std::fs::remove_file(&graph_path)?;
        println!("Removed cached dependency graph");
    }

    // This should trigger on-demand extraction
    let query = RelationshipQueryType::FindCallers {
        target: "FileStorage".to_string(),
    };

    match engine.execute_query(query).await {
        Ok(result) => {
            println!("Query succeeded!");
            println!("Direct relationships: {}", result.stats.direct_count);
            println!("Symbols analyzed: {}", result.stats.symbols_analyzed);
        }
        Err(e) => {
            println!("Query failed: {}", e);
        }
    }

    // Check if a graph was created
    if graph_path.exists() {
        println!("\nDependency graph was created at: {:?}", graph_path);
        let metadata = std::fs::metadata(&graph_path)?;
        println!("Graph file size: {} bytes", metadata.len());
    } else {
        println!("\n❌ No dependency graph was created!");
    }

    Ok(())
}
