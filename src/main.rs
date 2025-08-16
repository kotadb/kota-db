// KotaDB CLI - Simple command-line interface for database operations
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, create_wrapped_storage,
    init_logging, start_server, validate_post_ingestion_search, with_trace_id, Document,
    DocumentBuilder, Index, QueryBuilder, Storage, ValidatedDocumentId, ValidatedPath,
    ValidationStatus,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Parser)]
#[command(author, version, about = "KotaDB - A simple document database CLI", long_about = None)]
struct Cli {
    /// Database directory path
    #[arg(short, long, default_value = "./kota-db-data")]
    db_path: PathBuf,

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

    /// Search for documents
    Search {
        /// Search query text
        #[arg(default_value = "*")]
        query: String,
        /// Limit number of results
        #[arg(short, long, default_value = "10")]
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
    },
}

struct Database {
    storage: Arc<Mutex<Box<dyn Storage>>>,
    primary_index: Arc<Mutex<Box<dyn Index>>>,
    trigram_index: Arc<Mutex<Box<dyn Index>>>,
    // Cache for path -> document ID lookups (built lazily)
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl Database {
    async fn new(db_path: &Path) -> Result<Self> {
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
        let trigram_index = create_trigram_index(
            trigram_index_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
            })?,
            Some(1000),
        )
        .await?;

        let db = Self {
            storage: Arc::new(Mutex::new(Box::new(storage))),
            primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
            trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Build the path cache on startup
        db.rebuild_path_cache().await?;

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
            self.trigram_index
                .lock()
                .await
                .update(doc_id, new_validated_path)
                .await?;

            // Update cache: remove old path, add new path
            let mut cache = self.path_cache.write().await;
            cache.retain(|_, id| *id != doc_id);
            cache.insert(new_path_str.clone(), doc_id);
        }

        Ok(())
    }

    async fn delete_by_path(&self, path: &str) -> Result<bool> {
        // First find the document by path using cache
        let doc_id = {
            let cache = self.path_cache.read().await;
            cache.get(path).copied()
        };

        if let Some(doc_id) = doc_id {
            // Delete from storage
            let deleted = self.storage.lock().await.delete(&doc_id).await?;

            if deleted {
                // Delete from both indices
                self.primary_index.lock().await.delete(&doc_id).await?;
                self.trigram_index.lock().await.delete(&doc_id).await?;

                // Remove from cache
                self.path_cache.write().await.remove(path);
            }

            Ok(deleted)
        } else {
            Ok(false)
        }
    }

    async fn search(
        &self,
        query_text: &str,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> Result<Vec<Document>> {
        // Build query
        let mut query_builder = QueryBuilder::new();

        if query_text != "*" && !query_text.is_empty() {
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
        let doc_ids = if query_text == "*" || query_text.is_empty() {
            // Use Primary Index for wildcard queries
            self.primary_index.lock().await.search(&query).await?
        } else {
            // Use Trigram Index for text search queries
            self.trigram_index.lock().await.search(&query).await?
        };

        // Retrieve documents from storage
        let doc_ids_limited: Vec<_> = doc_ids.into_iter().take(limit).collect();
        let mut documents = Vec::with_capacity(doc_ids_limited.len());
        let storage = self.storage.lock().await;

        for doc_id in doc_ids_limited {
            if let Some(doc) = storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        Ok(documents)
    }

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

        let db = Database {
            storage: Arc::new(Mutex::new(Box::new(storage))),
            primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
            trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
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

        let db = Database {
            storage: Arc::new(Mutex::new(Box::new(storage))),
            primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
            trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _ = init_logging(); // Ignore error if already initialized

    let cli = Cli::parse();

    // Run everything within trace context
    with_trace_id("kotadb-cli", async move {
        // Initialize database
        let db = Database::new(&cli.db_path).await?;

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
                let tag_list = tags.map(|t| t.split(',').map(String::from).collect());
                let results = db.search(&query, tag_list, limit).await?;

                if results.is_empty() {
                    println!("No documents found matching the query");
                } else {
                    println!("🔍 Found {} documents:", results.len());
                    println!();
                    for doc in results {
                        println!("📄 {}", doc.title.as_str());
                        println!("   ID: {}", doc.id.as_uuid());
                        println!("   Path: {}", doc.path.as_str());
                        println!("   Size: {} bytes", doc.size);
                        println!();
                    }
                }
            }

            Commands::List { limit } => {
                let documents = db.list_all(limit).await?;

                if documents.is_empty() {
                    println!("No documents in database");
                } else {
                    println!("📚 Documents ({} total):", documents.len());
                    println!();
                    for doc in documents {
                        println!("📄 {}", doc.title.as_str());
                        println!("   ID: {}", doc.id.as_uuid());
                        println!("   Path: {}", doc.path.as_str());
                        println!("   Size: {} bytes", doc.size);
                        println!();
                    }
                }
            }

            Commands::Stats => {
                let (count, total_size) = db.stats().await?;
                println!("📊 Database Statistics:");
                println!("   Total documents: {count}");
                println!("   Total size: {total_size} bytes");
                if count > 0 {
                    println!("   Average size: {} bytes", total_size / count);
                }
            }

            Commands::Validate => {
                println!("🔍 Running search functionality validation...");

                let validation_result = {
                    let storage = db.storage.lock().await;
                    let primary_index = db.primary_index.lock().await;
                    let trigram_index = db.trigram_index.lock().await;
                    validate_post_ingestion_search(&**storage, &**primary_index, &**trigram_index).await?
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

            #[cfg(feature = "git-integration")]
            Commands::IngestRepo {
                repo_path,
                prefix,
                include_files,
                include_commits,
                max_file_size_mb,
            } => {
                use indicatif::{ProgressBar, ProgressStyle};
                use kotadb::git::types::IngestionOptions;
                use kotadb::git::{IngestionConfig, ProgressCallback, RepositoryIngester};

                println!("🔄 Ingesting git repository: {:?}", repo_path);

                // Configure ingestion options
                let options = IngestionOptions {
                    include_file_contents: include_files,
                    include_commit_history: include_commits,
                    max_file_size: max_file_size_mb * 1024 * 1024,
                    ..Default::default()
                };

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
                let ingester = RepositoryIngester::new(config);
                let mut storage = db.storage.lock().await;
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
                    validate_post_ingestion_search(&**storage, &**primary_index, &**trigram_index).await?
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
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
}
