#[cfg(test)]
#[path = "tests/prototype.rs"]
mod tests;

use crate::db::rocksdb::RocksDB;
use crate::db::{Database, Key};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use eyre::Result as EyreResult;
use merkletree::hash::{Algorithm, Hashable};
use merkletree::merkle::MerkleTree;
use merkletree::proof::Proof;
use merkletree::store::VecStore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::Hasher;

#[derive(Default, Clone)]
struct Sha256Algorithm(Sha256);

impl Hasher for Sha256Algorithm {
    fn finish(&self) -> u64 {
        // This is not currently used
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
}

impl Algorithm<[u8; 32]> for Sha256Algorithm {
    fn hash(&mut self) -> [u8; 32] {
        self.0.clone().finalize().into()
    }

    fn reset(&mut self) {
        self.0 = Sha256::new();
    }

    fn leaf(&mut self, leaf: [u8; 32]) -> [u8; 32] {
        self.reset();
        self.write(&leaf);
        self.hash()
    }

    fn node(&mut self, left: [u8; 32], right: [u8; 32], _height: usize) -> [u8; 32] {
        self.reset();
        self.write(&left);
        self.write(&right);
        self.hash()
    }
}

impl Hashable<Sha256Algorithm> for GCounter {
    fn hash(&self, state: &mut Sha256Algorithm) {
        for (node, value) in &self.nodes {
            state.write(node.as_bytes());
            state.write(&value.to_le_bytes());
        }
    }
}

impl Hashable<Sha256Algorithm> for Node {
    fn hash(&self, state: &mut Sha256Algorithm) {
        self.crdt.hash(state);
        for (key, child) in &self.children {
            state.write(key.as_bytes());
            child.hash(state);
        }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
struct GCounter {
    nodes: HashMap<String, u64>,
}

impl GCounter {
    fn new() -> Self {
        GCounter {
            nodes: HashMap::new(),
        }
    }

    fn increment(&mut self, node: String) {
        let counter = self.nodes.entry(node).or_insert(0);
        *counter += 1;
    }

    fn merge(&mut self, other: &GCounter) {
        for (node, &value) in &other.nodes {
            let counter = self.nodes.entry(node.clone()).or_insert(0);
            *counter = (*counter).max(value);
        }
    }

    fn value(&self) -> u64 {
        self.nodes.values().sum()
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
struct Node {
    crdt: GCounter,
    children: HashMap<String, Node>,
}

impl Node {
    fn new() -> Self {
        Node {
            crdt: GCounter::new(),
            children: HashMap::new(),
        }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
struct HierarchicalCRDT {
    root: Node,
}

impl HierarchicalCRDT {
    fn new() -> Self {
        HierarchicalCRDT { root: Node::new() }
    }

    fn increment(&mut self, path: Vec<String>, node_id: String) {
        let mut current = &mut self.root;
        for p in path {
            current = current.children.entry(p).or_insert_with(Node::new);
        }
        current.crdt.increment(node_id);
    }

    fn merge(&mut self, other: &HierarchicalCRDT) {
        let mut new_root = self.root.clone();
        Self::merge_node(&mut new_root, &other.root);
        self.root = new_root;
    }

    fn merge_node(node: &mut Node, other: &Node) {
        node.crdt.merge(&other.crdt);
        for (key, child) in &other.children {
            Self::merge_node(
                node.children.entry(key.clone()).or_insert_with(Node::new),
                child,
            );
        }
    }
}

fn create_merkle_tree(node: &Node) -> MerkleTree<[u8; 32], Sha256Algorithm, VecStore<[u8; 32]>> {
    let mut leaves = vec![];
    let mut hasher = Sha256Algorithm::default();
    node.crdt.hash(&mut hasher);
    leaves.push(hasher.hash());
    for child in node.children.values() {
        hasher.reset();
        child.hash(&mut hasher);
        leaves.push(hasher.hash());
    }
    MerkleTree::from_data(leaves).expect("Failed to create Merkle Tree")
}

fn generate_proof(
    merkle_tree: &MerkleTree<[u8; 32], Sha256Algorithm, VecStore<[u8; 32]>>,
    index: usize,
) -> Result<Proof<[u8; 32]>, eyre::Error> {
    merkle_tree.gen_proof(index).map_err(|e| eyre::eyre!(e))
}

fn verify_proof(
    proof: &Proof<[u8; 32]>,
    root: &[u8; 32],
    leaf: &[u8; 32],
) -> Result<(), eyre::Error> {
    if proof.validate::<Sha256Algorithm>().is_err() {
        return Err(eyre::eyre!("Invalid proof"));
    }
    if proof.root() != *root {
        return Err(eyre::eyre!("Invalid root hash"));
    }
    if proof.item() != *leaf {
        return Err(eyre::eyre!("Invalid leaf hash"));
    }
    Ok(())
}

impl RocksDB {
    pub fn store_hierarchical_crdt(&self, key: &Key, crdt: &HierarchicalCRDT) -> EyreResult<()> {
        let serialized = to_vec(crdt)?;
        self.put(key, serialized)?;
        Ok(())
    }

    pub fn load_hierarchical_crdt(&self, key: &Key) -> EyreResult<Option<HierarchicalCRDT>> {
        match self.get(key)? {
            Some(serialized) => {
                let crdt = HierarchicalCRDT::try_from_slice(&serialized)?;
                Ok(Some(crdt))
            }
            None => Ok(None),
        }
    }
}
