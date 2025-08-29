// KotaDB CLI - Codebase intelligence platform for distributed human-AI cognition
use anyhow::Result;
use clap::{Parser, Subcommand};

// Macro for conditional printing based on quiet flag
macro_rules! qprintln {
    ($quiet:expr, $($arg:tt)*) => {
        if !$quiet {
            println!($($arg)*);
        }
    };
}
use kotadb::{
    create_binary_trigram_index, create_file_storage, create_primary_index, create_trigram_index,
    create_wrapped_storage, init_logging_with_level, start_server, validate_post_ingestion_search,
    with_trace_id, Document, DocumentBuilder, Index, QueryBuilder, Storage, ValidatedDocumentId,
    ValidatedPath, ValidationStatus,
};

#[cfg(feature = "tree-sitter-parsing")]
use kotadb::relationship_query::{
    parse_natural_language_relationship_query, RelationshipQueryType,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Match a string against a wildcard pattern
/// Supports patterns like "*.rs", "*Controller.rs", "test_*", "create_*", etc.
/// Copied from primary_index.rs to make it available for symbol search
fn matches_wildcard_pattern(text: &str, pattern: &str) -> bool {
    // Handle pure wildcard
    if pattern == "*" {
        return true;
    }

    // Split pattern by '*' to get fixed parts
    let parts: Vec<&str> = pattern.split('*').collect();

    // Handle patterns with wildcards
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue; // Skip empty parts (from consecutive * or leading/trailing *)
        }

        // First part must match at beginning unless pattern starts with *
        if i == 0 && !pattern.starts_with('*') {
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        }
        // Last part must match at end unless pattern ends with *
        else if i == parts.len() - 1 && !pattern.ends_with('*') {
            if !text.ends_with(part) {
                return false;
            }
        }
        // Middle parts or wildcard-bounded parts can appear anywhere after current position
        else if let Some(found_pos) = text[pos..].find(part) {
            pos += found_pos + part.len();
        } else {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod wildcard_tests {
    use super::*;

    #[test]
    fn test_matches_wildcard_pattern() {
        // Test pure wildcard
        assert!(matches_wildcard_pattern("anything", "*"));
        assert!(matches_wildcard_pattern("create_file_storage", "*"));
        assert!(matches_wildcard_pattern("", "*"));

        // Test prefix wildcard patterns
        assert!(matches_wildcard_pattern("create_file_storage", "create_*"));
        assert!(matches_wildcard_pattern("create_index", "create_*"));
        assert!(matches_wildcard_pattern("create_", "create_*"));
        assert!(!matches_wildcard_pattern("make_create", "create_*"));
        assert!(!matches_wildcard_pattern("file_storage", "create_*"));

        // Test suffix wildcard patterns
        assert!(matches_wildcard_pattern("file_storage", "*_storage"));
        assert!(matches_wildcard_pattern("memory_storage", "*_storage"));
        assert!(matches_wildcard_pattern("_storage", "*_storage"));
        assert!(!matches_wildcard_pattern("storage_file", "*_storage"));
        assert!(!matches_wildcard_pattern("file_index", "*_storage"));

        // Test middle wildcard patterns
        assert!(matches_wildcard_pattern("create_file_storage", "*file*"));
        assert!(matches_wildcard_pattern("file", "*file*"));
        assert!(matches_wildcard_pattern("myfile", "*file*"));
        assert!(matches_wildcard_pattern("filetest", "*file*"));
        assert!(matches_wildcard_pattern("myfiletest", "*file*"));
        assert!(!matches_wildcard_pattern("storage", "*file*"));

        // Test exact matches (no wildcards)
        assert!(matches_wildcard_pattern(
            "create_file_storage",
            "create_file_storage"
        ));
        assert!(!matches_wildcard_pattern(
            "create_file_storage",
            "create_index"
        ));
        assert!(!matches_wildcard_pattern(
            "create_index",
            "create_file_storage"
        ));

        // Test complex patterns
        assert!(matches_wildcard_pattern("BinaryTrigramIndex", "*Index"));
        assert!(matches_wildcard_pattern("PrimaryIndex", "*Index"));
        assert!(!matches_wildcard_pattern("IndexHelper", "*Index"));

        // Test multiple wildcards
        assert!(matches_wildcard_pattern(
            "create_file_storage_impl",
            "create_*_*"
        ));
        assert!(matches_wildcard_pattern(
            "create_memory_index_impl",
            "create_*_*"
        ));
        assert!(!matches_wildcard_pattern("create_file", "create_*_*"));
        assert!(!matches_wildcard_pattern("make_file_storage", "create_*_*"));
    }
}

#[derive(Parser)]
#[command(
    author,
    version,
    about = "KotaDB - Codebase intelligence platform for AI assistants",
    long_about = None,
    after_help = "QUICK START:
  1. Index a codebase:        kotadb index-codebase /path/to/repo
  2. Search for code:         kotadb search-code 'function_name'
  3. Find relationships:      kotadb find-callers 'MyFunction'
  4. Analyze impact:          kotadb analyze-impact 'StorageClass'

EXAMPLES:
  # Index and search your codebase
  kotadb index-codebase ./my-project
  kotadb search-code 'database query'
  kotadb search-symbols 'FileStorage'
  
  # Analyze code relationships
  kotadb find-callers FileStorage
  kotadb analyze-impact Config
  kotadb relationship-query 'what calls MyFunction?'

  # System management
  kotadb stats
  kotadb serve --port 8080"
)]
struct Cli {
    /// Enable verbose logging (DEBUG level). Default is WARN level.
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    verbose: bool,

    /// Suppress all non-essential output for LLM-friendly results (ERROR level logging only)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Database directory path
    #[arg(short, long, default_value = "./kota-db-data")]
    db_path: PathBuf,

    /// Use binary format for indices (10x faster performance)
    #[arg(long, global = true, default_value = "true")]
    binary_index: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP REST API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Search for code and symbols in the indexed codebase
    SearchCode {
        /// Search query (use '*' for all, or search terms for content/symbol matching)
        #[arg(default_value = "*")]
        query: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "10",
            help = "Control number of results (increase with -l 100)"
        )]
        limit: usize,
        /// Filter by tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
        /// Context level for output (none/minimal/medium/full)
        #[arg(
            short = 'c',
            long,
            default_value = "medium",
            help = "Context detail level for LLM consumption",
            value_parser = ["none", "minimal", "medium", "full"]
        )]
        context: String,
    },

    /// Show comprehensive database statistics (documents, symbols, relationships)
    Stats {
        /// Show only basic document statistics
        #[arg(long, help = "Show only document count and size statistics")]
        basic: bool,
        /// Show detailed symbol analysis
        #[arg(long, help = "Show symbol extraction and type breakdown")]
        symbols: bool,
        /// Show relationship and dependency data
        #[arg(long, help = "Show relationship graph and dependency analysis")]
        relationships: bool,
    },

    /// Validate search functionality
    Validate,

    /// Verify documentation accuracy against implementation
    VerifyDocs,

    /// Index a codebase for intelligent analysis
    #[cfg(feature = "git-integration")]
    IndexCodebase {
        /// Path to the git repository
        repo_path: PathBuf,
        /// Prefix for document paths in the database
        #[arg(short, long, default_value = "repos")]
        prefix: String,
        /// Include file contents
        #[arg(long, default_value = "true")]
        include_files: bool,
        /// Include commit history
        #[arg(long, default_value = "true")]
        include_commits: bool,
        /// Maximum file size to ingest (in MB)
        #[arg(long, default_value = "10")]
        max_file_size_mb: usize,
        /// Maximum total memory usage for ingestion (in MB, None = unlimited)
        #[arg(long)]
        max_memory_mb: Option<u64>,
        /// Maximum number of files to process in parallel (None = auto-detect)
        #[arg(long)]
        max_parallel_files: Option<usize>,
        /// Enable adaptive chunking to reduce memory usage during ingestion
        #[arg(long, default_value = "true")]
        enable_chunking: bool,
        /// Extract code symbols using tree-sitter parsing (enabled by default)
        /// Use --extract-symbols=false or --no-symbols to disable
        #[cfg(feature = "tree-sitter-parsing")]
        #[arg(long, default_value = "true")]
        extract_symbols: Option<bool>,
        /// Skip symbol extraction (convenience flag, same as --extract-symbols=false)
        #[cfg(feature = "tree-sitter-parsing")]
        #[arg(long, conflicts_with = "extract_symbols")]
        no_symbols: bool,
    },

    /// Search for symbols (functions, classes, variables) by name or pattern
    #[cfg(feature = "tree-sitter-parsing")]
    SearchSymbols {
        /// Symbol name or pattern to search for (supports partial matching)
        pattern: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "25",
            help = "Control number of results (use -l 100 for more)"
        )]
        limit: usize,
        /// Show only specific symbol types (function, class, variable, etc.)
        #[arg(short = 't', long)]
        symbol_type: Option<String>,
    },

    /// Find all places where a symbol (function, class, variable) is called or referenced
    #[cfg(feature = "tree-sitter-parsing")]
    FindCallers {
        /// Name or qualified name of the target symbol (e.g., 'FileStorage' or 'storage::FileStorage')
        target: String,
        /// Maximum number of results to return (default: unlimited)
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Analyze impact: what would break if you change a symbol
    #[cfg(feature = "tree-sitter-parsing")]
    AnalyzeImpact {
        /// Name or qualified name of the target symbol (e.g., 'StorageError' or 'errors::StorageError')
        target: String,
        /// Maximum number of impacted items to show (default: unlimited)
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Natural language queries about code symbols and relationships
    #[cfg(feature = "tree-sitter-parsing")]
    RelationshipQuery {
        /// Natural language query (e.g., 'what calls FileStorage?', 'who uses Config?', 'find unused functions')
        query: String,
        /// Maximum number of results to return
        #[arg(
            short,
            long,
            default_value = "50",
            help = "Control number of results returned"
        )]
        limit: Option<usize>,
    },

    /// Run performance benchmarks on database operations
    ///
    /// Note: Benchmark data remains in the database after completion for inspection.
    /// Use a fresh database path to avoid data accumulation across runs.
    Benchmark {
        /// Number of operations to perform
        #[arg(short, long, default_value = "10000")]
        operations: usize,
        /// Run only specific benchmark types (storage, index, query, all)
        #[arg(short = 't', long, default_value = "all")]
        benchmark_type: String,
        /// Output format (human, json, csv)
        #[arg(short = 'f', long, default_value = "human")]
        format: String,
        /// Maximum number of search queries to run (prevents excessive runtime)
        #[arg(
            long,
            default_value = "100",
            help = "Limit search operations to prevent excessive runtime"
        )]
        max_search_queries: usize,
    },
}

