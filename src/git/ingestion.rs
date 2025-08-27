//! Git repository ingestion into KotaDB

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, instrument, warn};

use crate::builders::DocumentBuilder;
use crate::git::file_organization::FileOrganizationManager;
use crate::git::repository::GitRepository;
use crate::git::types::{CommitInfo, FileEntry, IngestionOptions};
use crate::Document;
use crate::Storage;

// For parallel processing
use futures::stream::StreamExt;
use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

// Symbol extraction imports
#[cfg(feature = "tree-sitter-parsing")]
use crate::binary_symbols::BinarySymbolWriter;
#[cfg(feature = "tree-sitter-parsing")]
use crate::parsing::{CodeParser, SupportedLanguage};
#[cfg(feature = "tree-sitter-parsing")]
use crate::symbol_storage::SymbolStorage;

/// Progress update callback type
pub type ProgressCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Configuration for repository ingestion
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Prefix for document paths in KotaDB
    pub path_prefix: String,
    /// Repository-specific options
    pub options: IngestionOptions,
    /// Whether to create an index document for the repository
    pub create_index: bool,
    /// Repository file organization configuration
    pub organization_config: Option<crate::git::document_metadata::RepositoryOrganizationConfig>,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            path_prefix: "repos".to_string(),
            options: IngestionOptions::default(),
            create_index: true,
            organization_config: Some(
                crate::git::document_metadata::RepositoryOrganizationConfig::default(),
            ),
        }
    }
}

/// Ingests git repositories into KotaDB
pub struct RepositoryIngester {
    config: IngestionConfig,
}

impl RepositoryIngester {
    /// Create a new repository ingester
    pub fn new(config: IngestionConfig) -> Self {
        Self { config }
    }

    /// Sanitize repository name for safe filesystem usage
    fn sanitize_name(name: &str) -> String {
        let sanitized = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_lowercase();

        // Ensure we never return an empty string
        if sanitized.is_empty() {
            "repository".to_string()
        } else {
            sanitized
        }
    }

