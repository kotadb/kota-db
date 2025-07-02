// Contract-First Design - Stage 2
// This module defines all contracts (preconditions, postconditions, invariants)
// for the KotaDB system. These contracts serve as formal specifications
// that will be validated at runtime.

use anyhow::{ensure, Result};
use std::path::Path;
use uuid::Uuid;
use async_trait::async_trait;
use tracing::debug;
use crate::observability::*;

/// Core trait for storage operations with clear contracts
#[async_trait]
pub trait Storage: Send + Sync {
    /// Open a database at the specified path
    /// 
    /// # Preconditions
    /// - `path` must be a valid directory path
    /// - Process must have read/write permissions
    /// - Path length must be < 4096 bytes (filesystem limit)
    /// 
    /// # Postconditions
    /// - Database directories are created if they don't exist
    /// - WAL is initialized and ready for writes
    /// - All indices are loaded into memory
    /// - Returns error if preconditions not met
    async fn open(path: &str) -> Result<Self> where Self: Sized;
    
    /// Insert a new document
    /// 
    /// # Preconditions
    /// - Document ID must be unique (not already exist)
    /// - Document path must be non-empty and valid
    /// - Document size must be > 0
    /// - Hash must be exactly 32 bytes (SHA-256)
    /// 
    /// # Postconditions
    /// - Document is persisted to storage
    /// - All indices are updated atomically
    /// - WAL entry is written before confirmation
    /// - Operation is idempotent if document unchanged
    /// 
    /// # Invariants
    /// - Total document count increases by 1
    /// - Storage size increases by document size
    async fn insert(&mut self, doc: Document) -> Result<()>;
    
    /// Retrieve a document by ID
    /// 
    /// # Preconditions
    /// - ID must be a valid UUID
    /// 
    /// # Postconditions
    /// - Returns Some(doc) if document exists
    /// - Returns None if document doesn't exist
    /// - Does not modify any state
    /// - Concurrent reads are safe
    async fn get(&self, id: &Uuid) -> Result<Option<Document>>;
    
    /// Update an existing document
    /// 
    /// # Preconditions
    /// - Document must already exist
    /// - New document ID must match existing ID
    /// - Updated timestamp must be >= created timestamp
    /// 
    /// # Postconditions
    /// - Document is updated atomically
    /// - Old version is preserved until transaction commits
    /// - Indices reflect new content
    /// - WAL contains update record
    async fn update(&mut self, doc: Document) -> Result<()>;
    
    /// Delete a document
    /// 
    /// # Preconditions
    /// - Document ID must exist (for success)
    /// 
    /// # Postconditions
    /// - Document is marked for deletion
    /// - All index entries are removed
    /// - Space is marked for reclamation
    /// - Operation succeeds even if document doesn't exist
    /// 
    /// # Invariants
    /// - Document count decreases by 1 (if existed)
    async fn delete(&mut self, id: &Uuid) -> Result<()>;
    
    /// Force sync to disk
    /// 
    /// # Preconditions
    /// - Storage must be open and valid
    /// 
    /// # Postconditions
    /// - All pending writes are flushed to disk
    /// - WAL is checkpointed
    /// - Indices are persisted
    /// - Data is crash-safe after return
    async fn sync(&mut self) -> Result<()>;
    
    /// Close the storage engine
    /// 
    /// # Preconditions
    /// - No active transactions
    /// 
    /// # Postconditions
    /// - All resources are released
    /// - Files are properly closed
    /// - Cannot use storage after close
    async fn close(self) -> Result<()>;
}

/// Core trait for index operations
#[async_trait]
pub trait Index: Send + Sync {
    type Key: Send + Sync;
    type Value: Send + Sync;
    
    /// Insert a key-value pair
    /// 
    /// # Preconditions
    /// - Key must be valid for index type
    /// - Value must be non-null
    /// 
    /// # Postconditions
    /// - Entry is searchable immediately
    /// - Previous value (if any) is overwritten
    /// - Index remains balanced/optimized
    async fn insert(&mut self, key: Self::Key, value: Self::Value) -> Result<()>;
    
    /// Remove a key
    /// 
    /// # Preconditions
    /// - Key must be valid format
    /// 
    /// # Postconditions
    /// - Key no longer appears in searches
    /// - Space is reclaimed (eventually)
    /// - Returns success even if key didn't exist
    async fn delete(&mut self, key: &Self::Key) -> Result<()>;
    
