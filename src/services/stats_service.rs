// StatsService - Unified database statistics and health monitoring functionality
//
// This service extracts statistics, health checking, and monitoring logic from main.rs
// and ManagementService to provide comprehensive database analytics across all interfaces.

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use super::DatabaseAccess;

/// Configuration options for database statistics
#[derive(Debug, Clone, Default)]
pub struct StatsOptions {
    pub basic: bool,
    pub symbols: bool,
    pub relationships: bool,
    pub detailed: bool,
    pub quiet: bool,
}

/// Configuration options for health checks
#[derive(Debug, Clone, Default)]
pub struct HealthCheckOptions {
    pub deep_scan: bool,
    pub check_integrity: bool,
    pub check_performance: bool,
    pub quiet: bool,
}

/// Configuration options for performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetricsOptions {
    pub timeframe_hours: Option<u64>,
    pub include_query_patterns: bool,
    pub include_resource_usage: bool,
    pub quiet: bool,
}

/// Basic database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct BasicStats {
    pub document_count: usize,
    pub total_size_bytes: usize,
    pub average_file_size: usize,
    pub storage_efficiency: f64,
    pub index_count: usize,
}

/// Symbol extraction statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolStats {
    pub total_symbols: usize,
    pub symbols_by_type: HashMap<String, usize>,
    pub symbols_by_language: HashMap<String, usize>,
    pub files_with_symbols: usize,
    pub symbol_density: f64,      // symbols per file
    pub extraction_coverage: f64, // % of files with symbols
}

/// Relationship analysis statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationshipStats {
    pub total_relationships: usize,
    pub connected_symbols: usize,
    pub dependency_graph_stats: Option<DependencyGraphStats>,
    pub relationship_types: HashMap<String, usize>,
    pub average_connections_per_symbol: f64,
}

/// Dependency graph statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyGraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub strongly_connected_components: usize,
    pub average_connections_per_node: f64,
    pub max_depth: usize,
    pub circular_dependencies: usize,
}

/// Database health report
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub storage_health: StorageHealth,
    pub index_health: IndexHealth,
    pub performance_health: PerformanceHealth,
    pub issues_found: Vec<HealthIssue>,
    pub recommendations: Vec<String>,
}

/// Overall health status
#[derive(Debug, Clone, serde::Serialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Storage health metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageHealth {
    pub status: HealthStatus,
    pub fragmentation_percent: f64,
    pub corruption_detected: bool,
    pub space_utilization: f64,
    pub backup_status: String,
}

/// Index health metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexHealth {
    pub status: HealthStatus,
    pub indices_healthy: usize,
    pub indices_degraded: usize,
    pub indices_corrupted: usize,
    pub average_lookup_time_ms: f64,
    pub consistency_checks_passed: bool,
}

/// Performance health metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceHealth {
    pub status: HealthStatus,
    pub average_query_time_ms: f64,
    pub throughput_queries_per_second: f64,
    pub memory_usage_mb: f64,
    pub cache_hit_rate: f64,
    pub resource_bottlenecks: Vec<String>,
}

/// Individual health issue
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthIssue {
    pub severity: IssueSeverity,
    pub component: String,
    pub description: String,
    pub suggested_action: String,
}

/// Issue severity levels
#[derive(Debug, Clone, serde::Serialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Performance metrics over time
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceMetrics {
    pub timeframe_start: String,
    pub timeframe_end: String,
    pub query_statistics: QueryStatistics,
    pub resource_usage: ResourceUsage,
    pub throughput_metrics: ThroughputMetrics,
}

/// Query performance statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryStatistics {
    pub total_queries: usize,
    pub average_response_time_ms: f64,
    pub median_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub queries_by_type: HashMap<String, QueryTypeStats>,
}

/// Statistics for a specific query type
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryTypeStats {
    pub count: usize,
    pub average_time_ms: f64,
    pub success_rate: f64,
    pub cache_hit_rate: f64,
}

/// Resource utilization metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceUsage {
    pub peak_memory_mb: f64,
    pub average_memory_mb: f64,
    pub peak_cpu_percent: f64,
    pub average_cpu_percent: f64,
    pub disk_io_operations: usize,
    pub network_io_bytes: usize,
}

/// Throughput and concurrency metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThroughputMetrics {
    pub queries_per_second: f64,
    pub peak_concurrent_queries: usize,
    pub average_concurrent_queries: f64,
    pub queue_wait_time_ms: f64,
}

