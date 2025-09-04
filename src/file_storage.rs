// File-based Storage Implementation
// This implements the Storage trait using a simple file-based backend
// Designed to work with all Stage 6 component library wrappers

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::contracts::{Document, Storage};
use crate::types::{ValidatedDocumentId, ValidatedPath, ValidatedTag, ValidatedTitle};
use crate::validation;
use crate::wrappers::create_wrapped_storage;
use chrono::{DateTime, Utc};

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
    wal_writer: Mutex<Option<tokio::fs::File>>,
}

/// Metadata for documents stored in memory for fast access
#[derive(Debug, Clone)]
struct DocumentMetadata {
    id: Uuid,
    file_path: PathBuf,
    original_path: String, // Store the original document path
    title: String,         // Store the original document title
    size: u64,
    created: i64,
    updated: i64,
    hash: [u8; 32],
    embedding: Option<Vec<f32>>, // Vector embedding for semantic search
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
            fs::create_dir_all(path)
                .await
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

        *self.wal_writer.lock().await = Some(wal_file);
        Ok(())
    }

    /// Load existing documents from disk into memory
    async fn load_existing_documents(&self) -> Result<()> {
        let docs_dir = self.db_path.join("documents");

        if !docs_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&docs_dir).await.with_context(|| {
            format!("Failed to read documents directory: {}", docs_dir.display())
        })?;

        // Collect all metadata files first, then process them
        let mut metadata_files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .json metadata files
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                metadata_files.push(path);
            }
        }

        // Process metadata files and collect results
        let mut metadata_list = Vec::new();
        for path in metadata_files {
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(metadata) = serde_json::from_str::<DocumentMetadata>(&content) {
                    metadata_list.push(metadata);
                }
            }
        }

        // Now acquire the lock and insert all metadata at once
        {
            let mut documents = self.documents.write().await;
            for metadata in metadata_list {
                documents.insert(metadata.id, metadata);
            }
        }

        Ok(())
    }

    /// Get the file path for a document
    fn document_file_path(&self, id: &Uuid) -> PathBuf {
        self.db_path.join("documents").join(format!("{id}.md"))
    }

    /// Get the metadata file path for a document
    fn metadata_file_path(&self, id: &Uuid) -> PathBuf {
        self.db_path.join("documents").join(format!("{id}.json"))
    }

    /// Save document metadata to disk
    async fn save_metadata(&self, metadata: &DocumentMetadata) -> Result<()> {
        let metadata_path = self.metadata_file_path(&metadata.id);
        let content = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize document metadata")?;

        fs::write(&metadata_path, content).await.with_context(|| {
            format!("Failed to write metadata file: {}", metadata_path.display())
        })?;

        Ok(())
    }

    /// Create a Document from stored metadata and content
    async fn metadata_to_document(&self, metadata: &DocumentMetadata) -> Result<Document> {
        let content_path = self.document_file_path(&metadata.id);
        let content = fs::read(&content_path).await.with_context(|| {
            format!(
                "Failed to read document content: {}",
                content_path.display()
            )
        })?;

        // Parse frontmatter for tags (title is now stored in metadata)
        let content_str = String::from_utf8_lossy(&content);

        // Parse frontmatter for tags
        let tags = if let Some(frontmatter) = crate::pure::metadata::parse_frontmatter(&content_str)
        {
            crate::pure::metadata::extract_tags(&frontmatter)
                .into_iter()
                .filter_map(|tag| ValidatedTag::new(&tag).ok())
                .collect()
        } else {
            Vec::new()
        };

        Ok(Document {
            id: ValidatedDocumentId::from_uuid(metadata.id)?,
            path: ValidatedPath::new(&metadata.original_path)?,
            title: ValidatedTitle::new(&metadata.title)?,
            content,
            tags,
            created_at: DateTime::<Utc>::from_timestamp(metadata.created, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid created timestamp"))?,
            updated_at: DateTime::<Utc>::from_timestamp(metadata.updated, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid updated timestamp"))?,
            size: metadata.size as usize,
            embedding: metadata.embedding.clone(),
        })
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Validate path for internal storage (allows absolute paths)
        validation::path::validate_storage_directory_path(path)?;

        let db_path = PathBuf::from(path);
        let storage = Self {
            db_path,
            documents: RwLock::new(HashMap::new()),
            wal_writer: Mutex::new(None),
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
        let doc_uuid = doc.id.as_uuid();
        {
            let documents = self.documents.read().await;
            if documents.contains_key(&doc_uuid) {
                anyhow::bail!("Document with ID {} already exists", doc_uuid);
            }
        }

        // Write document content to file with frontmatter
        let doc_path = self.document_file_path(&doc_uuid);

        // Add YAML frontmatter with tags if document has tags
        let content_to_write = if !doc.tags.is_empty() {
            let tag_strings: Vec<String> =
                doc.tags.iter().map(|t| t.as_str().to_string()).collect();
            // Create proper YAML frontmatter with tags array
            let mut frontmatter_data = std::collections::HashMap::new();
            frontmatter_data.insert(
                "tags".to_string(),
                serde_yaml::Value::Sequence(
                    tag_strings
                        .into_iter()
                        .map(serde_yaml::Value::String)
                        .collect(),
                ),
            );

            let frontmatter = format!(
                "---\n{}\n---\n",
                serde_yaml::to_string(&frontmatter_data)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize frontmatter: {}", e))?
                    .trim()
            );

            // Check if content already has frontmatter
            let content_str = String::from_utf8_lossy(&doc.content);
            if content_str.starts_with("---\n") {
                // Content already has frontmatter, replace it
                if let Some(end_pos) = content_str.find("\n---\n") {
                    let after_frontmatter = &content_str[end_pos + 5..];
                    format!("{frontmatter}{after_frontmatter}")
                } else {
                    // Malformed frontmatter, just prepend our frontmatter
                    format!("{frontmatter}{content_str}")
                }
            } else {
                // No frontmatter, add ours
                format!("{frontmatter}{content_str}")
            }
        } else {
            String::from_utf8_lossy(&doc.content).to_string()
        };

        fs::write(&doc_path, content_to_write.as_bytes())
            .await
            .with_context(|| format!("Failed to write document: {}", doc_path.display()))?;

        // Calculate hash
        let hash = crate::pure::metadata::calculate_hash(&doc.content);

        // Create metadata
        let metadata = DocumentMetadata {
            id: doc_uuid,
            file_path: doc_path,
            original_path: doc.path.as_str().to_string(),
            title: doc.title.as_str().to_string(),
            size: doc.content.len() as u64,
            created: doc.created_at.timestamp(),
            updated: doc.updated_at.timestamp(),
            hash,
            embedding: doc.embedding.clone(),
        };

        // Save metadata to disk
        self.save_metadata(&metadata).await?;

        // Update in-memory index
        {
            let mut documents = self.documents.write().await;
            documents.insert(doc_uuid, metadata);
        }

        Ok(())
    }

    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>> {
        let metadata = {
            let documents = self.documents.read().await;
            documents.get(&id.as_uuid()).cloned()
        };

        if let Some(metadata) = metadata {
            let document = self.metadata_to_document(&metadata).await?;
            Ok(Some(document))
        } else {
            Ok(None)
        }
    }

    async fn update(&mut self, doc: Document) -> Result<()> {
        // Check if document exists
        let doc_uuid = doc.id.as_uuid();
        let metadata = {
            let documents = self.documents.read().await;
            documents.get(&doc_uuid).cloned()
        };

        let mut metadata =
            metadata.ok_or_else(|| anyhow::anyhow!("Document with ID {} not found", doc_uuid))?;

        // Update content
        let doc_path = self.document_file_path(&doc_uuid);
        fs::write(&doc_path, &doc.content)
            .await
            .with_context(|| format!("Failed to update document: {}", doc_path.display()))?;

        // Calculate new hash
        let hash = crate::pure::metadata::calculate_hash(&doc.content);

        // Update metadata
        metadata.original_path = doc.path.as_str().to_string();
        metadata.title = doc.title.as_str().to_string();
        metadata.size = doc.content.len() as u64;
        metadata.updated = doc.updated_at.timestamp();
        metadata.hash = hash;
        metadata.embedding = doc.embedding.clone();

        // Save metadata
        self.save_metadata(&metadata).await?;

        // Update in-memory index
        {
            let mut documents = self.documents.write().await;
            documents.insert(doc_uuid, metadata);
        }

        Ok(())
    }

    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        // Remove from in-memory index
        let metadata = {
            let mut documents = self.documents.write().await;
            documents.remove(&id.as_uuid())
        };

        if let Some(_metadata) = metadata {
            // Remove files
            let doc_path = self.document_file_path(&id.as_uuid());
            let meta_path = self.metadata_file_path(&id.as_uuid());

            // Remove document file if it exists
            if doc_path.exists() {
                fs::remove_file(&doc_path).await.with_context(|| {
                    format!("Failed to remove document file: {}", doc_path.display())
                })?;
            }

            // Remove metadata file if it exists
            if meta_path.exists() {
                fs::remove_file(&meta_path).await.with_context(|| {
                    format!("Failed to remove metadata file: {}", meta_path.display())
                })?;
            }

            Ok(true) // Document was deleted
        } else {
            Ok(false) // Document didn't exist
        }
    }

    async fn sync(&mut self) -> Result<()> {
        // For this simple implementation, we sync by ensuring all files are written
        // In a more sophisticated implementation, this would flush WAL buffers

        if let Some(wal_file) = self.wal_writer.lock().await.as_mut() {
            wal_file
                .sync_all()
                .await
                .context("Failed to sync WAL file")?;
        }

        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<Document>> {
        // Clone metadata to avoid holding the lock across async calls
        let metadata_list = {
            let documents = self.documents.read().await;
            documents.values().cloned().collect::<Vec<_>>()
        };

        let mut result = Vec::with_capacity(metadata_list.len());

        // Convert each metadata entry to a full document
        for metadata in metadata_list {
            if let Ok(doc) = self.metadata_to_document(&metadata).await {
                result.push(doc);
            }
        }

        Ok(result)
    }

    async fn flush(&mut self) -> Result<()> {
        // For file-based storage, flush is similar to sync
        // Ensure all buffered writes are persisted to disk
        self.sync().await?;

        // Additionally, we could force metadata updates
        let metadata_list = {
            let documents = self.documents.read().await;
            documents.values().cloned().collect::<Vec<_>>()
        };

        for metadata in metadata_list {
            self.save_metadata(&metadata).await?;
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
        let mut state = serializer.serialize_struct("DocumentMetadata", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("file_path", &self.file_path)?;
        state.serialize_field("original_path", &self.original_path)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("size", &self.size)?;
        state.serialize_field("created", &self.created)?;
        state.serialize_field("updated", &self.updated)?;
        state.serialize_field("hash", &self.hash)?;
        state.serialize_field("embedding", &self.embedding)?;
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
            original_path: Option<String>, // Optional for backward compatibility
            title: Option<String>,         // Optional for backward compatibility
            size: u64,
            created: i64,
            updated: i64,
            hash: [u8; 32],
            embedding: Option<Vec<f32>>, // Optional for backward compatibility
        }

        let helper = DocumentMetadataHelper::deserialize(deserializer)?;
        Ok(DocumentMetadata {
            id: helper.id,
            file_path: helper.file_path.clone(),
            original_path: helper
                .original_path
                .unwrap_or_else(|| format!("/documents/{}.md", helper.id)),
            title: helper
                .title
                .unwrap_or_else(|| format!("Document {}", helper.id)),
            size: helper.size,
            created: helper.created,
            updated: helper.updated,
            hash: helper.hash,
            embedding: helper.embedding,
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
) -> Result<impl Storage> {
    // Create base FileStorage
    let base_storage = FileStorage::open(path).await?;

    // Apply Stage 6 wrapper composition including buffering
    let wrapped = create_wrapped_storage(base_storage, cache_capacity.unwrap_or(1000)).await;

    Ok(wrapped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;
    use tempfile::TempDir;
    use tokio::fs;
    use uuid::Uuid;

    async fn create_test_storage() -> (FileStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = FileStorage::open(temp_dir.path().to_str().unwrap())
            .await
            .expect("Failed to open storage");
        (storage, temp_dir)
    }

    fn create_test_document(content: &str) -> Document {
        let now = Utc::now();
        Document {
            id: ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap(),
            path: ValidatedPath::new("test/document.md").unwrap(),
            title: ValidatedTitle::new("Test Document").unwrap(),
            content: content.as_bytes().to_vec(),
            tags: vec![ValidatedTag::new("test").unwrap()],
            created_at: now,
            updated_at: now,
            size: content.len(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
        }
    }

    fn create_test_document_no_tags(content: &str) -> Document {
        let now = Utc::now();
        Document {
            id: ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap(),
            path: ValidatedPath::new("test/document.md").unwrap(),
            title: ValidatedTitle::new("Test Document").unwrap(),
            content: content.as_bytes().to_vec(),
            tags: vec![],
            created_at: now,
            updated_at: now,
            size: content.len(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
        }
    }

    #[tokio::test]
    async fn test_storage_initialization() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Check that directories were created
        let base_path = &storage.db_path;
        assert!(base_path.join("documents").exists());
        assert!(base_path.join("indices").exists());
        assert!(base_path.join("wal").exists());
        assert!(base_path.join("meta").exists());

        // Check that WAL file was created
        assert!(base_path.join("wal").join("current.wal").exists());
    }

    #[tokio::test]
    async fn test_document_insertion_and_retrieval() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document_no_tags("Hello, world!"); // Use no tags for this test

        // Insert document
        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Retrieve document
        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_some());

        let retrieved_doc = retrieved.unwrap();
        assert_eq!(retrieved_doc.id, doc.id);
        assert_eq!(retrieved_doc.path, doc.path);
        assert_eq!(retrieved_doc.title, doc.title);
        assert_eq!(retrieved_doc.content, doc.content); // Original content should match
        assert_eq!(retrieved_doc.tags, doc.tags); // Empty tags
        assert_eq!(retrieved_doc.embedding, doc.embedding);
    }

    #[tokio::test]
    async fn test_document_update() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let mut doc = create_test_document("Original content");

        // Insert document
        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Update document
        doc.content = b"Updated content".to_vec();
        doc.size = doc.content.len();
        doc.updated_at = Utc::now();
        storage
            .update(doc.clone())
            .await
            .expect("Failed to update document");

        // Retrieve and verify
        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_some());

        let retrieved_doc = retrieved.unwrap();
        assert_eq!(retrieved_doc.content, b"Updated content");
        assert_eq!(retrieved_doc.size, 15);
    }

    #[tokio::test]
    async fn test_document_deletion() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("To be deleted");

        // Insert document
        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Verify it exists
        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_some());

        // Delete document
        let deleted = storage
            .delete(&doc.id)
            .await
            .expect("Failed to delete document");
        assert!(deleted);

        // Verify it's gone
        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_none());

        // Delete non-existent document should return false
        let deleted_again = storage
            .delete(&doc.id)
            .await
            .expect("Failed to delete document");
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_all_documents() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc1 = create_test_document("First document");
        let doc2 = create_test_document("Second document");

        // Initially empty
        let docs = storage.list_all().await.expect("Failed to list documents");
        assert_eq!(docs.len(), 0);

        // Insert documents
        storage
            .insert(doc1.clone())
            .await
            .expect("Failed to insert document");
        storage
            .insert(doc2.clone())
            .await
            .expect("Failed to insert document");

        // List all
        let docs = storage.list_all().await.expect("Failed to list documents");
        assert_eq!(docs.len(), 2);

        let ids: Vec<_> = docs.iter().map(|d| d.id.as_uuid()).collect();
        assert!(ids.contains(&doc1.id.as_uuid()));
        assert!(ids.contains(&doc2.id.as_uuid()));
    }

    #[tokio::test]
    async fn test_sync_and_flush() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("Sync test");

        storage
            .insert(doc)
            .await
            .expect("Failed to insert document");

        // Sync should not fail
        storage.sync().await.expect("Failed to sync");

        // Flush should not fail
        storage.flush().await.expect("Failed to flush");
    }

    #[tokio::test]
    async fn test_close() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Close should not fail
        storage.close().await.expect("Failed to close storage");
    }

    #[tokio::test]
    async fn test_duplicate_insertion_fails() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("Duplicate test");

        // Insert document
        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Try to insert same document again - should fail
        let result = storage.insert(doc).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update_nonexistent_document_fails() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("Nonexistent update test");

        // Try to update document that doesn't exist - should fail
        let result = storage.update(doc).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_persistence_across_reopens() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_str().unwrap();
        let doc = create_test_document_no_tags("Persistence test"); // Use no tags

        {
            // First storage instance - insert document
            let mut storage1 = FileStorage::open(path)
                .await
                .expect("Failed to open storage");
            storage1
                .insert(doc.clone())
                .await
                .expect("Failed to insert document");
        } // storage1 is dropped here

        {
            // Second storage instance - should load existing document
            let storage2 = FileStorage::open(path)
                .await
                .expect("Failed to open storage");
            let retrieved = storage2.get(&doc.id).await.expect("Failed to get document");
            assert!(retrieved.is_some());

            let retrieved_doc = retrieved.unwrap();
            assert_eq!(retrieved_doc.id, doc.id);
            assert_eq!(retrieved_doc.content, doc.content);
        }
    }

    #[tokio::test]
    async fn test_frontmatter_handling() {
        let (mut storage, temp_dir) = create_test_storage().await;
        let mut doc = create_test_document("Content with tags");
        doc.tags = vec![
            ValidatedTag::new("rust").unwrap(),
            ValidatedTag::new("storage").unwrap(),
        ];

        // Insert document with tags
        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Check that file was written with frontmatter
        let file_path = storage.document_file_path(&doc.id.as_uuid());
        let file_content = fs::read_to_string(&file_path)
            .await
            .expect("Failed to read file");

        assert!(file_content.starts_with("---\n"));
        assert!(file_content.contains("tags:"));
        assert!(file_content.contains("- rust"));
        assert!(file_content.contains("- storage"));
        assert!(file_content.contains("Content with tags"));

        // Retrieve and verify tags are parsed correctly
        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        let retrieved_doc = retrieved.unwrap();
        assert_eq!(retrieved_doc.tags.len(), 2);
        assert!(retrieved_doc.tags.iter().any(|t| t.as_str() == "rust"));
        assert!(retrieved_doc.tags.iter().any(|t| t.as_str() == "storage"));
    }

    #[tokio::test]
    async fn test_hash_calculation_and_storage() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("Hash test content");

        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Access metadata to check hash
        let documents = storage.documents.read().await;
        let metadata = documents.get(&doc.id.as_uuid()).unwrap();

        // Hash should be calculated for the content
        let expected_hash = crate::pure::metadata::calculate_hash(&doc.content);
        assert_eq!(metadata.hash, expected_hash);
    }

    #[tokio::test]
    async fn test_metadata_serialization() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let doc = create_test_document("Metadata test");

        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        // Check that metadata file exists and is valid JSON
        let metadata_path = storage.metadata_file_path(&doc.id.as_uuid());
        assert!(metadata_path.exists());

        let metadata_content = fs::read_to_string(&metadata_path)
            .await
            .expect("Failed to read metadata file");

        // Should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&metadata_content).expect("Metadata file is not valid JSON");

        // Check key fields
        assert!(parsed.get("id").is_some());
        assert!(parsed.get("original_path").is_some());
        assert!(parsed.get("title").is_some());
        assert!(parsed.get("size").is_some());
        assert!(parsed.get("hash").is_some());
    }

    #[tokio::test]
    async fn test_invalid_path_rejection() {
        // Test path validation - these should fail due to directory traversal
        let result = FileStorage::open("../invalid/path").await;
        assert!(result.is_err());

        let result = FileStorage::open("/etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_file_storage_factory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_str().unwrap();

        // Test factory function with default cache
        let storage = create_file_storage(path, None)
            .await
            .expect("Failed to create storage");

        // Should be able to use Storage trait methods
        let docs = storage.list_all().await.expect("Failed to list documents");
        assert_eq!(docs.len(), 0);
    }

    #[tokio::test]
    async fn test_create_file_storage_factory_with_custom_cache() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_str().unwrap();

        // Test factory function with custom cache size
        let storage = create_file_storage(path, Some(500))
            .await
            .expect("Failed to create storage");

        // Should be able to use Storage trait methods
        let docs = storage.list_all().await.expect("Failed to list documents");
        assert_eq!(docs.len(), 0);
    }

    #[tokio::test]
    async fn test_document_paths_and_metadata_paths() {
        let (storage, _temp_dir) = create_test_storage().await;
        let doc_id = Uuid::new_v4();

        let doc_path = storage.document_file_path(&doc_id);
        let meta_path = storage.metadata_file_path(&doc_id);

        assert!(doc_path
            .to_string_lossy()
            .contains(&format!("{}.md", doc_id)));
        assert!(meta_path
            .to_string_lossy()
            .contains(&format!("{}.json", doc_id)));
        assert!(doc_path.parent().unwrap().ends_with("documents"));
        assert!(meta_path.parent().unwrap().ends_with("documents"));
    }

    #[tokio::test]
    async fn test_empty_content_document() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let mut doc = create_test_document_no_tags(""); // Use no tags version
        doc.content = Vec::new();
        doc.size = 0;

        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert empty document");

        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        assert!(retrieved.is_some());

        let retrieved_doc = retrieved.unwrap();
        assert_eq!(retrieved_doc.content.len(), 0);
        assert_eq!(retrieved_doc.size, 0);
    }

    #[tokio::test]
    async fn test_large_embedding_vector() {
        let (mut storage, _temp_dir) = create_test_storage().await;
        let mut doc = create_test_document_no_tags("Document with large embedding"); // Use no tags

        // Create a large embedding vector
        doc.embedding = Some((0..1536).map(|i| i as f32 * 0.001).collect());

        storage
            .insert(doc.clone())
            .await
            .expect("Failed to insert document");

        let retrieved = storage.get(&doc.id).await.expect("Failed to get document");
        let retrieved_doc = retrieved.unwrap();

        assert_eq!(retrieved_doc.embedding.as_ref().unwrap().len(), 1536);
        assert_eq!(retrieved_doc.embedding.as_ref().unwrap()[0], 0.0);
        // Use approximate comparison for floating point precision
        let last_val = retrieved_doc.embedding.as_ref().unwrap()[1535];
        assert!(
            (last_val - 1.535).abs() < 0.001,
            "Expected ~1.535, got {}",
            last_val
        );
    }
}
