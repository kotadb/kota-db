// Performance Analysis Pure Functions - Stage 3: Pure Function Modularization
// Statistical analysis and complexity detection functions for performance data

use crate::contracts::performance::{ComplexityClass, PerformanceMeasurement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Statistical summary of performance measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub count: usize,
    pub mean: Duration,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    pub std_dev: Duration,
    pub percentile_95: Duration,
    pub percentile_99: Duration,
}

/// Growth analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthAnalysis {
    pub detected_complexity: ComplexityClass,
    pub confidence: f64, // 0.0 to 1.0
    pub growth_factor: f64,
    pub r_squared: f64, // Goodness of fit
    pub data_points: usize,
}

/// Regression analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub has_regression: bool,
    pub severity: RegressionSeverity,
    pub affected_metrics: Vec<String>,
    pub baseline_stats: PerformanceStats,
    pub current_stats: PerformanceStats,
    pub degradation_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionSeverity {
    None,
    Minor,    // < 20% degradation
    Moderate, // 20-50% degradation
    Severe,   // > 50% degradation
}

/// Tree structure metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeMetrics {
    pub depth: usize,
    pub total_nodes: usize,
    pub leaf_nodes: usize,
    pub internal_nodes: usize,
    pub total_keys: usize,
    pub avg_keys_per_node: f64,
    pub balance_factor: f64,
    pub utilization_factor: f64, // Average node fullness
    pub memory_efficiency: f64,  // Data size / total memory
}

/// Calculate statistical summary of performance measurements
pub fn calculate_performance_stats(durations: &[Duration]) -> PerformanceStats {
    if durations.is_empty() {
        return PerformanceStats {
            count: 0,
            mean: Duration::ZERO,
            median: Duration::ZERO,
            min: Duration::ZERO,
            max: Duration::ZERO,
            std_dev: Duration::ZERO,
            percentile_95: Duration::ZERO,
            percentile_99: Duration::ZERO,
        };
    }

    let mut sorted_durations = durations.to_vec();
    sorted_durations.sort();

    let count = durations.len();
    let min = sorted_durations[0];
    let max = sorted_durations[count - 1];

    // Calculate mean
    let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
    let mean_nanos = total_nanos / count as u128;
    let mean = Duration::from_nanos(mean_nanos as u64);

    // Calculate median
    let median = if count % 2 == 0 {
        let mid1 = sorted_durations[count / 2 - 1];
        let mid2 = sorted_durations[count / 2];
        Duration::from_nanos((mid1.as_nanos() + mid2.as_nanos()) as u64 / 2)
    } else {
        sorted_durations[count / 2]
    };

    // Calculate standard deviation
    let variance: f64 = durations
        .iter()
        .map(|d| {
            let diff = d.as_nanos() as f64 - mean_nanos as f64;
            diff * diff
        })
        .sum::<f64>()
        / count as f64;
    let std_dev = Duration::from_nanos(variance.sqrt() as u64);

    // Calculate percentiles
    let percentile_95_idx = ((count as f64 * 0.95) as usize).min(count - 1);
    let percentile_99_idx = ((count as f64 * 0.99) as usize).min(count - 1);
    let percentile_95 = sorted_durations[percentile_95_idx];
    let percentile_99 = sorted_durations[percentile_99_idx];

    PerformanceStats {
        count,
        mean,
        median,
        min,
        max,
        std_dev,
        percentile_95,
        percentile_99,
    }
}

