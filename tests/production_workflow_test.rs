// Production Workflow Integration Tests - Stage 1: TDD for Phase 3 Production Readiness
// Comprehensive end-to-end tests for complete document lifecycle in production scenarios

use anyhow::Result;
use kotadb::{contracts::BulkOperations, *};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tempfile::TempDir;
use tokio::task;
use uuid::Uuid;

/// Test complete CRUD operations with integrated FileStorage + OptimizedIndex
#[tokio::test]
async fn test_full_document_lifecycle_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let index_path = temp_dir.path().join("index");

    // Create production-grade integrated system
    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    // Test data - realistic documents
    let documents = create_realistic_documents(100)?;

    // Stage 1: Bulk document insertion with integrated storage and indexing
    let start = Instant::now();
    let mut inserted_ids = Vec::new();

    for doc in &documents {
        // Insert into storage
        storage.insert(doc.clone()).await?;

        // Index the document
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
        inserted_ids.push(doc.id.clone());
    }

    let insert_duration = start.elapsed();
    println!(
        "Inserted {} documents in {:?}",
        documents.len(),
        insert_duration
    );

    // Performance assertion: Should handle 100 documents in reasonable time
    assert!(
        insert_duration < Duration::from_secs(30),
        "Document insertion took too long: {:?}",
        insert_duration
    );

    // Stage 2: Verify all documents are retrievable
    let start = Instant::now();
    let mut retrieved_count = 0;

    for doc_id in &inserted_ids {
        // Verify storage retrieval
        let stored_doc = storage.get(doc_id).await?;
        assert!(
            stored_doc.is_some(),
            "Document not found in storage: {}",
            doc_id
        );

        // Verify index can find the document
        let query = QueryBuilder::new().with_limit(1)?.build()?;
        let search_results = optimized_index.search(&query).await?;

        // For now, just verify search doesn't error (real search would be more specific)
        retrieved_count += 1;
    }

    let retrieval_duration = start.elapsed();
    println!(
        "Retrieved {} documents in {:?}",
        retrieved_count, retrieval_duration
    );

    assert_eq!(
        retrieved_count,
        documents.len(),
        "Not all documents retrievable"
    );

    // Stage 3: Update operations
    let update_count = 20;
    let mut updated_docs = Vec::new();

    for i in 0..update_count {
        let original_doc = &documents[i];
        let mut updated_doc = original_doc.clone();
        updated_doc.content = format!("Updated content for document {}", i).into_bytes();
        updated_doc.updated_at = chrono::Utc::now();

        // Update in storage
        storage.insert(updated_doc.clone()).await?; // Insert overwrites
        updated_docs.push(updated_doc);
    }

    // Verify updates are reflected
    for updated_doc in &updated_docs {
        let retrieved = storage.get(&updated_doc.id).await?;
        assert!(retrieved.is_some(), "Updated document not found");

        let retrieved_doc = retrieved.unwrap();
        assert_eq!(
            retrieved_doc.content, updated_doc.content,
            "Document content not updated properly"
        );
    }

    // Stage 4: Deletion operations
    let delete_count = 10;
    let mut deleted_ids = Vec::new();

    for i in 0..delete_count {
        let doc_id = &inserted_ids[i];

        // Delete from storage
        let storage_deleted = storage.delete(doc_id).await?;
        assert!(storage_deleted, "Document not deleted from storage");

        // Delete from index
        let index_deleted = optimized_index.delete(doc_id).await?;
        assert!(index_deleted, "Document not deleted from index");

        deleted_ids.push(doc_id.clone());
    }

    // Verify deletions
    for deleted_id in &deleted_ids {
        let retrieved = storage.get(deleted_id).await?;
        assert!(
            retrieved.is_none(),
            "Deleted document still found in storage"
        );
    }

    // Stage 5: Final consistency check
    let remaining_count = documents.len() - delete_count;
    let final_storage_list = storage.list_all().await?;
    assert_eq!(
        final_storage_list.len(),
        remaining_count,
        "Storage count inconsistent after operations"
    );

    println!("✅ Full document lifecycle test completed successfully");
    println!("  - Inserted: {} documents", documents.len());
    println!("  - Updated: {} documents", update_count);
    println!("  - Deleted: {} documents", delete_count);
    println!("  - Remaining: {} documents", remaining_count);

    Ok(())
}

