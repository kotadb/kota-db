// IndexingService - Unified repository and codebase indexing functionality
//
// This service extracts all indexing logic from main.rs and ManagementService
// to enable consistent indexing operations across CLI, MCP, and future interfaces.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::git::{IngestionConfig, ProgressCallback, RepositoryIngester};

use super::DatabaseAccess;

/// Configuration options for codebase indexing operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexCodebaseOptions {
    pub repo_path: PathBuf,
    pub prefix: String,
    pub include_files: bool,
    pub include_commits: bool,
    pub max_file_size_mb: usize,
    pub max_memory_mb: Option<u64>,
    pub max_parallel_files: Option<usize>,
    pub enable_chunking: bool,
    pub extract_symbols: Option<bool>,
    pub no_symbols: bool,
    pub quiet: bool,
    pub include_paths: Option<Vec<String>>,
    pub create_index: bool,
}

impl Default for IndexCodebaseOptions {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            prefix: "repos".to_string(),
            include_files: true,
            include_commits: true,
            max_file_size_mb: 10,
            max_memory_mb: None,
            max_parallel_files: None,
            enable_chunking: true,
            extract_symbols: Some(true),
            no_symbols: false,
            quiet: false,
            include_paths: None,
            create_index: true,
        }
    }
}

/// Configuration options for git repository indexing
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexGitOptions {
    pub repo_path: PathBuf,
    pub prefix: String,
    pub include_commits: bool,
    pub include_branches: bool,
    pub max_commits: Option<usize>,
    pub quiet: bool,
}

impl Default for IndexGitOptions {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            prefix: "repos".to_string(),
            include_commits: true,
            include_branches: true,
            max_commits: None,
            quiet: false,
        }
    }
}

/// Configuration options for incremental updates
#[derive(Debug, Clone, serde::Serialize)]
pub struct IncrementalUpdateOptions {
    pub changes: Vec<PathBuf>,
    pub delete_removed: bool,
    pub update_symbols: bool,
    pub quiet: bool,
}

impl Default for IncrementalUpdateOptions {
    fn default() -> Self {
        Self {
            changes: Vec::new(),
            delete_removed: true,
            update_symbols: true,
            quiet: false,
        }
    }
}

/// Result structure for indexing operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexResult {
    pub files_processed: usize,
    pub symbols_extracted: usize,
    pub relationships_found: usize,
    pub total_time_ms: u64,
    pub success: bool,
    pub formatted_output: String,
    pub errors: Vec<String>,
}

/// Result structure for git indexing operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitIndexResult {
    pub commits_processed: usize,
    pub branches_processed: usize,
    pub files_analyzed: usize,
    pub total_time_ms: u64,
    pub success: bool,
    pub formatted_output: String,
    pub errors: Vec<String>,
}

/// Result structure for incremental update operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateResult {
    pub files_updated: usize,
    pub files_added: usize,
    pub files_removed: usize,
    pub symbols_updated: usize,
    pub total_time_ms: u64,
    pub success: bool,
    pub formatted_output: String,
    pub errors: Vec<String>,
}

/// IndexingService handles all codebase and repository indexing operations
#[allow(dead_code)]
pub struct IndexingService<'a> {
    database: &'a dyn DatabaseAccess,
    db_path: PathBuf,
}

impl<'a> IndexingService<'a> {
    /// Create a new IndexingService instance
    pub fn new(database: &'a dyn DatabaseAccess, db_path: PathBuf) -> Self {
        Self { database, db_path }
    }

    /// Index a complete codebase with symbol extraction and relationship analysis
    ///
    /// This method extracts the complex indexing logic from main.rs, providing
    /// consistent codebase indexing across all interfaces.
    pub async fn index_codebase(&self, options: IndexCodebaseOptions) -> Result<IndexResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut formatted_output = String::new();

        warn!(
            repo = %options.repo_path.display(),
            include_paths = options.include_paths.as_ref().map(|paths| paths.len()),
            create_index = options.create_index,
            "IndexingService::index_codebase invoked"
        );

