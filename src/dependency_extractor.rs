//! Dependency extraction and call graph building for code analysis
//!
//! This module extends the symbol extraction pipeline to capture relationships
//! between symbols including function calls, type usage, imports, and module dependencies.

use anyhow::{Context, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{instrument, warn};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};
use uuid::Uuid;

use crate::parsing::{CodeParser, ParsedCode, ParsedSymbol, SupportedLanguage, SymbolType};
use crate::symbol_storage::{RelationType, SymbolEntry};

/// Dependency graph representation for code analysis
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying directed graph
    pub graph: DiGraph<SymbolNode, DependencyEdge>,
    /// Mapping from symbol ID to graph node index
    pub symbol_to_node: HashMap<Uuid, NodeIndex>,
    /// Mapping from qualified name to symbol ID for resolution
    pub name_to_symbol: HashMap<String, Uuid>,
    /// Import mappings for each file
    pub file_imports: HashMap<PathBuf, Vec<ImportStatement>>,
    /// Statistics about the graph
    pub stats: GraphStats,
}

/// Serializable representation of the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDependencyGraph {
    /// All nodes in the graph
    pub nodes: Vec<SymbolNode>,
    /// All edges in the graph with source and target IDs
    pub edges: Vec<SerializableEdge>,
    /// Mapping from qualified name to symbol ID
    pub name_to_symbol: HashMap<String, Uuid>,
    /// Import mappings for each file
    pub file_imports: HashMap<PathBuf, Vec<ImportStatement>>,
    /// Statistics about the graph
    pub stats: GraphStats,
}

/// Serializable edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEdge {
    /// Source symbol ID
    pub from_id: Uuid,
    /// Target symbol ID
    pub to_id: Uuid,
    /// Edge data
    pub edge: DependencyEdge,
}

/// Node in the dependency graph representing a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    /// Symbol ID from the symbol storage
    pub symbol_id: Uuid,
    /// Fully qualified name of the symbol
    pub qualified_name: String,
    /// Type of the symbol
    pub symbol_type: SymbolType,
    /// File path containing this symbol
    pub file_path: PathBuf,
    /// Number of incoming dependencies
    pub in_degree: usize,
    /// Number of outgoing dependencies
    pub out_degree: usize,
}

/// Edge in the dependency graph representing a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Type of relationship
    pub relation_type: RelationType,
    /// Line number where the reference occurs
    pub line_number: usize,
    /// Column number where the reference occurs
    pub column_number: usize,
    /// Context snippet around the reference
    pub context: Option<String>,
}

/// Import statement representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    /// The import path (e.g., "std::collections::HashMap")
    pub path: String,
    /// Imported items (e.g., ["HashMap", "HashSet"])
    pub items: Vec<String>,
    /// Alias if any (e.g., "use foo as bar")
    pub alias: Option<String>,
    /// Line number of the import
    pub line_number: usize,
    /// Whether it's a wildcard import (use foo::*)
    pub is_wildcard: bool,
}

/// Statistics about the dependency graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes (symbols)
    pub node_count: usize,
    /// Total number of edges (dependencies)
    pub edge_count: usize,
    /// Number of files analyzed
    pub file_count: usize,
    /// Number of import statements
    pub import_count: usize,
    /// Strongly connected components (potential circular dependencies)
    pub scc_count: usize,
    /// Maximum dependency depth
    pub max_depth: usize,
    /// Average dependencies per symbol
    pub avg_dependencies: f64,
}

/// Reference found in code (function call, type usage, etc.)
#[derive(Debug, Clone)]
pub struct CodeReference {
    /// Name being referenced
    pub name: String,
    /// Type of reference
    pub ref_type: ReferenceType,
    /// Location in source
    pub line: usize,
    pub column: usize,
    /// Full text of the reference
    pub text: String,
}

/// Type of reference found in code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceType {
    FunctionCall,
    TypeUsage,
    TraitImpl,
    MacroInvocation,
    FieldAccess,
    MethodCall,
}

/// Dependency extractor that analyzes code for relationships
pub struct DependencyExtractor {
    /// Code parser for symbol extraction (kept for future use)
    #[allow(dead_code)]
    parser: CodeParser,
    /// Parser pool for efficient reuse
    parser_pool: Arc<Mutex<Vec<Parser>>>,
    /// Tree-sitter queries for different languages
    queries: HashMap<SupportedLanguage, DependencyQueries>,
}

