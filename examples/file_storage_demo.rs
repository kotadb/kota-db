// Demo: FileStorage with Stage 6 Components
// This example shows how to use the complete KotaDB stack

use kotadb::{
    create_file_storage, 
    DocumentBuilder,
    Storage,
    init_logging,
    Operation,
    log_operation,
    with_trace_id,
};
use anyhow::Result;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();
    
    println!("🚀 KotaDB FileStorage Demo");
    println!("========================");
    
    // Create temporary directory for demo
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    println!("📁 Creating database at: {}", db_path);
    
    // Create fully wrapped FileStorage with all Stage 6 components
    let mut storage = create_file_storage(db_path, Some(100)).await?;
    
    println!("✅ Storage created with all Stage 6 wrappers:");
    println!("   - TracedStorage (automatic logging)");
    println!("   - ValidatedStorage (contract enforcement)");
    println!("   - RetryableStorage (failure recovery)");
    println!("   - CachedStorage (performance optimization)");
    
    // Create a test document using Stage 6 builder
    println!("\n📝 Creating document with DocumentBuilder...");
    let doc = DocumentBuilder::new()
        .path("/knowledge/rust-patterns.md")?
        .title("Advanced Rust Design Patterns")?
        .content(b"# Advanced Rust Design Patterns\n\nThis document covers advanced patterns in Rust programming including:\n\n- Zero-cost abstractions\n- Type-state patterns\n- Builder patterns\n- RAII patterns\n\n## Zero-Cost Abstractions\n\nRust allows you to write high-level code that compiles down to efficient machine code...")?
        .build()?;
    
    println!("✅ Document created:");
    println!("   ID: {}", doc.id);
    println!("   Title: {}", doc.title);
    println!("   Word count: {}", doc.word_count);
    
    // Insert document (automatically traced, validated, cached)
    println!("\n💾 Inserting document...");
    storage.insert(doc.clone()).await?;
    println!("✅ Document inserted successfully");
    
    // Retrieve document (cache hit on second access)
    println!("\n🔍 Retrieving document...");
    let retrieved = storage.get(&doc.id).await?;
    match retrieved {
        Some(doc) => {
            println!("✅ Document retrieved successfully:");
            println!("   ID: {}", doc.id);
            println!("   Title: {}", doc.title);
            println!("   Size: {} bytes", doc.size);
        }
        None => println!("❌ Document not found"),
    }
    
    // Test cache behavior - second retrieval should be faster
    println!("\n🔍 Retrieving document again (cache test)...");
    let _retrieved_again = storage.get(&doc.id).await?;
    println!("✅ Second retrieval completed (should hit cache)");
    
    // Update document
    println!("\n✏️  Updating document...");
    let mut updated_doc = doc;
    updated_doc.title = "Updated: Advanced Rust Design Patterns".to_string();
    updated_doc.updated = chrono::Utc::now().timestamp();
    
    storage.update(updated_doc.clone()).await?;
    println!("✅ Document updated successfully");
    
    // Verify update
    let updated_retrieved = storage.get(&updated_doc.id).await?;
    if let Some(doc) = updated_retrieved {
        println!("✅ Updated document verified:");
        println!("   New title: {}", doc.title);
    }
    
    // Create another document to test multiple documents
    println!("\n📝 Creating second document...");
    let doc2 = DocumentBuilder::new()
        .path("/knowledge/async-patterns.md")?
        .title("Async Programming in Rust")?
        .content(b"# Async Programming in Rust\n\nAsync/await patterns and best practices...")?
        .build()?;
    
    storage.insert(doc2.clone()).await?;
    println!("✅ Second document inserted: {}", doc2.title);
    
    // Delete first document
    println!("\n🗑️  Deleting first document...");
    storage.delete(&updated_doc.id).await?;
    println!("✅ Document deleted successfully");
    
    // Verify deletion
    let deleted_check = storage.get(&updated_doc.id).await?;
    match deleted_check {
        Some(_) => println!("❌ Document still exists after deletion"),
        None => println!("✅ Document deletion confirmed"),
    }
    
    // Sync to ensure all changes are persisted
    println!("\n💽 Syncing changes to disk...");
    storage.sync().await?;
    println!("✅ All changes synced successfully");
    
    println!("\n🎉 Demo completed successfully!");
    println!("\n📊 This demo showcased:");
    println!("   ✓ Stage 6 Component Library usage");
    println!("   ✓ Builder patterns for safe construction");
    println!("   ✓ Automatic tracing and validation");
    println!("   ✓ File-based storage implementation");
    println!("   ✓ CRUD operations with error handling");
    println!("   ✓ Cache behavior and performance");
    
    Ok(())
}