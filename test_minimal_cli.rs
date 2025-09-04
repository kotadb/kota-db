// Minimal CLI test based on working binary_symbols_integration_test.rs pattern
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn create_test_files(repo_path: &Path) -> Result<()> {
    // Create the exact same code as the working integration test
    let rust_content = r#"
pub struct FileStorage {
    field1: String,
    field2: i32,
}

impl FileStorage {
    pub fn new() -> Self {
        FileStorage {
            field1: String::new(),
            field2: 0,
        }
    }
    
    pub fn method1(&self) -> &str {
        &self.field1
    }
}

pub fn standalone_function(x: i32) -> i32 {
    x * 2
}

pub enum TestEnum {
    Variant1,
    Variant2(String),
}

pub const TEST_CONSTANT: i32 = 42;
"#;

    std::fs::write(repo_path.join("test.rs"), rust_content)?;

    // Create lib.rs
    let lib_content = r#"
mod test;

pub fn library_function() {
    println!("Library function");
}
"#;

    std::fs::write(repo_path.join("lib.rs"), lib_content)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Creating minimal test repository...");

    // Create test repository structure - exact same as working test
    let temp_dir = TempDir::new()?;
    let test_repo = temp_dir.path().join("test_repo");
    std::fs::create_dir_all(&test_repo)?;

    // Initialize git repo - exact same as working test
    let output = Command::new("git")
        .args(["init"])
        .current_dir(&test_repo)
        .output()?;
    println!("Git init status: {:?}", output.status);

    // Set git user (added this as it was missing and might be important)
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&test_repo)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&test_repo)
        .output()?;

    // Create test files - exact same as working test
    create_test_files(&test_repo)?;
    println!("Created test files");

    // List files created
    let find_output = Command::new("find")
        .arg(&test_repo)
        .args(["-name", "*.rs"])
        .output()?;
    println!(
        "Rust files created:\n{}",
        String::from_utf8_lossy(&find_output.stdout)
    );

    // Commit files - exact same as working test
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(&test_repo)
        .output()?;
    println!("Git add status: {:?}", output.status);

    let output = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&test_repo)
        .output()?;
    println!("Git commit status: {:?}", output.status);

    // Now try indexing with KotaDB CLI
    let db_dir = temp_dir.path().join("db");
    let db_path = db_dir.to_str().unwrap();

    println!("Indexing repository with KotaDB CLI...");
    let index_output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            db_path,
            "index-codebase",
        ])
        .arg(&test_repo)
        .output()?;

    println!("Indexing status: {:?}", index_output.status);
    println!(
        "Indexing stdout:\n{}",
        String::from_utf8_lossy(&index_output.stdout)
    );
    println!(
        "Indexing stderr:\n{}",
        String::from_utf8_lossy(&index_output.stderr)
    );

    // Check database stats
    println!("Checking database stats...");
    let stats_output = Command::new("cargo")
        .args(["run", "--bin", "kotadb", "--", "-d", db_path, "stats"])
        .output()?;

    println!("Stats status: {:?}", stats_output.status);
    println!(
        "Stats stdout:\n{}",
        String::from_utf8_lossy(&stats_output.stdout)
    );
    println!(
        "Stats stderr:\n{}",
        String::from_utf8_lossy(&stats_output.stderr)
    );

    // Try find-callers
    println!("Testing find-callers...");
    let find_output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            db_path,
            "--quiet",
            "find-callers",
            "FileStorage",
        ])
        .output()?;

    println!("Find-callers status: {:?}", find_output.status);
    println!(
        "Find-callers stdout:\n{}",
        String::from_utf8_lossy(&find_output.stdout)
    );
    println!(
        "Find-callers stderr:\n{}",
        String::from_utf8_lossy(&find_output.stderr)
    );

    Ok(())
}
