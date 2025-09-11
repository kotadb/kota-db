---
tags:
- file
- kota-db
- ext_rs
---
// Performance Regression Test for Issue #596
// Validates the 151x improvement claim from PR #597 (79 seconds → 0.5 seconds)
// Ensures search performance remains under strict thresholds to prevent future regressions

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index,
    services::search_service::{DatabaseAccess, SearchOptions, SearchService},
    DocumentBuilder, Index, Storage, ValidatedDocumentId,
};

/// Test database implementation for performance testing
struct PerfTestDatabase {
    storage: Arc<Mutex<dyn Storage>>,
    primary_index: Arc<Mutex<dyn Index>>,
    trigram_index: Arc<Mutex<dyn Index>>,
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl DatabaseAccess for PerfTestDatabase {
    fn storage(&self) -> Arc<Mutex<dyn Storage>> {
        self.storage.clone()
    }

    fn primary_index(&self) -> Arc<Mutex<dyn Index>> {
        self.primary_index.clone()
    }

    fn trigram_index(&self) -> Arc<Mutex<dyn Index>> {
        self.trigram_index.clone()
    }

    fn path_cache(&self) -> Arc<RwLock<HashMap<String, ValidatedDocumentId>>> {
        self.path_cache.clone()
    }
}

/// Create a realistic database with substantial content for performance testing
async fn setup_large_test_database() -> Result<(TempDir, PerfTestDatabase)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let primary_index =
        create_primary_index(db_path.join("primary").to_str().unwrap(), None).await?;
    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;

    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
    let primary_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));
    let path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let database = PerfTestDatabase {
        storage: storage.clone(),
        primary_index: primary_index.clone(),
        trigram_index: trigram_index.clone(),
        path_cache,
    };

    // Create realistic codebase content similar to actual usage
    {
        let mut storage_guard = storage.lock().await;
        let mut primary_guard = primary_index.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        // Generate a realistic number of documents with various content types
        let file_templates = vec![
            ("src/lib.rs", "rust", generate_rust_lib_content()),
            ("src/main.rs", "rust", generate_rust_main_content()),
            ("src/storage.rs", "rust", generate_storage_content()),
            ("src/index.rs", "rust", generate_index_content()),
            ("src/query.rs", "rust", generate_query_content()),
            ("tests/integration.rs", "rust", generate_test_content()),
            ("README.md", "markdown", generate_readme_content()),
            ("docs/api.md", "markdown", generate_api_docs_content()),
            ("Cargo.toml", "toml", generate_cargo_content()),
            ("scripts/build.sh", "shell", generate_script_content()),
        ];

        // Create multiple copies to simulate a larger codebase
        for i in 0..50 {
            // Creates 500 documents total
            for (base_path, file_type, content) in &file_templates {
                let path = if i == 0 {
                    base_path.to_string()
                } else {
                    base_path
                        .replace(".", &format!("_{}", i))
                        .replace("/", &format!("_{}/", i))
                };

                let doc = DocumentBuilder::new()
                    .path(&path)?
                    .title(format!("{} - {}", file_type, i))?
                    .content(content.as_bytes())
                    .build()?;

                storage_guard.insert(doc.clone()).await?;
                primary_guard.insert(doc.id, doc.path.clone()).await?;
                trigram_guard
                    .insert_with_content(doc.id, doc.path.clone(), &doc.content)
                    .await?;
            }
        }
    }

    Ok((temp_dir, database))
}

fn generate_rust_lib_content() -> String {
    r#"
//! KotaDB - High-performance codebase intelligence platform
//!
//! This library provides fast indexing and search capabilities for code analysis.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub mod storage;
pub mod index;  
pub mod query;
pub mod types;

/// Main database interface for codebase analysis
#[derive(Debug)]
pub struct Database {
    storage: storage::Storage,
    primary_index: index::PrimaryIndex,
    trigram_index: index::TrigramIndex,
    config: DatabaseConfig,
}

/// Configuration for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub storage_path: PathBuf,
    pub index_path: PathBuf,
    pub max_documents: usize,
    pub enable_caching: bool,
    pub cache_size: usize,
}

