// Index Operation Tests - Stage 1: TDD
// These tests define the expected behavior of all index types
// Written BEFORE implementation following 6-stage risk reduction

use anyhow::Result;
use kotadb::*;
use std::collections::HashSet;
use tempfile::TempDir;
use uuid::Uuid;

// Test helper
fn temp_indices() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_string_lossy().to_string();
    (dir, path)
}

// ===== B+ Tree Primary Index Tests =====

#[tokio::test]
async fn test_btree_insert_and_lookup() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = BTreeIndex::new(&path).await?;

    // Insert path -> document ID mappings
    let doc_id1 = Uuid::new_v4();
    let doc_id2 = Uuid::new_v4();
    let doc_id3 = Uuid::new_v4();

    index.insert("/projects/kota/README.md", doc_id1).await?;
    index.insert("/personal/notes.md", doc_id2).await?;
    index.insert("/businesses/cogzia/plan.md", doc_id3).await?;

    // Exact lookups should work
    assert_eq!(index.get("/projects/kota/README.md").await?, Some(doc_id1));
    assert_eq!(index.get("/personal/notes.md").await?, Some(doc_id2));
    assert_eq!(
        index.get("/businesses/cogzia/plan.md").await?,
        Some(doc_id3)
    );

    // Non-existent paths should return None
    assert_eq!(index.get("/missing/file.md").await?, None);

    Ok(())
}

#[tokio::test]
async fn test_btree_range_queries() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = BTreeIndex::new(&path).await?;

    // Insert documents with paths that can be range-queried
    let docs = vec![
        ("/a/1.md", Uuid::new_v4()),
        ("/a/2.md", Uuid::new_v4()),
        ("/b/1.md", Uuid::new_v4()),
        ("/b/2.md", Uuid::new_v4()),
        ("/c/1.md", Uuid::new_v4()),
    ];

    for (path, id) in &docs {
        index.insert(path, *id).await?;
    }

    // Range query: all paths starting with "/b/"
    let range = index.range("/b/", "/b0").await?;
    assert_eq!(range.len(), 2);
    assert!(range.contains(&docs[2].1));
    assert!(range.contains(&docs[3].1));

    // Range query: all paths between "/a/" and "/c/"
    let range = index.range("/a/", "/c/").await?;
    assert_eq!(range.len(), 4); // Excludes /c/1.md

    Ok(())
}

#[tokio::test]
async fn test_btree_delete() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = BTreeIndex::new(&path).await?;

    let doc_id = Uuid::new_v4();
    let path_str = "/test/delete.md";

    // Insert then delete
    index.insert(path_str, doc_id).await?;
    assert_eq!(index.get(path_str).await?, Some(doc_id));

    index.delete(path_str).await?;
    assert_eq!(index.get(path_str).await?, None);

    // Delete non-existent should not error
    index.delete("/non/existent.md").await?;

    Ok(())
}

// ===== Trigram Full-Text Index Tests =====

#[tokio::test]
async fn test_trigram_extraction() -> Result<()> {
    init_logging()?;

    // Test pure trigram extraction function
    let trigrams = extract_trigrams("hello world");
    let expected = vec![
        "hel", "ell", "llo", "lo ", "o w", " wo", "wor", "orl", "rld",
    ];

    assert_eq!(trigrams.len(), expected.len());
    for expected_trigram in expected {
        assert!(trigrams.contains(&expected_trigram.as_bytes().try_into().unwrap()));
    }

    // Edge cases
    assert_eq!(extract_trigrams("hi").len(), 0); // Too short
    assert_eq!(extract_trigrams("abc").len(), 1); // Exactly 3 chars
    assert_eq!(extract_trigrams("").len(), 0); // Empty

    Ok(())
}

#[tokio::test]
async fn test_trigram_index_and_search() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TrigramIndex::new(&path).await?;

    // Index some documents
    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();

    index
        .index_document(doc1, "The quick brown fox jumps over the lazy dog")
        .await?;
    index
        .index_document(doc2, "Rust programming language is fast and safe")
        .await?;
    index
        .index_document(doc3, "KOTA is a knowledge-oriented thinking assistant")
        .await?;

    // Search for exact matches
    let results = index.search("quick brown").await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].doc_id, doc1);

    // Search for partial matches
    let results = index.search("Rust").await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].doc_id, doc2);

    // Search across multiple documents
    let results = index.search("is").await?;
    assert!(results.len() >= 2); // Both doc2 and doc3 contain "is"

    Ok(())
}

