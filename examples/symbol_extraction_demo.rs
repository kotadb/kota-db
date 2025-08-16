//! Demo of symbol extraction pipeline on KotaDB's own codebase

#[cfg(feature = "tree-sitter-parsing")]
use anyhow::Result;
#[cfg(feature = "tree-sitter-parsing")]
use kotadb::{
    create_symbol_storage,
    parsing::{CodeParser, SupportedLanguage},
};
#[cfg(feature = "tree-sitter-parsing")]
use std::path::Path;

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 Symbol Extraction Pipeline Demo");
    println!("===================================\n");

    // Create symbol storage
    let symbol_storage = create_symbol_storage("data/symbols_demo", Some(100)).await?;

    // Parse a sample Rust file from KotaDB
    let rust_code = std::fs::read_to_string("src/types.rs")?;
    let mut parser = CodeParser::new()?;
    let parsed = parser.parse_content(&rust_code, SupportedLanguage::Rust)?;

    println!("📊 Parse Statistics for src/types.rs:");
    println!("  Total nodes: {}", parsed.stats.total_nodes);
    println!("  Named nodes: {}", parsed.stats.named_nodes);
    println!("  Max depth: {}", parsed.stats.max_depth);
    println!("  Symbols found: {}", parsed.symbols.len());
    println!();

    // Extract and store symbols
    {
        let mut storage = symbol_storage.lock().await;
        let symbol_ids = storage
            .extract_symbols(
                Path::new("src/types.rs"),
                parsed,
                Some("kota-db".to_string()),
            )
            .await?;

        println!(
            "✅ Extracted {} symbols from src/types.rs",
            symbol_ids.len()
        );
        println!();

        // Show symbol statistics
        let stats = storage.get_stats();
        println!("📈 Symbol Index Statistics:");
        println!("  Total symbols: {}", stats.total_symbols);
        println!("  Symbol types:");
        for (sym_type, count) in &stats.symbols_by_type {
            println!("    - {}: {}", sym_type, count);
        }
        println!();

        // Demo: Search for symbols
        println!("🔎 Symbol Search Examples:");

        // Search for "Validated"
        let results = storage.search("Validated", 5);
        println!("\n  Searching for 'Validated':");
        for entry in results {
            println!(
                "    - {} ({:?})",
                entry.symbol.name, entry.symbol.symbol_type
            );
        }

        // Find all structs
        let structs = storage.find_by_type(&kotadb::parsing::SymbolType::Struct);
        println!("\n  All structs in types.rs: {} found", structs.len());
        if structs.len() <= 10 {
            for s in structs {
                println!("    - {} at line {}", s.symbol.name, s.symbol.start_line);
            }
        }
    }

    println!("\n✨ Symbol extraction demo complete!");

    Ok(())
}

#[cfg(not(feature = "tree-sitter-parsing"))]
fn main() {
    eprintln!("This demo requires the 'tree-sitter-parsing' feature.");
    eprintln!(
        "Run with: cargo run --example symbol_extraction_demo --features tree-sitter-parsing"
    );
}