impl Database {
    /// Create a new database instance
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let storage = storage::Storage::new(&config.storage_path).await
            .context("Failed to initialize storage")?;
            
        let primary_index = index::PrimaryIndex::new(&config.index_path.join("primary")).await
            .context("Failed to initialize primary index")?;
            
        let trigram_index = index::TrigramIndex::new(&config.index_path.join("trigram")).await
            .context("Failed to initialize trigram index")?;

        Ok(Self {
            storage,
            primary_index,
            trigram_index,
            config,
        })
    }

    /// Search for documents matching the given query
    pub async fn search(&self, query: &str) -> Result<Vec<Document>> {
        let start_time = std::time::Instant::now();
        
        let doc_ids = if query.contains('*') {
            self.primary_index.search(query).await?
        } else {
            self.trigram_index.search(query).await?
        };

        let mut documents = Vec::new();
        for doc_id in doc_ids {
            if let Some(doc) = self.storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        let elapsed = start_time.elapsed();
        tracing::info!("Search completed in {:?}", elapsed);

        Ok(documents)
    }

    /// Add a document to the database
    pub async fn add_document(&mut self, path: &Path, content: &[u8]) -> Result<DocumentId> {
        let doc = Document::new(path, content)?;
        let doc_id = doc.id;

        self.storage.insert(doc.clone()).await?;
        self.primary_index.insert(doc_id, doc.path.clone()).await?;
        self.trigram_index.insert_with_content(doc_id, doc.path, content).await?;

        Ok(doc_id)
    }
}

/// Document representation in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub path: PathBuf,
    pub title: Option<String>,
    pub content: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

/// Unique identifier for documents
pub type DocumentId = uuid::Uuid;

impl Document {
    pub fn new(path: &Path, content: &[u8]) -> Result<Self> {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            path: path.to_path_buf(),
            title: path.file_stem().map(|s| s.to_string_lossy().to_string()),
            content: content.to_vec(),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let config = DatabaseConfig {
            storage_path: "/tmp/test".into(),
            index_path: "/tmp/test_index".into(),
            max_documents: 1000,
            enable_caching: true,
            cache_size: 100,
        };

        let result = Database::new(config).await;
        assert!(result.is_ok());
    }
}
"#
    .to_string()
}

fn generate_rust_main_content() -> String {
    r#"
//! Main entry point for KotaDB CLI
//!
//! Provides command-line interface for codebase indexing and search operations.

use anyhow::{Context, Result};
use clap::{App, Arg, SubCommand};
use env_logger;
use std::path::PathBuf;
use tokio;

mod cli;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("kotadb")
        .version("0.1.0")
        .about("High-performance codebase intelligence platform")
        .arg(
            Arg::with_name("database")
                .short("d")
                .long("database")
                .value_name("PATH")
                .help("Database directory path")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Enable verbose logging"),
        )
        .subcommand(
            SubCommand::with_name("index")
                .about("Index a codebase")
                .arg(
                    Arg::with_name("path")
                        .help("Path to codebase")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search the indexed codebase")
                .arg(
                    Arg::with_name("query")
                        .help("Search query")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("limit")
                        .short("l")
                        .long("limit")
                        .help("Maximum number of results")
                        .default_value("10"),
                ),
        )
        .get_matches();

    let database_path = PathBuf::from(matches.value_of("database").unwrap());
    
    match matches.subcommand() {
        ("index", Some(sub_matches)) => {
            let codebase_path = PathBuf::from(sub_matches.value_of("path").unwrap());
            commands::index_codebase(&database_path, &codebase_path).await
                .context("Failed to index codebase")?;
        }
        ("search", Some(sub_matches)) => {
            let query = sub_matches.value_of("query").unwrap();
            let limit: usize = sub_matches.value_of("limit").unwrap().parse()
                .context("Invalid limit value")?;
            commands::search_codebase(&database_path, query, limit).await
                .context("Failed to search codebase")?;
        }
        _ => {
            eprintln!("No subcommand provided. Use --help for usage information.");
            std::process::exit(1);
        }
    }

    Ok(())
}
"#
    .to_string()
}

