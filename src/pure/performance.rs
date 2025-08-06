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
    let mut growth_factors = Vec::new();
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
}