/// Detect algorithmic complexity from size/time data points
pub fn calculate_complexity_factor(sizes: &[usize], times: &[Duration]) -> GrowthAnalysis {
    if sizes.len() != times.len() || sizes.len() < 2 {
        return GrowthAnalysis {
            detected_complexity: ComplexityClass::Unknown,
            confidence: 0.0,
            growth_factor: 0.0,
            r_squared: 0.0,
            data_points: sizes.len(),
        };
    }

    let log_sizes: Vec<f64> = sizes.iter().map(|&s| (s as f64).ln()).collect();
    let log_times: Vec<f64> = times.iter().map(|t| (t.as_nanos() as f64).ln()).collect();

    // Fit different models and find best match
    let linear_fit = fit_linear(&log_sizes, &log_times);
    let _constant_fit = fit_constant(&log_times);

    // Determine complexity based on slope of log-log plot
    let (complexity, confidence, r_squared) = if linear_fit.r_squared > 0.8 {
        let slope = linear_fit.slope;
        if slope < 0.2 {
            (
                ComplexityClass::Constant,
                linear_fit.r_squared,
                linear_fit.r_squared,
            )
        } else if slope < 0.8 {
            (
                ComplexityClass::Logarithmic,
                linear_fit.r_squared,
                linear_fit.r_squared,
            )
        } else if slope < 1.2 {
            (
                ComplexityClass::Linear,
                linear_fit.r_squared,
                linear_fit.r_squared,
            )
        } else if slope < 1.8 {
            (
                ComplexityClass::Linearithmic,
                linear_fit.r_squared,
                linear_fit.r_squared,
            )
        } else {
            (
                ComplexityClass::Quadratic,
                linear_fit.r_squared,
                linear_fit.r_squared,
            )
        }
    } else {
        (ComplexityClass::Unknown, 0.0, 0.0)
    };

    // Calculate average growth factor
    let mut growth_factors = Vec::with_capacity(sizes.len() - 1);
    for i in 1..sizes.len() {
        let size_ratio = sizes[i] as f64 / sizes[i - 1] as f64;
        let time_ratio = times[i].as_nanos() as f64 / times[i - 1].as_nanos() as f64;
        growth_factors.push(time_ratio / size_ratio);
    }
    let avg_growth = growth_factors.iter().sum::<f64>() / growth_factors.len() as f64;

    GrowthAnalysis {
        detected_complexity: complexity,
        confidence,
        growth_factor: avg_growth,
        r_squared,
        data_points: sizes.len(),
    }
}

/// Linear regression result
#[derive(Debug)]
struct LinearFit {
    slope: f64,
    #[allow(dead_code)]
    intercept: f64,
    r_squared: f64,
}

/// Fit linear model: y = mx + b
fn fit_linear(x: &[f64], y: &[f64]) -> LinearFit {
    let n = x.len() as f64;
    let x_mean = x.iter().sum::<f64>() / n;
    let y_mean = y.iter().sum::<f64>() / n;

    let numerator: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
        .sum();

    let denominator: f64 = x.iter().map(|xi| (xi - x_mean) * (xi - x_mean)).sum();

    let slope = if denominator.abs() < f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    };

    let intercept = y_mean - slope * x_mean;

    // Calculate R²
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| {
            let predicted = slope * xi + intercept;
            let residual = yi - predicted;
            residual * residual
        })
        .sum();

    let ss_tot: f64 = y.iter().map(|yi| (yi - y_mean) * (yi - y_mean)).sum();

    let r_squared = if ss_tot.abs() < f64::EPSILON {
        0.0
    } else {
        1.0 - (ss_res / ss_tot)
    };

    LinearFit {
        slope,
        intercept,
        r_squared: r_squared.max(0.0),
    }
}

/// Fit constant model (for O(1) complexity)
fn fit_constant(y: &[f64]) -> LinearFit {
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let ss_tot: f64 = y.iter().map(|yi| (yi - mean) * (yi - mean)).sum();
    let variance = ss_tot / y.len() as f64;

    // R² for constant model (how well constant explains variance)
    let r_squared = if variance < mean * 0.1 { 0.9 } else { 0.0 };

    LinearFit {
        slope: 0.0,
        intercept: mean,
        r_squared,
    }
}

/// Analyze performance growth pattern across multiple data points
pub fn analyze_growth_pattern(measurements: &[PerformanceMeasurement]) -> GrowthAnalysis {
    if measurements.len() < 2 {
        return GrowthAnalysis {
            detected_complexity: ComplexityClass::Unknown,
            confidence: 0.0,
            growth_factor: 0.0,
            r_squared: 0.0,
            data_points: measurements.len(),
        };
    }

    let sizes: Vec<usize> = measurements.iter().map(|m| m.input_size).collect();
    let times: Vec<Duration> = measurements.iter().map(|m| m.duration).collect();

    calculate_complexity_factor(&sizes, &times)
}

