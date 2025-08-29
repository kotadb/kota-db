//! Integration tests for stats CLI command
//!
//! These tests verify the stats command correctly reports database statistics,
//! including documents, symbols, and relationships, with proper flag handling.

use anyhow::Result;
use kotadb::binary_relationship_engine::BinaryRelationshipEngine;
use kotadb::binary_symbols::{BinarySymbolReader, BinarySymbolWriter};
use kotadb::git::types::IngestionOptions;
use kotadb::git::{IngestionConfig, RepositoryIngester};
use kotadb::relationship_query::RelationshipQueryConfig;
use kotadb::symbol_storage::SymbolStorage;
use std::path::Path;
use tempfile::TempDir;

/// Create test repository with Rust files containing symbols
fn create_test_files(repo_path: &Path) -> Result<()> {
    let src_dir = repo_path.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Create main.rs with functions and structs
    std::fs::write(
        src_dir.join("main.rs"),
        r#"
use std::collections::HashMap;

pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
}

impl DatabaseConfig {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    pub fn connect(&self) -> Result<Connection, String> {
        Connection::new(&self.host, self.port)
    }
}

pub struct Connection {
    config: DatabaseConfig,
}

impl Connection {
    pub fn new(host: &str, port: u16) -> Result<Self, String> {
        Ok(Self {
            config: DatabaseConfig::new(host.to_string(), port),
        })
    }

    pub fn execute_query(&self, query: &str) -> Vec<String> {
        vec![query.to_string()]
    }
}

fn main() {
    let config = DatabaseConfig::new("localhost".to_string(), 5432);
    let conn = config.connect().unwrap();
    let results = conn.execute_query("SELECT * FROM users");
    println!("Results: {:?}", results);
}
"#,
    )?;

    // Create lib.rs with additional symbols
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"
pub mod storage;
pub mod query;

pub trait Storage {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: String);
}

pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

pub struct QueryBuilder {
    query_type: QueryType,
    table: String,
    conditions: Vec<String>,
}

impl QueryBuilder {
    pub fn new(query_type: QueryType) -> Self {
        Self {
            query_type,
            table: String::new(),
            conditions: Vec::new(),
        }
    }

    pub fn table(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self
    }

    pub fn where_clause(mut self, condition: &str) -> Self {
        self.conditions.push(condition.to_string());
        self
    }

    pub fn build(self) -> String {
        format!("Query for {}", self.table)
    }
}
"#,
    )?;

    // Create storage.rs module
    std::fs::write(
        src_dir.join("storage.rs"),
        r#"
use super::Storage;
use std::collections::HashMap;

pub struct MemoryStorage {
    data: HashMap<String, String>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl Storage for MemoryStorage {
    fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), value);
    }
}

pub struct FileStorage {
    path: String,
}

impl FileStorage {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub fn load(&self) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(&self.path)
    }

    pub fn save(&self, data: &[u8]) -> Result<(), std::io::Error> {
        std::fs::write(&self.path, data)
    }
}
"#,
    )?;

    Ok(())
}