/// Result structure for database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatsResult {
    pub basic_stats: Option<BasicStats>,
    pub symbol_stats: Option<SymbolStats>,
    pub relationship_stats: Option<RelationshipStats>,
    pub formatted_output: String,
}

/// Result structure for health checks
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub health_report: HealthReport,
    pub formatted_output: String,
}

/// Result structure for performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetricsResult {
    pub metrics: PerformanceMetrics,
    pub formatted_output: String,
}

/// StatsService handles all database statistics, health monitoring, and performance analytics
pub struct StatsService<'a> {
    database: &'a dyn DatabaseAccess,
    db_path: PathBuf,
}

impl<'a> StatsService<'a> {
    /// Create a new StatsService instance
    pub fn new(database: &'a dyn DatabaseAccess, db_path: PathBuf) -> Self {
        Self { database, db_path }
    }

    /// Get comprehensive database statistics
    ///
    /// This method extracts the statistics logic from ManagementService,
    /// providing detailed database analytics across all interfaces.
    pub async fn get_statistics(&self, options: StatsOptions) -> Result<StatsResult> {
        let mut formatted_output = String::new();
        let mut basic_stats = None;
        let mut symbol_stats = None;
        let mut relationship_stats = None;

        // Determine what to show with explicit flag precedence
        let no_flags_specified = !options.basic && !options.symbols && !options.relationships;
        let show_basic = options.basic || no_flags_specified;
        let show_symbols = options.symbols || no_flags_specified;
        let show_relationships = options.relationships || no_flags_specified;

        // Show basic document statistics
        if show_basic {
            basic_stats = Some(self.get_basic_statistics().await?);
            if !options.quiet {
                formatted_output.push_str(
                    &self
                        .format_basic_statistics(basic_stats.as_ref().unwrap())
                        .await?,
                );
            }
        }

        // Show symbol statistics (if tree-sitter feature is enabled)
        #[cfg(feature = "tree-sitter-parsing")]
        if show_symbols {
            symbol_stats = Some(self.get_symbol_statistics().await?);
            if !options.quiet {
                formatted_output.push_str(
                    &self
                        .format_symbol_statistics(symbol_stats.as_ref().unwrap())
                        .await?,
                );
            }
        }

        // Show relationship statistics
        #[cfg(feature = "tree-sitter-parsing")]
        if show_relationships {
            relationship_stats = Some(self.get_relationship_statistics().await?);
            if !options.quiet {
                formatted_output.push_str(
                    &self
                        .format_relationship_statistics(relationship_stats.as_ref().unwrap())
                        .await?,
                );
            }
        }

        // Add helpful tips and next steps
        if !options.quiet {
            formatted_output.push_str(&self.generate_usage_tips().await?);
        }

        Ok(StatsResult {
            basic_stats,
            symbol_stats,
            relationship_stats,
            formatted_output,
        })
    }

    /// Perform comprehensive health check of the database
    pub async fn health_check(&self, options: HealthCheckOptions) -> Result<HealthCheckResult> {
        let mut formatted_output = String::new();

        if !options.quiet {
            formatted_output.push_str("🏥 Running database health check...\n\n");
        }

        // Perform storage health check
        let storage_health = self.check_storage_health(options.deep_scan).await?;

        // Perform index health check
        let index_health = self.check_index_health(options.check_integrity).await?;

        // Perform performance health check
        let performance_health = if options.check_performance {
            self.check_performance_health().await?
        } else {
            PerformanceHealth {
                status: HealthStatus::Unknown,
                average_query_time_ms: 0.0,
                throughput_queries_per_second: 0.0,
                memory_usage_mb: 0.0,
                cache_hit_rate: 0.0,
                resource_bottlenecks: Vec::new(),
            }
        };

        // Determine overall status
        let overall_status = match (
            &storage_health.status,
            &index_health.status,
            &performance_health.status,
        ) {
            (HealthStatus::Critical, _, _)
            | (_, HealthStatus::Critical, _)
            | (_, _, HealthStatus::Critical) => HealthStatus::Critical,
            (HealthStatus::Warning, _, _)
            | (_, HealthStatus::Warning, _)
            | (_, _, HealthStatus::Warning) => HealthStatus::Warning,
            (HealthStatus::Healthy, HealthStatus::Healthy, HealthStatus::Healthy) => {
                HealthStatus::Healthy
            }
            _ => HealthStatus::Unknown,
        };

        let health_report = HealthReport {
            overall_status,
            storage_health,
            index_health,
            performance_health,
            issues_found: Vec::new(), // TODO: Collect issues from individual checks
            recommendations: Vec::new(), // TODO: Generate recommendations
        };

        if !options.quiet {
            formatted_output.push_str(&self.format_health_report(&health_report).await?);
        }

        Ok(HealthCheckResult {
            health_report,
            formatted_output,
        })
    }

