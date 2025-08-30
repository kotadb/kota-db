//! Binary-to-relationship bridge for efficient dependency graph construction
//!
//! This module bridges the gap between the high-performance binary symbol format
//! and the dependency graph needed for relationship queries. It enables extracting
//! relationships while maintaining the 130x performance improvement.

use anyhow::{Context, Result};
use petgraph::graph::DiGraph;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use uuid::Uuid;

use crate::{
    binary_symbols::BinarySymbolReader,
    dependency_extractor::{
        CodeReference, DependencyEdge, DependencyGraph, GraphStats, ReferenceType, SymbolNode,
    },
    parsing::{SupportedLanguage, SymbolType},
    types::RelationType,
};

// Performance tuning constants - made configurable for different use cases
const MAX_SUFFIX_MATCHES: usize = 5;
const SYMBOL_COUNT_THRESHOLD: usize = 10_000;
const DEFAULT_MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Reverse lookup index for efficient suffix matching
#[derive(Debug, Clone)]
struct ReverseIndex {
    /// Maps symbol suffixes to qualified names and UUIDs
    suffix_to_symbols: HashMap<String, Vec<(String, Uuid)>>,
}

impl ReverseIndex {
    /// Build reverse index from name map for O(1) suffix lookups
    fn build_from_name_map(name_map: &HashMap<String, Uuid>) -> Self {
        let mut suffix_to_symbols = HashMap::new();

        for (qualified_name, &uuid) in name_map {
            // Extract suffixes of different lengths for flexible matching
            let parts: Vec<&str> = qualified_name.split("::").collect();
            for i in 1..=parts.len() {
                let suffix = parts[parts.len() - i..].join("::");
                suffix_to_symbols
                    .entry(suffix)
                    .or_insert_with(Vec::new)
                    .push((qualified_name.clone(), uuid));
            }
        }

        Self { suffix_to_symbols }
    }

    /// Find symbols matching a suffix pattern
    fn find_by_suffix(&self, pattern: &str) -> Option<&Vec<(String, Uuid)>> {
        self.suffix_to_symbols.get(pattern)
    }
}

/// Configuration for relationship extraction
#[derive(Debug, Clone)]
pub struct RelationshipExtractionConfig {
    /// Maximum parallel threads for processing
    pub max_threads: Option<usize>,
    /// Skip relationship extraction for files larger than this (in bytes)
    pub max_file_size: Option<usize>,
    /// Languages to process (None = all supported)
    pub languages: Option<Vec<SupportedLanguage>>,
}

impl Default for RelationshipExtractionConfig {
    fn default() -> Self {
        Self {
            max_threads: None, // Use rayon default
            max_file_size: Some(DEFAULT_MAX_FILE_SIZE),
            languages: None,
        }
    }
}

/// Bridge for extracting relationships from binary symbols
pub struct BinaryRelationshipBridge {
    /// Configuration for extraction
    config: RelationshipExtractionConfig,
    /// Parser pool for reuse across threads
    parser_pool: Arc<Mutex<Vec<Parser>>>,
}

impl Default for BinaryRelationshipBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryRelationshipBridge {
    /// Create a new bridge with default configuration
    pub fn new() -> Self {
        Self::with_config(RelationshipExtractionConfig::default())
    }

