// Performance Metrics Collection - Stage 4: Observability
// Enhanced metrics infrastructure for performance monitoring and alerting

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use serde::{Serialize, Deserialize};
use crate::contracts::performance::{PerformanceMeasurement, ComplexityClass};
use crate::pure::performance::{PerformanceStats, GrowthAnalysis, RegressionReport};

/// Performance metrics collector
#[derive(Debug)]
pub struct PerformanceCollector {
    measurements: Arc<RwLock<Vec<PerformanceMeasurement>>>,
    operation_timers: Arc<Mutex<HashMap<String, Instant>>>,
    configuration: PerformanceConfig,
}

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub max_measurements_per_operation: usize,
    pub measurement_retention_hours: u64,
    pub enable_detailed_timing: bool,
    pub enable_memory_tracking: bool,
    pub regression_threshold: f64,  // Factor triggering alerts
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_measurements_per_operation: 1000,
            measurement_retention_hours: 24,
            enable_detailed_timing: true,
            enable_memory_tracking: true,
            regression_threshold: 1.3,  // 30% slowdown triggers alert
        }
    }
}

/// Real-time performance dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDashboard {
    pub timestamp: SystemTime,
    pub operations: HashMap<String, OperationMetrics>,
    pub system_metrics: SystemMetrics,
    pub alerts: Vec<PerformanceAlert>,
}

/// Metrics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub operation_name: String,
    pub total_invocations: u64,
    pub current_stats: PerformanceStats,
    pub complexity_analysis: GrowthAnalysis,
    pub last_regression_check: SystemTime,
    pub avg_memory_usage: usize,
    pub peak_memory_usage: usize,
}

