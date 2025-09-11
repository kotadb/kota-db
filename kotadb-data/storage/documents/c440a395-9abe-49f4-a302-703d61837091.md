---
tags:
- file
- kota-db
- ext_rs
---
//! Memory management and resource monitoring for KotaDB operations
//!
//! This module provides utilities for tracking memory usage, enforcing limits,
//! and managing resource consumption during large repository ingestion operations.

use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Memory usage tracking and limits
#[derive(Debug, Clone)]
pub struct MemoryManager {
    /// Maximum total memory usage allowed (in bytes)
    max_memory: u64,
    /// Current memory usage estimate (in bytes)
    current_usage: Arc<AtomicU64>,
    /// Enable memory monitoring
    enabled: bool,
}

impl MemoryManager {
    /// Create a new memory manager with specified limits
    pub fn new(max_memory_mb: Option<u64>) -> Self {
        let (max_memory, enabled) = match max_memory_mb {
            Some(mb) => (mb * 1024 * 1024, true),
            None => (0, false), // No limits when None
        };

        info!(
            "Memory manager initialized: {} (limit: {}MB)",
            if enabled { "enabled" } else { "disabled" },
            max_memory / (1024 * 1024)
        );

        Self {
            max_memory,
            current_usage: Arc::new(AtomicU64::new(0)),
            enabled,
        }
    }

    /// Check if we can allocate the specified amount of memory
    pub fn can_allocate(&self, size: u64) -> bool {
        if !self.enabled {
            return true; // No limits when disabled
        }

        let current = self.current_usage.load(Ordering::Relaxed);
        let would_use = current + size;

        let can_allocate = would_use <= self.max_memory;

        if !can_allocate {
            warn!(
                "Memory allocation would exceed limit: current={}MB, requesting={}MB, limit={}MB",
                current / (1024 * 1024),
                size / (1024 * 1024),
                self.max_memory / (1024 * 1024)
            );
        }

        can_allocate
    }

    /// Reserve memory for an operation
    pub fn reserve(&self, size: u64) -> Result<MemoryReservation> {
        if !self.can_allocate(size) {
            return Err(anyhow::anyhow!(
                "Cannot allocate {}MB: would exceed memory limit of {}MB (current: {}MB)",
                size / (1024 * 1024),
                self.max_memory / (1024 * 1024),
                self.current_usage.load(Ordering::Relaxed) / (1024 * 1024)
            ));
        }

        self.current_usage.fetch_add(size, Ordering::Relaxed);

        debug!(
            "Reserved {}MB memory (total: {}MB/{}MB)",
            size / (1024 * 1024),
            self.current_usage.load(Ordering::Relaxed) / (1024 * 1024),
            self.max_memory / (1024 * 1024)
        );

        Ok(MemoryReservation {
            size,
            manager: self.current_usage.clone(),
        })
    }

    /// Get current memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let current = self.current_usage.load(Ordering::Relaxed);
        MemoryStats {
            current_usage_mb: current / (1024 * 1024),
            max_memory_mb: if self.enabled {
                Some(self.max_memory / (1024 * 1024))
            } else {
                None
            },
            utilization_percent: if self.enabled && self.max_memory > 0 {
                Some((current as f64 / self.max_memory as f64 * 100.0) as u8)
            } else {
                None
            },
            enabled: self.enabled,
        }
    }

    /// Check if we're approaching memory limits (>80% usage)
    pub fn is_memory_pressure(&self) -> bool {
        if !self.enabled {
            return false;
        }

        let current = self.current_usage.load(Ordering::Relaxed);
        let usage_percent = current as f64 / self.max_memory as f64;
        usage_percent > 0.8
    }

    /// Estimate memory usage for a file entry
    pub fn estimate_file_memory(&self, file_size: usize, include_parsing: bool) -> u64 {
        let base_overhead = 200; // FileEntry struct overhead in bytes
        let content_multiplier = if include_parsing {
            3 // Original content + parsed content + symbol data
        } else {
            1 // Just the original content
        };

        ((file_size * content_multiplier) + base_overhead) as u64
    }
}

/// RAII memory reservation that automatically releases on drop
#[derive(Debug)]
pub struct MemoryReservation {
    size: u64,
    manager: Arc<AtomicU64>,
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        self.manager.fetch_sub(self.size, Ordering::Relaxed);
        debug!("Released {}MB memory", self.size / (1024 * 1024));
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in MB
    pub current_usage_mb: u64,
    /// Maximum memory limit in MB (None if unlimited)
    pub max_memory_mb: Option<u64>,
    /// Memory utilization percentage (None if unlimited)
    pub utilization_percent: Option<u8>,
    /// Whether memory management is enabled
    pub enabled: bool,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.enabled {
            if let (Some(max), Some(util)) = (self.max_memory_mb, self.utilization_percent) {
                write!(
                    f,
                    "Memory: {}MB/{}MB ({}%)",
                    self.current_usage_mb, max, util
                )
            } else {
                write!(f, "Memory: {}MB (unlimited)", self.current_usage_mb)
            }
        } else {
            write!(f, "Memory management: disabled")
        }
    }
}

