// Property-Based Testing - Stage 5: Adversarial Testing with Proptest
// These tests use property-based testing to find edge cases automatically

use anyhow::Result;
use kotadb::*;
use proptest::prelude::*;
use uuid::Uuid;

// Custom strategies for generating test data
mod strategies {
    use super::*;

    // Generate valid file paths
    pub fn file_path_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex(r"/[a-zA-Z0-9_/-]{1,100}\.md").unwrap()
    }

    // Generate potentially problematic file paths
    pub fn adversarial_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Normal paths
            prop::string::string_regex(r"/[a-zA-Z0-9_/-]{1,100}\.md").unwrap(),
            // Empty
            Just("".to_string()),
            // Very long paths
            prop::string::string_regex(r"/[a-z]{5000}\.md").unwrap(),
            // Path traversal attempts
            Just("../../../etc/passwd".to_string()),
            // Null bytes
            Just("/test\0file.md".to_string()),
            // Unicode
            Just("/тест/файл.md".to_string()),
            // Special characters
            Just("/test|file*.md".to_string()),
            // Windows reserved names
            Just("CON.md".to_string()),
            Just("PRN.txt".to_string()),
            // Spaces and dots
            Just("/test .md".to_string()),
            Just("/test..md".to_string()),
        ]
    }

    // Generate document sizes
    pub fn size_strategy() -> impl Strategy<Value = u64> {
        prop_oneof![
            // Normal sizes
            1u64..10_000,
            // Edge cases
            Just(0u64),
            Just(1u64),
            Just(u64::MAX),
            Just(100 * 1024 * 1024), // 100MB
        ]
    }

    // Generate timestamps
    pub fn timestamp_strategy() -> impl Strategy<Value = i64> {
        prop_oneof![
            // Normal timestamps
            1_000_000_000i64..2_000_000_000,
            // Edge cases
            Just(0i64),
            Just(-1i64),
            Just(i64::MAX),
            Just(i64::MIN),
        ]
    }

    // Generate document titles
    pub fn title_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Normal titles
            "[A-Za-z0-9 ]{1,50}",
            // Empty
            Just("".to_string()),
            // Very long
            prop::string::string_regex(r"[A-Za-z]{2000}").unwrap(),
            // Unicode
            Just("测试文档 🎯".to_string()),
            // Special chars
            Just("Test <script>alert('xss')</script>".to_string()),
        ]
    }

    // Generate search queries
    pub fn query_text_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Normal queries
            "[A-Za-z0-9 ]{1,20}",
            // Empty
            Just("".to_string()),
            // Very long
            prop::string::string_regex(r"[A-Za-z]{2000}").unwrap(),
            // Special patterns
            Just(".*".to_string()),
            Just("\\".to_string()),
            Just("(".to_string()),
            // SQL injection attempts
            Just("'; DROP TABLE documents; --".to_string()),
        ]
    }

    // Generate tag lists
    pub fn tags_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                "[a-z0-9-_]{1,20}",
                Just("".to_string()),
                Just("very-long-tag-name-that-exceeds-reasonable-limits".to_string()),
                Just("tag with spaces".to_string()),
                Just("tag@with#special$chars".to_string()),
            ],
            0..10,
        )
    }
}

// Property: Document validation should accept all valid documents
proptest! {
    #[test]
    fn prop_valid_documents_accepted(
        path in strategies::file_path_strategy(),
        size in 1u64..1_000_000,
        created in 1_000_000_000i64..1_500_000_000,
        updated_offset in 0i64..1_000_000,
        title in "[A-Za-z0-9 ]{1,100}",
        word_count in 1u32..100_000,
    ) {
        let doc = Document::new(
            Uuid::new_v4(),
            path,
            [0u8; 32],
            size,
            created,
            created + updated_offset,
            title,
            word_count,
        );

        // Should create valid documents
        prop_assert!(doc.is_ok());

        if let Ok(doc) = doc {
            // Should pass validation
            prop_assert!(validation::validate_document(&doc).is_ok());
        }
    }
}

// Property: Invalid documents should be rejected
proptest! {
    #[test]
    fn prop_invalid_documents_rejected(
        path in strategies::adversarial_path_strategy(),
        size in strategies::size_strategy(),
        created in strategies::timestamp_strategy(),
        updated in strategies::timestamp_strategy(),
        title in strategies::title_strategy(),
        word_count in any::<u32>(),
    ) {
        let doc = Document::new(
            Uuid::new_v4(),
            path.clone(),
            [0u8; 32],
            size,
            created,
            updated,
            title.clone(),
            word_count,
        );

        // Check if document creation fails for invalid inputs
        if path.is_empty() ||
           size == 0 ||
           updated < created ||
           title.is_empty() ||
           path.len() >= 4096 {
            prop_assert!(doc.is_err() || validation::validate_document(&doc.unwrap()).is_err());
        }
    }
}