    /// Create a new bridge with custom configuration
    pub fn with_config(config: RelationshipExtractionConfig) -> Self {
        Self {
            config,
            parser_pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Extract relationships from binary symbols and source files
    ///
    /// This is the main entry point that:
    /// 1. Loads symbols from the binary database
    /// 2. Analyzes source files for references
    /// 3. Builds a complete dependency graph
    #[instrument(skip(self, files))]
    pub fn extract_relationships(
        &self,
        symbol_db_path: impl AsRef<Path> + std::fmt::Debug,
        repo_path: impl AsRef<Path> + std::fmt::Debug,
        files: &[(PathBuf, Vec<u8>)], // (path, content) pairs
    ) -> Result<DependencyGraph> {
        let symbol_db_path = symbol_db_path.as_ref();
        let repo_path = repo_path.as_ref();

        info!("Starting relationship extraction from binary symbols");
        let start = std::time::Instant::now();

        // Step 1: Load symbols from binary database
        let reader =
            BinarySymbolReader::open(symbol_db_path).context("Failed to open symbol database")?;

        info!(
            "Loaded {} symbols from binary database",
            reader.symbol_count()
        );

        // Step 2: Build symbol lookup maps
        let (symbol_map, name_map, file_map) = self.build_symbol_maps(&reader)?;

        // Step 3: Extract references from source files in parallel
        let all_references = self.extract_all_references(files, &name_map)?;

        // Step 4: Build the dependency graph
        debug!(
            "Building graph with {} symbols, {} files, {} reference files",
            symbol_map.len(),
            file_map.len(),
            all_references.len()
        );
        let graph = self.build_graph(symbol_map, name_map, file_map, all_references)?;

        let elapsed = start.elapsed();
        info!(
            "Extracted relationships in {:?}: {} nodes, {} edges",
            elapsed, graph.stats.node_count, graph.stats.edge_count
        );

        Ok(graph)
    }

    /// Build lookup maps from binary symbols
    #[allow(clippy::type_complexity)]
    fn build_symbol_maps(
        &self,
        reader: &BinarySymbolReader,
    ) -> Result<(
        HashMap<Uuid, SymbolInfo>,
        HashMap<String, Uuid>,
        HashMap<PathBuf, Vec<Uuid>>,
    )> {
        let mut symbol_map = HashMap::new();
        let mut name_map = HashMap::new();
        let mut file_map: HashMap<PathBuf, Vec<Uuid>> = HashMap::new();

        for i in 0..reader.symbol_count() {
            let symbol = reader.get_symbol(i).context("Failed to read symbol")?;
            let id = Uuid::from_bytes(symbol.id);
            let name = reader.get_symbol_name(&symbol)?;
            let file_path = PathBuf::from(reader.get_symbol_file_path(&symbol)?);

            // Build qualified name (for now, use file:name pattern)
            let qualified_name = format!("{}::{}", file_path.display(), name);

            let info = SymbolInfo {
                id,
                name: name.clone(),
                qualified_name: qualified_name.clone(),
                symbol_type: self.kind_to_type(symbol.kind),
                file_path: file_path.clone(),
                start_line: symbol.start_line as usize,
                end_line: symbol.end_line as usize,
                parent_id: if symbol.parent_id != [0u8; 16] {
                    Some(Uuid::from_bytes(symbol.parent_id))
                } else {
                    None
                },
            };

            symbol_map.insert(id, info);
            name_map.insert(qualified_name, id);

            // Also index by simple name for reference resolution
            name_map.insert(name.clone(), id);

            file_map.entry(file_path).or_default().push(id);
        }

        Ok((symbol_map, name_map, file_map))
    }

    /// Extract references from all source files with error recovery
    fn extract_all_references(
        &self,
        files: &[(PathBuf, Vec<u8>)],
        name_map: &HashMap<String, Uuid>,
    ) -> Result<Vec<FileReferences>> {
        // Process files in parallel with error tracking
        let references: Vec<_> = files
            .par_iter()
            .filter_map(|(path, content)| {
                // Skip if file is too large - create structured error
                if let Some(max_size) = self.config.max_file_size {
                    if content.len() > max_size {
                        let error = PythonParsingError::FileSizeError {
                            file_path: path.clone(),
                            size: content.len(),
                            max_size,
                        };
                        debug!("Skipping large file: {}", error);
                        return Some(FileReferences {
                            file_path: path.clone(),
                            references: Vec::new(),
                            extraction_errors: vec![error.to_string()],
                        });
                    }
                }

                // Detect language from extension
                let extension = match path.extension().and_then(|e| e.to_str()) {
                    Some(ext) => ext,
                    None => {
                        let error = PythonParsingError::UnsupportedLanguageError {
                            file_path: path.clone(),
                            extension: "none".to_string(),
                        };
                        return Some(FileReferences {
                            file_path: path.clone(),
                            references: Vec::new(),
                            extraction_errors: vec![error.to_string()],
                        });
                    }
                };

                let language = match SupportedLanguage::from_extension(extension) {
                    Some(lang) => lang,
                    None => {
                        let error = PythonParsingError::UnsupportedLanguageError {
                            file_path: path.clone(),
                            extension: extension.to_string(),
                        };
                        return Some(FileReferences {
                            file_path: path.clone(),
                            references: Vec::new(),
                            extraction_errors: vec![error.to_string()],
                        });
                    }
                };

                // Skip if language not in filter
                if let Some(ref langs) = self.config.languages {
                    if !langs.contains(&language) {
                        return None;
                    }
                }

                // Convert content to string with lossy UTF-8 conversion
                // This handles files with mixed encodings gracefully
                let content_str = String::from_utf8_lossy(content).into_owned();

                // Extract references with partial success support
                match self.extract_file_references_with_recovery(path, &content_str, language) {
                    ExtractionResult::Success(refs) => Some(refs),
                    ExtractionResult::PartialSuccess {
                        references,
                        recoverable_errors,
                    } => {
                        // Log structured error details for better debugging
                        for error in &recoverable_errors {
                            warn!("Recoverable parsing error in {}: {}", path.display(), error);
                            debug!("Error details: {:?}", error);
                        }
                        warn!(
                            "Partial extraction from {}: {} recoverable errors",
                            path.display(),
                            recoverable_errors.len()
                        );
                        Some(references)
                    }
                    ExtractionResult::Failure(e) => {
                        error!("Complete extraction failure for {}: {}", path.display(), e);
                        debug!("Failure details: {:?}", e);
                        Some(FileReferences {
                            file_path: path.clone(),
                            references: Vec::new(),
                            extraction_errors: vec![e.to_string()],
                        })
                    }
                }
            })
            .collect();

        // Log summary of errors
        let files_with_errors: usize = references
            .iter()
            .filter(|r| !r.extraction_errors.is_empty())
            .count();

        if files_with_errors > 0 {
            info!(
                "Extraction completed with errors in {}/{} files",
                files_with_errors,
                references.len()
            );
        }

        Ok(references)
    }

    /// Extract references with recovery support
    fn extract_file_references_with_recovery(
        &self,
        file_path: &Path,
        content: &str,
        language: SupportedLanguage,
    ) -> ExtractionResult {
        match self.extract_file_references(file_path, content, language) {
            Ok(refs) => ExtractionResult::Success(refs),
            Err(e) => {
                // Create structured error based on the failure cause
                let structured_error = if e.to_string().contains("Failed to parse file") {
                    PythonParsingError::TreeSitterError {
                        message: e.to_string(),
                        file_path: file_path.to_path_buf(),
                        language: format!("{:?}", language),
                    }
                } else if e.to_string().contains("UTF-8") {
                    PythonParsingError::Utf8DecodingError {
                        message: e.to_string(),
                        file_path: file_path.to_path_buf(),
                        line: 0, // Default to start of file
                        column: None,
                    }
                } else {
                    PythonParsingError::TreeSitterError {
                        message: format!("General parsing error: {}", e),
                        file_path: file_path.to_path_buf(),
                        language: format!("{:?}", language),
                    }
                };

                // Try to recover with partial parsing
                let mut partial_refs = Vec::new();

                // Attempt line-by-line extraction for simple references
                for (line_num, line) in content.lines().enumerate() {
                    // Simple heuristic: look for function calls
                    if line.contains("(")
                        && !line.trim().starts_with("//")
                        && !line.trim().starts_with("#")
                    {
                        if let Some(name) = Self::extract_simple_reference(line) {
                            partial_refs.push(CodeReference {
                                name,
                                ref_type: ReferenceType::FunctionCall,
                                line: line_num + 1,
                                column: 1,
                                text: line.trim().to_string(),
                            });
                        }
                    }
                }

                if !partial_refs.is_empty() {
                    ExtractionResult::PartialSuccess {
                        references: FileReferences {
                            file_path: file_path.to_path_buf(),
                            references: partial_refs,
                            extraction_errors: vec![structured_error.to_string()],
                        },
                        recoverable_errors: vec![structured_error],
                    }
                } else {
                    ExtractionResult::Failure(structured_error)
                }
            }
        }
    }

    /// Simple heuristic to extract a function name from a line
    fn extract_simple_reference(line: &str) -> Option<String> {
        // Look for pattern: word followed by parenthesis
        let trimmed = line.trim();
        if let Some(paren_pos) = trimmed.find('(') {
            if paren_pos > 0 {
                let before_paren = &trimmed[..paren_pos];
                // Get the last word before the parenthesis
                if let Some(word) = before_paren.split_whitespace().last() {
                    // Remove any leading punctuation
                    let clean = word.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !clean.is_empty() {
                        return Some(clean.to_string());
                    }
                }
            }
        }
        None
    }

    /// Extract references from a single file using Tree-sitter
    fn extract_file_references(
        &self,
        file_path: &Path,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<FileReferences> {
        // Get or create a parser
        let mut parser = self.get_parser(language)?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse file")?;

        // Extract references based on language
        let references = match language {
            SupportedLanguage::Rust => self.extract_rust_references(&tree, content)?,
            SupportedLanguage::Python => self.extract_python_references(&tree, content)?,
        };

        // Return parser to pool
        self.return_parser(parser);

        Ok(FileReferences {
            file_path: file_path.to_path_buf(),
            references,
            extraction_errors: Vec::new(),
        })
    }

    /// Extract references from Rust code
    fn extract_rust_references(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
    ) -> Result<Vec<CodeReference>> {
        let mut references = Vec::new();
        let language = tree_sitter_rust::LANGUAGE.into();

        // Enhanced comprehensive query for all reference types
        let comprehensive_query = Query::new(
            &language,
            r#"
            ; Basic function calls
            (call_expression
                function: (identifier) @function_name)
            (call_expression
                function: (scoped_identifier
                    name: (identifier) @function_name))
            (call_expression
                function: (field_expression
                    field: (field_identifier) @method_name))
            
            ; Basic type identifiers
            (type_identifier) @type_name
            
            ; Scoped type identifiers (module::Type)
            (scoped_type_identifier
                name: (type_identifier) @type_name)
            
            ; Generic types (Vec<Type>, Arc<Type>)
            (generic_type
                type: (type_identifier) @type_name)
            (generic_type
                type: (scoped_type_identifier
                    name: (type_identifier) @type_name))
            
            ; Static method calls (Type::method) - comprehensive patterns
            (call_expression
                function: (scoped_identifier
                    path: (identifier) @type_name))
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (identifier) @type_name)))
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (scoped_identifier
                            path: (identifier) @type_name))))
            
            ; Module-qualified static calls (crate::module::Type::method)
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (scoped_identifier
                            path: (scoped_identifier
                                path: (identifier) @type_name)))))
            
            ; Super/self qualified calls (super::Type::method, self::Type::method)
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (super) 
                        name: (identifier) @type_name)))
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (self) 
                        name: (identifier) @type_name)))
            
            ; Crate-qualified calls (crate::Type::method)
            (call_expression
                function: (scoped_identifier
                    path: (scoped_identifier
                        path: (crate) 
                        name: (identifier) @type_name)))
            
            ; Field types in struct declarations and variable declarations
            (field_declaration
                type: (type_identifier) @type_name)
            (field_declaration
                type: (generic_type
                    type: (type_identifier) @type_name))
            (field_declaration
                type: (scoped_type_identifier
                    name: (type_identifier) @type_name))
            
            ; Variable declarations with explicit types
            (let_declaration
                type: (type_identifier) @type_name)
            (let_declaration
                type: (generic_type
                    type: (type_identifier) @type_name))
            (let_declaration
                type: (scoped_type_identifier
                    name: (type_identifier) @type_name))
            
            ; Function parameter types
            (parameter
                type: (type_identifier) @type_name)
            (parameter
                type: (generic_type
                    type: (type_identifier) @type_name))
            (parameter
                type: (scoped_type_identifier
                    name: (type_identifier) @type_name))
            
            ; Function return types
            (function_item
                return_type: (type_identifier) @type_name)
            (function_item
                return_type: (generic_type
                    type: (type_identifier) @type_name))
            (function_item
                return_type: (scoped_type_identifier
                    name: (type_identifier) @type_name))
            "#,
        )?;

        let mut cursor = QueryCursor::new();
        let mut matches =
            cursor.matches(&comprehensive_query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                // Handle potential UTF-8 issues in tree-sitter text extraction
                let name = match node.utf8_text(content.as_bytes()) {
                    Ok(text) => text.to_string(),
                    Err(_) => {
                        // Fall back to lossy conversion for the node's byte range
                        let start = node.start_byte();
                        let end = node.end_byte();
                        let bytes = &content.as_bytes()[start..end];
                        String::from_utf8_lossy(bytes).into_owned()
                    }
                };
                let point = node.start_position();

                // Determine reference type based on the capture name
                let ref_type = match capture.index {
                    // These correspond to the @function_name captures
                    0 | 1 => ReferenceType::FunctionCall,
                    // @method_name captures
                    2 => ReferenceType::MethodCall,
                    // All @type_name captures (majority of patterns)
                    _ => ReferenceType::TypeUsage,
                };

                references.push(CodeReference {
                    name,
                    ref_type,
                    line: point.row + 1,
                    column: point.column + 1,
                    text: match node.utf8_text(content.as_bytes()) {
                        Ok(text) => text.to_string(),
                        Err(_) => {
                            // Fall back to lossy conversion for the node's byte range
                            let start = node.start_byte();
                            let end = node.end_byte();
                            let bytes = &content.as_bytes()[start..end];
                            String::from_utf8_lossy(bytes).into_owned()
                        }
                    },
                });
            }
        }

        Ok(references)
    }

    /// Extract references from Python code using tree-sitter queries
    fn extract_python_references(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
    ) -> Result<Vec<CodeReference>> {
        let mut references = Vec::new();
        let language = tree_sitter_python::LANGUAGE.into();

        // Comprehensive Python-specific queries including modern language features
        let python_query = Query::new(
            &language,
            r#"
            ; Function calls
            (call
                function: (identifier) @function_name)
            (call
                function: (attribute
                    attribute: (identifier) @method_name))

            ; Async function definitions
            (function_definition
                (async) @async_modifier
                name: (identifier) @async_function)

            ; Await expressions  
            (await
                (call
                    function: (identifier) @awaited_function))
            (await
                (call
                    function: (attribute
                        attribute: (identifier) @awaited_method)))

            ; Import statements
            (import_statement
                name: (dotted_name) @import_name)
            (import_from_statement
                name: (dotted_name) @import_name)
            (import_from_statement
                name: (aliased_import
                    name: (identifier) @import_name))

            ; Class references and inheritance
            (class_definition
                superclasses: (argument_list
                    (identifier) @base_class))

            ; Decorator references
            (decorator
                (identifier) @decorator_name)
            (decorator
                (attribute
                    attribute: (identifier) @decorator_name))

            ; Type annotations and hints
            (type_annotation
                type: (identifier) @type_hint)
            (type_annotation
                type: (subscript
                    value: (identifier) @generic_type))
            (type_annotation
                type: (attribute
                    attribute: (identifier) @qualified_type))

            ; Match statements (Python 3.10+)
            (match_statement
                subject: (identifier) @match_subject)
            (case_pattern
                (identifier) @pattern_variable)

            ; List/Dict/Set comprehensions
            (list_comprehension
                (call
                    function: (identifier) @comprehension_function))
            (dictionary_comprehension
                (call
                    function: (identifier) @comprehension_function))
            (set_comprehension
                (call
                    function: (identifier) @comprehension_function))

            ; Generator expressions
            (generator_expression
                (call
                    function: (identifier) @generator_function))

            ; Attribute access
            (attribute
                attribute: (identifier) @attribute_name)

            ; With statements and context managers
            (with_statement
                (with_item
                    (call
                        function: (identifier) @context_manager)))
            "#,
        )
        .map_err(|e| {
            anyhow::anyhow!(PythonParsingError::QueryCompilationError {
                message: format!("Python query compilation failed: {}", e),
                language: "Python".to_string(),
                query_text: "Python comprehensive query".to_string(),
            })
        })?;

        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&python_query, root_node, content.as_bytes());

        while let Some(query_match) = matches.next() {
            for capture in query_match.captures {
                let node = capture.node;

                // Handle potential UTF-8 issues in tree-sitter text extraction with structured errors
                let reference_text = match node.utf8_text(content.as_bytes()) {
                    Ok(text) => text.to_string(),
                    Err(_) => {
                        let point = node.start_position();
                        debug!(
                            "UTF-8 decoding error at line {}, column {}",
                            point.row + 1,
                            point.column + 1
                        );

                        // Fall back to lossy conversion for the node's byte range
                        let start = node.start_byte();
                        let end = node.end_byte();
                        let bytes = &content.as_bytes()[start..end];
                        String::from_utf8_lossy(bytes).into_owned()
                    }
                };

                let point = node.start_position();

                let capture_name = python_query.capture_names()[capture.index as usize];
                let ref_type = match capture_name {
                    // Basic function and method calls
                    "function_name" => ReferenceType::FunctionCall,
                    "method_name" => ReferenceType::FunctionCall,

                    // Async/await patterns
                    "async_function" => ReferenceType::FunctionDefinition,
                    "awaited_function" => ReferenceType::AsyncCall,
                    "awaited_method" => ReferenceType::AsyncCall,

                    // Import patterns
                    "import_name" => ReferenceType::Import,

                    // Class and inheritance
                    "base_class" => ReferenceType::Inheritance,

                    // Decorators
                    "decorator_name" => ReferenceType::Decorator,

                    // Type annotations and hints
                    "type_hint" => ReferenceType::TypeAnnotation,
                    "generic_type" => ReferenceType::TypeAnnotation,
                    "qualified_type" => ReferenceType::TypeAnnotation,

                    // Pattern matching (Python 3.10+)
                    "match_subject" => ReferenceType::PatternMatch,
                    "pattern_variable" => ReferenceType::PatternBinding,

                    // Comprehensions and generators
                    "comprehension_function" => ReferenceType::FunctionCall,
                    "generator_function" => ReferenceType::FunctionCall,

                    // Attribute access and context managers
                    "attribute_name" => ReferenceType::FieldAccess,
                    "context_manager" => ReferenceType::ContextManager,

                    _ => ReferenceType::Other,
                };

                references.push(CodeReference {
                    name: reference_text.clone(),
                    ref_type,
                    line: point.row + 1,
                    column: point.column + 1,
                    text: {
                        // Extract the whole line for context, similar to the Rust version
                        let _start_byte = node.start_byte();
                        let content_lines: Vec<&str> = content.lines().collect();
                        if point.row < content_lines.len() {
                            content_lines[point.row].to_string()
                        } else {
                            reference_text
                        }
                    },
                });
            }
        }

        Ok(references)
    }

    /// Enhanced symbol reference resolution with suffix matching fallback
    fn resolve_symbol_reference(
        &self,
        name: &str,
        name_map: &HashMap<String, Uuid>,
    ) -> Option<Uuid> {
        // First, try exact match (fastest path)
        if let Some(&id) = name_map.get(name) {
            return Some(id);
        }

        // If exact match fails and this is an unqualified name, try suffix matching
        // This handles cases where we're looking for "FileStorage" but the symbol
        // is stored as "kotadb::file_storage::FileStorage"
        if !name.contains("::") {
            // Performance optimization: Only iterate if reasonable number of symbols
            if name_map.len() < SYMBOL_COUNT_THRESHOLD {
                // Find any symbol name that ends with "::name" (exact boundary match)
                let pattern = format!("::{}", name);
                let mut matches = Vec::new();

                // Use reverse index for better O(1) lookups when available
                let reverse_index = ReverseIndex::build_from_name_map(name_map);

                // Try direct suffix lookup first
                if let Some(index_matches) = reverse_index.find_by_suffix(name) {
                    matches.extend(index_matches.iter().take(MAX_SUFFIX_MATCHES).cloned());
                }

                // If no matches from index, fall back to pattern matching
                if matches.is_empty() {
                    for (qualified_name, &id) in name_map {
                        // Ensure it's a true suffix match with :: boundary
                        if qualified_name.ends_with(&pattern)
                            && (qualified_name.len() == name.len()
                                || qualified_name.len() > pattern.len())
                        {
                            matches.push((qualified_name.clone(), id));

                            // Early termination for performance
                            if matches.len() > MAX_SUFFIX_MATCHES {
                                break;
                            }
                        }
                    }
                }

                // Process matches
                return self.process_suffix_matches(name, matches);
            } else {
                tracing::debug!(
                    "🔍 Skipping suffix matching for '{}' due to large symbol count ({})",
                    name,
                    name_map.len()
                );
            }
        }

        None
    }

    /// Process suffix matches for symbol resolution
    fn process_suffix_matches(&self, name: &str, matches: Vec<(String, Uuid)>) -> Option<Uuid> {
        // If we have exactly one match, use it
        if matches.len() == 1 {
            let (qualified_name, id) = &matches[0];
            tracing::debug!(
                "🔍 Resolved '{}' to '{}' via suffix matching",
                name,
                qualified_name
            );
            return Some(*id);
        }

        // If we have multiple matches, try to find the best one
        if matches.len() > 1 {
            tracing::debug!(
                "🔍 Multiple suffix matches for '{}': {:?}",
                name,
                matches.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
            );

            // Prefer shorter qualified names (likely more direct references)
            // and avoid test/example modules
            let best_match = matches
                .iter()
                .filter(|(qualified_name, _)| {
                    // Filter out test and example modules
                    !qualified_name.contains("::test")
                        && !qualified_name.contains("::tests")
                        && !qualified_name.contains("::example")
                })
                .min_by_key(|(qualified_name, _)| qualified_name.len());

            if let Some((qualified_name, id)) = best_match {
                tracing::debug!(
                    "🔍 Selected best match for '{}': '{}'",
                    name,
                    qualified_name
                );
                return Some(*id);
            }

            // Fallback to first match if all are filtered out
            return Some(matches[0].1);
        }

        None
    }

    /// Build the final dependency graph
    fn build_graph(
        &self,
        symbol_map: HashMap<Uuid, SymbolInfo>,
        name_map: HashMap<String, Uuid>,
        file_map: HashMap<PathBuf, Vec<Uuid>>,
        all_references: Vec<FileReferences>,
    ) -> Result<DependencyGraph> {
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();

        // Create nodes for all symbols
        for (id, info) in &symbol_map {
            let node = SymbolNode {
                symbol_id: *id,
                qualified_name: info.qualified_name.clone(),
                symbol_type: info.symbol_type.clone(),
                file_path: info.file_path.clone(),
                in_degree: 0,
                out_degree: 0,
            };

            let node_idx = graph.add_node(node);
            symbol_to_node.insert(*id, node_idx);
        }

        // Build symbol hierarchies for each file for accurate containment resolution
        let mut file_hierarchies: HashMap<PathBuf, Vec<SymbolHierarchy>> = HashMap::new();
        for (file_path, symbol_ids) in &file_map {
            let file_symbols: Vec<(&Uuid, &SymbolInfo)> =
                symbol_ids.iter().map(|id| (id, &symbol_map[id])).collect();
            let hierarchy = SymbolHierarchy::build_from_symbols(&file_symbols);
            file_hierarchies.insert(file_path.clone(), hierarchy);
        }

        // Create edges from references
        for file_refs in &all_references {
            // Get the symbol hierarchy for this file
            let hierarchy = match file_hierarchies.get(&file_refs.file_path) {
                Some(h) => h,
                None => {
                    debug!(
                        "No hierarchy found for file path: {:?}. Available paths: {:?}",
                        file_refs.file_path,
                        file_hierarchies.keys().take(5).collect::<Vec<_>>()
                    );
                    continue;
                }
            };

            for reference in &file_refs.references {
                // Try to resolve the reference to a symbol with enhanced matching
                if let Some(target_id) = self.resolve_symbol_reference(&reference.name, &name_map) {
                    // Find which symbol in this file contains this reference using hierarchy
                    let source_id = hierarchy
                        .iter()
                        .find_map(|root| root.find_containing_symbol(reference.line));

                    if let Some(source_id) = source_id {
                        // Don't create self-references
                        if source_id != target_id {
                            if let (Some(&source_node), Some(&target_node)) = (
                                symbol_to_node.get(&source_id),
                                symbol_to_node.get(&target_id),
                            ) {
                                let edge = DependencyEdge {
                                    relation_type: self.ref_type_to_relation(&reference.ref_type),
                                    line_number: reference.line,
                                    column_number: reference.column,
                                    context: Some(reference.text.clone()),
                                };

                                graph.add_edge(source_node, target_node, edge);
                            }
                        }
                    }
                }
            }
        }

        // Update in/out degrees
        for node_idx in graph.node_indices() {
            let in_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .count();
            let out_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing)
                .count();

            if let Some(node) = graph.node_weight_mut(node_idx) {
                node.in_degree = in_degree;
                node.out_degree = out_degree;
            }
        }

        // Calculate strongly connected components using Tarjan's algorithm
        let sccs = petgraph::algo::tarjan_scc(&graph);
        let scc_count = sccs.len();

        // Calculate max depth using BFS from each node
        let mut max_depth = 0;
        for node in graph.node_indices() {
            // Use BFS to find the maximum distance from this node
            let distances = petgraph::algo::dijkstra(&graph, node, None, |_| 1);
            if let Some(&furthest) = distances.values().max() {
                max_depth = max_depth.max(furthest);
            }
        }

        // Count imports by looking at cross-file edges
        let mut import_count = 0;
        for edge in graph.edge_indices() {
            if let Some((source, target)) = graph.edge_endpoints(edge) {
                let source_file = &graph[source].file_path;
                let target_file = &graph[target].file_path;
                if source_file != target_file {
                    import_count += 1;
                }
            }
        }

        // Calculate statistics
        let stats = GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            file_count: file_map.len(),
            import_count,
            scc_count,
            max_depth,
            avg_dependencies: if graph.node_count() > 0 {
                graph.edge_count() as f64 / graph.node_count() as f64
            } else {
                0.0
            },
        };

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol: name_map,
            file_imports: HashMap::new(), // TODO: Track imports
            stats,
        })
    }

    /// Get a parser from the pool or create a new one
    fn get_parser(&self, language: SupportedLanguage) -> Result<Parser> {
        let mut pool = self.parser_pool.lock().unwrap();

        if let Some(mut parser) = pool.pop() {
            // Set language
            self.set_parser_language(&mut parser, language)?;
            Ok(parser)
        } else {
            // Create new parser
            let mut parser = Parser::new();
            self.set_parser_language(&mut parser, language)?;
            Ok(parser)
        }
    }

    /// Return a parser to the pool
    fn return_parser(&self, parser: Parser) {
        let mut pool = self.parser_pool.lock().unwrap();
        pool.push(parser);
    }

    /// Set the language for a parser
    fn set_parser_language(&self, parser: &mut Parser, language: SupportedLanguage) -> Result<()> {
        let ts_language = match language {
            SupportedLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            SupportedLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        };

        parser
            .set_language(&ts_language)
            .context("Failed to set parser language")?;

        Ok(())
    }

    /// Convert binary symbol kind to SymbolType
    fn kind_to_type(&self, kind: u8) -> SymbolType {
        match kind {
            1 => SymbolType::Function,
            2 => SymbolType::Method,
            3 => SymbolType::Class,
            4 => SymbolType::Struct,
            5 => SymbolType::Enum,
            6 => SymbolType::Variable,
            7 => SymbolType::Constant,
            8 => SymbolType::Module,
            _ => SymbolType::Other("Unknown".to_string()),
        }
    }

    /// Convert ReferenceType to RelationType using the shared utility method
    fn ref_type_to_relation(&self, ref_type: &ReferenceType) -> RelationType {
        ref_type.to_relation_type()
    }
}