#[tokio::test]
async fn test_trigram_fuzzy_search() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TrigramIndex::new(&path).await?;

    // Index documents
    let doc1 = Uuid::new_v4();
    index
        .index_document(doc1, "consciousness implementation details")
        .await?;

    // Fuzzy search with typos
    let results = index.search_fuzzy("conciousness", 2).await?; // Missing 's'
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].doc_id, doc1);

    let results = index.search_fuzzy("implementaton", 2).await?; // Missing 'i'
    assert_eq!(results.len(), 1);

    // Too many typos should not match
    let results = index.search_fuzzy("xonxiousness", 2).await?; // 3 typos
    assert_eq!(results.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_trigram_relevance_scoring() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TrigramIndex::new(&path).await?;

    // Index documents with different relevance
    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();

    // Doc1 has "rust" multiple times
    index
        .index_document(doc1, "Rust is great. I love Rust. Rust is fast.")
        .await?;
    // Doc2 has "rust" once
    index
        .index_document(doc2, "Rust programming language")
        .await?;
    // Doc3 mentions it in passing
    index
        .index_document(doc3, "Languages like Python, Java, and Rust")
        .await?;

    let results = index.search("Rust").await?;
    assert_eq!(results.len(), 3);

    // Doc1 should score highest (most occurrences)
    assert_eq!(results[0].doc_id, doc1);
    assert!(results[0].score > results[1].score);
    assert!(results[1].score > results[2].score);

    Ok(())
}

// ===== Tag Inverted Index Tests =====

#[tokio::test]
async fn test_tag_index_operations() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TagIndex::new(&path).await?;

    // Add documents with tags
    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();

    index
        .add_tags(doc1, &["rust", "programming", "systems"])
        .await?;
    index.add_tags(doc2, &["rust", "web", "async"]).await?;
    index.add_tags(doc3, &["python", "web", "ml"]).await?;

    // Search by single tag
    let results = index.search_tags(&["rust"]).await?;
    assert_eq!(results.len(), 2);
    assert!(results.contains(&doc1));
    assert!(results.contains(&doc2));

    // Search by multiple tags (OR)
    let results = index.search_tags(&["rust", "python"]).await?;
    assert_eq!(results.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_tag_intersection_queries() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TagIndex::new(&path).await?;

    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();

    index.add_tags(doc1, &["rust", "async", "web"]).await?;
    index.add_tags(doc2, &["rust", "systems"]).await?;
    index.add_tags(doc3, &["rust", "async", "embedded"]).await?;

    // AND query: documents with both "rust" AND "async"
    let results = index.search_tags_all(&["rust", "async"]).await?;
    assert_eq!(results.len(), 2);
    assert!(results.contains(&doc1));
    assert!(results.contains(&doc3));
    assert!(!results.contains(&doc2));

    // AND query with no matches
    let results = index.search_tags_all(&["rust", "python"]).await?;
    assert_eq!(results.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_tag_removal() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = TagIndex::new(&path).await?;

    let doc_id = Uuid::new_v4();

    // Add tags
    index.add_tags(doc_id, &["rust", "async", "web"]).await?;
    let results = index.search_tags(&["rust"]).await?;
    assert_eq!(results.len(), 1);

    // Remove some tags
    index.remove_tags(doc_id, &["async", "web"]).await?;

    // Should still find by remaining tag
    let results = index.search_tags(&["rust"]).await?;
    assert_eq!(results.len(), 1);

    // Should not find by removed tags
    let results = index.search_tags(&["async"]).await?;
    assert_eq!(results.len(), 0);

    Ok(())
}

// ===== Graph/Relationship Index Tests =====

#[tokio::test]
async fn test_graph_index_relationships() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = GraphIndex::new(&path).await?;

    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();
    let doc4 = Uuid::new_v4();

    // Create relationships: doc1 -> doc2 -> doc3, doc1 -> doc4
    index.add_edge(doc1, doc2, EdgeType::Related).await?;
    index.add_edge(doc2, doc3, EdgeType::Related).await?;
    index.add_edge(doc1, doc4, EdgeType::References).await?;

    // Get direct relationships (depth 1)
    let related = index.get_related(doc1, 1).await?;
    assert_eq!(related.len(), 2);
    assert!(related.contains(&doc2));
    assert!(related.contains(&doc4));

    // Get relationships with depth 2
    let related = index.get_related(doc1, 2).await?;
    assert_eq!(related.len(), 3);
    assert!(related.contains(&doc2));
    assert!(related.contains(&doc3));
    assert!(related.contains(&doc4));

    // Backward relationships
    let referencing = index.get_referencing(doc2).await?;
    assert_eq!(referencing.len(), 1);
    assert!(referencing.contains(&doc1));

    Ok(())
}