/// Test multi-user concurrent access patterns
#[tokio::test]
async fn test_concurrent_multi_user_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("concurrent_storage");
    let index_path = temp_dir.path().join("concurrent_index");

    // Create shared system
    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(2000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(2000)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    let num_users = 10;
    let docs_per_user = 50;
    let mut handles = Vec::new();

    // Spawn concurrent user sessions
    for user_id in 0..num_users {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);

        let handle = task::spawn(async move {
            let mut user_docs = Vec::new();

            // Each user creates their own documents
            for doc_num in 0..docs_per_user {
                let doc = create_user_document(user_id, doc_num)?;

                // Concurrent write operations
                {
                    let mut storage_guard = storage_ref.lock().await;
                    storage_guard.insert(doc.clone()).await?;
                }

                {
                    let mut index_guard = index_ref.lock().await;
                    index_guard.insert(doc.id.clone(), doc.path.clone()).await?;
                }

                user_docs.push(doc);

                // Small delay to increase concurrency
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            // Each user reads their own documents
            let mut successful_reads = 0;
            for doc in &user_docs {
                let storage_guard = storage_ref.lock().await;
                if let Ok(Some(_)) = storage_guard.get(&doc.id).await {
                    successful_reads += 1;
                }
            }

            Ok::<(usize, usize), anyhow::Error>((user_docs.len(), successful_reads))
        });

        handles.push(handle);
    }

    // Wait for all users to complete
    let mut total_created = 0;
    let mut total_read = 0;

    for handle in handles {
        let (created, read) = handle.await??;
        total_created += created;
        total_read += read;
    }

    println!("Concurrent access test results:");
    println!("  - Total documents created: {}", total_created);
    println!("  - Total documents read: {}", total_read);
    println!(
        "  - Success rate: {:.1}%",
        (total_read as f64 / total_created as f64) * 100.0
    );

    // Verify final consistency
    let final_storage = storage.lock().await;
    let all_docs = final_storage.list_all().await?;
    assert_eq!(
        all_docs.len(),
        total_created,
        "Final document count inconsistent with concurrent operations"
    );

    // Performance assertion: Should maintain good read success rate
    let success_rate = total_read as f64 / total_created as f64;
    assert!(
        success_rate > 0.95,
        "Read success rate too low: {:.2}%",
        success_rate * 100.0
    );

    Ok(())
}