/// Tree-sitter queries for extracting dependencies
struct DependencyQueries {
    /// Query for function calls
    function_calls: Query,
    /// Query for type references
    type_references: Query,
    /// Query for imports
    imports: Query,
    /// Query for method calls
    method_calls: Query,
}

impl DependencyExtractor {
    /// Create a new dependency extractor
    pub fn new() -> Result<Self> {
        let parser = CodeParser::new()?;
        let parser_pool = Arc::new(Mutex::new(Vec::new()));
        let mut queries = HashMap::new();

        // Initialize Rust queries
        let rust_queries = Self::init_rust_queries()?;
        queries.insert(SupportedLanguage::Rust, rust_queries);

        Ok(Self {
            parser,
            parser_pool,
            queries,
        })
    }

    /// Initialize tree-sitter queries for Rust
    fn init_rust_queries() -> Result<DependencyQueries> {
        let language = tree_sitter_rust::LANGUAGE.into();

        // Query for function calls
        let function_calls = Query::new(
            &language,
            r#"
            (call_expression
                function: (identifier) @function_name)
            (call_expression
                function: (scoped_identifier
                    name: (identifier) @function_name))
            (call_expression
                function: (field_expression
                    field: (field_identifier) @method_name))
            "#,
        )
        .context("Failed to create function calls query")?;

        // Query for type references
        let type_references = Query::new(
            &language,
            r#"
            (type_identifier) @type_name
            (scoped_type_identifier
                name: (type_identifier) @type_name)
            (generic_type
                type: (type_identifier) @type_name)
            "#,
        )
        .context("Failed to create type references query")?;

        // Query for imports
        let imports = Query::new(
            &language,
            r#"
            (use_declaration
                argument: (scoped_identifier) @import_path)
            (use_declaration
                argument: (use_list) @import_list)
            (use_declaration
                argument: (use_as_clause
                    path: (scoped_identifier) @import_path
                    alias: (identifier) @import_alias))
            "#,
        )
        .context("Failed to create imports query")?;

        // Query for method calls (using Rust's actual node type)
        let method_calls = Query::new(
            &language,
            r#"
            (call_expression
                function: (field_expression
                    field: (field_identifier) @method_name))
            "#,
        )
        .context("Failed to create method calls query")?;

        Ok(DependencyQueries {
            function_calls,
            type_references,
            imports,
            method_calls,
        })
    }

    /// Get or create a parser from the pool
    fn acquire_parser(&self, language: SupportedLanguage) -> Result<Parser> {
        let mut pool = self.parser_pool.lock().unwrap();

        if let Some(mut parser) = pool.pop() {
            // Reuse existing parser
            let ts_language = language.tree_sitter_language()?;
            parser.set_language(&ts_language)?;
            Ok(parser)
        } else {
            // Create new parser
            let mut parser = Parser::new();
            let ts_language = language.tree_sitter_language()?;
            parser.set_language(&ts_language)?;
            Ok(parser)
        }
    }

    /// Return a parser to the pool for reuse
    fn release_parser(&self, parser: Parser) {
        let mut pool = self.parser_pool.lock().unwrap();
        // Limit pool size to prevent unbounded growth
        if pool.len() < 10 {
            pool.push(parser);
        }
    }

    /// Extract dependencies from a parsed code file
    #[instrument(skip(self, parsed_code, content))]
    pub fn extract_dependencies(
        &self,
        parsed_code: &ParsedCode,
        content: &str,
        file_path: &Path,
    ) -> Result<DependencyAnalysis> {
        let mut analysis = DependencyAnalysis {
            file_path: file_path.to_path_buf(),
            imports: Vec::new(),
            references: Vec::new(),
            symbols: parsed_code.symbols.clone(),
        };

        // Get a parser from the pool
        let mut parser = self.acquire_parser(parsed_code.language)?;

        let tree = parser
            .parse(content, None)
            .context("Failed to parse content for dependency extraction")?;

        // Extract imports
        analysis.imports = self.extract_imports(&tree, content, parsed_code.language)?;

        // Extract references (function calls, type usage, etc.)
        analysis.references = self.extract_references(&tree, content, parsed_code.language)?;

        // Return parser to pool for reuse
        self.release_parser(parser);

        Ok(analysis)
    }

