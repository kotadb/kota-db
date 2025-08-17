//! Debug test for symbol extraction

use anyhow::Result;
use tempfile::TempDir;

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn debug_symbol_extraction() -> Result<()> {
    use kotadb::parsing::{CodeParser, SupportedLanguage};

    let rust_code = r#"
fn hello() {
    println!("Hello, world!");
}

struct Test {
    field: i32,
}
"#;

    // Test the parser directly
    let mut parser = CodeParser::new()?;
    let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

    println!("Parsed symbols: {:?}", parsed.symbols);
    println!("Number of symbols found: {}", parsed.symbols.len());

    for symbol in &parsed.symbols {
        println!("Symbol: {} (type: {:?})", symbol.name, symbol.symbol_type);
    }

    assert!(
        !parsed.symbols.is_empty(),
        "Should find at least some symbols"
    );

    Ok(())
}

#[cfg(feature = "tree-sitter-parsing")]
#[tokio::test]
async fn debug_symbol_storage() -> Result<()> {
    use kotadb::parsing::{CodeParser, SupportedLanguage};
    use kotadb::symbol_storage::SymbolStorage;

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_path_buf();

    let storage =
        kotadb::file_storage::create_file_storage(temp_path.to_str().unwrap(), Some(100)).await?;

    let mut symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    let rust_code = r#"
fn hello() {
    println!("Hello, world!");
}

struct Test {
    field: i32,
}
"#;

    // Parse and extract symbols
    let mut parser = CodeParser::new()?;
    let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

    println!(
        "Extracting symbols from parsed code with {} symbols",
        parsed.symbols.len()
    );

    let symbol_ids = symbol_storage
        .extract_symbols(std::path::Path::new("test.rs"), parsed, None)
        .await?;

    println!(
        "Extracted {} symbol IDs: {:?}",
        symbol_ids.len(),
        symbol_ids
    );

    // Check what's in storage
    let stats = symbol_storage.get_stats();
    println!("Storage stats: {:?}", stats);

    let all_symbols = symbol_storage.search("*", 100);
    println!("All symbols in storage: {}", all_symbols.len());

    for symbol in all_symbols {
        println!(
            "Stored symbol: {} (type: {:?})",
            symbol.symbol.name, symbol.symbol.symbol_type
        );
    }

    Ok(())
}