fn generate_storage_content() -> String {
    r#"
//! Storage layer implementation
//!
//! Provides persistent storage for documents with ACID properties.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// High-performance storage engine
#[derive(Debug)]
pub struct Storage {
    path: PathBuf,
    write_ahead_log: WriteAheadLog,
    page_cache: PageCache,
    document_index: HashMap<Uuid, u64>,
}

impl Storage {
    pub async fn new(path: &Path) -> Result<Self> {
        fs::create_dir_all(path).await?;
        
        let wal = WriteAheadLog::new(path.join("wal")).await?;
        let cache = PageCache::new(1024); // 1MB cache
        
        Ok(Self {
            path: path.to_path_buf(),
            write_ahead_log: wal,
            page_cache: cache,
            document_index: HashMap::new(),
        })
    }

    pub async fn insert(&mut self, document: Document) -> Result<()> {
        let serialized = bincode::serialize(&document)?;
        let page_id = self.write_ahead_log.append(&serialized).await?;
        self.document_index.insert(document.id, page_id);
        Ok(())
    }

    pub async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        if let Some(&page_id) = self.document_index.get(id) {
            if let Some(data) = self.page_cache.get(page_id) {
                let doc: Document = bincode::deserialize(&data)?;
                return Ok(Some(doc));
            }
            
            let data = self.write_ahead_log.read_page(page_id).await?;
            let doc: Document = bincode::deserialize(&data)?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    pub async fn update(&mut self, document: Document) -> Result<()> {
        // Implementation for updates
        self.insert(document).await
    }

    pub async fn delete(&mut self, id: &Uuid) -> Result<bool> {
        if self.document_index.remove(id).is_some() {
            // Mark as deleted in WAL
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn document_count(&self) -> usize {
        self.document_index.len()
    }
}

#[derive(Debug)]
struct WriteAheadLog {
    path: PathBuf,
    current_offset: u64,
}

impl WriteAheadLog {
    async fn new(path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&path).await?;
        Ok(Self {
            path,
            current_offset: 0,
        })
    }

    async fn append(&mut self, data: &[u8]) -> Result<u64> {
        let page_id = self.current_offset;
        // Write data to WAL
        self.current_offset += data.len() as u64;
        Ok(page_id)
    }

    async fn read_page(&self, _page_id: u64) -> Result<Vec<u8>> {
        // Implementation for reading pages
        Ok(vec![])
    }
}

#[derive(Debug)]
struct PageCache {
    capacity: usize,
    cache: HashMap<u64, Vec<u8>>,
}

impl PageCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
        }
    }

    fn get(&self, page_id: u64) -> Option<&[u8]> {
        self.cache.get(&page_id).map(|v| v.as_slice())
    }
}

use crate::{Document, DocumentId};
"#
    .to_string()
}

fn generate_index_content() -> String {
    "// Index implementations for fast lookups\n".repeat(100)
}

fn generate_query_content() -> String {
    "// Query processing and optimization\n".repeat(100)
}

fn generate_test_content() -> String {
    "// Integration tests for database functionality\n".repeat(100)
}

fn generate_readme_content() -> String {
    "# KotaDB Documentation\n\nHigh-performance codebase intelligence.\n".repeat(50)
}

fn generate_api_docs_content() -> String {
    "# API Documentation\n\nComplete API reference.\n".repeat(50)
}

fn generate_cargo_content() -> String {
    r#"[package]
name = "kotadb"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
serde = "1.0"
tokio = "1.0"
uuid = "1.0"
"#
    .to_string()
}

fn generate_script_content() -> String {
    "#!/bin/bash\n# Build script\necho 'Building project...'\n".repeat(20)
}

// Performance threshold constants (based on PR #597 claims)
const MAX_SEARCH_TIME_MS: u128 = 1000; // 1 second max (was 79+ seconds before)
const TARGET_SEARCH_TIME_MS: u128 = 500; // Target 0.5 seconds
const STRICT_SEARCH_TIME_MS: u128 = 100; // Strict threshold for small queries