    /// Extract import statements from the parse tree
    fn extract_imports(
        &self,
        tree: &Tree,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<Vec<ImportStatement>> {
        let queries = self
            .queries
            .get(&language)
            .context("No queries for language")?;

        let mut imports = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&queries.imports, tree.root_node(), content.as_bytes());

        while let Some(match_) = matches.next() {
            let mut import = ImportStatement {
                path: String::new(),
                items: Vec::new(),
                alias: None,
                line_number: 0,
                is_wildcard: false,
            };

            for capture in match_.captures {
                let node = capture.node;
                let pos = node.start_position();
                let text = node.utf8_text(content.as_bytes()).with_context(|| {
                    format!("Failed to parse UTF-8 at {}:{}", pos.row + 1, pos.column)
                })?;

                match queries.imports.capture_names()[capture.index as usize] {
                    "import_path" => {
                        import.path = text.to_string();
                        import.line_number = node.start_position().row + 1;
                    }
                    "import_alias" => {
                        import.alias = Some(text.to_string());
                    }
                    "import_list" => {
                        // Parse the use list to extract individual items
                        import.items = self.parse_use_list(node, content)?;
                    }
                    _ => {}
                }
            }

            // Check for wildcard imports
            if import.path.ends_with("*") {
                import.is_wildcard = true;
            }

            if !import.path.is_empty() {
                imports.push(import);
            }
        }

        // Also extract inline qualified paths (e.g., std::collections::HashMap)
        // These are implicit imports that should be tracked
        let inline_imports = self.extract_inline_qualified_paths(tree, content, language)?;
        imports.extend(inline_imports);

        Ok(imports)
    }

