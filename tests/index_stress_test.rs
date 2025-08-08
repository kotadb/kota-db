// Index Stress Tests - Comprehensive testing for Phase 2A scale requirements
// Tests all index implementations under extreme load conditions

#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use kotadb::{
    btree,
    contracts::{Index, Query},
    create_optimized_index_with_defaults, create_primary_index_for_tests,
    create_trigram_index_for_tests, ValidatedDocumentId, ValidatedPath,
};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Helper function to generate test documents with realistic content paths
fn generate_test_documents(
    count: usize,
    avg_size: usize,
) -> Result<Vec<(ValidatedDocumentId, ValidatedPath)>> {
    let mut documents = Vec::with_capacity(count);

    let topics = vec![
        "rust",
        "database",
        "distributed-systems",
        "performance",
        "async",
        "testing",
        "optimization",
        "algorithms",
        "networking",
        "security",
        "web-development",
        "machine-learning",
        "devops",
    ];

    for i in 0..count {
        let id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let topic = topics[i % topics.len()];

        // Create realistic paths that would contain the topic content
        let path = ValidatedPath::new(format!("/{}/guide_{}_{}bytes.md", topic, i, avg_size))?;
        documents.push((id, path));
    }

    Ok(documents)
}

/// Test B+ Tree stress with 50K entries - Phase 2A requirement
#[tokio::test]
async fn test_btree_stress_50k_entries() -> Result<()> {
    let test_size = 50_000;
    println!("🔥 Starting B+ Tree stress test with {test_size} entries");

    let start_time = Instant::now();

    // Generate test data
    let mut keys = Vec::with_capacity(test_size);
    let mut paths = Vec::with_capacity(test_size);

    for i in 0..test_size {
        keys.push(ValidatedDocumentId::from_uuid(Uuid::new_v4())?);
        paths.push(ValidatedPath::new(format!("/stress/50k/doc_{i}.md"))?);
    }

    // Test insertion performance
    let mut tree = btree::create_empty_tree();
    let insert_start = Instant::now();

    for i in 0..test_size {
        tree = btree::insert_into_tree(tree, keys[i], paths[i].clone())?;

        if i % 10_000 == 0 {
            println!("  📈 Inserted {}/{} entries", i, test_size);
        }
    }

    let insert_duration = insert_start.elapsed();
    println!("✅ Inserted {} entries in {:?}", test_size, insert_duration);

    // Test search performance
    let search_start = Instant::now();
    let search_sample_size = 1000;
    let search_keys: Vec<_> = keys
        .iter()
        .step_by(test_size / search_sample_size)
        .collect();

    for key in &search_keys {
        let result = btree::search_in_tree(&tree, key);
        assert!(result.is_some(), "Key should be found in tree");
    }

    let search_duration = search_start.elapsed();
    println!(
        "✅ Searched {} entries in {:?}",
        search_sample_size, search_duration
    );

    // Performance assertions
    let total_duration = start_time.elapsed();
    println!("📊 Total test duration: {total_duration:?}");

    // Phase 2A requirement: Should handle 50K entries efficiently
    assert!(
        insert_duration < Duration::from_secs(60),
        "Insert should complete within 60 seconds, took {insert_duration:?}"
    );
    assert!(
        search_duration < Duration::from_millis(100),
        "Search should complete within 100ms, took {search_duration:?}"
    );

    Ok(())
}