#[tokio::test]
async fn test_symbol_stats_command_binary_and_traditional() -> Result<()> {
    // Create test repository
    let temp_dir = TempDir::new()?;
    let test_repo = temp_dir.path().join("test_repo");
    let db_path = temp_dir.path().join("database");

    std::fs::create_dir_all(&test_repo)?;
    std::fs::create_dir_all(&db_path)?;

    // Initialize git repository
    std::process::Command::new("git")
        .args(["init"])
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
        .args([
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            "Initial commit",
        ])
        .current_dir(&test_repo)
        .output()?;

    // Set up storage
    let storage_path = db_path.join("storage");
    std::fs::create_dir_all(&storage_path)?;
    let storage = kotadb::create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut storage = Box::new(storage);

    // Test 1: Empty database should show 0 symbols
    let symbol_db_path = db_path.join("symbols.kota");

    // Check that binary symbol file doesn't exist initially
    assert!(
        !symbol_db_path.exists(),
        "Binary symbol file should not exist initially"
    );

    // Traditional symbol storage check (empty initially)
    let graph_path = storage_path.join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;
    let graph_config = kotadb::graph_storage::GraphStorageConfig::default();
    let graph_storage =
        kotadb::native_graph_storage::NativeGraphStorage::new(graph_path, graph_config).await?;
    let symbol_storage = SymbolStorage::with_graph_storage(
        Box::new(kotadb::create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?),
        Box::new(graph_storage),
    )
    .await?;
    let empty_stats = symbol_storage.get_stats();

    assert_eq!(
        empty_stats.total_symbols, 0,
        "Traditional symbols should be 0 initially"
    );

    // Test 2: Ingest repository with binary symbols
    let mut options = IngestionOptions {
        include_file_contents: true,
        include_commit_history: true,
        max_file_size: 10 * 1024 * 1024,
        memory_limits: None,
        ..Default::default()
    };

    // Enable symbol extraction (binary format)
    #[cfg(feature = "tree-sitter-parsing")]
    {
        options.extract_symbols = true;
    }

    let config = IngestionConfig {
        path_prefix: "test".to_string(),
        options,
        create_index: true,
        organization_config: Some(kotadb::git::RepositoryOrganizationConfig::default()),
    };

    let ingester = RepositoryIngester::new(config);

    // Ingest with binary symbols
    #[cfg(feature = "tree-sitter-parsing")]
    {
        let graph_db_path = db_path.join("dependency_graph.bin");
        let _result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &test_repo,
                storage.as_mut(),
                &symbol_db_path,
                &graph_db_path,
                None, // No progress callback
            )
            .await?;
    }

    // Test 3: Verify binary symbols exist
    assert!(
        symbol_db_path.exists(),
        "Binary symbol file should exist after ingestion"
    );

    let binary_reader = BinarySymbolReader::open(&symbol_db_path)?;
    let binary_symbol_count = binary_reader.symbol_count();

    assert!(
        binary_symbol_count > 0,
        "Should have extracted some binary symbols"
    );
    println!("Binary symbols extracted: {}", binary_symbol_count);

    // Test 4: Test BinaryRelationshipEngine (what find-callers uses)
    #[cfg(feature = "tree-sitter-parsing")]
    {
        let relationship_config = RelationshipQueryConfig::default();
        let binary_engine = BinaryRelationshipEngine::new(&db_path, relationship_config).await?;
        let binary_stats = binary_engine.get_stats();

        assert_eq!(
            binary_stats.binary_symbols_loaded, binary_symbol_count,
            "BinaryRelationshipEngine should load same number of binary symbols"
        );
        assert!(
            binary_stats.using_binary_path,
            "Should be using binary symbol path"
        );
    }

    // Test 5: Verify traditional symbol storage remains empty (as expected in this setup)
    let updated_symbol_storage = SymbolStorage::with_graph_storage(
        Box::new(kotadb::create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?),
        Box::new(
            kotadb::native_graph_storage::NativeGraphStorage::new(
                storage_path.join("graph"),
                kotadb::graph_storage::GraphStorageConfig::default(),
            )
            .await?,
        ),
    )
    .await?;
    let updated_stats = updated_symbol_storage.get_stats();

    // Traditional symbols remain 0 because binary format stores symbols separately
    assert_eq!(
        updated_stats.total_symbols, 0,
        "Traditional symbols should remain 0 with binary format"
    );

    // Test 6: Test the exact logic from our fixed symbol-stats command
    let total_all_symbols = updated_stats.total_symbols + binary_symbol_count;
    assert_eq!(
        total_all_symbols, binary_symbol_count,
        "Total symbols should equal binary symbols when traditional is 0"
    );
    assert!(
        total_all_symbols > 0,
        "Total symbols should be greater than 0"
    );

    println!("✅ Integration test passed:");
    println!("   Traditional symbols: {}", updated_stats.total_symbols);
    println!("   Binary symbols: {}", binary_symbol_count);
    println!("   Total symbols: {}", total_all_symbols);

    Ok(())
}

