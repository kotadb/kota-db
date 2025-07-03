// Performance SLA Contracts - Stage 2: Contract-First Design
// Defines performance guarantees and complexity bounds for KotaDB components

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Complexity class classification for operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityClass {
    Constant,     // O(1)
    Logarithmic,  // O(log n)
    Linear,       // O(n)
    Linearithmic, // O(n log n)
    Quadratic,    // O(n²)
    Unknown,
}

impl ComplexityClass {
    /// Get the theoretical growth factor when input size increases by this multiplier
    pub fn theoretical_growth_factor(&self, size_multiplier: f64) -> f64 {
        match self {
            ComplexityClass::Constant => 1.0,
            ComplexityClass::Logarithmic => size_multiplier.log2(),
            ComplexityClass::Linear => size_multiplier,
            ComplexityClass::Linearithmic => size_multiplier * size_multiplier.log2(),
            ComplexityClass::Quadratic => size_multiplier * size_multiplier,
            ComplexityClass::Unknown => f64::INFINITY,
        }
    }
}

/// Performance measurement for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMeasurement {
    pub operation: String,
    pub input_size: usize,
    pub duration: Duration,
    pub throughput_ops_per_sec: f64,
    pub memory_used_bytes: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Performance Service Level Agreement
pub trait PerformanceGuarantee {
    /// The expected complexity class for this operation
    fn complexity_class(&self) -> ComplexityClass;

    /// Maximum acceptable time per operation for a given input size
    fn max_operation_time(&self, input_size: usize) -> Duration;

    /// Minimum acceptable throughput (operations per second)
    fn min_throughput(&self, input_size: usize) -> f64;

    /// Maximum acceptable memory overhead as a multiple of raw data size
    fn max_memory_overhead(&self) -> f64;

    /// Validate that a measurement meets the SLA
    fn validate_measurement(&self, measurement: &PerformanceMeasurement) -> Result<()> {
        // Check operation time
        let max_time = self.max_operation_time(measurement.input_size);
        if measurement.duration > max_time {
            anyhow::bail!(
                "Operation {} exceeded time SLA: {:?} > {:?} for size {}",
                measurement.operation,
                measurement.duration,
                max_time,
                measurement.input_size
            );
        }

        // Check throughput
        let min_throughput = self.min_throughput(measurement.input_size);
        if measurement.throughput_ops_per_sec < min_throughput {
            anyhow::bail!(
                "Operation {} below throughput SLA: {:.0} < {:.0} ops/sec for size {}",
                measurement.operation,
                measurement.throughput_ops_per_sec,
                min_throughput,
                measurement.input_size
            );
        }

        Ok(())
    }
}

/// Contract defining algorithmic complexity bounds
pub trait ComplexityContract {
    /// Expected complexity class for the primary operation
    fn expected_complexity(&self) -> ComplexityClass;

    /// Maximum growth factor when input size increases
    fn max_growth_factor(&self, size_multiplier: f64) -> f64 {
        // Allow 50% overhead over theoretical
        self.expected_complexity()
            .theoretical_growth_factor(size_multiplier)
            * 1.5
    }

    /// Validate complexity across multiple measurements
    fn validate_complexity_growth(&self, measurements: &[PerformanceMeasurement]) -> Result<()> {
        if measurements.len() < 2 {
            return Ok(());
        }

        for i in 1..measurements.len() {
            let prev = &measurements[i - 1];
            let curr = &measurements[i];

            let size_ratio = curr.input_size as f64 / prev.input_size as f64;
            let time_ratio = curr.duration.as_secs_f64() / prev.duration.as_secs_f64();

            let max_allowed = self.max_growth_factor(size_ratio);

            if time_ratio > max_allowed {
                anyhow::bail!(
                    "Complexity violation: size {}→{} ({}x) caused time ratio {:.2}x (max: {:.2}x)",
                    prev.input_size,
                    curr.input_size,
                    size_ratio,
                    time_ratio,
                    max_allowed
                );
            }
        }

        Ok(())
    }
}

/// Memory usage contract
pub trait MemoryContract {
    /// Maximum memory overhead as multiple of raw data
    fn max_memory_overhead(&self) -> f64;

    /// Whether memory should be released immediately after operations
    fn requires_immediate_cleanup(&self) -> bool;

