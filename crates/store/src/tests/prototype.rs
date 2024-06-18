use super::*;
use crate::config::StoreConfig;
use camino::Utf8PathBuf;
use tempdir::TempDir;

#[test]
fn test_gcounter_increment() {
    let mut counter = GCounter::new();
    counter.increment("node1".to_owned());
    counter.increment("node1".to_owned());
    assert_eq!(counter.value(), 2);
}

#[test]
fn test_gcounter_merge() {
    let mut counter1 = GCounter::new();
    counter1.increment("node1".to_owned()); // counter1: {"node1": 1}

    let mut counter2 = GCounter::new();
    counter2.increment("node1".to_owned()); // counter2: {"node1": 1}
    counter2.increment("node2".to_owned()); // counter2: {"node1": 1, "node2": 1}

    counter1.merge(&counter2); // counter1: {"node1": 1, "node2": 1}

    assert_eq!(counter1.value(), 2); // Expected value is 2 because {"node1": 1, "node2": 1}
}

#[test]
fn test_node_new() {
    let node = Node::new();
    assert_eq!(node.crdt.value(), 0);
    assert!(node.children.is_empty());
}

#[test]
fn test_hierarchical_crdt_increment() {
    let mut crdt = HierarchicalCRDT::new();
    crdt.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );
    assert_eq!(
        crdt.root
            .children
            .get("parent")
            .unwrap()
            .children
            .get("child")
            .unwrap()
            .crdt
            .value(),
        1
    );
}

#[test]
fn test_hierarchical_crdt_merge() {
    let mut crdt1 = HierarchicalCRDT::new();
    crdt1.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );

    let mut crdt2 = HierarchicalCRDT::new();
    crdt2.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node2".to_owned(),
    );

    crdt1.merge(&crdt2);

    let merged_child = crdt1
        .root
        .children
        .get("parent")
        .unwrap()
        .children
        .get("child")
        .unwrap();
    assert_eq!(merged_child.crdt.value(), 2);
}

#[test]
fn test_create_merkle_tree() {
    let mut crdt = HierarchicalCRDT::new();
    crdt.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );
    let merkle_tree = create_merkle_tree(&crdt.root);
    assert_eq!(merkle_tree.leafs(), 2); // 1 for crdt, 1 for child node
}

#[test]
fn test_generate_and_verify_proof() {
    let mut crdt = HierarchicalCRDT::new();
    crdt.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );
    let merkle_tree = create_merkle_tree(&crdt.root);
    let proof = generate_proof(&merkle_tree, 0).expect("Failed to generate proof");
    let root_hash = merkle_tree.root();
    let leaf_hash = proof.item();
    assert!(verify_proof(&proof, &root_hash, &leaf_hash).is_ok());
}

#[test]
fn test_store_and_load_hierarchical_crdt() {
    // Configuration for RocksDB
    let tmp_dir = TempDir::new("test_db").expect("Failed to create temp dir");
    let config = StoreConfig {
        path: Utf8PathBuf::from_path_buf(tmp_dir.path().into()).unwrap(),
    };

    // Initialize RocksDB
    let db = RocksDB::open(&config).expect("Failed to create database");

    // Create a new hierarchical CRDT
    let mut crdt = HierarchicalCRDT::new();
    crdt.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );

    // Store the CRDT in RocksDB
    db.store_hierarchical_crdt(&b"crdt_key".to_vec(), &crdt)
        .expect("Failed to store CRDT");

    // Load the CRDT from RocksDB
    let loaded_crdt = db
        .load_hierarchical_crdt(&b"crdt_key".to_vec())
        .expect("Failed to load CRDT")
        .expect("CRDT not found");
    assert_eq!(crdt, loaded_crdt);
}

#[test]
fn test_store_and_load_and_verify_hierarchical_crdt() {
    // Configuration for RocksDB
    let tmp_dir = TempDir::new("test_db").expect("Failed to create temp dir");
    let config = StoreConfig {
        path: Utf8PathBuf::from_path_buf(tmp_dir.path().into()).unwrap(),
    };

    // Initialize RocksDB
    let db = RocksDB::open(&config).expect("Failed to create database");

    // Create a new hierarchical CRDT
    let mut crdt = HierarchicalCRDT::new();

    // Increment a node
    crdt.increment(
        vec!["parent".to_owned(), "child".to_owned()],
        "node1".to_owned(),
    );

    // Store the CRDT in RocksDB
    db.store_hierarchical_crdt(&b"crdt_key".to_vec(), &crdt)
        .expect("Failed to store CRDT");

    // Load the CRDT from RocksDB
    let loaded_crdt = db
        .load_hierarchical_crdt(&b"crdt_key".to_vec())
        .expect("Failed to load CRDT")
        .expect("CRDT not found");
    assert_eq!(crdt, loaded_crdt);

    // Create a Merkle tree from the hierarchical CRDT
    let merkle_tree = create_merkle_tree(&crdt.root);

    // Generate a proof for the first leaf
    let proof = generate_proof(&merkle_tree, 0).expect("Failed to generate proof");

    // Verify the proof
    let root_hash = merkle_tree.root();
    let leaf_hash = proof.item();
    let is_valid = verify_proof(&proof, &root_hash, &leaf_hash);
    assert!(is_valid.is_ok());
}