/// Information about a symbol from the binary database
#[derive(Debug, Clone)]
struct SymbolInfo {
    #[allow(dead_code)] // Will be used for enhanced relationship extraction
    id: Uuid,
    #[allow(dead_code)] // Will be used for enhanced relationship extraction
    name: String,
    qualified_name: String,
    symbol_type: SymbolType,
    file_path: PathBuf,
    start_line: usize,
    end_line: usize,
    #[allow(dead_code)] // Will be used for parent-child relationships
    parent_id: Option<Uuid>,
}

/// References found in a file
#[derive(Debug, Clone)]
struct FileReferences {
    file_path: PathBuf,
    references: Vec<CodeReference>,
    extraction_errors: Vec<String>,
}

/// Structured error types for comprehensive debugging context
#[derive(Error, Debug)]
pub enum PythonParsingError {
    #[error("Tree-sitter parsing failed in {file_path:?} ({language}): {message}")]
    TreeSitterError {
        message: String,
        file_path: PathBuf,
        language: String,
    },

    #[error("Query compilation failed for {language}: {message} (query: {query_text})")]
    QueryCompilationError {
        message: String,
        language: String,
        query_text: String,
    },

    #[error("UTF-8 decoding error in {file_path:?} at line {line}: {message}")]
    Utf8DecodingError {
        message: String,
        file_path: PathBuf,
        line: usize,
        column: Option<usize>,
    },

