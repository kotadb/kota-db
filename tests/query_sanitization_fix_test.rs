// Test to ensure query sanitization doesn't block legitimate programming terms
// Addresses issue #345: CLI text search functionality not working for trigram queries

use anyhow::Result;
use kotadb::query_sanitization::sanitize_search_query;

#[test]
fn test_legitimate_programming_terms_allowed() -> Result<()> {
    // Test that common programming terms are not blocked by sanitization
    let programming_terms = [
        "rust",
        "script",
        "javascript",
        "typescript",
        "exec",
        "execute",
        "eval",
    ];

    for term in &programming_terms {
        let result = sanitize_search_query(term)?;
        assert!(
            !result.is_empty(),
            "Term '{}' should not be empty after sanitization",
            term
        );
        assert_eq!(
            result.text.trim(),
            *term,
            "Term '{}' should be preserved",
            term
        );
        assert!(
            !result.was_modified,
            "Term '{}' should not be modified",
            term
        );
    }

    // SQL-flavored keywords are rejected when strict sanitization is enabled.
    let sql_terms = [
        "select", "insert", "update", "delete", "create", "drop", "alter", "union",
    ];

    for term in &sql_terms {
        let result = sanitize_search_query(term);
        if cfg!(feature = "strict-sanitization") {
            assert!(
                result.is_err(),
                "Term '{}' should be rejected in strict sanitization mode",
                term
            );
        } else {
            let result = result?;
            assert!(
                !result.is_empty(),
                "Term '{}' should not be empty after sanitization",
                term
            );
            assert_eq!(
                result.text.trim(),
                *term,
                "Term '{}' should be preserved",
                term
            );
            assert!(
                !result.was_modified,
                "Term '{}' should not be modified",
                term
            );
        }
    }

    Ok(())
}

#[test]
fn test_multi_word_programming_queries_allowed() -> Result<()> {
    let queries = if cfg!(feature = "strict-sanitization") {
        vec![
            ("rust programming", "rust programming"),
            ("javascript function", "javascript function"),
            ("create component", "component"),
            ("select element", "element"),
            ("insert data", "data"),
            ("script tag", "script tag"),
        ]
    } else {
        vec![
            ("rust programming", "rust programming"),
            ("javascript function", "javascript function"),
            ("create component", "create component"),
            ("select element", "select element"),
            ("insert data", "insert data"),
            ("script tag", "script tag"),
        ]
    };

    for (query, expected) in &queries {
        let result = sanitize_search_query(query)?;
        assert!(
            !result.is_empty(),
            "Query '{}' should not be empty after sanitization",
            query
        );
        assert_eq!(
            result.text.trim(),
            *expected,
            "Query '{}' should be preserved or safely reduced",
            query
        );
    }

    Ok(())
}

#[test]
fn test_actual_sql_injection_blocked() -> Result<()> {
    // Test that actual SQL injection patterns are still blocked
    let malicious_queries = [
        "union select * from users",
        "select * from passwords",
        "insert into admin values",
        "update users set password",
        "delete from important_table",
        "drop table users",
        "create table backdoor",
        "alter table users add",
        "; drop table users",
    ];

    for query in &malicious_queries {
        let result = sanitize_search_query(query);
        // These should either fail validation or be heavily sanitized
        match result {
            Ok(sanitized) => {
                assert!(
                    sanitized.is_empty() || sanitized.was_modified,
                    "Malicious query '{}' should be blocked or modified",
                    query
                );
            }
            Err(_) => {
                // It's ok if these fail entirely
            }
        }
    }

    Ok(())
}

#[test]
fn test_html_injection_still_blocked() -> Result<()> {
    let malicious_html = [
        "<script>alert('xss')</script>",
        "<iframe src='evil.com'></iframe>",
        "<object data='malware.exe'></object>",
    ];

    for query in &malicious_html {
        let result = sanitize_search_query(query)?;
        assert!(
            result.is_empty()
                || !result.text.contains("<script") && !result.text.contains("<iframe"),
            "HTML injection '{}' should be blocked",
            query
        );
    }

    Ok(())
}

#[test]
fn test_issue_345_original_failing_cases() -> Result<()> {
    // Test the exact cases from issue #345 that were failing
    let test_cases = ["rust programming", "rust", "programming"];

    for query in &test_cases {
        let result = sanitize_search_query(query)?;
        assert!(
            !result.is_empty(),
            "Query '{}' from issue #345 should work",
            query
        );
        assert_eq!(result.text, *query, "Query '{}' should be unchanged", query);
        assert!(
            !result.was_modified,
            "Query '{}' should not be modified",
            query
        );
        assert!(
            result.warnings.is_empty(),
            "Query '{}' should have no warnings",
            query
        );
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use kotadb::{
        create_binary_trigram_index, create_file_storage, DocumentBuilder, Index, QueryBuilder,
        Storage,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_end_to_end_search_with_programming_terms() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("storage");
        let index_path = temp_dir.path().join("trigram_index");

        // Create storage and index
        let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
        let mut index =
            create_binary_trigram_index(index_path.to_str().unwrap(), Some(100)).await?;

        // Insert test document
        let doc = DocumentBuilder::new()
            .path("test/script.md")?
            .title("Script Testing")?
            .content(b"This document contains javascript and script content for testing rust programming")
            .build()?;

        storage.insert(doc.clone()).await?;
        index
            .insert_with_content(doc.id, doc.path.clone(), &doc.content)
            .await?;

        // Test queries that were failing in issue #345
        let test_queries = ["script", "javascript", "rust", "programming"];

        for query_text in &test_queries {
            let query = QueryBuilder::new()
                .with_text(*query_text)?
                .with_limit(10)?
                .build()?;

            let results = index.search(&query).await?;
            assert!(
                !results.is_empty(),
                "Search for '{}' should find the document",
                query_text
            );
            assert!(
                results.contains(&doc.id),
                "Search for '{}' should find our test document",
                query_text
            );
        }

        Ok(())
    }
}
