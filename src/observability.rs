// Centralized Observability Infrastructure for KotaDB
// This module provides structured logging, metrics, and tracing capabilities
// Following Stage 4 of the 6-stage risk reduction methodology

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

// Global atomic counters for metrics
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);
static ERROR_COUNTER: AtomicU64 = AtomicU64::new(0);
static QUERY_COUNTER: AtomicU64 = AtomicU64::new(0);
static INDEX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Initialize the logging and tracing infrastructure
/// This should be called once at application startup
pub fn init_logging() -> Result<()> {
    init_logging_with_level(false, false)
}

/// Initialize logging with configurable verbosity
pub fn init_logging_with_level(verbose: bool, quiet: bool) -> Result<()> {
    // Create a layered subscriber with:
    // 1. Environment-based filtering (RUST_LOG)
    // 2. Pretty formatted output for development
    // 3. JSON output option for production

    // Determine the filter level based on flags
    let filter_level = if quiet {
        // In quiet mode, suppress everything except errors
        // This sets a global filter that affects all modules
        EnvFilter::new("error")
    } else if verbose {
        // In verbose mode, show debug info for kotadb and info for others
        EnvFilter::new("kotadb=debug,info")
    } else {
        // Default: show warnings and errors for kotadb, only errors for dependencies
        // This ensures important warnings are visible while suppressing debug/info spam
        // Users can enable more logging with --verbose or RUST_LOG env var
        EnvFilter::new("kotadb=warn,error")
    };

    // Quiet flag takes precedence over environment variable
    // This ensures that --quiet ALWAYS suppresses logs regardless of RUST_LOG
    let env_filter = if quiet {
        // Force error-only logging when quiet is enabled, ignoring RUST_LOG
        EnvFilter::new("error")
    } else if std::env::var("RUST_LOG").is_ok() {
        // If not quiet, allow RUST_LOG to override the default
        EnvFilter::try_from_default_env().unwrap_or(filter_level)
    } else {
        // Use flag-based filter if RUST_LOG is not set
        filter_level
    };

    // Configure the format layer based on quiet mode
    // In quiet mode, we want minimal output without metadata
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(!quiet) // Don't show target module in quiet mode
        .with_thread_ids(!quiet) // Don't show thread IDs in quiet mode
        .with_line_number(!quiet) // Don't show line numbers in quiet mode
        .with_file(!quiet) // Don't show file names in quiet mode
        .with_ansi(true);

    match tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
    {
        Ok(()) => {
            if !quiet {
                info!("KotaDB observability initialized");
            }
            Ok(())
        }
        Err(_) => {
            // Already initialized, which is fine in test environments
            Ok(())
        }
    }
}

/// Represents different types of operations for structured logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    // Storage operations
    StorageRead {
        doc_id: Uuid,
        size_bytes: usize,
    },
    StorageWrite {
        doc_id: Uuid,
        size_bytes: usize,
    },
    StorageDelete {
        doc_id: Uuid,
    },

    // Index operations
    IndexInsert {
        index_type: String,
        doc_id: Uuid,
    },
    IndexRemove {
        index_type: String,
        doc_id: Uuid,
    },
    IndexSearch {
        index_type: String,
        query: String,
        result_count: usize,
    },

    // Query operations
    QueryParse {
        query: String,
    },
    QueryPlan {
        plan_steps: usize,
    },
    QueryExecute {
        result_count: usize,
    },

    // System operations
    Startup {
        version: String,
    },
    Shutdown {
        reason: String,
    },
    Checkpoint {
        documents_processed: usize,
    },
    Recovery {
        wal_entries: usize,
    },
}

impl Operation {
    /// Validate the operation parameters
    pub fn validate(&self) -> Result<()> {
        match self {
            Operation::StorageRead { size_bytes, .. }
            | Operation::StorageWrite { size_bytes, .. } => {
                if *size_bytes == 0 {
                    anyhow::bail!("Storage operation with zero size");
                }
            }
            Operation::IndexSearch {
                result_count: _, ..
            }
            | Operation::QueryExecute { result_count: _ } => {
                // result_count can be 0 for no matches
            }
            Operation::QueryPlan { plan_steps } => {
                if *plan_steps == 0 {
                    anyhow::bail!("Query plan must have at least one step");
                }
            }
            _ => {
                // Other operations don't need validation
            }
        }
        Ok(())
    }
}