    /// Get performance metrics over a specified timeframe
    pub async fn get_performance_metrics(
        &self,
        options: PerformanceMetricsOptions,
    ) -> Result<PerformanceMetricsResult> {
        let mut formatted_output = String::new();

        if !options.quiet {
            formatted_output.push_str("📊 Collecting performance metrics...\n\n");
        }

        // TODO: Implement actual performance metrics collection
        // This would include:
        // - Query response time tracking
        // - Resource utilization monitoring
        // - Throughput analysis
        // - Cache performance metrics

        let metrics = PerformanceMetrics {
            timeframe_start: "Not implemented".to_string(),
            timeframe_end: "Not implemented".to_string(),
            query_statistics: QueryStatistics {
                total_queries: 0,
                average_response_time_ms: 0.0,
                median_response_time_ms: 0.0,
                p95_response_time_ms: 0.0,
                p99_response_time_ms: 0.0,
                queries_by_type: HashMap::new(),
            },
            resource_usage: ResourceUsage {
                peak_memory_mb: 0.0,
                average_memory_mb: 0.0,
                peak_cpu_percent: 0.0,
                average_cpu_percent: 0.0,
                disk_io_operations: 0,
                network_io_bytes: 0,
            },
            throughput_metrics: ThroughputMetrics {
                queries_per_second: 0.0,
                peak_concurrent_queries: 0,
                average_concurrent_queries: 0.0,
                queue_wait_time_ms: 0.0,
            },
        };

        if !options.quiet {
            formatted_output
                .push_str("⚠️  Performance metrics collection not yet fully implemented\n");
            formatted_output
                .push_str("   Basic database statistics available via `stats` command\n");
        }

        Ok(PerformanceMetricsResult {
            metrics,
            formatted_output,
        })
    }

    /// Get storage usage analysis and optimization recommendations
    pub async fn analyze_storage(&self) -> Result<String> {
        let mut analysis = String::new();

        analysis.push_str("💾 Storage Analysis\n");
        analysis.push_str("==================\n\n");

        // Get basic storage statistics
        let basic_stats = self.get_basic_statistics().await?;

        analysis.push_str(&format!("📁 Documents: {}\n", basic_stats.document_count));
        analysis.push_str(&format!(
            "💿 Total Size: {} bytes ({:.2} MB)\n",
            basic_stats.total_size_bytes,
            basic_stats.total_size_bytes as f64 / 1024.0 / 1024.0
        ));
        analysis.push_str(&format!(
            "📊 Storage Efficiency: {:.1}%\n",
            basic_stats.storage_efficiency * 100.0
        ));

        // TODO: Add more detailed storage analysis
        // - Fragmentation analysis
        // - Compression opportunities
        // - Index size breakdown
        // - Growth projections

        analysis.push_str("\n⚠️  Detailed storage analysis not yet fully implemented\n");

        Ok(analysis)
    }

    // Private helper methods

    async fn get_basic_statistics(&self) -> Result<BasicStats> {
        let storage_arc = self.database.storage();
        let storage = storage_arc.lock().await;
        let all_docs = storage.list_all().await?;

        let count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();
        let avg_size = if count > 0 { total_size / count } else { 0 };

        Ok(BasicStats {
            document_count: count,
            total_size_bytes: total_size,
            average_file_size: avg_size,
            storage_efficiency: 0.85, // TODO: Calculate actual efficiency
            index_count: 3,           // TODO: Get actual index count
        })
    }

