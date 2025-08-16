//! Factory functions for creating production-ready components
//!
//! This module provides factory functions that return fully-wrapped components
//! with all production features enabled (tracing, validation, retries, caching).

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::contracts::Storage;
use crate::file_storage::create_file_storage;
use crate::symbol_storage::SymbolStorage;

/// Create a production-ready symbol storage with all wrappers
///
/// Returns a symbol storage instance wrapped with:
/// - Tracing for observability
/// - Validation for input safety
/// - Retry logic for resilience
/// - Caching for performance
///
/// # Arguments
/// * `data_dir` - Directory for storing data
/// * `cache_size` - Optional cache size (defaults to 1000)
pub async fn create_symbol_storage(
    data_dir: &str,
    cache_size: Option<usize>,
) -> Result<Arc<Mutex<SymbolStorage>>> {
    // Create base storage with all wrappers
    let storage = create_file_storage(data_dir, cache_size).await?;

    // Create symbol storage with wrapped storage
    let symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    Ok(Arc::new(Mutex::new(symbol_storage)))
}

/// Create a test symbol storage for unit tests
///
/// Returns a symbol storage backed by temporary directory storage
pub async fn create_test_symbol_storage() -> Result<Arc<Mutex<SymbolStorage>>> {
    // Use temporary directory for test storage
    let test_dir = format!("test_data/symbol_test_{}", Uuid::new_v4());
    tokio::fs::create_dir_all(&test_dir).await?;

    let storage = create_file_storage(&test_dir, Some(100)).await?;
    let symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    // Clean up will happen when test ends
    Ok(Arc::new(Mutex::new(symbol_storage)))
}

/// Create a symbol storage with custom underlying storage
///
/// Allows providing a custom storage implementation while still
/// getting the full symbol extraction and indexing capabilities
pub async fn create_symbol_storage_with_storage(
    storage: Box<dyn Storage + Send + Sync>,
) -> Result<Arc<Mutex<SymbolStorage>>> {
    let symbol_storage = SymbolStorage::new(storage).await?;
    Ok(Arc::new(Mutex::new(symbol_storage)))
}