    /// Ingest a git repository into KotaDB storage
    #[instrument(skip(self, storage, repo_path))]
    pub async fn ingest<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
    ) -> Result<IngestResult> {
        self.ingest_with_progress(repo_path, storage, None).await
    }

    /// Ingest a git repository into KotaDB storage with progress reporting
    #[cfg(not(feature = "tree-sitter-parsing"))]
    #[instrument(skip(self, storage, repo_path, progress_callback))]
    pub async fn ingest_with_progress<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<IngestResult> {
        self.ingest_internal(repo_path, storage, progress_callback)
            .await
    }

    /// Ingest a git repository into KotaDB storage with progress reporting and optional symbol extraction
    #[cfg(feature = "tree-sitter-parsing")]
    #[instrument(skip(self, storage, repo_path, progress_callback))]
    pub async fn ingest_with_progress<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<IngestResult> {
        self.ingest_internal(repo_path, storage, progress_callback, None, None)
            .await
    }

    /// Ingest a git repository into KotaDB storage with symbol extraction and progress reporting
    #[cfg(feature = "tree-sitter-parsing")]
    #[instrument(skip(
        self,
        storage,
        repo_path,
        progress_callback,
        symbol_storage,
        code_parser
    ))]
    pub async fn ingest_with_symbols<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        progress_callback: Option<ProgressCallback>,
        symbol_storage: &mut SymbolStorage,
        code_parser: &mut CodeParser,
    ) -> Result<IngestResult> {
        self.ingest_internal(
            repo_path,
            storage,
            progress_callback,
            Some(symbol_storage),
            Some(code_parser),
        )
        .await
    }

    /// Ingest a git repository with binary symbol storage (high-performance)
    #[cfg(feature = "tree-sitter-parsing")]
    pub async fn ingest_with_binary_symbols<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        symbol_db_path: impl AsRef<Path>,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<IngestResult> {
        let repo_path = repo_path.as_ref();
        info!(
            "Starting repository ingestion with binary symbols from: {:?}",
            repo_path
        );

        let report_progress = |message: &str| {
            if let Some(ref callback) = progress_callback {
                callback(message);
            }
        };

        report_progress("Opening repository...");

        // Open the repository
        let repo = GitRepository::open(repo_path, self.config.options.clone())
            .context("Failed to open git repository")?;

        // Get repository metadata
        let metadata = repo
            .metadata()
            .context("Failed to get repository metadata")?;

        let safe_repo_name = Self::sanitize_name(&metadata.name);
        info!(
            "Repository: {} ({} commits)",
            metadata.name, metadata.commit_count
        );

        let mut result = IngestResult::default();

        // Ingest files first
        report_progress("Discovering repository files...");
        let files = repo
            .list_files()
            .context("Failed to list repository files")?;

        info!("Found {} files to ingest", files.len());

        if !files.is_empty() {
            // Phase 1: Insert documents
            report_progress("Phase 1: Inserting documents...");

            for file in &files {
                match self.create_file_document(&safe_repo_name, file) {
                    Ok(doc) => {
                        if let Err(e) = storage.insert(doc).await {
                            warn!("Failed to insert file document {}: {}", file.path, e);
                            result.errors += 1;
                        } else {
                            result.documents_created += 1;
                            result.files_ingested += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create document for {}: {}", file.path, e);
                        result.errors += 1;
                    }
                }
            }

            // Phase 2: Extract symbols with binary format
            if self.config.options.extract_symbols {
                report_progress("Phase 2: Extracting symbols with binary format...");

                let parse_start = std::time::Instant::now();

                // Parse all files in parallel using rayon
                let parsed_symbols: Vec<_> = files
                    .par_iter()
                    .filter_map(|file| {
                        // Skip binary files
                        if file.is_binary {
                            return None;
                        }

                        // Check file extension
                        let extension = file.extension.as_ref()?;
                        let language = SupportedLanguage::from_extension(extension)?;

                        // Convert content to string
                        let content = String::from_utf8(file.content.clone()).ok()?;

                        // Create a local parser for this thread
                        let mut local_parser = CodeParser::new().ok()?;

                        // Parse the file
                        let parsed_code = local_parser.parse_content(&content, language).ok()?;

                        // Return symbols with file context
                        Some((file.path.clone(), parsed_code.symbols))
                    })
                    .collect();

                let parse_elapsed = parse_start.elapsed();
                info!(
                    "Parsed {} files in {:?}",
                    parsed_symbols.len(),
                    parse_elapsed
                );

                // Write symbols to binary format
                report_progress("Writing symbols to binary database...");
                let mut writer = BinarySymbolWriter::new();

                for (file_path, symbols) in parsed_symbols {
                    if !symbols.is_empty() {
                        result.files_with_symbols += 1;
                    }

                    for symbol in symbols {
                        // Convert symbol type to byte representation
                        let kind = match symbol.symbol_type {
                            crate::parsing::SymbolType::Function => 1,
                            crate::parsing::SymbolType::Method => 2,
                            crate::parsing::SymbolType::Class => 3,
                            crate::parsing::SymbolType::Struct => 4,
                            crate::parsing::SymbolType::Enum => 5,
                            crate::parsing::SymbolType::Variable => 6,
                            crate::parsing::SymbolType::Constant => 7,
                            crate::parsing::SymbolType::Module => 8,
                            _ => 0,
                        };

                        // TODO: Implement parent relationship tracking
                        // This requires either:
                        // 1. Two-pass processing to build parent ID map
                        // 2. Maintaining a name->UUID map during processing
                        let parent_id: Option<uuid::Uuid> = None;

                        writer.add_symbol(
                            uuid::Uuid::new_v4(),
                            &symbol.name,
                            kind,
                            &file_path,
                            symbol.start_line as u32,
                            symbol.end_line as u32,
                            parent_id,
                        );

                        result.symbols_extracted += 1;
                    }
                }

                // Write to file
                let write_start = std::time::Instant::now();
                writer.write_to_file(symbol_db_path.as_ref())?;
                let write_elapsed = write_start.elapsed();

                info!(
                    "Wrote {} symbols to binary database in {:?} (total: {:?})",
                    result.symbols_extracted,
                    write_elapsed,
                    parse_elapsed + write_elapsed
                );

                report_progress(&format!(
                    "Symbol extraction complete: {} symbols in {:?}",
                    result.symbols_extracted,
                    parse_elapsed + write_elapsed
                ));
            }
        }

        info!(
            "Binary ingestion complete: {} documents, {} symbols",
            result.documents_created, result.symbols_extracted
        );

        Ok(result)
    }

    /// Internal ingestion method that handles both cases
    #[cfg(feature = "tree-sitter-parsing")]
    #[instrument(skip(
        self,
        storage,
        repo_path,
        progress_callback,
        symbol_storage,
        code_parser
    ))]
    async fn ingest_internal<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        progress_callback: Option<ProgressCallback>,
        mut symbol_storage: Option<&mut SymbolStorage>,
        mut code_parser: Option<&mut CodeParser>,
    ) -> Result<IngestResult> {
        let repo_path = repo_path.as_ref();
        info!("Starting repository ingestion from: {:?}", repo_path);

        // Helper function to report progress
        let report_progress = |message: &str| {
            if let Some(ref callback) = progress_callback {
                callback(message);
            }
        };

        report_progress("Opening repository...");

        // Open the repository
        let repo = GitRepository::open(repo_path, self.config.options.clone())
            .context("Failed to open git repository")?;

        report_progress("Analyzing repository metadata...");

        // Get repository metadata
        let metadata = repo
            .metadata()
            .context("Failed to get repository metadata")?;

        let safe_repo_name = Self::sanitize_name(&metadata.name);
        info!(
            "Repository: {} ({} commits) - using safe name: {}",
            metadata.name, metadata.commit_count, safe_repo_name
        );

        let mut result = IngestResult::default();

        // Initialize file organization manager if configured
        let _file_org_manager = self
            .config
            .organization_config
            .as_ref()
            .map(|org_config| FileOrganizationManager::new(org_config.clone()));

        // Create repository index document if requested
        if self.config.create_index {
            report_progress("Creating repository index document...");
            let index_doc = self.create_index_document(&metadata, &safe_repo_name)?;
            storage
                .insert(index_doc)
                .await
                .context("Failed to insert repository index document")?;
            result.documents_created += 1;
        }

        // Ingest files
        if self.config.options.include_file_contents {
            report_progress("Discovering repository files...");
            let files = repo
                .list_files()
                .context("Failed to list repository files")?;

            info!("Found {} files to ingest", files.len());

            if !files.is_empty() {
                report_progress(&format!("Processing {} files...", files.len()));

                // Optimized parallel processing with symbol extraction
                info!(
                    "Symbol extraction flag: {}",
                    self.config.options.extract_symbols
                );
                if self.config.options.extract_symbols {
                    info!("Entering optimized symbol extraction branch");
                    // When extracting symbols, process in two phases for optimal performance:
                    // Phase 1: Batch insert all documents into storage (leverages dual-index)
                    // Phase 2: Extract symbols in parallel batches

                    report_progress("Phase 1: Inserting documents into storage...");
                    info!("Starting Phase 1");

                    // Process documents in batches of 100 for efficient I/O
                    let doc_batch_size = 100;
                    let mut doc_batch_start = 0;

                    while doc_batch_start < files.len() {
                        let doc_batch_end =
                            std::cmp::min(doc_batch_start + doc_batch_size, files.len());
                        let doc_batch = &files[doc_batch_start..doc_batch_end];

                        // Create documents synchronously (they're not async)
                        let documents_results: Vec<_> = doc_batch
                            .iter()
                            .map(|file| self.create_file_document(&safe_repo_name, file))
                            .collect();

                        // Insert documents
                        for (file, doc_result) in doc_batch.iter().zip(documents_results) {
                            match doc_result {
                                Ok(doc) => {
                                    if let Err(e) = storage.insert(doc).await {
                                        warn!(
                                            "Failed to insert file document {}: {}",
                                            file.path, e
                                        );
                                        result.errors += 1;
                                    } else {
                                        result.documents_created += 1;
                                        result.files_ingested += 1;
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to create document for {}: {}", file.path, e);
                                    result.errors += 1;
                                }
                            }
                        }

                        let progress = (doc_batch_end as f64 / files.len() as f64 * 100.0) as u32;
                        report_progress(&format!(
                            "Inserted documents: {}/{} ({}%)",
                            doc_batch_end,
                            files.len(),
                            progress
                        ));

                        doc_batch_start = doc_batch_end;
                    }

                    // Phase 2: Extract symbols with true parallel processing using rayon
                    if self.config.options.extract_symbols {
                        if let (Some(symbol_storage), Some(_code_parser)) =
                            (symbol_storage.as_mut(), code_parser.as_mut())
                        {
                            report_progress(
                                "Phase 2: Extracting symbols with rayon parallelization...",
                            );
                            info!(
                                "Starting rayon-based parallel symbol extraction for {} files",
                                files.len()
                            );

                            let parse_start = std::time::Instant::now();

                            // Parse all files in parallel using rayon
                            let parsed_files: Vec<_> = files
                                .par_iter()
                                .filter_map(|file| {
                                    // Skip binary files
                                    if file.is_binary {
                                        return None;
                                    }

                                    // Check file extension
                                    let extension = file.extension.as_ref()?;
                                    let language = SupportedLanguage::from_extension(extension)?;

                                    // Convert content to string
                                    let content = String::from_utf8(file.content.clone()).ok()?;

                                    // Create a local parser for this thread
                                    let mut local_parser = CodeParser::new().ok()?;

                                    // Parse the file
                                    let parsed_code =
                                        local_parser.parse_content(&content, language).ok()?;

                                    Some((file.path.clone(), parsed_code))
                                })
                                .collect();

                            let parse_elapsed = parse_start.elapsed();
                            info!(
                                "Parsed {} files in parallel in {:?}",
                                parsed_files.len(),
                                parse_elapsed
                            );
                            report_progress(&format!(
                                "Parsed {} files in {:?}, storing symbols...",
                                parsed_files.len(),
                                parse_elapsed
                            ));

                            // Store symbols sequentially (SymbolStorage requires mutable access)
                            let storage_start = std::time::Instant::now();
                            let mut processed_count = 0;
                            let total_parsed = parsed_files.len();
                            let mut last_progress_time = std::time::Instant::now();
                            info!("Starting to store {} parsed files", total_parsed);

                            for (file_path, parsed_code) in parsed_files {
                                let path = Path::new(&file_path);
                                match symbol_storage
                                    .extract_symbols(
                                        path,
                                        parsed_code,
                                        Some(safe_repo_name.to_string()),
                                    )
                                    .await
                                {
                                    Ok(symbol_ids) => {
                                        let count = symbol_ids.len();
                                        if count > 0 {
                                            result.symbols_extracted += count;
                                            result.files_with_symbols += 1;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to store symbols from {}: {}", file_path, e);
                                        result.errors += 1;
                                    }
                                }

                                processed_count += 1;

                                // Report progress periodically
                                let now = std::time::Instant::now();
                                if now.duration_since(last_progress_time)
                                    >= std::time::Duration::from_millis(500)
                                {
                                    let progress = (processed_count as f64 / total_parsed as f64
                                        * 100.0)
                                        as u32;
                                    report_progress(&format!(
                                        "Storing symbols: {}/{} files ({}%), {} symbols found",
                                        processed_count,
                                        total_parsed,
                                        progress,
                                        result.symbols_extracted
                                    ));
                                    last_progress_time = now;
                                }
                            }

                            let storage_elapsed = storage_start.elapsed();
                            info!("Stored symbols in {:?}", storage_elapsed);
                            report_progress(&format!(
                                "Symbol extraction complete: {} symbols from {} files (parse: {:?}, store: {:?})",
                                result.symbols_extracted,
                                result.files_with_symbols,
                                parse_elapsed,
                                storage_elapsed
                            ));
                        }
                    }
                } else {
                    // Non-symbol extraction path: just insert documents in batches
                    let batch_size = 100;
                    let mut batch_start = 0;
                    let mut last_progress_time = std::time::Instant::now();
                    let progress_throttle = std::time::Duration::from_millis(250);

                    while batch_start < files.len() {
                        let batch_end = std::cmp::min(batch_start + batch_size, files.len());
                        let batch = &files[batch_start..batch_end];

                        // Report progress
                        let now = std::time::Instant::now();
                        if now.duration_since(last_progress_time) >= progress_throttle {
                            let progress = (batch_end as f64 / files.len() as f64 * 100.0) as u32;
                            report_progress(&format!(
                                "Processing files: {}/{} ({}%)",
                                batch_end,
                                files.len(),
                                progress
                            ));
                            last_progress_time = now;
                        }

                        // Create documents synchronously (they're not async)
                        let documents_results: Vec<_> = batch
                            .iter()
                            .map(|file| self.create_file_document(&safe_repo_name, file))
                            .collect();

                        // Insert documents
                        for (file, doc_result) in batch.iter().zip(documents_results) {
                            match doc_result {
                                Ok(doc) => {
                                    if let Err(e) = storage.insert(doc).await {
                                        warn!(
                                            "Failed to insert file document {}: {}",
                                            file.path, e
                                        );
                                        result.errors += 1;
                                    } else {
                                        result.documents_created += 1;
                                        result.files_ingested += 1;
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to create document for {}: {}", file.path, e);
                                    result.errors += 1;
                                }
                            }
                        }

                        batch_start = batch_end;
                    }
                }
            }
        }

        // Ingest commit history
        if self.config.options.include_commit_history {
            report_progress("Loading commit history...");
            let commits = repo
                .get_commits(None)
                .context("Failed to get repository commits")?;

            info!("Processing {} commits", commits.len());

            if !commits.is_empty() {
                report_progress(&format!("Processing {} commits...", commits.len()));

                let mut last_progress_time = std::time::Instant::now();
                let progress_throttle = std::time::Duration::from_millis(250); // Update every 250ms max

                for (index, commit) in commits.iter().enumerate() {
                    let now = std::time::Instant::now();
                    let should_report = index % 20 == 0 || // Every 20 commits  
                        index + 1 == commits.len() || // Last commit
                        now.duration_since(last_progress_time) >= progress_throttle; // Time-based throttle

                    if should_report {
                        let progress = ((index + 1) as f64 / commits.len() as f64 * 100.0) as u32;
                        report_progress(&format!(
                            "Processing commits: {}/{} ({}%)",
                            index + 1,
                            commits.len(),
                            progress
                        ));
                        last_progress_time = now;
                    }

                    match self.create_commit_document(&safe_repo_name, commit) {
                        Ok(doc) => {
                            if let Err(e) = storage.insert(doc).await {
                                warn!("Failed to insert commit document {}: {}", commit.sha, e);
                                result.errors += 1;
                            } else {
                                result.documents_created += 1;
                                result.commits_ingested += 1;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create document for commit {}: {}", commit.sha, e);
                            result.errors += 1;
                        }
                    }
                }
            }
        }

        report_progress("Finalizing ingestion...");

        // Build dependency graph if symbols were extracted
        if result.symbols_extracted > 0 {
            if let (Some(symbol_storage), _) = (symbol_storage.as_mut(), code_parser.as_mut()) {
                report_progress("Building dependency graph...");
                match symbol_storage.build_dependency_graph().await {
                    Ok(()) => {
                        let stats = symbol_storage.get_dependency_stats();
                        info!(
                            "Dependency graph built: {} relationships between {} symbols",
                            stats.total_relationships, stats.total_symbols
                        );

                        // Flush symbol storage to ensure all symbols and relationships are persisted
                        report_progress("Persisting symbols to storage...");
                        match symbol_storage.flush_storage().await {
                            Ok(()) => {
                                info!("Symbol storage flushed successfully");
                            }
                            Err(e) => {
                                warn!("Failed to flush symbol storage: {}", e);
                                result.errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to build dependency graph: {}", e);
                        result.errors += 1;
                    }
                }
            }
        }

        info!(
            "Ingestion complete: {} documents created ({} files, {} commits), {} symbols extracted from {} files, {} errors",
            result.documents_created, result.files_ingested, result.commits_ingested,
            result.symbols_extracted, result.files_with_symbols, result.errors
        );

        Ok(result)
    }

    /// Internal ingestion method that handles both cases
    #[cfg(not(feature = "tree-sitter-parsing"))]
    #[instrument(skip(self, storage, repo_path, progress_callback))]
    async fn ingest_internal<S: Storage + ?Sized>(
        &self,
        repo_path: impl AsRef<Path>,
        storage: &mut S,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<IngestResult> {
        let repo_path = repo_path.as_ref();
        info!("Starting repository ingestion from: {:?}", repo_path);

        // Helper function to report progress
        let report_progress = |message: &str| {
            if let Some(ref callback) = progress_callback {
                callback(message);
            }
        };

        report_progress("Opening repository...");

        // Open the repository
        let repo = GitRepository::open(repo_path, self.config.options.clone())
            .context("Failed to open git repository")?;

        report_progress("Analyzing repository metadata...");

        // Get repository metadata
        let metadata = repo
            .metadata()
            .context("Failed to get repository metadata")?;

        let safe_repo_name = Self::sanitize_name(&metadata.name);
        info!(
            "Repository: {} ({} commits) - using safe name: {}",
            metadata.name, metadata.commit_count, safe_repo_name
        );

        let mut result = IngestResult::default();

        // Initialize file organization manager if configured
        let _file_org_manager = self
            .config
            .organization_config
            .as_ref()
            .map(|org_config| FileOrganizationManager::new(org_config.clone()));

        // Create repository index document if requested
        if self.config.create_index {
            report_progress("Creating repository index document...");
            let index_doc = self.create_index_document(&metadata, &safe_repo_name)?;
            storage
                .insert(index_doc)
                .await
                .context("Failed to insert repository index document")?;
            result.documents_created += 1;
        }

        // Ingest files
        if self.config.options.include_file_contents {
            report_progress("Discovering repository files...");
            let files = repo
                .list_files()
                .context("Failed to list repository files")?;

            info!("Found {} files to ingest", files.len());

            if !files.is_empty() {
                report_progress(&format!("Processing {} files...", files.len()));

                let mut last_progress_time = std::time::Instant::now();
                let progress_throttle = std::time::Duration::from_millis(250); // Update every 250ms max

                for (index, file) in files.iter().enumerate() {
                    let now = std::time::Instant::now();
                    let should_report = index % 50 == 0 || // Every 50 files
                        index + 1 == files.len() || // Last file
                        now.duration_since(last_progress_time) >= progress_throttle; // Time-based throttle

                    if should_report {
                        let progress = ((index + 1) as f64 / files.len() as f64 * 100.0) as u32;
                        report_progress(&format!(
                            "Processing files: {}/{} ({}%)",
                            index + 1,
                            files.len(),
                            progress
                        ));
                        last_progress_time = now;
                    }

                    match self.create_file_document(&safe_repo_name, file) {
                        Ok(doc) => {
                            if let Err(e) = storage.insert(doc).await {
                                warn!("Failed to insert file document {}: {}", file.path, e);
                                result.errors += 1;
                            } else {
                                result.documents_created += 1;
                                result.files_ingested += 1;

                                // Extract symbols if enabled and this is a supported file type
                                #[cfg(feature = "tree-sitter-parsing")]
                                if self.config.options.extract_symbols {
                                    if let (Some(symbol_storage), Some(code_parser)) =
                                        (symbol_storage.as_mut(), code_parser.as_mut())
                                    {
                                        if let Some(symbols_extracted) =
                                            Self::extract_symbols_from_file(
                                                file,
                                                &safe_repo_name,
                                                symbol_storage,
                                                code_parser,
                                            )
                                            .await
                                        {
                                            result.symbols_extracted += symbols_extracted;
                                            result.files_with_symbols += 1;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create document for {}: {}", file.path, e);
                            result.errors += 1;
                        }
                    }
                }
            }
        }

        // Ingest commit history
        if self.config.options.include_commit_history {
            report_progress("Loading commit history...");
            let commits = repo
                .get_commits(None)
                .context("Failed to get repository commits")?;

            info!("Processing {} commits", commits.len());

            if !commits.is_empty() {
                report_progress(&format!("Processing {} commits...", commits.len()));

                let mut last_progress_time = std::time::Instant::now();
                let progress_throttle = std::time::Duration::from_millis(250); // Update every 250ms max

                for (index, commit) in commits.iter().enumerate() {
                    let now = std::time::Instant::now();
                    let should_report = index % 20 == 0 || // Every 20 commits  
                        index + 1 == commits.len() || // Last commit
                        now.duration_since(last_progress_time) >= progress_throttle; // Time-based throttle

                    if should_report {
                        let progress = ((index + 1) as f64 / commits.len() as f64 * 100.0) as u32;
                        report_progress(&format!(
                            "Processing commits: {}/{} ({}%)",
                            index + 1,
                            commits.len(),
                            progress
                        ));
                        last_progress_time = now;
                    }

                    match self.create_commit_document(&safe_repo_name, commit) {
                        Ok(doc) => {
                            if let Err(e) = storage.insert(doc).await {
                                warn!("Failed to insert commit document {}: {}", commit.sha, e);
                                result.errors += 1;
                            } else {
                                result.documents_created += 1;
                                result.commits_ingested += 1;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create document for commit {}: {}", commit.sha, e);
                            result.errors += 1;
                        }
                    }
                }
            }
        }

        report_progress("Finalizing ingestion...");

        info!(
            "Ingestion complete: {} documents created ({} files, {} commits), {} symbols extracted from {} files, {} errors",
            result.documents_created, result.files_ingested, result.commits_ingested,
            result.symbols_extracted, result.files_with_symbols, result.errors
        );

        Ok(result)
    }

    fn create_index_document(
        &self,
        metadata: &crate::git::types::RepositoryMetadata,
        safe_name: &str,
    ) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let path = format!("{}/{}/index.md", prefix, safe_name);

        let content = format!(
            "# Repository: {}\n\n\
            - **Path**: {}\n\
            - **Branch**: {}\n\
            - **Commits**: {}\n\
            - **Updated**: {}\n",
            metadata.name,
            metadata.path.display(),
            metadata.default_branch,
            metadata.commit_count,
            metadata.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(url) = &metadata.url {
            let content = format!("{}\n- **URL**: {}\n", content, url);
        }

        let mut builder = DocumentBuilder::new()
            .path(&path)?
            .title(format!("Repository: {}", metadata.name))?;
        builder = builder.content(content.as_bytes());
        builder = builder.tag("repository")?;
        builder = builder.tag("index")?;
        builder = builder.tag(&metadata.name)?;
        builder.build()
    }

    fn create_file_document(&self, repo_name: &str, file: &FileEntry) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let doc_path = format!("{}/{}/files/{}", prefix, repo_name, file.path);

        let title = format!("File: {}", file.path);

        let mut builder = DocumentBuilder::new().path(&doc_path)?.title(&title)?;

        // Add content based on whether it's binary
        if file.is_binary {
            let metadata = format!(
                "# Binary File: {}\n\n\
                - **Size**: {} bytes\n\
                - **Type**: Binary\n",
                file.path, file.size
            );
            builder = builder.content(metadata.as_bytes());
        } else {
            builder = builder.content(file.content.clone());
        }

        // Add tags
        builder = builder.tag("file")?;
        builder = builder.tag(repo_name)?;

        if let Some(ext) = &file.extension {
            // Sanitize extension by replacing dots with underscores for tag validation
            // Also use underscore instead of colon since colons aren't allowed in tags
            let sanitized_ext = ext.replace('.', "_");
            builder = builder.tag(&format!("ext_{}", sanitized_ext))?;
        }

        builder.build()
    }

    fn create_commit_document(&self, repo_name: &str, commit: &CommitInfo) -> Result<Document> {
        // Remove leading slash if present to create relative path
        let prefix = self.config.path_prefix.trim_start_matches('/');
        let doc_path = format!(
            "{}/{}/commits/{}.md",
            prefix,
            repo_name,
            &commit.sha[..8] // Use short SHA for path
        );

        let mut content = format!(
            "# Commit: {}\n\n\
            **Author**: {} <{}>\n\
            **Date**: {}\n\n\
            ## Message\n\
            {}\n\n\
            ## Details\n\
            - **SHA**: {}\n\
            - **Parents**: {}\n\
            - **Changes**: {} insertions(+), {} deletions(-)\n\
            - **Files Modified**: {}\n",
            &commit.sha[..8],
            commit.author_name,
            commit.author_email,
            commit.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            commit.message,
            commit.sha,
            if commit.parents.is_empty() {
                "none (initial commit)".to_string()
            } else {
                commit
                    .parents
                    .iter()
                    .map(|p| &p[..8])
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            commit.insertions,
            commit.deletions,
            commit.files_changed.len()
        );

        // Add list of changed files if any
        if !commit.files_changed.is_empty() {
            content.push_str("\n## Files Changed\n");
            for file in &commit.files_changed {
                content.push_str(&format!("- {}\n", file));
            }
        }

        let mut builder = DocumentBuilder::new()
            .path(&doc_path)?
            .title(format!("Commit: {}", &commit.sha[..8]))?;
        builder = builder.content(content.as_bytes());
        builder = builder.tag("commit")?;
        builder = builder.tag(repo_name)?;

        // Sanitize author name for use as tag - replace special characters with underscores
        let sanitized_author = commit
            .author_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        builder = builder.tag(&sanitized_author)?;
        builder.build()
    }

    /// Process symbol extraction for a batch of files in parallel
    #[cfg(feature = "tree-sitter-parsing")]
    #[allow(dead_code)]
    async fn extract_symbols_batch(
        files: &[FileEntry],
        repository_name: &str,
        symbol_storage: Arc<Mutex<SymbolStorage>>,
    ) -> Result<(usize, usize)> {
        use futures::stream;

        // Create a separate parser for each concurrent task to avoid contention
        let concurrent_limit = num_cpus::get().min(8); // Limit concurrency to available CPUs

        let results = stream::iter(files.iter())
            .map(|file| {
                let repo_name = repository_name.to_string();
                let storage_clone = Arc::clone(&symbol_storage);

                async move {
                    // Create explicit result type
                    let result: Result<(usize, bool)> = async {
                        // Skip binary files
                        if file.is_binary {
                            return Ok((0, false));
                        }

                        // Get file extension
                        let extension = file.extension.as_ref().ok_or_else(|| {
                            anyhow::anyhow!("No extension for file: {}", file.path)
                        })?;

                        // Check if language is supported
                        let language = match SupportedLanguage::from_extension(extension) {
                            Some(lang) => lang,
                            None => return Ok((0, false)), // Not a supported language
                        };

                        // Convert content to string
                        let content = String::from_utf8(file.content.clone())
                            .context("Failed to decode file as UTF-8")?;

                        // Create a local parser for this file
                        let mut code_parser = CodeParser::new()?;

                        // Parse the file content
                        let parsed_code = code_parser
                            .parse_content(&content, language)
                            .context("Failed to parse file")?;

                        // Extract symbols using shared storage
                        let file_path = Path::new(&file.path);
                        let mut storage = storage_clone.lock().await;

                        let symbol_ids = storage
                            .extract_symbols(file_path, parsed_code, Some(repo_name))
                            .await
                            .context("Failed to extract symbols")?;

                        let count = symbol_ids.len();
                        if count > 0 {
                            info!("Extracted {} symbols from {}", count, file.path);
                        }

                        Ok((count, count > 0))
                    }
                    .await;

                    result
                }
            })
            .buffer_unordered(concurrent_limit)
            .collect::<Vec<_>>()
            .await;

        // Aggregate results
        let mut total_symbols = 0;
        let mut files_with_symbols = 0;

        for result in results {
            match result {
                Ok((count, has_symbols)) => {
                    total_symbols += count;
                    if has_symbols {
                        files_with_symbols += 1;
                    }
                }
                Err(e) => {
                    warn!("Symbol extraction error: {}", e);
                }
            }
        }

        Ok((total_symbols, files_with_symbols))
    }

    /// Extract symbols from a file if it's a supported language (legacy sequential method)
    #[cfg(feature = "tree-sitter-parsing")]
    #[allow(dead_code)]
    async fn extract_symbols_from_file(
        file: &FileEntry,
        repository_name: &str,
        symbol_storage: &mut SymbolStorage,
        code_parser: &mut CodeParser,
    ) -> Option<usize> {
        // Skip binary files
        if file.is_binary {
            return None;
        }

        // Get file extension
        let extension = file.extension.as_ref()?;

        // Check if language is supported
        let language = SupportedLanguage::from_extension(extension)?;

        // Convert content to string
        let content = match String::from_utf8(file.content.clone()) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to decode file {} as UTF-8: {}", file.path, e);
                return None;
            }
        };

        // Parse the file content
        let parsed_code = match code_parser.parse_content(&content, language) {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!("Failed to parse file {}: {}", file.path, e);
                return None;
            }
        };

        // Extract symbols and store them
        let file_path = Path::new(&file.path);
        match symbol_storage
            .extract_symbols(file_path, parsed_code, Some(repository_name.to_string()))
            .await
        {
            Ok(symbol_ids) => {
                info!("Extracted {} symbols from {}", symbol_ids.len(), file.path);
                Some(symbol_ids.len())
            }
            Err(e) => {
                warn!("Failed to extract symbols from {}: {}", file.path, e);
                None
            }
        }
    }
}

/// Result of repository ingestion
#[derive(Debug, Default)]
pub struct IngestResult {
    /// Number of documents created
    pub documents_created: usize,
    /// Number of files ingested
    pub files_ingested: usize,
    /// Number of commits ingested
    pub commits_ingested: usize,
    /// Number of symbols extracted from code files
    pub symbols_extracted: usize,
    /// Number of files that had symbol extraction attempted
    pub files_with_symbols: usize,
    /// Number of errors encountered
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_file_storage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ingester_creation() -> Result<()> {
        let config = IngestionConfig::default();
        let _ingester = RepositoryIngester::new(config);
        Ok(())
    }

    #[test]
    fn test_repository_name_sanitization() {
        assert_eq!(RepositoryIngester::sanitize_name("my-repo"), "my-repo");
        assert_eq!(RepositoryIngester::sanitize_name("My_Repo"), "my_repo");
        assert_eq!(RepositoryIngester::sanitize_name("repo.name"), "repo.name");
        assert_eq!(
            RepositoryIngester::sanitize_name("repo/with/slashes"),
            "repo-with-slashes"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("repo with spaces"),
            "repo-with-spaces"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("UPPERCASE-REPO"),
            "uppercase-repo"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("@special#chars$"),
            "special-chars"
        );
        assert_eq!(
            RepositoryIngester::sanitize_name("--leading-trailing--"),
            "leading-trailing"
        );
    }

    #[tokio::test]
    async fn test_ingest_nonexistent_repo() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;

        let config = IngestionConfig::default();
        let ingester = RepositoryIngester::new(config);

        let result = ingester.ingest("/nonexistent/path", &mut storage).await;
        assert!(result.is_err());

        Ok(())
    }
}
