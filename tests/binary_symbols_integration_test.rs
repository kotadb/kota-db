//! Integration tests for binary symbol format
//!
//! These tests verify the binary format works correctly in real-world scenarios
//! and maintains the expected performance characteristics.

use anyhow::Result;
use kotadb::binary_symbols::{BinarySymbolReader, BinarySymbolWriter};
use kotadb::git::{IngestionConfig, RepositoryIngester};
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_binary_format_performance_regression() -> Result<()> {
    // Create test repository structure
    let temp_dir = TempDir::new()?;
    let test_repo = temp_dir.path().join("test_repo");
    std::fs::create_dir_all(&test_repo)?;

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&test_repo)
        .output()?;

    // Configure local git identity for CI before committing
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&test_repo)
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&test_repo)
        .output()?;

    // Create test files with symbols
    create_test_files(&test_repo)?;

    // Commit files
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&test_repo)
        .output()?;

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&test_repo)
        .output()?;

    // Set up storage
    let storage_dir = temp_dir.path().join("storage");
    let symbol_db = temp_dir.path().join("symbols.kota");
    let mut storage = kotadb::create_file_storage(storage_dir.to_str().unwrap(), Some(100)).await?;

    // Configure and run ingestion
    let config = IngestionConfig::default();
    let ingester = RepositoryIngester::new(config);

    let start = Instant::now();
    let result = ingester
        .ingest_with_binary_symbols(&test_repo, &mut storage, &symbol_db, None)
        .await?;
    let elapsed = start.elapsed();

    // Performance assertions
    assert!(result.symbols_extracted > 0, "Should extract symbols");
    assert!(
        elapsed < Duration::from_secs(5),
        "Should complete within 5 seconds for small repo"
    );

    // Verify binary format
    let reader = BinarySymbolReader::open(&symbol_db)?;
    assert_eq!(reader.symbol_count(), result.symbols_extracted);

    // Verify first symbol can be read
    if let Some(symbol) = reader.get_symbol(0) {
        let name = reader.get_symbol_name(&symbol)?;
        assert!(!name.is_empty(), "Symbol name should not be empty");
    }

    Ok(())
}

#[test]
fn test_binary_format_stress() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("stress.kota");

    // Create writer and add many symbols
    let mut writer = BinarySymbolWriter::new();
    let symbol_count = 10_000;

    let start = Instant::now();
    for i in 0..symbol_count {
        writer.add_symbol(
            Uuid::new_v4(),
            &format!("symbol_{}", i),
            (i % 8) as u8 + 1,
            &format!("src/file_{}.rs", i % 100),
            (i * 10) as u32,
            (i * 10 + 5) as u32,
            None,
        );
    }

    writer.write_to_file(&db_path)?;
    let write_elapsed = start.elapsed();

    // Performance assertions for write
    assert!(
        write_elapsed < Duration::from_secs(1),
        "Writing 10k symbols took {:?}, expected < 1s",
        write_elapsed
    );

    // Test reading performance
    let reader = BinarySymbolReader::open(&db_path)?;
    assert_eq!(reader.symbol_count(), symbol_count);

    let read_start = Instant::now();
    for i in 0..100 {
        let symbol = reader.get_symbol(i * 100).expect("Symbol should exist");
        let _ = reader.get_symbol_name(&symbol)?;
    }
    let read_elapsed = read_start.elapsed();

    // Performance assertion for reads
    assert!(
        read_elapsed < Duration::from_millis(10),
        "Reading 100 symbols took {:?}, expected < 10ms",
        read_elapsed
    );

    Ok(())
}