#[tokio::test]
async fn test_performance_regression_protection_issue_596() -> Result<()> {
    let (_temp_dir, database) = setup_large_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols_perf");
    let search_service = SearchService::new(&database, symbol_db_path);

    // Test the exact queries that were problematic in issue #596
    let problematic_queries = vec![
        "rust", "async fn", "Database", "search", "impl", "pub fn", "use", "struct",
    ];

    let mut total_time = Duration::ZERO;
    let mut max_time = Duration::ZERO;
    let mut results_count = 0;

    println!("🔥 Testing performance regression protection for Issue #596");
    println!(
        "   Target: <{}ms per query (was 79,000ms+ before fix)",
        TARGET_SEARCH_TIME_MS
    );

    for query in &problematic_queries {
        let options = SearchOptions {
            query: query.to_string(),
            limit: 10,
            tags: None,
            context: "minimal".to_string(), // Use new default context
            quiet: false,
        };

        let start_time = Instant::now();
        let result = search_service.search_content(options).await?;
        let elapsed = start_time.elapsed();

        // CRITICAL: These must complete within strict time limits
        assert!(
            elapsed.as_millis() < MAX_SEARCH_TIME_MS,
            "🚨 PERFORMANCE REGRESSION DETECTED!\n   Query '{}' took {}ms (limit: {}ms)\n   This indicates the 675x performance regression has returned!",
            query,
            elapsed.as_millis(),
            MAX_SEARCH_TIME_MS
        );

        // Track performance metrics
        total_time += elapsed;
        if elapsed > max_time {
            max_time = elapsed;
        }
        results_count += result.total_count;

        // Verify it's using fast search, not slow LLM processing
        assert!(
            matches!(
                result.search_type,
                kotadb::services::search_service::SearchType::RegularSearch
            ),
            "Query '{}' should use fast regular search, not LLM processing",
            query
        );

        println!(
            "   ✅ '{}': {}ms ({} results)",
            query,
            elapsed.as_millis(),
            result.total_count
        );
    }

    let avg_time = total_time / problematic_queries.len() as u32;

    println!("\n📊 Performance Summary:");
    println!(
        "   Average time: {}ms (target: <{}ms)",
        avg_time.as_millis(),
        TARGET_SEARCH_TIME_MS
    );
    println!(
        "   Maximum time: {}ms (limit: <{}ms)",
        max_time.as_millis(),
        MAX_SEARCH_TIME_MS
    );
    println!("   Total results: {}", results_count);
    println!(
        "   Performance improvement: {}x faster than broken state",
        79000 / avg_time.as_millis().max(1)
    );

    // Validate overall performance meets targets
    assert!(
        avg_time.as_millis() < TARGET_SEARCH_TIME_MS,
        "Average search time {}ms exceeds target of {}ms",
        avg_time.as_millis(),
        TARGET_SEARCH_TIME_MS
    );

    println!("\n🎯 SUCCESS: Performance regression protection is working!");
    println!(
        "   All searches complete in <{}ms (vs 79,000ms+ before fix)",
        max_time.as_millis()
    );

    Ok(())
}