    /// Search the index
    /// 
    /// # Preconditions
    /// - Query must be valid for index type
    /// 
    /// # Postconditions
    /// - Results are sorted by relevance
    /// - All matching entries are returned
    /// - Empty result for no matches
    async fn search(&self, query: &Query) -> Result<Vec<Self::Value>>;
    
    /// Flush index to disk
    /// 
    /// # Preconditions
    /// - Index must be valid
    /// 
    /// # Postconditions
    /// - All changes are persisted
    /// - Index can be recovered after crash
    async fn flush(&mut self) -> Result<()>;
}

/// Document structure with validation
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub path: String,
    pub hash: [u8; 32],
    pub size: u64,
    pub created: i64,
    pub updated: i64,
    pub title: String,
    pub word_count: u32,
}

impl Document {
    /// Create a new document with validation
    /// 
    /// # Contract
    /// - Path must be non-empty and < 4096 bytes
    /// - Size must be > 0
    /// - Updated >= created
    /// - Title must be non-empty
    pub fn new(
        id: Uuid,
        path: String,
        hash: [u8; 32],
        size: u64,
        created: i64,
        updated: i64,
        title: String,
        word_count: u32,
    ) -> Result<Self> {
        // Validate preconditions
        ensure!(!path.is_empty(), "Document path cannot be empty");
        ensure!(path.len() < 4096, "Document path too long");
        ensure!(size > 0, "Document size must be positive");
        ensure!(updated >= created, "Updated time must be >= created time");
        ensure!(!title.is_empty(), "Document title cannot be empty");
        
        Ok(Self {
            id,
            path,
            hash,
            size,
            created,
            updated,
            title,
            word_count,
        })
    }
}

/// Query types with validation
#[derive(Debug)]
pub struct Query {
    pub text: Option<String>,
    pub tags: Option<Vec<String>>,
    pub date_range: Option<(i64, i64)>,
    pub limit: usize,
}

impl Query {
    /// Create a new query with validation
    /// 
    /// # Contract
    /// - At least one search criterion must be specified
    /// - Limit must be > 0 and <= 1000
    /// - Date range start must be <= end
    pub fn new(
        text: Option<String>,
        tags: Option<Vec<String>>,
        date_range: Option<(i64, i64)>,
        limit: usize,
    ) -> Result<Self> {
        // Validate preconditions
        ensure!(
            text.is_some() || tags.is_some() || date_range.is_some(),
            "Query must have at least one search criterion"
        );
        ensure!(limit > 0 && limit <= 1000, "Query limit must be between 1 and 1000");
        
        if let Some((start, end)) = date_range {
            ensure!(start <= end, "Date range start must be <= end");
        }
        
        Ok(Self {
            text,
            tags,
            date_range,
            limit,
        })
    }
}

/// Storage metrics with invariants
#[derive(Debug)]
pub struct StorageMetrics {
    pub document_count: usize,
    pub total_size_bytes: u64,
    pub index_sizes: std::collections::HashMap<String, usize>,
    
    // Invariants:
    // - document_count >= 0
    // - total_size_bytes >= document_count (at least 1 byte per doc)
    // - sum of index sizes <= total storage usage
}

impl StorageMetrics {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.total_size_bytes >= self.document_count as u64,
            "Invalid metrics: size < count"
        );
        Ok(())
    }
}

/// Page allocation contracts
pub struct PageId(pub u64);

impl PageId {
    /// Create a new page ID with validation
    /// 
    /// # Contract
    /// - ID must be > 0 (0 is reserved for null)
    /// - ID must be < MAX_PAGES
    pub fn new(id: u64) -> Result<Self> {
        ensure!(id > 0, "Page ID must be positive");
        ensure!(id < u64::MAX / 4096, "Page ID too large");
        Ok(Self(id))
    }
}

/// Transaction contracts
#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    pub operations: Vec<Operation>,
}

impl Transaction {
    /// Begin a new transaction
    /// 
    /// # Contract
    /// - Transaction ID must be unique
    /// - No other transaction can be active
    /// 
    /// # Postconditions
    /// - Transaction is registered in WAL
    /// - All operations are isolated
    pub fn begin(id: u64) -> Result<Self> {
        ensure!(id > 0, "Transaction ID must be positive");
        Ok(Self {
            id,
            operations: Vec::new(),
        })
    }
    