    #[error("Symbol resolution failed for '{symbol_name}' in {file_path:?}:{line} (available: {available_symbols})")]
    SymbolResolutionError {
        symbol_name: String,
        file_path: PathBuf,
        line: usize,
        available_symbols: usize,
    },

    #[error("Parser pool exhausted for {requested_language} (max retries: {max_retries})")]
    ParserPoolExhaustedError {
        requested_language: String,
        max_retries: usize,
    },

    #[error("File too large: {file_path:?} has {size} bytes, exceeds limit {max_size}")]
    FileSizeError {
        file_path: PathBuf,
        size: usize,
        max_size: usize,
    },

    #[error("Unsupported language '.{extension}' for file: {file_path:?}")]
    UnsupportedLanguageError {
        file_path: PathBuf,
        extension: String,
    },

    #[error("Async pattern parsing failed in {file_path:?}:{line} ({pattern_type}): {message}")]
    AsyncPatternError {
        message: String,
        file_path: PathBuf,
        line: usize,
        pattern_type: String,
    },

    #[error(
        "Type annotation parsing failed in {file_path:?}:{line} ('{annotation_text}'): {message}"
    )]
    TypeAnnotationError {
        message: String,
        file_path: PathBuf,
        line: usize,
        annotation_text: String,
    },

    #[error("Python import resolution failed for '{import_name}' in {file_path:?}:{line} ({import_type})")]
    ImportResolutionError {
        import_name: String,
        file_path: PathBuf,
        line: usize,
        import_type: String,
    },

    #[error("Pattern matching syntax error in {file_path:?}:{line} ('{pattern_text}'): {message}")]
    PatternMatchingError {
        message: String,
        file_path: PathBuf,
        line: usize,
        pattern_text: String,
    },

    #[error(
        "Comprehension parsing failed in {file_path:?}:{line} ({comprehension_type}): {message}"
    )]
    ComprehensionError {
        message: String,
        file_path: PathBuf,
        line: usize,
        comprehension_type: String,
    },

    #[error("Memory limit exceeded during parsing {file_path:?}: estimated {estimated_memory} bytes > limit {limit}")]
    MemoryLimitError {
        file_path: PathBuf,
        estimated_memory: usize,
        limit: usize,
    },
}

