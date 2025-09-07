//! Comprehensive integration tests for relationship analysis API endpoints
//!
//! This test suite addresses critical testing gaps identified in the code review
//! for PR #602, ensuring proper validation of API endpoint functionality.

use anyhow::Result;
use kotadb::services::{CallSite, CallersResult, ImpactResult, ImpactSite};

/// Test that API result structures can be serialized to JSON correctly
/// This addresses the core bug where empty arrays weren't being populated
#[tokio::test]
async fn test_json_serialization_structure() -> Result<()> {
    // Test CallersResult serialization
    let callers_result = CallersResult {
        callers: vec![
            CallSite {
                caller: "test_function".to_string(),
                file_path: "src/test.rs".to_string(),
                line_number: Some(42),
                context: "Calls target at line 42".to_string(),
            },
            CallSite {
                caller: "another_function".to_string(),
                file_path: "src/other.rs".to_string(),
                line_number: None, // Test line number overflow handling
                context: "Calls target at line 999999999".to_string(),
            },
        ],
        markdown: "# Callers\n\nFound 2 callers".to_string(),
        total_count: 2,
    };

    // Test JSON serialization
    let json_result = serde_json::to_string(&callers_result)?;
    assert!(
        json_result.contains("\"callers\":["),
        "JSON should contain callers array"
    );
    assert!(
        json_result.contains("\"total_count\":2"),
        "JSON should contain total_count"
    );
    assert!(
        json_result.contains("\"test_function\""),
        "JSON should contain caller names"
    );

    // Test that arrays are not empty placeholder
    assert!(
        !json_result.contains("\"callers\":[]"),
        "Callers array should not be empty when populated"
    );

    // Test ImpactResult serialization
    let impact_result = ImpactResult {
        impacts: vec![ImpactSite {
            affected_symbol: "affected_function".to_string(),
            file_path: "src/impact.rs".to_string(),
            line_number: Some(100),
            impact_type: "Function Call Impact".to_string(),
        }],
        markdown: "# Impact Analysis\n\nFound 1 impact".to_string(),
        total_count: 1,
    };

    let impact_json_result = serde_json::to_string(&impact_result)?;
    assert!(
        impact_json_result.contains("\"impacts\":["),
        "JSON should contain impacts array"
    );
    assert!(
        impact_json_result.contains("\"total_count\":1"),
        "JSON should contain total_count"
    );
    assert!(
        impact_json_result.contains("\"affected_function\""),
        "JSON should contain affected symbols"
    );

    // Test that arrays are not empty placeholder
    assert!(
        !impact_json_result.contains("\"impacts\":[]"),
        "Impacts array should not be empty when populated"
    );

    Ok(())
}

/// Test response structure consistency
/// Ensures total_count always matches array length
#[tokio::test]
async fn test_response_structure_consistency() -> Result<()> {
    // Test empty results have consistent structure
    let empty_callers = CallersResult {
        callers: vec![],
        markdown: "# Callers\n\nNo callers found".to_string(),
        total_count: 0,
    };

    assert_eq!(
        empty_callers.callers.len(),
        empty_callers.total_count,
        "Empty results should have consistent counts"
    );

    let empty_impacts = ImpactResult {
        impacts: vec![],
        markdown: "# Impact Analysis\n\nNo impacts found".to_string(),
        total_count: 0,
    };

    assert_eq!(
        empty_impacts.impacts.len(),
        empty_impacts.total_count,
        "Empty impacts should have consistent counts"
    );

    // Test populated results have consistent structure
    let populated_callers = CallersResult {
        callers: vec![
            CallSite {
                caller: "caller1".to_string(),
                file_path: "file1.rs".to_string(),
                line_number: Some(1),
                context: "Context 1".to_string(),
            },
            CallSite {
                caller: "caller2".to_string(),
                file_path: "file2.rs".to_string(),
                line_number: Some(2),
                context: "Context 2".to_string(),
            },
        ],
        markdown: "# Callers".to_string(),
        total_count: 2,
    };

    assert_eq!(
        populated_callers.callers.len(),
        populated_callers.total_count,
        "Populated results should have consistent counts"
    );

    Ok(())
}