/// Configuration for memory limits during ingestion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryLimitsConfig {
    /// Total memory limit for ingestion process (in MB)
    /// None means no limit
    pub max_total_memory_mb: Option<u64>,
    /// Maximum number of files to process in parallel
    /// Helps control memory usage with large repositories
    pub max_parallel_files: Option<usize>,
    /// Enable automatic chunking when memory pressure is high
    pub enable_adaptive_chunking: bool,
    /// Chunk size in number of files when chunking is enabled
    pub chunk_size: usize,
}

impl Default for MemoryLimitsConfig {
    fn default() -> Self {
        Self {
            max_total_memory_mb: None, // No limits by default for backward compatibility
            max_parallel_files: None,  // Use system CPU count
            enable_adaptive_chunking: true,
            chunk_size: 100,
        }
    }
}

impl MemoryLimitsConfig {
    /// Create memory limits configuration for production use
    pub fn production() -> Self {
        Self {
            max_total_memory_mb: Some(1024), // 1GB default limit
            max_parallel_files: Some(num_cpus::get() * 2),
            enable_adaptive_chunking: true,
            chunk_size: 50,
        }
    }

    /// Create memory limits configuration for development use
    pub fn development() -> Self {
        Self {
            max_total_memory_mb: Some(512), // 512MB limit for dev
            max_parallel_files: Some(4),
            enable_adaptive_chunking: true,
            chunk_size: 25,
        }
    }

    /// Create memory limits configuration for testing
    pub fn testing() -> Self {
        Self {
            max_total_memory_mb: Some(100), // 100MB limit for tests
            max_parallel_files: Some(2),
            enable_adaptive_chunking: true,
            chunk_size: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new(Some(512));
        assert!(manager.enabled);
        assert_eq!(manager.max_memory, 512 * 1024 * 1024);
        assert_eq!(manager.current_usage.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_memory_manager_disabled() {
        let manager = MemoryManager::new(None);
        assert!(!manager.enabled);
        assert!(manager.can_allocate(u64::MAX));
        assert!(!manager.is_memory_pressure());
    }

    #[test]
    fn test_memory_reservation() -> Result<()> {
        let manager = MemoryManager::new(Some(100)); // 100MB

        // Should be able to allocate within limits
        let _reservation = manager.reserve(50 * 1024 * 1024)?; // 50MB
        assert_eq!(
            manager.current_usage.load(Ordering::Relaxed),
            50 * 1024 * 1024
        );

        // Should fail to allocate beyond limits
        let result = manager.reserve(60 * 1024 * 1024); // Another 60MB would exceed 100MB
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_memory_reservation_cleanup() -> Result<()> {
        let manager = MemoryManager::new(Some(100)); // 100MB

        {
            let _reservation = manager.reserve(50 * 1024 * 1024)?; // 50MB
            assert_eq!(
                manager.current_usage.load(Ordering::Relaxed),
                50 * 1024 * 1024
            );
        } // reservation drops here

        // Memory should be released
        assert_eq!(manager.current_usage.load(Ordering::Relaxed), 0);

        Ok(())
    }

    #[test]
    fn test_memory_pressure_detection() {
        let manager = MemoryManager::new(Some(100)); // 100MB

        // Below 80% should not trigger pressure
        let _low_usage = manager.reserve(70 * 1024 * 1024).unwrap(); // 70MB
        assert!(!manager.is_memory_pressure());

        drop(_low_usage);

        // Above 80% should trigger pressure
        let _high_usage = manager.reserve(85 * 1024 * 1024).unwrap(); // 85MB
        assert!(manager.is_memory_pressure());
    }

    #[test]
    fn test_file_memory_estimation() {
        let manager = MemoryManager::new(Some(100));

        let file_size = 1024; // 1KB file

        // Without parsing should be roughly the file size plus overhead
        let without_parsing = manager.estimate_file_memory(file_size, false);
        assert_eq!(without_parsing, 1024 + 200); // file + overhead

        // With parsing should be roughly 3x the file size plus overhead
        let with_parsing = manager.estimate_file_memory(file_size, true);
        assert_eq!(with_parsing, (1024 * 3) + 200); // 3x file + overhead
    }

    #[test]
    fn test_memory_stats() {
        let manager = MemoryManager::new(Some(100)); // 100MB
        let _reservation = manager.reserve(30 * 1024 * 1024).unwrap(); // 30MB

        let stats = manager.get_stats();
        assert_eq!(stats.current_usage_mb, 30);
        assert_eq!(stats.max_memory_mb, Some(100));
        assert_eq!(stats.utilization_percent, Some(30));
        assert!(stats.enabled);
    }

    #[test]
    fn test_memory_limits_config_presets() {
        let prod = MemoryLimitsConfig::production();
        assert_eq!(prod.max_total_memory_mb, Some(1024));
        assert_eq!(prod.chunk_size, 50);

        let dev = MemoryLimitsConfig::development();
        assert_eq!(dev.max_total_memory_mb, Some(512));
        assert_eq!(dev.chunk_size, 25);

        let test = MemoryLimitsConfig::testing();
        assert_eq!(test.max_total_memory_mb, Some(100));
        assert_eq!(test.chunk_size, 10);
    }
}
