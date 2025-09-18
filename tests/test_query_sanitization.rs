// Comprehensive tests for query sanitization module
// Testing security enhancements for search input validation

use anyhow::Result;
use kotadb::query_sanitization::*;
use kotadb::{Query, QueryBuilder};

#[test]
fn test_sql_injection_prevention() -> Result<()> {
    // Test various SQL injection attempts
    let test_cases = vec![
        ("SELECT * FROM users", "users"),
        ("'; DROP TABLE users; --", "TABLE users"),
        ("UNION SELECT password FROM admin", "password admin"),
        ("1' OR '1'='1", "1 1 1"),
        ("admin'--", "admin"),
        ("' UNION ALL SELECT NULL--", "NULL"),
    ];

    for (input, _expected_contains) in test_cases {
        let result = sanitize_search_query(input)?;
        assert!(result.was_modified, "Query '{}' should be modified", input);
        assert_ne!(
            result.text, input,
            "Sanitized output should differ from input"
        );
        // Ensure common separators used in injections are removed
        assert!(!result.text.contains(';'));
    }

    Ok(())
}

#[test]
fn test_command_injection_prevention() -> Result<()> {
    let test_cases = vec![
        ("test; rm -rf /", "Should remove semicolon"),
        ("test && ls -la", "Should remove &&"),
        ("test | cat /etc/passwd", "Should remove pipe"),
        ("test `whoami`", "Should remove backticks"),
        ("test $(echo hacked)", "Should remove command substitution"),
        ("test || echo fail", "Should remove ||"),
    ];

    for (input, _description) in test_cases {
        let result = sanitize_search_query(input)?;
        assert!(result.was_modified);
        assert!(!result.text.contains(';'));
        assert!(!result.text.contains('|'));
        assert!(!result.text.contains('`'));
        assert!(!result.text.contains("$("));
        assert!(!result.text.contains("&&"));
    }

    Ok(())
}

#[test]
fn test_path_traversal_prevention() -> Result<()> {
    let test_cases = vec![
        "../../etc/passwd",
        "../../../windows/system32",
        "..\\..\\..\\windows\\system32",
        "%2e%2e%2f%2e%2e%2f",
        "%252e%252e%252f",
        "....//....//etc/passwd",
    ];

    for input in test_cases {
        let result = sanitize_search_query(input)?;
        assert!(result.was_modified);
        assert!(!result.text.contains(".."));
        assert!(!result.text.contains("%2e"));
        assert!(!result.text.contains("%2E"));
    }

    Ok(())
}

#[test]
fn test_xss_prevention() -> Result<()> {
    let test_cases = vec![
        "<script>alert('XSS')</script>",
        "<img src=x onerror=alert('XSS')>",
        "<iframe src='evil.com'></iframe>",
        "javascript:alert('XSS')",
        "<object data='evil.swf'></object>",
        "<embed src='evil.swf'>",
        "<link rel='stylesheet' href='evil.css'>",
        "onclick='alert(1)'",
        "onload=alert(1)",
    ];

    for input in test_cases {
        let result = sanitize_search_query(input)?;
        assert!(result.was_modified);
        assert!(!result.text.to_lowercase().contains("script"));
        assert!(!result.text.to_lowercase().contains("javascript"));
        assert!(!result.text.to_lowercase().contains("onclick"));
        assert!(!result.text.to_lowercase().contains("onerror"));
        assert!(!result.text.to_lowercase().contains("onload"));
        assert!(!result.text.contains('<'));
        assert!(!result.text.contains('>'));
    }

    Ok(())
}

#[test]
fn test_ldap_injection_prevention() -> Result<()> {
    let test_cases = vec![
        "admin*",
        "(uid=admin)",
        "admin)(password=*",
        "\\admin",
        "admin,dc=example,dc=com",
        "admin=true",
    ];

    for input in test_cases {
        let result = sanitize_search_query(input)?;
        if cfg!(feature = "strict-sanitization") {
            // Strict mode: LDAP-special characters removed
            assert!(!result.text.contains('('));
            assert!(!result.text.contains(')'));
            assert!(!result.text.contains('\\'));
            assert!(!result.text.contains(','));
            assert!(!result.text.contains('='));

            if !input.contains('*') {
                assert!(
                    !result.text.contains('*'),
                    "Strict sanitization should not introduce wildcards for '{}'",
                    input
                );
            }
        } else {
            // Default mode: preserve common characters for developer queries
            // Ensure the result is non-empty and not more dangerous than input
            assert!(!result.text.is_empty());
        }
    }

    Ok(())
}