    /// Commit the transaction
    /// 
    /// # Preconditions
    /// - All operations must be valid
    /// - No conflicts with other transactions
    /// 
    /// # Postconditions
    /// - All changes are durable
    /// - Transaction cannot be rolled back
    pub async fn commit(self) -> Result<()> {
        // Validate all operations
        for op in &self.operations {
            op.validate()?;
        }
        // Implementation will handle actual commit
        Ok(())
    }
}

/// Runtime contract validation
pub mod validation {
    use super::*;
    
    /// Validate storage path
    pub fn validate_storage_path(path: &str) -> Result<()> {
        ensure!(!path.is_empty(), "Storage path cannot be empty");
        ensure!(path.len() < 4096, "Storage path too long");
        
        let path = Path::new(path);
        ensure!(
            path.is_absolute() || path.is_relative(),
            "Invalid path format"
        );
        
        Ok(())
    }
    
    /// Validate document invariants
    pub fn validate_document(doc: &Document) -> Result<()> {
        ensure!(doc.size > 0, "Document size must be positive");
        ensure!(doc.updated >= doc.created, "Invalid timestamps");
        ensure!(!doc.path.is_empty(), "Document path required");
        ensure!(!doc.title.is_empty(), "Document title required");
        Ok(())
    }
    
    /// Validate index operation
    pub fn validate_index_op<K, V>(key: &K, value: &V) -> Result<()> 
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        // Generic validation - specific indices will add more
        debug!("Validating index operation: {:?} -> {:?}", key, value);
        Ok(())
    }
}

/// Contract enforcement middleware
pub struct ContractEnforcer<T> {
    inner: T,
}

impl<T> ContractEnforcer<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
    
    /// Wrap a function call with contract validation
    pub async fn call_with_validation<F, R>(
        &self,
        operation: &str,
        preconditions: impl FnOnce() -> Result<()>,
        f: F,
        postconditions: impl FnOnce(&R) -> Result<()>,
    ) -> Result<R>
    where
        F: std::future::Future<Output = Result<R>>,
    {
        // Create operation context
        let mut ctx = OperationContext::new(operation);
        
        // Validate preconditions
        if let Err(e) = preconditions() {
            log_error_with_context(&e, &ctx);
            return Err(e);
        }
        
        // Execute operation
        let result = f.await;
        
        // Validate postconditions if successful
        if let Ok(ref value) = result {
            if let Err(e) = postconditions(value) {
                ctx.add_attribute("postcondition_failed", "true");
                log_error_with_context(&e, &ctx);
                return Err(e);
            }
        }
        
        // Log operation result
        log_operation(
            &ctx,
            &Operation::StorageRead { 
                doc_id: Uuid::new_v4(), 
                size_bytes: 0 
            },
            &result.as_ref().map(|_| ()).map_err(|e| anyhow::anyhow!("{}", e)),
        );
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_validation() {
        // Valid document
        let doc = Document::new(
            Uuid::new_v4(),
            "/test/path.md".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "Test Doc".to_string(),
            100,
        );
        assert!(doc.is_ok());
        
        // Invalid: empty path
        let doc = Document::new(
            Uuid::new_v4(),
            "".to_string(),
            [0u8; 32],
            1024,
            1000,
            2000,
            "Test Doc".to_string(),
            100,
        );
        assert!(doc.is_err());
        
        // Invalid: updated < created
        let doc = Document::new(
            Uuid::new_v4(),
            "/test/path.md".to_string(),
            [0u8; 32],
            1024,
            2000,
            1000,
            "Test Doc".to_string(),
            100,
        );
        assert!(doc.is_err());
    }
    
    #[test]
    fn test_query_validation() {
        // Valid query with text
        let query = Query::new(
            Some("test".to_string()),
            None,
            None,
            10,
        );
        assert!(query.is_ok());
        
        // Invalid: no criteria
        let query = Query::new(None, None, None, 10);
        assert!(query.is_err());
        
        // Invalid: limit too high
        let query = Query::new(
            Some("test".to_string()),
            None,
            None,
            10000,
        );
        assert!(query.is_err());
    }
}