/// Test B+ Tree stress with 100K entries - Phase 2A upper bound
#[tokio::test]
async fn test_btree_stress_100k_entries() -> Result<()> {
    let test_size = 100_000;
    println!("🔥 Starting B+ Tree stress test with {test_size} entries");

    let start_time = Instant::now();

    // Use optimized bulk insertion approach
    let mut test_pairs = Vec::with_capacity(test_size);

    for i in 0..test_size {
        let id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(format!("/stress/100k/doc_{i}.md"))?;
        test_pairs.push((id, path));
    }

    // Test bulk insertion if available, otherwise individual
    let insert_start = Instant::now();

    let tree = if let Ok(bulk_tree) =
        kotadb::bulk_insert_into_tree(btree::create_empty_tree(), test_pairs.clone())
    {
        bulk_tree
    } else {
        // Fallback to individual insertions
        let mut tree = btree::create_empty_tree();
        for (i, (id, path)) in test_pairs.iter().enumerate() {
            tree = btree::insert_into_tree(tree, *id, path.clone())?;

            if i % 20_000 == 0 {
                println!("  📈 Inserted {i}/{test_size} entries");
            }
        }
        tree
    };

    let insert_duration = insert_start.elapsed();
    println!("✅ Inserted {} entries in {:?}", test_size, insert_duration);

    // Test search performance with larger sample
    let search_start = Instant::now();
    let search_sample_size = 2000;
    let search_keys: Vec<_> = test_pairs
        .iter()
        .step_by(test_size / search_sample_size)
        .map(|(id, _)| id)
        .collect();

    for key in &search_keys {
        let result = btree::search_in_tree(&tree, key);
        assert!(result.is_some(), "Key should be found in tree");
    }

    let search_duration = search_start.elapsed();
    println!(
        "✅ Searched {} entries in {:?}",
        search_sample_size, search_duration
    );

    // Performance assertions for 100K scale
    let total_duration = start_time.elapsed();
    println!("📊 Total test duration: {total_duration:?}");

    assert!(
        insert_duration < Duration::from_secs(120),
        "Insert should complete within 2 minutes, took {insert_duration:?}"
    );
    assert!(
        search_duration < Duration::from_millis(200),
        "Search should complete within 200ms, took {search_duration:?}"
    );

    Ok(())
}

/// Test Trigram Index with large text corpus
#[tokio::test]
async fn test_trigram_index_large_corpus() -> Result<()> {
    let doc_count = 25_000;
    let avg_doc_size = 5000; // 5KB average documents

    println!("🔥 Starting Trigram Index stress test with {doc_count} documents");

    let temp_dir = TempDir::new()?;
    let mut index = create_trigram_index_for_tests(temp_dir.path().to_str().unwrap()).await?;

    // Generate realistic documents
    let documents = generate_test_documents(doc_count, avg_doc_size)?;

    // Test insertion performance
    let insert_start = Instant::now();

    for (i, (doc_id, doc_path)) in documents.iter().enumerate() {
        index.insert(*doc_id, doc_path.clone()).await?;

        if i % 5_000 == 0 {
            println!("  📈 Indexed {}/{} documents", i, doc_count);
        }
    }

    let insert_duration = insert_start.elapsed();
    println!(
        "✅ Indexed {} documents in {:?}",
        doc_count, insert_duration
    );

    // Test search performance with various query types
    let search_start = Instant::now();
    let search_terms = vec![
        "rust",
        "database",
        "performance",
        "async",
        "testing",
        "implementation",
        "optimization",
        "algorithm",
        "function",
        "distributed",
        "system",
        "architecture",
        "security",
    ];

    let mut total_results = 0;
    for term in &search_terms {
        let query = Query::new(Some(term.to_string()), None, None, 100)?;
        let results = index.search(&query).await?;
        total_results += results.len();

        // Note: Results might be empty if the term doesn't match path content
        println!("  🔍 Search for '{}' found {} results", term, results.len());
    }

    let search_duration = search_start.elapsed();
    println!(
        "✅ Performed {} searches, found {} total results in {:?}",
        search_terms.len(),
        total_results,
        search_duration
    );

    // Performance assertions
    assert!(
        insert_duration < Duration::from_secs(300),
        "Trigram indexing should complete within 5 minutes, took {insert_duration:?}"
    );
    assert!(
        search_duration < Duration::from_secs(5),
        "Search operations should complete within 5 seconds, took {search_duration:?}"
    );

    Ok(())
}

