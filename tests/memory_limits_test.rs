//! Tests for memory limits functionality in repository ingestion

use anyhow::Result;
use kotadb::git::types::IngestionOptions;
use kotadb::git::{IngestionConfig, RepositoryIngester};
use kotadb::memory::{MemoryLimitsConfig, MemoryManager};
use kotadb::{create_file_storage, Storage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;
use tracing::info;

/// Helper to create test storage with tracing
async fn create_test_storage() -> Result<(impl Storage, TempDir)> {
    let temp_dir = TempDir::new()?;
    let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;
    Ok((storage, temp_dir))
}

#[tokio::test]
async fn test_memory_manager_basic_functionality() -> Result<()> {
    // Test memory manager creation and basic operations
    let manager = MemoryManager::new(Some(50)); // 50MB limit

    // Test reservation within limits
    let reservation1 = manager.reserve(25 * 1024 * 1024)?; // 25MB
    assert_eq!(manager.get_stats().current_usage_mb, 25);
    assert!(!manager.is_memory_pressure());

    // Test memory pressure detection (>80% = 40MB+)
    let reservation2 = manager.reserve(20 * 1024 * 1024)?; // 20MB more = 45MB total
    assert_eq!(manager.get_stats().current_usage_mb, 45);
    assert!(manager.is_memory_pressure()); // 45/50 = 90% > 80%

    // Test exceeding limits
    let result = manager.reserve(10 * 1024 * 1024); // Would be 55MB > 50MB
    assert!(result.is_err());

    // Test cleanup on drop
    drop(reservation1);
    assert_eq!(manager.get_stats().current_usage_mb, 20);
    assert!(!manager.is_memory_pressure());

    drop(reservation2);
    assert_eq!(manager.get_stats().current_usage_mb, 0);

    Ok(())
}

#[tokio::test]
async fn test_memory_manager_disabled() -> Result<()> {
    // Test memory manager with no limits
    let manager = MemoryManager::new(None);

    // Should allow any allocation when disabled
    assert!(manager.can_allocate(u64::MAX));
    let _reservation = manager.reserve(1024 * 1024 * 1024)?; // 1GB should work
    assert!(!manager.is_memory_pressure());

    let stats = manager.get_stats();
    assert!(!stats.enabled);
    assert_eq!(stats.max_memory_mb, None);
    assert_eq!(stats.utilization_percent, None);

    Ok(())
}

#[tokio::test]
async fn test_memory_limits_config_presets() -> Result<()> {
    let prod = MemoryLimitsConfig::production();
    assert_eq!(prod.max_total_memory_mb, Some(1024));
    assert_eq!(prod.chunk_size, 50);
    assert!(prod.enable_adaptive_chunking);

    let dev = MemoryLimitsConfig::development();
    assert_eq!(dev.max_total_memory_mb, Some(512));
    assert_eq!(dev.chunk_size, 25);

    let test = MemoryLimitsConfig::testing();
    assert_eq!(test.max_total_memory_mb, Some(100));
    assert_eq!(test.chunk_size, 10);

    Ok(())
}

#[tokio::test]
async fn test_memory_aware_ingestion_chunking() -> Result<()> {
    let (mut storage, _temp_dir) = create_test_storage().await?;

    // Create a test Git repository in a temporary directory
    let repo_dir = TempDir::new()?;
    let repo_path = repo_dir.path();

    // Initialize git repo and add some test files
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()?;

    // Create test files
    for i in 0..20 {
        let file_content = format!(
            "// Test file {}\nfn test_function_{}() {{\n    println!(\"Hello, world!\");\n}}",
            i, i
        );
        std::fs::write(repo_path.join(format!("test_file_{}.rs", i)), file_content)?;
    }

    // Add files to git
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()?;

    std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "Initial commit",
        ])
        .current_dir(repo_path)
        .output()?;

    // Configure ingestion with memory limits
    let memory_limits = MemoryLimitsConfig {
        max_total_memory_mb: Some(5), // Very small limit to force chunking
        max_parallel_files: Some(2),
        enable_adaptive_chunking: true,
        chunk_size: 5, // Small chunks
    };

    let options = IngestionOptions {
        include_file_contents: true,
        include_commit_history: false,
        extract_symbols: false,
        memory_limits: Some(memory_limits),
        ..Default::default()
    };

    let config = IngestionConfig {
        path_prefix: "test".to_string(),
        options,
        create_index: false,
        organization_config: None,
    };

    let ingester = RepositoryIngester::new(config);

    // Track progress calls
    let progress_calls = Arc::new(AtomicUsize::new(0));
    let progress_calls_clone = progress_calls.clone();
    let progress_callback = Some(Box::new(move |msg: &str| {
        progress_calls_clone.fetch_add(1, Ordering::Relaxed);
        info!("Progress: {}", msg);
    }) as Box<dyn Fn(&str) + Send + Sync>);

    // Run ingestion with memory limits
    let result = ingester
        .ingest_with_progress(repo_path, &mut storage, progress_callback)
        .await?;

    // Verify results
    assert!(
        result.documents_created > 0,
        "Should have created some documents"
    );
    assert!(result.files_ingested > 0, "Should have ingested some files");
    assert_eq!(result.errors, 0, "Should have no errors");

    // Verify progress was reported
    assert!(
        progress_calls.load(Ordering::Relaxed) > 0,
        "Should have reported progress"
    );

    info!(
        "Ingestion complete: {} documents, {} files, {} errors",
        result.documents_created, result.files_ingested, result.errors
    );

    Ok(())
}

