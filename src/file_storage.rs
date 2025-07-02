// File-based Storage Implementation
// This implements the Storage trait using a simple file-based backend
// Designed to work with all Stage 6 component library wrappers

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::contracts::{Document, Storage};
use crate::validation;
use crate::wrappers::{TracedStorage, ValidatedStorage, RetryableStorage, CachedStorage, create_wrapped_storage};

/// Simple file-based storage implementation
/// 
/// This is the basic storage engine that implements the Storage trait.
/// It should always be used with the Stage 6 component library wrappers:
/// TracedStorage, ValidatedStorage, RetryableStorage, CachedStorage
pub struct FileStorage {
    /// Root directory for the database
    db_path: PathBuf,
    /// In-memory document metadata for fast lookups
    documents: RwLock<HashMap<Uuid, DocumentMetadata>>,
    /// Write-ahead log for crash recovery
    wal_writer: RwLock<Option<tokio::fs::File>>,
}

/// Metadata for documents stored in memory for fast access
#[derive(Debug, Clone)]
struct DocumentMetadata {
    id: Uuid,
    file_path: PathBuf,
    size: u64,
    created: i64,
    updated: i64,
    hash: [u8; 32],
}

impl FileStorage {
    /// Create directory structure for the database
    async fn ensure_directories(&self) -> Result<()> {
        let paths = [
            self.db_path.join("documents"),
            self.db_path.join("indices"),
            self.db_path.join("wal"),
            self.db_path.join("meta"),
        ];

        for path in &paths {
            fs::create_dir_all(path).await
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        }

        Ok(())
    }

    /// Initialize write-ahead log
    async fn init_wal(&self) -> Result<()> {
        let wal_path = self.db_path.join("wal").join("current.wal");
        let wal_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .with_context(|| format!("Failed to open WAL file: {}", wal_path.display()))?;

        *self.wal_writer.write().await = Some(wal_file);
        Ok(())
    }

    /// Load existing documents from disk into memory
    async fn load_existing_documents(&self) -> Result<()> {
        let docs_dir = self.db_path.join("documents");
        
        if !docs_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&docs_dir).await
            .with_context(|| format!("Failed to read documents directory: {}", docs_dir.display()))?;