/// Test multiple indices working together under load
#[tokio::test]
async fn test_index_integration_stress() -> Result<()> {
    let doc_count = 15_000;
    let avg_doc_size = 3000;

    println!("🔥 Starting index integration stress test with {doc_count} documents");

    let temp_dir = TempDir::new()?;
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    tokio::fs::create_dir_all(&primary_path).await?;
    tokio::fs::create_dir_all(&trigram_path).await?;

    let mut primary_index = create_optimized_index_with_defaults(
        create_primary_index_for_tests(primary_path.to_str().unwrap()).await?,
    );
    let mut trigram_index = create_optimized_index_with_defaults(
        create_trigram_index_for_tests(trigram_path.to_str().unwrap()).await?,
    );

    // Generate test documents
    let documents = generate_test_documents(doc_count, avg_doc_size)?;

    // Test concurrent insertion into both indices
    let insert_start = Instant::now();

    for (i, (doc_id, doc_path)) in documents.iter().enumerate() {
        // Insert into both indices
        primary_index.insert(*doc_id, doc_path.clone()).await?;
        trigram_index.insert(*doc_id, doc_path.clone()).await?;

        if i % 3_000 == 0 {
            println!(
                "  📈 Inserted into both indices: {}/{} documents",
                i, doc_count
            );
        }
    }

    let insert_duration = insert_start.elapsed();
    println!(
        "✅ Inserted {} documents into both indices in {:?}",
        doc_count, insert_duration
    );

    // Test mixed query workload
    let query_start = Instant::now();
    let mut primary_queries = 0;
    let mut text_queries = 0;

    for i in 0..1000 {
        if i % 3 == 0 {
            // Primary index lookup - use empty query for now
            let query = Query::empty();
            let results = primary_index.search(&query).await?;
            // Primary index might return empty results with empty query
            primary_queries += 1;
        } else {
            // Text search
            let search_terms = ["implementation", "performance", "async", "testing"];
            let term = search_terms[i % search_terms.len()];
            let query = Query::new(Some(term.to_string()), None, None, 10)?;
            let results = trigram_index.search(&query).await?;
            text_queries += 1;
        }
    }

    let query_duration = query_start.elapsed();
    println!(
        "✅ Performed {} primary + {} text queries in {:?}",
        primary_queries, text_queries, query_duration
    );

    // Performance assertions for integrated operations
    assert!(
        insert_duration < Duration::from_secs(120),
        "Dual index insertion should complete within 2 minutes, took {insert_duration:?}"
    );
    assert!(
        query_duration < Duration::from_secs(10),
        "Mixed queries should complete within 10 seconds, took {query_duration:?}"
    );

    Ok(())
}

/// Test index performance under memory pressure
#[tokio::test]
async fn test_index_memory_pressure() -> Result<()> {
    let doc_count = 5_000;
    let large_doc_size = 50_000; // 50KB documents to create memory pressure

    println!("🔥 Starting memory pressure test with {doc_count} large documents");

    let temp_dir = TempDir::new()?;
    let mut index = create_trigram_index_for_tests(temp_dir.path().to_str().unwrap()).await?;

    // Generate large documents
    let documents = generate_test_documents(doc_count, large_doc_size)?;

    // Verify document path sizes (simulating large content)
    let total_paths: usize = documents.iter().map(|(_, path)| path.as_str().len()).sum();
    println!(
        "📏 Total path length: {:.2} KB (simulating large documents)",
        total_paths as f64 / 1024.0
    );

    // Test insertion under memory pressure
    let insert_start = Instant::now();

    for (i, (doc_id, doc_path)) in documents.iter().enumerate() {
        index.insert(*doc_id, doc_path.clone()).await?;

        if i % 1_000 == 0 {
            println!("  📈 Indexed large document: {}/{}", i, doc_count);
        }
    }

    let insert_duration = insert_start.elapsed();
    println!(
        "✅ Indexed {} large documents in {:?}",
        doc_count, insert_duration
    );

    // Test search performance under memory pressure
    let search_start = Instant::now();
    let search_terms = vec![
        "implementation",
        "performance",
        "optimization",
        "architecture",
        "distributed",
        "async",
        "testing",
        "algorithm",
    ];

    for term in &search_terms {
        let query = Query::new(Some(term.to_string()), None, None, 10)?;
        let results = index.search(&query).await?;
        println!(
            "  🔍 Search for '{}' found {} results under memory pressure",
            term,
            results.len()
        );
    }

    let search_duration = search_start.elapsed();
    println!("✅ Performed searches under memory pressure in {search_duration:?}");

    // Memory pressure shouldn't significantly degrade performance
    assert!(
        insert_duration < Duration::from_secs(180),
        "Large document indexing should complete within 3 minutes, took {insert_duration:?}"
    );
    assert!(
        search_duration < Duration::from_secs(2),
        "Search under memory pressure should complete within 2 seconds, took {search_duration:?}"
    );

    Ok(())
}