#[tokio::test]
async fn test_memory_estimation_accuracy() -> Result<()> {
    let manager = MemoryManager::new(Some(100));

    // Test file memory estimation
    let small_file_estimate = manager.estimate_file_memory(1024, false); // 1KB file, no parsing
    assert_eq!(small_file_estimate, 1024 + 200); // file size + overhead

    let large_file_estimate = manager.estimate_file_memory(10 * 1024, true); // 10KB file, with parsing
    assert_eq!(large_file_estimate, (10 * 1024 * 3) + 200); // 3x multiplier + overhead

    // Test that estimates are reasonable for memory reservation
    let estimate = manager.estimate_file_memory(5 * 1024, false); // 5KB
    let _reservation = manager.reserve(estimate)?;
    assert_eq!(manager.get_stats().current_usage_mb, 0); // Should round to 0MB for small sizes

    Ok(())
}

#[tokio::test]
async fn test_memory_limits_edge_cases() -> Result<()> {
    // Test with extremely low memory limits
    let manager = MemoryManager::new(Some(1)); // 1MB limit

    // Should be able to reserve very small amounts
    let _small_reservation = manager.reserve(500 * 1024)?; // 500KB

    // Should reject large reservations
    let result = manager.reserve(2 * 1024 * 1024); // 2MB
    assert!(result.is_err());

    // Test with zero limit
    let zero_manager = MemoryManager::new(Some(0));
    let result = zero_manager.reserve(1);
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_memory_reservations() -> Result<()> {
    let manager = Arc::new(MemoryManager::new(Some(100))); // 100MB

    // Test concurrent reservations
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let size = (i + 1) * 1024 * 1024; // 1MB, 2MB, ... 10MB
            manager_clone.reserve(size)
        });
        handles.push(handle);
    }

    // Wait for all reservations
    let mut successful = 0;
    let mut failed = 0;

    for handle in handles {
        match handle.await.unwrap() {
            Ok(_reservation) => {
                successful += 1;
                // Reservations are dropped when they go out of scope
            }
            Err(_) => failed += 1,
        }
    }

    // Some should succeed, some should fail due to memory limits
    assert!(successful > 0, "Some reservations should succeed");
    // Total requested would be 55MB, which is within 100MB limit
    assert!(successful >= 8, "Most small reservations should succeed");

    Ok(())
}

#[tokio::test]
async fn test_memory_stats_display() -> Result<()> {
    let manager = MemoryManager::new(Some(200)); // 200MB
    let _reservation = manager.reserve(60 * 1024 * 1024)?; // 60MB

    let stats = manager.get_stats();
    let display = format!("{}", stats);

    // Should show current usage, max, and percentage
    assert!(display.contains("60MB"));
    assert!(display.contains("200MB"));
    assert!(display.contains("30%")); // 60/200 = 30%

    let disabled_manager = MemoryManager::new(None);
    let disabled_stats = disabled_manager.get_stats();
    let disabled_display = format!("{}", disabled_stats);
    assert!(disabled_display.contains("disabled"));

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    async fn create_large_test_repo(
        repo_path: &Path,
        num_files: usize,
        file_size: usize,
    ) -> Result<()> {
        // Initialize git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()?;

        // Create files of specified size
        let content = "x".repeat(file_size);
        for i in 0..num_files {
            fs::write(repo_path.join(format!("file_{}.txt", i)), &content)?;
        }

        // Add and commit files
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;

        std::process::Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@test.com",
                "commit",
                "-m",
                "Test",
            ])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_large_repository_memory_limits() -> Result<()> {
        let (mut storage, _temp_dir) = create_test_storage().await?;
        let repo_dir = TempDir::new()?;
        let repo_path = repo_dir.path();

        // Create a repository with many small files
        create_large_test_repo(repo_path, 100, 1024).await?; // 100 files of 1KB each

        let memory_limits = MemoryLimitsConfig {
            max_total_memory_mb: Some(1), // 1MB limit
            max_parallel_files: Some(1),
            enable_adaptive_chunking: true,
            chunk_size: 10,
        };

        let options = IngestionOptions {
            include_file_contents: true,
            include_commit_history: false,
            extract_symbols: false,
            memory_limits: Some(memory_limits),
            ..Default::default()
        };

        let config = IngestionConfig {
            path_prefix: "large_test".to_string(),
            options,
            create_index: false,
            organization_config: None,
        };

        let ingester = RepositoryIngester::new(config);
        let result = ingester.ingest(repo_path, &mut storage).await?;

        // Should complete successfully despite memory limits
        assert!(result.files_ingested > 0);
        assert_eq!(result.errors, 0);

        Ok(())
    }
}