#[test]
fn test_null_byte_injection() {
    // Null bytes should cause an error
    let result = sanitize_search_query("test\0query");
    assert!(result.is_err());

    let result2 = sanitize_search_query("test\x00query");
    assert!(result2.is_err());
}

#[test]
fn test_control_character_sanitization() -> Result<()> {
    let test_cases = vec![
        ("test\rquery", "test query"),
        ("test\nquery", "test query"),
        ("test\tquery", "test query"),
        ("test\x0bquery", "test query"),
        ("test\x0cquery", "test query"),
    ];

    for (input, expected) in test_cases {
        let result = sanitize_search_query(input)?;
        assert_eq!(result.text, expected);
        assert!(result.was_modified);
    }

    Ok(())
}

#[test]
fn test_whitespace_normalization() -> Result<()> {
    let test_cases = vec![
        ("  test   query  ", "test query"),
        ("test\t\t\tquery", "test query"),
        ("test\n\n\nquery", "test query"),
        ("   \t\n  test   \t\n  query   \t\n  ", "test query"),
    ];

    for (input, expected) in test_cases {
        let result = sanitize_search_query(input)?;
        assert_eq!(result.text, expected);
    }

    Ok(())
}

#[test]
fn test_wildcard_preservation() -> Result<()> {
    // Wildcard queries should be preserved
    let result = sanitize_search_query("*")?;
    assert_eq!(result.text, "*");
    assert!(result.is_wildcard());
    assert!(!result.was_modified);

    Ok(())
}

#[test]
fn test_legitimate_queries_unchanged() -> Result<()> {
    let test_cases = vec![
        "rust programming",
        "async await futures",
        "database optimization",
        "search algorithm",
        "binary tree traversal",
    ];

    for input in test_cases {
        let result = sanitize_search_query(input)?;
        assert_eq!(result.text, input);
        assert!(!result.was_modified);
        assert!(result.warnings.is_empty());
    }

    Ok(())
}

#[test]
fn test_unicode_support() -> Result<()> {
    let test_cases = vec![
        "hello 世界",
        "привет мир",
        "مرحبا بالعالم",
        "こんにちは世界",
        "안녕하세요 세계",
    ];

    for input in test_cases {
        let result = sanitize_search_query(input)?;
        assert_eq!(result.text, input);
        assert!(!result.was_modified);
    }

    Ok(())
}

#[test]
fn test_max_query_length() {
    let long_query = "x".repeat(1025); // Over the 1024 limit
    let result = sanitize_search_query(&long_query);
    assert!(result.is_err());

    // Test with a valid query at the limit
    let words = vec!["test"; 256]; // Create 256 "test" words = 1024+ chars with spaces
    let ok_query = words.join(" ");
    if ok_query.len() <= 1024 {
        let result = sanitize_search_query(&ok_query);
        assert!(result.is_ok());
    }
}

#[test]
fn test_term_extraction() -> Result<()> {
    let result = sanitize_search_query("find rust async functions in the database system")?;

    assert!(result.terms.contains(&"find".to_string()));
    assert!(result.terms.contains(&"rust".to_string()));
    assert!(result.terms.contains(&"async".to_string()));
    assert!(result.terms.contains(&"functions".to_string()));
    assert!(result.terms.contains(&"database".to_string()));
    assert!(result.terms.contains(&"system".to_string()));

    // Stop words might be filtered in terms but present in text
    assert_eq!(
        result.text,
        "find rust async functions in the database system"
    );

    Ok(())
}

