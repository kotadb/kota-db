// Contracts Module - Stage 2: Contract-First Design
// Defines all contracts and interfaces for KotaDB components

pub mod performance;
pub mod optimization;

// Re-export key types for convenience
pub use performance::{
    PerformanceGuarantee,
    ComplexityContract, 
    MemoryContract,
    PerformanceMeasurement,
    ComplexityClass,
};

pub use optimization::{
    BulkOperations,
    ConcurrentAccess,
    TreeAnalysis,
    MemoryOptimization,
    OptimizationSLA,
    BulkOperationResult,
    ContentionMetrics,
    TreeStructureMetrics,
    BalanceInfo,
    MemoryUsage,
    BulkOperationType,
    SLAComplianceReport,
};

// Core domain contracts (re-exported from original contracts.rs)
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use crate::types::{ValidatedDocumentId, ValidatedPath, ValidatedTitle, ValidatedTag, ValidatedSearchQuery, ValidatedPageId, ValidatedLimit};

/// Core storage contract
pub trait Storage {
    async fn insert(&mut self, document: Document) -> Result<()>;
    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>>;
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool>;
    async fn list_all(&self) -> Result<Vec<Document>>;
    async fn flush(&mut self) -> Result<()>;
}

/// Core index contract
pub trait Index {
    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()>;
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool>;
    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>>;
    async fn flush(&mut self) -> Result<()>;
}

/// Document representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: ValidatedDocumentId,
    pub path: ValidatedPath,
    pub title: ValidatedTitle,
    pub content: Vec<u8>,
    pub tags: Vec<ValidatedTag>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub size: usize,
}

/// Query representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub search_terms: Vec<ValidatedSearchQuery>,
    pub tags: Vec<ValidatedTag>,
    pub path_pattern: Option<String>,
    pub limit: ValidatedLimit,
    pub offset: ValidatedPageId,
}

impl Query {
    pub fn new() -> Self {
        Self {
            search_terms: Vec::new(),
            tags: Vec::new(),
            path_pattern: None,
            limit: ValidatedLimit::new(10).unwrap(), // Safe default
            offset: ValidatedPageId::new(0).unwrap(), // Safe default
        }
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub total_documents: u64,
    pub total_size_bytes: u64,
    pub avg_document_size: f64,
    pub storage_efficiency: f64,
    pub fragmentation: f64,
}

/// Page identifier for pagination
pub type PageId = ValidatedPageId;

/// Transaction interface for ACID operations
pub trait Transaction {
    async fn commit(&mut self) -> Result<()>;
    async fn rollback(&mut self) -> Result<()>;
    fn is_active(&self) -> bool;
}

/// Metrics collection interface
pub trait MetricsCollector {
    fn record_operation(&self, operation: &str, duration: std::time::Duration);
    fn record_size(&self, metric: &str, size: u64);
    fn get_metrics(&self) -> HashMap<String, f64>;
}

/// Health check interface
pub trait HealthCheck {
    async fn health(&self) -> Result<HealthStatus>;
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Configuration interface
pub trait Configuration {
    fn get_config(&self) -> &DatabaseConfig;
    fn update_config(&mut self, config: DatabaseConfig) -> Result<()>;
}

/// Database configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub storage_path: PathBuf,
    pub max_file_size: u64,
    pub cache_size: usize,
    pub sync_interval: std::time::Duration,
    pub enable_compression: bool,
    pub enable_encryption: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./data"),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            cache_size: 1000,
            sync_interval: std::time::Duration::from_secs(5),
            enable_compression: false,
            enable_encryption: false,
        }
    }
}