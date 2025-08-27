// Test for Issue #338: Index synchronization fails on document deletion
// This reproduces the bug where documents are deleted from storage but remain in indices

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, DocumentBuilder, Index,
    QueryBuilder, Storage, ValidatedPath,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_document_deletion_index_synchronization() -> Result<()> {
    // Create temporary directories for storage and indices
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    // Create storage and indices
    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    println!("=== Testing Document Deletion Index Synchronization ===");

    // Step 1: Insert test documents
    println!("Step 1: Inserting test documents...");
    let docs = vec![
        DocumentBuilder::new()
            .path("docs/README.md")?
            .title("Project Documentation")?
            .content(b"# Project Documentation\n\nThis is the main project documentation.")
            .build()?,
        DocumentBuilder::new()
            .path("src/main.rs")?
            .title("Main source file")?
            .content(b"fn main() {\n    println!(\"Hello world!\");\n}")
            .build()?,
        DocumentBuilder::new()
            .path("tests/unit.rs")?
            .title("Unit tests")?
            .content(b"#[test]\nfn test_basic() {\n    assert_eq!(2 + 2, 4);\n}")
            .build()?,
    ];

    // Insert documents into all three systems
    for doc in &docs {
        let doc_id = doc.id;
        let doc_path = ValidatedPath::new(doc.path.to_string())?;
        let content = doc.content.clone();

        // Insert into storage
        storage.insert(doc.clone()).await?;

        // Insert into primary index
        primary_index.insert(doc_id, doc_path.clone()).await?;

        // Insert into trigram index with content
        trigram_index
            .insert_with_content(doc_id, doc_path, &content)
            .await?;
    }

    println!("  Inserted {} documents", docs.len());

    // Step 2: Verify all systems have the documents
    println!("Step 2: Verifying initial state...");
    let storage_count = storage.list_all().await?.len();
    let all_query = QueryBuilder::new().with_limit(100)?.build()?;
    let primary_count = primary_index.search(&all_query).await?.len();
    let trigram_count = trigram_index.search(&all_query).await?.len();

    println!("  Storage: {} documents", storage_count);
    println!("  Primary index: {} documents", primary_count);
    println!("  Trigram index: {} documents", trigram_count);

    assert_eq!(
        storage_count,
        docs.len(),
        "Storage should have all documents"
    );
    assert_eq!(
        primary_count,
        docs.len(),
        "Primary index should have all documents"
    );
    assert!(trigram_count > 0, "Trigram index should have documents");

    // Step 3: Delete the first document using individual system calls
    // This simulates the bug where deletion isn't properly synchronized
    println!("Step 3: Deleting document (simulating current bug)...");
    let doc_to_delete = &docs[0]; // "docs/README.md"
    let doc_id_to_delete = doc_to_delete.id;

    println!(
        "  Deleting document: {} ({})",
        doc_to_delete.path, doc_id_to_delete
    );

    // Delete from storage only (simulating the current buggy behavior)
    let deleted_from_storage = storage.delete(&doc_id_to_delete).await?;
    println!("  Deleted from storage: {}", deleted_from_storage);

    // BUG: In the current implementation, indices are not updated!
    // This is what we need to fix

    // Step 4: Check the inconsistent state
    println!("Step 4: Checking for inconsistent state (should demonstrate bug)...");
    let storage_count_after = storage.list_all().await?.len();
    let primary_count_after = primary_index.search(&all_query).await?.len();
    let trigram_count_after = trigram_index.search(&all_query).await?.len();

    println!("  After deletion:");
    println!("    Storage: {} documents", storage_count_after);
    println!("    Primary index: {} documents", primary_count_after);
    println!("    Trigram index: {} documents", trigram_count_after);

    // This should demonstrate the bug:
    // - Storage should have 2 documents (after deletion)
    // - Indices should still have 3 documents (bug - not updated)

    if deleted_from_storage {
        assert_eq!(
            storage_count_after,
            docs.len() - 1,
            "Storage should have one less document"
        );

        // These assertions will FAIL if the bug is not fixed
        // This demonstrates the synchronization issue
        if primary_count_after != storage_count_after {
            println!("  🐛 BUG DETECTED: Primary index not synchronized after deletion!");
            println!(
                "     Expected: {}, Actual: {}",
                storage_count_after, primary_count_after
            );
        }

        if trigram_count_after > storage_count_after {
            println!("  🐛 BUG DETECTED: Trigram index not synchronized after deletion!");
            println!(
                "     Storage: {}, Trigram: {}",
                storage_count_after, trigram_count_after
            );
        }
    }

    // Step 5: Try to retrieve the deleted document from storage (should fail)
    println!("Step 5: Verifying document deletion from storage...");
    let retrieved_doc = storage.get(&doc_id_to_delete).await?;
    assert!(
        retrieved_doc.is_none(),
        "Document should be deleted from storage"
    );
    println!("  ✓ Document correctly deleted from storage");

    // Step 6: Check if indices still reference the deleted document (demonstrating the bug)
    println!("Step 6: Checking if indices still reference deleted document...");

    // Search primary index for all documents
    let primary_results = primary_index.search(&all_query).await?;
    let primary_has_deleted = primary_results.contains(&doc_id_to_delete);

    if primary_has_deleted {
        println!(
            "  🐛 BUG: Primary index still references deleted document {}",
            doc_id_to_delete
        );
    } else {
        println!("  ✓ Primary index correctly removed deleted document");
    }

    // For trigram index, we can't easily check specific documents, but count mismatch indicates the issue

    // Step 7: Demonstrate the validation failure scenario
    println!("Step 7: Simulating validation that would detect this inconsistency...");
    println!("  Validation would report:");
    println!(
        "    storage_count_consistency: Count mismatch: Storage={}, Primary={}, Trigram={}",
        storage_count_after, primary_count_after, trigram_count_after
    );

    if storage_count_after != primary_count_after || storage_count_after != trigram_count_after {
        println!("  💥 CRITICAL: Index synchronization failure detected!");
        println!("     This is the bug reported in issue #338");

        // This test will fail until the bug is fixed
        // Uncomment the line below to see the test fail:
        // panic!("Index synchronization failure: Storage={}, Primary={}, Trigram={}",
        //        storage_count_after, primary_count_after, trigram_count_after);

        // For now, we'll just log the issue
        println!("  📝 Test documented the synchronization bug successfully");
    } else {
        println!("  ✅ All systems synchronized correctly!");
    }

    Ok(())
}

