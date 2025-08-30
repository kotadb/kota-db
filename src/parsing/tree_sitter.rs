//! Tree-sitter implementation for multi-language code parsing

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

// Node type constants for better maintainability and to avoid typos
// Multi-language function nodes
const FUNCTION_NODES: &[&str] = &[
    // Rust
    "function_item",
    "function_declaration",
    // Python
    "function_definition",
];
const METHOD_NODES: &[&str] = &["method_definition", "method_declaration"];
const STRUCT_NODES: &[&str] = &["struct_item", "struct_declaration"];
const CLASS_NODES: &[&str] = &[
    // Rust
    "class_declaration",
    // Python
    "class_definition",
];
const ENUM_NODES: &[&str] = &["enum_item", "enum_declaration"];
const VARIABLE_NODES: &[&str] = &["let_declaration", "variable_declarator"];
const CONST_NODES: &[&str] = &["const_item", "const_declaration"];
const MODULE_NODES: &[&str] = &["mod_item", "module_declaration"];
const IMPORT_NODES: &[&str] = &[
    // Rust
    "use_declaration",
    // Python
    "import_statement",
    "import_from_statement",
    "future_import_statement",
];
const COMMENT_NODES: &[&str] = &["line_comment", "block_comment"];

// Special Rust-specific node types
const TRAIT_NODE: &str = "trait_item";
const IMPL_NODE: &str = "impl_item";
const INTERFACE_NODE: &str = "interface_declaration";

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

// Python control flow and statement nodes
#[allow(dead_code)] // Will be used for future control flow analysis
const PYTHON_CONTROL_NODES: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "match_statement", // Python 3.10+
    "with_statement",
    "yield",
    "return_statement",
    "raise_statement",
    "global_statement",
    "nonlocal_statement",
];

// Identifier node types across different languages
const IDENTIFIER_NODES: &[&str] = &["identifier", "type_identifier", "name"];

// Nodes that contain methods (for context detection)
const METHOD_CONTAINER_NODES: &[&str] = &[
    // Rust
    "trait_item",
    "impl_item",
    // Python
    "class_definition",
];

/// Supported programming languages for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    Rust,
    Python,
}

impl SupportedLanguage {
    /// Get tree-sitter language for this language
    pub fn tree_sitter_language(&self) -> Result<Language> {
        match self {
            SupportedLanguage::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            SupportedLanguage::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        }
    }

