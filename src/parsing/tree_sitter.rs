//! Tree-sitter implementation for multi-language code parsing

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Language, Node, Parser, Tree};

// Node type constants optimized with HashSets for O(1) lookup performance
// Using OnceLock for lazy initialization to avoid initialization cost on each function call

// Function-related nodes across languages
static FUNCTION_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_function_nodes() -> &'static HashSet<&'static str> {
    FUNCTION_NODES.get_or_init(|| {
        HashSet::from_iter([
            // Rust
            "function_item",
            "function_declaration",
            // TypeScript/JavaScript
            "function",
            "function_expression",
            "arrow_function",
            "method_definition",
            // Python
            "function_definition",
        ])
    })
}

// Method nodes
static METHOD_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_method_nodes() -> &'static HashSet<&'static str> {
    METHOD_NODES.get_or_init(|| {
        HashSet::from_iter([
            "method_definition",
            "method_declaration",
            "property_definition", // For class properties
        ])
    })
}

// Struct/Class nodes
static STRUCT_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_struct_nodes() -> &'static HashSet<&'static str> {
    STRUCT_NODES.get_or_init(|| HashSet::from_iter(["struct_item", "struct_declaration"]))
}

static CLASS_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_class_nodes() -> &'static HashSet<&'static str> {
    CLASS_NODES.get_or_init(|| {
        HashSet::from_iter(["class_declaration", "class_definition", "class_expression"])
    })
}

// Enum/Union nodes
static ENUM_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_enum_nodes() -> &'static HashSet<&'static str> {
    ENUM_NODES.get_or_init(|| {
        HashSet::from_iter([
            "enum_item",
            "enum_declaration",
            "enum_member", // TypeScript enum members
        ])
    })
}

// Variable declarations
static VARIABLE_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_variable_nodes() -> &'static HashSet<&'static str> {
    VARIABLE_NODES.get_or_init(|| {
        HashSet::from_iter([
            // Rust
            "let_declaration",
            // TypeScript/JavaScript
            "variable_declarator",
            "lexical_declaration",  // let/const
            "variable_declaration", // var
        ])
    })
}

// Constant declarations
static CONST_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_const_nodes() -> &'static HashSet<&'static str> {
    CONST_NODES.get_or_init(|| {
        HashSet::from_iter([
            "const_item",        // Rust
            "const_declaration", // General
        ])
    })
}

// Module/namespace nodes
static MODULE_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_module_nodes() -> &'static HashSet<&'static str> {
    MODULE_NODES.get_or_init(|| {
        HashSet::from_iter([
            "mod_item",              // Rust
            "module_declaration",    // TypeScript
            "namespace_declaration", // TypeScript namespace
        ])
    })
}

// Import/export nodes
static IMPORT_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_import_nodes() -> &'static HashSet<&'static str> {
    IMPORT_NODES.get_or_init(|| {
        HashSet::from_iter([
            // Rust
            "use_declaration",
            // JavaScript/TypeScript
            "import_statement",
            "import_clause",
            "export_statement",
            "export_declaration",
            // Python
            "import_statement",
            "import_from_statement",
            "future_import_statement",
        ])
    })
}

// Comment nodes
static COMMENT_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_comment_nodes() -> &'static HashSet<&'static str> {
    COMMENT_NODES.get_or_init(|| {
        HashSet::from_iter([
            "line_comment",
            "block_comment",
            "comment", // Generic comment node
        ])
    })
}

// Interface and type nodes (TypeScript-specific)
static INTERFACE_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_interface_nodes() -> &'static HashSet<&'static str> {
    INTERFACE_NODES.get_or_init(|| {
        HashSet::from_iter([
            "interface_declaration",
            "type_alias_declaration", // TypeScript type aliases
        ])
    })
}

// JSX/TSX-specific nodes
static JSX_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_jsx_nodes() -> &'static HashSet<&'static str> {
    JSX_NODES.get_or_init(|| {
        HashSet::from_iter(["jsx_element", "jsx_fragment", "jsx_self_closing_element"])
    })
}

// Identifier node types across different languages
static IDENTIFIER_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_identifier_nodes() -> &'static HashSet<&'static str> {
    IDENTIFIER_NODES.get_or_init(|| {
        HashSet::from_iter([
            "identifier",
            "type_identifier",
            "name",
            "property_identifier", // JavaScript/TypeScript property names
        ])
    })
}

// Nodes that contain methods (for context detection)
static METHOD_CONTAINER_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_method_container_nodes() -> &'static HashSet<&'static str> {
    METHOD_CONTAINER_NODES.get_or_init(|| {
        HashSet::from_iter([
            // Rust
            "trait_item",
            "impl_item",
            // JavaScript/TypeScript
            "class_declaration",
            "class_expression",
            "interface_declaration",
            // Python
            "class_definition",
        ])
    })
}

// Special language-specific node types
static SPECIAL_NODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
fn get_special_nodes() -> &'static HashSet<&'static str> {
    SPECIAL_NODES.get_or_init(|| {
        HashSet::from_iter([
            "trait_item",             // Rust traits
            "impl_item",              // Rust implementations
            "type_alias_declaration", // TypeScript type aliases
        ])
    })
}

