// Tests for Validated Types - Stage 6
// These tests ensure that our validated types enforce their invariants correctly

use kotadb::types::*;
use kotadb::types::state::*;
use anyhow::Result;
use uuid::Uuid;

#[test]
fn test_validated_path_accepts_valid_paths() {
    // Absolute paths
    assert!(ValidatedPath::new("/home/user/documents/file.md").is_ok());
    assert!(ValidatedPath::new("/var/log/kotadb/data.db").is_ok());
    
    // Relative paths
    assert!(ValidatedPath::new("documents/notes/todo.md").is_ok());
    assert!(ValidatedPath::new("./local/file.txt").is_ok());
    
    // Paths with spaces (properly handled)
    assert!(ValidatedPath::new("/Users/name/My Documents/file.md").is_ok());
}

#[test]
fn test_validated_path_rejects_invalid_paths() {
    // Empty path
    assert!(ValidatedPath::new("").is_err());
    
    // Path traversal
    assert!(ValidatedPath::new("../../../etc/passwd").is_err());
    assert!(ValidatedPath::new("/home/../../../etc/passwd").is_err());
    
    // Null bytes
    assert!(ValidatedPath::new("file\0name").is_err());
    
    // Windows reserved names
    assert!(ValidatedPath::new("CON").is_err());
    assert!(ValidatedPath::new("PRN.txt").is_err());
    assert!(ValidatedPath::new("AUX.md").is_err());
    assert!(ValidatedPath::new("COM1").is_err());
    assert!(ValidatedPath::new("LPT1.doc").is_err());
    
    // Extremely long paths
    let long_path = format!("/{}", "x".repeat(5000));
    assert!(ValidatedPath::new(long_path).is_err());
}

#[test]
fn test_validated_document_id() {
    // New ID should be valid
    let id1 = ValidatedDocumentId::new();
    let id2 = ValidatedDocumentId::new();
    assert_ne!(id1, id2); // Should be unique
    
    // From existing UUID
    let uuid = Uuid::new_v4();
    let id = ValidatedDocumentId::from_uuid(uuid).unwrap();
    assert_eq!(id.as_uuid(), uuid);
    
    // Nil UUID should fail
    assert!(ValidatedDocumentId::from_uuid(Uuid::nil()).is_err());
}

#[test]
fn test_validated_title() {
    // Valid titles
    assert!(ValidatedTitle::new("My Document").is_ok());
    assert!(ValidatedTitle::new("  Spaces get trimmed  ").is_ok());
    
    let title = ValidatedTitle::new("  Trimmed  ").unwrap();
    assert_eq!(title.as_str(), "Trimmed");
    
    // Empty or whitespace-only titles fail
    assert!(ValidatedTitle::new("").is_err());
    assert!(ValidatedTitle::new("   ").is_err());
    assert!(ValidatedTitle::new("\t\n").is_err());
    
    // Title too long
    let long_title = "x".repeat(2000);
    assert!(ValidatedTitle::new(long_title).is_err());
    
    // Title at max length succeeds
    let max_title = "x".repeat(1024);
    assert!(ValidatedTitle::new(max_title).is_ok());
}

#[test]
fn test_non_zero_size() {
    assert!(NonZeroSize::new(1).is_ok());
    assert!(NonZeroSize::new(1024).is_ok());
    assert!(NonZeroSize::new(u64::MAX).is_ok());
    
    // Zero fails
    assert!(NonZeroSize::new(0).is_err());
    
    // Can get inner value
    let size = NonZeroSize::new(42).unwrap();
    assert_eq!(size.get(), 42);
}