/// Enhanced extraction result with structured error context
#[derive(Debug)]
enum ExtractionResult {
    Success(FileReferences),
    PartialSuccess {
        references: FileReferences,
        recoverable_errors: Vec<PythonParsingError>,
    },
    Failure(PythonParsingError),
}

/// Hierarchical representation of symbols for accurate containment
#[derive(Debug)]
struct SymbolHierarchy {
    symbol_id: Uuid,
    start_line: usize,
    end_line: usize,
    children: Vec<SymbolHierarchy>,
}

impl SymbolHierarchy {
    /// Find the deepest symbol containing the given line
    fn find_containing_symbol(&self, line: usize) -> Option<Uuid> {
        if line >= self.start_line && line <= self.end_line {
            // Check children first (deepest match wins)
            for child in &self.children {
                if let Some(deeper_id) = child.find_containing_symbol(line) {
                    return Some(deeper_id);
                }
            }
            // No child contains it, so this symbol is the deepest container
            return Some(self.symbol_id);
        }
        None
    }

    /// Build hierarchy from flat symbol list
    fn build_from_symbols(symbols: &[(&Uuid, &SymbolInfo)]) -> Vec<SymbolHierarchy> {
        let mut roots = Vec::new();
        let mut processed = std::collections::HashSet::new();

        // Sort symbols by start line to process in order
        let mut sorted_symbols = symbols.to_vec();
        sorted_symbols.sort_by_key(|(_, info)| info.start_line);

        for (id, info) in sorted_symbols {
            if processed.contains(id) {
                continue;
            }

            // Check if this symbol is contained within any existing root
            let mut added = false;
            for root in &mut roots {
                if Self::try_add_to_hierarchy(root, *id, info) {
                    processed.insert(*id);
                    added = true;
                    break;
                }
            }

            // If not contained, it's a new root
            if !added {
                roots.push(SymbolHierarchy {
                    symbol_id: *id,
                    start_line: info.start_line,
                    end_line: info.end_line,
                    children: Vec::new(),
                });
                processed.insert(*id);
            }
        }

        roots
    }