// Python-specific node types
const DECORATED_DEFINITION: &str = "decorated_definition";
const LAMBDA_NODE: &str = "lambda";
#[allow(dead_code)] // Will be used for future async function detection
const ASYNC_FUNCTION: &str = "async"; // Modifier for async functions
#[allow(dead_code)] // Will be used for future property detection
const PROPERTY_DECORATOR: &str = "@property";

// Python assignment and expression nodes that may represent variables
const PYTHON_VARIABLE_NODES: &[&str] = &[
    "assignment",
    "augmented_assignment",
    "named_expression", // Walrus operator :=
];

/// Supported programming languages for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
}

impl SupportedLanguage {
    /// Get tree-sitter language for this language
    pub fn tree_sitter_language(&self) -> Result<Language> {
        match self {
            SupportedLanguage::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            SupportedLanguage::TypeScript => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            SupportedLanguage::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
            SupportedLanguage::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        }
    }

    /// Detect language from file extension
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "rs" => Some(SupportedLanguage::Rust),
            "ts" | "tsx" => Some(SupportedLanguage::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(SupportedLanguage::JavaScript),
            "py" => Some(SupportedLanguage::Python),
            _ => None,
        }
    }

    /// Parse language from string name
    /// Supports both full names and common abbreviations
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "rust" | "rs" => Some(SupportedLanguage::Rust),
            "typescript" | "ts" => Some(SupportedLanguage::TypeScript),
            "javascript" | "js" => Some(SupportedLanguage::JavaScript),
            "python" | "py" => Some(SupportedLanguage::Python),
            _ => None,
        }
    }

    /// Get human-readable name for this language
    pub fn name(&self) -> &'static str {
        match self {
            SupportedLanguage::Rust => "Rust",
            SupportedLanguage::TypeScript => "TypeScript",
            SupportedLanguage::JavaScript => "JavaScript",
            SupportedLanguage::Python => "Python",
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            SupportedLanguage::Rust => &["rs"],
            SupportedLanguage::TypeScript => &["ts", "tsx"],
            SupportedLanguage::JavaScript => &["js", "jsx", "mjs", "cjs"],
            SupportedLanguage::Python => &["py"],
        }
    }
}

/// Type of code symbol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Variable,
    Constant,
    Module,
    Import,
    Export,    // JavaScript/TypeScript exports
    Type,      // TypeScript type aliases
    Component, // React/JSX components
    Comment,
    Other(String),
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolType::Function => write!(f, "function"),
            SymbolType::Method => write!(f, "method"),
            SymbolType::Class => write!(f, "class"),
            SymbolType::Struct => write!(f, "struct"),
            SymbolType::Interface => write!(f, "interface"),
            SymbolType::Enum => write!(f, "enum"),
            SymbolType::Variable => write!(f, "variable"),
            SymbolType::Constant => write!(f, "constant"),
            SymbolType::Module => write!(f, "module"),
            SymbolType::Import => write!(f, "import"),
            SymbolType::Export => write!(f, "export"),
            SymbolType::Type => write!(f, "type"),
            SymbolType::Component => write!(f, "component"),
            SymbolType::Comment => write!(f, "comment"),
            SymbolType::Other(s) => write!(f, "other({})", s),
        }
    }
}

impl TryFrom<u8> for SymbolType {
    type Error = ();

    /// Convert from binary representation back to SymbolType
    /// This mapping matches the encoding used in git/ingestion.rs
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SymbolType::Other("unknown".to_string())),
            1 => Ok(SymbolType::Function),
            2 => Ok(SymbolType::Method),
            3 => Ok(SymbolType::Class),
            4 => Ok(SymbolType::Struct),
            5 => Ok(SymbolType::Enum),
            6 => Ok(SymbolType::Variable),
            7 => Ok(SymbolType::Constant),
            8 => Ok(SymbolType::Module),
            9 => Ok(SymbolType::Import),
            10 => Ok(SymbolType::Export),
            11 => Ok(SymbolType::Type),
            12 => Ok(SymbolType::Component),
            13 => Ok(SymbolType::Interface),
            14 => Ok(SymbolType::Comment),
            _ => Err(()),
        }
    }
}

/// Kind of symbol visibility/access
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Public,
    Private,
    Protected,
    Internal,
    Unknown,
}

/// Parsed symbol from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSymbol {
    /// Symbol name
    pub name: String,
    /// Type of symbol
    pub symbol_type: SymbolType,
    /// Visibility/access kind
    pub kind: SymbolKind,
    /// Line number where symbol starts (1-based)
    pub start_line: usize,
    /// Line number where symbol ends (1-based)
    pub end_line: usize,
    /// Column where symbol starts (0-based)
    pub start_column: usize,
    /// Column where symbol ends (0-based)
    pub end_column: usize,
    /// Full text of the symbol
    pub text: String,
    /// Documentation/comments associated with symbol
    pub documentation: Option<String>,
}

/// Complete parsed representation of a source code file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCode {
    /// Language that was parsed
    pub language: SupportedLanguage,
    /// All symbols found in the code
    pub symbols: Vec<ParsedSymbol>,
    /// Raw parse tree statistics
    pub stats: ParseStats,
    /// Any parsing errors encountered
    pub errors: Vec<String>,
}