#[test]
fn test_validated_timestamp() {
    // Valid timestamps
    assert!(ValidatedTimestamp::new(1).is_ok());
    assert!(ValidatedTimestamp::new(1_000_000_000).is_ok()); // Year 2001
    assert!(ValidatedTimestamp::new(1_700_000_000).is_ok()); // Year 2023
    
    // Invalid timestamps
    assert!(ValidatedTimestamp::new(0).is_err()); // Epoch itself
    assert!(ValidatedTimestamp::new(-1).is_err()); // Before epoch
    assert!(ValidatedTimestamp::new(33_000_000_000).is_err()); // Too far future
    
    // Current time should always be valid
    let now = ValidatedTimestamp::now();
    assert!(now.as_secs() > 0);
}

#[test]
fn test_timestamp_pair() {
    let created = ValidatedTimestamp::new(1000).unwrap();
    let updated = ValidatedTimestamp::new(2000).unwrap();
    
    // Valid pair (updated >= created)
    let pair = TimestampPair::new(created, updated);
    assert!(pair.is_ok());
    
    let pair = pair.unwrap();
    assert_eq!(pair.created().as_secs(), 1000);
    assert_eq!(pair.updated().as_secs(), 2000);
    
    // Invalid pair (updated < created)
    let pair = TimestampPair::new(updated, created);
    assert!(pair.is_err());
    
    // Same timestamps are valid
    let pair = TimestampPair::new(created, created);
    assert!(pair.is_ok());
    
    // Touch updates the updated timestamp
    let mut pair = TimestampPair::now();
    let original_updated = pair.updated();
    std::thread::sleep(std::time::Duration::from_millis(10));
    pair.touch();
    assert!(pair.updated().as_secs() >= original_updated.as_secs());
}

#[test]
fn test_validated_tag() {
    // Valid tags
    assert!(ValidatedTag::new("rust").is_ok());
    assert!(ValidatedTag::new("rust-lang").is_ok());
    assert!(ValidatedTag::new("rust_programming").is_ok());
    assert!(ValidatedTag::new("Rust 2024").is_ok());
    assert!(ValidatedTag::new("my-awesome-tag_123").is_ok());
    
    // Invalid tags
    assert!(ValidatedTag::new("").is_err());
    assert!(ValidatedTag::new("   ").is_err());
    assert!(ValidatedTag::new("tag@with#special").is_err());
    assert!(ValidatedTag::new("../../etc/passwd").is_err());
    
    // Tag too long
    let long_tag = "x".repeat(200);
    assert!(ValidatedTag::new(long_tag).is_err());
}

#[test]
fn test_validated_search_query() {
    // Valid queries
    assert!(ValidatedSearchQuery::new("search term", 3).is_ok());
    assert!(ValidatedSearchQuery::new("  trimmed query  ", 5).is_ok());
    
    let query = ValidatedSearchQuery::new("  test  ", 3).unwrap();
    assert_eq!(query.as_str(), "test");
    
    // Empty query
    assert!(ValidatedSearchQuery::new("", 3).is_err());
    assert!(ValidatedSearchQuery::new("   ", 3).is_err());
    
    // Too short for minimum
    assert!(ValidatedSearchQuery::new("ab", 3).is_err());
    assert!(ValidatedSearchQuery::new("ab", 2).is_ok());
    
    // Too long
    let long_query = "x".repeat(2000);
    assert!(ValidatedSearchQuery::new(long_query, 3).is_err());
}

#[test]
fn test_validated_page_id() {
    // Valid page IDs
    assert!(ValidatedPageId::new(1).is_ok());
    assert!(ValidatedPageId::new(1000).is_ok());
    assert!(ValidatedPageId::new(1_000_000).is_ok());
    
    // Invalid page IDs
    assert!(ValidatedPageId::new(0).is_err()); // Zero is reserved
    
    // Can get inner value
    let page_id = ValidatedPageId::new(42).unwrap();
    assert_eq!(page_id.get(), 42);
}

#[test]
fn test_validated_limit() {
    // Valid limits
    assert!(ValidatedLimit::new(1, 100).is_ok());
    assert!(ValidatedLimit::new(50, 100).is_ok());
    assert!(ValidatedLimit::new(100, 100).is_ok());
    
    // Invalid limits
    assert!(ValidatedLimit::new(0, 100).is_err()); // Zero
    assert!(ValidatedLimit::new(101, 100).is_err()); // Exceeds max
    
    let limit = ValidatedLimit::new(25, 100).unwrap();
    assert_eq!(limit.get(), 25);
    assert_eq!(limit.max(), 100);
}

