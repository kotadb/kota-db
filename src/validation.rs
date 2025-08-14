// Validation Layer - Stage 2: Contract Enforcement
// This module provides runtime validation of all contracts
// ensuring that preconditions and postconditions are met

use crate::contracts::*;
use crate::types::ValidatedDocumentId;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;
use tracing::error;

/// Validation errors with detailed context
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Precondition failed: {condition}")]
    PreconditionFailed { condition: String, context: String },

    #[error("Postcondition failed: {condition}")]
    PostconditionFailed { condition: String, context: String },

    #[error("Invariant violated: {invariant}")]
    InvariantViolated { invariant: String, state: String },

    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },
}

/// Validation context for better error messages
#[derive(Clone)]
pub struct ValidationContext {
    operation: String,
    attributes: HashMap<String, String>,
}

impl ValidationContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn validate(self, condition: bool, message: &str) -> Result<()> {
        if !condition {
            let context = format!(
                "Operation: {}, Attributes: {:?}",
                self.operation, self.attributes
            );
            bail!(ValidationError::PreconditionFailed {
                condition: message.to_string(),
                context,
            });
        }
        Ok(())
    }
}

/// Path validation with detailed checks
pub mod path {
    use super::*;
    use std::ffi::OsStr;

    /// Maximum path length across platforms
    const MAX_PATH_LENGTH: usize = 4096;

    /// Reserved filenames on Windows
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    /// Validate a file path for storage
    pub fn validate_file_path(path: &str) -> Result<()> {
        let ctx = ValidationContext::new("validate_file_path").with_attribute("path", path);

        // Check empty
        ctx.clone()
            .validate(!path.is_empty(), "Path cannot be empty")?;

        // Check length
        ctx.clone().validate(
            path.len() < MAX_PATH_LENGTH,
            &format!("Path exceeds maximum length of {MAX_PATH_LENGTH}"),
        )?;

        // Check for null bytes
        ctx.clone()
            .validate(!path.contains('\0'), "Path contains null bytes")?;

        let path_obj = Path::new(path);

        // Check for directory traversal attempts
        for component in path_obj.components() {
            if let std::path::Component::ParentDir = component {
                bail!(ValidationError::InvalidInput {
                    field: "path".to_string(),
                    reason: "Parent directory references (..) not allowed".to_string(),
                });
            }
        }

        // Check for reserved names (Windows compatibility)
        if let Some(stem) = path_obj.file_stem().and_then(OsStr::to_str) {
            let upper = stem.to_uppercase();
            if RESERVED_NAMES.contains(&upper.as_str()) {
                bail!(ValidationError::InvalidInput {
                    field: "path".to_string(),
                    reason: format!("Reserved filename: {stem}"),
                });
            }
        }

        // Check for valid UTF-8
        if path_obj.to_str().is_none() {
            bail!(ValidationError::InvalidInput {
                field: "path".to_string(),
                reason: "Path is not valid UTF-8".to_string(),
            });
        }

        Ok(())
    }

    /// Validate a directory path
    pub fn validate_directory_path(path: &str) -> Result<()> {
        validate_file_path(path)?;

        // Additional directory-specific checks
        let path_obj = Path::new(path);

        // Ensure it's not a file with extension
        if path_obj.extension().is_some() {
            bail!(ValidationError::InvalidInput {
                field: "path".to_string(),
                reason: "Directory path should not have file extension".to_string(),
            });
        }

        Ok(())
    }
}

/// Document validation
pub mod document {
    use super::*;

    /// Validate document for insertion
    pub fn validate_for_insert(
        doc: &Document,
        existing_ids: &std::collections::HashSet<ValidatedDocumentId>,
    ) -> Result<()> {
        let ctx = ValidationContext::new("document_insert")
            .with_attribute("doc_id", doc.id.to_string())
            .with_attribute("path", doc.path.as_str());

        // Check ID uniqueness
        ctx.clone().validate(
            !existing_ids.contains(&doc.id),
            "Document ID already exists",
        )?;

        // Validate path
        path::validate_file_path(doc.path.as_str())?;

        // Size validation (empty content is allowed)
        // Note: doc.size is usize so it can't be negative, removed size > 0 check to allow empty content

        ctx.clone().validate(
            doc.size < 100 * 1024 * 1024, // 100MB limit
            "Document size exceeds maximum (100MB)",
        )?;

        // Check timestamps
        ctx.clone().validate(
            doc.created_at.timestamp() > 0,
            "Created timestamp must be positive",
        )?;

        ctx.clone().validate(
            doc.updated_at >= doc.created_at,
            "Updated timestamp must be >= created",
        )?;

        // Check title (ValidatedTitle is already validated, just check it's reasonable)
        ctx.clone().validate(
            !doc.title.as_str().is_empty(),
            "Document title cannot be empty",
        )?;

        ctx.validate(
            doc.title.as_str().len() < 1024,
            "Document title too long (max 1024 chars)",
        )?;

        Ok(())
    }

