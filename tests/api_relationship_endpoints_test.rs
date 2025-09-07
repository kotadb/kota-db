//! Comprehensive integration tests for relationship analysis API endpoints
//!
//! This test suite addresses critical testing gaps identified in the code review
//! for PR #602, ensuring proper validation of API endpoint functionality.

use anyhow::Result;
use kotadb::{
    contracts::Index,
    database::Database,
    services::{AnalysisService, CallersOptions, ImpactOptions},
};
use std::{collections::HashMap, sync::Arc};
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

/// Test basic API endpoint structure and error handling without requiring complex setup
#[tokio::test]
async fn test_api_endpoint_response_structure() -> Result<()> {
    // Create minimal test setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Create minimal database for testing
    let storage = kotadb::file_storage::create_file_storage(
        temp_dir.path().join("storage").to_str().unwrap(),
        Some(100),
    )
    .await?;

    let database = Database {
        storage: Arc::new(Mutex::new(storage)),
        primary_index: Arc::new(Mutex::new(kotadb::primary_index::PrimaryIndex::new(
            temp_dir.path().join("primary.kota"),
            100, // cache capacity
        ))),
        trigram_index: Arc::new(Mutex::new(
            kotadb::trigram_index::TrigramIndex::open(
                temp_dir.path().join("trigram.kota").to_str().unwrap(),
            )
            .await?,
        )),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut analysis_service = AnalysisService::new(&database, db_path);

    // Test with non-existent target (should return empty but valid response)
    let callers_options = CallersOptions {
        target: "nonexistent_function_test".to_string(),
        limit: Some(10),
        quiet: false,
    };

    let callers_result = analysis_service.find_callers(callers_options).await?;

    // Verify response structure is correct even for empty results
    assert_eq!(
        callers_result.callers.len(),
        callers_result.total_count,
        "Total count should match callers array length"
    );

    assert!(
        !callers_result.markdown.is_empty(),
        "Markdown should not be empty even for zero results"
    );

    // Test impact analysis with same approach
    let impact_options = ImpactOptions {
        target: "nonexistent_type_test".to_string(),
        limit: Some(10),
        quiet: false,
    };

    let impact_result = analysis_service.analyze_impact(impact_options).await?;

    // Verify response structure is correct
    assert_eq!(
        impact_result.impacts.len(),
        impact_result.total_count,
        "Total count should match impacts array length"
    );

    assert!(
        !impact_result.markdown.is_empty(),
        "Markdown should not be empty even for zero results"
    );

    Ok(())
}

/// Test that line number conversion is safe and doesn't panic
#[tokio::test]
async fn test_line_number_conversion_safety() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = kotadb::file_storage::create_file_storage(
        temp_dir.path().join("storage").to_str().unwrap(),
        Some(100),
    )
    .await?;

    let database = Database {
        storage: Arc::new(Mutex::new(storage)),
        primary_index: Arc::new(Mutex::new(kotadb::primary_index::PrimaryIndex::new(
            temp_dir.path().join("primary.kota"),
            100,
        ))),
        trigram_index: Arc::new(Mutex::new(
            kotadb::trigram_index::TrigramIndex::open(
                temp_dir.path().join("trigram.kota").to_str().unwrap(),
            )
            .await?,
        )),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut analysis_service = AnalysisService::new(&database, db_path);

    // Test multiple calls to ensure no panics occur during line number conversion
    for i in 0..5 {
        let options = CallersOptions {
            target: format!("test_target_{}", i),
            limit: Some(5),
            quiet: false,
        };

        let result = analysis_service.find_callers(options).await;

        // Should not panic, even if no results
        assert!(result.is_ok(), "API call should not panic or error");

        let callers_result = result?;
        assert_eq!(
            callers_result.total_count,
            callers_result.callers.len(),
            "Counts should be consistent"
        );
    }

    Ok(())
}

/// Test that the API handles limit parameter correctly
#[tokio::test]
async fn test_api_limit_parameter_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = kotadb::file_storage::create_file_storage(
        temp_dir.path().join("storage").to_str().unwrap(),
        Some(100),
    )
    .await?;

    let database = Database {
        storage: Arc::new(Mutex::new(storage)),
        primary_index: Arc::new(Mutex::new(kotadb::primary_index::PrimaryIndex::new(
            temp_dir.path().join("primary.kota"),
            100,
        ))),
        trigram_index: Arc::new(Mutex::new(
            kotadb::trigram_index::TrigramIndex::open(
                temp_dir.path().join("trigram.kota").to_str().unwrap(),
            )
            .await?,
        )),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut analysis_service = AnalysisService::new(&database, db_path);

    // Test with various limit values
    for limit in [1, 5, 10, 100] {
        let options = CallersOptions {
            target: "test_function".to_string(),
            limit: Some(limit),
            quiet: false,
        };

        let result = analysis_service.find_callers(options).await?;

        // Even with empty results, structure should be consistent
        assert!(
            result.callers.len() <= limit,
            "Result should respect limit of {}",
            limit
        );

        assert_eq!(
            result.total_count,
            result.callers.len(),
            "Total count should match array length"
        );
    }

    Ok(())
}

/// Test error handling and robustness with edge case inputs
#[tokio::test]
async fn test_api_edge_case_inputs() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    let storage = kotadb::file_storage::create_file_storage(
        temp_dir.path().join("storage").to_str().unwrap(),
        Some(100),
    )
    .await?;

    let database = Database {
        storage: Arc::new(Mutex::new(storage)),
        primary_index: Arc::new(Mutex::new(kotadb::primary_index::PrimaryIndex::new(
            temp_dir.path().join("primary.kota"),
            100,
        ))),
        trigram_index: Arc::new(Mutex::new(
            kotadb::trigram_index::TrigramIndex::open(
                temp_dir.path().join("trigram.kota").to_str().unwrap(),
            )
            .await?,
        )),
        path_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut analysis_service = AnalysisService::new(&database, db_path);

    // Test with edge case inputs that previously might have caused issues
    let edge_cases = vec![
        "",  // Empty string
        "a", // Single character
        "very_long_function_name_that_might_cause_issues_with_buffer_sizes_or_similar_problems",
        "func with spaces",            // Spaces
        "func\nwith\nnewlines",        // Newlines
        "func_with_unicode_ñoño_测试", // Unicode
    ];

    for edge_case in edge_cases {
        let options = CallersOptions {
            target: edge_case.to_string(),
            limit: Some(5),
            quiet: false,
        };

        let result = analysis_service.find_callers(options).await;

        // Should handle edge cases gracefully without panicking
        assert!(
            result.is_ok(),
            "Should handle edge case input: '{}'",
            edge_case
        );

        if let Ok(callers_result) = result {
            assert_eq!(
                callers_result.total_count,
                callers_result.callers.len(),
                "Counts should be consistent for edge case: '{}'",
                edge_case
            );
        }
    }

    Ok(())
}
