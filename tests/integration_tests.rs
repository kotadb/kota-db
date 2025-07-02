// Integration Tests - Stage 1: TDD
// These tests verify the full system working together
// Written BEFORE implementation following 6-stage risk reduction

use anyhow::Result;
use kotadb::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

// Test data setup
fn create_test_markdown_files(dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let files = vec![
        (
            "projects/kota-ai/README.md",
            r#"---
title: KOTA AI Project
tags: [ai, rust, distributed-cognition]
related: ["architecture.md", "roadmap.md"]
created: 2024-01-01
updated: 2024-06-01
---

# KOTA AI Project

Knowledge-Oriented Thinking Assistant - A distributed cognition system.

## Overview

KOTA represents a new paradigm in human-AI collaboration.
"#,
        ),
        (
            "personal/notes/consciousness.md",
            r#"---
title: Consciousness Research Notes
tags: [philosophy, consciousness, ai]
related: ["../books/godel-escher-bach.md"]
created: 2024-03-15
updated: 2024-03-20
---

# Consciousness Research

Exploring the nature of consciousness in artificial systems.

## Key Questions

What does it mean for an AI system to be conscious?
"#,
        ),
        (
            "businesses/cogzia/meetings/2024-06-01.md",
            r#"---
title: Cogzia Strategy Meeting
tags: [meeting, cogzia, strategy]
participants: [jaymin, greg]
created: 2024-06-01
---

# Cogzia Strategy Meeting

Discussed the roadmap for Q3 2024.

## Action Items
- Review architecture proposal
- Schedule follow-up with team
"#,
        ),
    ];
    
    let mut created_files = Vec::new();
    
    for (path, content) in files {
        let full_path = dir.join(path);
        fs::create_dir_all(full_path.parent().unwrap())?;
        fs::write(&full_path, content)?;
        created_files.push((full_path, content.to_string()));
    }
    
    Ok(created_files)
}

#[tokio::test]
async fn test_full_indexing_workflow() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    // Create test markdown files
    let files = create_test_markdown_files(&kb_path)?;
    
    // Initialize database
    let db = Database::new(&db_path).await?;
    
    // Index all files
    let start = Instant::now();
    let indexed_count = db.index_directory(&kb_path).await?;
    let index_time = start.elapsed();
    
    // Verify all files were indexed
    assert_eq!(indexed_count, files.len());
    
    // Indexing should be fast
    assert!(index_time < Duration::from_secs(1), "Indexing took too long: {:?}", index_time);
    
    // Verify we can search immediately
    let results = db.search("KOTA").await?;
    assert!(!results.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    create_test_markdown_files(&kb_path)?;
    
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    
    // Test 1: Simple text search
    let results = db.search("consciousness").await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("consciousness.md"));
    
    // Test 2: Tag search
    let results = db.search_by_tags(&["meeting"]).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("2024-06-01.md"));
    
    // Test 3: Combined search (text + tags)
    let results = db.search_advanced(Query {
        text: Some("AI".to_string()),
        tags: Some(vec!["rust".to_string()]),
        date_range: None,
    }).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("README.md"));
    
    // Test 4: Relationship traversal
    let doc = db.get_by_path("projects/kota-ai/README.md").await?.unwrap();
    let related = db.get_related(&doc.id, 1).await?;
    assert_eq!(related.len(), 2); // architecture.md and roadmap.md
    
    Ok(())
}