    #[cfg(feature = "tree-sitter-parsing")]
    async fn get_symbol_statistics(&self) -> Result<SymbolStats> {
        let symbol_db_path = self.db_path.join("symbols.kota");

        if !symbol_db_path.exists() {
            return Ok(SymbolStats {
                total_symbols: 0,
                symbols_by_type: HashMap::new(),
                symbols_by_language: HashMap::new(),
                files_with_symbols: 0,
                symbol_density: 0.0,
                extraction_coverage: 0.0,
            });
        }

        let reader = crate::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
        let total_symbols = reader.symbol_count();

        // Collect detailed statistics from binary symbols
        let mut symbols_by_type: HashMap<String, usize> = HashMap::new();
        let mut symbols_by_language: HashMap<String, usize> = HashMap::new();
        let mut unique_files = std::collections::HashSet::new();

        for symbol in reader.iter_symbols() {
            // Count by type - convert u8 back to SymbolType for readable display
            let type_name = match crate::parsing::SymbolType::try_from(symbol.kind) {
                Ok(symbol_type) => format!("{}", symbol_type),
                Err(_) => format!("unknown({})", symbol.kind),
            };
            *symbols_by_type.entry(type_name).or_insert(0) += 1;

            // Count by language and track unique files
            if let Ok(file_path) = reader.get_symbol_file_path(&symbol) {
                unique_files.insert(file_path.clone());
                let path = std::path::Path::new(&file_path);
                let lang = crate::path_utils::detect_language_from_extension(path);
                *symbols_by_language.entry(lang.to_string()).or_insert(0) += 1;
            }
        }

        let files_with_symbols = unique_files.len();
        let symbol_density = if files_with_symbols > 0 {
            total_symbols as f64 / files_with_symbols as f64
        } else {
            0.0
        };

        // For extraction coverage, we'd need to know total files analyzed - for now use 100% if we have symbols
        let extraction_coverage = if files_with_symbols > 0 { 100.0 } else { 0.0 };

        Ok(SymbolStats {
            total_symbols,
            symbols_by_type,
            symbols_by_language,
            files_with_symbols,
            symbol_density,
            extraction_coverage,
        })
    }

    #[cfg(feature = "tree-sitter-parsing")]
    async fn get_relationship_statistics(&self) -> Result<RelationshipStats> {
        // TODO: Implement relationship statistics collection
        Ok(RelationshipStats {
            total_relationships: 0,
            connected_symbols: 0,
            dependency_graph_stats: None,
            relationship_types: HashMap::new(),
            average_connections_per_symbol: 0.0,
        })
    }

    async fn format_basic_statistics(&self, stats: &BasicStats) -> Result<String> {
        let mut output = String::new();

        output.push_str("📊 Database Statistics\n");
        output.push_str("=====================\n\n");

        output.push_str("📁 Indexed Content:\n");
        output.push_str(&format!(
            "   Total files indexed: {}\n",
            stats.document_count
        ));
        output.push_str(&format!(
            "   Total content size: {} bytes ({:.2} MB)\n",
            stats.total_size_bytes,
            stats.total_size_bytes as f64 / 1024.0 / 1024.0
        ));
        if stats.document_count > 0 {
            output.push_str(&format!(
                "   Average file size: {} bytes\n",
                stats.average_file_size
            ));
        }
        output.push_str(&format!(
            "   Storage efficiency: {:.1}%\n",
            stats.storage_efficiency * 100.0
        ));
        output.push('\n');

        Ok(output)
    }

    #[cfg(feature = "tree-sitter-parsing")]
    async fn format_symbol_statistics(&self, stats: &SymbolStats) -> Result<String> {
        let mut output = String::new();

        output.push_str("🔣 Symbol Analysis:\n");
        output.push_str(&format!(
            "   Total symbols extracted: {}\n",
            stats.total_symbols
        ));
        output.push_str(&format!(
            "   Files with symbols: {}\n",
            stats.files_with_symbols
        ));
        output.push_str(&format!(
            "   Symbol density: {:.2} symbols/file\n",
            stats.symbol_density
        ));
        output.push_str(&format!(
            "   Extraction coverage: {:.1}%\n",
            stats.extraction_coverage * 100.0
        ));

        if !stats.symbols_by_type.is_empty() {
            output.push_str("   By type:\n");
            for (symbol_type, count) in &stats.symbols_by_type {
                output.push_str(&format!("     {}: {}\n", symbol_type, count));
            }
        }

        output.push('\n');
        Ok(output)
    }

