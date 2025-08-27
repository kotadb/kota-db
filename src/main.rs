// KotaDB CLI - Codebase intelligence platform for distributed human-AI cognition
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kotadb::{
    create_binary_trigram_index, create_file_storage, create_primary_index, create_trigram_index,
    create_wrapped_storage, init_logging_with_level, start_server, validate_post_ingestion_search,
    with_trace_id, Document, DocumentBuilder, Index, QueryBuilder, Storage, ValidatedDocumentId,
    ValidatedPath, ValidationStatus,
};

#[cfg(feature = "tree-sitter-parsing")]
use kotadb::{
    relationship_query::{
        parse_natural_language_relationship_query, RelationshipQueryEngine, RelationshipQueryType,
    },
    symbol_storage::SymbolStorage,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "KotaDB - A codebase intelligence platform for distributed human-AI cognition",
    long_about = None,
    after_help = "QUICK START:
  1. Ingest a repository:     kotadb ingest-repo /path/to/repo
  2. Search for code:         kotadb search 'function_name'
  3. Find relationships:      kotadb relationship-query 'what calls MyFunction?'
  4. Analyze impact:          kotadb impact-analysis 'StorageClass'

EXAMPLES:
  # Basic document operations
  kotadb insert docs/readme.md \"README\" \"Content here\"
  kotadb get docs/readme.md
  kotadb search \"database query\"

  # Codebase intelligence
  kotadb ingest-repo ./my-project
  kotadb find-callers FileStorage
  kotadb relationship-query 'what would break if I change Config?'

  # System management
  kotadb stats
  kotadb serve --port 8080"
)]
struct Cli {
    /// Enable verbose logging (DEBUG level). Default is WARN level.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Database directory path
    #[arg(short, long, default_value = "./kota-db-data")]
    db_path: PathBuf,

    /// Use binary format for indices (10x faster, experimental)
    #[arg(long, global = true)]
    binary_index: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP REST API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Insert a new document
    Insert {
        /// Path of the document (e.g., docs/readme.md)
        path: String,
        /// Title of the document
        title: String,
        /// Content of the document (can be piped in)
        #[arg(value_name = "CONTENT")]
        content: Option<String>,
    },

    /// Get a document by path
    Get {
        /// Path of the document (e.g., docs/readme.md)
        path: String,
    },

    /// Update an existing document
    Update {
        /// Path of the document to update
        path: String,
        /// New path (optional)
        #[arg(short = 'n', long)]
        new_path: Option<String>,
        /// New title (optional)
        #[arg(short, long)]
        title: Option<String>,
        /// New content (optional, can be piped in)
        #[arg(short, long)]
        content: Option<String>,
    },

    /// Delete a document by path
    Delete {
        /// Path of the document to delete
        path: String,
    },

    /// Search for documents and symbols by content or path
    Search {
        /// Search query (use '*' for all, or search terms for content/symbol matching)
        #[arg(default_value = "*")]
        query: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "10",
            help = "Control number of results (increase with -l 100)"
        )]
        limit: usize,
        /// Filter by tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// List all documents
    List {
        /// Limit number of results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Show database statistics
    Stats,

    /// Validate search functionality
    Validate,

    /// Verify documentation accuracy against implementation
    VerifyDocs,

    /// Ingest a git repository into the database
    #[cfg(feature = "git-integration")]
    IngestRepo {
        /// Path to the git repository
        repo_path: PathBuf,
        /// Prefix for document paths in the database
        #[arg(short, long, default_value = "repos")]
        prefix: String,
        /// Include file contents
        #[arg(long, default_value = "true")]
        include_files: bool,
        /// Include commit history
        #[arg(long, default_value = "true")]
        include_commits: bool,
        /// Maximum file size to ingest (in MB)
        #[arg(long, default_value = "10")]
        max_file_size_mb: usize,
        /// Extract code symbols using tree-sitter parsing (enabled by default)
        /// Use --extract-symbols=false or --no-symbols to disable
        #[cfg(feature = "tree-sitter-parsing")]
        #[arg(long, default_value = "true")]
        extract_symbols: Option<bool>,
        /// Skip symbol extraction (convenience flag, same as --extract-symbols=false)
        #[cfg(feature = "tree-sitter-parsing")]
        #[arg(long, conflicts_with = "extract_symbols")]
        no_symbols: bool,
    },

    /// Search for symbols (functions, classes, variables) by name or pattern
    #[cfg(feature = "tree-sitter-parsing")]
    SearchSymbols {
        /// Symbol name or pattern to search for (supports partial matching)
        pattern: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "25",
            help = "Control number of results (use -l 100 for more)"
        )]
        limit: usize,
        /// Show only specific symbol types (function, class, variable, etc.)
        #[arg(short = 't', long)]
        symbol_type: Option<String>,
    },

    /// Find all places where a symbol (function, class, variable) is called or referenced
    #[cfg(feature = "tree-sitter-parsing")]
    FindCallers {
        /// Name or qualified name of the target symbol (e.g., 'FileStorage' or 'storage::FileStorage')
        target: String,
        /// Maximum number of results to return (default: unlimited)
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Analyze dependencies: what would break if you change a symbol (safe refactoring analysis)
    #[cfg(feature = "tree-sitter-parsing")]
    ImpactAnalysis {
        /// Name or qualified name of the target symbol (e.g., 'StorageError' or 'errors::StorageError')
        target: String,
        /// Maximum number of impacted items to show (default: unlimited)
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Natural language queries about code symbols and relationships
    #[cfg(feature = "tree-sitter-parsing")]
    RelationshipQuery {
        /// Natural language query (e.g., 'what calls FileStorage?', 'who uses Config?', 'find unused functions')
        query: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Show statistics about extracted code symbols (functions, classes, variables)
    #[cfg(feature = "tree-sitter-parsing")]
    SymbolStats,

    /// Run performance benchmarks on database operations
    ///
    /// Note: Benchmark data remains in the database after completion for inspection.
    /// Use a fresh database path to avoid data accumulation across runs.
    Benchmark {
        /// Number of operations to perform
        #[arg(short, long, default_value = "10000")]
        operations: usize,
        /// Run only specific benchmark types (storage, index, query, all)
        #[arg(short = 't', long, default_value = "all")]
        benchmark_type: String,
        /// Output format (human, json, csv)
        #[arg(short = 'f', long, default_value = "human")]
        format: String,
        /// Maximum number of search queries to run (prevents excessive runtime)
        #[arg(
            long,
            default_value = "100",
            help = "Limit search operations to prevent excessive runtime"
        )]
        max_search_queries: usize,
    },
}