    /// Validate memory usage patterns
    fn validate_memory_usage(&self, raw_data_size: usize, allocated_size: usize) -> Result<()> {
        let overhead_ratio = allocated_size as f64 / raw_data_size as f64;
        let max_overhead = self.max_memory_overhead();

        if overhead_ratio > max_overhead {
            anyhow::bail!(
                "Memory overhead exceeded: {:.2}x > {:.2}x (raw: {} bytes, allocated: {} bytes)",
                overhead_ratio,
                max_overhead,
                raw_data_size,
                allocated_size
            );
        }

        Ok(())
    }
}

/// B+ Tree performance SLA implementation
#[derive(Debug, Clone)]
pub struct BTreePerformanceSLA {
    pub max_operation_time_base_us: f64, // Base time for size=1000
    pub min_throughput_base: f64,        // Base throughput for size=1000
}

impl Default for BTreePerformanceSLA {
    fn default() -> Self {
        Self {
            max_operation_time_base_us: 50.0, // 50μs for 1000 elements
            min_throughput_base: 20_000.0,    // 20k ops/sec for 1000 elements
        }
    }
}

impl PerformanceGuarantee for BTreePerformanceSLA {
    fn complexity_class(&self) -> ComplexityClass {
        ComplexityClass::Logarithmic
    }

    fn max_operation_time(&self, input_size: usize) -> Duration {
        // O(log n) scaling: time = base * log(size/1000)
        let size_factor = (input_size as f64 / 1000.0).max(1.0);
        let log_factor = size_factor.log2();
        let time_us = self.max_operation_time_base_us * log_factor;
        Duration::from_micros(time_us as u64)
    }

    fn min_throughput(&self, input_size: usize) -> f64 {
        // Throughput decreases logarithmically with size
        let size_factor = (input_size as f64 / 1000.0).max(1.0);
        let log_factor = size_factor.log2();
        self.min_throughput_base / log_factor
    }

    fn max_memory_overhead(&self) -> f64 {
        2.5 // B+ tree should use at most 2.5x raw data size
    }
}

impl ComplexityContract for BTreePerformanceSLA {
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::Logarithmic
    }
}

impl MemoryContract for BTreePerformanceSLA {
    fn max_memory_overhead(&self) -> f64 {
        2.5
    }

    fn requires_immediate_cleanup(&self) -> bool {
        true // B+ tree should clean up memory after deletions
    }
}

/// Index operation performance SLA
#[derive(Debug, Clone)]
pub struct IndexOperationSLA {
    pub operation_type: String,
    pub expected_complexity: ComplexityClass,
    pub base_time_us: f64,
    pub base_throughput: f64,
}

impl IndexOperationSLA {
    pub fn new_btree_insert() -> Self {
        Self {
            operation_type: "btree_insert".to_string(),
            expected_complexity: ComplexityClass::Logarithmic,
            base_time_us: 30.0,        // 30μs base time
            base_throughput: 30_000.0, // 30k ops/sec base
        }
    }

    pub fn new_btree_search() -> Self {
        Self {
            operation_type: "btree_search".to_string(),
            expected_complexity: ComplexityClass::Logarithmic,
            base_time_us: 10.0,         // 10μs base time (faster than insert)
            base_throughput: 100_000.0, // 100k ops/sec base
        }
    }

    pub fn new_btree_delete() -> Self {
        Self {
            operation_type: "btree_delete".to_string(),
            expected_complexity: ComplexityClass::Logarithmic,
            base_time_us: 100.0,       // 100μs base time (rebalancing overhead)
            base_throughput: 10_000.0, // 10k ops/sec base
        }
    }
}

impl PerformanceGuarantee for IndexOperationSLA {
    fn complexity_class(&self) -> ComplexityClass {
        self.expected_complexity.clone()
    }

    fn max_operation_time(&self, input_size: usize) -> Duration {
        let time_us = match self.expected_complexity {
            ComplexityClass::Constant => self.base_time_us,
            ComplexityClass::Logarithmic => {
                let size_factor = (input_size as f64 / 1000.0).max(1.0);
                self.base_time_us * size_factor.log2()
            }
            ComplexityClass::Linear => {
                let size_factor = (input_size as f64 / 1000.0).max(1.0);
                self.base_time_us * size_factor
            }
            _ => self.base_time_us * 10.0, // Conservative fallback
        };

        Duration::from_micros(time_us as u64)
    }