#[cfg(test)]
mod new_tests {
    use super::*;
    use tempdir::TempDir;

    // Test the behavior of the CRDT when merging with an empty CRDT or merging
    // an empty CRDT with a non-empty one.
    #[test]
    fn test_merge_with_empty_crdt() {
        let mut crdt1 = HierarchicalCRDT::new();
        crdt1.increment(
            vec!["parent".to_owned(), "child".to_owned()],
            "node1".to_owned(),
        );

        let crdt2 = HierarchicalCRDT::new();

        // Merge non-empty CRDT with empty CRDT
        crdt1.merge(&crdt2);
        assert_eq!(
            crdt1
                .root
                .children
                .get("parent")
                .unwrap()
                .children
                .get("child")
                .unwrap()
                .crdt
                .value(),
            1
        );

        let mut crdt3 = HierarchicalCRDT::new();
        let crdt4 = HierarchicalCRDT::new();
        crdt3.increment(
            vec!["parent".to_owned(), "child".to_owned()],
            "node2".to_owned(),
        );

        // Merge empty CRDT with non-empty CRDT
        crdt3.merge(&crdt4);
        assert_eq!(
            crdt3
                .root
                .children
                .get("parent")
                .unwrap()
                .children
                .get("child")
                .unwrap()
                .crdt
                .value(),
            1
        );
    }

    // Test the generation and verification of proofs for different leaf
    // indices.
    #[test]
    fn test_generate_and_verify_proof_different_indices() {
        let mut crdt = HierarchicalCRDT::new();
        crdt.increment(
            vec!["parent".to_owned(), "child1".to_owned()],
            "node1".to_owned(),
        );
        crdt.increment(
            vec!["parent".to_owned(), "child2".to_owned()],
            "node2".to_owned(),
        );
        let merkle_tree = create_merkle_tree(&crdt.root);

        for i in 0..merkle_tree.leafs() {
            let proof = generate_proof(&merkle_tree, i).expect("Failed to generate proof");
            let root_hash = merkle_tree.root();
            let leaf_hash = proof.item();
            assert!(verify_proof(&proof, &root_hash, &leaf_hash).is_ok());
        }
    }

    // Test the behavior of the CRDT when incrementing counters at different
    // paths and levels.
    #[test]
    fn test_increment_different_paths_and_levels() {
        let mut crdt = HierarchicalCRDT::new();
        crdt.increment(vec!["level1".to_owned()], "node1".to_owned());
        crdt.increment(
            vec!["level1".to_owned(), "level2".to_owned()],
            "node2".to_owned(),
        );
        crdt.increment(
            vec![
                "level1".to_owned(),
                "level2".to_owned(),
                "level3".to_owned(),
            ],
            "node3".to_owned(),
        );

        assert_eq!(crdt.root.children.get("level1").unwrap().crdt.value(), 1);
        assert_eq!(
            crdt.root
                .children
                .get("level1")
                .unwrap()
                .children
                .get("level2")
                .unwrap()
                .crdt
                .value(),
            1
        );
        assert_eq!(
            crdt.root
                .children
                .get("level1")
                .unwrap()
                .children
                .get("level2")
                .unwrap()
                .children
                .get("level3")
                .unwrap()
                .crdt
                .value(),
            1
        );
    }

    // Test error handling scenarios, such as attempting to load a non-existent
    // CRDT from the database or generating a proof for an invalid leaf index.
    #[test]
    fn test_load_non_existent_crdt() -> EyreResult<()> {
        let tmp_dir = TempDir::new("test_db").expect("Failed to create temp dir");
        let config = StoreConfig {
            path: Utf8PathBuf::from_path_buf(tmp_dir.path().into()).unwrap(),
        };
        let db = RocksDB::open(&config)?;

        let result = db.load_hierarchical_crdt(&b"non_existent_key".to_vec());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        Ok(())
    }

    #[test]
    fn test_generate_proof_invalid_index() {
        let mut crdt = HierarchicalCRDT::new();
        crdt.increment(
            vec!["parent".to_owned(), "child".to_owned()],
            "node1".to_owned(),
        );
        let merkle_tree = create_merkle_tree(&crdt.root);

        let result = generate_proof(&merkle_tree, merkle_tree.leafs() + 1);
        assert!(result.is_err());
    }
}