#[test]
fn test_document_state_machine() {
    // Create a draft document
    let draft = TypedDocument::<Draft>::new(
        ValidatedPath::new("/test/doc.md").unwrap(),
        [0u8; 32],
        NonZeroSize::new(1024).unwrap(),
        ValidatedTitle::new("Test Document").unwrap(),
        100,
    );
    
    // Check initial state
    assert_eq!(draft.path.as_str(), "/test/doc.md");
    assert_eq!(draft.title.as_str(), "Test Document");
    assert_eq!(draft.size.get(), 1024);
    assert_eq!(draft.word_count, 100);
    
    // Transition to persisted
    let persisted = draft.into_persisted();
    
    // Transition to modified
    let modified = persisted.into_modified();
    
    // Updated timestamp should be newer
    assert!(modified.timestamps.updated().as_secs() >= 
            modified.timestamps.created().as_secs());
    
    // Back to persisted
    let _persisted_again = modified.into_persisted();
    
    // Note: The following would not compile due to type safety:
    // let bad = draft.into_modified(); // Error: Draft cannot transition to Modified
}

#[test]
fn test_type_conversions() {
    // ValidatedPath to String
    let path = ValidatedPath::new("/test/file.md").unwrap();
    assert_eq!(path.to_string(), "/test/file.md");
    assert_eq!(path.as_str(), "/test/file.md");
    
    // ValidatedTitle to String
    let title = ValidatedTitle::new("My Title").unwrap();
    assert_eq!(title.to_string(), "My Title");
    
    // ValidatedTag to String
    let tag = ValidatedTag::new("rust-lang").unwrap();
    assert_eq!(tag.to_string(), "rust-lang");
    
    // ValidatedDocumentId to String
    let id = ValidatedDocumentId::new();
    let id_string = id.to_string();
    assert!(Uuid::parse_str(&id_string).is_ok());
}

#[test]
fn test_equality_and_hashing() {
    use std::collections::HashSet;
    
    // ValidatedPath equality and hashing
    let path1 = ValidatedPath::new("/test/file.md").unwrap();
    let path2 = ValidatedPath::new("/test/file.md").unwrap();
    let path3 = ValidatedPath::new("/test/other.md").unwrap();
    
    assert_eq!(path1, path2);
    assert_ne!(path1, path3);
    
    let mut set = HashSet::new();
    set.insert(path1.clone());
    assert!(set.contains(&path2));
    assert!(!set.contains(&path3));
    
    // ValidatedTag equality and hashing
    let tag1 = ValidatedTag::new("rust").unwrap();
    let tag2 = ValidatedTag::new("rust").unwrap();
    let tag3 = ValidatedTag::new("python").unwrap();
    
    assert_eq!(tag1, tag2);
    assert_ne!(tag1, tag3);
    
    let mut tag_set = HashSet::new();
    tag_set.insert(tag1.clone());
    assert!(tag_set.contains(&tag2));
    assert!(!tag_set.contains(&tag3));
}

#[test]
fn test_ordering() {
    // NonZeroSize ordering
    let size1 = NonZeroSize::new(100).unwrap();
    let size2 = NonZeroSize::new(200).unwrap();
    assert!(size1 < size2);
    
    // ValidatedTimestamp ordering
    let ts1 = ValidatedTimestamp::new(1000).unwrap();
    let ts2 = ValidatedTimestamp::new(2000).unwrap();
    assert!(ts1 < ts2);
    
    // ValidatedPageId ordering
    let page1 = ValidatedPageId::new(10).unwrap();
    let page2 = ValidatedPageId::new(20).unwrap();
    assert!(page1 < page2);
}