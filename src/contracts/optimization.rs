// Optimization Contracts - Stage 2: Contract-First Design for Phase 2 Infrastructure
// Defines contracts for bulk operations and concurrent access patterns

use anyhow::Result;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::contracts::performance::{ComplexityClass, PerformanceMeasurement};

/// Contract for bulk operations with performance guarantees
pub trait BulkOperations {
    /// Bulk insert multiple key-value pairs in a single operation
    /// 
    /// # Preconditions
    /// - pairs must not be empty
    /// - all keys must be valid UUIDs
    /// - all paths must be valid and non-empty
    /// 
    /// # Postconditions  
    /// - all keys are searchable in the resulting tree
    /// - operation completes in O(n log n) time where n is pairs.len()
    /// - memory overhead is <2x the raw data size
    /// - tree balance is maintained (all leaves at same level)
    /// 
    /// # Performance Guarantee
    /// - Must achieve 5-10x speedup vs individual insertions
    /// - Memory allocation should be optimized for bulk size
    fn bulk_insert(&mut self, pairs: Vec<(ValidatedDocumentId, ValidatedPath)>) -> Result<BulkOperationResult>;
    
    /// Bulk delete multiple keys in a single operation
    /// 
    /// # Preconditions
    /// - keys must not be empty
    /// - all keys must be valid UUIDs
    /// 
    /// # Postconditions
    /// - none of the keys are searchable in the resulting tree
    /// - operation completes in O(k log n) time where k is keys.len(), n is tree size
    /// - memory is properly reclaimed (>90% of deleted entries)
    /// - tree balance is maintained after deletions
    /// 
    /// # Performance Guarantee  
    /// - Must achieve 5-10x speedup vs individual deletions
    /// - Memory cleanup should be optimized
    fn bulk_delete(&mut self, keys: Vec<ValidatedDocumentId>) -> Result<BulkOperationResult>;
    
    /// Bulk search for multiple keys in a single operation
    /// 
    /// # Preconditions
    /// - keys must not be empty
    /// - all keys must be valid UUIDs
    /// 
    /// # Postconditions
    /// - returns results for all keys (Some for found, None for not found)
    /// - operation completes in O(k log n) time where k is keys.len()
    /// - no side effects on tree structure
    /// 
    /// # Performance Guarantee
    /// - Must achieve 2-5x speedup vs individual searches
    /// - Should leverage cache locality
    fn bulk_search(&self, keys: Vec<ValidatedDocumentId>) -> Result<Vec<Option<ValidatedPath>>>;
}

/// Contract for concurrent access patterns
pub trait ConcurrentAccess {
    /// Perform read operation with optimized concurrent access
    /// 
    /// # Preconditions
    /// - key must be valid UUID
    /// 
    /// # Postconditions
    /// - returns current value if key exists
    /// - operation does not block other concurrent reads
    /// - operation completes in O(log n) time
    /// 
    /// # Concurrency Guarantee
    /// - Multiple concurrent reads are allowed
    /// - Read operations scale linearly with CPU cores
    /// - No read starvation under write load
    fn concurrent_read(&self, key: &ValidatedDocumentId) -> Result<Option<ValidatedPath>>;
    
    /// Perform write operation with proper isolation
    /// 
    /// # Preconditions
    /// - key must be valid UUID
    /// - path must be valid and non-empty
    /// 
    /// # Postconditions
    /// - key is searchable after operation completes
    /// - operation maintains ACID properties
    /// - tree structure integrity is preserved
    /// 
    /// # Concurrency Guarantee
    /// - Write operations are properly isolated
    /// - No lost updates under concurrent access
    /// - Deadlock prevention mechanisms active
    fn concurrent_write(&mut self, key: ValidatedDocumentId, path: ValidatedPath) -> Result<()>;
    
    /// Get current lock contention metrics
    /// 
    /// # Postconditions
    /// - returns real-time lock statistics
    /// - metrics reflect actual system state
    fn get_contention_metrics(&self) -> ContentionMetrics;
}

/// Result of bulk operations with performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResult {
    pub operations_completed: usize,
    pub duration: Duration,
    pub throughput_ops_per_sec: f64,
    pub memory_delta_bytes: i64,  // Can be negative for deletions
    pub tree_balance_factor: f64,
    pub complexity_class: ComplexityClass,
    pub errors: Vec<String>,
}

