//! Demo of tree-sitter integration for parsing Rust code

use kotadb::parsing::{CodeParser, SupportedLanguage};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🌳 KotaDB Tree-sitter Demo");
    println!("==========================");

    // Create a parser
    let mut parser = CodeParser::new()?;

    // Sample Rust code to parse
    let sample_code = r#"
//! This is a demo module

use std::collections::HashMap;

/// A simple struct representing a person
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    age: u32,
}

impl Person {
    /// Create a new person
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
    
    /// Get the person's age
    pub fn age(&self) -> u32 {
        self.age
    }
    
    /// Crate-visible helper method
    pub(crate) fn internal_method(&self) -> bool {
        true
    }
    
    /// Private helper method
    fn validate_age(age: u32) -> bool {
        age > 0 && age < 150
    }
}

/// Main function demonstrating usage
fn main() {
    let people: HashMap<String, Person> = HashMap::new();
    println!("Created empty people map");
    
    let person = Person::new("Alice".to_string(), 30);
    println!("Created person: {:?}", person);
}

/// A simple enum
#[derive(Debug)]
enum Status {
    Active,
    Inactive,
    Pending(String),
}

/// Constants
const MAX_PEOPLE: usize = 1000;
static SYSTEM_NAME: &str = "PeopleManager";
"#;

    println!("📝 Parsing Rust code sample...");

    // Parse the code
    let parsed = parser.parse_content(sample_code, SupportedLanguage::Rust)?;

    println!("✅ Parsing completed!");
    println!();

    // Display parsing statistics
    println!("📊 Parse Statistics:");
    println!("  Language: {:?}", parsed.language);
    println!("  Total nodes: {}", parsed.stats.total_nodes);
    println!("  Named nodes: {}", parsed.stats.named_nodes);
    println!("  Max depth: {}", parsed.stats.max_depth);
    println!("  Error count: {}", parsed.stats.error_count);
    println!();

    // Display found symbols
    println!("🔍 Found {} symbols:", parsed.symbols.len());
    println!();

    for (i, symbol) in parsed.symbols.iter().enumerate() {
        println!(
            "  {}. {} ({:?}) - {:?}",
            i + 1,
            symbol.name,
            symbol.symbol_type,
            symbol.kind
        );
        println!(
            "     Location: lines {}-{}",
            symbol.start_line, symbol.end_line
        );
        if !symbol.text.is_empty() && symbol.text.len() < 100 {
            println!("     Text: {}", symbol.text.replace('\n', " ").trim());
        }
        println!();
    }

    // Show any parse errors
    if !parsed.errors.is_empty() {
        println!("⚠️  Parse Errors:");
        for error in &parsed.errors {
            println!("  - {}", error);
        }
        println!();
    }

    println!("🎉 Demo completed successfully!");

    // Wait for user input to see output
    print!("Press Enter to exit...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}