        // Validate repository path
        if !options.repo_path.exists() {
            let error = format!("Repository path does not exist: {:?}", options.repo_path);
            errors.push(error.clone());
            return Ok(IndexResult {
                files_processed: 0,
                symbols_extracted: 0,
                relationships_found: 0,
                total_time_ms: start_time.elapsed().as_millis() as u64,
                success: false,
                formatted_output: format!("❌ {}", error),
                errors,
            });
        }

        // Essential progress info: show unless in quiet mode
        if !options.quiet {
            formatted_output.push_str(&format!("🔄 Indexing codebase: {:?}\n", options.repo_path));
        }

        // Determine symbol extraction settings
        #[cfg(feature = "tree-sitter-parsing")]
        let should_extract_symbols = if options.no_symbols {
            if !options.quiet {
                formatted_output.push_str("⚠️  Symbol extraction disabled via --no-symbols flag\n");
            }
            false
        } else if let Some(extract) = options.extract_symbols {
            if extract {
                if !options.quiet {
                    formatted_output
                        .push_str("✅ Symbol extraction enabled via --extract-symbols flag\n");
                }
            } else if !options.quiet {
                formatted_output
                    .push_str("⚠️  Symbol extraction disabled via --extract-symbols=false\n");
            }
            extract
        } else {
            if !options.quiet {
                formatted_output
                    .push_str("✅ Symbol extraction enabled (default with tree-sitter feature)\n");
            }
            true // Default to true when tree-sitter is available
        };

        #[cfg(not(feature = "tree-sitter-parsing"))]
        let should_extract_symbols = false;

        // Configure memory limits if specified
        let memory_limits = if options.max_memory_mb.is_some()
            || options.max_parallel_files.is_some()
            || !options.enable_chunking
        {
            Some(crate::memory::MemoryLimitsConfig {
                max_total_memory_mb: options.max_memory_mb,
                max_parallel_files: options.max_parallel_files,
                enable_adaptive_chunking: options.enable_chunking,
                chunk_size: if options.enable_chunking {
                    50
                } else {
                    usize::MAX
                },
            })
        } else {
            None
        };

        // Configure ingestion options
        #[allow(unused_mut)]
        let mut ingestion_options = crate::git::types::IngestionOptions {
            include_file_contents: options.include_files,
            include_commit_history: options.include_commits,
            max_file_size: options.max_file_size_mb * 1024 * 1024,
            memory_limits,
            ..Default::default()
        };

        ingestion_options.include_paths = options.include_paths.clone();

        #[cfg(feature = "tree-sitter-parsing")]
        {
            ingestion_options.extract_symbols = should_extract_symbols;
        }

        let config = IngestionConfig {
            path_prefix: options.prefix.clone(),
            options: ingestion_options,
            create_index: options.create_index,
            organization_config: Some(crate::git::RepositoryOrganizationConfig::default()),
        };

        // Create progress callback for tracking
        let files_processed: usize;
        let symbols_extracted: usize;
        let relationships_found: usize;

        let progress_callback: ProgressCallback = Box::new(move |message: &str| {
            if !options.quiet {
                // Could update progress here if needed
            }
        });

        // Perform the indexing operation
        warn!(
            repo = %options.repo_path.display(),
            include_paths = options.include_paths.as_ref().map(|paths| paths.len()),
            extract_symbols = should_extract_symbols,
            create_index = options.create_index,
            "IndexingService::index_codebase starting ingestion"
        );
        let ingester = RepositoryIngester::new(config.clone());
        let storage_arc = self.database.storage();
        let mut storage = storage_arc.lock().await;