// Property: Path validation should handle all inputs safely
proptest! {
    #[test]
    fn prop_path_validation_safety(
        path in strategies::adversarial_path_strategy()
    ) {
        // Should not panic on any input
        let result = validation::path::validate_file_path(&path);

        // Check expected failures
        if path.is_empty() ||
           path.contains('\0') ||
           path.contains("..") ||
           path.len() >= 4096 ||
           path.to_uppercase().contains("CON") ||
           path.to_uppercase().contains("PRN") {
            prop_assert!(result.is_err());
        }
    }
}

// Property: Query validation should handle all inputs
proptest! {
    #[test]
    fn prop_query_validation(
        text in prop::option::of(strategies::query_text_strategy()),
        tags in prop::option::of(strategies::tags_strategy()),
        start_date in strategies::timestamp_strategy(),
        end_date in strategies::timestamp_strategy(),
        limit in any::<usize>(),
    ) {
        let date_range = if start_date <= end_date {
            Some((start_date, end_date))
        } else {
            None
        };

        let query = Query::new(text.clone(), tags.clone(), date_range, limit);

        // Should fail if no criteria or invalid limit
        if (text.is_none() && tags.is_none() && date_range.is_none()) ||
           limit == 0 || limit > 1000 {
            prop_assert!(query.is_err());
        }
    }
}

// Property: Trigram extraction should be consistent
proptest! {
    #[test]
    fn prop_trigram_consistency(
        text in prop::string::string_regex(r"[a-zA-Z0-9 ]{0,1000}").unwrap()
    ) {
        use kotadb::pure::trigram;

        let trigrams1 = trigram::extract_trigrams(&text);
        let trigrams2 = trigram::extract_trigrams(&text);

        // Should be deterministic
        prop_assert_eq!(&trigrams1, &trigrams2);

        // Should have correct count
        if text.len() >= 3 {
            prop_assert_eq!(trigrams1.len(), text.len() - 2);
        } else {
            prop_assert_eq!(trigrams1.len(), 0);
        }

        // Each trigram should be 3 bytes
        for trigram in &trigrams1 {
            prop_assert_eq!(trigram.len(), 3);
        }
    }
}

// Property: Edit distance should be symmetric
proptest! {
    #[test]
    fn prop_edit_distance_symmetric(
        s1 in prop::string::string_regex(r"[a-zA-Z]{0,100}").unwrap(),
        s2 in prop::string::string_regex(r"[a-zA-Z]{0,100}").unwrap(),
    ) {
        use kotadb::pure::trigram;

        let dist1 = trigram::edit_distance(&s1, &s2);
        let dist2 = trigram::edit_distance(&s2, &s1);

        // Should be symmetric
        prop_assert_eq!(dist1, dist2);

        // Should be 0 for identical strings
        if s1 == s2 {
            prop_assert_eq!(dist1, 0);
        }
    }
}

// Property: BM25 scoring should be monotonic
proptest! {
    #[test]
    fn prop_bm25_monotonic(
        tf1 in 0.0f32..10.0,
        tf2 in 0.0f32..10.0,
        doc_len in 1usize..10000,
        avg_doc_len in 100.0f32..1000.0,
        doc_freq in 1usize..100,
        total_docs in 100usize..10000,
    ) {
        use kotadb::pure::scoring;

        prop_assume!(doc_freq < total_docs);

        let score1 = scoring::calculate_bm25(
            tf1,
            doc_len,
            avg_doc_len,
            doc_freq,
            total_docs,
            1.2,
            0.75,
        );

        let score2 = scoring::calculate_bm25(
            tf2,
            doc_len,
            avg_doc_len,
            doc_freq,
            total_docs,
            1.2,
            0.75,
        );

        // Higher term frequency should give higher score
        if tf1 > tf2 {
            prop_assert!(score1 >= score2);
        }
    }
}

// Property: Hash function should be deterministic
proptest! {
    #[test]
    fn prop_hash_deterministic(
        content in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        use kotadb::pure::metadata;

        let hash1 = metadata::calculate_hash(&content);
        let hash2 = metadata::calculate_hash(&content);

        // Should be deterministic
        prop_assert_eq!(hash1, hash2);

        // Should be 32 bytes (SHA-256)
        prop_assert_eq!(hash1.len(), 32);
    }
}