    /// Parse a use list node to extract individual imported items
    fn parse_use_list(&self, node: Node, content: &str) -> Result<Vec<String>> {
        let mut items = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(content.as_bytes()) {
                    items.push(text.to_string());
                }
            }
        }

        Ok(items)
    }

    /// Extract inline qualified paths (e.g., std::collections::HashMap used directly in code)
    fn extract_inline_qualified_paths(
        &self,
        tree: &Tree,
        content: &str,
        _language: SupportedLanguage,
    ) -> Result<Vec<ImportStatement>> {
        let mut inline_imports = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // Walk the tree looking for scoped type identifiers
        Self::walk_tree_for_qualified_paths(
            tree.root_node(),
            content,
            &mut inline_imports,
            &mut seen_paths,
        )?;

        Ok(inline_imports)
    }

    /// Recursively walk the tree to find qualified paths
    fn walk_tree_for_qualified_paths(
        node: Node,
        content: &str,
        imports: &mut Vec<ImportStatement>,
        seen_paths: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        // Check if this is a scoped type identifier (e.g., std::collections::HashMap)
        if node.kind() == "scoped_type_identifier" {
            if let Ok(full_path) = node.utf8_text(content.as_bytes()) {
                let full_path = full_path.to_string();

                // Only track paths that look like module paths (contain ::)
                if full_path.contains("::") && !seen_paths.contains(&full_path) {
                    seen_paths.insert(full_path.clone());

                    // Create an import statement for this inline path
                    // Keep the full path to match test expectations
                    imports.push(ImportStatement {
                        path: full_path.clone(),
                        items: Vec::new(),
                        alias: None,
                        line_number: node.start_position().row + 1,
                        is_wildcard: false,
                    });
                }
            }
        }

        // Check for scoped identifiers in non-type contexts too
        if node.kind() == "scoped_identifier" {
            if let Ok(full_path) = node.utf8_text(content.as_bytes()) {
                let full_path = full_path.to_string();

                // Track module-like paths but skip method calls like self.data
                // NOTE: This also filters out legitimate module names starting with "self"
                // (e.g., self_config::Module) but such naming is extremely rare in practice
                if full_path.contains("::")
                    && !full_path.starts_with("self")
                    && !seen_paths.contains(&full_path)
                {
                    seen_paths.insert(full_path.clone());

                    // Create an import statement for this inline path
                    // Keep the full path to match test expectations
                    imports.push(ImportStatement {
                        path: full_path.clone(),
                        items: Vec::new(),
                        alias: None,
                        line_number: node.start_position().row + 1,
                        is_wildcard: false,
                    });
                }
            }
        }

        // Recursively walk all children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_tree_for_qualified_paths(child, content, imports, seen_paths)?;
        }

        Ok(())
    }

    /// Extract references (function calls, type usage, etc.) from the parse tree
    fn extract_references(
        &self,
        tree: &Tree,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<Vec<CodeReference>> {
        let queries = self
            .queries
            .get(&language)
            .context("No queries for language")?;

        let mut references = Vec::new();

        // Extract function calls
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(
            &queries.function_calls,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let pos = node.start_position();
                let text = node.utf8_text(content.as_bytes()).with_context(|| {
                    format!(
                        "Failed to extract function call at {}:{}",
                        pos.row + 1,
                        pos.column
                    )
                })?;

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::FunctionCall,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        // Extract type references
        let mut matches = cursor.matches(
            &queries.type_references,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let pos = node.start_position();
                let text = node.utf8_text(content.as_bytes()).with_context(|| {
                    format!(
                        "Failed to extract type reference at {}:{}",
                        pos.row + 1,
                        pos.column
                    )
                })?;

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::TypeUsage,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        // Extract method calls
        let mut matches =
            cursor.matches(&queries.method_calls, tree.root_node(), content.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let pos = node.start_position();
                let text = node.utf8_text(content.as_bytes()).with_context(|| {
                    format!(
                        "Failed to extract method call at {}:{}",
                        pos.row + 1,
                        pos.column
                    )
                })?;

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::MethodCall,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        Ok(references)
    }

    /// Build a complete dependency graph from multiple analyzed files
    pub fn build_dependency_graph(
        &self,
        analyses: Vec<DependencyAnalysis>,
        symbol_entries: &[SymbolEntry],
    ) -> Result<DependencyGraph> {
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();
        let mut file_imports = HashMap::new();

        // First pass: Create nodes for all symbols
        for entry in symbol_entries {
            let node = SymbolNode {
                symbol_id: entry.id,
                qualified_name: entry.qualified_name.clone(),
                symbol_type: entry.symbol.symbol_type.clone(),
                file_path: entry.file_path.clone(),
                in_degree: 0,
                out_degree: 0,
            };

            let node_idx = graph.add_node(node);
            symbol_to_node.insert(entry.id, node_idx);
            name_to_symbol.insert(entry.qualified_name.clone(), entry.id);

            // Also index by simple name for fallback resolution
            name_to_symbol.insert(entry.symbol.name.clone(), entry.id);
        }

        // Second pass: Create edges based on references
        for analysis in &analyses {
            file_imports.insert(analysis.file_path.clone(), analysis.imports.clone());

            // Build spatial index for efficient symbol lookup
            let mut symbols_by_line: BTreeMap<usize, Vec<&SymbolEntry>> = BTreeMap::new();
            for entry in symbol_entries
                .iter()
                .filter(|e| e.file_path == analysis.file_path)
            {
                for line in entry.symbol.start_line..=entry.symbol.end_line {
                    symbols_by_line.entry(line).or_default().push(entry);
                }
            }

            for reference in &analysis.references {
                // Try to resolve the reference to a symbol
                let resolved_id =
                    self.resolve_reference(&reference.name, &analysis.imports, &name_to_symbol);
                if resolved_id.is_none() {
                    tracing::trace!(
                        "Failed to resolve reference '{}' at line {} in file {:?}",
                        reference.name,
                        reference.line,
                        analysis.file_path
                    );
                }
                if let Some(target_id) = resolved_id {
                    // Find the source symbol (the one containing this reference)
                    if let Some(source_symbol) =
                        self.find_containing_symbol_indexed(reference.line, &symbols_by_line)
                    {
                        tracing::debug!(
                            "Creating edge: {} -> {} (reference: {} at line {})",
                            source_symbol.qualified_name,
                            name_to_symbol
                                .iter()
                                .find_map(|(k, v)| if v == &target_id {
                                    Some(k.as_str())
                                } else {
                                    None
                                })
                                .unwrap_or("unknown"),
                            reference.name,
                            reference.line
                        );
                        if let (Some(&source_idx), Some(&target_idx)) = (
                            symbol_to_node.get(&source_symbol.id),
                            symbol_to_node.get(&target_id),
                        ) {
                            // Don't add self-references
                            if source_idx != target_idx {
                                let edge = DependencyEdge {
                                    relation_type: match reference.ref_type {
                                        ReferenceType::FunctionCall => RelationType::Calls,
                                        ReferenceType::TypeUsage => {
                                            RelationType::Custom("uses_type".to_string())
                                        }
                                        ReferenceType::MethodCall => RelationType::Calls,
                                        _ => RelationType::Custom("references".to_string()),
                                    },
                                    line_number: reference.line,
                                    column_number: reference.column,
                                    context: Some(reference.text.clone()),
                                };

                                graph.add_edge(source_idx, target_idx, edge);
                            }
                        }
                    }
                }
            }
        }

        // Calculate statistics
        let stats = self.calculate_graph_stats(&graph, &analyses);

        // Update in/out degrees for nodes
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

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol,
            file_imports,
            stats,
        })
    }

    /// Resolve a reference name to a symbol ID with sophisticated import handling
    fn resolve_reference(
        &self,
        name: &str,
        imports: &[ImportStatement],
        name_to_symbol: &HashMap<String, Uuid>,
    ) -> Option<Uuid> {
        // Direct lookup - exact match
        if let Some(&id) = name_to_symbol.get(name) {
            return Some(id);
        }

        // Try with import prefixes
        for import in imports {
            // Handle wildcard imports (use foo::*)
            if import.is_wildcard {
                let base_path = import.path.trim_end_matches("*").trim_end_matches("::");
                let qualified = format!("{}::{}", base_path, name);
                if let Some(&id) = name_to_symbol.get(&qualified) {
                    return Some(id);
                }
            }

            // Handle aliased imports (use foo as bar)
            if let Some(alias) = &import.alias {
                if name == alias || name.starts_with(&format!("{}::", alias)) {
                    let without_alias = name.strip_prefix(alias).unwrap_or(name);
                    let without_alias = without_alias.strip_prefix("::").unwrap_or(without_alias);
                    let qualified = if without_alias.is_empty() {
                        import.path.clone()
                    } else {
                        format!("{}::{}", import.path, without_alias)
                    };
                    if let Some(&id) = name_to_symbol.get(&qualified) {
                        return Some(id);
                    }
                }
            }

            // Check if this import could resolve the reference
            if import.items.contains(&name.to_string()) {
                let qualified = format!("{}::{}", import.path, name);
                if let Some(&id) = name_to_symbol.get(&qualified) {
                    return Some(id);
                }
            }

            // Handle nested imports (use foo::{bar, baz})
            for item in &import.items {
                if name == item || name.starts_with(&format!("{}::", item)) {
                    let qualified = format!("{}::{}", import.path, name);
                    if let Some(&id) = name_to_symbol.get(&qualified) {
                        return Some(id);
                    }
                }
            }

            // Check if it's a path that starts with an imported module
            if name.contains("::") {
                let parts: Vec<&str> = name.split("::").collect();
                if !parts.is_empty() {
                    // Check if first part matches any imported item
                    if import.items.contains(&parts[0].to_string()) {
                        let qualified = format!("{}::{}", import.path, name);
                        if let Some(&id) = name_to_symbol.get(&qualified) {
                            return Some(id);
                        }
                    }

                    // Check if first part matches the last segment of import path
                    if let Some(last_segment) = import.path.split("::").last() {
                        if last_segment == parts[0] {
                            let rest = parts[1..].join("::");
                            let qualified = format!("{}::{}", import.path, rest);
                            if let Some(&id) = name_to_symbol.get(&qualified) {
                                return Some(id);
                            }
                        }
                    }
                }
            }
        }

        // Try standard library resolution for common types
        if !name.contains("::") {
            let std_types = ["String", "Vec", "HashMap", "Option", "Result"];
            if std_types.contains(&name) {
                let qualified = format!("std::{}", name);
                if let Some(&id) = name_to_symbol.get(&qualified) {
                    return Some(id);
                }
            }
        }

        None
    }

    /// Find the symbol that contains a given line number (optimized with spatial index)
    fn find_containing_symbol_indexed<'a>(
        &self,
        line: usize,
        symbols_by_line: &BTreeMap<usize, Vec<&'a SymbolEntry>>,
    ) -> Option<&'a SymbolEntry> {
        symbols_by_line.get(&line).and_then(|symbols| {
            // Among symbols at this line, find the one with smallest scope
            symbols
                .iter()
                .min_by_key(|s| s.symbol.end_line - s.symbol.start_line)
                .copied()
        })
    }

    /// Find the symbol that contains a given line number (fallback for compatibility)
    #[allow(dead_code)]
    fn find_containing_symbol<'a>(
        &self,
        line: usize,
        symbols: &[&'a SymbolEntry],
    ) -> Option<&'a SymbolEntry> {
        symbols
            .iter()
            .filter(|s| s.symbol.start_line <= line && s.symbol.end_line >= line)
            .min_by_key(|s| s.symbol.end_line - s.symbol.start_line)
            .copied()
    }

    /// Calculate statistics for the dependency graph
    fn calculate_graph_stats(
        &self,
        graph: &DiGraph<SymbolNode, DependencyEdge>,
        analyses: &[DependencyAnalysis],
    ) -> GraphStats {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();
        let file_count = analyses.len();
        let import_count: usize = analyses.iter().map(|a| a.imports.len()).sum();

        // Find strongly connected components
        let scc = petgraph::algo::kosaraju_scc(graph);
        let scc_count = scc.iter().filter(|component| component.len() > 1).count();

        // Calculate maximum depth using BFS from root nodes
        let max_depth = self.calculate_max_depth(graph);

        let avg_dependencies = if node_count > 0 {
            edge_count as f64 / node_count as f64
        } else {
            0.0
        };

        GraphStats {
            node_count,
            edge_count,
            file_count,
            import_count,
            scc_count,
            max_depth,
            avg_dependencies,
        }
    }

    /// Calculate the maximum dependency depth in the graph
    fn calculate_max_depth(&self, graph: &DiGraph<SymbolNode, DependencyEdge>) -> usize {
        let mut max_depth = 0;

        // Find root nodes (nodes with no incoming edges)
        let root_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&idx| {
                graph
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .count()
                    == 0
            })
            .collect();

        // BFS from each root to find maximum depth
        for root in root_nodes {
            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            queue.push_back((root, 0));

            while let Some((node, depth)) = queue.pop_front() {
                if visited.contains(&node) {
                    continue;
                }
                visited.insert(node);
                max_depth = max_depth.max(depth);

                for edge in graph.edges(node) {
                    queue.push_back((edge.target(), depth + 1));
                }
            }
        }

        max_depth
    }
}