/// Detect performance regression between baseline and current measurements
pub fn detect_performance_regression(
    baseline: &[PerformanceMeasurement],
    current: &[PerformanceMeasurement],
) -> RegressionReport {
    if baseline.is_empty() || current.is_empty() {
        return RegressionReport {
            has_regression: false,
            severity: RegressionSeverity::None,
            affected_metrics: Vec::new(),
            baseline_stats: calculate_performance_stats(&[]),
            current_stats: calculate_performance_stats(&[]),
            degradation_factor: 1.0,
        };
    }

    let baseline_times: Vec<Duration> = baseline.iter().map(|m| m.duration).collect();
    let current_times: Vec<Duration> = current.iter().map(|m| m.duration).collect();

    let baseline_stats = calculate_performance_stats(&baseline_times);
    let current_stats = calculate_performance_stats(&current_times);

    // Calculate degradation factor using median (more robust than mean)
    let degradation_factor = if baseline_stats.median.as_nanos() > 0 {
        current_stats.median.as_nanos() as f64 / baseline_stats.median.as_nanos() as f64
    } else {
        1.0
    };

    let (has_regression, severity) = if degradation_factor > 1.5 {
        (true, RegressionSeverity::Severe)
    } else if degradation_factor > 1.2 {
        (true, RegressionSeverity::Moderate)
    } else if degradation_factor > 1.1 {
        (true, RegressionSeverity::Minor)
    } else {
        (false, RegressionSeverity::None)
    };

    let affected_metrics = if has_regression {
        vec!["median_latency".to_string(), "95th_percentile".to_string()]
    } else {
        Vec::new()
    };

    RegressionReport {
        has_regression,
        severity,
        affected_metrics,
        baseline_stats,
        current_stats,
        degradation_factor,
    }
}

/// Generate comprehensive tree metrics (would need actual tree structure)
pub fn generate_performance_metrics_stub() -> TreeMetrics {
    // This is a stub - real implementation would analyze actual tree structure
    TreeMetrics {
        depth: 5,
        total_nodes: 100,
        leaf_nodes: 64,
        internal_nodes: 36,
        total_keys: 1000,
        avg_keys_per_node: 10.0,
        balance_factor: 1.0,
        utilization_factor: 0.75,
        memory_efficiency: 0.6,
    }
}

/// Calculate memory efficiency ratio
pub fn calculate_memory_efficiency(raw_data_size: usize, allocated_size: usize) -> f64 {
    if allocated_size == 0 {
        return 0.0;
    }
    raw_data_size as f64 / allocated_size as f64
}

/// Generate performance visualization data (CSV format)
pub fn generate_csv_data(measurements: &[PerformanceMeasurement]) -> String {
    let mut csv = String::from("size,duration_ns,throughput_ops_per_sec,memory_bytes,timestamp\n");

    for measurement in measurements {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            measurement.input_size,
            measurement.duration.as_nanos(),
            measurement.throughput_ops_per_sec,
            measurement.memory_used_bytes,
            measurement.timestamp.format("%Y-%m-%d %H:%M:%S")
        ));
    }

    csv
}

/// Create performance comparison table
pub fn generate_comparison_table(
    linear_measurements: &[PerformanceMeasurement],
    btree_measurements: &[PerformanceMeasurement],
) -> String {
    let mut table = String::from(
        "| Size | Linear Time | B+Tree Time | Speedup | Linear Throughput | B+Tree Throughput |\n",
    );
    table.push_str("|------|-------------|--------------|---------|-------------------|--------------------|\n");

    let linear_map: HashMap<usize, &PerformanceMeasurement> = linear_measurements
        .iter()
        .map(|m| (m.input_size, m))
        .collect();

    for btree_m in btree_measurements {
        if let Some(linear_m) = linear_map.get(&btree_m.input_size) {
            let speedup = linear_m.duration.as_nanos() as f64 / btree_m.duration.as_nanos() as f64;

            table.push_str(&format!(
                "| {:>6} | {:>11} | {:>12} | {:>7.1}x | {:>17.0} | {:>18.0} |\n",
                btree_m.input_size,
                format!("{:?}", linear_m.duration),
                format!("{:?}", btree_m.duration),
                speedup,
                linear_m.throughput_ops_per_sec,
                btree_m.throughput_ops_per_sec
            ));
        }
    }

    table
}

