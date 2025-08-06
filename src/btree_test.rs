// Module test for B+ tree implementation
#[cfg(test)]
mod tests {
    use crate::{btree, ValidatedDocumentId, ValidatedPath};
    use uuid::Uuid;

    #[test]
    fn test_btree_deletion_simple() {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap();
        let path = ValidatedPath::new("/test/delete.md").unwrap();

        let mut tree = btree::create_empty_tree();
        tree = btree::insert_into_tree(tree, doc_id, path).unwrap();

        assert!(btree::search_in_tree(&tree, &doc_id).is_some());

        tree = btree::delete_from_tree(tree, &doc_id).unwrap();

        assert!(btree::search_in_tree(&tree, &doc_id).is_none());
        assert!(btree::is_valid_btree(&tree));
    }

    #[test]
    fn test_btree_deletion_multiple() {
        let keys: Vec<_> = (0..10)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();

        let mut tree = btree::create_empty_tree();
        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("/test/doc{i}.md")).unwrap();
            tree = btree::insert_into_tree(tree, *key, path).unwrap();
        }

        assert_eq!(btree::count_total_keys(&tree), 10);

        // Delete several keys
        for i in [2, 5, 7] {
            tree = btree::delete_from_tree(tree, &keys[i]).unwrap();
        }

        assert_eq!(btree::count_total_keys(&tree), 7);
        assert!(btree::is_valid_btree(&tree));

        // Verify specific keys are gone
        assert!(btree::search_in_tree(&tree, &keys[2]).is_none());
        assert!(btree::search_in_tree(&tree, &keys[5]).is_none());
        assert!(btree::search_in_tree(&tree, &keys[7]).is_none());

        // Verify other keys still exist
        assert!(btree::search_in_tree(&tree, &keys[0]).is_some());
        assert!(btree::search_in_tree(&tree, &keys[1]).is_some());
        assert!(btree::search_in_tree(&tree, &keys[3]).is_some());
    }
}