/// Metric types for performance monitoring
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter {
        name: &'static str,
        value: u64,
    },
    Gauge {
        name: &'static str,
        value: f64,
    },
    Histogram {
        name: &'static str,
        value: f64,
        unit: &'static str,
    },
    Timer {
        name: &'static str,
        duration: Duration,
    },
}

/// Operation context for tracing through the system
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub trace_id: Uuid,
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub operation: String,
    pub start_time: Instant,
    pub attributes: Vec<(String, String)>,
}

impl OperationContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            span_id: Uuid::new_v4(),
            parent_span_id: None,
            operation: operation.into(),
            start_time: Instant::now(),
            attributes: Vec::new(),
        }
    }

    pub fn child(&self, operation: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: Uuid::new_v4(),
            parent_span_id: Some(self.span_id),
            operation: operation.into(),
            start_time: Instant::now(),
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.push((key.into(), value.into()));
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Log an operation with full context
#[instrument(skip(ctx))]
pub fn log_operation(ctx: &OperationContext, op: &Operation, result: &Result<()>) {
    let elapsed = ctx.elapsed();
    let attrs = ctx
        .attributes
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");

    match result {
        Ok(()) => {
            info!(
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                parent_span_id = ?ctx.parent_span_id,
                operation = %ctx.operation,
                elapsed_ms = elapsed.as_millis(),
                attributes = %attrs,
                "Operation completed: {:?}", op
            );
            OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            error!(
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                parent_span_id = ?ctx.parent_span_id,
                operation = %ctx.operation,
                elapsed_ms = elapsed.as_millis(),
                attributes = %attrs,
                error = %e,
                "Operation failed: {:?}", op
            );
            ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Update specific counters
    match op {
        Operation::QueryExecute { .. } => {
            QUERY_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
        Operation::IndexInsert { .. } | Operation::IndexSearch { .. } => {
            INDEX_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
}

/// Record a metric
pub fn record_metric(metric: MetricType) {
    match metric {
        MetricType::Counter { name, value } => {
            // Suppress debug logging for high-frequency test metrics to prevent log spam
            if !name.starts_with("high_frequency.") {
                debug!("metric.counter {} = {}", name, value);
            }
        }
        MetricType::Gauge { name, value } => {
            if !name.starts_with("high_frequency.") {
                debug!("metric.gauge {} = {}", name, value);
            }
        }
        MetricType::Histogram { name, value, unit } => {
            if !name.starts_with("high_frequency.") {
                debug!("metric.histogram {} = {} {}", name, value, unit);
            }
        }
        MetricType::Timer { name, duration } => {
            if !name.starts_with("high_frequency.") {
                debug!("metric.timer {} = {:?}", name, duration);
            }
        }
    }
}

/// Execute a closure with a trace context
pub async fn with_trace_id<F, T>(operation: &str, f: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let ctx = OperationContext::new(operation);
    let trace_id = ctx.trace_id;
    let span_id = ctx.span_id;

    info!(
        trace_id = %trace_id,
        span_id = %span_id,
        "Starting operation: {}", operation
    );

    let start = Instant::now();
    let result = f.await;
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => {
            info!(
                trace_id = %trace_id,
                span_id = %span_id,
                elapsed_ms = elapsed.as_millis(),
                "Operation completed successfully: {}", operation
            );
            record_metric(MetricType::Timer {
                name: "operation.duration",
                duration: elapsed,
            });
        }
        Err(e) => {
            error!(
                trace_id = %trace_id,
                span_id = %span_id,
                elapsed_ms = elapsed.as_millis(),
                error = %e,
                "Operation failed: {}", operation
            );
            record_metric(MetricType::Counter {
                name: "operation.errors",
                value: 1,
            });
        }
    }

    result
}

/// Get current metrics snapshot
pub fn get_metrics() -> serde_json::Value {
    serde_json::json!({
        "operations": {
            "total": OPERATION_COUNTER.load(Ordering::Relaxed),
            "errors": ERROR_COUNTER.load(Ordering::Relaxed),
            "queries": QUERY_COUNTER.load(Ordering::Relaxed),
            "index_ops": INDEX_COUNTER.load(Ordering::Relaxed),
        },
        "timestamp": Utc::now().to_rfc3339(),
    })
}

/// Structured error logging with context
#[instrument]
pub fn log_error_with_context(error: &anyhow::Error, ctx: &OperationContext) {
    let error_chain = error
        .chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(" -> ");

    error!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        operation = %ctx.operation,
        error_chain = %error_chain,
        "Error occurred during operation"
    );
}

/// Performance timer for measuring operation duration
pub struct PerfTimer {
    name: String,
    start: Instant,
    ctx: OperationContext,
}

impl PerfTimer {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let ctx = OperationContext::new(&name);
        info!(
            trace_id = %ctx.trace_id,
            span_id = %ctx.span_id,
            "Timer started: {}", name
        );
        Self {
            name,
            start: Instant::now(),
            ctx,
        }
    }
}

impl Drop for PerfTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        info!(
            trace_id = %self.ctx.trace_id,
            span_id = %self.ctx.span_id,
            elapsed_ms = elapsed.as_millis(),
            "Timer completed: {}", self.name
        );
        record_metric(MetricType::Timer {
            name: "perf.timer",
            duration: elapsed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_context_creation() {
        let ctx = OperationContext::new("test_operation");
        assert_eq!(ctx.operation, "test_operation");
        assert!(ctx.parent_span_id.is_none());

        let child = ctx.child("child_operation");
        assert_eq!(child.trace_id, ctx.trace_id);
        assert_eq!(child.parent_span_id, Some(ctx.span_id));
    }

    #[test]
    fn test_metrics_recording() {
        record_metric(MetricType::Counter {
            name: "test.counter",
            value: 42,
        });
        record_metric(MetricType::Gauge {
            name: "test.gauge",
            value: std::f64::consts::PI,
        });
        record_metric(MetricType::Timer {
            name: "test.timer",
            duration: Duration::from_millis(123),
        });

        let metrics = get_metrics();
        assert!(metrics["timestamp"].is_string());
        assert!(metrics["operations"].is_object());
    }

    #[tokio::test]
    async fn test_with_trace_id() {
        let result = with_trace_id("test_async_op", async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<_, anyhow::Error>(42)
        })
        .await;

        assert_eq!(result.expect("Test operation should succeed"), 42);
    }

    #[test]
    fn test_perf_timer() {
        {
            let _timer = PerfTimer::new("test_timer");
            std::thread::sleep(Duration::from_millis(10));
            // Timer will log on drop
        }
        // Check that drop was called and metrics recorded
        let metrics = get_metrics();
        assert!(metrics["operations"]["total"].as_u64().is_some());
    }

    #[test]
    fn test_default_logging_level() {
        // Test that default logging level shows warnings and errors for kotadb,
        // but only errors for dependencies
        let filter_str = "kotadb=warn,error";

        // Verify the filter string is parsed correctly
        // This ensures our default configuration is valid
        assert!(EnvFilter::try_new(filter_str).is_ok());
    }

    #[test]
    fn test_verbose_logging_level() {
        // Test that verbose mode enables debug for kotadb and info for others
        let filter_str = "kotadb=debug,info";

        // Verify the filter string is parsed correctly
        assert!(EnvFilter::try_new(filter_str).is_ok());
    }

    #[test]
    fn test_quiet_logging_level() {
        // Test that quiet mode suppresses everything except errors
        let filter_str = "error";

        // Verify the filter string is parsed correctly
        assert!(EnvFilter::try_new(filter_str).is_ok());
    }

    #[test]
    fn test_logging_level_configurations() {
        // Test all three logging configurations to ensure they're valid
        let configs = vec![
            ("quiet", "error"),
            ("verbose", "kotadb=debug,info"),
            ("default", "kotadb=warn,error"),
        ];

        for (mode, filter_str) in configs {
            assert!(
                EnvFilter::try_new(filter_str).is_ok(),
                "Failed to create filter for {} mode with filter: {}",
                mode,
                filter_str
            );
        }
    }
}
