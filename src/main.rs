// KotaDB CLI - Simple command-line interface for database operations
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, init_logging, with_trace_id,
    Document, DocumentBuilder, Index, QueryBuilder, Storage, ValidatedDocumentId, ValidatedPath,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    /// Insert a new document
    Insert {
        /// Path of the document (e.g., /docs/readme.md)
        path: String,
        /// Title of the document
        title: String,
        /// Content of the document (can be piped in)
        #[arg(value_name = "CONTENT")]
        content: Option<String>,
    },

    /// Get a document by ID
    Get {
        /// Document ID (UUID format)
        id: String,
    },

    /// Update an existing document
    Update {
        /// Document ID to update
        id: String,
        /// New path (optional)
        #[arg(short, long)]
        path: Option<String>,
        /// New title (optional)
        #[arg(short, long)]
        title: Option<String>,
        /// New content (optional, can be piped in)
        #[arg(short, long)]
        content: Option<String>,
    },

    /// Delete a document by ID
    Delete {
        /// Document ID to delete
        id: String,
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
}

struct Database {
    storage: Arc<Mutex<Box<dyn Storage>>>,
    primary_index: Arc<Mutex<Box<dyn Index>>>,
    trigram_index: Arc<Mutex<Box<dyn Index>>>,
}

impl Database {
    async fn new(db_path: &PathBuf) -> Result<Self> {
        let storage_path = db_path.join("storage");
        let primary_index_path = db_path.join("primary_index");
        let trigram_index_path = db_path.join("trigram_index");

        // Create directories if they don't exist
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&primary_index_path)?;
        std::fs::create_dir_all(&trigram_index_path)?;

        let storage = create_file_storage(
            storage_path.to_str().unwrap(),
            Some(100), // Cache size
        )
        .await?;

        let primary_index =
            create_primary_index(primary_index_path.to_str().unwrap(), Some(1000)).await?;
        let trigram_index =
            create_trigram_index(trigram_index_path.to_str().unwrap(), Some(1000)).await?;

        Ok(Self {
            storage: Arc::new(Mutex::new(Box::new(storage))),
            primary_index: Arc::new(Mutex::new(Box::new(primary_index))),
            trigram_index: Arc::new(Mutex::new(Box::new(trigram_index))),
        })
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

        let doc_id = doc.id.clone();
        let doc_path = ValidatedPath::new(&path)?;

        // Insert into storage
        self.storage.lock().await.insert(doc.clone()).await?;

        // Insert into both indices
        self.primary_index
            .lock()
            .await
            .insert(doc_id.clone(), doc_path.clone())
            .await?;

        // For trigram index, we need to pass the document content
        // Since the Index trait is limited, we'll use a workaround by adding content to the trigram index directly
        {
            use kotadb::TrigramIndex;
            let mut trigram_guard = self.trigram_index.lock().await;

            // Downcast to access trigram-specific functionality if possible
            // For now, use the standard insert (which only uses path)
            trigram_guard.insert(doc_id.clone(), doc_path).await?;
        }

        // Flush all to ensure persistence
        self.storage.lock().await.flush().await?;
        self.primary_index.lock().await.flush().await?;
        self.trigram_index.lock().await.flush().await?;

        Ok(doc_id)
    }

    async fn get(&self, id: &str) -> Result<Option<Document>> {
        let doc_id = ValidatedDocumentId::parse(id).context("Invalid document ID format")?;

        self.storage.lock().await.get(&doc_id).await
    }

    async fn update(
        &self,
        id: &str,
        new_path: Option<String>,
        new_title: Option<String>,
        new_content: Option<String>,
    ) -> Result<()> {
        let doc_id = ValidatedDocumentId::parse(id).context("Invalid document ID format")?;

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

        // Build and set the same ID
        let mut updated_doc = builder.build()?;
        updated_doc.id = doc_id.clone();

        // Update storage
        storage.update(updated_doc.clone()).await?;

        // Update indices if path changed
        if new_path.is_some() {
            let new_validated_path = ValidatedPath::new(&new_path.unwrap())?;
            self.primary_index
                .lock()
                .await
                .update(doc_id.clone(), new_validated_path.clone())
                .await?;
            self.trigram_index
                .lock()
                .await
                .update(doc_id, new_validated_path)
                .await?;
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let doc_id = ValidatedDocumentId::parse(id).context("Invalid document ID format")?;

        // Delete from storage
        let deleted = self.storage.lock().await.delete(&doc_id).await?;

        if deleted {
            // Delete from both indices
            self.primary_index.lock().await.delete(&doc_id).await?;
            self.trigram_index.lock().await.delete(&doc_id).await?;
        }

        Ok(deleted)
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
        let mut documents = Vec::new();
        let storage = self.storage.lock().await;

        for doc_id in doc_ids.into_iter().take(limit) {
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
                println!("✅ Document inserted successfully!");
                println!("   ID: {}", doc_id.as_uuid());
                println!("   Path: {}", path);
                println!("   Title: {}", title);
            }

            Commands::Get { id } => match db.get(&id).await? {
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
                id,
                path,
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

                db.update(&id, path, title, content).await?;
                println!("✅ Document updated successfully!");
            }

            Commands::Delete { id } => {
                let deleted = db.delete(&id).await?;
                if deleted {
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
                println!("   Total documents: {}", count);
                println!("   Total size: {} bytes", total_size);
                if count > 0 {
                    println!("   Average size: {} bytes", total_size / count);
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
}
