// Database module - Shared database abstraction for service integration
//
// This module provides a unified Database struct that implements the DatabaseAccess trait,
// allowing it to be used across CLI, HTTP API, and MCP interfaces while maintaining
// a consistent API surface.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::{
    create_binary_trigram_index, create_file_storage, create_primary_index, create_trigram_index,
    create_wrapped_storage,
    services::{AnalysisServiceDatabase, DatabaseAccess},
    Index, Storage, ValidatedDocumentId,
};

/// Main database abstraction that coordinates storage and indices
///
/// This struct serves as the primary interface to KotaDB's storage and indexing systems,
/// implementing the DatabaseAccess trait required by all services.
pub struct Database {
    pub storage: Arc<Mutex<dyn Storage>>,
    pub primary_index: Arc<Mutex<dyn Index>>,
    pub trigram_index: Arc<Mutex<dyn Index>>,
    // Cache for path -> document ID lookups (built lazily)
    pub path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl Database {
    /// Create a new Database instance with storage and indices
    ///
    /// # Arguments
    /// * `db_path` - Root path for database storage
    /// * `use_binary_index` - Whether to use binary or text-based trigram index
    pub async fn new(db_path: &Path, use_binary_index: bool) -> Result<Self> {
        let storage_path = db_path.join("storage");
        let primary_index_path = db_path.join("primary_index");
        let trigram_index_path = db_path.join("trigram_index");

        // Create directories if they don't exist
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&primary_index_path)?;
        std::fs::create_dir_all(&trigram_index_path)?;

        let storage = create_file_storage(
            storage_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid storage path: {:?}", storage_path))?,
            Some(100), // Cache size
        )
        .await?;

        let primary_index = create_primary_index(
            primary_index_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid primary index path: {:?}", primary_index_path)
            })?,
            Some(1000), // Cache size
        )
        .await?;

        // Choose trigram index implementation based on binary flag
        let trigram_index_arc: Arc<Mutex<dyn Index>> = if use_binary_index {
            Arc::new(Mutex::new(
                create_binary_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?,
            ))
        } else {
            Arc::new(Mutex::new(
                create_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?,
            ))
        };

        // Apply wrappers for production safety
        let wrapped_storage = create_wrapped_storage(storage, 100).await;

        Ok(Self {
            storage: Arc::new(Mutex::new(wrapped_storage)),
            primary_index: Arc::new(Mutex::new(primary_index)),
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get database statistics (document count and total size)
    pub async fn stats(&self) -> Result<(usize, usize)> {
        let all_docs = self.storage.lock().await.list_all().await?;
        let doc_count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();
        Ok((doc_count, total_size))
    }
}

// Implement DatabaseAccess trait for the Database struct
impl DatabaseAccess for Database {
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

// Implement AnalysisServiceDatabase trait for the Database struct
impl AnalysisServiceDatabase for Database {
    fn storage(&self) -> Arc<Mutex<dyn Storage>> {
        self.storage.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Document, Query};
    use crate::services::{DatabaseAccess, AnalysisServiceDatabase};
    use crate::types::*;
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_document(content: &str, path: &str) -> Document {
        let now = Utc::now();
        Document {
            id: ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap(),
            path: ValidatedPath::new(path).unwrap(),
            title: ValidatedTitle::new("Test Document").unwrap(),
            content: content.as_bytes().to_vec(),
            tags: vec![],
            created_at: now,
            updated_at: now,
            size: content.len(),
            embedding: None,
        }
    }

    fn create_test_query(search_term: &str) -> Query {
        Query {
            search_terms: vec![ValidatedSearchQuery::new(search_term, 1).unwrap()],
            tags: vec![],
            path_pattern: None,
            limit: ValidatedLimit::new(100, 1000).unwrap(),
            offset: ValidatedPageId::new(1).unwrap(), // Page IDs must be > 0
        }
    }

    #[tokio::test]
    async fn test_database_new_with_binary_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await;
        assert!(database.is_ok(), "Should successfully create database with binary index");

        let db = database.unwrap();
        // Verify all components are present
        assert!(db.storage.lock().await.list_all().await.is_ok());
        let wildcard_query = create_test_query("*");
        assert!(db.primary_index.lock().await.search(&wildcard_query).await.is_ok());
        let test_query = create_test_query("test");
        assert!(db.trigram_index.lock().await.search(&test_query).await.is_ok());
        
        // Verify path cache is initialized
        assert_eq!(db.path_cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_database_new_with_text_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, false).await;
        assert!(database.is_ok(), "Should successfully create database with text index");

        let db = database.unwrap();
        // Verify all components are present
        assert!(db.storage.lock().await.list_all().await.is_ok());
        let wildcard_query = create_test_query("*");
        assert!(db.primary_index.lock().await.search(&wildcard_query).await.is_ok());
        let test_query = create_test_query("test");
        assert!(db.trigram_index.lock().await.search(&test_query).await.is_ok());
        
        // Verify path cache is initialized
        assert_eq!(db.path_cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_database_stats_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        let (doc_count, total_size) = database.stats().await.expect("Failed to get stats");
        
        assert_eq!(doc_count, 0, "Empty database should have 0 documents");
        assert_eq!(total_size, 0, "Empty database should have 0 total size");
    }

    #[tokio::test]
    async fn test_database_stats_with_documents() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Insert test documents
        let doc1 = create_test_document("Hello world", "test1.md");
        let doc2 = create_test_document("Another document", "test2.md");
        
        database.storage.lock().await.insert(doc1.clone()).await.expect("Failed to insert doc1");
        database.storage.lock().await.insert(doc2.clone()).await.expect("Failed to insert doc2");
        
        let (doc_count, total_size) = database.stats().await.expect("Failed to get stats");
        
        assert_eq!(doc_count, 2, "Should have 2 documents");
        assert_eq!(total_size, doc1.size + doc2.size, "Total size should match document sizes");
    }

    #[tokio::test]
    async fn test_database_access_trait_storage() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Test DatabaseAccess trait method
        let storage = DatabaseAccess::storage(&database);
        assert!(storage.lock().await.list_all().await.is_ok());
    }

    #[tokio::test]
    async fn test_database_access_trait_primary_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Test DatabaseAccess trait method
        let primary_index = database.primary_index();
        let query = create_test_query("*");
        assert!(primary_index.lock().await.search(&query).await.is_ok());
    }

    #[tokio::test]
    async fn test_database_access_trait_trigram_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Test DatabaseAccess trait method
        let trigram_index = database.trigram_index();
        let query = create_test_query("test");
        assert!(trigram_index.lock().await.search(&query).await.is_ok());
    }

    #[tokio::test]
    async fn test_database_access_trait_path_cache() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Test DatabaseAccess trait method
        let path_cache = database.path_cache();
        assert_eq!(path_cache.read().await.len(), 0);
        
        // Test cache modification
        let doc = create_test_document("test", "test.md");
        path_cache.write().await.insert("test.md".to_string(), doc.id.clone());
        assert_eq!(path_cache.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_analysis_service_database_trait() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Test AnalysisServiceDatabase trait method  
        let storage: Arc<Mutex<dyn Storage>> = AnalysisServiceDatabase::storage(&database);
        assert!(storage.lock().await.list_all().await.is_ok());
    }

    #[tokio::test]
    async fn test_database_directory_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("nested_db");

        // Directory doesn't exist initially
        assert!(!db_path.exists(), "Database directory shouldn't exist initially");

        let database = Database::new(&db_path, true).await.expect("Failed to create database");
        
        // Verify directories were created
        assert!(db_path.exists(), "Database root directory should be created");
        assert!(db_path.join("storage").exists(), "Storage directory should be created");
        assert!(db_path.join("primary_index").exists(), "Primary index directory should be created");
        assert!(db_path.join("trigram_index").exists(), "Trigram index directory should be created");
    }

    #[tokio::test]
    async fn test_database_concurrent_access() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Arc::new(Database::new(db_path, true).await.expect("Failed to create database"));
        
        // Test concurrent access to different components
        let handles: Vec<tokio::task::JoinHandle<Result<usize>>> = vec![
            tokio::spawn({
                let db = database.clone();
                async move { 
                    db.storage.lock().await.list_all().await.map(|docs| docs.len())
                }
            }),
            tokio::spawn({
                let db = database.clone();
                async move { 
                    let query = create_test_query("*");
                    db.primary_index.lock().await.search(&query).await.map(|ids| ids.len())
                }
            }),
            tokio::spawn({
                let db = database.clone();
                async move { 
                    let query = create_test_query("test");
                    db.trigram_index.lock().await.search(&query).await.map(|ids| ids.len())
                }
            }),
        ];
        
        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.expect("Task should complete");
            assert!(result.is_ok(), "All concurrent operations should succeed");
        }
    }

    #[tokio::test]
    async fn test_database_error_handling_invalid_path() {
        // Test with an invalid path containing null bytes
        let result = Database::new(Path::new("test\0path"), true).await;
        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[tokio::test]
    async fn test_database_with_real_operations() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Insert a document
        let doc = create_test_document("Hello world test content", "example.md");
        database.storage.lock().await.insert(doc.clone()).await.expect("Failed to insert document");
        
        // Verify stats reflect the insertion
        let (doc_count, total_size) = database.stats().await.expect("Failed to get stats");
        assert_eq!(doc_count, 1);
        assert_eq!(total_size, doc.size);
        
        // Verify document can be retrieved
        let retrieved = database.storage.lock().await.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, doc.id);
        
        // Test path cache usage
        database.path_cache.write().await.insert("example.md".to_string(), doc.id.clone());
        let cached_id = database.path_cache.read().await.get("example.md").cloned();
        assert_eq!(cached_id, Some(doc.id));
    }

    #[tokio::test]
    async fn test_database_binary_vs_text_index_behavior() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Create databases with both index types
        let binary_db_path = temp_dir.path().join("binary");
        let text_db_path = temp_dir.path().join("text");
        
        let binary_db = Database::new(&binary_db_path, true).await.expect("Failed to create binary database");
        let text_db = Database::new(&text_db_path, false).await.expect("Failed to create text database");
        
        // Both should support basic operations
        let test_query = create_test_query("test");
        assert!(binary_db.trigram_index.lock().await.search(&test_query).await.is_ok());
        assert!(text_db.trigram_index.lock().await.search(&test_query).await.is_ok());
        
        // Both should return empty results for non-existent content
        let nonexistent_query = create_test_query("nonexistent");
        let binary_results = binary_db.trigram_index.lock().await.search(&nonexistent_query).await.expect("Search should work");
        let text_results = text_db.trigram_index.lock().await.search(&nonexistent_query).await.expect("Search should work");
        
        assert_eq!(binary_results.len(), 0);
        assert_eq!(text_results.len(), 0);
    }

    #[tokio::test]
    async fn test_database_stats_with_varying_document_sizes() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path();

        let database = Database::new(db_path, true).await.expect("Failed to create database");
        
        // Insert documents of different sizes
        let small_doc = create_test_document("small", "small.md");
        let medium_doc = create_test_document("This is a medium sized document with more content", "medium.md");
        let large_doc = create_test_document(&"Large document content ".repeat(100), "large.md");
        
        database.storage.lock().await.insert(small_doc.clone()).await.expect("Failed to insert small doc");
        database.storage.lock().await.insert(medium_doc.clone()).await.expect("Failed to insert medium doc");
        database.storage.lock().await.insert(large_doc.clone()).await.expect("Failed to insert large doc");
        
        let (doc_count, total_size) = database.stats().await.expect("Failed to get stats");
        
        assert_eq!(doc_count, 3);
        let expected_size = small_doc.size + medium_doc.size + large_doc.size;
        assert_eq!(total_size, expected_size);
        assert!(total_size > 2000, "Total size should be substantial with large document");
    }
}