        // Choose the appropriate ingestion method based on symbol extraction setting
        #[cfg(feature = "tree-sitter-parsing")]
        let result = if should_extract_symbols {
            // Use binary symbol storage with relationship extraction for complete analysis
            let symbol_db_path = self.db_path.join("symbols.kota");
            let graph_db_path = self.db_path.join("dependency_graph.bin");
            debug!("Invoking ingest_with_binary_symbols_and_relationships");
            ingester
                .ingest_with_binary_symbols_and_relationships(
                    &options.repo_path,
                    &mut *storage,
                    &symbol_db_path,
                    &graph_db_path,
                    Some(progress_callback),
                )
                .await
                .context("Failed to ingest repository with symbol and relationship extraction")
        } else {
            debug!("Invoking ingest_with_progress without symbol extraction");
            ingester
                .ingest_with_progress(&options.repo_path, &mut *storage, Some(progress_callback))
                .await
                .context("Failed to ingest repository without symbol extraction")
        };

        #[cfg(not(feature = "tree-sitter-parsing"))]
        let result = ingester
            .ingest_with_progress(&options.repo_path, &mut *storage, Some(progress_callback))
            .await
            .context("Failed to ingest repository without tree-sitter parsing");

        let (files_processed, symbols_extracted, relationships_found) = match result {
            Ok(ingestion_result) => {
                let files_proc = ingestion_result.files_ingested;

                // Extract symbol counts if available
                #[cfg(feature = "tree-sitter-parsing")]
                let symbols_ext = ingestion_result.symbols_extracted;

                #[cfg(not(feature = "tree-sitter-parsing"))]
                let symbols_ext = 0;

                // Extract relationship counts from ingestion result
                let relationships_found = ingestion_result.relationships_extracted;

                // Defer trigram index population to reduce indexing time
                // The trigram index will be populated lazily on first search
                // This dramatically improves indexing performance while maintaining functionality
                if files_proc > 0 && !options.quiet {
                    // Essential completion info: show unless in quiet mode
                    formatted_output.push_str("📝 Documents stored successfully. Search index will be built on first search.\n");
                }

                (files_proc, symbols_ext, relationships_found)
            }
            Err(e) => {
                let error = format!("Indexing failed: {}", e);
                errors.push(error.clone());

                // Essential error info: always show regardless of quiet mode
                formatted_output.push_str(&format!("❌ {}\n", error));

                return Ok(IndexResult {
                    files_processed: 0,
                    symbols_extracted: 0,
                    relationships_found: 0,
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    formatted_output,
                    errors,
                });
            }
        };

        warn!(
            repo = %options.repo_path.display(),
            files_processed,
            symbols_extracted,
            relationships_found,
            elapsed_ms = start_time.elapsed().as_millis() as u64,
            "IndexingService::index_codebase completed"
        );

        // Essential completion status: show unless in quiet mode
        if !options.quiet {
            formatted_output.push_str(&format!("Indexed {} files\n", files_processed));
        }

        // Detailed info: only in non-quiet mode
        if !options.quiet {
            formatted_output.push_str("✅ Indexing completed successfully\n");

            #[cfg(feature = "tree-sitter-parsing")]
            {
                formatted_output
                    .push_str(&format!("   🔣 Symbols extracted: {}\n", symbols_extracted));
                formatted_output.push_str(&format!(
                    "   🔗 Relationships found: {}\n",
                    relationships_found
                ));
            }

            let duration = start_time.elapsed();
            formatted_output.push_str(&format!(
                "   ⏱️  Total time: {:.2}s\n",
                duration.as_secs_f64()
            ));
        }

        // CRITICAL: Flush storage buffer to ensure all documents are persisted
        // This fixes issue #553 where documents were buffered but not flushed for small repositories
        if !options.quiet {
            formatted_output.push_str("💾 Flushing storage buffer...\n");
        }
        // The storage wrapper may be buffering writes for performance, so we need to flush
        // This is especially important for small repositories that don't reach the buffer threshold
        if let Err(e) = storage.flush().await {
            let error = format!("Failed to flush storage: {}", e);
            errors.push(error.clone());
            if !options.quiet {
                formatted_output.push_str(&format!("⚠️  Warning: {}\n", error));
            }
        }
        drop(storage); // Release storage lock before rebuilding indices

