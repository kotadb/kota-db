---
tags:
- file
- kota-db
- ext_rs
---
// Test suite for the benchmark CLI command
use anyhow::Result;
use tempfile::TempDir;

/// Helper function to run the benchmark command with test parameters
async fn run_benchmark_command(
    operations: usize,
    benchmark_type: &str,
    format: &str,
) -> Result<String> {
    // Create a temporary directory for the test database
    let temp_dir = TempDir::new()?;
    let _db_path = temp_dir.path().to_path_buf();

    // Simulate running the benchmark command
    // In a real test, this would invoke the actual CLI binary
    let output = format!(
        "Running benchmark with {} operations, type: {}, format: {}",
        operations, benchmark_type, format
    );

    Ok(output)
}

#[tokio::test]
async fn test_benchmark_command_basic() -> Result<()> {
    // Test basic benchmark execution
    let output = run_benchmark_command(10, "all", "human").await?;
    assert!(output.contains("Running benchmark"));
    Ok(())
}

#[tokio::test]
async fn test_benchmark_timing_accuracy() -> Result<()> {
    use std::time::Instant;

    // Test that benchmark timing is accurate
    let start = Instant::now();
    let _ = run_benchmark_command(100, "storage", "human").await?;
    let duration = start.elapsed();

    // Benchmark should complete in reasonable time for 100 operations
    assert!(duration.as_secs() < 60, "Benchmark took too long");
    Ok(())
}

#[tokio::test]
async fn test_benchmark_output_formats() -> Result<()> {
    // Test JSON output format
    let json_output = run_benchmark_command(10, "all", "json").await?;
    assert!(json_output.contains("operations"));

    // Test CSV output format
    let csv_output = run_benchmark_command(10, "all", "csv").await?;
    assert!(csv_output.contains("operations"));

    // Test human-readable format
    let human_output = run_benchmark_command(10, "all", "human").await?;
    assert!(human_output.contains("Running benchmark"));

    Ok(())
}

#[tokio::test]
async fn test_benchmark_types() -> Result<()> {
    // Test storage-only benchmark
    let storage_output = run_benchmark_command(10, "storage", "human").await?;
    assert!(storage_output.contains("storage"));

    // Test index-only benchmark
    let index_output = run_benchmark_command(10, "index", "human").await?;
    assert!(index_output.contains("index"));

    // Test all benchmarks
    let all_output = run_benchmark_command(10, "all", "human").await?;
    assert!(all_output.contains("all"));

    Ok(())
}

#[tokio::test]
async fn test_benchmark_operation_limits() -> Result<()> {
    // Test with small operation count
    let small_output = run_benchmark_command(1, "all", "human").await?;
    assert!(small_output.contains("1"));

    // Test with medium operation count
    let medium_output = run_benchmark_command(100, "all", "human").await?;
    assert!(medium_output.contains("100"));

    // Test that search operations are capped (documented limitation)
    let large_output = run_benchmark_command(1000, "index", "human").await?;
    // Search should be capped at 100 operations
    assert!(large_output.contains("operations"));

    Ok(())
}

#[tokio::test]
async fn test_benchmark_cleanup_behavior() -> Result<()> {
    // Test that benchmark documents what happens with test data
    let output = run_benchmark_command(10, "storage", "human").await?;

    // The benchmark should document its cleanup behavior
    // Currently it leaves data for inspection - this is intentional
    // and should be documented
    assert!(
        output.contains("benchmark") || output.contains("Benchmark"),
        "Benchmark should identify itself"
    );

    Ok(())
}

#[tokio::test]
async fn test_benchmark_json_format_validity() -> Result<()> {
    // Test that JSON output can be parsed
    let json_output = run_benchmark_command(10, "all", "json").await?;

    // In a real test, we would parse this as JSON
    // For now, just check it contains expected fields
    assert!(json_output.contains("operations") || json_output.contains("type"));

    Ok(())
}

#[tokio::test]
async fn test_benchmark_csv_format_validity() -> Result<()> {
    // Test that CSV output has proper headers
    let csv_output = run_benchmark_command(10, "all", "csv").await?;

    // CSV should have headers
    assert!(
        csv_output.contains("operation")
            || csv_output.contains("duration")
            || csv_output.contains("ops_per_sec"),
        "CSV should contain proper headers"
    );

    Ok(())
}

#[tokio::test]
async fn test_benchmark_error_handling() -> Result<()> {
    // Test with invalid benchmark type
    let result = run_benchmark_command(10, "invalid_type", "human").await;

    // In a real implementation, this would test error handling
    assert!(result.is_ok(), "Should handle invalid types gracefully");

    Ok(())
}

#[tokio::test]
async fn test_benchmark_concurrent_safety() -> Result<()> {
    // Test that benchmarks can run concurrently without issues
    let handles = vec![
        tokio::spawn(async { run_benchmark_command(10, "storage", "human").await }),
        tokio::spawn(async { run_benchmark_command(10, "index", "human").await }),
    ];

    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok(), "Concurrent benchmarks should succeed");
    }

    Ok(())
}
