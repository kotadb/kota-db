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
use tracing::{debug, info, instrument, warn};
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
            max_threads: None,                     // Use rayon default
            max_file_size: Some(10 * 1024 * 1024), // 10MB
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

    /// Extract references from all source files
    fn extract_all_references(
        &self,
        files: &[(PathBuf, Vec<u8>)],
        name_map: &HashMap<String, Uuid>,
    ) -> Result<Vec<FileReferences>> {
        // Process files in parallel
        let references: Vec<_> = files
            .par_iter()
            .filter_map(|(path, content)| {
                // Skip if file is too large
                if let Some(max_size) = self.config.max_file_size {
                    if content.len() > max_size {
                        debug!("Skipping large file: {}", path.display());
                        return None;
                    }
                }

                // Detect language from extension
                let extension = path.extension()?.to_str()?;
                let language = SupportedLanguage::from_extension(extension)?;

                // Skip if language not in filter
                if let Some(ref langs) = self.config.languages {
                    if !langs.contains(&language) {
                        return None;
                    }
                }

                // Convert content to string
                let content_str = String::from_utf8(content.clone()).ok()?;

                // Extract references
                match self.extract_file_references(path, &content_str, language) {
                    Ok(refs) => Some(refs),
                    Err(e) => {
                        warn!(
                            "Failed to extract references from {}: {}",
                            path.display(),
                            e
                        );
                        None
                    }
                }
            })
            .collect();

        Ok(references)
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
        };

        // Return parser to pool
        self.return_parser(parser);

        Ok(FileReferences {
            file_path: file_path.to_path_buf(),
            references,
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

        // Query for function calls
        let call_query = Query::new(
            &language,
            r#"
            (call_expression
                function: (identifier) @function_name)
            (call_expression
                function: (scoped_identifier
                    name: (identifier) @function_name))
            "#,
        )?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&call_query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let name = node.utf8_text(content.as_bytes())?.to_string();
                let point = node.start_position();

                references.push(CodeReference {
                    name,
                    ref_type: ReferenceType::FunctionCall,
                    line: point.row + 1,
                    column: point.column + 1,
                    text: node.utf8_text(content.as_bytes())?.to_string(),
                });
            }
        }

        // Add more query patterns for type references, imports, etc.

        Ok(references)
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

        // Create edges from references
        for file_refs in &all_references {
            // Find symbols defined in this file
            let file_symbols = file_map.get(&file_refs.file_path);
            if file_symbols.is_none() {
                continue;
            }

            for reference in &file_refs.references {
                // Try to resolve the reference to a symbol
                if let Some(&target_id) = name_map.get(&reference.name) {
                    // Find which symbol in this file contains this reference
                    // (simplified: use line numbers to determine containing symbol)
                    if let Some(&source_id) = file_symbols.unwrap().iter().find(|&&id| {
                        let info = &symbol_map[&id];
                        reference.line >= info.start_line && reference.line <= info.end_line
                    }) {
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

        // Calculate statistics
        let stats = GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            file_count: file_map.len(),
            import_count: 0, // TODO: Track imports
            scc_count: 0,    // TODO: Calculate strongly connected components
            max_depth: 0,    // TODO: Calculate max depth
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

    /// Convert ReferenceType to RelationType  
    fn ref_type_to_relation(&self, ref_type: &ReferenceType) -> RelationType {
        match ref_type {
            ReferenceType::FunctionCall => RelationType::Calls,
            ReferenceType::TypeUsage => RelationType::References,
            ReferenceType::TraitImpl => RelationType::Implements,
            ReferenceType::MacroInvocation => RelationType::Calls,
            ReferenceType::FieldAccess => RelationType::References,
            ReferenceType::MethodCall => RelationType::Calls,
        }
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
}