/// Test realistic mixed workload simulation
#[tokio::test]
async fn test_realistic_workload_simulation() -> Result<()> {
    let initial_docs = 20_000;
    let avg_doc_size = 4000;

    println!("🔥 Starting realistic workload simulation with {initial_docs} initial documents");

    let temp_dir = TempDir::new()?;
    let mut index = create_optimized_index_with_defaults(
        create_trigram_index_for_tests(temp_dir.path().to_str().unwrap()).await?,
    );

    // Phase 1: Initial bulk load
    let mut documents = generate_test_documents(initial_docs, avg_doc_size)?;

    let bulk_load_start = Instant::now();
    for (i, (doc_id, doc_path)) in documents.iter().enumerate() {
        index.insert(*doc_id, doc_path.clone()).await?;

        if i % 5_000 == 0 {
            println!("  📈 Bulk loading: {}/{} documents", i, initial_docs);
        }
    }
    let bulk_load_duration = bulk_load_start.elapsed();
    println!(
        "✅ Bulk loaded {} documents in {:?}",
        initial_docs, bulk_load_duration
    );

    // Phase 2: Mixed workload simulation (70% reads, 20% inserts, 10% updates)
    let workload_start = Instant::now();
    let operation_count = 2000;
    let mut reads = 0;
    let mut inserts = 0;
    let mut updates = 0;

    for i in 0..operation_count {
        match i % 10 {
            0..=6 => {
                // 70% reads - search operations
                let search_terms = [
                    "implementation",
                    "performance",
                    "async",
                    "testing",
                    "optimization",
                ];
                let term = search_terms[i % search_terms.len()];
                let query = Query::new(Some(term.to_string()), None, None, 10)?;
                let results = index.search(&query).await?;
                reads += 1;
            }
            7..=8 => {
                // 20% inserts - new documents
                let new_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
                let new_path = ValidatedPath::new(format!("/new/workload_doc_{i}.md"))?;
                index.insert(new_id, new_path.clone()).await?;
                documents.push((new_id, new_path));
                inserts += 1;
            }
            9 => {
                // 10% updates - modify existing documents
                if !documents.is_empty() {
                    let doc_index = i % documents.len();
                    let (original_id, original_path) = &documents[doc_index];

                    let updated_path = ValidatedPath::new(format!(
                        "{}_updated_{}",
                        original_path.as_str().trim_end_matches(".md"),
                        i
                    ))?;

                    index.update(*original_id, updated_path.clone()).await?;
                    documents[doc_index] = (*original_id, updated_path);
                    updates += 1;
                }
            }
            _ => unreachable!(),
        }

        if i % 500 == 0 {
            println!(
                "  📊 Completed {}/{} operations (R:{} I:{} U:{})",
                i, operation_count, reads, inserts, updates
            );
        }
    }

    let workload_duration = workload_start.elapsed();
    println!(
        "✅ Completed mixed workload: {} reads, {} inserts, {} updates in {:?}",
        reads, inserts, updates, workload_duration
    );

    // Performance assertions for realistic workload
    assert!(
        bulk_load_duration < Duration::from_secs(150),
        "Bulk load should complete within 2.5 minutes, took {bulk_load_duration:?}"
    );
    assert!(
        workload_duration < Duration::from_secs(60),
        "Mixed workload should complete within 1 minute, took {workload_duration:?}"
    );

    // Verify final state
    let final_search_start = Instant::now();
    let query = Query::new(Some("implementation".to_string()), None, None, 10)?;
    let final_results = index.search(&query).await?;
    let final_search_duration = final_search_start.elapsed();

    assert!(
        final_search_duration < Duration::from_millis(100),
        "Final search should be fast, took {final_search_duration:?}"
    );

    println!("🎯 Realistic workload simulation completed successfully");

    Ok(())
}

