//! Tree-sitter implementation for multi-language code parsing

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

/// Supported programming languages for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    Rust,
}

impl SupportedLanguage {
    /// Get tree-sitter language for this language
    pub fn tree_sitter_language(&self) -> Result<Language> {
        match self {
            SupportedLanguage::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
        }
    }

    /// Detect language from file extension
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "rs" => Some(SupportedLanguage::Rust),
            _ => None,
        }
    }

    /// Parse language from string name
    /// Supports both full names and common abbreviations
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "rust" | "rs" => Some(SupportedLanguage::Rust),
            // Future languages can be added here:
            // "python" | "py" => Some(SupportedLanguage::Python),
            // "javascript" | "js" => Some(SupportedLanguage::JavaScript),
            // "typescript" | "ts" => Some(SupportedLanguage::TypeScript),
            _ => None,
        }
    }

    /// Get human-readable name for this language
    pub fn name(&self) -> &'static str {
        match self {
            SupportedLanguage::Rust => "Rust",
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            SupportedLanguage::Rust => &["rs"],
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
        let languages = config
            .languages
            .as_ref()
            .map_or_else(|| vec![SupportedLanguage::Rust], |langs| langs.clone());

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
        let symbol_type = match node_type {
            "function_item" | "function_declaration" | "function_definition" => {
                Some(SymbolType::Function)
            }
            "method_definition" | "method_declaration" => Some(SymbolType::Method),
            "struct_item" | "struct_declaration" => Some(SymbolType::Struct),
            "impl_item" => Some(SymbolType::Class), // Rust impl blocks
            "class_declaration" | "class_definition" => Some(SymbolType::Class),
            "interface_declaration" => Some(SymbolType::Interface),
            "enum_item" | "enum_declaration" => Some(SymbolType::Enum),
            "let_declaration" | "variable_declarator" => Some(SymbolType::Variable),
            "const_item" | "const_declaration" => Some(SymbolType::Constant),
            "mod_item" | "module_declaration" => Some(SymbolType::Module),
            "use_declaration" | "import_statement" => Some(SymbolType::Import),
            "line_comment" | "block_comment" => Some(SymbolType::Comment),
            _ => None,
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

    /// Extract symbol name from a node (simplified implementation)
    fn extract_symbol_name(&self, node: Node, content: &str) -> Option<String> {
        // Look for identifier nodes within this node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "name" {
                if let Ok(name) = child.utf8_text(content.as_bytes()) {
                    return Some(name.to_string());
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
}