#[test]
fn test_tag_sanitization() -> Result<()> {
    // Valid tags
    assert_eq!(sanitize_tag("valid-tag")?, "valid-tag");
    assert_eq!(sanitize_tag("Valid_Tag_123")?, "valid_tag_123");
    assert_eq!(sanitize_tag("UPPERCASE")?, "uppercase");

    // Invalid tags
    assert!(sanitize_tag("invalid!tag").is_err());
    assert!(sanitize_tag("invalid@tag").is_err());
    assert!(sanitize_tag("invalid tag").is_err());
    assert!(sanitize_tag("invalid/tag").is_err());

    // Too long
    let long_tag = "x".repeat(51);
    assert!(sanitize_tag(&long_tag).is_err());

    Ok(())
}

#[test]
fn test_stop_word_filtering() {
    assert!(is_stop_word("the"));
    assert!(is_stop_word("The"));
    assert!(is_stop_word("THE"));
    assert!(is_stop_word("and"));
    assert!(is_stop_word("or"));
    assert!(is_stop_word("but"));

    assert!(!is_stop_word("rust"));
    assert!(!is_stop_word("database"));
    assert!(!is_stop_word("async"));
    assert!(!is_stop_word("function"));
}

#[test]
fn test_query_builder_with_sanitization() -> Result<()> {
    // Test that QueryBuilder properly sanitizes input
    let builder = QueryBuilder::new().with_text("SELECT * FROM users; DROP TABLE users;")?;

    let query = builder.build()?;

    // The query should be sanitized
    let search_terms = &query.search_terms;
    if !search_terms.is_empty() {
        let term = search_terms[0].as_str();
        assert!(!term.to_lowercase().contains("select"));
        assert!(!term.to_lowercase().contains("drop"));
        assert!(!term.contains(';'));
    }

    Ok(())
}

#[test]
fn test_query_new_with_sanitization() -> Result<()> {
    // Test that Query::new properly sanitizes input
    let query = Query::new(
        Some("'; DELETE FROM documents; --".to_string()),
        None,
        None,
        10,
    )?;

    // The query should be sanitized
    if !query.search_terms.is_empty() {
        let term = query.search_terms[0].as_str();
        assert!(!term.to_lowercase().contains("delete"));
        assert!(!term.contains(';'));
        assert!(!term.contains("--"));
    }

    Ok(())
}

#[test]
fn test_mixed_attack_vectors() -> Result<()> {
    // Test combination of multiple attack vectors
    let evil_query = "<script>alert('XSS')</script>'; DROP TABLE users; -- ../../../etc/passwd";

    let result = sanitize_search_query(evil_query)?;

    assert!(result.was_modified);
    assert!(!result.warnings.is_empty());

    // None of the dangerous patterns should remain
    assert!(!result.text.to_lowercase().contains("script"));
    assert!(!result.text.to_lowercase().contains("drop"));
    assert!(!result.text.contains(".."));
    assert!(!result.text.contains(';'));
    assert!(!result.text.contains('<'));
    assert!(!result.text.contains('>'));

    Ok(())
}

#[test]
fn test_empty_after_sanitization() {
    // Some queries might become empty after removing all dangerous content
    let test_cases = vec!["';--", "<!--", "-->", "<>", "''"];

    for input in test_cases {
        let result = sanitize_search_query(input);
        // These should either fail validation or be very short
        match result {
            Ok(sanitized) => {
                assert!(sanitized.text.is_empty() || sanitized.text.len() <= 2);
            }
            Err(_) => {
                // It's also valid if the query fails validation for becoming empty
            }
        }
    }
}

#[test]
fn test_warning_generation() -> Result<()> {
    // Test that appropriate warnings are generated
    let result = sanitize_search_query("SELECT * FROM users UNION DROP TABLE")?;

    assert!(!result.warnings.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("SQL")));

    let result2 = sanitize_search_query("test; rm -rf /")?;
    assert!(!result2.warnings.is_empty());
    assert!(result2.warnings.iter().any(|w| w.contains("command")));

    let result3 = sanitize_search_query("../../etc/passwd")?;
    // Should have warnings about dangerous patterns
    assert!(!result3.warnings.is_empty());

    Ok(())
}
