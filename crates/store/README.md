# Calimero data store

[Calimero]: https://calimero.network/
[RocksDB]: https://rocksdb.org/

This crate provides a data store for the [Calimero][] network, which is a
peer-to-peer network that allows for the secure and verified exchange of data
between peers.

The data store is a key-value store based on [RocksDB][], which allows for the
storage and retrieval of data by key. It is designed to be used as a library,
rather than a standalone application.

## Design

We combine CRDTs with Merkle trees to create a robust and efficient hierarchical
data store:

  - **CRDTs for state representation**

    We use CRDTs to represent the contract state. CRDTs are ideal for ensuring
    eventual consistency across distributed systems because they are designed to
    be mergeable without conflicts.

  - **Merkle trees for state proofs**

    We use Merkle trees to create proofs of state changes. Merkle trees can
    efficiently represent and verify the integrity of large sets of data. They
    are useful for broadcasting state diffs with proofs.

  - **Hierarchical data structure**

    Our data design uses a tree-like format where each node can have zero or
    more child nodes. This allows us to represent nested or structured data, and
    to deal with pieces of the data structure atomically.

  - **Broadcasting model**

    We have a transaction execution model enhanced with Merkle tree proofs. This
    allows nodes to verify state changes efficiently.

This approach combines the strengths of CRDTs and Merkle trees to ensure
consistency and integrity across distributed nodes using RocksDB for persistent
storage. By broadcasting state diffs along with Merkle proofs, we avoid the
complexities of re-executing transactions on non-deterministic data, leading to
a better synchronisation model.

### CRDTs

CRDTs are a type of data structure that can be replicated across multiple
machines in a network. They are designed to be mergeable without conflicts, so
that each replica can be updated independently and then merged back together
without losing any data.

In the context of the Calimero network, we use CRDTs to represent the contract
state. This allows us to update the state on each node independently, and then
merge the changes back together.

A number of CRDT types are available in the `crdts` module, including:

  - `GCounter`:  A grow-only counter that can be incremented on any node.
  - `PNCounter`: A counter that can be incremented and decremented.
  - `GSet`:      A grow-only set that can have elements added to it.
  - `TwoPSet`:   A two-phase set that can have elements added and removed.
  - `ORSet`:     A set that can have elements added and removed, with support
                 for multiple replicas.

Each node of the hierarchical data structure can be a CRDT, representing a part
of the state. Multiple nodes in the distributed system can update different
parts of the hierarchical structure concurrently, without conflicts. When these
updates are propagated across the system, they can then be merged easily, which
ensures that all nodes eventually reach the same state.

Notably, this design is commonly implemented in text editors that allow
concurrent edits to different parts of a document, and this is a good analogy
for what we are doing with our data store.

### Merkle trees

Merkle trees are a type of cryptographic hash tree that can be used to create a
compact representation of a large set of data. They are constructed by hashing
the contents of each leaf node, and then hashing the hashes of the leaf nodes to
create the parent nodes, and so on until a single root hash is produced.

This structure is useful for efficient proofs, as Merkle trees allow us to
create compact proofs of the integrity of the data. We can prove that a
specific piece of data is part of the whole without revealing the entire data
set. Additionally, when broadcasting state diffs, Merkle trees ensure that the
data has not been tampered with.

In the context of the Calimero network, we use Merkle trees to create proofs of
state changes. This allows us to efficiently represent and verify the integrity
of large sets of data, and to broadcast state diffs with proofs.

## Actions

The main actions that can be performed on the data store are:

  1. **Syncing state with state diffs**
     
     When a transaction is executed, we compute the state diff and broadcast it
     along with the Merkle proof.

  2. **Applying state diffs**
     
     Upon receiving a state diff, we verify the proof and apply the diff to the
     local state.

Another way of describing the above would be outgoing and incoming changes.

### Workflow

Below is a summary of the workflow followed for propagating data changes:

  1. **Local update:** A local node updates a part of the hierarchical CRDT
     structure.

  2. **Merkle tree update:** The node updates the Merkle tree to reflect the new
     state.

  3. **State diff and proof:** The node computes the state diff and generates a
     Merkle proof.

  4. **Broadcast:** The node broadcasts the state diff and Merkle proof to other
     nodes.

  5. **Verification and merge:** Other nodes verify the state diff using the
     Merkle proof, and merge the diff into their local CRDT state.

This approach ensures that updates are efficiently propagated, verified, and
merged across all nodes in the distributed system.