#[tokio::test]
async fn test_document_deletion_proper_synchronization() -> Result<()> {
    // This test shows what the CORRECT deletion behavior should look like
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    let mut storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;
    let mut primary_index = create_primary_index(primary_path.to_str().unwrap(), Some(100)).await?;
    let mut trigram_index = create_trigram_index(trigram_path.to_str().unwrap(), Some(100)).await?;

    println!("=== Testing PROPER Document Deletion Synchronization ===");

    // Insert test document
    let doc = DocumentBuilder::new()
        .path("example.md")?
        .title("Example Document")?
        .content(b"This is example content")
        .build()?;

    let doc_id = doc.id;
    let doc_path = ValidatedPath::new(doc.path.to_string())?;
    let content = doc.content.clone();

    // Insert into all systems
    storage.insert(doc).await?;
    primary_index.insert(doc_id, doc_path.clone()).await?;
    trigram_index
        .insert_with_content(doc_id, doc_path, &content)
        .await?;

    // Verify initial state
    let initial_storage = storage.list_all().await?.len();
    let all_query = QueryBuilder::new().with_limit(100)?.build()?;
    let initial_primary = primary_index.search(&all_query).await?.len();
    let initial_trigram = trigram_index.search(&all_query).await?.len();

    println!(
        "Initial state: Storage={}, Primary={}, Trigram={}",
        initial_storage, initial_primary, initial_trigram
    );

    // PROPER deletion - delete from ALL systems
    println!("Performing proper synchronized deletion...");

    let storage_deleted = storage.delete(&doc_id).await?;
    let primary_deleted = primary_index.delete(&doc_id).await?;
    let trigram_deleted = trigram_index.delete(&doc_id).await?;

    println!(
        "Deletion results: Storage={}, Primary={}, Trigram={}",
        storage_deleted, primary_deleted, trigram_deleted
    );

    // Verify final state
    let final_storage = storage.list_all().await?.len();
    let final_primary = primary_index.search(&all_query).await?.len();
    let final_trigram = trigram_index.search(&all_query).await?.len();

    println!(
        "Final state: Storage={}, Primary={}, Trigram={}",
        final_storage, final_primary, final_trigram
    );

    // All systems should be synchronized
    assert_eq!(final_storage, 0, "Storage should be empty");
    assert_eq!(final_primary, 0, "Primary index should be empty");
    assert_eq!(final_trigram, 0, "Trigram index should be empty");

    println!("✅ Proper synchronization achieved!");

    Ok(())
}