/// System-wide performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_operations: u64,
    pub operations_per_second: f64,
    pub avg_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub memory_efficiency: f64,
    pub active_operations: u32,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub alert_type: AlertType,
    pub operation: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: SystemTime,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    Regression,
    Threshold,
    ComplexityAnomaly,
    MemoryLeak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl PerformanceCollector {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            measurements: Arc::new(RwLock::new(Vec::new())),
            operation_timers: Arc::new(Mutex::new(HashMap::new())),
            configuration: config,
        }
    }
    
    /// Start timing an operation
    pub fn start_operation(&self, operation_id: String) {
        if self.configuration.enable_detailed_timing {
            let mut timers = self.operation_timers.lock().unwrap();
            timers.insert(operation_id, Instant::now());
        }
    }
    
    /// End timing and record measurement
    pub fn end_operation(
        &self,
        operation_id: String,
        operation_name: String,
        input_size: usize,
        memory_used: Option<usize>,
    ) {
        if !self.configuration.enable_detailed_timing {
            return;
        }
        
        let duration = {
            let mut timers = self.operation_timers.lock().unwrap();
            timers.remove(&operation_id)
                .map(|start| start.elapsed())
                .unwrap_or(Duration::ZERO)
        };
        
        if duration > Duration::ZERO {
            let throughput = 1.0 / duration.as_secs_f64();
            let measurement = PerformanceMeasurement {
                operation: operation_name,
                input_size,
                duration,
                throughput_ops_per_sec: throughput,
                memory_used_bytes: memory_used.unwrap_or(0),
                timestamp: chrono::Utc::now(),
            };
            
            self.record_measurement(measurement);
        }
    }
    
    /// Record a performance measurement
    pub fn record_measurement(&self, measurement: PerformanceMeasurement) {
        let mut measurements = self.measurements.write().unwrap();
        
        // Cleanup old measurements
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(self.configuration.measurement_retention_hours as i64);
        measurements.retain(|m| m.timestamp > cutoff);
        
        // Limit measurements per operation
        let operation_count = measurements.iter()
            .filter(|m| m.operation == measurement.operation)
            .count();
        
        if operation_count >= self.configuration.max_measurements_per_operation {
            // Remove oldest measurement for this operation
            if let Some(pos) = measurements.iter().position(|m| m.operation == measurement.operation) {
                measurements.remove(pos);
            }
        }
        
        measurements.push(measurement);
    }
    
    /// Generate current performance dashboard
    pub fn generate_dashboard(&self) -> PerformanceDashboard {
        let measurements = self.measurements.read().unwrap();
        
        // Group measurements by operation
        let mut operation_groups: HashMap<String, Vec<&PerformanceMeasurement>> = HashMap::new();
        for measurement in measurements.iter() {
            operation_groups
                .entry(measurement.operation.clone())
                .or_insert_with(Vec::new)
                .push(measurement);
        }
        
        // Calculate metrics for each operation
        let mut operations = HashMap::new();
        for (op_name, op_measurements) in operation_groups {
            let durations: Vec<Duration> = op_measurements.iter().map(|m| m.duration).collect();
            let current_stats = crate::pure::performance::calculate_performance_stats(&durations);
            
            let sizes: Vec<usize> = op_measurements.iter().map(|m| m.input_size).collect();
            let times: Vec<Duration> = op_measurements.iter().map(|m| m.duration).collect();
            let complexity_analysis = crate::pure::performance::calculate_complexity_factor(&sizes, &times);
            
            let avg_memory = if op_measurements.is_empty() {
                0
            } else {
                op_measurements.iter().map(|m| m.memory_used_bytes).sum::<usize>() / op_measurements.len()
            };
            
            let peak_memory = op_measurements.iter().map(|m| m.memory_used_bytes).max().unwrap_or(0);
            
            operations.insert(op_name.clone(), OperationMetrics {
                operation_name: op_name,
                total_invocations: op_measurements.len() as u64,
                current_stats,
                complexity_analysis,
                last_regression_check: SystemTime::now(),
                avg_memory_usage: avg_memory,
                peak_memory_usage: peak_memory,
            });
        }
        
        // Calculate system metrics
        let all_durations: Vec<Duration> = measurements.iter().map(|m| m.duration).collect();
        let system_stats = crate::pure::performance::calculate_performance_stats(&all_durations);
        
        let total_operations = measurements.len() as u64;
        let operations_per_second = if !measurements.is_empty() {
            let time_span = measurements.iter()
                .map(|m| m.timestamp)
                .max()
                .unwrap_or(chrono::Utc::now())
                .signed_duration_since(
                    measurements.iter()
                        .map(|m| m.timestamp)
                        .min()
                        .unwrap_or(chrono::Utc::now())
                )
                .num_seconds() as f64;
            
            if time_span > 0.0 {
                total_operations as f64 / time_span
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        let system_metrics = SystemMetrics {
            total_operations,
            operations_per_second,
            avg_response_time: system_stats.mean,
            p95_response_time: system_stats.percentile_95,
            p99_response_time: system_stats.percentile_99,
            memory_efficiency: 0.75, // Placeholder - would calculate from real data
            active_operations: self.operation_timers.lock().unwrap().len() as u32,
        };
        
        // Generate alerts
        let alerts = self.check_for_alerts(&operations);
        
        PerformanceDashboard {
            timestamp: SystemTime::now(),
            operations,
            system_metrics,
            alerts,
        }
    }
    
    /// Check for performance alerts
    fn check_for_alerts(&self, operations: &HashMap<String, OperationMetrics>) -> Vec<PerformanceAlert> {
        let mut alerts = Vec::new();
        
        for (op_name, metrics) in operations {
            // Check for complexity anomalies
            if matches!(metrics.complexity_analysis.detected_complexity, ComplexityClass::Quadratic | ComplexityClass::Unknown) {
                alerts.push(PerformanceAlert {
                    alert_type: AlertType::ComplexityAnomaly,
                    operation: op_name.clone(),
                    message: format!("Operation {} showing unexpected complexity: {:?}", 
                                   op_name, metrics.complexity_analysis.detected_complexity),
                    severity: AlertSeverity::Warning,
                    timestamp: SystemTime::now(),
                    details: HashMap::from([
                        ("confidence".to_string(), format!("{:.2}", metrics.complexity_analysis.confidence)),
                        ("r_squared".to_string(), format!("{:.3}", metrics.complexity_analysis.r_squared)),
                    ]),
                });
            }
            
            // Check for performance thresholds
            if metrics.current_stats.p95 > Duration::from_millis(100) {
                alerts.push(PerformanceAlert {
                    alert_type: AlertType::Threshold,
                    operation: op_name.clone(),
                    message: format!("Operation {} 95th percentile exceeds 100ms: {:?}", 
                                   op_name, metrics.current_stats.p95),
                    severity: AlertSeverity::Critical,
                    timestamp: SystemTime::now(),
                    details: HashMap::from([
                        ("p95_time".to_string(), format!("{:?}", metrics.current_stats.p95)),
                        ("threshold".to_string(), "100ms".to_string()),
                    ]),
                });
            }
            
            // Check for memory usage
            if metrics.peak_memory_usage > 100 * 1024 * 1024 { // 100MB
                alerts.push(PerformanceAlert {
                    alert_type: AlertType::MemoryLeak,
                    operation: op_name.clone(),
                    message: format!("Operation {} peak memory usage: {} bytes", 
                                   op_name, metrics.peak_memory_usage),
                    severity: AlertSeverity::Warning,
                    timestamp: SystemTime::now(),
                    details: HashMap::from([
                        ("peak_memory".to_string(), format!("{}", metrics.peak_memory_usage)),
                        ("avg_memory".to_string(), format!("{}", metrics.avg_memory_usage)),
                    ]),
                });
            }
        }
        
        alerts
    }
    
    /// Get measurements for a specific operation
    pub fn get_operation_measurements(&self, operation: &str) -> Vec<PerformanceMeasurement> {
        let measurements = self.measurements.read().unwrap();
        measurements.iter()
            .filter(|m| m.operation == operation)
            .cloned()
            .collect()
    }
    
    /// Export metrics to JSON
    pub fn export_json(&self) -> String {
        let dashboard = self.generate_dashboard();
        serde_json::to_string_pretty(&dashboard).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Export metrics to Prometheus format
    pub fn export_prometheus(&self) -> String {
        let dashboard = self.generate_dashboard();
        let mut prometheus = String::new();
        
        // System metrics
        prometheus.push_str(&format!("kotadb_total_operations {}\n", dashboard.system_metrics.total_operations));
        prometheus.push_str(&format!("kotadb_operations_per_second {}\n", dashboard.system_metrics.operations_per_second));
        prometheus.push_str(&format!("kotadb_avg_response_time_seconds {}\n", dashboard.system_metrics.avg_response_time.as_secs_f64()));
        prometheus.push_str(&format!("kotadb_p95_response_time_seconds {}\n", dashboard.system_metrics.p95_response_time.as_secs_f64()));
        prometheus.push_str(&format!("kotadb_memory_efficiency {}\n", dashboard.system_metrics.memory_efficiency));
        
        // Operation-specific metrics
        for (op_name, metrics) in &dashboard.operations {
            let safe_name = op_name.replace("-", "_").replace(" ", "_");
            prometheus.push_str(&format!("kotadb_operation_invocations{{operation=\"{}\"}} {}\n", 
                                       op_name, metrics.total_invocations));
            prometheus.push_str(&format!("kotadb_operation_avg_time_seconds{{operation=\"{}\"}} {}\n", 
                                       op_name, metrics.current_stats.mean.as_secs_f64()));
            prometheus.push_str(&format!("kotadb_operation_p95_time_seconds{{operation=\"{}\"}} {}\n", 
                                       op_name, metrics.current_stats.percentile_95.as_secs_f64()));
            prometheus.push_str(&format!("kotadb_operation_memory_bytes{{operation=\"{}\"}} {}\n", 
                                       op_name, metrics.avg_memory_usage));
        }
        
        prometheus
    }
}

/// Performance monitoring helper macros
#[macro_export]
macro_rules! measure_performance {
    ($collector:expr, $operation:expr, $input_size:expr, $code:block) => {{
        let operation_id = format!("{}_{}", $operation, uuid::Uuid::new_v4());
        $collector.start_operation(operation_id.clone());
        
        let result = $code;
        
        $collector.end_operation(
            operation_id, 
            $operation.to_string(), 
            $input_size, 
            None
        );
        
        result
    }};
}

#[macro_export]
macro_rules! measure_performance_with_memory {
    ($collector:expr, $operation:expr, $input_size:expr, $memory:expr, $code:block) => {{
        let operation_id = format!("{}_{}", $operation, uuid::Uuid::new_v4());
        $collector.start_operation(operation_id.clone());
        
        let result = $code;
        
        $collector.end_operation(
            operation_id, 
            $operation.to_string(), 
            $input_size, 
            Some($memory)
        );
        
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_performance_collector() {
        let config = PerformanceConfig::default();
        let collector = PerformanceCollector::new(config);
        
        // Record some test measurements
        collector.start_operation("test_op_1".to_string());
        thread::sleep(Duration::from_millis(10));
        collector.end_operation(
            "test_op_1".to_string(),
            "test_operation".to_string(),
            1000,
            Some(1024),
        );
        
        let dashboard = collector.generate_dashboard();
        
        assert!(!dashboard.operations.is_empty());
        assert!(dashboard.system_metrics.total_operations > 0);
    }
    
    #[test]
    fn test_metrics_export() {
        let config = PerformanceConfig::default();
        let collector = PerformanceCollector::new(config);
        
        let measurement = PerformanceMeasurement {
            operation: "test_export".to_string(),
            input_size: 1000,
            duration: Duration::from_micros(500),
            throughput_ops_per_sec: 2000.0,
            memory_used_bytes: 2048,
            timestamp: chrono::Utc::now(),
        };
        
        collector.record_measurement(measurement);
        
        let json_export = collector.export_json();
        assert!(!json_export.is_empty());
        
        let prometheus_export = collector.export_prometheus();
        assert!(prometheus_export.contains("kotadb_"));
    }
}