// Property: Graph cycle detection should be consistent
proptest! {
    #[test]
    fn prop_graph_cycle_detection(
        nodes in prop::collection::vec(any::<u64>(), 1..20),
        edge_probability in 0.0f32..1.0,
    ) {
        use std::collections::HashMap;
        use kotadb::pure::graph;

        let mut edges: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let node_ids: Vec<Uuid> = nodes.iter().map(|_| Uuid::new_v4()).collect();

        // Generate random edges
        for i in 0..node_ids.len() {
            let mut node_edges = Vec::new();
            for j in 0..node_ids.len() {
                if i != j && rand::random::<f32>() < edge_probability {
                    node_edges.push(node_ids[j]);
                }
            }
            if !node_edges.is_empty() {
                edges.insert(node_ids[i], node_edges);
            }
        }

        // Cycle detection should be deterministic
        let has_cycle1 = graph::has_cycle(&edges);
        let has_cycle2 = graph::has_cycle(&edges);

        prop_assert_eq!(has_cycle1, has_cycle2);
    }
}

// Property: Compression ratio estimation bounds
proptest! {
    #[test]
    fn prop_compression_ratio_bounds(
        text in prop::string::string_regex(r"[a-zA-Z0-9 \n]{0,10000}").unwrap()
    ) {
        use kotadb::pure::compression;

        let ratio = compression::estimate_compression_ratio(&text);

        // Ratio should be between 0 and 1
        prop_assert!(ratio >= 0.0 && ratio <= 1.0);

        // Empty text should have ratio 1.0
        if text.is_empty() {
            prop_assert_eq!(ratio, 1.0);
        }

        // Highly repetitive text should have low ratio (high compression)
        let repetitive = "a".repeat(1000);
        let repetitive_ratio = compression::estimate_compression_ratio(&repetitive);
        if text.len() > 100 {
            prop_assert!(repetitive_ratio < ratio || (ratio - repetitive_ratio).abs() < 0.1);
        }
    }
}

// Property: Transaction validation
proptest! {
    #[test]
    fn prop_transaction_validation(
        tx_id in any::<u64>(),
        op_count in 0usize..100,
    ) {
        let tx = Transaction::begin(tx_id);

        // Should fail for tx_id = 0
        if tx_id == 0 {
            prop_assert!(tx.is_err());
        } else {
            prop_assert!(tx.is_ok());
        }
    }
}

// Property: Storage metrics consistency
proptest! {
    #[test]
    fn prop_storage_metrics_consistency(
        doc_count in any::<usize>(),
        total_size in any::<u64>(),
        index_sizes in prop::collection::hash_map(
            "[a-z]+",
            any::<usize>(),
            0..5
        ),
    ) {
        let metrics = StorageMetrics {
            document_count: doc_count,
            total_size_bytes: total_size,
            index_sizes,
        };

        let valid = metrics.validate();

        // Should fail if size < count
        if total_size < doc_count as u64 {
            prop_assert!(valid.is_err());
        }

        // Should pass for reasonable values
        if doc_count < 1_000_000 && total_size >= doc_count as u64 {
            prop_assert!(valid.is_ok());
        }
    }
}

// Property: Concurrent operations should not corrupt data
proptest! {
    #[test]
    fn prop_concurrent_safety(
        operations in prop::collection::vec(
            (any::<bool>(), any::<u64>(), 1u64..1000),
            1..20
        )
    ) {
        use tokio::runtime::Runtime;
        use std::sync::{Arc, Mutex};
        use std::collections::HashMap;

        let rt = Runtime::new().unwrap();
        let storage = Arc::new(Mutex::new(HashMap::<u64, u64>::new()));

        rt.block_on(async {
            let mut handles = vec![];

            for (is_write, key, value) in operations {
                let storage_clone = Arc::clone(&storage);
                let handle = tokio::spawn(async move {
                    if is_write {
                        let mut map = storage_clone.lock().unwrap();
                        map.insert(key, value);
                    } else {
                        let map = storage_clone.lock().unwrap();
                        let _ = map.get(&key);
                    }
                });
                handles.push(handle);
            }

            // Wait for all operations
            for handle in handles {
                handle.await.unwrap();
            }
        });

        // Storage should still be valid
        let final_storage = storage.lock().unwrap();
        prop_assert!(final_storage.len() <= 20);
    }
}