/// Lock contention and concurrency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentionMetrics {
    pub active_readers: u32,
    pub active_writers: u32,
    pub pending_readers: u32,
    pub pending_writers: u32,
    pub read_lock_wait_time: Duration,
    pub write_lock_wait_time: Duration,
    pub lock_acquisition_rate: f64,  // locks per second
    pub contention_ratio: f64,       // contested locks / total locks
}

/// Contract for tree structure analysis
pub trait TreeAnalysis {
    /// Analyze tree structure for optimization opportunities
    /// 
    /// # Postconditions
    /// - returns comprehensive tree metrics
    /// - identifies potential optimization points
    /// - recommends rebalancing if needed
    fn analyze_structure(&self) -> TreeStructureMetrics;
    
    /// Count total entries in the tree
    /// 
    /// # Postconditions
    /// - returns exact count of key-value pairs
    /// - operation completes in O(1) time (cached)
    fn count_entries(&self) -> usize;
    
    /// Get tree depth and balance information
    /// 
    /// # Postconditions
    /// - returns current tree depth and balance factor
    /// - identifies any balance issues
    fn get_balance_info(&self) -> BalanceInfo;
}

/// Comprehensive tree structure metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeStructureMetrics {
    pub total_entries: usize,
    pub tree_depth: usize,
    pub balance_factor: f64,
    pub utilization_factor: f64,
    pub memory_efficiency: f64,
    pub node_distribution: NodeDistribution,
    pub leaf_depth_variance: usize,
    pub recommended_actions: Vec<OptimizationRecommendation>,
}

/// Distribution of nodes in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDistribution {
    pub total_nodes: usize,
    pub leaf_nodes: usize,
    pub internal_nodes: usize,
    pub avg_keys_per_node: f64,
    pub min_keys_per_node: usize,
    pub max_keys_per_node: usize,
}

/// Tree balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub depth: usize,
    pub is_balanced: bool,
    pub balance_factor: f64,
    pub max_acceptable_depth: usize,
    pub rebalancing_recommended: bool,
    pub estimated_rebalance_time: Duration,
}

/// Optimization recommendations based on analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationRecommendation {
    RebalanceTree { 
        reason: String, 
        estimated_improvement: f64 
    },
    CompactNodes { 
        fragmented_nodes: usize, 
        estimated_memory_savings: usize 
    },
    SplitOversizedNodes { 
        oversized_nodes: usize, 
        recommended_split_threshold: usize 
    },
    EnableCaching { 
        hot_paths: Vec<String>, 
        estimated_speedup: f64 
    },
    OptimizeBulkOperations { 
        operation_type: String, 
        current_efficiency: f64, 
        target_efficiency: f64 
    },
}

/// Contract for memory optimization
pub trait MemoryOptimization {
    /// Get current memory usage breakdown
    /// 
    /// # Postconditions
    /// - returns detailed memory statistics
    /// - identifies memory optimization opportunities
    fn get_memory_usage(&self) -> MemoryUsage;
    
    /// Perform memory compaction to reduce fragmentation
    /// 
    /// # Preconditions
    /// - tree must be in consistent state
    /// 
    /// # Postconditions
    /// - memory fragmentation is reduced
    /// - tree functionality is preserved
    /// - operation completes in O(n) time
    fn compact_memory(&mut self) -> Result<MemoryCompactionResult>;
    
    /// Set memory usage limits and policies
    /// 
    /// # Preconditions
    /// - limits must be reasonable (> current usage)
    /// 
    /// # Postconditions
    /// - memory usage is monitored against limits
    /// - automatic cleanup triggers are set
    fn set_memory_limits(&mut self, limits: MemoryLimits) -> Result<()>;
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub total_allocated: usize,
    pub tree_data: usize,
    pub metadata: usize,
    pub fragmentation: usize,
    pub efficiency_ratio: f64,  // data / total
    pub overhead_ratio: f64,    // overhead / data
    pub peak_usage: usize,
    pub allocations_per_second: f64,
}

/// Result of memory compaction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCompactionResult {
    pub bytes_freed: usize,
    pub fragmentation_reduced: f64,  // percentage
    pub compaction_duration: Duration,
    pub performance_impact: f64,     // slowdown factor during compaction
}

/// Memory usage limits and policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_total_memory: usize,
    pub max_fragmentation_ratio: f64,
    pub auto_compact_threshold: f64,
    pub emergency_cleanup_threshold: f64,
    pub growth_rate_limit: f64,  // bytes per second
}