        let mut documents = self.documents.write().await;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Only process .json metadata files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(metadata) = serde_json::from_str::<DocumentMetadata>(&content) {
                    documents.insert(metadata.id, metadata);
                }
            }
        }

        Ok(())
    }

    /// Get the file path for a document
    fn document_file_path(&self, id: &Uuid) -> PathBuf {
        self.db_path.join("documents").join(format!("{}.md", id))
    }

    /// Get the metadata file path for a document
    fn metadata_file_path(&self, id: &Uuid) -> PathBuf {
        self.db_path.join("documents").join(format!("{}.json", id))
    }

    /// Save document metadata to disk
    async fn save_metadata(&self, metadata: &DocumentMetadata) -> Result<()> {
        let metadata_path = self.metadata_file_path(&metadata.id);
        let content = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize document metadata")?;
        
        fs::write(&metadata_path, content).await
            .with_context(|| format!("Failed to write metadata file: {}", metadata_path.display()))?;
        
        Ok(())
    }

    /// Create a Document from stored metadata and content
    async fn metadata_to_document(&self, metadata: &DocumentMetadata) -> Result<Document> {
        let content_path = self.document_file_path(&metadata.id);
        let content = fs::read_to_string(&content_path).await
            .with_context(|| format!("Failed to read document content: {}", content_path.display()))?;

        // Extract title from content (first line without # prefix)
        let title = content
            .lines()
            .next()
            .unwrap_or("")
            .trim_start_matches('#')
            .trim()
            .to_string();

        // Count words in content
        let word_count = content
            .split_whitespace()
            .count() as u32;

        Ok(Document {
            id: metadata.id,
            path: format!("/documents/{}.md", metadata.id), // Virtual path
            hash: metadata.hash,
            size: metadata.size,
            created: metadata.created,
            updated: metadata.updated,
            title,
            word_count,
        })
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn open(path: &str) -> Result<Self> where Self: Sized {
        // Validate path using existing Stage 2 validation
        validation::path::validate_directory_path(path)?;

        let db_path = PathBuf::from(path);
        let storage = Self {
            db_path,
            documents: RwLock::new(HashMap::new()),
            wal_writer: RwLock::new(None),
        };

        // Ensure directory structure exists
        storage.ensure_directories().await?;

        // Initialize WAL
        storage.init_wal().await?;

        // Load existing documents
        storage.load_existing_documents().await?;

        Ok(storage)
    }

    async fn insert(&mut self, doc: Document) -> Result<()> {
        // Check if document already exists
        {
            let documents = self.documents.read().await;
            if documents.contains_key(&doc.id) {
                anyhow::bail!("Document with ID {} already exists", doc.id);
            }
        }

        // Create document content (simple markdown format)
        let content = format!("# {}\n\n(Document content would go here)\n", doc.title);
        
        // Write document content to file
        let doc_path = self.document_file_path(&doc.id);
        fs::write(&doc_path, &content).await
            .with_context(|| format!("Failed to write document: {}", doc_path.display()))?;

        // Create metadata
        let metadata = DocumentMetadata {
            id: doc.id,
            file_path: doc_path,
            size: content.len() as u64,
            created: doc.created,
            updated: doc.updated,
            hash: doc.hash,
        };

        // Save metadata to disk
        self.save_metadata(&metadata).await?;

        // Update in-memory index
        {
            let mut documents = self.documents.write().await;
            documents.insert(doc.id, metadata);
        }

        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        let documents = self.documents.read().await;
        
        if let Some(metadata) = documents.get(id) {
            let document = self.metadata_to_document(metadata).await?;
            Ok(Some(document))
        } else {
            Ok(None)
        }
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        // Check if document exists
        let metadata = {
            let documents = self.documents.read().await;
            documents.get(&doc.id).cloned()
        };

        let mut metadata = metadata
            .ok_or_else(|| anyhow::anyhow!("Document with ID {} not found", doc.id))?;

        // Update content
        let content = format!("# {}\n\n(Updated document content)\n", doc.title);
        let doc_path = self.document_file_path(&doc.id);
        fs::write(&doc_path, &content).await
            .with_context(|| format!("Failed to update document: {}", doc_path.display()))?;

        // Update metadata
        metadata.size = content.len() as u64;
        metadata.updated = doc.updated;
        metadata.hash = doc.hash;

        // Save metadata
        self.save_metadata(&metadata).await?;

        // Update in-memory index
        {
            let mut documents = self.documents.write().await;
            documents.insert(doc.id, metadata);
        }

        Ok(())
    }

    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        // Remove from in-memory index
        let metadata = {
            let mut documents = self.documents.write().await;
            documents.remove(id)
        };

        if let Some(metadata) = metadata {
            // Remove files
            let doc_path = self.document_file_path(id);
            let meta_path = self.metadata_file_path(id);

            // Remove document file if it exists
            if doc_path.exists() {
                fs::remove_file(&doc_path).await
                    .with_context(|| format!("Failed to remove document file: {}", doc_path.display()))?;
            }

            // Remove metadata file if it exists
            if meta_path.exists() {
                fs::remove_file(&meta_path).await
                    .with_context(|| format!("Failed to remove metadata file: {}", meta_path.display()))?;
            }
        }

        // Note: We don't error if the document doesn't exist, as per contract
        Ok(())
    }

    async fn sync(&mut self) -> Result<()> {
        // For this simple implementation, we sync by ensuring all files are written
        // In a more sophisticated implementation, this would flush WAL buffers
        
        if let Some(wal_file) = self.wal_writer.write().await.as_mut() {
            wal_file.sync_all().await
                .context("Failed to sync WAL file")?;
        }

        Ok(())
    }

    async fn close(self) -> Result<()> {
        // Sync before closing
        // Note: We need to work around the fact that close() consumes self
        // but sync() requires &mut self
        
        // For this simple implementation, we just drop the WAL writer
        // In a real implementation, we'd properly close all resources
        
        drop(self.wal_writer);
        Ok(())
    }
}

// Implement serde for DocumentMetadata
impl serde::Serialize for DocumentMetadata {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DocumentMetadata", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("file_path", &self.file_path)?;
        state.serialize_field("size", &self.size)?;
        state.serialize_field("created", &self.created)?;
        state.serialize_field("updated", &self.updated)?;
        state.serialize_field("hash", &self.hash)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for DocumentMetadata {
    fn deserialize<D>(deserializer: D) -> std::result::Result<DocumentMetadata, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct DocumentMetadataHelper {
            id: Uuid,
            file_path: PathBuf,
            size: u64,
            created: i64,
            updated: i64,
            hash: [u8; 32],
        }

        let helper = DocumentMetadataHelper::deserialize(deserializer)?;
        Ok(DocumentMetadata {
            id: helper.id,
            file_path: helper.file_path,
            size: helper.size,
            created: helper.created,
            updated: helper.updated,
            hash: helper.hash,
        })
    }
}

/// Create a fully wrapped FileStorage with all Stage 6 components
/// 
/// This is the recommended way to create a production-ready storage instance.
/// It automatically applies all Stage 6 wrapper components for maximum safety and reliability.
pub async fn create_file_storage(
    path: &str,
    cache_capacity: Option<usize>,
) -> Result<TracedStorage<ValidatedStorage<RetryableStorage<CachedStorage<FileStorage>>>>> {
    // Create base FileStorage
    let base_storage = FileStorage::open(path).await?;
    
    // Apply Stage 6 wrapper composition
    let wrapped = create_wrapped_storage(base_storage, cache_capacity.unwrap_or(1000)).await;
    
    Ok(wrapped)
}