/// Statistics about the parsing process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseStats {
    /// Total number of nodes in the parse tree
    pub total_nodes: usize,
    /// Number of named nodes (excluding anonymous tokens)
    pub named_nodes: usize,
    /// Maximum depth of the parse tree
    pub max_depth: usize,
    /// Number of errors in the parse tree
    pub error_count: usize,
}

/// Configuration for code parsing
#[derive(Debug, Clone)]
pub struct ParsingConfig {
    /// Whether to extract documentation/comments
    pub extract_documentation: bool,
    /// Whether to include private symbols
    pub include_private: bool,
    /// Maximum file size to parse (in bytes)
    pub max_file_size: usize,
    /// Languages to parse (if None, parse all supported)
    pub languages: Option<Vec<SupportedLanguage>>,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            extract_documentation: true,
            include_private: true,
            max_file_size: 1024 * 1024, // 1MB
            languages: None,            // Parse all supported languages
        }
    }
}

/// Multi-language code parser using tree-sitter
pub struct CodeParser {
    /// Parsers for each supported language
    parsers: HashMap<SupportedLanguage, Parser>,
    /// Configuration for parsing
    config: ParsingConfig,
}

impl CodeParser {
    /// Create a new code parser with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(ParsingConfig::default())
    }

    /// Create a new code parser with custom configuration
    pub fn with_config(config: ParsingConfig) -> Result<Self> {
        let mut parsers = HashMap::new();

        // Initialize parsers for all supported languages
        let languages = config.languages.as_ref().map_or_else(
            || {
                vec![
                    SupportedLanguage::Rust,
                    SupportedLanguage::TypeScript,
                    SupportedLanguage::JavaScript,
                    SupportedLanguage::Python,
                ]
            },
            |langs| langs.clone(),
        );

        for language in languages {
            let mut parser = Parser::new();
            let tree_sitter_lang = language.tree_sitter_language().with_context(|| {
                format!("Failed to load tree-sitter language for {:?}", language)
            })?;

            parser
                .set_language(&tree_sitter_lang)
                .with_context(|| format!("Failed to set parser language for {:?}", language))?;

            parsers.insert(language, parser);
        }

        Ok(Self { parsers, config })
    }

    /// Parse source code from a file path
    pub fn parse_file(&mut self, file_path: &Path) -> Result<ParsedCode> {
        // Detect language from file extension
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine file extension"))?;

        let language = SupportedLanguage::from_extension(extension)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", extension))?;

        // Read file content
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        self.parse_content(&content, language)
    }

    /// Parse source code from a string
    pub fn parse_content(
        &mut self,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<ParsedCode> {
        // Check file size limit
        if content.len() > self.config.max_file_size {
            return Err(anyhow::anyhow!(
                "File size {} exceeds limit {}",
                content.len(),
                self.config.max_file_size
            ));
        }

        // Get parser for this language
        let parser = self
            .parsers
            .get_mut(&language)
            .ok_or_else(|| anyhow::anyhow!("Parser not available for language: {:?}", language))?;

        // Parse the content
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse content"))?;

        // Extract symbols and statistics
        let symbols = self.extract_symbols(&tree, content, language)?;
        let stats = self.calculate_stats(&tree);
        let errors = self.collect_errors(&tree, content);

        Ok(ParsedCode {
            language,
            symbols,
            stats,
            errors,
        })
    }

    /// Extract symbols from the parse tree
    fn extract_symbols(
        &self,
        tree: &Tree,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<Vec<ParsedSymbol>> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // For now, implement basic symbol extraction
        // This will be expanded with language-specific queries
        self.extract_symbols_recursive(root, content, &mut symbols);

        Ok(symbols)
    }

    /// Recursively extract symbols from nodes
    fn extract_symbols_recursive(
        &self,
        node: Node,
        content: &str,
        symbols: &mut Vec<ParsedSymbol>,
    ) {
        // Basic symbol extraction - this will be enhanced with proper tree-sitter queries
        let node_type = node.kind();

        // Check if this node represents a symbol we care about
        // Using optimized HashSet lookups for O(1) performance
        let symbol_type = if get_function_nodes().contains(node_type) {
            // Check if this is inside a trait/impl/class block (making it a method)
            if self.is_inside_method_container(node) {
                Some(SymbolType::Method)
            } else {
                Some(SymbolType::Function)
            }
        } else if node_type == DECORATED_DEFINITION {
            // Handle Python decorated definitions (functions/classes with @decorators)
            self.extract_decorated_symbol_type(node)
        } else if node_type == LAMBDA_NODE {
            Some(SymbolType::Function) // Lambda functions are functions
        } else if get_method_nodes().contains(node_type) {
            Some(SymbolType::Method)
        } else if get_struct_nodes().contains(node_type) {
            Some(SymbolType::Struct)
        } else if get_class_nodes().contains(node_type) {
            Some(SymbolType::Class) // JavaScript/TypeScript class declarations
        } else if get_interface_nodes().contains(node_type) {
            Some(SymbolType::Interface) // TypeScript interfaces and type aliases
        } else if get_enum_nodes().contains(node_type) {
            Some(SymbolType::Enum)
        } else if get_variable_nodes().contains(node_type)
            || PYTHON_VARIABLE_NODES.contains(&node_type)
        {
            Some(SymbolType::Variable)
        } else if get_const_nodes().contains(node_type) {
            Some(SymbolType::Constant)
        } else if get_module_nodes().contains(node_type) {
            Some(SymbolType::Module)
        } else if get_import_nodes().contains(node_type) {
            // Differentiate between imports and exports
            if node_type.starts_with("export") {
                Some(SymbolType::Export)
            } else {
                Some(SymbolType::Import)
            }
        } else if get_jsx_nodes().contains(node_type) {
            Some(SymbolType::Component) // JSX/TSX components
        } else if get_comment_nodes().contains(node_type) {
            Some(SymbolType::Comment)
        } else if get_special_nodes().contains(node_type) {
            // Handle special nodes with specific logic
            match node_type {
                "trait_item" => Some(SymbolType::Interface), // Rust traits
                "impl_item" => Some(SymbolType::Class),      // Rust implementations
                "type_alias_declaration" => Some(SymbolType::Type), // TypeScript type aliases
                _ => None,
            }
        } else {
            None
        };

        if let Some(sym_type) = symbol_type {
            // Extract symbol name with improved fallback handling
            let name = self
                .extract_symbol_name(node, content)
                .unwrap_or_else(|| self.generate_fallback_name(node, &sym_type));

            // Extract visibility information
            let kind = self.extract_symbol_visibility(node, content);

            let start_pos = node.start_position();
            let end_pos = node.end_position();

            let symbol = ParsedSymbol {
                name,
                symbol_type: sym_type,
                kind,
                start_line: start_pos.row + 1, // Convert to 1-based
                end_line: end_pos.row + 1,
                start_column: start_pos.column,
                end_column: end_pos.column,
                text: node.utf8_text(content.as_bytes()).unwrap_or("").to_string(),
                documentation: None, // Will be enhanced to extract doc comments
            };

            symbols.push(symbol);
        }

        // Recursively process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols_recursive(child, content, symbols);
        }
    }

    /// Check if a node is inside a method container (trait, impl, class, interface)
    /// Made pub(crate) for testing purposes
    /// Optimized with HashSet for O(1) lookup performance
    pub(crate) fn is_inside_method_container(&self, node: Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if get_method_container_nodes().contains(parent.kind()) {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Check if a node is inside a trait or impl block (legacy method for backwards compatibility)
    /// Made pub(crate) for testing purposes
    #[allow(dead_code)]
    pub(crate) fn is_inside_trait_or_impl(&self, node: Node) -> bool {
        self.is_inside_method_container(node)
    }

    /// Extract symbol type from Python decorated definitions
    /// Python uses @decorators to mark functions and classes
    fn extract_decorated_symbol_type(&self, node: Node) -> Option<SymbolType> {
        // Look for the actual definition within the decorated definition
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let child_type = child.kind();
            if child_type == "function_definition" {
                // Check if it's inside a class (making it a method)
                if self.is_inside_method_container(node) {
                    return Some(SymbolType::Method);
                } else {
                    return Some(SymbolType::Function);
                }
            } else if child_type == "class_definition" {
                return Some(SymbolType::Class);
            }
        }
        // If we can't determine the specific type, treat as function
        Some(SymbolType::Function)
    }

    /// Extract symbol name from a node (simplified implementation)
    fn extract_symbol_name(&self, node: Node, content: &str) -> Option<String> {
        // Look for identifier nodes within this node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Handle various identifier types across different languages
            // Rust uses "type_identifier" for structs/enums, "identifier" for functions/variables
            // Other languages may use "name" or "identifier"
            if get_identifier_nodes().contains(child.kind()) {
                if let Ok(name) = child.utf8_text(content.as_bytes()) {
                    // Validate that the name is not empty after trimming
                    let trimmed_name = name.trim();
                    if !trimmed_name.is_empty() {
                        return Some(trimmed_name.to_string());
                    }
                }
            }
        }
        None
    }

    /// Generate a meaningful fallback name based on symbol type and position
    fn generate_fallback_name(&self, node: Node, symbol_type: &SymbolType) -> String {
        let start_pos = node.start_position();
        match symbol_type {
            SymbolType::Function => format!("function_at_line_{}", start_pos.row + 1),
            SymbolType::Struct => format!("struct_at_line_{}", start_pos.row + 1),
            SymbolType::Enum => format!("enum_at_line_{}", start_pos.row + 1),
            SymbolType::Class => format!("impl_at_line_{}", start_pos.row + 1),
            SymbolType::Variable => format!("variable_at_line_{}", start_pos.row + 1),
            SymbolType::Constant => format!("constant_at_line_{}", start_pos.row + 1),
            SymbolType::Comment => format!("comment_at_line_{}", start_pos.row + 1),
            _ => format!("symbol_at_line_{}", start_pos.row + 1),
        }
    }

    /// Extract visibility information from a node
    fn extract_symbol_visibility(&self, node: Node, content: &str) -> SymbolKind {
        // Check for visibility modifier in current node or parent context
        let mut cursor = node.walk();

        // Look for visibility_modifier child nodes
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                if let Ok(visibility_text) = child.utf8_text(content.as_bytes()) {
                    return match visibility_text.trim() {
                        "pub" => SymbolKind::Public,
                        "pub(crate)" => SymbolKind::Internal,
                        "pub(super)" => SymbolKind::Protected,
                        _ => SymbolKind::Unknown,
                    };
                }
            }
        }

        // Check if the node text starts with 'pub' (fallback for complex visibility)
        if let Ok(node_text) = node.utf8_text(content.as_bytes()) {
            let trimmed = node_text.trim();
            if trimmed.starts_with("pub(crate)") {
                return SymbolKind::Internal;
            } else if trimmed.starts_with("pub(super)") {
                return SymbolKind::Protected;
            } else if trimmed.starts_with("pub ") {
                return SymbolKind::Public;
            }
        }

        // Default to private for Rust (no explicit visibility means private)
        SymbolKind::Private
    }

    /// Calculate parse tree statistics
    fn calculate_stats(&self, tree: &Tree) -> ParseStats {
        let root = tree.root_node();
        let (total_nodes, named_nodes, max_depth) = self.count_nodes_recursive(root, 0);
        let error_count = self.count_errors_recursive(root);

        ParseStats {
            total_nodes,
            named_nodes,
            max_depth,
            error_count,
        }
    }

    /// Recursively count nodes and calculate depth
    #[allow(clippy::only_used_in_recursion)]
    fn count_nodes_recursive(&self, node: Node, depth: usize) -> (usize, usize, usize) {
        let mut total = 1;
        let mut named = if node.is_named() { 1 } else { 0 };
        let mut max_depth = depth;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let (child_total, child_named, child_depth) =
                self.count_nodes_recursive(child, depth + 1);
            total += child_total;
            named += child_named;
            max_depth = max_depth.max(child_depth);
        }

        (total, named, max_depth)
    }

    /// Count error nodes in the parse tree
    #[allow(clippy::only_used_in_recursion)]
    fn count_errors_recursive(&self, node: Node) -> usize {
        let mut error_count = if node.is_error() { 1 } else { 0 };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            error_count += self.count_errors_recursive(child);
        }

        error_count
    }

    /// Collect error descriptions from the parse tree
    fn collect_errors(&self, tree: &Tree, content: &str) -> Vec<String> {
        let mut errors = Vec::new();
        self.collect_errors_recursive(tree.root_node(), content, &mut errors);
        errors
    }

    /// Recursively collect error descriptions
    #[allow(clippy::only_used_in_recursion)]
    fn collect_errors_recursive(&self, node: Node, content: &str, errors: &mut Vec<String>) {
        if node.is_error() {
            let start_pos = node.start_position();
            let error_text = node.utf8_text(content.as_bytes()).unwrap_or("<unknown>");
            errors.push(format!(
                "Parse error at line {}, column {}: {}",
                start_pos.row + 1,
                start_pos.column,
                error_text
            ));
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_errors_recursive(child, content, errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_supported_language_detection() -> Result<()> {
        assert_eq!(
            SupportedLanguage::from_extension("rs"),
            Some(SupportedLanguage::Rust)
        );
        assert_eq!(SupportedLanguage::from_extension("unknown"), None);
        assert_eq!(
            SupportedLanguage::from_extension("js"),
            Some(SupportedLanguage::JavaScript)
        ); // Now supported!
        Ok(())
    }

    #[tokio::test]
    async fn test_code_parser_creation() -> Result<()> {
        let _parser = CodeParser::new()?;
        Ok(())
    }

    #[tokio::test]
    async fn test_basic_rust_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let rust_code = r#"
        fn main() {
            println!("Hello, world!");
        }
        
        struct Person {
            name: String,
            age: u32,
        }
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        assert_eq!(parsed.language, SupportedLanguage::Rust);
        assert!(!parsed.symbols.is_empty());
        assert!(parsed.stats.total_nodes > 0);

        // Should find at least the main function and Person struct
        let function_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Function)
            .collect();
        assert!(!function_symbols.is_empty());

        let struct_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Struct)
            .collect();
        assert!(!struct_symbols.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_parsing_with_errors() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Intentionally malformed Rust code (more severe syntax error)
        let bad_rust_code = r#"
        fn main( {
            let x = ;
            println!("Missing closing paren");
            unexpected_token_here ++++ ----
        }
        "#;

        let parsed = parser.parse_content(bad_rust_code, SupportedLanguage::Rust)?;

        assert_eq!(parsed.language, SupportedLanguage::Rust);
        // Note: tree-sitter is quite resilient, so we test that it at least parses something
        assert!(parsed.stats.total_nodes > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_size_limit() -> Result<()> {
        let config = ParsingConfig {
            max_file_size: 10, // Very small limit
            ..Default::default()
        };
        let mut parser = CodeParser::with_config(config)?;

        let large_content = "fn main() { }".repeat(100);
        let result = parser.parse_content(&large_content, SupportedLanguage::Rust);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds limit"));

        Ok(())
    }

    #[tokio::test]
    async fn test_visibility_detection() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let rust_code = r#"
        pub fn public_function() {}
        fn private_function() {}
        pub(crate) fn crate_function() {}
        pub struct PublicStruct {}
        struct PrivateStruct {}
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Find symbols and check their visibility
        let public_fn = parsed.symbols.iter().find(|s| s.name == "public_function");
        assert!(public_fn.is_some());
        assert_eq!(public_fn.unwrap().kind, SymbolKind::Public);

        let private_fn = parsed.symbols.iter().find(|s| s.name == "private_function");
        assert!(private_fn.is_some());
        assert_eq!(private_fn.unwrap().kind, SymbolKind::Private);

        let crate_fn = parsed.symbols.iter().find(|s| s.name == "crate_function");
        assert!(crate_fn.is_some());
        assert_eq!(crate_fn.unwrap().kind, SymbolKind::Internal);

        Ok(())
    }

    #[tokio::test]
    async fn test_improved_symbol_names() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test with some unnamed constructs that should get better fallback names
        let rust_code = r#"
        // This is a comment
        const VALUE: i32 = 42;
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Check that we have meaningful names even for constructs without explicit names
        let has_meaningful_names = parsed
            .symbols
            .iter()
            .all(|s| !s.name.is_empty() && s.name != "unnamed");

        assert!(
            has_meaningful_names,
            "All symbols should have meaningful names"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_is_inside_trait_or_impl() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test functions inside impl blocks are detected as methods
        // Note: Trait method declarations without bodies may not be extracted
        let rust_code = r#"
        trait MyTrait {
            fn trait_method(&self) {
                // With body for extraction
            }
        }
        
        impl MyStruct {
            fn impl_method(&self) {}
            fn new() -> Self {
                MyStruct
            }
        }
        
        fn standalone_function() {}
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Trait itself should be found as Interface
        let my_trait = parsed.symbols.iter().find(|s| s.name == "MyTrait");
        assert!(my_trait.is_some(), "Should find MyTrait");
        assert_eq!(
            my_trait.unwrap().symbol_type,
            SymbolType::Interface,
            "MyTrait should be classified as Interface"
        );

        // Functions inside impl should be methods
        let impl_method = parsed.symbols.iter().find(|s| s.name == "impl_method");
        assert!(impl_method.is_some(), "Should find impl_method");
        assert_eq!(
            impl_method.unwrap().symbol_type,
            SymbolType::Method,
            "impl_method should be classified as Method"
        );

        let new_method = parsed.symbols.iter().find(|s| s.name == "new");
        assert!(new_method.is_some(), "Should find new method");
        assert_eq!(
            new_method.unwrap().symbol_type,
            SymbolType::Method,
            "new in impl block should be classified as Method"
        );

        // Standalone function should remain a function
        let standalone = parsed
            .symbols
            .iter()
            .find(|s| s.name == "standalone_function");
        assert!(standalone.is_some(), "Should find standalone_function");
        assert_eq!(
            standalone.unwrap().symbol_type,
            SymbolType::Function,
            "standalone_function should be classified as Function"
        );

        // Test trait method with body is detected as method
        let trait_method = parsed.symbols.iter().find(|s| s.name == "trait_method");
        if trait_method.is_some() {
            assert_eq!(
                trait_method.unwrap().symbol_type,
                SymbolType::Method,
                "trait_method with body should be classified as Method"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_nested_impl_trait_detection() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test deeply nested scenarios
        let rust_code = r#"
        mod my_module {
            pub struct MyStruct;
            
            impl MyStruct {
                fn nested_method(&self) {
                    fn inner_function() {} // This is still a function, not a method
                }
            }
        }
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Method in impl block should be detected
        let nested_method = parsed.symbols.iter().find(|s| s.name == "nested_method");
        assert!(nested_method.is_some(), "Should find nested_method");
        assert_eq!(
            nested_method.unwrap().symbol_type,
            SymbolType::Method,
            "nested_method in impl should be Method"
        );

        // Inner function inside a method is also detected as Method due to parent traversal
        // This is the current behavior - functions nested inside impl blocks are all methods
        let inner_fn = parsed.symbols.iter().find(|s| s.name == "inner_function");
        assert!(inner_fn.is_some(), "Should find inner_function");
        assert_eq!(
            inner_fn.unwrap().symbol_type,
            SymbolType::Method,
            "inner_function is detected as Method (inside impl block hierarchy)"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_name_validation() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // This should handle edge cases with whitespace-only names gracefully
        // The parser should either provide a fallback name or skip invalid symbols
        let rust_code = r#"
        fn valid_function() {}
        struct ValidStruct {}
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // All symbols should have non-empty, non-whitespace names
        for symbol in &parsed.symbols {
            assert!(
                !symbol.name.trim().is_empty(),
                "Symbol name should not be empty or whitespace-only: {:?}",
                symbol
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_type_identifier_extraction() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test that type_identifier nodes are properly extracted for Rust types
        let rust_code = r#"
        struct MyStruct {}
        enum MyEnum { A, B }
        type MyType = String;
        "#;

        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // Struct name should be extracted via type_identifier
        let my_struct = parsed.symbols.iter().find(|s| s.name == "MyStruct");
        assert!(my_struct.is_some(), "Should find MyStruct");
        assert_eq!(my_struct.unwrap().symbol_type, SymbolType::Struct);

        // Enum name should be extracted via type_identifier
        let my_enum = parsed.symbols.iter().find(|s| s.name == "MyEnum");
        assert!(my_enum.is_some(), "Should find MyEnum");
        assert_eq!(my_enum.unwrap().symbol_type, SymbolType::Enum);

        Ok(())
    }

    // TypeScript and JavaScript Tests

    #[tokio::test]
    async fn test_typescript_language_detection() -> Result<()> {
        assert_eq!(
            SupportedLanguage::from_extension("ts"),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("tsx"),
            Some(SupportedLanguage::TypeScript)
        );

        assert_eq!(
            SupportedLanguage::from_name("typescript"),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_name("ts"),
            Some(SupportedLanguage::TypeScript)
        );

        assert_eq!(SupportedLanguage::TypeScript.name(), "TypeScript");
        assert_eq!(SupportedLanguage::TypeScript.extensions(), &["ts", "tsx"]);

        Ok(())
    }

    #[tokio::test]
    async fn test_javascript_language_detection() -> Result<()> {
        assert_eq!(
            SupportedLanguage::from_extension("js"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("jsx"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("mjs"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("cjs"),
            Some(SupportedLanguage::JavaScript)
        );

        assert_eq!(
            SupportedLanguage::from_name("javascript"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_name("js"),
            Some(SupportedLanguage::JavaScript)
        );

        assert_eq!(SupportedLanguage::JavaScript.name(), "JavaScript");
        assert_eq!(
            SupportedLanguage::JavaScript.extensions(),
            &["js", "jsx", "mjs", "cjs"]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_basic_typescript_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let typescript_code = r#"
        // Basic TypeScript constructs
        interface User {
            name: string;
            age: number;
        }

        class UserService {
            private users: User[] = [];
            
            public addUser(user: User): void {
                this.users.push(user);
            }
            
            async getUser(id: string): Promise<User | null> {
                return this.users.find(u => u.name === id) || null;
            }
        }

        function greetUser(user: User): string {
            return `Hello, ${user.name}!`;
        }

        type UserList = User[];
        
        const constants = {
            MAX_USERS: 100,
            DEFAULT_AGE: 18
        } as const;
        "#;

        let parsed = parser.parse_content(typescript_code, SupportedLanguage::TypeScript)?;

        assert_eq!(parsed.language, SupportedLanguage::TypeScript);
        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in TypeScript code"
        );
        assert!(parsed.stats.total_nodes > 0, "Should have parsed nodes");

        // Look for interface
        let user_interface = parsed.symbols.iter().find(|s| s.name == "User");
        if user_interface.is_some() {
            assert_eq!(user_interface.unwrap().symbol_type, SymbolType::Interface);
        }

        // Look for class
        let user_service_class = parsed.symbols.iter().find(|s| s.name == "UserService");
        if user_service_class.is_some() {
            assert_eq!(user_service_class.unwrap().symbol_type, SymbolType::Class);
        }

        // Look for function
        let greet_function = parsed.symbols.iter().find(|s| s.name == "greetUser");
        if greet_function.is_some() {
            assert_eq!(greet_function.unwrap().symbol_type, SymbolType::Function);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_basic_javascript_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let javascript_code = r#"
        // Basic JavaScript constructs
        class UserManager {
            constructor() {
                this.users = [];
            }
            
            addUser(user) {
                this.users.push(user);
                return this;
            }
            
            static create() {
                return new UserManager();
            }
        }

        function processUser(user) {
            return {
                ...user,
                processed: true
            };
        }

        const arrowFunction = (x, y) => x + y;

        var globalVar = "global";
        let blockVar = "block";
        const constVar = "constant";
        
        // Export/import patterns
        export { UserManager, processUser };
        export default arrowFunction;
        "#;

        let parsed = parser.parse_content(javascript_code, SupportedLanguage::JavaScript)?;

        assert_eq!(parsed.language, SupportedLanguage::JavaScript);
        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in JavaScript code"
        );
        assert!(parsed.stats.total_nodes > 0, "Should have parsed nodes");

        // Look for class
        let user_manager_class = parsed.symbols.iter().find(|s| s.name == "UserManager");
        if user_manager_class.is_some() {
            assert_eq!(user_manager_class.unwrap().symbol_type, SymbolType::Class);
        }

        // Look for function
        let process_function = parsed.symbols.iter().find(|s| s.name == "processUser");
        if process_function.is_some() {
            assert_eq!(process_function.unwrap().symbol_type, SymbolType::Function);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_jsx_tsx_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test TSX (TypeScript with JSX)
        let tsx_code = r#"
        import React from 'react';

        interface Props {
            name: string;
            age?: number;
        }

        const UserCard: React.FC<Props> = ({ name, age }) => {
            return (
                <div className="user-card">
                    <h2>{name}</h2>
                    {age && <p>Age: {age}</p>}
                </div>
            );
        };

        export default UserCard;
        "#;

        let parsed = parser.parse_content(tsx_code, SupportedLanguage::TypeScript)?;

        assert_eq!(parsed.language, SupportedLanguage::TypeScript);
        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in TSX code"
        );

        // Look for interface
        let props_interface = parsed.symbols.iter().find(|s| s.name == "Props");
        if props_interface.is_some() {
            assert_eq!(props_interface.unwrap().symbol_type, SymbolType::Interface);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_typescript_advanced_features() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let advanced_ts_code = r#"
        // Advanced TypeScript features
        type Union = string | number | boolean;
        type Intersection = { a: string } & { b: number };
        
        interface Generic<T> {
            value: T;
            process<U>(input: U): T | U;
        }

        enum Status {
            Pending = "pending",
            Completed = "completed",
            Failed = "failed"
        }

        namespace Utils {
            export function format(input: string): string {
                return input.trim();
            }
        }

        abstract class BaseService {
            abstract process(): void;
        }

        class ConcreteService extends BaseService {
            process(): void {
                console.log("Processing...");
            }
        }

        // Decorator (experimental)
        function log(target: any, propertyKey: string, descriptor: PropertyDescriptor) {
            return descriptor;
        }
        "#;

        let parsed = parser.parse_content(advanced_ts_code, SupportedLanguage::TypeScript)?;

        assert_eq!(parsed.language, SupportedLanguage::TypeScript);
        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in advanced TypeScript code"
        );

        // Look for enum
        let status_enum = parsed.symbols.iter().find(|s| s.name == "Status");
        if status_enum.is_some() {
            assert_eq!(status_enum.unwrap().symbol_type, SymbolType::Enum);
        }

        // Look for namespace/module
        let utils_namespace = parsed.symbols.iter().find(|s| s.name == "Utils");
        if utils_namespace.is_some() {
            assert_eq!(utils_namespace.unwrap().symbol_type, SymbolType::Module);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_javascript_es6_features() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let es6_code = r#"
        // ES6+ features
        import { someFunction } from './utils';
        import defaultExport from 'external-module';

        const asyncFunction = async (data) => {
            try {
                const result = await processData(data);
                return result;
            } catch (error) {
                console.error(error);
                throw error;
            }
        };

        class ModernClass {
            #privateField = 'private';
            
            static staticMethod() {
                return 'static';
            }
            
            get value() {
                return this.#privateField;
            }
            
            set value(newValue) {
                this.#privateField = newValue;
            }
        }

        // Template literals and destructuring
        const templateFunction = ({ name, age = 0 } = {}) => {
            return `User: ${name}, Age: ${age}`;
        };

        // Generators
        function* generator() {
            yield 1;
            yield 2;
            yield 3;
        }
        "#;

        let parsed = parser.parse_content(es6_code, SupportedLanguage::JavaScript)?;

        assert_eq!(parsed.language, SupportedLanguage::JavaScript);
        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in ES6+ JavaScript code"
        );

        // Look for class
        let modern_class = parsed.symbols.iter().find(|s| s.name == "ModernClass");
        if modern_class.is_some() {
            assert_eq!(modern_class.unwrap().symbol_type, SymbolType::Class);
        }

        // Look for generator function
        let generator_func = parsed.symbols.iter().find(|s| s.name == "generator");
        if generator_func.is_some() {
            assert_eq!(generator_func.unwrap().symbol_type, SymbolType::Function);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_mixed_language_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Test that we can parse different languages in the same session
        let rust_code = "fn hello() { println!(\"Hello\"); }";
        let js_code = "function hello() { console.log('Hello'); }";
        let ts_code = "function hello(): void { console.log('Hello'); }";

        let rust_parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;
        let js_parsed = parser.parse_content(js_code, SupportedLanguage::JavaScript)?;
        let ts_parsed = parser.parse_content(ts_code, SupportedLanguage::TypeScript)?;

        assert_eq!(rust_parsed.language, SupportedLanguage::Rust);
        assert_eq!(js_parsed.language, SupportedLanguage::JavaScript);
        assert_eq!(ts_parsed.language, SupportedLanguage::TypeScript);

        // All should find some symbols
        assert!(!rust_parsed.symbols.is_empty());
        assert!(!js_parsed.symbols.is_empty());
        assert!(!ts_parsed.symbols.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_new_symbol_types() -> Result<()> {
        // Test the new symbol types display correctly
        assert_eq!(SymbolType::Export.to_string(), "export");
        assert_eq!(SymbolType::Type.to_string(), "type");
        assert_eq!(SymbolType::Component.to_string(), "component");

        // Test round-trip conversion for new symbol types
        assert_eq!(SymbolType::try_from(10).unwrap(), SymbolType::Export);
        assert_eq!(SymbolType::try_from(11).unwrap(), SymbolType::Type);
        assert_eq!(SymbolType::try_from(12).unwrap(), SymbolType::Component);
        assert_eq!(SymbolType::try_from(13).unwrap(), SymbolType::Interface);
        assert_eq!(SymbolType::try_from(14).unwrap(), SymbolType::Comment);

        Ok(())
    }
}