    /// Validate document for update
    pub fn validate_for_update(new_doc: &Document, old_doc: &Document) -> Result<()> {
        let ctx = ValidationContext::new("document_update")
            .with_attribute("doc_id", new_doc.id.to_string());

        // IDs must match
        ctx.clone().validate(
            new_doc.id == old_doc.id,
            "Document ID cannot change during update",
        )?;

        // Updated timestamp must increase
        ctx.clone().validate(
            new_doc.updated_at > old_doc.updated_at,
            "Updated timestamp must increase",
        )?;

        // Created timestamp must not change
        ctx.validate(
            new_doc.created_at == old_doc.created_at,
            "Created timestamp cannot change",
        )?;

        // Validate other fields
        path::validate_file_path(new_doc.path.as_str())?;

        Ok(())
    }
}

/// Index validation
pub mod index {
    use super::*;

    /// Validate trigram extraction
    pub fn validate_trigram(text: &str) -> Result<()> {
        let ctx = ValidationContext::new("trigram_extraction")
            .with_attribute("text_length", text.len().to_string());

        ctx.clone().validate(
            text.len() >= 3,
            "Text too short for trigram extraction (min 3 chars)",
        )?;

        ctx.validate(
            text.len() < 1024 * 1024, // 1MB limit
            "Text too long for trigram extraction (max 1MB)",
        )?;

        Ok(())
    }

    /// Validate search query
    pub fn validate_search_query(query: &str) -> Result<()> {
        let ctx = ValidationContext::new("search_query").with_attribute("query", query);

        ctx.clone()
            .validate(!query.trim().is_empty(), "Search query cannot be empty")?;

        ctx.validate(query.len() < 1024, "Search query too long (max 1024 chars)")?;

        Ok(())
    }

    /// Validate tag
    pub fn validate_tag(tag: &str) -> Result<()> {
        let ctx = ValidationContext::new("tag_validation").with_attribute("tag", tag);

        ctx.clone()
            .validate(!tag.trim().is_empty(), "Tag cannot be empty")?;

        ctx.clone()
            .validate(tag.len() < 128, "Tag too long (max 128 chars)")?;

        // Check for valid characters (alphanumeric, dash, underscore)
        let valid_chars = tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ');

        ctx.validate(valid_chars, "Tag contains invalid characters")?;

        Ok(())
    }
}

/// Transaction validation
pub mod transaction {
    use super::*;

    /// Active transactions tracking
    use std::sync::LazyLock;
    static ACTIVE_TRANSACTIONS: LazyLock<std::sync::Mutex<std::collections::HashSet<u64>>> =
        LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

    /// Validate transaction begin
    pub fn validate_begin(tx_id: u64) -> Result<()> {
        let ctx =
            ValidationContext::new("transaction_begin").with_attribute("tx_id", tx_id.to_string());

        ctx.clone()
            .validate(tx_id > 0, "Transaction ID must be positive")?;

        let mut active = ACTIVE_TRANSACTIONS
            .lock()
            .map_err(|e| anyhow::anyhow!("Transaction lock poisoned: {}", e))?;
        ctx.validate(!active.contains(&tx_id), "Transaction ID already active")?;

        active.insert(tx_id);
        Ok(())
    }

    /// Validate transaction commit
    pub fn validate_commit(tx_id: u64) -> Result<()> {
        let ctx =
            ValidationContext::new("transaction_commit").with_attribute("tx_id", tx_id.to_string());

        let mut active = ACTIVE_TRANSACTIONS
            .lock()
            .map_err(|e| anyhow::anyhow!("Transaction lock poisoned: {}", e))?;
        ctx.validate(active.contains(&tx_id), "Transaction not active")?;

        active.remove(&tx_id);
        Ok(())
    }
}

/// Storage state validation
pub mod storage {
    use super::*;