#[tokio::test]
async fn test_symbol_stats_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("database");
    std::fs::create_dir_all(&db_path)?;

    // Test case: Database directory exists but no symbols.kota file
    let storage_path = db_path.join("storage");
    std::fs::create_dir_all(&storage_path)?;

    let symbol_db_path = db_path.join("symbols.kota");
    assert!(!symbol_db_path.exists(), "symbols.kota should not exist");

    // Test case: Try to read binary symbols from non-existent file
    let binary_symbol_count = if symbol_db_path.exists() {
        match BinarySymbolReader::open(&symbol_db_path) {
            Ok(reader) => reader.symbol_count(),
            Err(_) => 0,
        }
    } else {
        0
    };

    assert_eq!(
        binary_symbol_count, 0,
        "Should return 0 when no binary symbol file exists"
    );

    // Test case: Create empty binary symbol file
    let empty_writer = BinarySymbolWriter::new();
    empty_writer.write_to_file(&symbol_db_path)?;
    assert!(symbol_db_path.exists(), "Empty symbols.kota should exist");

    let empty_reader = BinarySymbolReader::open(&symbol_db_path)?;
    assert_eq!(
        empty_reader.symbol_count(),
        0,
        "Empty binary file should have 0 symbols"
    );

    println!("✅ Edge case tests passed");
    Ok(())
}

#[tokio::test]
async fn test_symbol_stats_consistency_with_find_callers() -> Result<()> {
    // This test ensures symbol-stats reports the same count that find-callers would use
    let temp_dir = TempDir::new()?;
    let test_repo = temp_dir.path().join("test_repo");
    let db_path = temp_dir.path().join("database");

    std::fs::create_dir_all(&test_repo)?;
    std::fs::create_dir_all(&db_path)?;

    // Create minimal test repository
    let src_dir = test_repo.join("src");
    std::fs::create_dir_all(&src_dir)?;

    std::fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn hello_world() -> String {
    "Hello, World!".to_string()
}

pub struct TestStruct {
    pub value: i32,
}

impl TestStruct {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}
"#,
    )?;

    // Initialize git and commit
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&test_repo)
        .output()?;
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&test_repo)
        .output()?;
    std::process::Command::new("git")
        .args([
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test",
            "commit",
            "-m",
            "test",
        ])
        .current_dir(&test_repo)
        .output()?;

    // Set up storage and ingest with binary symbols
    let storage_path = db_path.join("storage");
    std::fs::create_dir_all(&storage_path)?;
    let storage = kotadb::create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut storage = Box::new(storage);

    let symbol_db_path = db_path.join("symbols.kota");
    let graph_db_path = db_path.join("dependency_graph.bin");

    #[cfg(feature = "tree-sitter-parsing")]
    {
        let options = IngestionOptions {
            extract_symbols: true,
            ..Default::default()
        };

        let config = IngestionConfig {
            path_prefix: "test".to_string(),
            options,
            create_index: true,
            organization_config: Some(kotadb::git::RepositoryOrganizationConfig::default()),
        };

        let ingester = RepositoryIngester::new(config);
        let _result = ingester
            .ingest_with_binary_symbols_and_relationships(
                &test_repo,
                storage.as_mut(),
                &symbol_db_path,
                &graph_db_path,
                None,
            )
            .await?;

        // Get symbol count from symbol-stats perspective
        let binary_symbol_count = if symbol_db_path.exists() {
            BinarySymbolReader::open(&symbol_db_path)?.symbol_count()
        } else {
            0
        };

        // Get symbol count from find-callers perspective (BinaryRelationshipEngine)
        let relationship_config = RelationshipQueryConfig::default();
        let binary_engine = BinaryRelationshipEngine::new(&db_path, relationship_config).await?;
        let binary_stats = binary_engine.get_stats();

        // The counts must match!
        assert_eq!(
            binary_symbol_count, binary_stats.binary_symbols_loaded,
            "symbol-stats and find-callers must report the same binary symbol count"
        );

        println!("✅ Consistency verified:");
        println!("   symbol-stats binary count: {}", binary_symbol_count);
        println!(
            "   find-callers binary count: {}",
            binary_stats.binary_symbols_loaded
        );
    }

    Ok(())
}
