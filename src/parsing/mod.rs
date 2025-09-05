//! Multi-language code parsing for KotaDB using tree-sitter
//!
//! This module provides sophisticated code parsing capabilities to extract
//! structural information from source code files, enabling advanced codebase
//! analysis features like symbol extraction, dependency mapping, and
//! intelligent code queries.

#[cfg(feature = "tree-sitter-parsing")]
mod tree_sitter;

#[cfg(feature = "tree-sitter-parsing")]
pub use tree_sitter::{
    CodeParser, ParseStats, ParsedCode, ParsedSymbol, ParsingConfig, SupportedLanguage, SymbolKind,
    SymbolType,
};

#[cfg(not(feature = "tree-sitter-parsing"))]
pub mod stub {
    //! Stub implementations when tree-sitter parsing is not enabled
    use anyhow::{anyhow, Result};

    pub struct CodeParser;

    #[derive(Debug, Clone)]
    pub struct ParsedCode {
        pub language: SupportedLanguage,
        pub stats: ParsedStats,
        pub symbols: Vec<ParsedSymbol>,
        pub errors: Vec<ParseError>,
    }

    #[derive(Debug, Clone)]
    pub struct ParsedStats {
        pub total_nodes: usize,
        pub named_nodes: usize,
        pub max_depth: usize,
        pub error_count: usize,
    }

    #[derive(Debug, Clone)]
    pub struct ParsedSymbol {
        pub name: String,
        pub kind: String,
        pub line: usize,
        pub column: usize,
    }

    #[derive(Debug, Clone)]
    pub struct ParseError {
        pub message: String,
        pub line: usize,
        pub column: usize,
    }

    #[derive(Debug, Clone)]
    pub enum SupportedLanguage {
        Rust,
        Python,
    }

    impl CodeParser {
        pub fn new() -> Result<Self> {
            Err(anyhow!(
                "Tree-sitter parsing not enabled. Enable the 'tree-sitter-parsing' feature."
            ))
        }

        pub fn parse_content(
            &self,
            _content: &str,
            _language: SupportedLanguage,
        ) -> Result<ParsedCode> {
            Err(anyhow!(
                "Tree-sitter parsing not enabled. Enable the 'tree-sitter-parsing' feature."
            ))
        }
    }
}

#[cfg(not(feature = "tree-sitter-parsing"))]
pub use stub::*;

#[cfg(test)]
mod tests {
    use anyhow::Result;

    #[tokio::test]
    async fn test_parsing_module_imports() -> Result<()> {
        // Basic test to ensure module structure is correct
        #[cfg(feature = "tree-sitter-parsing")]
        {
            use crate::parsing::SupportedLanguage;
            let _rust_lang = SupportedLanguage::Rust;
        }
        Ok(())
    }

    #[cfg(feature = "tree-sitter-parsing")]
    #[test]
    fn test_symbol_extraction_basic_rust_code() -> Result<()> {
        use crate::parsing::{CodeParser, SupportedLanguage};

        let rust_code = r#"
fn hello() {
    println!("Hello, world!");
}

struct Test {
    field: i32,
}
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        assert!(
            !parsed.symbols.is_empty(),
            "Should find at least some symbols"
        );

        // Should find function and struct
        let function_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.name == "hello")
            .collect();
        let struct_symbols: Vec<_> = parsed.symbols.iter().filter(|s| s.name == "Test").collect();

        assert!(!function_symbols.is_empty(), "Should find 'hello' function");
        assert!(!struct_symbols.is_empty(), "Should find 'Test' struct");

        Ok(())
    }

    #[cfg(feature = "tree-sitter-parsing")]
    #[test]
    fn test_symbol_extraction_empty_code() -> Result<()> {
        use crate::parsing::{CodeParser, SupportedLanguage};

        let empty_code = "";

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(empty_code, SupportedLanguage::Rust)?;

        // Empty code should still parse successfully but with no symbols
        assert_eq!(parsed.symbols.len(), 0, "Empty code should have no symbols");
        assert_eq!(
            parsed.stats.total_nodes, 1,
            "Empty code should have root node only"
        );

        Ok(())
    }

    #[cfg(feature = "tree-sitter-parsing")]
    #[test]
    fn test_symbol_extraction_malformed_code() -> Result<()> {
        use crate::parsing::{CodeParser, SupportedLanguage};

        let malformed_code = r#"
fn incomplete_function( {
    // Missing closing parenthesis and brace
struct IncompleteStruct
    // Missing braces
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(malformed_code, SupportedLanguage::Rust)?;

        // Malformed code should still parse but may have errors
        assert!(
            parsed.stats.error_count > 0 || !parsed.errors.is_empty(),
            "Malformed code should have parsing errors"
        );

        Ok(())
    }

    #[cfg(feature = "tree-sitter-parsing")]
    #[test]
    fn test_symbol_extraction_different_languages() -> Result<()> {
        use crate::parsing::{CodeParser, SupportedLanguage};

        let python_code = r#"
def hello():
    print("Hello, world!")

class Test:
    def __init__(self):
        self.field = 42
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(python_code, SupportedLanguage::Python)?;

        assert!(
            !parsed.symbols.is_empty(),
            "Should find symbols in Python code"
        );

        // Should find function and class
        let function_symbols: Vec<_> = parsed
            .symbols
            .iter()
            .filter(|s| s.name == "hello")
            .collect();
        let class_symbols: Vec<_> = parsed.symbols.iter().filter(|s| s.name == "Test").collect();

        assert!(!function_symbols.is_empty(), "Should find 'hello' function");
        assert!(!class_symbols.is_empty(), "Should find 'Test' class");

        Ok(())
    }

    #[cfg(feature = "tree-sitter-parsing")]
    #[test]
    fn test_symbol_name_validation() -> Result<()> {
        use crate::parsing::{CodeParser, SupportedLanguage};

        let rust_code = r#"
fn valid_function_name() {}
fn _private_function() {}
fn function2() {}
fn snake_case_function() {}
struct ValidStruct {}
struct _PrivateStruct {}
"#;

        let mut parser = CodeParser::new()?;
        let parsed = parser.parse_content(rust_code, SupportedLanguage::Rust)?;

        // All symbols should have valid names
        for symbol in &parsed.symbols {
            assert!(!symbol.name.is_empty(), "Symbol name should not be empty");
            assert!(
                !symbol.name.contains(' '),
                "Symbol name should not contain spaces"
            );

            // Check for reasonable symbol names
            let valid_chars = symbol.name.chars().all(|c| c.is_alphanumeric() || c == '_');
            assert!(
                valid_chars,
                "Symbol name '{}' should only contain alphanumeric chars and underscores",
                symbol.name
            );
        }

        Ok(())
    }
}