#[test]
fn test_binary_format_cross_platform_compatibility() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("compat.kota");

    // Write a known pattern
    let mut writer = BinarySymbolWriter::new();
    let test_id = Uuid::parse_str("12345678-1234-5678-1234-567812345678")?;

    writer.add_symbol(test_id, "test_symbol", 1, "test.rs", 100, 200, None);

    writer.write_to_file(&db_path)?;

    // Read back and verify
    let reader = BinarySymbolReader::open(&db_path)?;
    let symbol = reader.get_symbol(0).expect("Symbol should exist");

    // Verify fields are correctly read
    assert_eq!(symbol.id, *test_id.as_bytes());
    assert_eq!(symbol.kind, 1);
    assert_eq!(symbol.start_line, 100);
    assert_eq!(symbol.end_line, 200);
    assert_eq!(reader.get_symbol_name(&symbol)?, "test_symbol");
    assert_eq!(reader.get_symbol_file_path(&symbol)?, "test.rs");

    Ok(())
}

#[test]
fn test_binary_format_unicode_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("unicode.kota");

    let mut writer = BinarySymbolWriter::new();

    // Test various unicode strings
    let unicode_names = [
        "测试函数",           // Chinese
        "テスト关数",         // Japanese
        "함수_테스트",        // Korean
        "тест_функция",       // Russian
        "🚀_rocket_function", // Emoji
        "café_résumé",        // Accented
    ];

    for (i, name) in unicode_names.iter().enumerate() {
        writer.add_symbol(
            Uuid::new_v4(),
            name,
            1,
            &format!("файл_{}.rs", i), // Unicode in file path too
            10,
            20,
            None,
        );
    }

    writer.write_to_file(&db_path)?;

    // Read back and verify
    let reader = BinarySymbolReader::open(&db_path)?;
    for (i, expected_name) in unicode_names.iter().enumerate() {
        let symbol = reader.get_symbol(i).expect("Symbol should exist");
        let actual_name = reader.get_symbol_name(&symbol)?;
        assert_eq!(
            &actual_name, expected_name,
            "Unicode name mismatch at index {}",
            i
        );
    }

    Ok(())
}

#[test]
fn test_binary_format_parent_relationships() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("parents.kota");

    let mut writer = BinarySymbolWriter::new();

    // Create a hierarchy
    let parent_id = Uuid::new_v4();
    let child1_id = Uuid::new_v4();
    let child2_id = Uuid::new_v4();

    writer.add_symbol(parent_id, "ParentClass", 3, "test.rs", 10, 50, None);
    writer.add_symbol(child1_id, "method1", 2, "test.rs", 15, 20, Some(parent_id));
    writer.add_symbol(child2_id, "method2", 2, "test.rs", 25, 30, Some(parent_id));

    writer.write_to_file(&db_path)?;

    // Read back and verify relationships
    let reader = BinarySymbolReader::open(&db_path)?;

    let parent = reader.get_symbol(0).expect("Parent should exist");
    assert_eq!(parent.parent_id, [0u8; 16], "Parent should have no parent");

    let child1 = reader.get_symbol(1).expect("Child1 should exist");
    assert_eq!(
        child1.parent_id,
        *parent_id.as_bytes(),
        "Child1 parent mismatch"
    );

    let child2 = reader.get_symbol(2).expect("Child2 should exist");
    assert_eq!(
        child2.parent_id,
        *parent_id.as_bytes(),
        "Child2 parent mismatch"
    );

    Ok(())
}

// Helper function to create test files
fn create_test_files(repo_path: &Path) -> Result<()> {
    // Create a Rust file with various symbols
    let rust_content = r#"
pub struct TestStruct {
    field1: String,
    field2: i32,
}

impl TestStruct {
    pub fn new() -> Self {
        TestStruct {
            field1: String::new(),
            field2: 0,
        }
    }
    
    pub fn method1(&self) -> &str {
        &self.field1
    }
}

pub fn standalone_function(x: i32) -> i32 {
    x * 2
}

pub enum TestEnum {
    Variant1,
    Variant2(String),
}

pub const TEST_CONSTANT: i32 = 42;
"#;

    std::fs::write(repo_path.join("test.rs"), rust_content)?;

    // Create another file
    let lib_content = r#"
mod test;

pub fn library_function() {
    println!("Library function");
}
"#;

    std::fs::write(repo_path.join("lib.rs"), lib_content)?;

    Ok(())
}