        // CRITICAL: Rebuild indices after successful codebase indexing
        // This populates the Primary Index with document paths, enabling wildcard searches
        // and builds the Trigram Index for full-text search functionality
        if files_processed > 0 {
            if !options.quiet {
                formatted_output
                    .push_str("🔄 Rebuilding indices to enable search functionality...\n");
            }

            // Implement index rebuilding directly using the DatabaseAccess trait
            // Get all documents from storage
            let all_docs = {
                let storage = self.database.storage();
                let storage = storage.lock().await;
                match storage.list_all().await {
                    Ok(docs) => docs,
                    Err(e) => {
                        let error = format!("Failed to list documents for index rebuild: {}", e);
                        errors.push(error.clone());
                        if !options.quiet {
                            formatted_output.push_str(&format!("❌ {}\n", error));
                        }
                        return Ok(IndexResult {
                            files_processed,
                            symbols_extracted,
                            relationships_found,
                            total_time_ms: start_time.elapsed().as_millis() as u64,
                            success: false,
                            formatted_output,
                            errors,
                        });
                    }
                }
            };

            let total_docs = all_docs.len();
            if total_docs == 0 {
                if !options.quiet {
                    formatted_output
                        .push_str("⚠️ No documents found in storage, skipping index rebuild.\n");
                }
            } else {
                // Process documents in batches for better performance
                const BATCH_SIZE: usize = 100;
                let mut processed = 0;

                // Process in chunks to reduce lock contention and prevent OOM
                for chunk in all_docs.chunks(BATCH_SIZE) {
                    // Collect document data for this batch (including content for trigram indexing)
                    let mut batch_entries = Vec::with_capacity(chunk.len());
                    for doc in chunk {
                        let doc_id = doc.id;
                        let doc_path = match crate::types::ValidatedPath::new(doc.path.to_string())
                        {
                            Ok(path) => path,
                            Err(e) => {
                                let error = format!("Invalid document path: {}", e);
                                errors.push(error.clone());
                                if !options.quiet {
                                    formatted_output.push_str(&format!("⚠️ Warning: {}\n", error));
                                }
                                continue; // Skip this document
                            }
                        };
                        batch_entries.push((doc_id, doc_path, doc.content.clone()));
                    }

                    // Insert batch into primary index (path-based)
                    {
                        let primary_index_arc = self.database.primary_index();
                        let mut primary_index = primary_index_arc.lock().await;
                        for (doc_id, doc_path, _) in &batch_entries {
                            if let Err(e) = primary_index.insert(*doc_id, doc_path.clone()).await {
                                let error =
                                    format!("Failed to insert document into primary index: {}", e);
                                errors.push(error.clone());
                                if !options.quiet {
                                    formatted_output.push_str(&format!("⚠️ Warning: {}\n", error));
                                }
                            }
                        }
                    }

                    // Insert batch into trigram index (content-based)
                    {
                        let trigram_index_arc = self.database.trigram_index();
                        let mut trigram_index = trigram_index_arc.lock().await;
                        for (doc_id, doc_path, content) in &batch_entries {
                            if let Err(e) = trigram_index
                                .insert_with_content(*doc_id, doc_path.clone(), content)
                                .await
                            {
                                let error =
                                    format!("Failed to insert document into trigram index: {}", e);
                                errors.push(error.clone());
                                if !options.quiet {
                                    formatted_output.push_str(&format!("⚠️ Warning: {}\n", error));
                                }
                            }
                        }
                    }

                    processed += batch_entries.len();

                    // Periodic flush for large datasets
                    if processed % 500 == 0 || processed >= total_docs {
                        {
                            let primary_index_arc = self.database.primary_index();
                            let mut primary_index = primary_index_arc.lock().await;
                            if let Err(e) = primary_index.flush().await {
                                let error = format!("Failed to flush primary index: {}", e);
                                errors.push(error.clone());
                                if !options.quiet {
                                    formatted_output.push_str(&format!("⚠️ Warning: {}\n", error));
                                }
                            }
                        }
                        {
                            let trigram_index_arc = self.database.trigram_index();
                            let mut trigram_index = trigram_index_arc.lock().await;
                            if let Err(e) = trigram_index.flush().await {
                                let error = format!("Failed to flush trigram index: {}", e);
                                errors.push(error.clone());
                                if !options.quiet {
                                    formatted_output.push_str(&format!("⚠️ Warning: {}\n", error));
                                }
                            }
                        }
                    }
                }

                if !options.quiet {
                    formatted_output.push_str(
                        "✅ Index rebuild completed. Search functionality is now available.\n",
                    );
                }
            }
        }