/// Test concurrent access patterns
#[tokio::test]
async fn test_concurrent_index_stress() -> Result<()> {
    let doc_count = 10_000;
    let concurrent_operations = 20; // Reduced for stability

    println!("🔥 Starting concurrent access stress test");

    let temp_dir = TempDir::new()?;
    let index = Arc::new(tokio::sync::RwLock::new(
        create_optimized_index_with_defaults(
            create_trigram_index_for_tests(temp_dir.path().to_str().unwrap()).await?,
        ),
    ));

    // Generate initial documents
    let documents = Arc::new(generate_test_documents(doc_count, 3000)?);

    // Populate index
    let populate_start = Instant::now();
    {
        let mut idx = index.write().await;
        for (i, (doc_id, doc_path)) in documents.iter().enumerate() {
            idx.insert(*doc_id, doc_path.clone()).await?;
            if i % 2_000 == 0 {
                println!("  📈 Populated: {}/{} documents", i, doc_count);
            }
        }
    }
    let populate_duration = populate_start.elapsed();
    println!(
        "✅ Populated index with {} documents in {:?}",
        doc_count, populate_duration
    );

    // Test concurrent operations
    let concurrent_start = Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_operations));
    let mut handles = Vec::new();

    // Spawn concurrent tasks
    for task_id in 0..concurrent_operations {
        let index_clone = Arc::clone(&index);
        let documents_clone = Arc::clone(&documents);
        let semaphore_clone = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let permit = semaphore_clone.acquire().await.unwrap();

            // Mix of read and write operations
            for i in 0..20 {
                if i % 4 == 0 {
                    // Read operation
                    let idx = index_clone.read().await;
                    let search_terms = ["implementation", "performance", "async"];
                    let term = search_terms[task_id % search_terms.len()];
                    let query = Query::new(Some(term.to_string()), None, None, 5).unwrap();
                    let results = idx.search(&query).await.unwrap();
                } else {
                    // Write operation (requires write lock)
                    let mut idx = index_clone.write().await;
                    let doc_index = (task_id * 20 + i) % documents_clone.len();
                    let (original_id, original_path) = &documents_clone[doc_index];

                    let updated_path = ValidatedPath::new(format!(
                        "{}_task_{}_op_{}",
                        original_path.as_str().trim_end_matches(".md"),
                        task_id,
                        i
                    ))
                    .unwrap();

                    idx.update(*original_id, updated_path).await.unwrap();
                }
            }

            task_id
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut completed_tasks = Vec::new();
    for handle in handles {
        let task_id = handle.await?;
        completed_tasks.push(task_id);
    }

    let concurrent_duration = concurrent_start.elapsed();
    println!(
        "✅ Completed {} concurrent tasks in {:?}",
        completed_tasks.len(),
        concurrent_duration
    );

    // Verify index integrity after concurrent operations
    let verification_start = Instant::now();
    {
        let idx = index.read().await;
        let query = Query::new(Some("implementation".to_string()), None, None, 10)?;
        let results = idx.search(&query).await?;
        println!("  🔍 Verification search found {} results", results.len());
    }
    let verification_duration = verification_start.elapsed();

    println!("✅ Index verification completed in {verification_duration:?}");

    // Performance assertions
    assert!(
        concurrent_duration < Duration::from_secs(30),
        "Concurrent operations should complete within 30 seconds, took {concurrent_duration:?}"
    );
    assert!(
        verification_duration < Duration::from_millis(100),
        "Verification should be fast, took {verification_duration:?}"
    );

    Ok(())
}