#[tokio::test]
async fn test_file_watcher_integration() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    create_test_markdown_files(&kb_path)?;
    
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    
    // Start file watcher
    db.start_file_watcher(&kb_path).await?;
    
    // Add a new file
    let new_file = kb_path.join("test/new_file.md");
    fs::create_dir_all(new_file.parent().unwrap())?;
    fs::write(&new_file, r#"---
title: New Test File
tags: [test, watcher]
---

# New Test File

This file was added while the watcher was running.
"#)?;
    
    // Give watcher time to process (with debouncing)
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify the new file is indexed
    let results = db.search("watcher").await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("new_file.md"));
    
    // Modify an existing file
    let existing = kb_path.join("projects/kota-ai/README.md");
    let content = fs::read_to_string(&existing)?;
    fs::write(&existing, format!("{}\n\n## Update\n\nThis section was added by the test.", content))?;
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify the update is indexed
    let results = db.search("section was added").await?;
    assert_eq!(results.len(), 1);
    
    // Delete a file
    fs::remove_file(&new_file)?;
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify the file is removed from index
    let results = db.search("watcher").await?;
    assert_eq!(results.len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_startup_performance() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    // Create many files to test performance
    for i in 0..100 {
        let path = kb_path.join(format!("notes/note_{}.md", i));
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, format!(r#"---
title: Note {}
tags: [note, test]
---

# Note {}

This is test note number {}.
"#, i, i, i))?;
    }
    
    // Initial indexing
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    db.close().await?;
    
    // Measure startup time with existing indices
    let start = Instant::now();
    let db = Database::new(&db_path).await?;
    let startup_time = start.elapsed();
    
    // Should start up in under 1 second
    assert!(startup_time < Duration::from_secs(1), "Startup took too long: {:?}", startup_time);
    
    // Should be immediately searchable
    let results = db.search("note number 42").await?;
    assert_eq!(results.len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_search_performance() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    create_test_markdown_files(&kb_path)?;
    
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    
    // Measure search latency
    let queries = vec![
        "consciousness",
        "KOTA AI",
        "meeting strategy",
        "distributed cognition",
        "rust programming",
    ];
    
    for query in queries {
        let start = Instant::now();
        let _ = db.search(query).await?;
        let latency = start.elapsed();
        
        // Each search should complete in under 10ms
        assert!(latency < Duration::from_millis(10), 
            "Search '{}' took too long: {:?}", query, latency);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_frontmatter_parsing() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    // Create file with complex frontmatter
    let complex_file = kb_path.join("complex.md");
    fs::create_dir_all(complex_file.parent().unwrap())?;
    fs::write(&complex_file, r#"---
title: Complex Document
tags: [ai, rust, philosophy, quantum]
related: 
  - ../other/doc1.md
  - ../other/doc2.md
  - https://example.com/external
participants:
  - name: Alice
    role: Lead
  - name: Bob
    role: Contributor
metadata:
  version: 1.2.3
  draft: false
  priority: high
created: 2024-01-01T10:00:00Z
updated: 2024-06-15T15:30:00Z
---

# Complex Document

This document has complex frontmatter that should be fully parsed and indexed.
"#)?;
    
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    
    // Verify all tags are indexed
    for tag in &["ai", "rust", "philosophy", "quantum"] {
        let results = db.search_by_tags(&[tag]).await?;
        assert_eq!(results.len(), 1);
    }
    
    // Verify document metadata is preserved
    let doc = db.get_by_path("complex.md").await?.unwrap();
    assert_eq!(doc.title, "Complex Document");
    assert_eq!(doc.related.len(), 3);
    
    Ok(())
}

#[tokio::test]
async fn test_cli_integration() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    create_test_markdown_files(&kb_path)?;
    
    // Test index command
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "index", &kb_path.to_string_lossy()])
        .env("KOTADB_PATH", &db_path)
        .output()?;
    
    assert!(output.status.success());
    
    // Test search command
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "search", "KOTA"])
        .env("KOTADB_PATH", &db_path)
        .output()?;
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("README.md"));
    
    // Test stats command
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "stats"])
        .env("KOTADB_PATH", &db_path)
        .output()?;
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("document_count"));
    
    Ok(())
}

#[tokio::test]
async fn test_memory_usage() -> Result<()> {
    init_logging()?;
    let temp_dir = TempDir::new()?;
    let kb_path = temp_dir.path().join("knowledge_base");
    let db_path = temp_dir.path().join("database");
    
    // Create 1000 documents
    for i in 0..1000 {
        let path = kb_path.join(format!("docs/doc_{}.md", i));
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, format!(r#"---
title: Document {}
tags: [test, bulk]
---

# Document {}

This is document number {} in our bulk test.
Lorem ipsum dolor sit amet, consectetur adipiscing elit.
"#, i, i, i))?;
    }
    
    let db = Database::new(&db_path).await?;
    db.index_directory(&kb_path).await?;
    
    let metrics = db.get_metrics().await?;
    
    // Memory usage should be under 100MB for 1000 documents
    assert!(metrics.memory_usage_mb < 100.0, 
        "Memory usage too high: {}MB", metrics.memory_usage_mb);
    
    Ok(())
}

// Helper type that will be implemented later
struct Database;

impl Database {
    async fn new(_path: &str) -> Result<Self> { todo!() }
    async fn index_directory(&self, _path: &Path) -> Result<usize> { todo!() }
    async fn search(&self, _query: &str) -> Result<Vec<SearchResult>> { todo!() }
    async fn search_by_tags(&self, _tags: &[&str]) -> Result<Vec<SearchResult>> { todo!() }
    async fn search_advanced(&self, _query: Query) -> Result<Vec<SearchResult>> { todo!() }
    async fn get_by_path(&self, _path: &str) -> Result<Option<Document>> { todo!() }
    async fn get_related(&self, _id: &Uuid, _depth: u32) -> Result<Vec<Document>> { todo!() }
    async fn start_file_watcher(&self, _path: &Path) -> Result<()> { todo!() }
    async fn close(self) -> Result<()> { todo!() }
    async fn get_metrics(&self) -> Result<DatabaseMetrics> { todo!() }
}

#[derive(Debug)]
struct SearchResult {
    path: String,
    score: f32,
}

#[derive(Debug)]
struct Document {
    id: Uuid,
    path: String,
    title: String,
    related: Vec<String>,
}

#[derive(Debug)]
struct Query {
    text: Option<String>,
    tags: Option<Vec<String>>,
    date_range: Option<(String, String)>,
}

#[derive(Debug)]
struct DatabaseMetrics {
    document_count: usize,
    memory_usage_mb: f64,
}