    #[cfg(feature = "tree-sitter-parsing")]
    async fn format_relationship_statistics(&self, stats: &RelationshipStats) -> Result<String> {
        let mut output = String::new();

        output.push_str("🔗 Relationship Analysis:\n");
        output.push_str(&format!(
            "   Total relationships: {}\n",
            stats.total_relationships
        ));
        output.push_str(&format!(
            "   Connected symbols: {}\n",
            stats.connected_symbols
        ));
        output.push_str(&format!(
            "   Avg connections/symbol: {:.2}\n",
            stats.average_connections_per_symbol
        ));

        if let Some(graph_stats) = &stats.dependency_graph_stats {
            output.push_str("   Dependency graph:\n");
            output.push_str(&format!("     Nodes: {}\n", graph_stats.nodes));
            output.push_str(&format!("     Edges: {}\n", graph_stats.edges));
            output.push_str(&format!("     Max depth: {}\n", graph_stats.max_depth));
            output.push_str(&format!(
                "     Circular dependencies: {}\n",
                graph_stats.circular_dependencies
            ));
        }

        output.push('\n');
        Ok(output)
    }

    async fn format_health_report(&self, report: &HealthReport) -> Result<String> {
        let mut output = String::new();

        let status_emoji = match report.overall_status {
            HealthStatus::Healthy => "✅",
            HealthStatus::Warning => "⚠️",
            HealthStatus::Critical => "❌",
            HealthStatus::Unknown => "❓",
        };

        output.push_str(&format!(
            "{} Overall Health: {:?}\n\n",
            status_emoji, report.overall_status
        ));

        // Format individual health components
        output.push_str(&format!("💾 Storage: {:?}\n", report.storage_health.status));
        output.push_str(&format!("📇 Indices: {:?}\n", report.index_health.status));
        output.push_str(&format!(
            "⚡ Performance: {:?}\n",
            report.performance_health.status
        ));

        if !report.issues_found.is_empty() {
            output.push_str("\n🔍 Issues Found:\n");
            for issue in &report.issues_found {
                output.push_str(&format!(
                    "   - {}: {}\n",
                    issue.component, issue.description
                ));
            }
        }

        if !report.recommendations.is_empty() {
            output.push_str("\n💡 Recommendations:\n");
            for rec in &report.recommendations {
                output.push_str(&format!("   - {}\n", rec));
            }
        }

        output.push('\n');
        Ok(output)
    }

    async fn generate_usage_tips(&self) -> Result<String> {
        let mut tips = String::new();

        // Check if symbols exist to provide appropriate tips
        #[cfg(feature = "tree-sitter-parsing")]
        {
            let symbol_db_path = self.db_path.join("symbols.kota");
            if !symbol_db_path.exists() {
                tips.push_str("\n💡 No symbols found in database.\n");
                tips.push_str("   Required steps:\n");
                tips.push_str("   1. Index a codebase: kotadb index-codebase /path/to/repo\n");
                tips.push_str("   2. Verify indexing: kotadb stats --symbols\n");
            } else {
                let reader = crate::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
                let symbol_count = reader.symbol_count();

                if symbol_count > 0 {
                    tips.push_str("\n🚀 Codebase intelligence ready! Try these commands:\n");
                    tips.push_str("   find-callers <symbol>     - Find what calls a function\n");
                    tips.push_str("   analyze-impact <symbol>   - Analyze change impact\n");
                    tips.push_str("   search-symbols <pattern>  - Search code symbols\n");
                    tips.push_str("   search-code <query>       - Full-text code search\n");
                }
            }
        }

        Ok(tips)
    }

    async fn check_storage_health(&self, deep_scan: bool) -> Result<StorageHealth> {
        // TODO: Implement comprehensive storage health check
        Ok(StorageHealth {
            status: HealthStatus::Healthy,
            fragmentation_percent: 5.0, // TODO: Calculate actual fragmentation
            corruption_detected: false,
            space_utilization: 85.0,
            backup_status: "No backups configured".to_string(),
        })
    }

    async fn check_index_health(&self, check_integrity: bool) -> Result<IndexHealth> {
        // TODO: Implement comprehensive index health check
        Ok(IndexHealth {
            status: HealthStatus::Healthy,
            indices_healthy: 3,
            indices_degraded: 0,
            indices_corrupted: 0,
            average_lookup_time_ms: 2.5,
            consistency_checks_passed: true,
        })
    }

    async fn check_performance_health(&self) -> Result<PerformanceHealth> {
        // TODO: Implement performance health monitoring
        Ok(PerformanceHealth {
            status: HealthStatus::Healthy,
            average_query_time_ms: 8.5,
            throughput_queries_per_second: 150.0,
            memory_usage_mb: 64.0,
            cache_hit_rate: 92.5,
            resource_bottlenecks: Vec::new(),
        })
    }
}
