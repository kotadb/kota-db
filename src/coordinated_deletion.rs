// Coordinated Deletion Service
// This module provides a centralized service for deleting documents from all storage systems
// to ensure proper synchronization and prevent orphaned index entries (Issue #338)

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::contracts::{Index, Storage};
use crate::types::ValidatedDocumentId;

/// Service that coordinates deletion across storage and indices
///
/// This service ensures that when a document is deleted, it is removed from:
/// 1. The primary storage system
/// 2. The primary index (B+ tree for path lookups)  
/// 3. The trigram index (for full-text search)
///
/// This prevents the critical synchronization bug reported in issue #338.
pub struct CoordinatedDeletionService {
    storage: Arc<Mutex<Box<dyn Storage>>>,
    primary_index: Arc<Mutex<Box<dyn Index>>>,
    trigram_index: Arc<Mutex<Box<dyn Index>>>,
}

impl CoordinatedDeletionService {
    /// Create a new coordinated deletion service
    pub fn new(
        storage: Arc<Mutex<Box<dyn Storage>>>,
        primary_index: Arc<Mutex<Box<dyn Index>>>,
        trigram_index: Arc<Mutex<Box<dyn Index>>>,
    ) -> Self {
        Self {
            storage,
            primary_index,
            trigram_index,
        }
    }

    /// Delete a document from all storage systems in a coordinated manner
    ///
    /// This method ensures that:
    /// 1. The document is deleted from storage first
    /// 2. If storage deletion succeeds, indices are updated
    /// 3. If any index update fails, it's treated as a critical error
    ///
    /// Returns true if the document was found and deleted, false if not found
    pub async fn delete_document(&self, doc_id: &ValidatedDocumentId) -> Result<bool> {
        info!(
            "Coordinated deletion starting for document: {}",
            doc_id.as_uuid()
        );

        // Step 1: Delete from storage first
        debug!("Deleting document from storage: {}", doc_id.as_uuid());
        let deleted_from_storage = {
            let mut storage = self.storage.lock().await;
            storage.delete(doc_id).await?
        };

        if !deleted_from_storage {
            debug!("Document not found in storage: {}", doc_id.as_uuid());
            return Ok(false);
        }

        info!(
            "Document deleted from storage, updating indices: {}",
            doc_id.as_uuid()
        );

        // Step 2: Update primary index
        debug!("Deleting document from primary index: {}", doc_id.as_uuid());
        let primary_result = {
            let mut primary_index = self.primary_index.lock().await;
            primary_index.delete(doc_id).await
        };

        match primary_result {
            Ok(primary_deleted) => {
                if primary_deleted {
                    debug!(
                        "Successfully removed document from primary index: {}",
                        doc_id.as_uuid()
                    );
                } else {
                    warn!(
                        "Document was not found in primary index: {}",
                        doc_id.as_uuid()
                    );
                }
            }
            Err(e) => {
                // This is critical - storage was deleted but index update failed
                warn!(
                    "CRITICAL: Primary index update failed after storage deletion: {} (doc: {})",
                    e,
                    doc_id.as_uuid()
                );
                return Err(anyhow::anyhow!(
                    "Index synchronization failure: primary index update failed after storage deletion: {}", e
                ));
            }
        }

        // Step 3: Update trigram index
        debug!("Deleting document from trigram index: {}", doc_id.as_uuid());
        let trigram_result = {
            let mut trigram_index = self.trigram_index.lock().await;
            trigram_index.delete(doc_id).await
        };

        match trigram_result {
            Ok(trigram_deleted) => {
                if trigram_deleted {
                    debug!(
                        "Successfully removed document from trigram index: {}",
                        doc_id.as_uuid()
                    );
                } else {
                    warn!(
                        "Document was not found in trigram index: {}",
                        doc_id.as_uuid()
                    );
                }
            }
            Err(e) => {
                // This is critical - storage and primary were deleted but trigram update failed
                warn!(
                    "CRITICAL: Trigram index update failed after storage deletion: {} (doc: {})",
                    e,
                    doc_id.as_uuid()
                );
                return Err(anyhow::anyhow!(
                    "Index synchronization failure: trigram index update failed after storage deletion: {}", e
                ));
            }
        }

        info!(
            "Coordinated deletion completed successfully for document: {}",
            doc_id.as_uuid()
        );
        Ok(true)
    }

    /// Get a shared reference to the storage for read operations
    ///
    /// This should ONLY be used for read operations (get, list, search).
    /// For deletion, always use delete_document() to ensure coordination.
    pub fn get_storage(&self) -> Arc<Mutex<Box<dyn Storage>>> {
        Arc::clone(&self.storage)
    }

    /// Get a shared reference to the primary index for read operations
    pub fn get_primary_index(&self) -> Arc<Mutex<Box<dyn Index>>> {
        Arc::clone(&self.primary_index)
    }

    /// Get a shared reference to the trigram index for read operations  
    pub fn get_trigram_index(&self) -> Arc<Mutex<Box<dyn Index>>> {
        Arc::clone(&self.trigram_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder,
        ValidatedPath,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_coordinated_deletion_service() -> Result<()> {
        // Setup test environment
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        // Create storage and indices
        let storage = Arc::new(Mutex::new(Box::new(
            create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Storage>));

        let primary_index = Arc::new(Mutex::new(Box::new(
            create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Index>));

        let trigram_index = Arc::new(Mutex::new(Box::new(
            create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Index>));

        // Create coordinated deletion service
        let deletion_service = CoordinatedDeletionService::new(
            Arc::clone(&storage),
            Arc::clone(&primary_index),
            Arc::clone(&trigram_index),
        );

        // Insert a test document
        let doc = DocumentBuilder::new()
            .path("test.md")?
            .title("Test Document")?
            .content(b"Test content")
            .build()?;

        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(doc.path.to_string())?;
        let content = doc.content.clone();

        // Insert into all systems
        storage.lock().await.insert(doc).await?;
        primary_index
            .lock()
            .await
            .insert(doc_id, doc_path.clone())
            .await?;
        trigram_index
            .lock()
            .await
            .insert_with_content(doc_id, doc_path, &content)
            .await?;

        // Verify document exists in all systems
        assert!(storage.lock().await.get(&doc_id).await?.is_some());

        // Use coordinated deletion
        let deleted = deletion_service.delete_document(&doc_id).await?;
        assert!(deleted, "Document should have been deleted");

        // Verify document is gone from all systems
        assert!(storage.lock().await.get(&doc_id).await?.is_none());

        // Note: We can't easily verify index deletion without more complex test setup,
        // but the coordinated deletion service ensures they're called

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_nonexistent_document() -> Result<()> {
        // Setup minimal test environment
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        let storage = Arc::new(Mutex::new(Box::new(
            create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Storage>));

        let primary_index = Arc::new(Mutex::new(Box::new(
            create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Index>));

        let trigram_index = Arc::new(Mutex::new(Box::new(
            create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?,
        ) as Box<dyn Index>));

        let deletion_service =
            CoordinatedDeletionService::new(storage, primary_index, trigram_index);

        // Try to delete non-existent document
        let fake_doc_id = crate::types::ValidatedDocumentId::new();
        let deleted = deletion_service.delete_document(&fake_doc_id).await?;

        assert!(!deleted, "Non-existent document should return false");

        Ok(())
    }
}
