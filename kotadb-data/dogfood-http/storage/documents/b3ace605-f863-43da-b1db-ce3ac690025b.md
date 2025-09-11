---
tags:
- file
- kota-db
- ext_rs
---
use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

mod git_test_helpers;
use git_test_helpers::TestGitRepository;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Debug: Creating test git repository...");
    let repo = TestGitRepository::new_with_extensive_symbols().await?;
    println!("✅ Git repository created at: {}", repo.path_ref().display());

    // Check git status
    let git_output = Command::new("git")
        .current_dir(repo.path_ref())
        .args(["log", "--oneline", "-5"])
        .output()?;
    
    println!("📝 Git log:");
    println!("{}", String::from_utf8_lossy(&git_output.stdout));

    // List files to see what we created
    println!("📁 Files in repository:");
    let files_output = Command::new("find")
        .current_dir(repo.path_ref())
        .args([".", "-name", "*.rs"])
        .output()?;
    println!("{}", String::from_utf8_lossy(&files_output.stdout));

    // Try manual indexing with debug output
    let db_temp_dir = TempDir::new()?;
    let db_path = db_temp_dir.path().join("test_db");
    
    println!("🔍 Debug: Indexing repository...");
    let index_output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "kotadb",
            "--",
            "-d",
            &db_path.to_string_lossy(),
            "index-codebase",
            "--symbols",
        ])
        .arg(repo.path_ref())
        .output()?;

    println!("📊 Index command status: {:?}", index_output.status);
    println!("📤 Index stdout:\n{}", String::from_utf8_lossy(&index_output.stdout));
    println!("📥 Index stderr:\n{}", String::from_utf8_lossy(&index_output.stderr));

    if index_output.status.success() {
        println!("✅ Indexing successful! Now testing find-callers...");
        
        // Test find-callers
        let find_output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "kotadb",
                "--",
                "-d",
                &db_path.to_string_lossy(),
                "--quiet",
                "find-callers",
                "FileStorage",
            ])
            .output()?;

        println!("📊 Find-callers status: {:?}", find_output.status);
        println!("📤 Find-callers stdout:\n{}", String::from_utf8_lossy(&find_output.stdout));
        println!("📥 Find-callers stderr:\n{}", String::from_utf8_lossy(&find_output.stderr));
        
        let result_count = String::from_utf8_lossy(&find_output.stdout).lines().count();
        println!("📈 Result count: {}", result_count);
    } else {
        println!("❌ Indexing failed!");
        return Err(anyhow::anyhow!("Indexing failed"));
    }

    Ok(())
}