    /// Validate storage metrics consistency
    pub fn validate_metrics(metrics: &StorageMetrics) -> Result<()> {
        let ctx = ValidationContext::new("storage_metrics")
            .with_attribute("doc_count", metrics.total_documents.to_string())
            .with_attribute("total_size", metrics.total_size_bytes.to_string());

        // Basic sanity checks
        ctx.clone().validate(
            metrics.total_size_bytes >= metrics.total_documents,
            "Total size less than document count",
        )?;

        // If no documents, size should be near zero
        if metrics.total_documents == 0 {
            ctx.clone().validate(
                metrics.total_size_bytes < 1024, // Allow some metadata
                "Non-zero size with zero documents",
            )?;
        }

        // Check average document size is reasonable
        if metrics.total_documents > 0 {
            ctx.validate(
                metrics.avg_document_size > 0.0 && metrics.avg_document_size < 100_000_000.0, // 100MB max
                "Average document size out of reasonable bounds",
            )?;
        }

        Ok(())
    }

    /// Validate page allocation
    pub fn validate_page_id(id: u64) -> Result<()> {
        let ctx =
            ValidationContext::new("page_allocation").with_attribute("page_id", id.to_string());

        ctx.clone().validate(id > 0, "Page ID must be positive")?;

        ctx.validate(id < u64::MAX / 4096, "Page ID too large")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ValidatedPath, ValidatedTitle};
    use uuid::Uuid;

    #[test]
    fn test_path_validation() {
        // Valid paths
        assert!(path::validate_file_path("/test/file.md").is_ok());
        assert!(path::validate_file_path("relative/path.txt").is_ok());

        // Invalid paths
        assert!(path::validate_file_path("").is_err());
        assert!(path::validate_file_path("../../../etc/passwd").is_err());
        assert!(path::validate_file_path("file\0with\0nulls").is_err());
        assert!(path::validate_file_path("CON.txt").is_err()); // Windows reserved

        // Path too long
        let long_path = "x".repeat(5000);
        assert!(path::validate_file_path(&long_path).is_err());
    }

    #[test]
    fn test_document_validation() {
        let mut existing_ids = std::collections::HashSet::new();
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).expect("UUID should be valid");

        let valid_doc = Document {
            id: doc_id,
            path: ValidatedPath::new("/test/doc.md").expect("Test path should be valid"),
            title: ValidatedTitle::new("Test Doc").expect("Test title should be valid"),
            content: vec![0u8; 1024],
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            size: 1024,
            embedding: None,
        };

        // Should validate successfully
        assert!(document::validate_for_insert(&valid_doc, &existing_ids).is_ok());

        // Add ID to existing set
        existing_ids.insert(doc_id);

        // Should fail with duplicate ID
        assert!(document::validate_for_insert(&valid_doc, &existing_ids).is_err());

        // Invalid documents (test with size exceeding limit)
        let mut invalid = valid_doc.clone();
        invalid.size = 200 * 1024 * 1024; // 200MB, exceeds 100MB limit
        assert!(
            document::validate_for_insert(&invalid, &std::collections::HashSet::new()).is_err()
        );

        invalid = valid_doc.clone();
        invalid.updated_at =
            chrono::DateTime::from_timestamp(500, 0).expect("Test timestamp should be valid"); // Before created
        assert!(
            document::validate_for_insert(&invalid, &std::collections::HashSet::new()).is_err()
        );
    }

    #[test]
    fn test_empty_content_validation() {
        let existing_ids = std::collections::HashSet::new();
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).expect("UUID should be valid");

        // Document with empty content should be valid
        let empty_content_doc = Document {
            id: doc_id,
            path: ValidatedPath::new("/test/empty.md").expect("Test path should be valid"),
            title: ValidatedTitle::new("Empty Document").expect("Test title should be valid"),
            content: vec![], // Empty content
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            size: 0, // Size of empty content
            embedding: None,
        };

        // Should validate successfully (empty content is allowed)
        assert!(document::validate_for_insert(&empty_content_doc, &existing_ids).is_ok());
    }

    #[test]
    fn test_tag_validation() {
        // Valid tags
        assert!(index::validate_tag("rust").is_ok());
        assert!(index::validate_tag("rust-lang").is_ok());
        assert!(index::validate_tag("rust_programming").is_ok());
        assert!(index::validate_tag("Rust 2024").is_ok());

        // Invalid tags
        assert!(index::validate_tag("").is_err());
        assert!(index::validate_tag("   ").is_err());
        assert!(index::validate_tag("x".repeat(200).as_str()).is_err());
        assert!(index::validate_tag("tag@with#special$chars").is_err());
    }
}