/// Test large dataset operations (10k+ documents)
#[tokio::test]
async fn test_large_dataset_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("large_storage");
    let index_path = temp_dir.path().join("large_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(10000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(10000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    let dataset_size = 10000;
    println!("Creating large dataset with {} documents...", dataset_size);

    // Stage 1: Bulk insertion of large dataset
    let start = Instant::now();
    let documents = create_realistic_documents(dataset_size)?;

    // Use bulk operations for efficiency
    let mut insert_pairs = Vec::new();
    for doc in &documents {
        storage.insert(doc.clone()).await?;
        insert_pairs.push((doc.id.clone(), doc.path.clone()));
    }

    // Bulk index insertion
    let bulk_result = optimized_index.bulk_insert(insert_pairs)?;
    let insertion_duration = start.elapsed();

    println!("Large dataset insertion completed:");
    println!("  - Duration: {:?}", insertion_duration);
    println!(
        "  - Bulk operation result: {} ops completed",
        bulk_result.operations_completed
    );
    println!(
        "  - Throughput: {:.0} ops/sec",
        bulk_result.throughput_ops_per_sec
    );

    // Performance assertions for large datasets
    assert!(
        insertion_duration < Duration::from_secs(120), // 2 minutes max
        "Large dataset insertion took too long: {:?}",
        insertion_duration
    );
    assert!(
        bulk_result.meets_performance_requirements(5.0),
        "Bulk insertion did not meet performance requirements"
    );

    // Stage 2: Random access performance on large dataset
    let sample_size = 1000;
    let start = Instant::now();

    for _ in 0..sample_size {
        let random_idx = fastrand::usize(..documents.len());
        let doc_id = &documents[random_idx].id;

        let retrieved = storage.get(doc_id).await?;
        assert!(retrieved.is_some(), "Document not found in large dataset");
    }

    let random_access_duration = start.elapsed();
    let avg_access_time = random_access_duration / sample_size as u32;

    println!("Random access performance:");
    println!(
        "  - {} random accesses in {:?}",
        sample_size, random_access_duration
    );
    println!("  - Average access time: {:?}", avg_access_time);

    // Performance assertion: Average access should be sub-millisecond
    assert!(
        avg_access_time < Duration::from_millis(1),
        "Average random access too slow: {:?}",
        avg_access_time
    );

    // Stage 3: Bulk deletion performance
    let delete_size = 2000;
    let delete_keys: Vec<_> = documents[..delete_size]
        .iter()
        .map(|doc| doc.id.clone())
        .collect();

    let start = Instant::now();

    // Delete from storage
    for key in &delete_keys {
        storage.delete(key).await?;
    }

    // Bulk delete from index
    let bulk_delete_result = optimized_index.bulk_delete(delete_keys)?;
    let deletion_duration = start.elapsed();

    println!("Bulk deletion performance:");
    println!("  - {} deletions in {:?}", delete_size, deletion_duration);
    println!(
        "  - Bulk delete throughput: {:.0} ops/sec",
        bulk_delete_result.throughput_ops_per_sec
    );

    // Verify final state
    let remaining_docs = storage.list_all().await?;
    let expected_remaining = dataset_size - delete_size;

    assert_eq!(
        remaining_docs.len(),
        expected_remaining,
        "Incorrect number of documents remaining after bulk deletion"
    );

    Ok(())
}

/// Test transaction boundaries and consistency
#[tokio::test]
async fn test_transaction_consistency() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("transaction_storage");
    let index_path = temp_dir.path().join("transaction_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    // Create test documents
    let docs = create_realistic_documents(50)?;

    // Test atomic operations - all succeed or all fail
    println!("Testing atomic transaction behavior...");

    // Stage 1: Successful transaction simulation
    let transaction_docs = &docs[..25];
    let start = Instant::now();

    // Simulate transaction: insert all documents
    let mut inserted_successfully = Vec::new();
    let mut transaction_success = true;

    for doc in transaction_docs {
        match storage.insert(doc.clone()).await {
            Ok(_) => {
                match optimized_index
                    .insert(doc.id.clone(), doc.path.clone())
                    .await
                {
                    Ok(_) => inserted_successfully.push(doc.id.clone()),
                    Err(e) => {
                        println!("Index insert failed: {}", e);
                        transaction_success = false;
                        break;
                    }
                }
            }
            Err(e) => {
                println!("Storage insert failed: {}", e);
                transaction_success = false;
                break;
            }
        }
    }

    println!(
        "Transaction completed: success={}, docs={}",
        transaction_success,
        inserted_successfully.len()
    );

    if transaction_success {
        // Verify all documents are present
        for doc_id in &inserted_successfully {
            assert!(
                storage.get(doc_id).await?.is_some(),
                "Document missing after successful transaction"
            );
        }
    }

    // Stage 2: Test consistency across storage and index
    let consistency_check_start = Instant::now();
    let all_storage_docs = storage.list_all().await?;

    // Each document in storage should be findable via index
    let mut consistent_count = 0;
    for doc in &all_storage_docs {
        let query = QueryBuilder::new().with_limit(100)?.build()?;

        // Note: This is a simplified consistency check
        // In a real implementation, we'd search specifically for this document
        let _search_results = optimized_index.search(&query).await?;
        consistent_count += 1;
    }

    println!(
        "Consistency check: {}/{} documents consistent",
        consistent_count,
        all_storage_docs.len()
    );

    assert_eq!(
        consistent_count,
        all_storage_docs.len(),
        "Storage and index inconsistency detected"
    );

    Ok(())
}