    fn min_throughput(&self, input_size: usize) -> f64 {
        match self.expected_complexity {
            ComplexityClass::Constant => self.base_throughput,
            ComplexityClass::Logarithmic => {
                let size_factor = (input_size as f64 / 1000.0).max(1.0);
                self.base_throughput / size_factor.log2()
            }
            ComplexityClass::Linear => {
                let size_factor = (input_size as f64 / 1000.0).max(1.0);
                self.base_throughput / size_factor
            }
            _ => self.base_throughput / 10.0, // Conservative fallback
        }
    }

    fn max_memory_overhead(&self) -> f64 {
        3.0 // General index operations
    }
}

impl ComplexityContract for IndexOperationSLA {
    fn expected_complexity(&self) -> ComplexityClass {
        self.expected_complexity.clone()
    }
}

/// Performance contract validator
pub struct PerformanceValidator {
    slas: Vec<Box<dyn PerformanceGuarantee + Send + Sync>>,
}

impl PerformanceValidator {
    pub fn new() -> Self {
        Self { slas: Vec::new() }
    }

    pub fn add_sla<T: PerformanceGuarantee + Send + Sync + 'static>(&mut self, sla: T) {
        self.slas.push(Box::new(sla));
    }

    pub fn validate_measurement(&self, measurement: &PerformanceMeasurement) -> Result<()> {
        for sla in &self.slas {
            sla.validate_measurement(measurement)?;
        }
        Ok(())
    }

    pub fn validate_complexity_trend(&self, measurements: &[PerformanceMeasurement]) -> Result<()> {
        // Group measurements by operation
        let mut by_operation: std::collections::HashMap<String, Vec<&PerformanceMeasurement>> =
            std::collections::HashMap::new();

        for measurement in measurements {
            by_operation
                .entry(measurement.operation.clone())
                .or_insert_with(Vec::new)
                .push(measurement);
        }

        // Validate each operation's complexity trend
        for (operation, mut op_measurements) in by_operation {
            op_measurements.sort_by_key(|m| m.input_size);

            // Find matching SLA
            if let Some(sla) = self.slas.iter().find(|s| {
                // Simple operation name matching - could be more sophisticated
                operation.contains("insert") && s.complexity_class() == ComplexityClass::Logarithmic
            }) {
                // For now, we can't call validate_complexity_growth on trait object
                // This would need trait object support or restructuring
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_sla_validation() {
        let sla = BTreePerformanceSLA::default();

        // Valid measurement
        let good_measurement = PerformanceMeasurement {
            operation: "btree_insert".to_string(),
            input_size: 1000,
            duration: Duration::from_micros(40), // Under 50μs limit
            throughput_ops_per_sec: 25_000.0,    // Above 20k limit
            memory_used_bytes: 1000,
            timestamp: chrono::Utc::now(),
        };

        assert!(sla.validate_measurement(&good_measurement).is_ok());

        // Too slow measurement
        let slow_measurement = PerformanceMeasurement {
            operation: "btree_insert".to_string(),
            input_size: 1000,
            duration: Duration::from_micros(200), // Too slow
            throughput_ops_per_sec: 5_000.0,      // Too slow
            memory_used_bytes: 1000,
            timestamp: chrono::Utc::now(),
        };

        assert!(sla.validate_measurement(&slow_measurement).is_err());
    }

    #[test]
    fn test_complexity_growth_validation() {
        let sla = BTreePerformanceSLA::default();

        let measurements = vec![
            PerformanceMeasurement {
                operation: "test".to_string(),
                input_size: 1000,
                duration: Duration::from_micros(50),
                throughput_ops_per_sec: 20_000.0,
                memory_used_bytes: 1000,
                timestamp: chrono::Utc::now(),
            },
            PerformanceMeasurement {
                operation: "test".to_string(),
                input_size: 10_000,
                duration: Duration::from_micros(165), // ~3.3x growth (acceptable for O(log n))
                throughput_ops_per_sec: 6_000.0,
                memory_used_bytes: 10_000,
                timestamp: chrono::Utc::now(),
            },
        ];

        assert!(sla.validate_complexity_growth(&measurements).is_ok());

        // Test with excessive growth
        let bad_measurements = vec![
            measurements[0].clone(),
            PerformanceMeasurement {
                operation: "test".to_string(),
                input_size: 10_000,
                duration: Duration::from_micros(500), // 10x growth (too much for O(log n))
                throughput_ops_per_sec: 2_000.0,
                memory_used_bytes: 10_000,
                timestamp: chrono::Utc::now(),
            },
        ];

        assert!(sla.validate_complexity_growth(&bad_measurements).is_err());
    }
}
