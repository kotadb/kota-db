#![allow(clippy::uninlined_format_args)]
// Standalone test for B+ tree deletion
// Run with: rustc test_btree_deletion.rs -L target/debug/deps && ./test_btree_deletion

extern crate kotadb;

use kotadb::{btree, ValidatedDocumentId, ValidatedPath};
use uuid::Uuid;

fn main() {
    println!("Testing B+ tree deletion algorithm...\n");

    // Test 1: Simple deletion
    println!("Test 1: Simple deletion");
    let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
    let path = ValidatedPath::new("/test/delete_me.md").unwrap();

    let mut tree = btree::create_empty_tree();
    tree = btree::insert_into_tree(tree, doc_id, path).unwrap();

    assert!(btree::search_in_tree(&tree, &doc_id).is_some());
    println!("  ✓ Key inserted successfully");

    tree = btree::delete_from_tree(tree, &doc_id).unwrap();

    assert!(btree::search_in_tree(&tree, &doc_id).is_none());
    assert!(btree::is_valid_btree(&tree));
    println!("  ✓ Key deleted successfully");

    // Test 2: Delete from multiple keys
    println!("\nTest 2: Delete from multiple keys");
    let keys: Vec<_> = (0..5)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();

    let mut tree = btree::create_empty_tree();
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(format!("/test/doc{i}.md")).unwrap();
        tree = btree::insert_into_tree(tree, *key, path).unwrap();
    }

    assert_eq!(kotadb::count_entries(&tree), 5);
    println!("  ✓ Inserted 5 keys");

    // Delete middle key
    tree = btree::delete_from_tree(tree, &keys[2]).unwrap();

    assert!(btree::search_in_tree(&tree, &keys[2]).is_none());
    assert_eq!(kotadb::count_entries(&tree), 4);
    assert!(btree::is_valid_btree(&tree));
    println!("  ✓ Deleted middle key successfully");

    // Test 3: Delete non-existent key
    println!("\nTest 3: Delete non-existent key");
    let non_existent = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
    let count_before = kotadb::count_entries(&tree);

    tree = btree::delete_from_tree(tree, &non_existent).unwrap();

    assert_eq!(kotadb::count_entries(&tree), count_before);
    println!("  ✓ Tree unchanged after deleting non-existent key");

    // Test 4: Performance with larger tree
    println!("\nTest 4: Performance with larger tree");
    let mut tree = btree::create_empty_tree();
    let large_keys: Vec<_> = (0..100)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();

    for (i, key) in large_keys.iter().enumerate() {
        let path = ValidatedPath::new(format!("/test/large{i}.md")).unwrap();
        tree = btree::insert_into_tree(tree, *key, path).unwrap();
    }

    println!("  ✓ Created tree with 100 keys");

    // Delete 20 random keys
    let start = std::time::Instant::now();
    for i in (0..20).step_by(5) {
        tree = btree::delete_from_tree(tree, &large_keys[i]).unwrap();
    }
    let duration = start.elapsed();

    assert_eq!(kotadb::count_entries(&tree), 96);
    assert!(btree::is_valid_btree(&tree));
    assert!(btree::all_leaves_at_same_level(&tree));
    println!("  ✓ Deleted 4 keys in {duration:?}");
    println!("  ✓ Tree remains valid and balanced");

    println!("\n✅ All tests passed!");
}