struct Database {
    storage: Arc<Mutex<dyn Storage>>,
    primary_index: Arc<Mutex<dyn Index>>,
    trigram_index: Arc<Mutex<dyn Index>>,
    // Cache for path -> document ID lookups (built lazily)
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
    // Coordinated deletion service to ensure index synchronization
    deletion_service: kotadb::CoordinatedDeletionService,
}

impl Database {
    async fn new(db_path: &Path, use_binary_index: bool) -> Result<Self> {
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
            Some(1000),
        )
        .await?;
        let (trigram_index, trigram_index_arc): (Arc<Mutex<dyn Index>>, Arc<Mutex<dyn Index>>) = if use_binary_index {
            tracing::info!("Using binary trigram index for 10x performance");
            let index = create_binary_trigram_index(
                trigram_index_path.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                })?,
                Some(1000),
            )
            .await?;
            let arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(index));
            (arc.clone(), arc)
        } else {
            let index = create_trigram_index(
                trigram_index_path.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                })?,
                Some(1000),
            )
            .await?;
            let arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(index));
            (arc.clone(), arc)
        };

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
        
        let deletion_service = kotadb::CoordinatedDeletionService::new(
            storage_arc.clone(),
            primary_index_arc.clone(),
            trigram_index,
        );

        let db = Self {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
            deletion_service,
        };

        // Skip path cache rebuild for read-only operations like search
        // The cache will be built lazily when needed for path-based lookups
        // This significantly improves startup time for search operations
        // from ~300ms to ~5ms (see issue #274)

        Ok(db)
    }

    /// Rebuild the path cache from current storage
    async fn rebuild_path_cache(&self) -> Result<()> {
        let mut cache = self.path_cache.write().await;
        cache.clear();

        // Get all documents to build the cache
        let all_docs = self.storage.lock().await.list_all().await?;
        for doc in all_docs {
            cache.insert(doc.path.to_string(), doc.id);
        }

        Ok(())
    }

    /// Rebuild all indices from current storage
    /// This is needed after bulk operations like git ingestion
    async fn rebuild_indices(&self) -> Result<()> {
        // Get all documents from storage
        let all_docs = self.storage.lock().await.list_all().await?;
        let total_docs = all_docs.len();

        if total_docs == 0 {
            return Ok(());
        }

        // Process documents in batches for better performance
        const BATCH_SIZE: usize = 100;
        let mut processed = 0;

        // Process in chunks to reduce lock contention and prevent OOM
        for chunk in all_docs.chunks(BATCH_SIZE) {
            // Collect document data for this batch (including content for trigram indexing)
            let mut batch_entries = Vec::with_capacity(chunk.len());
            for doc in chunk {
                let doc_id = doc.id;
                let doc_path = ValidatedPath::new(doc.path.to_string())?;
                batch_entries.push((doc_id, doc_path, doc.content.clone()));
            }

            // Insert batch into primary index (path-based)
            {
                let mut primary_index = self.primary_index.lock().await;
                for (doc_id, doc_path, _) in &batch_entries {
                    primary_index.insert(*doc_id, doc_path.clone()).await?;
                }
            }

            // Insert batch into trigram index with content for proper full-text search
            {
                let mut trigram_index = self.trigram_index.lock().await;
                for (doc_id, doc_path, content) in &batch_entries {
                    // Use the new content-aware method for proper trigram indexing
                    trigram_index
                        .insert_with_content(*doc_id, doc_path.clone(), content)
                        .await?;
                }
            }

            processed += chunk.len();

            // Periodic flush for large datasets
            if processed % 500 == 0 || processed == total_docs {
                self.primary_index.lock().await.flush().await?;
                self.trigram_index.lock().await.flush().await?;
            }
        }

        Ok(())
    }

    async fn insert(
        &self,
        path: String,
        title: String,
        content: String,
    ) -> Result<ValidatedDocumentId> {
        let doc = DocumentBuilder::new()
            .path(&path)?
            .title(&title)?
            .content(content.as_bytes())
            .build()?;

        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(&path)?;

        // Insert into storage
        self.storage.lock().await.insert(doc.clone()).await?;

        // Insert into both indices
        self.primary_index
            .lock()
            .await
            .insert(doc_id, doc_path.clone())
            .await?;

        // Insert into trigram index with content for proper full-text search
        {
            let mut trigram_guard = self.trigram_index.lock().await;
            // Use the new content-aware method for proper trigram indexing
            trigram_guard
                .insert_with_content(doc_id, doc_path, &doc.content)
                .await?;
        }

        // Update path cache
        self.path_cache.write().await.insert(path, doc_id);

        // Flush all to ensure persistence
        self.storage.lock().await.flush().await?;
        self.primary_index.lock().await.flush().await?;
        self.trigram_index.lock().await.flush().await?;

        Ok(doc_id)
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<Document>> {
        // Check if cache is empty and rebuild if needed (lazy initialization)
        {
            let cache = self.path_cache.read().await;
            if cache.is_empty() {
                drop(cache); // Release read lock before rebuilding
                self.rebuild_path_cache().await?;
            }
        }

        // O(1) lookup using the path cache
        let cache = self.path_cache.read().await;

        if let Some(doc_id) = cache.get(path) {
            // Found in cache, get the document
            self.storage.lock().await.get(doc_id).await
        } else {
            Ok(None)
        }
    }

    async fn update_by_path(
        &self,
        path: &str,
        new_path: Option<String>,
        new_title: Option<String>,
        new_content: Option<String>,
    ) -> Result<()> {
        // First find the document by path
        let doc = self
            .get_by_path(path)
            .await?
            .context("Document not found")?;

        let doc_id = doc.id;

        // Get existing document
        let mut storage = self.storage.lock().await;
        let existing = storage.get(&doc_id).await?.context("Document not found")?;

        // Build updated document
        let mut builder = DocumentBuilder::new();

        // Use new values or keep existing ones
        builder = builder.path(new_path.as_ref().unwrap_or(&existing.path.to_string()))?;
        builder = builder.title(new_title.as_ref().unwrap_or(&existing.title.to_string()))?;

        let content = if let Some(new_content) = new_content {
            new_content.into_bytes()
        } else {
            existing.content.clone()
        };
        builder = builder.content(content);

        // Build and set the same ID and created_at
        let mut updated_doc = builder.build()?;
        updated_doc.id = doc_id;
        updated_doc.created_at = existing.created_at;

        // Ensure updated_at is newer than the existing one
        // In case of rapid updates, add a small increment to ensure it's different
        if updated_doc.updated_at <= existing.updated_at {
            use chrono::Duration;
            updated_doc.updated_at = existing.updated_at + Duration::milliseconds(1);
        }

        // Update storage
        storage.update(updated_doc.clone()).await?;

        // Update indices and cache if path changed
        if let Some(ref new_path_str) = new_path {
            let new_validated_path = ValidatedPath::new(new_path_str)?;
            self.primary_index
                .lock()
                .await
                .update(doc_id, new_validated_path.clone())
                .await?;
            // Use update_with_content for trigram index since it needs content
            self.trigram_index
                .lock()
                .await
                .update_with_content(doc_id, new_validated_path, &updated_doc.content)
                .await?;

            // Update cache: remove old path, add new path
            let mut cache = self.path_cache.write().await;
            cache.retain(|_, id| *id != doc_id);
            cache.insert(new_path_str.clone(), doc_id);
        }

        Ok(())
    }

    /// Centralized delete method that ensures all storage systems remain synchronized
    /// This is the ONLY method that should be used to delete documents
    async fn delete_document(&self, doc_id: &ValidatedDocumentId) -> Result<bool> {
        // Use the coordinated deletion service to ensure proper synchronization
        let deleted = self.deletion_service.delete_document(doc_id).await?;

        if deleted {
            // Remove from path cache if it exists
            {
                let mut cache = self.path_cache.write().await;
                cache.retain(|_, cached_id| cached_id != doc_id);
            }
        }

        Ok(deleted)
    }

    /// Get access to the coordinated deletion service for other modules
    #[allow(dead_code)] // Will be used by HTTP server and MCP tools in future integration
    pub fn get_deletion_service(&self) -> &kotadb::CoordinatedDeletionService {
        &self.deletion_service
    }

    async fn delete_by_path(&self, path: &str) -> Result<bool> {
        // Check if cache is empty and rebuild if needed (lazy initialization)
        {
            let cache = self.path_cache.read().await;
            if cache.is_empty() {
                drop(cache); // Release read lock before rebuilding
                self.rebuild_path_cache().await?;
            }
        }

        // First find the document by path using cache
        let doc_id = {
            let cache = self.path_cache.read().await;
            cache.get(path).copied()
        };

        if let Some(doc_id) = doc_id {
            // Use the centralized delete method to ensure coordination
            self.delete_document(&doc_id).await
        } else {
            Ok(false)
        }
    }

    #[allow(dead_code)]
    async fn search(
        &self,
        query_text: &str,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> Result<Vec<Document>> {
        let (documents, _) = self.search_with_count(query_text, tags, limit).await?;
        Ok(documents)
    }

    async fn search_with_count(
        &self,
        query_text: &str,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> Result<(Vec<Document>, usize)> {
        // Handle empty queries at the method level - return nothing
        if query_text.is_empty() {
            return Ok((Vec::new(), 0));
        }

        // Build query
        let mut query_builder = QueryBuilder::new();

        if query_text != "*" {
            query_builder = query_builder.with_text(query_text)?;
        }

        if let Some(tag_list) = tags {
            for tag in tag_list {
                query_builder = query_builder.with_tag(tag)?;
            }
        }

        query_builder = query_builder.with_limit(limit)?;
        let query = query_builder.build()?;

        // Route to appropriate index based on query type
        // NOTE: Wildcard queries (containing "*") are explicitly routed to the primary index
        // because it supports pattern matching. Trigram indices are designed for full-text
        // search and don't handle wildcard patterns. Empty queries are handled above and return nothing.
        let doc_ids = if query_text.contains('*') {
            // Use Primary Index for wildcard/pattern queries
            self.primary_index.lock().await.search(&query).await?
        } else {
            // Use Trigram Index for full-text search queries
            self.trigram_index.lock().await.search(&query).await?
        };

        // Store total count before limiting
        let total_count = doc_ids.len();

        // Retrieve documents from storage
        let doc_ids_limited: Vec<_> = doc_ids.into_iter().take(limit).collect();
        let mut documents = Vec::with_capacity(doc_ids_limited.len());
        let storage = self.storage.lock().await;

        for doc_id in doc_ids_limited {
            if let Some(doc) = storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        Ok((documents, total_count))
    }

    #[allow(dead_code)]
    async fn list_all(&self, limit: usize) -> Result<Vec<Document>> {
        let all_docs = self.storage.lock().await.list_all().await?;
        Ok(all_docs.into_iter().take(limit).collect())
    }

    async fn stats(&self) -> Result<(usize, usize)> {
        let all_docs = self.storage.lock().await.list_all().await?;
        let doc_count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();
        Ok((doc_count, total_size))
    }

    /// Flush any buffered writes to ensure durability
    async fn flush(&self) -> Result<()> {
        self.storage.lock().await.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use kotadb::{
        create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rebuild_indices_empty_storage() -> Result<()> {
        // Test that rebuild_indices handles empty storage gracefully
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        let storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        let primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;

        let trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
        let trigram_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        let deletion_service = kotadb::CoordinatedDeletionService::new(
            storage_arc.clone(),
            primary_index_arc.clone(),
            trigram_index_arc.clone(),
        );

        let db = Database {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
            deletion_service,
        };

        // Should not panic with empty storage
        db.rebuild_indices().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_rebuild_indices_batch_processing() -> Result<()> {
        // Test that rebuild_indices handles many documents efficiently
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(1000)).await?;

        // Add test documents to storage
        for i in 0..150 {
            let doc = DocumentBuilder::new()
                .path(format!("batch/doc_{}.md", i))?
                .title(format!("Batch Document {}", i))?
                .content(format!("Content for batch document {}", i).as_bytes())
                .build()?;
            storage.insert(doc).await?;
        }

        let primary_index =
            create_primary_index(primary_path.to_str().unwrap(), Some(1000)).await?;

        let trigram_index =
            create_trigram_index(trigram_path.to_str().unwrap(), Some(1000)).await?;

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
        let trigram_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        let deletion_service = kotadb::CoordinatedDeletionService::new(
            storage_arc.clone(),
            primary_index_arc.clone(),
            trigram_index_arc.clone(),
        );

        let db = Database {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
            deletion_service,
        };

        // Time the rebuild operation
        let start = std::time::Instant::now();
        db.rebuild_indices().await?;
        let duration = start.elapsed();

        // Verify all documents are indexed
        let query = QueryBuilder::new().with_limit(200)?.build()?;
        let results = db.primary_index.lock().await.search(&query).await?;
        assert!(
            results.len() >= 150,
            "Expected at least 150 documents, got {}",
            results.len()
        );

        // Performance check: should complete in reasonable time
        assert!(
            duration.as_secs() < 3,
            "Batch rebuild took too long: {:?}",
            duration
        );

        Ok(())
    }
}

/// Create a relationship query engine for the given database path
#[cfg(feature = "tree-sitter-parsing")]
async fn create_relationship_engine(db_path: &Path) -> Result<RelationshipQueryEngine> {
    // Load real symbol storage from the main database storage path (db_path/storage)
    // This ensures we read symbols from the same location where they were written
    let storage_path = db_path.join("storage");

    // Ensure the database directory exists
    if !storage_path.exists() {
        std::fs::create_dir_all(&storage_path)
            .with_context(|| format!("Failed to create database directory: {:?}", storage_path))?;
    }
    let file_storage = create_file_storage(
        storage_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid storage path: {:?}", storage_path))?,
        Some(100), // Cache size
    )
    .await?;

    // Create symbol storage with dual storage architecture (document + graph)
    // This ensures relationships can be stored and queried efficiently
    let graph_path = storage_path.join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;
    let graph_config = kotadb::graph_storage::GraphStorageConfig::default();
    let graph_storage =
        kotadb::native_graph_storage::NativeGraphStorage::new(graph_path, graph_config).await?;

    let symbol_storage =
        SymbolStorage::with_graph_storage(Box::new(file_storage), Box::new(graph_storage)).await?;

    // Load statistics to check if we have data
    let stats = symbol_storage.get_stats();

    // If no symbols exist, return error with actionable guidance
    if stats.total_symbols == 0 {
        return Err(anyhow::anyhow!(
            "No symbols found in database. Required steps:\n\
             1. Ingest a repository with symbols: kotadb ingest-repo /path/to/repo\n\
             2. Verify ingestion: kotadb symbol-stats\n\
             3. Then retry this command"
        ));
    }

    // Build dependency graph from existing symbol relationships
    let dependency_graph = symbol_storage.to_dependency_graph().await?;

    Ok(RelationshipQueryEngine::new(
        dependency_graph,
        symbol_storage,
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args first to get verbose flag
    let cli = Cli::parse();

    // Initialize logging with appropriate level based on verbose flag
    let _ = init_logging_with_level(cli.verbose); // Ignore error if already initialized

    // Run everything within trace context
    with_trace_id("kotadb-cli", async move {
        // Initialize database
        let db = Database::new(&cli.db_path, cli.binary_index).await?;

        match cli.command {
            Commands::Serve { port } => {
                // Create storage for the HTTP server
                let storage_path = cli.db_path.join("storage");
                std::fs::create_dir_all(&storage_path)?;

                let storage = create_file_storage(
                    storage_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid storage path: {:?}", storage_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?;

                // Wrap storage with observability and validation
                let wrapped_storage = create_wrapped_storage(storage, 1000).await;
                let shared_storage = Arc::new(tokio::sync::Mutex::new(wrapped_storage));

                println!("🚀 Starting KotaDB HTTP server on port {port}");
                println!("📄 API endpoints:");
                println!("   POST   /documents       - Create document");
                println!("   GET    /documents/:id   - Get document");
                println!("   PUT    /documents/:id   - Update document");
                println!("   DELETE /documents/:id   - Delete document");
                println!("   GET    /documents/search - Search documents");
                println!("   GET    /health         - Health check");
                println!();

                start_server(shared_storage, port).await?;
            }

            Commands::Insert {
                path,
                title,
                content,
            } => {
                // Read content from stdin if not provided
                let content = match content {
                    Some(c) => c,
                    None => {
                        use std::io::Read;
                        let mut buffer = String::new();
                        std::io::stdin().read_to_string(&mut buffer)?;
                        buffer
                    }
                };

                let doc_id = db.insert(path.clone(), title.clone(), content).await?;
                // Ensure the write is persisted before exiting
                db.flush().await?;
                println!("✅ Document inserted successfully!");
                println!("   ID: {}", doc_id.as_uuid());
                println!("   Path: {path}");
                println!("   Title: {title}");
            }

            Commands::Get { path } => match db.get_by_path(&path).await? {
                Some(doc) => {
                    println!("📄 Document found:");
                    println!("   ID: {}", doc.id.as_uuid());
                    println!("   Path: {}", doc.path.as_str());
                    println!("   Title: {}", doc.title.as_str());
                    println!("   Size: {} bytes", doc.size);
                    println!("   Created: {}", doc.created_at);
                    println!("   Updated: {}", doc.updated_at);
                    println!("\n--- Content ---");
                    println!("{}", String::from_utf8_lossy(&doc.content));
                }
                None => {
                    println!("❌ Document not found");
                }
            },

            Commands::Update {
                path,
                new_path,
                title,
                content,
            } => {
                // Read content from stdin if specified but not provided
                let content = if content.as_ref().map(|c| c == "-").unwrap_or(false) {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    Some(buffer)
                } else {
                    content
                };

                db.update_by_path(&path, new_path, title, content).await?;
                // Ensure the write is persisted before exiting
                db.flush().await?;
                println!("✅ Document updated successfully!");
            }

            Commands::Delete { path } => {
                let deleted = db.delete_by_path(&path).await?;
                // Ensure the deletion is persisted before exiting
                if deleted {
                    db.flush().await?;
                    println!("✅ Document deleted successfully!");
                } else {
                    println!("❌ Document not found");
                }
            }

            Commands::Search { query, limit, tags } => {
                // Handle empty query explicitly - return nothing with informative message
                if query.is_empty() {
                    println!("Empty search query provided. Please specify a search term.");
                    println!("Use 'list' command to view all documents, or '*' for wildcard search.");
                    return Ok(());
                }

                let tag_list = tags.map(|t| t.split(',').map(String::from).collect());
                let (results, total_count) = db.search_with_count(&query, tag_list, limit).await?;

                if results.is_empty() {
                    println!("No documents found matching the query");
                } else {
                    // Show clear count information for LLM agents
                    if results.len() < total_count {
                        println!("Showing {} of {} results", results.len(), total_count);
                    } else {
                        println!("Found {} documents", results.len());
                    }
                    println!();
                    for doc in results {
                        // Minimal output optimized for LLM consumption
                        println!("{}", doc.path.as_str());
                        println!("  id: {}", doc.id.as_uuid());
                        println!("  title: {}", doc.title.as_str());
                        println!("  size: {} bytes", doc.size);
                        println!();
                    }
                }
            }

            Commands::List { limit } => {
                // Get all documents to know total, then limit
                let all_docs = db.storage.lock().await.list_all().await?;
                let total_count = all_docs.len();
                let documents: Vec<_> = all_docs.into_iter().take(limit).collect();

                if documents.is_empty() {
                    println!("No documents in database");
                } else {
                    // Clear count information for LLM agents
                    if documents.len() < total_count {
                        println!("Showing {} of {} documents", documents.len(), total_count);
                    } else {
                        println!("Total documents: {}", documents.len());
                    }
                    println!();
                    for doc in documents {
                        println!("{}", doc.path.as_str());
                        println!("  id: {}", doc.id.as_uuid());
                        println!("  title: {}", doc.title.as_str());
                        println!("  size: {} bytes", doc.size);
                        println!();
                    }
                }
            }

            Commands::Stats => {
                let (count, total_size) = db.stats().await?;
                println!("Database Statistics");
                println!("  total_documents: {count}");
                println!("  total_size: {total_size} bytes");
                if count > 0 {
                    println!("  average_size: {} bytes", total_size / count);
                }
            }

            Commands::Validate => {
                println!("🔍 Running search functionality validation...");

                let validation_result = {
                    let storage = db.storage.lock().await;
                    let primary_index = db.primary_index.lock().await;
                    let trigram_index = db.trigram_index.lock().await;
                    validate_post_ingestion_search(&*storage, &*primary_index, &*trigram_index).await?
                };

                // Display detailed results
                println!("\n📋 Validation Results:");
                println!("   Status: {}", match validation_result.overall_status {
                    ValidationStatus::Passed => "✅ PASSED",
                    ValidationStatus::Warning => "⚠️ WARNING",
                    ValidationStatus::Failed => "❌ FAILED",
                });
                println!("   Checks: {}/{} passed", validation_result.passed_checks, validation_result.total_checks);

                // Show individual check results
                for check in &validation_result.check_results {
                    let status_icon = if check.passed { "✅" } else { "❌" };
                    let critical_mark = if check.critical { " [CRITICAL]" } else { "" };
                    println!("   {} {}{}", status_icon, check.name, critical_mark);
                    if let Some(ref details) = check.details {
                        println!("      {}", details);
                    }
                    if let Some(ref error) = check.error {
                        println!("      Error: {}", error);
                    }
                }

                // Show issues and recommendations
                if !validation_result.issues.is_empty() {
                    println!("\n🚨 Issues Found:");
                    for issue in &validation_result.issues {
                        println!("   - {}", issue);
                    }
                }

                if !validation_result.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &validation_result.recommendations {
                        println!("   • {}", rec);
                    }
                }

                // Show warnings if any
                if !validation_result.warnings.is_empty() {
                    println!("\n⚠️ Warnings:");
                    for warning in &validation_result.warnings {
                        println!("   - {}", warning);
                    }
                }

                // Exit with error code if validation failed
                if validation_result.overall_status == ValidationStatus::Failed {
                    return Err(anyhow::anyhow!("Search validation failed"));
                }
            }

            Commands::VerifyDocs => {
                use kotadb::DocumentationVerifier;

                println!("📋 Running comprehensive documentation verification...");
                println!("   Checking claims vs actual implementation");
                println!();

                let verifier = DocumentationVerifier::new();
                let report = verifier.run_full_verification()?;

                println!("📊 Verification Results:");
                println!("   {}", report.summary);
                println!();

                // Show verification status
                if report.is_acceptable() {
                    println!("✅ Documentation accuracy is acceptable");
                } else {
                    println!("❌ Documentation accuracy needs improvement");
                }

                // Show detailed check results
                println!("\n📝 Feature Verification Details:");
                for check in &report.checks {
                    let status_icon = match check.status {
                        kotadb::VerificationStatus::Verified => "✅",
                        kotadb::VerificationStatus::Missing => "❌",
                        kotadb::VerificationStatus::Partial => "⚠️",
                        kotadb::VerificationStatus::Undocumented => "📝",
                    };

                    let severity_badge = match check.severity {
                        kotadb::Severity::Critical => " [CRITICAL]",
                        kotadb::Severity::High => " [HIGH]",
                        kotadb::Severity::Medium => " [MEDIUM]",
                        _ => "",
                    };

                    println!("   {} {}{}", status_icon, check.feature, severity_badge);
                    println!("      Claim: {}", check.documented_claim);
                    println!("      Reality: {}", check.actual_implementation);

                    if let Some(ref rec) = check.recommendation {
                        println!("      💡 Recommendation: {}", rec);
                    }
                    println!();
                }

                // Show critical issues
                if !report.critical_issues.is_empty() {
                    println!("🚨 Critical Issues Found:");
                    for issue in &report.critical_issues {
                        println!("   - {}", issue);
                    }
                    println!();
                }

                // Show recommendations
                if !report.recommendations.is_empty() {
                    println!("💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("   • {}", rec);
                    }
                    println!();
                }

                // Exit with error code if documentation is unacceptable
                if !report.is_acceptable() {
                    return Err(anyhow::anyhow!(
                        "Documentation verification failed. {} critical issues found.",
                        report.critical_issues.len()
                    ));
                }

                println!("✨ Documentation verification completed successfully!");
            }

            #[cfg(feature = "git-integration")]
            Commands::IngestRepo {
                repo_path,
                prefix,
                include_files,
                include_commits,
                max_file_size_mb,
                #[cfg(feature = "tree-sitter-parsing")]
                extract_symbols,
                #[cfg(feature = "tree-sitter-parsing")]
                no_symbols,
            } => {
                use indicatif::{ProgressBar, ProgressStyle};
                use kotadb::git::types::IngestionOptions;
                use kotadb::git::{IngestionConfig, ProgressCallback, RepositoryIngester};

                println!("🔄 Ingesting git repository: {:?}", repo_path);

                // Determine if symbols should be extracted
                #[cfg(feature = "tree-sitter-parsing")]
                let should_extract_symbols = if no_symbols {
                    println!("⚠️  Symbol extraction disabled via --no-symbols flag");
                    false
                } else if let Some(extract) = extract_symbols {
                    if extract {
                        println!("✅ Symbol extraction enabled via --extract-symbols flag");
                    } else {
                        println!("⚠️  Symbol extraction disabled via --extract-symbols=false");
                    }
                    extract
                } else {
                    println!("✅ Symbol extraction enabled (default with tree-sitter feature)");
                    true // Default to true when tree-sitter is available
                };

                // Configure ingestion options
                #[allow(unused_mut)]
                let mut options = IngestionOptions {
                    include_file_contents: include_files,
                    include_commit_history: include_commits,
                    max_file_size: max_file_size_mb * 1024 * 1024,
                    ..Default::default()
                };

                #[cfg(feature = "tree-sitter-parsing")]
                {
                    options.extract_symbols = should_extract_symbols;
                }

                let config = IngestionConfig {
                    path_prefix: prefix,
                    options,
                    create_index: true,
                    organization_config: Some(kotadb::git::RepositoryOrganizationConfig::default()),
                };

                // Create progress bar
                let progress_bar = ProgressBar::new_spinner();
                progress_bar.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .expect("Valid template")
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                );
                progress_bar.set_message("Initializing...");

                // Create progress callback
                let pb = progress_bar.clone();
                let progress_callback: ProgressCallback = Box::new(move |message: &str| {
                    pb.set_message(message.to_string());
                    pb.tick();
                });

                // Create ingester and run ingestion with progress
                let ingester = RepositoryIngester::new(config.clone());
                let mut storage = db.storage.lock().await;

                #[cfg(feature = "tree-sitter-parsing")]
                let result = if config.options.extract_symbols {
                    // Use binary format for efficient symbol storage
                    let symbol_db_path = cli.db_path.join("symbols.kota");
                    ingester.ingest_with_binary_symbols(
                        &repo_path,
                        &mut *storage,
                        &symbol_db_path,
                        Some(progress_callback),
                    ).await?
                } else {
                    ingester.ingest_with_progress(&repo_path, &mut *storage, Some(progress_callback)).await?
                };

                #[cfg(not(feature = "tree-sitter-parsing"))]
                let result = ingester.ingest_with_progress(&repo_path, &mut **storage, Some(progress_callback)).await?;

                progress_bar.finish_with_message("✅ Ingestion complete");

                // Release the storage lock before rebuilding indices
                drop(storage);

                // Rebuild indices and cache after ingestion with progress indication
                let rebuild_progress = ProgressBar::new_spinner();
                rebuild_progress.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.blue} {msg}")
                        .expect("Valid template")
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                );

                rebuild_progress.set_message("Rebuilding primary and trigram indices...");
                db.rebuild_indices().await?;

                rebuild_progress.set_message("Rebuilding path cache...");
                db.rebuild_path_cache().await?;

                rebuild_progress.finish_with_message("✅ Indices rebuilt");

                // Ensure all async operations are complete before validation
                println!("⏳ Ensuring index synchronization...");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Explicit flush verification
                {
                    let mut storage = db.storage.lock().await;
                    let mut primary_index = db.primary_index.lock().await;
                    let mut trigram_index = db.trigram_index.lock().await;
                    storage.flush().await?;
                    primary_index.flush().await?;
                    trigram_index.flush().await?;
                }

                // Validate search functionality after ingestion
                let validation_progress = ProgressBar::new_spinner();
                validation_progress.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.yellow} {msg}")
                        .expect("Valid template")
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                );

                validation_progress.set_message("Running search validation tests...");
                let validation_result = {
                    let storage = db.storage.lock().await;
                    let primary_index = db.primary_index.lock().await;
                    let trigram_index = db.trigram_index.lock().await;
                    validate_post_ingestion_search(&*storage, &*primary_index, &*trigram_index).await?
                };

                validation_progress.finish_with_message("✅ Validation complete");

                // Report validation results
                match validation_result.overall_status {
                    ValidationStatus::Passed => {
                        println!("✅ Search validation passed: All systems operational");
                    }
                    ValidationStatus::Warning => {
                        println!("⚠️ Search validation completed with warnings:");
                        for issue in &validation_result.issues {
                            println!("   - {}", issue);
                        }
                        println!("   Recommendations:");
                        for rec in &validation_result.recommendations {
                            println!("   • {}", rec);
                        }
                    }
                    ValidationStatus::Failed => {
                        println!("❌ Search validation failed - ingestion may not be fully operational:");
                        for issue in &validation_result.issues {
                            println!("   - {}", issue);
                        }
                        println!("   Recommendations:");
                        for rec in &validation_result.recommendations {
                            println!("   • {}", rec);
                        }

                        // Return error for critical failures
                        return Err(anyhow::anyhow!(
                            "Post-ingestion search validation failed. Search functionality is broken."
                        ));
                    }
                }

                // Show warnings for git ingestion
                if !validation_result.warnings.is_empty() {
                    println!("   Validation warnings:");
                    for warning in &validation_result.warnings {
                        println!("   ⚠️ {}", warning);
                    }
                }

                println!("✅ Repository ingestion complete!");
                println!("   Documents created: {}", result.documents_created);
                println!("   Files ingested: {}", result.files_ingested);
                println!("   Commits ingested: {}", result.commits_ingested);
                if result.symbols_extracted > 0 {
                    println!("   Symbols extracted: {} from {} files", result.symbols_extracted, result.files_with_symbols);
                }
                if result.errors > 0 {
                    println!("   ⚠️ Errors encountered: {}", result.errors);
                }

                // Show validation summary
                println!("   Validation: {} ({}/{})",
                    validation_result.summary(),
                    validation_result.passed_checks,
                    validation_result.total_checks
                );
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::SearchSymbols { pattern, limit, symbol_type } => {
                // Load symbol storage
                let storage_path = cli.db_path.join("storage");
                if !storage_path.exists() {
                    return Err(anyhow::anyhow!(
                        "No database found. Run 'ingest-repo' first to populate symbols."
                    ));
                }

                let file_storage = create_file_storage(
                    storage_path
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("Invalid storage path"))?,
                    Some(100),
                )
                .await?;

                // Create symbol storage
                let graph_path = storage_path.join("graph");
                tokio::fs::create_dir_all(&graph_path).await?;
                let graph_config = kotadb::graph_storage::GraphStorageConfig::default();
                let graph_storage =
                    kotadb::native_graph_storage::NativeGraphStorage::new(graph_path, graph_config).await?;

                let symbol_storage =
                    SymbolStorage::with_graph_storage(Box::new(file_storage), Box::new(graph_storage)).await?;

                // Search for symbols using the built-in search
                let mut matches = symbol_storage.search(&pattern, limit * 2); // Get extra for filtering

                // Filter by type if specified
                if let Some(ref filter_type) = symbol_type {
                    let filter_lower = filter_type.to_lowercase();
                    matches.retain(|entry| {
                        format!("{:?}", entry.symbol.kind).to_lowercase().contains(&filter_lower) ||
                        format!("{:?}", entry.symbol.symbol_type).to_lowercase().contains(&filter_lower)
                    });
                }

                // Limit results
                matches.truncate(limit);

                if matches.is_empty() {
                    println!("No symbols found matching '{}'", pattern);
                    if let Some(ref st) = symbol_type {
                        println!("  with type filter: {}", st);
                    }
                    if symbol_storage.get_stats().total_symbols == 0 {
                        println!("Note: No symbols in database. Run 'ingest-repo' first.");
                    }
                } else {
                    // Check if we have more results than shown
                    let full_results = symbol_storage.search(&pattern, limit + 1);
                    let has_more = full_results.len() > limit;

                    if has_more {
                        println!("Showing {} of {} matching symbols (use -l {} for more)",
                                limit, full_results.len(), limit * 2);
                    } else {
                        println!("Found {} matching symbols", matches.len());
                    }
                    println!();

                    for entry in matches {
                        println!("{}", entry.qualified_name);
                        println!("  type: {:?}", entry.symbol.symbol_type);
                        println!("  file: {}", entry.file_path.display());
                        println!("  line: {}", entry.symbol.start_line);
                        println!();
                    }
                }
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::FindCallers { target, limit } => {
                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let query_type = RelationshipQueryType::FindCallers {
                    target: target.clone(),
                };

                let mut result = relationship_engine.execute_query(query_type).await?;

                // Apply limit if specified
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                println!("{}", result.to_markdown());
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::ImpactAnalysis { target, limit } => {
                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let query_type = RelationshipQueryType::ImpactAnalysis {
                    target: target.clone(),
                };

                let mut result = relationship_engine.execute_query(query_type).await?;

                // Apply limit if specified
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                println!("{}", result.to_markdown());
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::RelationshipQuery { query, limit } => {
                let query_type = parse_natural_language_relationship_query(&query)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Could not parse query '{}'\n\
                            Valid query patterns:\n\
                            - what calls [symbol]?\n\
                            - what would break if I change [symbol]?\n\
                            - find unused functions\n\
                            - who uses [symbol]?\n\
                            - find callers of [symbol]",
                            query
                        )
                    })?;

                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let mut result = relationship_engine.execute_query(query_type).await?;
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                println!("{}", result.to_markdown());
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::SymbolStats => {
                println!("📊 Symbol Storage Statistics");
                println!("═══════════════════════════════");

                // Use the main database storage path (db_path/storage)
                // This ensures we read symbols from the same location where they were written
                let storage_path = cli.db_path.join("storage");
                // Create directory if it doesn't exist
                if !storage_path.exists() {
                    std::fs::create_dir_all(&storage_path)
                        .with_context(|| format!("Failed to create database directory: {:?}", storage_path))?;
                    println!("📁 Created database directory at {:?}", storage_path);
                }

                // Load symbol storage
                let file_storage = create_file_storage(
                    storage_path
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("Invalid storage path: {:?}", storage_path))?,
                    Some(100),
                )
                .await?;

                // Create symbol storage with dual storage architecture (document + graph)
                // This ensures we can properly read relationship statistics
                let graph_path = storage_path.join("graph");
                tokio::fs::create_dir_all(&graph_path).await?;
                let graph_config = kotadb::graph_storage::GraphStorageConfig::default();
                let graph_storage = kotadb::native_graph_storage::NativeGraphStorage::new(
                    graph_path,
                    graph_config,
                )
                .await?;

                let symbol_storage = SymbolStorage::with_graph_storage(
                    Box::new(file_storage),
                    Box::new(graph_storage),
                )
                .await?;
                let stats = symbol_storage.get_stats();
                let dep_stats = symbol_storage.get_dependency_stats();

                println!("\n📦 Symbol Storage Location:");
                println!("   Path: {:?}", storage_path);

                println!("\n🔤 Symbol Statistics:");
                println!("   Total symbols: {}", stats.total_symbols);
                println!("   Total files: {}", stats.file_count);

                if !stats.symbols_by_type.is_empty() {
                    println!("\n📝 Symbols by Type:");
                    for (symbol_type, count) in &stats.symbols_by_type {
                        println!("   {}: {}", symbol_type, count);
                    }
                }

                if !stats.symbols_by_language.is_empty() {
                    println!("\n🌐 Symbols by Language:");
                    for (language, count) in &stats.symbols_by_language {
                        println!("   {}: {}", language, count);
                    }
                }

                println!("\n🔗 Dependency Graph:");
                println!("   Total relationships: {}", dep_stats.total_relationships);
                println!("   Total symbols in graph: {}", dep_stats.total_symbols);

                if stats.total_symbols == 0 {
                    println!("\n💡 Tip: Run 'ingest-repo' on a repository to extract symbols");
                } else {
                    println!("\n✅ Symbol storage is ready for relationship queries!");
                    println!("💡 Try commands like:");
                    println!("   • find-callers <symbol>");
                    println!("   • impact-analysis <symbol>");
                    println!("   • relationship-query \"what calls X?\"");
                }
            }

            Commands::Benchmark { operations, benchmark_type, format, max_search_queries } => {
                println!("🔬 Running KotaDB Performance Benchmarks");
                println!("════════════════════════════════════════");
                println!("  Operations: {}", operations);
                println!("  Type: {}", benchmark_type);
                println!("  Format: {}", format);
                println!();

                use std::time::Instant;

                // Ensure database exists
                let db = Database::new(&cli.db_path, cli.binary_index).await?;

                let mut results = Vec::new();
                let mut search_count = 0usize;

                // Storage benchmarks
                if benchmark_type == "all" || benchmark_type == "storage" {
                    println!("📝 Storage Benchmarks:");

                    // Insert benchmark
                    let start = Instant::now();
                    for i in 0..operations {
                        let path = format!("benchmark/doc_{}.md", i);
                        let title = format!("Benchmark Doc {}", i);
                        let content = format!("Benchmark content {}", i);
                        db.insert(path, title, content).await?;
                    }
                    let insert_duration = start.elapsed();
                    let insert_ops_per_sec = operations as f64 / insert_duration.as_secs_f64();
                    println!("  Insert: {} ops in {:.2}s ({:.0} ops/sec)",
                             operations, insert_duration.as_secs_f64(), insert_ops_per_sec);
                    results.push(("insert", insert_duration, insert_ops_per_sec));

                    // Search benchmark (using search as proxy for read performance)
                    let start = Instant::now();
                    for i in 0..operations {
                        let path = format!("benchmark/doc_{}.md", i);
                        let _ = db.search(&path, None, 1).await?;
                    }
                    let read_duration = start.elapsed();
                    let read_ops_per_sec = operations as f64 / read_duration.as_secs_f64();
                    println!("  Read/Search: {} ops in {:.2}s ({:.0} ops/sec)",
                             operations, read_duration.as_secs_f64(), read_ops_per_sec);
                    results.push(("read_search", read_duration, read_ops_per_sec));
                }

                // Index benchmarks
                if benchmark_type == "all" || benchmark_type == "index" {
                    println!("\n🔍 Index Benchmarks:");

                    // Search benchmark (limited to prevent excessive runtime)
                    let start = Instant::now();
                    let search_limit = operations.min(max_search_queries);
                    for i in 0..search_limit {
                        let query = format!("content {}", i);
                        let _ = db.search(&query, None, 10).await?;
                    }
                    search_count = search_limit;
                    let search_duration = start.elapsed();
                    let search_ops_per_sec = search_count as f64 / search_duration.as_secs_f64();
                    println!("  Search: {} queries in {:.2}s ({:.0} queries/sec)",
                             search_count, search_duration.as_secs_f64(), search_ops_per_sec);
                    results.push(("search", search_duration, search_ops_per_sec));
                }

                // Output results based on format
                match format.as_str() {
                    "json" => {
                        let json_output = serde_json::json!({
                            "operations": operations,
                            "type": benchmark_type,
                            "results": results.iter().map(|(name, duration, ops_per_sec)| {
                                serde_json::json!({
                                    "operation": name,
                                    "duration_ms": duration.as_millis(),
                                    "ops_per_sec": ops_per_sec,
                                })
                            }).collect::<Vec<_>>(),
                        });
                        println!("\n{}", serde_json::to_string_pretty(&json_output)?);
                    }
                    "csv" => {
                        println!("\noperation,duration_ms,ops_per_sec");
                        for (name, duration, ops_per_sec) in results {
                            println!("{},{},{:.2}", name, duration.as_millis(), ops_per_sec);
                        }
                    }
                    _ => {
                        // Human format - already printed above
                        println!("\n✅ Benchmark complete!");
                    }
                }

                // Cleanup behavior documentation
                // Note: The Database struct doesn't expose a delete method by design
                // to maintain data integrity. Benchmark data is left for inspection.
                // This is intentional - users can:
                // 1. Inspect the benchmark data after runs
                // 2. Use a fresh database path for clean benchmarks
                // 3. Delete the database directory manually if needed
                println!("\n📊 Benchmark Complete!");
                println!("   Data remains in database for inspection at: {:?}", cli.db_path);
                println!("   💡 Tip: Use --db-path with a fresh directory for clean benchmarks");

                if search_count < operations {
                    println!("   ℹ️ Note: Search queries were limited to {} operations", search_count);
                    println!("      Use --max-search-queries to adjust this limit");
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
}