        Ok(IndexResult {
            files_processed,
            symbols_extracted,
            relationships_found,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            success: true,
            formatted_output,
            errors,
        })
    }

    /// Index git repository history and metadata
    ///
    /// Focuses on git-specific operations like commit history, branch analysis,
    /// and repository metadata extraction.
    pub async fn index_git_repository(&self, options: IndexGitOptions) -> Result<GitIndexResult> {
        let start_time = std::time::Instant::now();
        let errors = Vec::new();
        let mut formatted_output = String::new();

        if !options.quiet {
            formatted_output.push_str(&format!(
                "🔄 Indexing git repository: {:?}\n",
                options.repo_path
            ));
        }

        // Stub: git-specific indexing logic is handled separately (tracked in issue #706)
        // This would include:
        // - Commit history analysis
        // - Branch structure mapping
        // - Author and collaboration patterns
        // - File change patterns over time

        if !options.quiet {
            formatted_output.push_str("⚠️  Git-specific indexing not yet fully implemented\n");
            formatted_output.push_str("   Using standard codebase indexing as fallback\n");
        }

        // For now, delegate to codebase indexing
        let codebase_options = IndexCodebaseOptions {
            repo_path: options.repo_path.clone(),
            prefix: options.prefix.clone(),
            include_files: true,
            include_commits: options.include_commits,
            quiet: options.quiet,
            ..Default::default()
        };

        let codebase_result = self.index_codebase(codebase_options).await?;

        Ok(GitIndexResult {
            commits_processed: 0, // Commit counting not yet implemented (tracked in issue #706)
            branches_processed: 0, // Branch counting not yet implemented (tracked in issue #706)
            files_analyzed: codebase_result.files_processed,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            success: codebase_result.success,
            formatted_output: format!("{}{}", formatted_output, codebase_result.formatted_output),
            errors,
        })
    }

    /// Perform incremental update of indexed content
    ///
    /// Updates only changed files and their related symbols/relationships
    /// for efficient maintenance of large codebases.
    pub async fn incremental_update(
        &self,
        options: IncrementalUpdateOptions,
    ) -> Result<UpdateResult> {
        let start_time = std::time::Instant::now();
        let errors = Vec::new();
        let mut formatted_output = String::new();

        if !options.quiet {
            formatted_output.push_str(&format!(
                "🔄 Performing incremental update for {} files\n",
                options.changes.len()
            ));
        }

        // Stub: incremental update pipeline is still under construction (tracked in issue #706)
        // This would include:
        // - Identify changed, added, and removed files
        // - Update only affected documents in storage
        // - Recompute symbols for changed files
        // - Update relationship graph for affected symbols
        // - Clean up orphaned symbols and relationships

        if !options.quiet {
            formatted_output.push_str("⚠️  Incremental update not yet fully implemented\n");
            formatted_output.push_str("   Manual re-indexing recommended for now\n");
        }

        Ok(UpdateResult {
            files_updated: 0,
            files_added: 0,
            files_removed: 0,
            symbols_updated: 0,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            success: false, // Mark as not successful until implemented
            formatted_output,
            errors,
        })
    }

    /// Reindex a specific scope (path, file, or symbol)
    ///
    /// Selective reindexing for targeted updates after code changes.
    pub async fn reindex_scope(
        &self,
        scope_path: &Path,
        extract_symbols: bool,
    ) -> Result<IndexResult> {
        let start_time = std::time::Instant::now();
        let mut formatted_output = String::new();

        formatted_output.push_str(&format!("🔄 Reindexing scope: {:?}\n", scope_path));

        // Stub: scope-specific reindexing intentionally left unimplemented (tracked in issue #706)
        // This would include:
        // - Determine scope boundaries (file, directory, symbol)
        // - Remove existing data for the scope
        // - Re-index only the specified scope
        // - Update relationships affected by scope changes

        formatted_output.push_str("⚠️  Scope reindexing not yet implemented\n");

        Ok(IndexResult {
            files_processed: 0,
            symbols_extracted: 0,
            relationships_found: 0,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            success: false,
            formatted_output,
            errors: vec!["Scope reindexing not implemented".to_string()],
        })
    }

    /// Populate the trigram index with content from all stored documents
    ///
    /// This method reads all documents from storage and indexes their content
    /// in the trigram index to enable full-text search functionality.
    /// Called automatically after repository ingestion to ensure content search works.
    ///
    /// Performance optimized: processes documents in batches and only flushes once at the end.
    #[allow(dead_code)]
    async fn populate_trigram_index(&self) -> Result<usize> {
        // Get all documents from storage
        let storage_arc = self.database.storage();
        let storage = storage_arc.lock().await;
        let all_docs = storage.list_all().await?;
        drop(storage); // Release storage lock early

        if all_docs.is_empty() {
            return Ok(0);
        }

        // Get trigram index
        let trigram_index_arc = self.database.trigram_index();
        let mut trigram_index = trigram_index_arc.lock().await;

        let mut indexed_count = 0;
        let total_docs = all_docs.len();

        // Process documents in batches for better performance
        const BATCH_SIZE: usize = 50; // Process 50 docs before intermediate flush

        for (batch_idx, chunk) in all_docs.chunks(BATCH_SIZE).enumerate() {
            // Process batch without disk I/O
            for doc in chunk {
                // Use a special batch insert method that defers disk writes
                match Self::insert_document_content_batch(
                    &mut *trigram_index,
                    doc.id,
                    doc.path.clone(),
                    &doc.content,
                )
                .await
                {
                    Ok(()) => {
                        indexed_count += 1;
                    }
                    Err(e) => {
                        // Log the error but continue processing other documents
                        tracing::warn!(
                            "Failed to index content for document {}: {}",
                            doc.path.as_str(),
                            e
                        );
                    }
                }
            }

            // Periodic flush every few batches to prevent memory usage from growing too large
            // but avoid the expensive flush on every document
            if (batch_idx + 1) % 5 == 0 || (batch_idx + 1) * BATCH_SIZE >= total_docs {
                if let Err(e) = trigram_index.flush().await {
                    tracing::warn!(
                        "Failed to flush trigram index during batch processing: {}",
                        e
                    );
                }
            }
        }

        // Final flush to ensure all data is persisted
        if let Err(e) = trigram_index.flush().await {
            tracing::warn!("Failed to final flush trigram index: {}", e);
        }

        Ok(indexed_count)
    }

    /// Insert document content without triggering frequent disk I/O
    /// This is an optimized version of insert_with_content that defers expensive disk operations
    #[allow(dead_code)]
    async fn insert_document_content_batch(
        trigram_index: &mut dyn crate::contracts::Index,
        doc_id: crate::types::ValidatedDocumentId,
        path: crate::types::ValidatedPath,
        content: &[u8],
    ) -> Result<()> {
        // Direct call to insert_with_content - the performance improvement comes from
        // batching the flush operations in populate_trigram_index rather than
        // modifying the core insert_with_content method
        trigram_index
            .insert_with_content(doc_id, path, content)
            .await
    }
}