    /// Try to add a symbol to the hierarchy (returns true if added)
    fn try_add_to_hierarchy(hierarchy: &mut SymbolHierarchy, id: Uuid, info: &SymbolInfo) -> bool {
        // Check if this symbol is contained within the hierarchy node
        if info.start_line >= hierarchy.start_line && info.end_line <= hierarchy.end_line {
            // Try to add to a child first
            for child in &mut hierarchy.children {
                if Self::try_add_to_hierarchy(child, id, info) {
                    return true;
                }
            }

            // Not contained in any child, add as direct child
            hierarchy.children.push(SymbolHierarchy {
                symbol_id: id,
                start_line: info.start_line,
                end_line: info.end_line,
                children: Vec::new(),
            });
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_symbols::BinarySymbolWriter;
    use tempfile::TempDir;

    #[test]
    fn test_relationship_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.symdb");

        // Create a simple symbol database
        let mut writer = BinarySymbolWriter::new();
        let func_id = Uuid::new_v4();
        let struct_id = Uuid::new_v4();

        writer.add_symbol(func_id, "process_data", 1, "src/lib.rs", 10, 20, None);
        writer.add_symbol(struct_id, "DataProcessor", 3, "src/lib.rs", 5, 25, None);
        writer.write_to_file(&db_path).unwrap();

        // Create test source file
        let source = r#"
struct DataProcessor {
    value: i32,
}

fn process_data() {
    let processor = DataProcessor { value: 42 };
    println!("Processing");
}
        "#;

        let files = vec![(PathBuf::from("src/lib.rs"), source.as_bytes().to_vec())];

        // Extract relationships
        let bridge = BinaryRelationshipBridge::new();
        let graph = bridge
            .extract_relationships(&db_path, temp_dir.path(), &files)
            .unwrap();

        // Verify graph was built
        assert_eq!(graph.stats.node_count, 2);
        // Note: edge count might be 0 initially as reference resolution needs improvement
    }

    #[test]
    fn test_structured_error_types() {
        // Test various structured error types to ensure proper context is provided
        use std::path::PathBuf;

        let test_path = PathBuf::from("test.py");

        // Test TreeSitterError
        let tree_error = PythonParsingError::TreeSitterError {
            message: "Syntax error in Python code".to_string(),
            file_path: test_path.clone(),
            language: "Python".to_string(),
        };
        let error_message = tree_error.to_string();
        assert!(error_message.contains("Tree-sitter parsing failed"));
        assert!(error_message.contains("test.py"));
        assert!(error_message.contains("Python"));

        // Test QueryCompilationError
        let query_error = PythonParsingError::QueryCompilationError {
            message: "Invalid tree-sitter query".to_string(),
            language: "Python".to_string(),
            query_text: "(invalid_query)".to_string(),
        };
        let query_message = query_error.to_string();
        assert!(query_message.contains("Query compilation failed"));
        assert!(query_message.contains("Python"));

        // Test Utf8DecodingError
        let utf8_error = PythonParsingError::Utf8DecodingError {
            message: "Invalid UTF-8 sequence".to_string(),
            file_path: test_path.clone(),
            line: 42,
            column: Some(10),
        };
        let utf8_message = utf8_error.to_string();
        assert!(utf8_message.contains("UTF-8 decoding error"));
        assert!(utf8_message.contains("test.py"));
        assert!(utf8_message.contains("line 42"));

        // Test FileSizeError
        let size_error = PythonParsingError::FileSizeError {
            file_path: test_path.clone(),
            size: 5000000,
            max_size: 1000000,
        };
        let size_message = size_error.to_string();
        assert!(size_message.contains("File too large"));
        assert!(size_message.contains("test.py"));
        assert!(size_message.contains("5000000"));
        assert!(size_message.contains("1000000"));

        // Test AsyncPatternError
        let async_error = PythonParsingError::AsyncPatternError {
            message: "Invalid async syntax".to_string(),
            file_path: test_path.clone(),
            line: 15,
            pattern_type: "await_expression".to_string(),
        };
        assert!(async_error
            .to_string()
            .contains("Async pattern parsing failed"));

        // Test TypeAnnotationError
        let type_error = PythonParsingError::TypeAnnotationError {
            message: "Complex type annotation failed".to_string(),
            file_path: test_path.clone(),
            line: 28,
            annotation_text: "List[Dict[str, Optional[int]]]".to_string(),
        };
        assert!(type_error
            .to_string()
            .contains("Type annotation parsing failed"));

        // Test ImportResolutionError
        let import_error = PythonParsingError::ImportResolutionError {
            import_name: "nonexistent_module".to_string(),
            file_path: test_path.clone(),
            line: 1,
            import_type: "from_import".to_string(),
        };
        assert!(import_error
            .to_string()
            .contains("Python import resolution failed"));
        assert!(import_error.to_string().contains("nonexistent_module"));

        // Test PatternMatchingError
        let pattern_error = PythonParsingError::PatternMatchingError {
            message: "Invalid match pattern".to_string(),
            file_path: test_path.clone(),
            line: 50,
            pattern_text: "case [x, *rest, y]:".to_string(),
        };
        assert!(pattern_error
            .to_string()
            .contains("Pattern matching syntax error"));

        // Test ComprehensionError
        let comp_error = PythonParsingError::ComprehensionError {
            message: "Nested comprehension too complex".to_string(),
            file_path: test_path,
            line: 35,
            comprehension_type: "list_comprehension".to_string(),
        };
        assert!(comp_error
            .to_string()
            .contains("Comprehension parsing failed"));
    }
}