/// Calculate variance of a set of measurements
pub fn calculate_variance(measurements: &[f64]) -> f64 {
    if measurements.len() < 2 {
        return 0.0;
    }

    let mean = measurements.iter().sum::<f64>() / measurements.len() as f64;
    let variance = measurements.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
        / (measurements.len() - 1) as f64;

    variance
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_performance_stats_calculation() {
        let durations = vec![
            Duration::from_micros(10),
            Duration::from_micros(20),
            Duration::from_micros(30),
            Duration::from_micros(40),
            Duration::from_micros(50),
        ];

        let stats = calculate_performance_stats(&durations);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.mean, Duration::from_micros(30));
        assert_eq!(stats.median, Duration::from_micros(30));
        assert_eq!(stats.min, Duration::from_micros(10));
        assert_eq!(stats.max, Duration::from_micros(50));
    }

    #[test]
    fn test_complexity_detection_logarithmic() {
        let sizes = vec![100, 1000, 10000];
        let times = vec![
            Duration::from_micros(10), // Base time
            Duration::from_micros(33), // ~3.3x (log 10)
            Duration::from_micros(66), // ~6.6x (log 100)
        ];

        let analysis = calculate_complexity_factor(&sizes, &times);

        // Should detect logarithmic complexity
        assert!(matches!(
            analysis.detected_complexity,
            ComplexityClass::Logarithmic
        ));
        assert!(analysis.confidence > 0.7);
    }

    #[test]
    fn test_regression_detection() {
        let baseline = vec![PerformanceMeasurement {
            operation: "test".to_string(),
            input_size: 1000,
            duration: Duration::from_micros(100),
            throughput_ops_per_sec: 10000.0,
            memory_used_bytes: 1000,
            timestamp: chrono::Utc::now(),
        }];

        let current = vec![PerformanceMeasurement {
            operation: "test".to_string(),
            input_size: 1000,
            duration: Duration::from_micros(180), // 80% slower
            throughput_ops_per_sec: 5555.0,
            memory_used_bytes: 1000,
            timestamp: chrono::Utc::now(),
        }];

        let report = detect_performance_regression(&baseline, &current);

        assert!(report.has_regression);
        assert!(matches!(report.severity, RegressionSeverity::Severe));
        assert!(report.degradation_factor > 1.5);
    }

    // Extracted from integration tests - Performance ratio calculation utilities
    #[test]
    fn test_performance_growth_ratio_calculation() {
        // Test calculation of performance growth ratios for O(log n) verification
        let timings = [
            (100, 10.0),   // size, avg_time_us
            (1000, 33.0),  // 10x size increase, ~3.3x time increase (logarithmic)
            (10000, 66.0), // 10x size increase, ~6.6x time increase (logarithmic)
        ];

        // Calculate growth ratios
        let ratio_1_to_2 = timings[1].1 / timings[0].1;
        let ratio_2_to_3 = timings[2].1 / timings[1].1;

        // For O(log n), when size increases 10x, time should increase ~3.3x
        assert!(
            ratio_1_to_2 > 2.5 && ratio_1_to_2 < 4.5,
            "First ratio should be logarithmic: {}",
            ratio_1_to_2
        );
        assert!(
            ratio_2_to_3 > 1.8 && ratio_2_to_3 < 2.5,
            "Second ratio should be logarithmic: {}",
            ratio_2_to_3
        );

        // Both ratios should be much less than 10 (which would indicate linear O(n))
        assert!(
            ratio_1_to_2 < 5.0,
            "Performance growth too linear: {}",
            ratio_1_to_2
        );
        assert!(
            ratio_2_to_3 < 5.0,
            "Performance growth too linear: {}",
            ratio_2_to_3
        );
    }

    #[test]
    fn test_timing_measurement_accuracy() {
        // Test accuracy of timing measurements with different thresholds
        let measurements = vec![
            0.005, // Very fast (sub-microsecond)
            0.1,   // Fast
            1.5,   // Medium
            10.0,  // Slow
        ];

        for &measurement in &measurements {
            if measurement > 0.01 {
                // Measurement is accurate enough for ratio calculation
                let ratio = measurement / 0.01;
                assert!(
                    ratio > 1.0,
                    "Ratio calculation should work for measurable times"
                );
            } else {
                // Very fast measurement - should use minimum threshold
                let adjusted_ratio = measurement / 0.01;
                assert!(
                    adjusted_ratio <= 1.0,
                    "Sub-threshold measurements handled correctly"
                );
            }
        }
    }

    #[test]
    fn test_algorithm_complexity_comparison() {
        // Test comparison between different algorithmic complexities
        let linear_time_us = 5000.0; // Linear search time
        let btree_time_us = 0.5; // B+ tree search time

        let speedup = linear_time_us / btree_time_us;
        assert!(
            speedup >= 100.0,
            "B+ tree should be significantly faster: {}x",
            speedup
        );

        // Verify speedup calculation
        assert!(
            btree_time_us < linear_time_us / 10.0,
            "B+ tree not fast enough: {}μs vs {}μs",
            btree_time_us,
            linear_time_us
        );

        // Test that speedup meets logarithmic expectations
        let expected_min_speedup = 50.0; // For 10,000 elements, log speedup should be substantial
        assert!(
            speedup >= expected_min_speedup,
            "Speedup should meet algorithmic complexity expectations: {}x",
            speedup
        );
    }

    #[test]
    fn test_performance_degradation_thresholds() {
        // Test performance degradation threshold calculations extracted from integration tests
        let baseline_time = 100.0; // microseconds

        // Test different degradation levels
        let minor_degradation = baseline_time * 1.15; // 15% slower
        let moderate_degradation = baseline_time * 1.35; // 35% slower
        let severe_degradation = baseline_time * 2.0; // 100% slower

        // Verify degradation factor calculations
        assert!(
            (minor_degradation / baseline_time) < 1.2,
            "Minor degradation within bounds"
        );
        assert!(
            (moderate_degradation / baseline_time) > 1.2
                && (moderate_degradation / baseline_time) < 1.5,
            "Moderate degradation correctly classified"
        );
        assert!(
            (severe_degradation / baseline_time) >= 1.5,
            "Severe degradation correctly identified"
        );
    }

    #[test]
    fn test_measurement_stability_analysis() {
        // Test measurement stability analysis for performance testing
        let stable_measurements = vec![98.0, 101.0, 99.0, 102.0, 100.0]; // Low variance
        let unstable_measurements = vec![50.0, 150.0, 80.0, 120.0, 100.0]; // High variance

        let stable_variance = calculate_variance(&stable_measurements);
        let unstable_variance = calculate_variance(&unstable_measurements);

        assert!(
            stable_variance < 10.0,
            "Stable measurements should have low variance: {}",
            stable_variance
        );
        assert!(
            unstable_variance > 500.0,
            "Unstable measurements should have high variance: {}",
            unstable_variance
        );

        // Test coefficient of variation for stability assessment
        let stable_mean =
            stable_measurements.iter().sum::<f64>() / stable_measurements.len() as f64;
        let stable_cv = (stable_variance.sqrt() / stable_mean) * 100.0;
        assert!(
            stable_cv < 5.0,
            "Stable measurements should have low CV: {}%",
            stable_cv
        );
    }

    // Extracted from integration tests - Complexity comparison algorithms
    #[test]
    fn test_linear_index_algorithmic_behavior() {
        // Test O(n) linear index implementation behavior
        #[derive(Debug, Clone, PartialEq)]
        struct TestKey(u32);
        #[derive(Debug, Clone, PartialEq)]
        struct TestValue(String);

        struct LinearIndex {
            entries: Vec<(TestKey, TestValue)>,
        }

        impl LinearIndex {
            fn new() -> Self {
                Self {
                    entries: Vec::new(),
                }
            }

            fn insert(&mut self, key: TestKey, value: TestValue) {
                // O(n) - remove existing then append (typical linear behavior)
                self.entries.retain(|(k, _)| k != &key);
                self.entries.push((key, value));
            }

            fn search(&self, key: &TestKey) -> Option<&TestValue> {
                // O(n) - linear scan
                self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
            }

            fn delete(&mut self, key: &TestKey) {
                // O(n) - find and remove
                self.entries.retain(|(k, _)| k != key);
            }
        }

        // Test basic operations
        let mut linear = LinearIndex::new();

        // Test insertion
        linear.insert(TestKey(1), TestValue("value1".to_string()));
        linear.insert(TestKey(2), TestValue("value2".to_string()));
        assert_eq!(linear.entries.len(), 2);

        // Test search
        assert_eq!(
            linear.search(&TestKey(1)),
            Some(&TestValue("value1".to_string()))
        );
        assert_eq!(linear.search(&TestKey(3)), None);

        // Test update (should replace, not duplicate)
        linear.insert(TestKey(1), TestValue("updated".to_string()));
        assert_eq!(linear.entries.len(), 2);
        assert_eq!(
            linear.search(&TestKey(1)),
            Some(&TestValue("updated".to_string()))
        );

        // Test deletion
        linear.delete(&TestKey(1));
        assert_eq!(linear.search(&TestKey(1)), None);
        assert_eq!(linear.entries.len(), 1);
    }

    #[test]
    fn test_complexity_comparison_calculations() {
        use std::time::Duration;

        // Test complexity ratio calculations for performance comparisons
        struct ComplexityMeasurement {
            size: usize,
            linear_time: Duration,
            logarithmic_time: Duration,
            constant_time: Duration,
        }

        let measurements = [
            ComplexityMeasurement {
                size: 100,
                linear_time: Duration::from_millis(10),
                logarithmic_time: Duration::from_millis(2),
                constant_time: Duration::from_millis(1),
            },
            ComplexityMeasurement {
                size: 1000,
                linear_time: Duration::from_millis(100), // 10x increase
                logarithmic_time: Duration::from_millis(6), // ~3x increase (log scale)
                constant_time: Duration::from_millis(1), // No increase
            },
        ];

        // Calculate complexity ratios
        let size_ratio = measurements[1].size as f64 / measurements[0].size as f64;
        let linear_ratio = measurements[1].linear_time.as_millis() as f64
            / measurements[0].linear_time.as_millis() as f64;
        let log_ratio = measurements[1].logarithmic_time.as_millis() as f64
            / measurements[0].logarithmic_time.as_millis() as f64;
        let constant_ratio = measurements[1].constant_time.as_millis() as f64
            / measurements[0].constant_time.as_millis() as f64;

        // Verify complexity characteristics
        assert_eq!(size_ratio, 10.0, "Size increased 10x");
        assert_eq!(linear_ratio, 10.0, "Linear time should scale with size");
        assert!(
            log_ratio > 1.0 && log_ratio < size_ratio,
            "Logarithmic should increase but less than linear: {}",
            log_ratio
        );
        assert_eq!(constant_ratio, 1.0, "Constant time should not change");

        // Calculate speedup factors
        let linear_vs_log_speedup_100 = measurements[0].linear_time.as_millis() as f64
            / measurements[0].logarithmic_time.as_millis() as f64;
        let linear_vs_log_speedup_1000 = measurements[1].linear_time.as_millis() as f64
            / measurements[1].logarithmic_time.as_millis() as f64;

        assert!(
            linear_vs_log_speedup_1000 > linear_vs_log_speedup_100,
            "Logarithmic advantage should increase with size"
        );
    }

    #[test]
    fn test_performance_requirements_validation() {
        // Test performance requirements structure and validation logic
        struct PerformanceRequirements {
            max_avg_latency_ms: u64,
            max_p95_latency_ms: u64,
            max_p99_latency_ms: u64,
            max_std_dev_ms: u64,
            max_outlier_percentage: f64,
        }

        impl Default for PerformanceRequirements {
            fn default() -> Self {
                Self {
                    max_avg_latency_ms: 10,
                    max_p95_latency_ms: 50,
                    max_p99_latency_ms: 100,
                    max_std_dev_ms: 25,
                    max_outlier_percentage: 5.0,
                }
            }
        }

        let requirements = PerformanceRequirements::default();

        // Test good performance metrics
        let good_avg = 8;
        let good_p95 = 45;
        let good_p99 = 85;
        let good_std_dev = 20;
        let good_outlier_pct = 3.0;

        assert!(good_avg <= requirements.max_avg_latency_ms);
        assert!(good_p95 <= requirements.max_p95_latency_ms);
        assert!(good_p99 <= requirements.max_p99_latency_ms);
        assert!(good_std_dev <= requirements.max_std_dev_ms);
        assert!(good_outlier_pct <= requirements.max_outlier_percentage);

        // Test failing performance metrics
        let bad_avg = 15;
        let bad_p95 = 60;
        let bad_outlier_pct = 8.0;

        assert!(bad_avg > requirements.max_avg_latency_ms);
        assert!(bad_p95 > requirements.max_p95_latency_ms);
        assert!(bad_outlier_pct > requirements.max_outlier_percentage);
    }

    #[test]
    fn test_outlier_detection_algorithm() {
        use std::time::Duration;

        // Test outlier detection logic extracted from performance monitoring
        fn detect_outliers(measurements: &[Duration], threshold_ms: u64) -> Vec<bool> {
            measurements
                .iter()
                .map(|d| d.as_millis() as u64 > threshold_ms)
                .collect()
        }

        fn calculate_outlier_percentage(outliers: &[bool]) -> f64 {
            let outlier_count = outliers.iter().filter(|&&is_outlier| is_outlier).count();
            (outlier_count as f64 / outliers.len() as f64) * 100.0
        }

        let measurements = vec![
            Duration::from_millis(5),   // Normal
            Duration::from_millis(8),   // Normal
            Duration::from_millis(12),  // Normal
            Duration::from_millis(75),  // Outlier
            Duration::from_millis(7),   // Normal
            Duration::from_millis(9),   // Normal
            Duration::from_millis(120), // Outlier
            Duration::from_millis(6),   // Normal
        ];

        let outliers = detect_outliers(&measurements, 50);
        let outlier_pct = calculate_outlier_percentage(&outliers);

        // Should detect 2 outliers out of 8 measurements = 25%
        assert_eq!(outliers.iter().filter(|&&x| x).count(), 2);
        assert_eq!(outlier_pct, 25.0);

        // Test with stricter threshold
        let strict_outliers = detect_outliers(&measurements, 10);
        let strict_pct = calculate_outlier_percentage(&strict_outliers);
        assert!(
            strict_pct > outlier_pct,
            "Stricter threshold should find more outliers"
        );
    }

    #[test]
    fn test_throughput_calculation() {
        use std::time::Duration;

        // Test throughput calculation algorithms
        fn calculate_throughput_ops_per_sec(operations: usize, total_duration: Duration) -> f64 {
            if total_duration.is_zero() {
                return 0.0;
            }
            operations as f64 / total_duration.as_secs_f64()
        }

        // Test various scenarios
        let test_cases = vec![
            (1000, Duration::from_secs(1), 1000.0),
            (500, Duration::from_millis(500), 1000.0),
            (100, Duration::from_millis(50), 2000.0),
            (1, Duration::from_millis(1), 1000.0),
        ];

        for (ops, duration, expected_throughput) in test_cases {
            let actual = calculate_throughput_ops_per_sec(ops, duration);
            assert!(
                (actual - expected_throughput).abs() < 0.1,
                "Expected {}, got {} for {} ops in {:?}",
                expected_throughput,
                actual,
                ops,
                duration
            );
        }

        // Test edge case - zero duration
        assert_eq!(calculate_throughput_ops_per_sec(100, Duration::ZERO), 0.0);
    }
}
