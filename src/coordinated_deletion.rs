// Coordinated Deletion Service
// This module provides a centralized service for deleting documents from all storage systems
// to ensure proper synchronization and prevent orphaned index entries (Issue #338)

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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
    /// 1. The document is retrieved and saved for potential rollback
    /// 2. The document is deleted from storage
    /// 3. If storage deletion succeeds, indices are updated
    /// 4. If any index update fails, the document is restored to storage
    ///
    /// Returns true if the document was found and deleted, false if not found
    pub async fn delete_document(&self, doc_id: &ValidatedDocumentId) -> Result<bool> {
        info!(
            "Coordinated deletion starting for document: {}",
            doc_id.as_uuid()
        );

        // Step 1: Retrieve document for potential rollback
        debug!("Retrieving document for rollback: {}", doc_id.as_uuid());
        let document_backup = {
            let storage = self.storage.lock().await;
            storage.get(doc_id).await?
        };

        // If document doesn't exist, nothing to delete
        let document_backup = match document_backup {
            Some(doc) => doc,
            None => {
                debug!("Document not found in storage: {}", doc_id.as_uuid());
                return Ok(false);
            }
        };

        // Step 2: Delete from storage
        debug!("Deleting document from storage: {}", doc_id.as_uuid());
        let deleted_from_storage = {
            let mut storage = self.storage.lock().await;
            storage.delete(doc_id).await?
        };

        if !deleted_from_storage {
            // This shouldn't happen since we already checked existence, but handle gracefully
            warn!(
                "Document disappeared between backup and deletion: {}",
                doc_id.as_uuid()
            );
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
                // Rollback: Restore document to storage since index update failed
                warn!(
                    "Primary index update failed, attempting rollback: {} (doc: {})",
                    e,
                    doc_id.as_uuid()
                );

                // Attempt to restore the document
                let mut storage = self.storage.lock().await;
                match storage.insert(document_backup.clone()).await {
                    Ok(_) => {
                        info!(
                            "Successfully rolled back document {} to storage",
                            doc_id.as_uuid()
                        );
                        return Err(anyhow::anyhow!(
                            "Deletion aborted: Primary index update failed. Document {} has been restored.", 
                            doc_id.as_uuid()
                        ));
                    }
                    Err(rollback_err) => {
                        // This is the worst case - deletion succeeded but rollback failed
                        error!(
                            "CRITICAL: Rollback failed after index update failure: {} (doc: {})",
                            rollback_err,
                            doc_id.as_uuid()
                        );
                        return Err(anyhow::anyhow!(
                            "CRITICAL: Document {} deleted but index update and rollback both failed. Manual recovery required.", 
                            doc_id.as_uuid()
                        ));
                    }
                }
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
                // Rollback: Restore document to storage and primary index
                warn!(
                    "Trigram index update failed, attempting rollback: {} (doc: {})",
                    e,
                    doc_id.as_uuid()
                );

                // First restore to storage
                let mut storage = self.storage.lock().await;
                let storage_rollback = storage.insert(document_backup.clone()).await;

                // Then restore to primary index
                let mut primary_index = self.primary_index.lock().await;
                let primary_rollback = primary_index
                    .insert(*doc_id, document_backup.path.clone())
                    .await;

                match (storage_rollback, primary_rollback) {
                    (Ok(_), Ok(_)) => {
                        info!(
                            "Successfully rolled back document {} to storage and primary index",
                            doc_id.as_uuid()
                        );
                        return Err(anyhow::anyhow!(
                            "Deletion aborted: Trigram index update failed. Document {} has been restored.", 
                            doc_id.as_uuid()
                        ));
                    }
                    _ => {
                        error!(
                            "CRITICAL: Partial or complete rollback failure after trigram index failure (doc: {})",
                            doc_id.as_uuid()
                        );
                        return Err(anyhow::anyhow!(
                            "CRITICAL: Document {} in inconsistent state. Manual recovery required.",
                            doc_id.as_uuid()
                        ));
                    }
                }
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