struct Database {
    storage: Arc<Mutex<dyn Storage>>,
    primary_index: Arc<Mutex<dyn Index>>,
    trigram_index: Arc<Mutex<dyn Index>>,
    // Cache for path -> document ID lookups (built lazily)
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl Database {
    async fn new(db_path: &Path, use_binary_index: bool) -> Result<Self> {
        let storage_path = db_path.join("storage");
        let primary_index_path = db_path.join("primary_index");
        let trigram_index_path = db_path.join("trigram_index");

        // Create directories if they don't exist
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&primary_index_path)?;
        std::fs::create_dir_all(&trigram_index_path)?;

        let storage = create_file_storage(
            storage_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid storage path: {:?}", storage_path))?,
            Some(100), // Cache size
        )
        .await?;

        let primary_index = create_primary_index(
            primary_index_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid primary index path: {:?}", primary_index_path)
            })?,
            Some(1000),
        )
        .await?;
        let trigram_index_arc: Arc<Mutex<dyn Index>> = if use_binary_index {
            tracing::info!("Using binary trigram index for 10x performance");
            Arc::new(Mutex::new(
                create_binary_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000),
                )
                .await?,
            ))
        } else {
            Arc::new(Mutex::new(
                create_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000),
                )
                .await?,
            ))
        };

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));

        let db = Self {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Skip path cache rebuild for read-only operations like search
        // The cache will be built lazily when needed for path-based lookups
        // This significantly improves startup time for search operations
        // from ~300ms to ~5ms (see issue #274)

        Ok(db)
    }

    /// Rebuild the path cache from current storage
    async fn rebuild_path_cache(&self) -> Result<()> {
        let mut cache = self.path_cache.write().await;
        cache.clear();

        // Get all documents to build the cache
        let all_docs = self.storage.lock().await.list_all().await?;
        for doc in all_docs {
            cache.insert(doc.path.to_string(), doc.id);
        }

        Ok(())
    }

    /// Rebuild all indices from current storage
    /// This is needed after bulk operations like git ingestion
    async fn rebuild_indices(&self) -> Result<()> {
        // Get all documents from storage
        let all_docs = self.storage.lock().await.list_all().await?;
        let total_docs = all_docs.len();

        if total_docs == 0 {
            return Ok(());
        }

        // Process documents in batches for better performance
        const BATCH_SIZE: usize = 100;
        let mut processed = 0;

        // Process in chunks to reduce lock contention and prevent OOM
        for chunk in all_docs.chunks(BATCH_SIZE) {
            // Collect document data for this batch (including content for trigram indexing)
            let mut batch_entries = Vec::with_capacity(chunk.len());
            for doc in chunk {
                let doc_id = doc.id;
                let doc_path = ValidatedPath::new(doc.path.to_string())?;
                batch_entries.push((doc_id, doc_path, doc.content.clone()));
            }

            // Insert batch into primary index (path-based)
            {
                let mut primary_index = self.primary_index.lock().await;
                for (doc_id, doc_path, _) in &batch_entries {
                    primary_index.insert(*doc_id, doc_path.clone()).await?;
                }
            }

            // Insert batch into trigram index with content for proper full-text search
            {
                let mut trigram_index = self.trigram_index.lock().await;
                for (doc_id, doc_path, content) in &batch_entries {
                    // Use the new content-aware method for proper trigram indexing
                    trigram_index
                        .insert_with_content(*doc_id, doc_path.clone(), content)
                        .await?;
                }
            }

            processed += chunk.len();

            // Periodic flush for large datasets
            if processed % 500 == 0 || processed == total_docs {
                self.primary_index.lock().await.flush().await?;
                self.trigram_index.lock().await.flush().await?;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn search(
        &self,
        query_text: &str,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> Result<Vec<Document>> {
        let (documents, _) = self.search_with_count(query_text, tags, limit).await?;
        Ok(documents)
    }

    async fn search_with_count(
        &self,
        query_text: &str,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> Result<(Vec<Document>, usize)> {
        // Handle empty queries at the method level - return nothing
        if query_text.is_empty() {
            return Ok((Vec::new(), 0));
        }

        // Build query
        let mut query_builder = QueryBuilder::new();

        if query_text != "*" {
            query_builder = query_builder.with_text(query_text)?;
        }

        if let Some(tag_list) = tags {
            for tag in tag_list {
                query_builder = query_builder.with_tag(tag)?;
            }
        }

        query_builder = query_builder.with_limit(limit)?;
        let query = query_builder.build()?;

        // Route to appropriate index based on query type
        // NOTE: Wildcard queries (containing "*") are explicitly routed to the primary index
        // because it supports pattern matching. Trigram indices are designed for full-text
        // search and don't handle wildcard patterns. Empty queries are handled above and return nothing.
        let doc_ids = if query_text.contains('*') {
            // Use Primary Index for wildcard/pattern queries
            self.primary_index.lock().await.search(&query).await?
        } else {
            // Use Trigram Index for full-text search queries
            self.trigram_index.lock().await.search(&query).await?
        };

        // Store total count before limiting
        let total_count = doc_ids.len();

        // Retrieve documents from storage
        let doc_ids_limited: Vec<_> = doc_ids.into_iter().take(limit).collect();
        let mut documents = Vec::with_capacity(doc_ids_limited.len());
        let storage = self.storage.lock().await;

        for doc_id in doc_ids_limited {
            if let Some(doc) = storage.get(&doc_id).await? {
                documents.push(doc);
            }
        }

        Ok((documents, total_count))
    }

    async fn stats(&self) -> Result<(usize, usize)> {
        let all_docs = self.storage.lock().await.list_all().await?;
        let doc_count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();
        Ok((doc_count, total_size))
    }
}

/// Run performance benchmarks for various database operations
async fn run_benchmarks(
    database: Database,
    operations: usize,
    benchmark_type: &str,
    format: &str,
    max_search_queries: usize,
    quiet: bool,
) -> Result<()> {
    use serde_json::json;
    use std::time::{Duration, Instant};

    #[derive(Debug, Clone)]
    struct BenchmarkResult {
        operation: String,
        total_operations: usize,
        total_duration: Duration,
        ops_per_second: f64,
        avg_latency_ms: f64,
        min_latency_ms: f64,
        max_latency_ms: f64,
    }

    let mut results = Vec::new();

    // Storage benchmarks
    if benchmark_type == "storage" || benchmark_type == "all" {
        qprintln!(quiet, "\n📦 Storage Benchmarks");
        qprintln!(quiet, "   Testing insert and retrieve operations...");

        let mut durations = Vec::new();
        let start = Instant::now();

        for i in 0..operations {
            let doc = DocumentBuilder::new()
                .path(format!("benchmark/doc_{}.md", i))?
                .title(format!("Benchmark Document {}", i))?
                .content(format!("Benchmark document {} content with some test data", i).as_bytes())
                .build()?;

            let op_start = Instant::now();
            database.storage.lock().await.insert(doc.clone()).await?;
            durations.push(op_start.elapsed());

            // Also test retrieval
            let _ = database.storage.lock().await.get(&doc.id).await?;
        }

        let total_duration = start.elapsed();
        let ops_per_second = operations as f64 / total_duration.as_secs_f64();
        let avg_latency_ms = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / durations.len() as f64;
        let min_latency_ms = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let max_latency_ms = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        results.push(BenchmarkResult {
            operation: "storage".to_string(),
            total_operations: operations,
            total_duration,
            ops_per_second,
            avg_latency_ms,
            min_latency_ms,
            max_latency_ms,
        });

        if !quiet {
            println!(
                "   ✓ Completed {} storage operations in {:.2}s",
                operations,
                total_duration.as_secs_f64()
            );
            println!(
                "   ✓ {:.0} ops/sec, avg latency: {:.2}ms",
                ops_per_second, avg_latency_ms
            );
        }
    }

    // Index benchmarks
    if benchmark_type == "index" || benchmark_type == "all" {
        qprintln!(quiet, "\n🔍 Index Benchmarks");
        qprintln!(quiet, "   Rebuilding indices for benchmark documents...");

        let start = Instant::now();
        database.rebuild_indices().await?;
        let rebuild_duration = start.elapsed();

        qprintln!(
            quiet,
            "   ✓ Index rebuild completed in {:.2}s",
            rebuild_duration.as_secs_f64()
        );

        // Test search operations
        qprintln!(quiet, "   Testing search operations...");

        let search_limit = operations.min(max_search_queries);
        let mut search_durations = Vec::new();
        let search_start = Instant::now();

        for i in 0..search_limit {
            let query_text = format!("benchmark document {}", i % 10);
            let op_start = Instant::now();

            let _ = database.search(&query_text, None, 10).await?;

            search_durations.push(op_start.elapsed());
        }

        let search_duration = search_start.elapsed();
        let search_ops_per_second = search_limit as f64 / search_duration.as_secs_f64();
        let avg_search_latency_ms = search_durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / search_durations.len() as f64;
        let min_search_latency_ms = search_durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let max_search_latency_ms = search_durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        results.push(BenchmarkResult {
            operation: "search".to_string(),
            total_operations: search_limit,
            total_duration: search_duration,
            ops_per_second: search_ops_per_second,
            avg_latency_ms: avg_search_latency_ms,
            min_latency_ms: min_search_latency_ms,
            max_latency_ms: max_search_latency_ms,
        });

        if !quiet {
            println!(
                "   ✓ Completed {} search operations in {:.2}s",
                search_limit,
                search_duration.as_secs_f64()
            );
            println!(
                "   ✓ {:.0} searches/sec, avg latency: {:.2}ms",
                search_ops_per_second, avg_search_latency_ms
            );
        }
    }

    // Query benchmarks (testing different query types)
    if benchmark_type == "query" || benchmark_type == "all" {
        qprintln!(quiet, "\n📝 Query Benchmarks");

        // Test wildcard queries
        let wildcard_start = Instant::now();
        let _ = database.search("*", None, 10).await?;
        let wildcard_duration = wildcard_start.elapsed();

        results.push(BenchmarkResult {
            operation: "wildcard_query".to_string(),
            total_operations: 1,
            total_duration: wildcard_duration,
            ops_per_second: 1.0 / wildcard_duration.as_secs_f64(),
            avg_latency_ms: wildcard_duration.as_secs_f64() * 1000.0,
            min_latency_ms: wildcard_duration.as_secs_f64() * 1000.0,
            max_latency_ms: wildcard_duration.as_secs_f64() * 1000.0,
        });

        qprintln!(
            quiet,
            "   ✓ Wildcard query completed in {:.2}ms",
            wildcard_duration.as_secs_f64() * 1000.0
        );
    }

    // Output results based on format
    match format {
        "json" => {
            let json_output = json!({
                "benchmark_type": benchmark_type,
                "total_operations": operations,
                "results": results.iter().map(|r| json!({
                    "operation": r.operation,
                    "total_operations": r.total_operations,
                    "duration_seconds": r.total_duration.as_secs_f64(),
                    "ops_per_second": r.ops_per_second,
                    "avg_latency_ms": r.avg_latency_ms,
                    "min_latency_ms": r.min_latency_ms,
                    "max_latency_ms": r.max_latency_ms,
                })).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        "csv" => {
            println!("operation,total_operations,duration_seconds,ops_per_second,avg_latency_ms,min_latency_ms,max_latency_ms");
            for r in results {
                println!(
                    "{},{},{:.3},{:.2},{:.3},{:.3},{:.3}",
                    r.operation,
                    r.total_operations,
                    r.total_duration.as_secs_f64(),
                    r.ops_per_second,
                    r.avg_latency_ms,
                    r.min_latency_ms,
                    r.max_latency_ms
                );
            }
        }
        _ => {
            // human format
            if !quiet {
                println!("\n📊 Benchmark Summary");
                println!("   Type: {}", benchmark_type);
                println!("   Operations: {}", operations);
                println!("\n   Results:");
                for r in results {
                    println!(
                        "   - {}: {:.0} ops/sec, avg: {:.2}ms, min: {:.2}ms, max: {:.2}ms",
                        r.operation,
                        r.ops_per_second,
                        r.avg_latency_ms,
                        r.min_latency_ms,
                        r.max_latency_ms
                    );
                }
                println!("\n💡 Note: Benchmark data remains in the database for inspection.");
                println!("   Use a fresh database path to avoid data accumulation across runs.");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use kotadb::{
        create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rebuild_indices_empty_storage() -> Result<()> {
        // Test that rebuild_indices handles empty storage gracefully
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        let storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        let primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;

        let trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
        let trigram_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        let db = Database {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Should not panic with empty storage
        db.rebuild_indices().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_rebuild_indices_batch_processing() -> Result<()> {
        // Test that rebuild_indices handles many documents efficiently
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let primary_path = temp_dir.path().join("primary");
        let trigram_path = temp_dir.path().join("trigram");

        let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(1000)).await?;

        // Add test documents to storage
        for i in 0..150 {
            let doc = DocumentBuilder::new()
                .path(format!("batch/doc_{}.md", i))?
                .title(format!("Batch Document {}", i))?
                .content(format!("Content for batch document {}", i).as_bytes())
                .build()?;
            storage.insert(doc).await?;
        }

        let primary_index =
            create_primary_index(primary_path.to_str().unwrap(), Some(1000)).await?;

        let trigram_index =
            create_trigram_index(trigram_path.to_str().unwrap(), Some(1000)).await?;

        let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
        let primary_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
        let trigram_index_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

        let db = Database {
            storage: storage_arc,
            primary_index: primary_index_arc,
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Time the rebuild operation
        let start = std::time::Instant::now();
        db.rebuild_indices().await?;
        let duration = start.elapsed();

        // Verify all documents are indexed
        let query = QueryBuilder::new().with_limit(200)?.build()?;
        let results = db.primary_index.lock().await.search(&query).await?;
        assert!(
            results.len() >= 150,
            "Expected at least 150 documents, got {}",
            results.len()
        );

        // Performance check: should complete in reasonable time
        assert!(
            duration.as_secs() < 3,
            "Batch rebuild took too long: {:?}",
            duration
        );

        Ok(())
    }
}

/// Create a hybrid relationship query engine for the given database path
#[cfg(feature = "tree-sitter-parsing")]
async fn create_relationship_engine(
    db_path: &Path,
) -> Result<kotadb::hybrid_relationship_engine::HybridRelationshipEngine> {
    // Create hybrid relationship engine with direct binary symbol access
    // The engine loads symbols directly from symbols.kota and dependency_graph.bin
    let config = kotadb::relationship_query::RelationshipQueryConfig::default();
    let hybrid_engine =
        kotadb::hybrid_relationship_engine::HybridRelationshipEngine::new(db_path, config).await?;

    // Check if we have any symbols or relationships loaded
    let stats = hybrid_engine.get_stats();
    if !stats.using_binary_path && stats.binary_symbols_loaded == 0 {
        return Err(anyhow::anyhow!(
            "No symbols found in database. Required steps:\n\
             1. Index a codebase: kotadb index-codebase /path/to/repo\n\
             2. Verify indexing: kotadb symbol-stats\n\
             3. Then retry this command"
        ));
    }

    Ok(hybrid_engine)
}

/// Display symbol statistics from the binary symbol database
#[cfg(feature = "tree-sitter-parsing")]
async fn show_symbol_statistics(db_path: &std::path::Path, quiet: bool) -> Result<()> {
    use std::collections::HashMap;

    let symbol_db_path = db_path.join("symbols.kota");

    if !symbol_db_path.exists() {
        return Ok(()); // No symbols to show
    }

    qprintln!(quiet, "\n🔤 Symbol Analysis:");

    // Read binary symbols
    let reader = kotadb::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
    let binary_symbol_count = reader.symbol_count();

    // Collect statistics from binary symbols
    let mut symbols_by_type: HashMap<String, usize> = HashMap::new();
    let mut symbols_by_language: HashMap<String, usize> = HashMap::new();
    let mut unique_files = std::collections::HashSet::new();

    for symbol in reader.iter_symbols() {
        // Count by type - convert u8 back to SymbolType for readable display
        let type_name = match kotadb::parsing::SymbolType::try_from(symbol.kind) {
            Ok(symbol_type) => format!("{}", symbol_type),
            Err(_) => format!("unknown({})", symbol.kind),
        };
        *symbols_by_type.entry(type_name).or_insert(0) += 1;

        // Count by language (inferred from file extension)
        if let Ok(file_path) = reader.get_symbol_file_path(&symbol) {
            unique_files.insert(file_path.clone());
            if let Some(ext) = std::path::Path::new(&file_path).extension() {
                let lang = match ext.to_str() {
                    Some("rs") => "Rust",
                    Some("py") => "Python",
                    Some("js") | Some("jsx") => "JavaScript",
                    Some("ts") | Some("tsx") => "TypeScript",
                    Some("go") => "Go",
                    Some("java") => "Java",
                    Some("cpp") | Some("cc") | Some("cxx") => "C++",
                    Some("c") | Some("h") => "C",
                    _ => "Other",
                };
                *symbols_by_language.entry(lang.to_string()).or_insert(0) += 1;
            }
        }
    }

    qprintln!(quiet, "   Path: {:?}", symbol_db_path);
    qprintln!(quiet, "   Total symbols: {}", binary_symbol_count);
    qprintln!(quiet, "   Total files: {}", unique_files.len());

    if !symbols_by_type.is_empty() {
        qprintln!(quiet, "\n📝 Symbols by Type:");
        let mut types: Vec<_> = symbols_by_type.into_iter().collect();
        types.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        for (symbol_type, count) in types {
            qprintln!(quiet, "   {}: {}", symbol_type, count);
        }
    }

    if !symbols_by_language.is_empty() {
        qprintln!(quiet, "\n🌐 Symbols by Language:");
        let mut langs: Vec<_> = symbols_by_language.into_iter().collect();
        langs.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        for (language, count) in langs {
            qprintln!(quiet, "   {}: {}", language, count);
        }
    }

    if binary_symbol_count > 0 {
        qprintln!(quiet, "\n📍 Binary Symbol Database:");
        qprintln!(quiet, "   Format: KotaDB Binary Format (.kota)");
        qprintln!(quiet, "   Performance: 10x faster than JSON");
        qprintln!(quiet, "   Features: Memory-mapped, zero-copy access");
    }

    Ok(())
}

/// Display relationship and dependency graph statistics
#[cfg(feature = "tree-sitter-parsing")]
async fn show_relationship_statistics(db_path: &std::path::Path, quiet: bool) -> Result<()> {
    qprintln!(quiet, "\n🔗 Relationship Analysis:");

    // Check for binary dependency graph
    let graph_db_path = db_path.join("dependency_graph.bin");
    if graph_db_path.exists() {
        match std::fs::read(&graph_db_path) {
            Ok(graph_binary) => {
                match bincode::deserialize::<
                    kotadb::dependency_extractor::SerializableDependencyGraph,
                >(&graph_binary)
                {
                    Ok(serializable) => {
                        qprintln!(quiet, "   Path: {:?}", graph_db_path);
                        qprintln!(
                            quiet,
                            "   Total relationships: {}",
                            serializable.stats.edge_count
                        );
                        qprintln!(
                            quiet,
                            "   Total symbols in graph: {}",
                            serializable.stats.node_count
                        );
                    }
                    Err(e) => {
                        qprintln!(
                            quiet,
                            "   ⚠️  Failed to deserialize dependency graph: {}",
                            e
                        );
                        qprintln!(
                            quiet,
                            "   Unable to read dependency graph. Re-index to rebuild."
                        );
                    }
                }
            }
            Err(e) => {
                qprintln!(quiet, "   ⚠️  Failed to read dependency graph: {}", e);
                qprintln!(
                    quiet,
                    "   Unable to read dependency graph. Re-index to rebuild."
                );
            }
        }
    } else {
        qprintln!(quiet, "   No dependency graph found.");
        qprintln!(
            quiet,
            "\n   💡 Tip: To build dependency graph, re-index with symbol extraction:"
        );
        qprintln!(quiet, "      kotadb ingest-repo /path/to/repo");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args first to get verbose flag
    let cli = Cli::parse();

    // Initialize logging with appropriate level based on verbose/quiet flags
    let _ = init_logging_with_level(cli.verbose, cli.quiet); // Ignore error if already initialized

    // Store quiet flag for use in output
    let quiet = cli.quiet;

    // Run everything within trace context
    with_trace_id("kotadb-cli", async move {
        // Initialize database
        let db = Database::new(&cli.db_path, cli.binary_index).await?;

        match cli.command {
            Commands::Serve { port } => {
                // Create storage for the HTTP server
                let storage_path = cli.db_path.join("storage");
                std::fs::create_dir_all(&storage_path)?;

                let storage = create_file_storage(
                    storage_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid storage path: {:?}", storage_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?;

                // Wrap storage with observability and validation
                let wrapped_storage = create_wrapped_storage(storage, 1000).await;
                let shared_storage = Arc::new(tokio::sync::Mutex::new(wrapped_storage));

                println!("🚀 Starting KotaDB HTTP server on port {port}");
                println!("📄 API endpoints:");
                println!("   POST   /documents       - Create document");
                println!("   GET    /documents/:id   - Get document");
                println!("   PUT    /documents/:id   - Update document");
                println!("   DELETE /documents/:id   - Delete document");
                println!("   GET    /documents/search - Search documents");
                println!("   GET    /health         - Health check");
                println!();

                start_server(shared_storage, port).await?;
            }


            Commands::SearchCode { query, limit, tags, context } => {
                // Handle empty query explicitly - return nothing with informative message
                if query.is_empty() {
                    println!("Empty search query provided. Please specify a search term.");
                    println!("Use '*' for wildcard search or provide specific code/symbol patterns.");
                    return Ok(());
                }

                // Use LLM-optimized search for non-wildcard queries when content is not minimal
                if query != "*" && context != "none" {
                    // Try LLM-optimized search with fallback to regular search on error
                    let llm_search_result = async {
                        // Create LLM search engine with appropriate context configuration
                        let context_config = match context.as_str() {
                            "none" | "minimal" => kotadb::llm_search::ContextConfig {
                                token_budget: 2000,
                                max_snippet_chars: 200,
                                match_context_size: 30,
                                ..Default::default()
                            },
                            "medium" => kotadb::llm_search::ContextConfig {
                                token_budget: 4000,
                                max_snippet_chars: 500,
                                match_context_size: 50,
                                ..Default::default()
                            },
                            "full" => kotadb::llm_search::ContextConfig {
                                token_budget: 8000,
                                max_snippet_chars: 1000,
                                match_context_size: 100,
                                ..Default::default()
                            },
                            _ => kotadb::llm_search::ContextConfig::default(),
                        };

                        let llm_engine = kotadb::llm_search::LLMSearchEngine::with_config(
                            kotadb::llm_search::RelevanceConfig::default(),
                            context_config,
                        );

                        // Perform LLM-optimized search
                        let storage = db.storage.lock().await;
                        let trigram_index = db.trigram_index.lock().await;
                        llm_engine.search_optimized(
                            &query,
                            &*storage,
                            &*trigram_index,
                            Some(limit)
                        ).await
                    }.await;

                    match llm_search_result {
                        Ok(response) => {

                    // Format output based on context level
                    match context.as_str() {
                        "none" => {
                            // Ultra-minimal: just paths
                            for result in &response.results {
                                println!("{}", result.path);
                            }
                        }
                        "minimal" => {
                            // Minimal: paths with relevance scores
                            if !quiet {
                                println!("Found {} matches in {} files (showing top {}):",
                                    response.optimization.total_matches,
                                    response.optimization.total_matches,
                                    response.results.len());
                                println!();

                                for result in &response.results {
                                    println!("{} (score: {:.2})", result.path, result.relevance_score);
                                }
                            } else {
                                // In quiet mode, only show paths
                                for result in &response.results {
                                    println!("{}", result.path);
                                }
                            }
                        }
                        "medium" => {
                            // Medium: the dream workflow format from issue #370
                            // Count unique files in results
                            let unique_files: std::collections::HashSet<_> =
                                response.results.iter().map(|r| &r.path).collect();
                            let file_count = unique_files.len();

                            if !quiet {
                                println!("Found {} matches in {} files (showing top {}):",
                                    response.optimization.total_matches,
                                    file_count,
                                    response.results.len().min(3));
                                println!();
                            }

                            for (i, result) in response.results.iter().enumerate().take(3) {
                                // Extract line numbers from first match location if available
                                // Note: Line numbers are estimates based on average line length
                                // For exact line numbers, we'd need to load and parse the full file content
                                let line_range = if !result.match_details.exact_matches.is_empty() {
                                    let first_match = &result.match_details.exact_matches[0];
                                    let last_match = result.match_details.exact_matches.last().unwrap();

                                    // Better estimation: Use content snippet to calculate actual lines if possible
                                    let snippet_lines = result.content_snippet.lines().count();
                                    let avg_line_len = if snippet_lines > 0 {
                                        result.content_snippet.len() / snippet_lines.max(1)
                                    } else {
                                        50 // Default average line length
                                    };

                                    let start_line = (first_match.start_offset / avg_line_len.max(1)) + 1;
                                    let end_line = (last_match.end_offset / avg_line_len.max(1)) + 1;

                                    if start_line == end_line {
                                        format!(":{}", start_line)
                                    } else {
                                        format!(":{}-{}", start_line, end_line)
                                    }
                                } else if !result.match_details.term_matches.is_empty() {
                                    let first_match = &result.match_details.term_matches[0];
                                    let snippet_lines = result.content_snippet.lines().count();
                                    let avg_line_len = if snippet_lines > 0 {
                                        result.content_snippet.len() / snippet_lines.max(1)
                                    } else {
                                        50
                                    };
                                    format!(":{}", (first_match.start_offset / avg_line_len.max(1)) + 1)
                                } else {
                                    String::new()
                                };

                                // Check if this looks like a structured code snippet with line numbers
                                let has_line_numbers = result.content_snippet.starts_with("// Line");
                                if has_line_numbers {
                                    // New structured format as requested in issue #413
                                    println!("File: {}", result.path);
                                    if !result.content_snippet.is_empty() {
                                        println!("```rust");
                                        println!("{}", result.content_snippet.trim());
                                        println!("```");
                                    }
                                } else {
                                    // Legacy format for backward compatibility
                                    println!("{}{} (score: {:.2})", result.path, line_range, result.relevance_score);

                                    // Show content snippet with proper indentation
                                    if !result.content_snippet.is_empty() {
                                        // Clean up the snippet for better presentation
                                        let snippet = result.content_snippet
                                            .trim_start_matches("...")
                                            .trim_end_matches("...")
                                            .trim();

                                        for line in snippet.lines() {
                                            println!("  {}", line);
                                        }

                                        // Add ellipsis if content was truncated
                                        if result.content_snippet.ends_with("...") {
                                            println!("    ...");
                                        }
                                    }
                                }

                                if i < 2 && i < response.results.len() - 1 {
                                    println!();
                                }
                            }

                            if response.results.len() > 3 {
                                println!();
                                println!("[Run with --context=full for all results]");
                            }
                        }
                        _ => {
                            // Full: all results with complete context (default for "full" and unrecognized values)
                            // Add memory safeguard: limit results if too many
                            const MAX_FULL_CONTEXT_RESULTS: usize = 100;
                            let results_to_show = if response.results.len() > MAX_FULL_CONTEXT_RESULTS {
                                eprintln!("Warning: Limiting output to {} results to prevent excessive memory usage", MAX_FULL_CONTEXT_RESULTS);
                                &response.results[..MAX_FULL_CONTEXT_RESULTS]
                            } else {
                                &response.results[..]
                            };

                            if !quiet {
                                println!("Found {} matches in {} files (showing {}):",
                                    response.optimization.total_matches,
                                    response.optimization.total_matches,
                                    results_to_show.len());
                                println!();
                            }

                            for result in results_to_show {
                                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                println!("📄 {}", result.path);
                                println!("   Score: {:.2} | Tokens: ~{}",
                                    result.relevance_score,
                                    result.estimated_tokens);

                                // Show match details
                                println!("   Matches: {} exact, {} terms",
                                    result.match_details.exact_matches.len(),
                                    result.match_details.term_matches.len());

                                // Show context info if available
                                if !result.context_info.callees.is_empty() {
                                    println!("   Calls: {}", result.context_info.callees.join(", "));
                                }
                                if !result.context_info.related_types.is_empty() {
                                    println!("   Types: {}", result.context_info.related_types.join(", "));
                                }

                                println!();

                                // Check if this is a structured code snippet and format accordingly
                                let has_line_numbers = result.content_snippet.starts_with("// Line");
                                if has_line_numbers {
                                    // Enhanced structured format for code
                                    println!("```rust");
                                    println!("{}", result.content_snippet.trim());
                                    println!("```");
                                } else {
                                    // Legacy content display
                                    println!("Content:");
                                    for line in result.content_snippet.lines() {
                                        println!("  {}", line);
                                    }
                                }
                                println!();
                            }

                            // Show optimization info
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            println!("Search completed in {}ms", response.metadata.query_time_ms);
                            println!("Token usage: {}/{} ({:.0}%)",
                                response.optimization.token_usage.estimated_tokens,
                                response.optimization.token_usage.budget,
                                response.optimization.token_usage.efficiency * 100.0);

                            // Show suggestions if any
                            if !response.metadata.suggestions.is_empty() {
                                println!();
                                println!("Suggestions:");
                                for suggestion in &response.metadata.suggestions {
                                    println!("  • {}", suggestion);
                                }
                            }
                        }
                    }
                        }
                        Err(e) => {
                            // Log the error and fall back to regular search (suppress in quiet mode)
                            if !quiet {
                                eprintln!("Warning: LLM search failed, falling back to regular search: {}", e);
                            }

                            // Fall back to regular search
                            let tag_list = tags.clone().map(|t| t.split(',').map(String::from).collect());
                            let (results, total_count) = db.search_with_count(&query, tag_list, limit).await?;

                            if results.is_empty() {
                                if !quiet {
                                    println!("No documents found matching the query");
                                }
                            } else {
                                // Show results in simple format as fallback
                                if !quiet {
                                    if results.len() < total_count {
                                        println!("Showing {} of {} results (fallback mode)", results.len(), total_count);
                                    } else {
                                        println!("Found {} documents (fallback mode)", results.len());
                                    }
                                    println!();
                                }
                                for doc in results {
                                    println!("{}", doc.path.as_str());
                                    if context != "none" && !quiet {
                                        println!("  id: {}", doc.id.as_uuid());
                                        println!("  title: {}", doc.title.as_str());
                                        println!("  size: {} bytes", doc.size);
                                        println!();
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Fall back to original search for wildcard or when context is none
                    let tag_list = tags.map(|t| t.split(',').map(String::from).collect());
                    let (results, total_count) = db.search_with_count(&query, tag_list, limit).await?;

                    if results.is_empty() {
                        if !quiet {
                            println!("No documents found matching the query");
                        }
                    } else {
                        // Show clear count information for LLM agents (suppress in quiet mode)
                        if !quiet {
                            if results.len() < total_count {
                                println!("Showing {} of {} results", results.len(), total_count);
                            } else {
                                println!("Found {} documents", results.len());
                            }
                            println!();
                        }
                        for doc in results {
                            // Minimal output optimized for LLM consumption
                            println!("{}", doc.path.as_str());
                            if context != "none" && !quiet {
                                println!("  id: {}", doc.id.as_uuid());
                                println!("  title: {}", doc.title.as_str());
                                println!("  size: {} bytes", doc.size);
                                println!();
                            }
                        }
                    }
                }
            }


            Commands::Stats { basic, symbols, relationships } => {
                // Determine what to show with explicit flag precedence
                // If no flags specified, show everything
                let no_flags_specified = !basic && !symbols && !relationships;
                let show_basic = basic || no_flags_specified;
                let show_symbols = symbols || no_flags_specified;
                let show_relationships = relationships || no_flags_specified;

                // Show basic document statistics
                if show_basic {
                    let (count, total_size) = db.stats().await?;
                    qprintln!(quiet, "📊 Database Statistics");
                    qprintln!(quiet, "═══════════════════════════════");
                    qprintln!(quiet, "\n📄 Document Storage:");
                    qprintln!(quiet, "   Total documents: {count}");
                    qprintln!(quiet, "   Total size: {total_size} bytes");
                    if count > 0 {
                        qprintln!(quiet, "   Average size: {} bytes", total_size / count);
                    }
                }

                // Show symbol statistics (if tree-sitter feature is enabled)
                #[cfg(feature = "tree-sitter-parsing")]
                if show_symbols {
                    show_symbol_statistics(&cli.db_path, quiet).await?;
                }

                // Show relationship statistics
                #[cfg(feature = "tree-sitter-parsing")]
                if show_relationships {
                    show_relationship_statistics(&cli.db_path, quiet).await?;
                }

                // Add helpful tips and next steps
                #[cfg(feature = "tree-sitter-parsing")]
                if show_symbols || show_relationships {
                    let symbol_db_path = cli.db_path.join("symbols.kota");
                    if !symbol_db_path.exists() {
                        qprintln!(quiet, "\n❌ No symbols found in database.");
                        qprintln!(quiet, "   Required steps:");
                        qprintln!(quiet, "   1. Index a codebase: kotadb ingest-repo /path/to/repo");
                        qprintln!(quiet, "   2. Verify indexing: kotadb stats --symbols");
                    } else {
                        // Show success tips if symbols exist
                        let reader = kotadb::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
                        let binary_symbol_count = reader.symbol_count();

                        if binary_symbol_count > 0 {
                            qprintln!(quiet, "\n✅ Codebase intelligence ready! Try these commands:");
                            qprintln!(quiet, "   • find-callers <symbol>     - Find what calls a function");
                            qprintln!(quiet, "   • analyze-impact <symbol>   - Analyze change impact");
                            qprintln!(quiet, "   • search-symbols <pattern>  - Search code symbols");
                            qprintln!(quiet, "   • search-code <query>       - Full-text code search");
                        }
                    }
                }
            }

            Commands::Validate => {
                qprintln!(quiet, "🔍 Running search functionality validation...");

                let validation_result = {
                    let storage = db.storage.lock().await;
                    let primary_index = db.primary_index.lock().await;
                    let trigram_index = db.trigram_index.lock().await;
                    validate_post_ingestion_search(&*storage, &*primary_index, &*trigram_index).await?
                };

                // Display detailed results
                qprintln!(quiet, "\n📋 Validation Results:");
                qprintln!(quiet, "   Status: {}", match validation_result.overall_status {
                    ValidationStatus::Passed => "✅ PASSED",
                    ValidationStatus::Warning => "⚠️ WARNING",
                    ValidationStatus::Failed => "❌ FAILED",
                });
                qprintln!(quiet, "   Checks: {}/{} passed", validation_result.passed_checks, validation_result.total_checks);

                // Show individual check results
                for check in &validation_result.check_results {
                    let status_icon = if check.passed { "✅" } else { "❌" };
                    let critical_mark = if check.critical { " [CRITICAL]" } else { "" };
                    println!("   {} {}{}", status_icon, check.name, critical_mark);
                    if let Some(ref details) = check.details {
                        println!("      {}", details);
                    }
                    if let Some(ref error) = check.error {
                        println!("      Error: {}", error);
                    }
                }

                // Show issues and recommendations
                if !validation_result.issues.is_empty() {
                    println!("\n🚨 Issues Found:");
                    for issue in &validation_result.issues {
                        println!("   - {}", issue);
                    }
                }

                if !validation_result.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &validation_result.recommendations {
                        println!("   • {}", rec);
                    }
                }

                // Show warnings if any
                if !validation_result.warnings.is_empty() {
                    println!("\n⚠️ Warnings:");
                    for warning in &validation_result.warnings {
                        println!("   - {}", warning);
                    }
                }

                // Exit with error code if validation failed
                if validation_result.overall_status == ValidationStatus::Failed {
                    return Err(anyhow::anyhow!("Search validation failed"));
                }
            }

            Commands::VerifyDocs => {
                use kotadb::DocumentationVerifier;

                println!("📋 Running comprehensive documentation verification...");
                println!("   Checking claims vs actual implementation");
                println!();

                let verifier = DocumentationVerifier::new();
                let report = verifier.run_full_verification()?;

                println!("📊 Verification Results:");
                println!("   {}", report.summary);
                println!();

                // Show verification status
                if report.is_acceptable() {
                    println!("✅ Documentation accuracy is acceptable");
                } else {
                    println!("❌ Documentation accuracy needs improvement");
                }

                // Show detailed check results
                println!("\n📝 Feature Verification Details:");
                for check in &report.checks {
                    let status_icon = match check.status {
                        kotadb::VerificationStatus::Verified => "✅",
                        kotadb::VerificationStatus::Missing => "❌",
                        kotadb::VerificationStatus::Partial => "⚠️",
                        kotadb::VerificationStatus::Undocumented => "📝",
                    };

                    let severity_badge = match check.severity {
                        kotadb::Severity::Critical => " [CRITICAL]",
                        kotadb::Severity::High => " [HIGH]",
                        kotadb::Severity::Medium => " [MEDIUM]",
                        _ => "",
                    };

                    println!("   {} {}{}", status_icon, check.feature, severity_badge);
                    println!("      Claim: {}", check.documented_claim);
                    println!("      Reality: {}", check.actual_implementation);

                    if let Some(ref rec) = check.recommendation {
                        println!("      💡 Recommendation: {}", rec);
                    }
                    println!();
                }

                // Show critical issues
                if !report.critical_issues.is_empty() {
                    println!("🚨 Critical Issues Found:");
                    for issue in &report.critical_issues {
                        println!("   - {}", issue);
                    }
                    println!();
                }

                // Show recommendations
                if !report.recommendations.is_empty() {
                    println!("💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("   • {}", rec);
                    }
                    println!();
                }

                // Exit with error code if documentation is unacceptable
                if !report.is_acceptable() {
                    return Err(anyhow::anyhow!(
                        "Documentation verification failed. {} critical issues found.",
                        report.critical_issues.len()
                    ));
                }

                println!("✨ Documentation verification completed successfully!");
            }

            #[cfg(feature = "git-integration")]
            Commands::IndexCodebase {
                repo_path,
                prefix,
                include_files,
                include_commits,
                max_file_size_mb,
                max_memory_mb,
                max_parallel_files,
                enable_chunking,
                #[cfg(feature = "tree-sitter-parsing")]
                extract_symbols,
                #[cfg(feature = "tree-sitter-parsing")]
                no_symbols,
            } => {
                use indicatif::{ProgressBar, ProgressStyle};
                use kotadb::git::types::IngestionOptions;
                use kotadb::git::{IngestionConfig, ProgressCallback, RepositoryIngester};

                qprintln!(quiet, "🔄 Ingesting git repository: {:?}", repo_path);

                // Determine if symbols should be extracted
                #[cfg(feature = "tree-sitter-parsing")]
                let should_extract_symbols = if no_symbols {
                    qprintln!(quiet, "⚠️  Symbol extraction disabled via --no-symbols flag");
                    false
                } else if let Some(extract) = extract_symbols {
                    if extract {
                        qprintln!(quiet, "✅ Symbol extraction enabled via --extract-symbols flag");
                    } else {
                        qprintln!(quiet, "⚠️  Symbol extraction disabled via --extract-symbols=false");
                    }
                    extract
                } else {
                    qprintln!(quiet, "✅ Symbol extraction enabled (default with tree-sitter feature)");
                    true // Default to true when tree-sitter is available
                };

                // Configure memory limits if specified
                let memory_limits = if max_memory_mb.is_some() || max_parallel_files.is_some() || !enable_chunking {
                    Some(kotadb::memory::MemoryLimitsConfig {
                        max_total_memory_mb: max_memory_mb,
                        max_parallel_files,
                        enable_adaptive_chunking: enable_chunking,
                        chunk_size: if enable_chunking { 50 } else { usize::MAX },
                    })
                } else {
                    None
                };

                // Configure ingestion options
                #[allow(unused_mut)]
                let mut options = IngestionOptions {
                    include_file_contents: include_files,
                    include_commit_history: include_commits,
                    max_file_size: max_file_size_mb * 1024 * 1024,
                    memory_limits,
                    ..Default::default()
                };

                #[cfg(feature = "tree-sitter-parsing")]
                {
                    options.extract_symbols = should_extract_symbols;
                }

                let config = IngestionConfig {
                    path_prefix: prefix,
                    options,
                    create_index: true,
                    organization_config: Some(kotadb::git::RepositoryOrganizationConfig::default()),
                };

                // Create progress bar (disabled in quiet mode)
                let progress_bar = if quiet {
                    ProgressBar::hidden()
                } else {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.green} {msg}")
                            .expect("Valid template")
                            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                    );
                    pb.set_message("Initializing...");
                    pb
                };

                // Create progress callback
                let pb = progress_bar.clone();
                let progress_callback: ProgressCallback = Box::new(move |message: &str| {
                    pb.set_message(message.to_string());
                    pb.tick();
                });

                // Create ingester and run ingestion with progress
                let ingester = RepositoryIngester::new(config.clone());
                let mut storage = db.storage.lock().await;

                #[cfg(feature = "tree-sitter-parsing")]
                let result = if config.options.extract_symbols {
                    // Use binary format for efficient symbol storage with automatic dependency graph and relationship building
                    let symbol_db_path = cli.db_path.join("symbols.kota");
                    let graph_db_path = cli.db_path.join("dependency_graph.bin");
                    ingester.ingest_with_binary_symbols_and_relationships(
                        &repo_path,
                        &mut *storage,
                        &symbol_db_path,
                        &graph_db_path,
                        Some(progress_callback),
                    ).await?
                } else {
                    ingester.ingest_with_progress(&repo_path, &mut *storage, Some(progress_callback)).await?
                };

                #[cfg(not(feature = "tree-sitter-parsing"))]
                let result = ingester.ingest_with_progress(&repo_path, &mut **storage, Some(progress_callback)).await?;

                progress_bar.finish_with_message("✅ Ingestion complete");

                // Release the storage lock before rebuilding indices
                drop(storage);

                // Rebuild indices and cache after ingestion with progress indication
                let rebuild_progress = if quiet {
                    ProgressBar::hidden()
                } else {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.blue} {msg}")
                            .expect("Valid template")
                            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                    );
                    pb
                };

                rebuild_progress.set_message("Rebuilding primary and trigram indices...");
                db.rebuild_indices().await?;

                rebuild_progress.set_message("Rebuilding path cache...");
                db.rebuild_path_cache().await?;

                rebuild_progress.finish_with_message("✅ Indices rebuilt");

                // Ensure all async operations are complete before validation
                qprintln!(quiet, "⏳ Ensuring index synchronization...");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Explicit flush verification
                {
                    let mut storage = db.storage.lock().await;
                    let mut primary_index = db.primary_index.lock().await;
                    let mut trigram_index = db.trigram_index.lock().await;
                    storage.flush().await?;
                    primary_index.flush().await?;
                    trigram_index.flush().await?;
                }

                // Validate search functionality after ingestion
                let validation_progress = if quiet {
                    ProgressBar::hidden()
                } else {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.yellow} {msg}")
                            .expect("Valid template")
                            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                    );
                    pb
                };

                validation_progress.set_message("Running search validation tests...");
                let validation_result = {
                    let storage = db.storage.lock().await;
                    let primary_index = db.primary_index.lock().await;
                    let trigram_index = db.trigram_index.lock().await;
                    validate_post_ingestion_search(&*storage, &*primary_index, &*trigram_index).await?
                };

                validation_progress.finish_with_message("✅ Validation complete");

                // Report validation results
                match validation_result.overall_status {
                    ValidationStatus::Passed => {
                        qprintln!(quiet, "✅ Search validation passed: All systems operational");
                    }
                    ValidationStatus::Warning => {
                        qprintln!(quiet, "⚠️ Search validation completed with warnings:");
                        for issue in &validation_result.issues {
                            qprintln!(quiet, "   - {}", issue);
                        }
                        qprintln!(quiet, "   Recommendations:");
                        for rec in &validation_result.recommendations {
                            qprintln!(quiet, "   • {}", rec);
                        }
                    }
                    ValidationStatus::Failed => {
                        qprintln!(quiet, "❌ Search validation failed - ingestion may not be fully operational:");
                        for issue in &validation_result.issues {
                            qprintln!(quiet, "   - {}", issue);
                        }
                        qprintln!(quiet, "   Recommendations:");
                        for rec in &validation_result.recommendations {
                            qprintln!(quiet, "   • {}", rec);
                        }

                        // Return error for critical failures
                        return Err(anyhow::anyhow!(
                            "Post-ingestion search validation failed. Search functionality is broken."
                        ));
                    }
                }

                // Show warnings for git ingestion
                if !validation_result.warnings.is_empty() {
                    qprintln!(quiet, "   Validation warnings:");
                    for warning in &validation_result.warnings {
                        qprintln!(quiet, "   ⚠️ {}", warning);
                    }
                }

                qprintln!(quiet, "✅ Repository ingestion complete!");
                qprintln!(quiet, "   Documents created: {}", result.documents_created);
                qprintln!(quiet, "   Files ingested: {}", result.files_ingested);
                qprintln!(quiet, "   Commits ingested: {}", result.commits_ingested);
                if result.symbols_extracted > 0 {
                    qprintln!(quiet, "   Symbols extracted: {} from {} files", result.symbols_extracted, result.files_with_symbols);
                }
                if result.errors > 0 {
                    qprintln!(quiet, "   ⚠️ Errors encountered: {}", result.errors);
                }

                // Show validation summary
                qprintln!(quiet, "   Validation: {} ({}/{})",
                    validation_result.summary(),
                    validation_result.passed_checks,
                    validation_result.total_checks
                );
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::SearchSymbols { pattern, limit, symbol_type } => {
                // Use binary symbols which is where IndexCodebase stores them
                let symbol_db_path = cli.db_path.join("symbols.kota");

                if !symbol_db_path.exists() {
                    println!("❌ No symbols found in database.");
                    println!("   Required steps:");
                    println!("   1. Index a codebase: kotadb index-codebase /path/to/repo");
                    println!("   2. Verify indexing: kotadb symbol-stats");
                    println!("   3. Then search: kotadb search-symbols 'pattern'");
                    return Ok(());
                }

                // Open binary symbol reader for efficient searching
                let reader = kotadb::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
                let total_symbols = reader.symbol_count();

                if total_symbols == 0 {
                    println!("No symbols in database. Index a codebase first with: kotadb index-codebase /path/to/repo");
                    return Ok(());
                }

                // Search symbols
                let mut matches = Vec::new();
                let pattern_lower = pattern.to_lowercase();

                for packed_symbol in reader.iter_symbols() {
                    // Get the symbol name
                    if let Ok(symbol_name) = reader.get_symbol_name(&packed_symbol) {
                        let symbol_name_lower = symbol_name.to_lowercase();

                        // Match against pattern - check for wildcards first, then substring
                        let is_match = if pattern_lower.contains('*') {
                            // Use wildcard pattern matching if pattern contains '*'
                            matches_wildcard_pattern(&symbol_name_lower, &pattern_lower)
                        } else {
                            // Use substring matching for patterns without wildcards
                            symbol_name_lower.contains(&pattern_lower)
                        };

                        if is_match {
                            // Filter by type if specified
                            if let Some(ref filter_type) = symbol_type {
                                let filter_lower = filter_type.to_lowercase();
                                let type_str = format!("{}", packed_symbol.kind).to_lowercase();
                                if !type_str.contains(&filter_lower) {
                                    continue;
                                }
                            }

                            // Get file path for display
                            let file_path = reader.get_symbol_file_path(&packed_symbol)
                                .unwrap_or_else(|_| "<unknown>".to_string());

                            matches.push((symbol_name, packed_symbol, file_path));
                            if matches.len() >= limit {
                                break;
                            }
                        }
                    }
                }

                if matches.is_empty() {
                    if !quiet {
                        println!("No symbols found matching '{}'", pattern);
                        if let Some(ref st) = symbol_type {
                            println!("  with type filter: {}", st);
                        }
                        println!("  Total symbols in database: {}", total_symbols);
                    }
                } else {
                    if !quiet {
                        println!("Found {} matching symbols", matches.len());
                        if matches.len() == limit {
                            println!("(showing first {}, use -l for more)", limit);
                        }
                        println!();
                    }

                    for (name, symbol, file_path) in matches {
                        println!("{}", name);
                        if !quiet {
                            println!("  type: {}", symbol.kind);
                            println!("  file: {}", file_path);
                            println!("  line: {}", symbol.start_line);
                            println!();
                        }
                    }
                }
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::FindCallers { target, limit } => {
                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let query_type = RelationshipQueryType::FindCallers {
                    target: target.clone(),
                };

                let mut result = relationship_engine.execute_query(query_type).await?;

                // Apply limit if specified
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                if quiet {
                    // In quiet mode, output minimal information
                    let markdown = result.to_markdown();
                    for line in markdown.lines() {
                        if line.starts_with("- ") {
                            println!("{}", line.trim_start_matches("- "));
                        }
                    }
                } else {
                    println!("{}", result.to_markdown());
                }
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::AnalyzeImpact { target, limit } => {
                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let query_type = RelationshipQueryType::ImpactAnalysis {
                    target: target.clone(),
                };

                let mut result = relationship_engine.execute_query(query_type).await?;

                // Apply limit if specified
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                if quiet {
                    // In quiet mode, output minimal information
                    let markdown = result.to_markdown();
                    for line in markdown.lines() {
                        if line.starts_with("- ") {
                            println!("{}", line.trim_start_matches("- "));
                        }
                    }
                } else {
                    println!("{}", result.to_markdown());
                }
            }

            #[cfg(feature = "tree-sitter-parsing")]
            Commands::RelationshipQuery { query, limit } => {
                let query_type = parse_natural_language_relationship_query(&query)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Could not parse query '{}'\n\
                            Valid query patterns:\n\
                            - what calls [symbol]?\n\
                            - what would break if I change [symbol]?\n\
                            - find unused functions\n\
                            - who uses [symbol]?\n\
                            - find callers of [symbol]",
                            query
                        )
                    })?;

                let relationship_engine = create_relationship_engine(&cli.db_path).await?;
                let mut result = relationship_engine.execute_query(query_type).await?;
                if let Some(limit_value) = limit {
                    result.limit_results(limit_value);
                }
                if quiet {
                    // In quiet mode, output minimal information
                    let markdown = result.to_markdown();
                    for line in markdown.lines() {
                        if line.starts_with("- ") {
                            println!("{}", line.trim_start_matches("- "));
                        }
                    }
                } else {
                    println!("{}", result.to_markdown());
                }
            }


            Commands::Benchmark {
                operations,
                benchmark_type,
                format,
                max_search_queries,
            } => {
                qprintln!(quiet, "\n🚀 Running KotaDB Benchmarks");
                qprintln!(quiet, "   Operations: {}", operations);
                qprintln!(quiet, "   Type: {}", benchmark_type);
                qprintln!(quiet, "   Format: {}", format);

                let database = Database::new(&cli.db_path, cli.binary_index).await?;
                run_benchmarks(
                    database,
                    operations,
                    &benchmark_type,
                    &format,
                    max_search_queries,
                    quiet,
                ).await?;
            }
        }

        Ok::<(), anyhow::Error>(())
    })
    .await
}

#[cfg(test)]
mod stats_tests {

    #[test]
    fn test_stats_flag_logic_no_flags() {
        // When no flags are specified, should show everything
        let (basic, symbols, relationships) = (false, false, false);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(show_basic, "Should show basic when no flags specified");
        assert!(show_symbols, "Should show symbols when no flags specified");
        assert!(
            show_relationships,
            "Should show relationships when no flags specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_basic_only() {
        // When only --basic is specified, should show only basic
        let (basic, symbols, relationships) = (true, false, false);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(show_basic, "Should show basic when --basic specified");
        assert!(
            !show_symbols,
            "Should not show symbols when only --basic specified"
        );
        assert!(
            !show_relationships,
            "Should not show relationships when only --basic specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_symbols_only() {
        // When only --symbols is specified, should show only symbols
        let (basic, symbols, relationships) = (false, true, false);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(
            !show_basic,
            "Should not show basic when only --symbols specified"
        );
        assert!(show_symbols, "Should show symbols when --symbols specified");
        assert!(
            !show_relationships,
            "Should not show relationships when only --symbols specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_relationships_only() {
        // When only --relationships is specified, should show only relationships
        let (basic, symbols, relationships) = (false, false, true);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(
            !show_basic,
            "Should not show basic when only --relationships specified"
        );
        assert!(
            !show_symbols,
            "Should not show symbols when only --relationships specified"
        );
        assert!(
            show_relationships,
            "Should show relationships when --relationships specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_basic_and_symbols() {
        // When --basic and --symbols are specified, should show both
        let (basic, symbols, relationships) = (true, true, false);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(
            show_basic,
            "Should show basic when --basic and --symbols specified"
        );
        assert!(
            show_symbols,
            "Should show symbols when --basic and --symbols specified"
        );
        assert!(
            !show_relationships,
            "Should not show relationships when only --basic and --symbols specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_symbols_and_relationships() {
        // When --symbols and --relationships are specified, should show both
        let (basic, symbols, relationships) = (false, true, true);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(
            !show_basic,
            "Should not show basic when only --symbols and --relationships specified"
        );
        assert!(
            show_symbols,
            "Should show symbols when --symbols and --relationships specified"
        );
        assert!(
            show_relationships,
            "Should show relationships when --symbols and --relationships specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_basic_and_relationships() {
        // When --basic and --relationships are specified, should show both
        let (basic, symbols, relationships) = (true, false, true);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(
            show_basic,
            "Should show basic when --basic and --relationships specified"
        );
        assert!(
            !show_symbols,
            "Should not show symbols when only --basic and --relationships specified"
        );
        assert!(
            show_relationships,
            "Should show relationships when --basic and --relationships specified"
        );
    }

    #[test]
    fn test_stats_flag_logic_all_flags() {
        // When all flags are specified, should show everything
        let (basic, symbols, relationships) = (true, true, true);

        let no_flags_specified = !basic && !symbols && !relationships;
        let show_basic = basic || no_flags_specified;
        let show_symbols = symbols || no_flags_specified;
        let show_relationships = relationships || no_flags_specified;

        assert!(show_basic, "Should show basic when all flags specified");
        assert!(show_symbols, "Should show symbols when all flags specified");
        assert!(
            show_relationships,
            "Should show relationships when all flags specified"
        );
    }
}