#[tokio::test]
async fn test_context_mode_performance_difference() -> Result<()> {
    let (_temp_dir, database) = setup_large_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols_context");
    let search_service = SearchService::new(&database, symbol_db_path);

    let query = "Database implementation";

    // Test minimal context (should be very fast)
    let minimal_options = SearchOptions {
        query: query.to_string(),
        limit: 10,
        tags: None,
        context: "minimal".to_string(),
        quiet: false,
    };

    let start_time = Instant::now();
    let minimal_result = search_service.search_content(minimal_options).await?;
    let minimal_time = start_time.elapsed();

    // Test medium context (may use LLM, could be slower)
    let medium_options = SearchOptions {
        query: query.to_string(),
        limit: 10,
        tags: None,
        context: "medium".to_string(),
        quiet: false,
    };

    let start_time = Instant::now();
    let medium_result = search_service.search_content(medium_options).await?;
    let medium_time = start_time.elapsed();

    // Minimal context should always be fast
    assert!(
        minimal_time.as_millis() < STRICT_SEARCH_TIME_MS,
        "Minimal context should be very fast: {}ms",
        minimal_time.as_millis()
    );

    // Verify minimal uses regular search
    assert!(
        matches!(
            minimal_result.search_type,
            kotadb::services::search_service::SearchType::RegularSearch
        ),
        "Minimal context should use regular search"
    );

    println!("⚡ Context Performance Comparison:");
    println!(
        "   Minimal context: {}ms (regular search)",
        minimal_time.as_millis()
    );
    println!(
        "   Medium context:  {}ms ({})",
        medium_time.as_millis(),
        match medium_result.search_type {
            kotadb::services::search_service::SearchType::LLMOptimized => "LLM search",
            kotadb::services::search_service::SearchType::RegularSearch =>
                "regular search (LLM unavailable)",
            kotadb::services::search_service::SearchType::WildcardSearch => "wildcard search",
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_bulk_search_performance() -> Result<()> {
    let (_temp_dir, database) = setup_large_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols_bulk");
    let search_service = SearchService::new(&database, symbol_db_path);

    // Test multiple searches in sequence (simulates AI assistant usage)
    let queries = vec![
        "async", "function", "struct", "impl", "use", "pub", "fn", "let", "match", "if",
        "Database", "Storage", "Index", "Query", "Document", "Result", "Error", "Config", "test",
        "mod", "trait", "enum", "const", "static", "mut", "ref", "self", "super",
    ];

    let start_time = Instant::now();
    let mut total_results = 0;

    for query in &queries {
        let options = SearchOptions {
            query: query.to_string(),
            limit: 5, // Smaller limit for bulk testing
            tags: None,
            context: "minimal".to_string(),
            quiet: true,
        };

        let result = search_service.search_content(options).await?;
        total_results += result.total_count;
    }

    let total_time = start_time.elapsed();
    let avg_time = total_time / queries.len() as u32;

    println!("🔍 Bulk Search Performance Test:");
    println!(
        "   {} queries in {}ms (avg: {}ms per query)",
        queries.len(),
        total_time.as_millis(),
        avg_time.as_millis()
    );
    println!("   Total results found: {}", total_results);

    // Each query should still be fast even in bulk
    assert!(
        avg_time.as_millis() < STRICT_SEARCH_TIME_MS,
        "Average bulk search time {}ms exceeds strict limit {}ms",
        avg_time.as_millis(),
        STRICT_SEARCH_TIME_MS
    );

    // Total time should be reasonable for AI assistant usage
    assert!(
        total_time.as_secs() < 5,
        "Bulk search took too long: {}ms",
        total_time.as_millis()
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_search_performance() -> Result<()> {
    let (_temp_dir, database) = setup_large_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols_concurrent");

    // Use Arc to share database across tasks
    let database = Arc::new(database);

    let queries = vec!["rust", "async", "Database", "search", "function"];
    let mut handles = Vec::new();

    let start_time = Instant::now();

    // Launch concurrent searches
    for query in queries {
        let db = database.clone();
        let symbol_path = symbol_db_path.clone();

        let handle = tokio::spawn(async move {
            let search_service = SearchService::new(db.as_ref(), symbol_path);
            let options = SearchOptions {
                query: query.to_string(),
                limit: 10,
                tags: None,
                context: "minimal".to_string(),
                quiet: true,
            };

            let start = Instant::now();
            let result = search_service.search_content(options).await?;
            let elapsed = start.elapsed();

            Ok::<(String, Duration, usize), anyhow::Error>((
                query.to_string(),
                elapsed,
                result.total_count,
            ))
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        let (query, elapsed, count) = handle.await??;
        results.push((query, elapsed, count));
    }

    let total_time = start_time.elapsed();

    println!("⚡ Concurrent Search Performance:");
    for (query, elapsed, count) in &results {
        println!(
            "   '{}': {}ms ({} results)",
            query,
            elapsed.as_millis(),
            count
        );

        // Each concurrent search should still be fast
        assert!(
            elapsed.as_millis() < MAX_SEARCH_TIME_MS,
            "Concurrent search '{}' took too long: {}ms",
            query,
            elapsed.as_millis()
        );
    }

    println!("   Total concurrent time: {}ms", total_time.as_millis());

    // Concurrent searches should complete faster than sequential
    assert!(
        total_time.as_millis() < 5000, // 5 seconds max for concurrent execution
        "Concurrent searches took too long: {}ms",
        total_time.as_millis()
    );

    Ok(())
}