/// Performance SLA for optimization operations
pub trait OptimizationSLA {
    /// Verify bulk operation meets performance contract
    /// 
    /// # Postconditions
    /// - validates operation against defined SLA
    /// - returns compliance report
    fn verify_bulk_operation_sla(
        &self, 
        operation_type: BulkOperationType,
        result: &BulkOperationResult
    ) -> SLAComplianceReport;
    
    /// Verify concurrent access meets performance contract
    /// 
    /// # Postconditions
    /// - validates concurrency against defined SLA
    /// - returns compliance report
    fn verify_concurrent_access_sla(
        &self,
        metrics: &ContentionMetrics
    ) -> SLAComplianceReport;
}

/// Types of bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BulkOperationType {
    Insert,
    Delete,
    Search,
    Update,
}

/// SLA compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAComplianceReport {
    pub compliant: bool,
    pub operation_type: String,
    pub actual_performance: PerformanceMeasurement,
    pub sla_requirements: SLARequirements,
    pub violations: Vec<SLAViolation>,
    pub recommendations: Vec<String>,
}

/// SLA requirements for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLARequirements {
    pub max_latency: Duration,
    pub min_throughput: f64,
    pub max_memory_overhead: f64,
    pub required_complexity: ComplexityClass,
    pub max_contention_ratio: f64,
}

/// SLA violation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAViolation {
    pub metric: String,
    pub expected: String,
    pub actual: String,
    pub severity: ViolationSeverity,
}

/// Severity of SLA violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Minor,      // < 20% deviation
    Moderate,   // 20-50% deviation  
    Severe,     // > 50% deviation
    Critical,   // Complete failure
}

// Default implementations for common scenarios

impl BulkOperationResult {
    /// Create successful bulk operation result
    pub fn success(
        operations: usize,
        duration: Duration,
        memory_delta: i64,
        balance_factor: f64
    ) -> Self {
        let throughput = operations as f64 / duration.as_secs_f64();
        
        Self {
            operations_completed: operations,
            duration,
            throughput_ops_per_sec: throughput,
            memory_delta_bytes: memory_delta,
            tree_balance_factor: balance_factor,
            complexity_class: ComplexityClass::Linearithmic, // O(n log n) for bulk ops
            errors: Vec::new(),
        }
    }
    
    /// Create failed bulk operation result
    pub fn failure(error: String) -> Self {
        Self {
            operations_completed: 0,
            duration: Duration::ZERO,
            throughput_ops_per_sec: 0.0,
            memory_delta_bytes: 0,
            tree_balance_factor: 0.0,
            complexity_class: ComplexityClass::Unknown,
            errors: vec![error],
        }
    }
    
    /// Check if operation meets performance requirements
    pub fn meets_performance_requirements(&self, min_speedup: f64) -> bool {
        self.errors.is_empty() 
            && self.throughput_ops_per_sec > 0.0
            && self.tree_balance_factor > 0.8
            && matches!(self.complexity_class, 
                ComplexityClass::Constant | 
                ComplexityClass::Logarithmic | 
                ComplexityClass::Linearithmic
            )
    }
}

impl ContentionMetrics {
    /// Create empty contention metrics
    pub fn empty() -> Self {
        Self {
            active_readers: 0,
            active_writers: 0,
            pending_readers: 0,
            pending_writers: 0,
            read_lock_wait_time: Duration::ZERO,
            write_lock_wait_time: Duration::ZERO,
            lock_acquisition_rate: 0.0,
            contention_ratio: 0.0,
        }
    }
    
    /// Check if contention is within acceptable limits
    pub fn is_healthy(&self) -> bool {
        self.contention_ratio < 0.3  // Less than 30% contested locks
            && self.write_lock_wait_time < Duration::from_millis(100)
            && self.pending_writers < 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bulk_operation_result_success() {
        let result = BulkOperationResult::success(
            1000,
            Duration::from_millis(100),
            1024,
            0.95
        );
        
        assert!(result.meets_performance_requirements(5.0));
        assert_eq!(result.operations_completed, 1000);
        assert_eq!(result.throughput_ops_per_sec, 10000.0);
    }
    
    #[test]
    fn test_contention_metrics_health() {
        let healthy_metrics = ContentionMetrics::empty();
        assert!(healthy_metrics.is_healthy());
        
        let unhealthy_metrics = ContentionMetrics {
            contention_ratio: 0.5,  // 50% contested
            write_lock_wait_time: Duration::from_millis(200),
            ..ContentionMetrics::empty()
        };
        assert!(!unhealthy_metrics.is_healthy());
    }
}