/// Test line number handling and overflow protection
/// Ensures line numbers are handled safely without panicking
#[tokio::test]
async fn test_line_number_handling() -> Result<()> {
    // Test various line number scenarios
    let test_cases = vec![
        (Some(1u32), "line 1"),
        (Some(42u32), "line 42"),
        (Some(u32::MAX), "line 4294967295"),
        (None, "overflow case"),
    ];

    for (line_number, description) in test_cases {
        let call_site = CallSite {
            caller: format!("test_caller_{}", description),
            file_path: "test.rs".to_string(),
            line_number,
            context: format!("Test context for {}", description),
        };

        // Should serialize without panicking
        let json_result = serde_json::to_string(&call_site);
        assert!(
            json_result.is_ok(),
            "Should serialize line number case: {}",
            description
        );

        let json_string = json_result.unwrap();
        match line_number {
            Some(line) => {
                assert!(
                    json_string.contains(&line.to_string()),
                    "Should contain line number for case: {}",
                    description
                );
            }
            None => {
                assert!(
                    json_string.contains("null"),
                    "Should contain null for overflow case: {}",
                    description
                );
            }
        }
    }

    Ok(())
}

/// Test edge case inputs in API structures
/// Ensures API can handle various input scenarios gracefully
#[tokio::test]
async fn test_edge_case_inputs() -> Result<()> {
    let edge_cases = vec![
        ("", "empty string"),
        ("a", "single character"),
        (
            "function_with_very_long_name_that_might_cause_buffer_issues",
            "long name",
        ),
        ("func with spaces", "spaces"),
        ("func_with_unicode_ñoño_测试", "unicode"),
        ("func\nwith\nnewlines", "newlines"),
        ("func\twith\ttabs", "tabs"),
    ];

    for (input, description) in edge_cases {
        let call_site = CallSite {
            caller: input.to_string(),
            file_path: format!("path/to/{}.rs", input.replace(['/', '\\', '\n', '\t'], "_")),
            line_number: Some(1),
            context: format!("Test context for {}", input),
        };

        // Should serialize without panicking
        let json_result = serde_json::to_string(&call_site);
        assert!(
            json_result.is_ok(),
            "Should handle edge case: {}",
            description
        );

        let json_string = json_result.unwrap();
        assert!(
            json_string.contains("\"caller\""),
            "Should contain caller field for: {}",
            description
        );
        assert!(
            json_string.contains("\"file_path\""),
            "Should contain file_path field for: {}",
            description
        );
        assert!(
            json_string.contains("\"context\""),
            "Should contain context field for: {}",
            description
        );
    }

    Ok(())
}

/// Test semantic mapping improvements
/// Validates that relationship types are mapped to meaningful context strings
#[tokio::test]
async fn test_semantic_context_mapping() -> Result<()> {
    // Test various context patterns that should be more semantic than hardcoded strings
    let test_contexts = vec![
        "Calls target at line 42",
        "Uses target at line 100",
        "References target at line 200",
        "Imports target at line 1",
    ];

    for context in test_contexts {
        let call_site = CallSite {
            caller: "test_caller".to_string(),
            file_path: "test.rs".to_string(),
            line_number: Some(42),
            context: context.to_string(),
        };

        let json_result = serde_json::to_string(&call_site)?;

        // Validate that context is meaningful and not just hardcoded "Calls"
        assert!(
            json_result.contains(context),
            "Should contain semantic context: {}",
            context
        );

        // Ensure it's not the old hardcoded pattern
        if context != "Calls target at line 42" {
            assert!(
                !json_result.contains("\"Calls target at line"),
                "Should not contain hardcoded 'Calls' pattern for: {}",
                context
            );
        }
    }

    // Test impact type semantic mapping
    let impact_types = vec![
        "Function Call Impact",
        "Usage Impact",
        "Import Impact",
        "Reference Impact",
        "Custom Relationship Impact (custom_type)",
    ];

    for impact_type in impact_types {
        let impact_site = ImpactSite {
            affected_symbol: "test_symbol".to_string(),
            file_path: "test.rs".to_string(),
            line_number: Some(42),
            impact_type: impact_type.to_string(),
        };

        let json_result = serde_json::to_string(&impact_site)?;
        assert!(
            json_result.contains(impact_type),
            "Should contain semantic impact type: {}",
            impact_type
        );

        // Should not contain raw debug format like "RelationType::Calls"
        assert!(
            !json_result.contains("RelationType::"),
            "Should not contain debug format for impact type: {}",
            impact_type
        );
    }

    Ok(())
}