/// Result of dependency analysis for a single file
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// Path to the analyzed file
    pub file_path: PathBuf,
    /// Import statements found
    pub imports: Vec<ImportStatement>,
    /// References found in the code
    pub references: Vec<CodeReference>,
    /// Symbols defined in this file
    pub symbols: Vec<ParsedSymbol>,
}

impl DependencyGraph {
    /// Find all dependencies of a given symbol
    pub fn find_dependencies(&self, symbol_id: Uuid) -> Vec<(Uuid, RelationType)> {
        if let Some(&node_idx) = self.symbol_to_node.get(&symbol_id) {
            self.graph
                .edges(node_idx)
                .map(|edge| {
                    let target_node = &self.graph[edge.target()];
                    (target_node.symbol_id, edge.weight().relation_type.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find all symbols that depend on a given symbol
    pub fn find_dependents(&self, symbol_id: Uuid) -> Vec<(Uuid, RelationType)> {
        if let Some(&node_idx) = self.symbol_to_node.get(&symbol_id) {
            self.graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| {
                    let source_node = &self.graph[edge.source()];
                    (source_node.symbol_id, edge.weight().relation_type.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Detect circular dependencies in the graph
    pub fn find_circular_dependencies(&self) -> Vec<Vec<Uuid>> {
        let scc = petgraph::algo::kosaraju_scc(&self.graph);

        scc.into_iter()
            .filter(|component| component.len() > 1)
            .map(|component| {
                component
                    .into_iter()
                    .map(|idx| self.graph[idx].symbol_id)
                    .collect()
            })
            .collect()
    }

    /// Generate visualization data in DOT format
    pub fn to_dot(&self) -> String {
        use petgraph::dot::{Config, Dot};

        let dot = Dot::with_config(&self.graph, &[Config::EdgeNoLabel]);
        format!("{:?}", dot)
    }

    /// Convert to serializable representation
    pub fn to_serializable(&self) -> SerializableDependencyGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Collect all nodes
        for node_idx in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(node_idx) {
                nodes.push(node.clone());
            }
        }

        // Collect all edges
        for edge_ref in self.graph.edge_references() {
            let source_node = &self.graph[edge_ref.source()];
            let target_node = &self.graph[edge_ref.target()];

            edges.push(SerializableEdge {
                from_id: source_node.symbol_id,
                to_id: target_node.symbol_id,
                edge: edge_ref.weight().clone(),
            });
        }

        SerializableDependencyGraph {
            nodes,
            edges,
            name_to_symbol: self.name_to_symbol.clone(),
            file_imports: self.file_imports.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Reconstruct from serializable representation
    pub fn from_serializable(serializable: SerializableDependencyGraph) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();

        // Add all nodes
        for node in serializable.nodes {
            let node_idx = graph.add_node(node.clone());
            symbol_to_node.insert(node.symbol_id, node_idx);
        }

        // Add all edges
        for edge_data in serializable.edges {
            if let (Some(&from_idx), Some(&to_idx)) = (
                symbol_to_node.get(&edge_data.from_id),
                symbol_to_node.get(&edge_data.to_id),
            ) {
                graph.add_edge(from_idx, to_idx, edge_data.edge);
            }
        }

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol: serializable.name_to_symbol,
            file_imports: serializable.file_imports,
            stats: serializable.stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dependency_extraction() {
        let extractor = DependencyExtractor::new().unwrap();

        let rust_code = r#"
use std::collections::HashMap;
use crate::utils::helper;

fn process_data(data: HashMap<String, i32>) -> i32 {
    let result = helper::calculate(data);
    validate_result(result)
}

fn validate_result(value: i32) -> i32 {
    if value > 0 {
        value * 2
    } else {
        0
    }
}
"#;

        // Parse the code first
        let mut parser = CodeParser::new().unwrap();
        let parsed = parser
            .parse_content(rust_code, SupportedLanguage::Rust)
            .unwrap();

        // Extract dependencies
        let path = PathBuf::from("test.rs");
        let analysis = extractor
            .extract_dependencies(&parsed, rust_code, &path)
            .unwrap();

        // Check imports - we should have at least 2 (may have more from inline paths)
        assert!(analysis.imports.len() >= 2);
        assert!(analysis.imports.iter().any(|i| i.path.contains("HashMap")));
        assert!(analysis.imports.iter().any(|i| i.path.contains("helper")));

        // Check references
        assert!(analysis.references.iter().any(|r| r.name == "HashMap"));
        assert!(analysis.references.iter().any(|r| r.name == "calculate"));
        assert!(analysis
            .references
            .iter()
            .any(|r| r.name == "validate_result"));
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        // Create a simple circular dependency graph
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();

        // Create three symbols that form a cycle: A -> B -> C -> A
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();

        let node_a = graph.add_node(SymbolNode {
            symbol_id: id_a,
            qualified_name: "module::A".to_string(),
            symbol_type: SymbolType::Function,
            file_path: PathBuf::from("test.rs"),
            in_degree: 0,
            out_degree: 0,
        });

        let node_b = graph.add_node(SymbolNode {
            symbol_id: id_b,
            qualified_name: "module::B".to_string(),
            symbol_type: SymbolType::Function,
            file_path: PathBuf::from("test.rs"),
            in_degree: 0,
            out_degree: 0,
        });

        let node_c = graph.add_node(SymbolNode {
            symbol_id: id_c,
            qualified_name: "module::C".to_string(),
            symbol_type: SymbolType::Function,
            file_path: PathBuf::from("test.rs"),
            in_degree: 0,
            out_degree: 0,
        });

        // Add edges to create cycle
        graph.add_edge(
            node_a,
            node_b,
            DependencyEdge {
                relation_type: RelationType::Calls,
                line_number: 10,
                column_number: 5,
                context: Some("A calls B".to_string()),
            },
        );

        graph.add_edge(
            node_b,
            node_c,
            DependencyEdge {
                relation_type: RelationType::Calls,
                line_number: 20,
                column_number: 5,
                context: Some("B calls C".to_string()),
            },
        );

        graph.add_edge(
            node_c,
            node_a,
            DependencyEdge {
                relation_type: RelationType::Calls,
                line_number: 30,
                column_number: 5,
                context: Some("C calls A".to_string()),
            },
        );

        symbol_to_node.insert(id_a, node_a);
        symbol_to_node.insert(id_b, node_b);
        symbol_to_node.insert(id_c, node_c);

        name_to_symbol.insert("module::A".to_string(), id_a);
        name_to_symbol.insert("module::B".to_string(), id_b);
        name_to_symbol.insert("module::C".to_string(), id_c);

        let dep_graph = DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol,
            file_imports: HashMap::new(),
            stats: GraphStats::default(),
        };

        // Test circular dependency detection
        let cycles = dep_graph.find_circular_dependencies();
        assert_eq!(cycles.len(), 1, "Should find exactly one cycle");

        let cycle = &cycles[0];
        assert_eq!(cycle.len(), 3, "Cycle should contain 3 nodes");
        assert!(cycle.contains(&id_a));
        assert!(cycle.contains(&id_b));
        assert!(cycle.contains(&id_c));
    }

    #[tokio::test]
    async fn test_inline_qualified_path_extraction() {
        let extractor = DependencyExtractor::new().unwrap();

        let rust_code = r#"
pub struct DataStore {
    // Inline type path in field type annotation
    cache: std::collections::HashMap<String, Vec<u8>>,
    mutex: std::sync::Mutex<i32>,
}

impl DataStore {
    fn process(&self) -> std::io::Result<()> {
        // Inline path in function call
        let data = std::fs::read_to_string("file.txt")?;
        // Module path in nested call
        let parsed = crate::parser::utils::parse_data(&data);
        Ok(())
    }
}
"#;

        // Parse the code first
        let mut parser = CodeParser::new().unwrap();
        let parsed = parser
            .parse_content(rust_code, SupportedLanguage::Rust)
            .unwrap();

        // Extract dependencies
        let path = PathBuf::from("test.rs");
        let analysis = extractor
            .extract_dependencies(&parsed, rust_code, &path)
            .unwrap();

        // Verify inline paths are detected as imports
        assert!(
            analysis
                .imports
                .iter()
                .any(|i| i.path == "std::collections::HashMap"),
            "Should detect std::collections::HashMap"
        );
        assert!(
            analysis
                .imports
                .iter()
                .any(|i| i.path == "std::sync::Mutex"),
            "Should detect std::sync::Mutex"
        );
        assert!(
            analysis.imports.iter().any(|i| i.path == "std::io::Result"),
            "Should detect std::io::Result"
        );
        assert!(
            analysis
                .imports
                .iter()
                .any(|i| i.path == "std::fs::read_to_string"),
            "Should detect std::fs::read_to_string"
        );
        assert!(
            analysis
                .imports
                .iter()
                .any(|i| i.path == "crate::parser::utils::parse_data"),
            "Should detect crate::parser::utils::parse_data"
        );
    }

    #[tokio::test]
    async fn test_import_resolution_edge_cases() {
        let extractor = DependencyExtractor::new().unwrap();

        // Test wildcard imports
        let mut imports = vec![ImportStatement {
            path: "std::collections".to_string(),
            items: vec![],
            alias: None,
            line_number: 1,
            is_wildcard: true,
        }];

        let mut name_to_symbol = HashMap::new();
        let id = Uuid::new_v4();
        name_to_symbol.insert("std::collections::HashMap".to_string(), id);

        // Should resolve HashMap through wildcard import
        let resolved = extractor.resolve_reference("HashMap", &imports, &name_to_symbol);
        assert_eq!(resolved, Some(id));

        // Test aliased imports
        imports.push(ImportStatement {
            path: "crate::utils::helper".to_string(),
            items: vec![],
            alias: Some("h".to_string()),
            line_number: 2,
            is_wildcard: false,
        });

        let helper_id = Uuid::new_v4();
        name_to_symbol.insert("crate::utils::helper::calculate".to_string(), helper_id);

        // Should resolve through alias
        let resolved = extractor.resolve_reference("h::calculate", &imports, &name_to_symbol);
        assert_eq!(resolved, Some(helper_id));
    }

    #[tokio::test]
    async fn test_serialization_round_trip() {
        let _extractor = DependencyExtractor::new().unwrap();

        // Create a simple graph
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();

        let id = Uuid::new_v4();
        let node = graph.add_node(SymbolNode {
            symbol_id: id,
            qualified_name: "test::function".to_string(),
            symbol_type: SymbolType::Function,
            file_path: PathBuf::from("test.rs"),
            in_degree: 0,
            out_degree: 0,
        });

        symbol_to_node.insert(id, node);
        name_to_symbol.insert("test::function".to_string(), id);

        let original = DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol: name_to_symbol.clone(),
            file_imports: HashMap::new(),
            stats: GraphStats::default(),
        };

        // Convert to serializable and back
        let serializable = original.to_serializable();
        let reconstructed = DependencyGraph::from_serializable(serializable).unwrap();

        // Verify reconstruction
        assert_eq!(reconstructed.name_to_symbol, name_to_symbol);
        assert_eq!(reconstructed.graph.node_count(), 1);
        assert_eq!(reconstructed.graph.edge_count(), 0);
    }
}