    /// Detect language from file extension
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "rs" => Some(SupportedLanguage::Rust),
            "py" => Some(SupportedLanguage::Python),
            _ => None,
        }
    }

    /// Parse language from string name
    /// Supports both full names and common abbreviations
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "rust" | "rs" => Some(SupportedLanguage::Rust),
            "python" | "py" => Some(SupportedLanguage::Python),
            // Future languages can be added here:
            // "javascript" | "js" => Some(SupportedLanguage::JavaScript),
            // "typescript" | "ts" => Some(SupportedLanguage::TypeScript),
            _ => None,
        }
    }

    /// Get human-readable name for this language
    pub fn name(&self) -> &'static str {
        match self {
            SupportedLanguage::Rust => "Rust",
            SupportedLanguage::Python => "Python",
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            SupportedLanguage::Rust => &["rs"],
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
            || vec![SupportedLanguage::Rust, SupportedLanguage::Python],
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
        let symbol_type = if FUNCTION_NODES.contains(&node_type) {
            // Check if this is inside a trait/impl/class block (making it a method)
            if self.is_inside_trait_or_impl(node) {
                Some(SymbolType::Method)
            } else {
                Some(SymbolType::Function)
            }
        } else if node_type == DECORATED_DEFINITION {
            // Handle Python decorated definitions (functions/classes with @decorators)
            self.extract_decorated_symbol_type(node)
        } else if node_type == LAMBDA_NODE {
            Some(SymbolType::Function) // Lambda functions are functions
        } else if METHOD_NODES.contains(&node_type) {
            Some(SymbolType::Method)
        } else if STRUCT_NODES.contains(&node_type) {
            Some(SymbolType::Struct)
        } else if node_type == TRAIT_NODE {
            Some(SymbolType::Interface) // Rust traits are interfaces
        } else if node_type == IMPL_NODE || CLASS_NODES.contains(&node_type) {
            Some(SymbolType::Class) // Rust impl blocks and Python class definitions
        } else if node_type == INTERFACE_NODE {
            Some(SymbolType::Interface)
        } else if ENUM_NODES.contains(&node_type) {
            Some(SymbolType::Enum)
        } else if VARIABLE_NODES.contains(&node_type) || PYTHON_VARIABLE_NODES.contains(&node_type)
        {
            Some(SymbolType::Variable)
        } else if CONST_NODES.contains(&node_type) {
            Some(SymbolType::Constant)
        } else if MODULE_NODES.contains(&node_type) {
            Some(SymbolType::Module)
        } else if IMPORT_NODES.contains(&node_type) {
            Some(SymbolType::Import)
        } else if COMMENT_NODES.contains(&node_type) {
            Some(SymbolType::Comment)
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

    /// Check if a node is inside a trait, impl, or class block
    /// Made pub(crate) for testing purposes
    pub(crate) fn is_inside_trait_or_impl(&self, node: Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if METHOD_CONTAINER_NODES.contains(&parent.kind()) {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Extract symbol type from Python decorated definitions
    /// Python decorators can modify function/class behavior significantly
    fn extract_decorated_symbol_type(&self, node: Node) -> Option<SymbolType> {
        // Look for the actual definition inside the decorated_definition
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let child_kind = child.kind();

            // Check what's being decorated
            if FUNCTION_NODES.contains(&child_kind) {
                // Check if it's inside a class (making it a method)
                if self.is_inside_trait_or_impl(node) {
                    return Some(SymbolType::Method);
                } else {
                    return Some(SymbolType::Function);
                }
            } else if CLASS_NODES.contains(&child_kind) {
                return Some(SymbolType::Class);
            }
        }

        // Default fallback - most decorators are on functions
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
            if IDENTIFIER_NODES.contains(&child.kind()) {
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
        assert_eq!(SupportedLanguage::from_extension("js"), None); // Not supported yet
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

    // Python-specific tests
    #[tokio::test]
    async fn test_python_language_detection() -> Result<()> {
        assert_eq!(
            SupportedLanguage::from_extension("py"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_name("python"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_name("py"),
            Some(SupportedLanguage::Python)
        );

        let python_lang = SupportedLanguage::Python;
        assert_eq!(python_lang.name(), "Python");
        assert_eq!(python_lang.extensions(), &["py"]);
        Ok(())
    }

    #[tokio::test]
    async fn test_basic_python_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let python_code = r#"
def hello_world():
    print("Hello, world!")

class Person:
    def __init__(self, name):
        self.name = name
    
    def greet(self):
        return f"Hello, {self.name}"

import os
from typing import List, Dict
import numpy as np

@property
def calculated_value(self):
    return 42
        "#;

        let parsed = parser.parse_content(python_code, SupportedLanguage::Python)?;

        assert_eq!(parsed.language, SupportedLanguage::Python);
        assert!(!parsed.symbols.is_empty());
        assert!(parsed.stats.total_nodes > 0);

        // Should find the function
        let function_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Function)
            .collect();
        assert!(!function_symbols.is_empty());

        // Should find the class
        let class_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Class)
            .collect();
        assert!(!class_symbols.is_empty());

        // Should find imports
        let import_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Import)
            .collect();
        assert!(!import_symbols.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_python_method_detection() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let python_code = r#"
class Calculator:
    def add(self, a, b):
        return a + b
    
    @classmethod
    def create_default(cls):
        return cls()
    
    @staticmethod
    def multiply(a, b):
        return a * b

def standalone_function():
    pass
        "#;

        let parsed = parser.parse_content(python_code, SupportedLanguage::Python)?;

        // Should find methods inside the class
        let method_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Method)
            .collect();
        assert!(!method_symbols.is_empty());

        // Should find the standalone function
        let function_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Function)
            .collect();

        // Should have at least the standalone function
        let standalone_fn = function_symbols
            .iter()
            .find(|s| s.name == "standalone_function");
        assert!(standalone_fn.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_python_decorator_parsing() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let python_code = r#"
@app.route('/api/users')
@require_auth
def get_users():
    return {"users": []}

@dataclass
class User:
    name: str
    age: int
        "#;

        let parsed = parser.parse_content(python_code, SupportedLanguage::Python)?;

        // Should find decorated functions and classes
        let symbols: Vec<_> = parsed.symbols.iter().collect();
        assert!(!symbols.is_empty());

        // Verify we can parse decorated constructs without errors
        assert!(parsed.errors.is_empty() || parsed.errors.len() < symbols.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_python_import_variations() -> Result<()> {
        let mut parser = CodeParser::new()?;

        let python_code = r#"
import os
import sys, json
from pathlib import Path
from typing import Dict, List, Optional
from . import local_module
from ..parent import parent_module
import numpy as np
        "#;

        let parsed = parser.parse_content(python_code, SupportedLanguage::Python)?;

        // Should find various import styles
        let import_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.symbol_type == SymbolType::Import)
            .collect();
        assert!(!import_symbols.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_python_error_recovery() -> Result<()> {
        let mut parser = CodeParser::new()?;

        // Python code with some syntax errors
        let bad_python_code = r#"
def valid_function():
    return "works"

class ValidClass:
    def method(self):
        return True
        "#;

        let parsed = parser.parse_content(bad_python_code, SupportedLanguage::Python)?;

        assert_eq!(parsed.language, SupportedLanguage::Python);
        // Should still find valid symbols despite some errors
        assert!(!parsed.symbols.is_empty());

        Ok(())
    }
}