#[tokio::test]
async fn test_graph_cycle_detection() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();
    let mut index = GraphIndex::new(&path).await?;

    let doc1 = Uuid::new_v4();
    let doc2 = Uuid::new_v4();
    let doc3 = Uuid::new_v4();

    // Create a cycle: doc1 -> doc2 -> doc3 -> doc1
    index.add_edge(doc1, doc2, EdgeType::Related).await?;
    index.add_edge(doc2, doc3, EdgeType::Related).await?;
    index.add_edge(doc3, doc1, EdgeType::Related).await?;

    // Should handle cycles gracefully
    let related = index.get_related(doc1, 10).await?; // High depth
    assert_eq!(related.len(), 2); // Should only return doc2 and doc3, not repeat

    Ok(())
}

// ===== Index Persistence Tests =====

#[tokio::test]
async fn test_index_persistence() -> Result<()> {
    init_logging()?;
    let (_dir, path) = temp_indices();

    let doc_id = Uuid::new_v4();

    // Create and populate indices
    {
        let mut btree = BTreeIndex::new(&path).await?;
        let mut trigram = TrigramIndex::new(&path).await?;
        let mut tags = TagIndex::new(&path).await?;

        btree.insert("/test/persist.md", doc_id).await?;
        trigram
            .index_document(doc_id, "persistent content for testing")
            .await?;
        tags.add_tags(doc_id, &["test", "persistence"]).await?;

        // Force flush
        btree.flush().await?;
        trigram.flush().await?;
        tags.flush().await?;
    }

    // Reopen and verify
    {
        let btree = BTreeIndex::new(&path).await?;
        let trigram = TrigramIndex::new(&path).await?;
        let tags = TagIndex::new(&path).await?;

        assert_eq!(btree.get("/test/persist.md").await?, Some(doc_id));
        assert!(!trigram.search("persistent").await?.is_empty());
        assert!(tags.search_tags(&["test"]).await?.contains(&doc_id));
    }

    Ok(())
}

// Helper types that will be implemented later

fn extract_trigrams(text: &str) -> Vec<[u8; 3]> {
    todo!("Implement in Stage 3 - Pure Functions")
}

struct BTreeIndex;
impl BTreeIndex {
    async fn new(_path: &str) -> Result<Self> {
        todo!()
    }
    async fn insert(&mut self, _path: &str, _id: Uuid) -> Result<()> {
        todo!()
    }
    async fn get(&self, _path: &str) -> Result<Option<Uuid>> {
        todo!()
    }
    async fn delete(&mut self, _path: &str) -> Result<()> {
        todo!()
    }
    async fn range(&self, _start: &str, _end: &str) -> Result<Vec<Uuid>> {
        todo!()
    }
    async fn flush(&mut self) -> Result<()> {
        todo!()
    }
}

struct TrigramIndex;
impl TrigramIndex {
    async fn new(_path: &str) -> Result<Self> {
        todo!()
    }
    async fn index_document(&mut self, _id: Uuid, _content: &str) -> Result<()> {
        todo!()
    }
    async fn search(&self, _query: &str) -> Result<Vec<SearchResult>> {
        todo!()
    }
    async fn search_fuzzy(&self, _query: &str, _distance: u32) -> Result<Vec<SearchResult>> {
        todo!()
    }
    async fn flush(&mut self) -> Result<()> {
        todo!()
    }
}

struct TagIndex;
impl TagIndex {
    async fn new(_path: &str) -> Result<Self> {
        todo!()
    }
    async fn add_tags(&mut self, _id: Uuid, _tags: &[&str]) -> Result<()> {
        todo!()
    }
    async fn remove_tags(&mut self, _id: Uuid, _tags: &[&str]) -> Result<()> {
        todo!()
    }
    async fn search_tags(&self, _tags: &[&str]) -> Result<HashSet<Uuid>> {
        todo!()
    }
    async fn search_tags_all(&self, _tags: &[&str]) -> Result<HashSet<Uuid>> {
        todo!()
    }
    async fn flush(&mut self) -> Result<()> {
        todo!()
    }
}

struct GraphIndex;
impl GraphIndex {
    async fn new(_path: &str) -> Result<Self> {
        todo!()
    }
    async fn add_edge(&mut self, _from: Uuid, _to: Uuid, _edge_type: EdgeType) -> Result<()> {
        todo!()
    }
    async fn get_related(&self, _id: Uuid, _depth: u32) -> Result<HashSet<Uuid>> {
        todo!()
    }
    async fn get_referencing(&self, _id: Uuid) -> Result<HashSet<Uuid>> {
        todo!()
    }
}

#[derive(Debug)]
struct SearchResult {
    doc_id: Uuid,
    score: f32,
}

#[derive(Debug)]
enum EdgeType {
    Related,
    References,
}