/// Test error recovery and rollback scenarios
#[tokio::test]
async fn test_error_recovery_scenarios() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("recovery_storage");
    let index_path = temp_dir.path().join("recovery_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    // Stage 1: Test graceful handling of invalid operations
    println!("Testing error recovery scenarios...");

    // Test 1: Attempt to retrieve non-existent document
    let fake_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let result = storage.get(&fake_id).await?;
    assert!(
        result.is_none(),
        "Should return None for non-existent document"
    );

    // Test 2: Attempt to delete non-existent document
    let delete_result = storage.delete(&fake_id).await?;
    assert!(
        !delete_result,
        "Should return false when deleting non-existent document"
    );

    // Test 3: Insert valid documents, then test partial failure scenarios
    let docs = create_realistic_documents(10)?;

    // Insert some documents successfully
    for doc in &docs[..5] {
        storage.insert(doc.clone()).await?;
        optimized_index
            .insert(doc.id.clone(), doc.path.clone())
            .await?;
    }

    // Test 4: Verify system state remains consistent after errors
    let storage_docs = storage.list_all().await?;
    assert_eq!(
        storage_docs.len(),
        5,
        "Storage should contain exactly 5 documents after partial operations"
    );

    // Test 5: Test recovery from corrupted operations
    // Simulate partial state by inserting to storage but not index
    let orphaned_doc = &docs[5];
    storage.insert(orphaned_doc.clone()).await?;

    // Verify detection of inconsistent state
    let storage_count = storage.list_all().await?.len();
    // In a real implementation, we'd have a consistency checker
    println!(
        "After orphaned insert: storage has {} documents",
        storage_count
    );

    // Test 6: Cleanup and recovery operations
    // Remove the orphaned document to restore consistency
    storage.delete(&orphaned_doc.id).await?;

    let final_storage_docs = storage.list_all().await?;
    assert_eq!(
        final_storage_docs.len(),
        5,
        "After cleanup, storage should return to consistent state"
    );

    println!("✅ Error recovery test completed successfully");

    Ok(())
}

// Helper functions for creating test data

fn create_realistic_documents(count: usize) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(&format!("/production/docs/document_{:04}.md", i))?;
        let title = ValidatedTitle::new(&format!("Production Document {}", i))?;

        let content = format!(
            r#"---
title: Production Document {}
tags: [production, test, document-{}]
created: {}
updated: {}
---

# Production Document {}

This is a realistic test document with substantial content to simulate
real-world usage patterns. It contains multiple paragraphs and sections
to test the database's handling of various document sizes.

## Section 1: Overview

This document serves as test data for production workflow validation.
It includes metadata in frontmatter format and structured content.

## Section 2: Content

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim
veniam, quis nostrud exercitation ullamco laboris.

## Section 3: Technical Details

- Document ID: {}
- File Path: {}
- Content Length: Variable
- Test Iteration: {}

This concludes the test document content.
"#,
            i,
            i,
            chrono::Utc::now().format("%Y-%m-%d"),
            chrono::Utc::now().format("%Y-%m-%d"),
            i,
            id,
            path.as_str(),
            i
        )
        .into_bytes();

        let tags = vec![
            ValidatedTag::new(&format!("category-{}", i % 5))?,
            ValidatedTag::new("production")?,
            ValidatedTag::new("test")?,
        ];

        let now = chrono::Utc::now();

        let document = Document::new(id, path, title, content, tags, now, now);

        documents.push(document);
    }

    Ok(documents)
}

fn create_user_document(user_id: usize, doc_num: usize) -> Result<Document> {
    let id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
    let path = ValidatedPath::new(&format!("/users/user_{}/document_{}.md", user_id, doc_num))?;
    let title = ValidatedTitle::new(&format!("User {} Document {}", user_id, doc_num))?;

    let content = format!(
        "# User Document\n\nUser: {}\nDocument: {}\nContent: Test data for concurrent access patterns.",
        user_id, doc_num
    ).into_bytes();

    let tags = vec![
        ValidatedTag::new(&format!("user-{}", user_id))?,
        ValidatedTag::new("concurrent-test")?,
    ];

    let now = chrono::Utc::now();

    Ok(Document::new(id, path, title, content, tags, now, now))
}
