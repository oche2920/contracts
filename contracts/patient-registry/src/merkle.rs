//! Merkle tree utilities for patient record membership proofs.
//!
//! ## Tree construction
//! Leaves are the patient's record IDs in insertion order.
//! ```text
//! leaf  = sha256(0x00 || record_id_be_8)
//! node  = sha256(0x01 || min(left, right) || max(left, right))
//! ```
//! Sorting children before hashing means the verifier never needs position
//! bits — the proof is just `Vec<BytesN<32>>` of sibling hashes.
//!
//! Odd-length layers: the dangling node is paired with itself.

use soroban_sdk::{Bytes, BytesN, Env, Vec};

const LEAF_TAG: u8 = 0x00;
const NODE_TAG: u8 = 0x01;

// ─── primitives ────────────────────────────────────────────────────────────

/// Hash a record ID as a Merkle leaf.
///
/// `leaf = sha256(0x00 || record_id_be_8)`
pub fn hash_leaf(env: &Env, record_id: u64) -> BytesN<32> {
    let mut pre = Bytes::new(env);
    pre.extend_from_array(&[LEAF_TAG]);
    pre.extend_from_array(&record_id.to_be_bytes());
    env.crypto().sha256(&pre).into()
}

/// Hash two child hashes into a parent node.
///
/// Children are sorted lexicographically before hashing so that no position
/// information needs to be stored alongside proof siblings:
///
/// `node = sha256(0x01 || min(a,b) || max(a,b))`
pub fn hash_pair(env: &Env, a: BytesN<32>, b: BytesN<32>) -> BytesN<32> {
    let (lo, hi) = if a.to_array() <= b.to_array() {
        (a, b)
    } else {
        (b, a)
    };
    let mut pre = Bytes::new(env);
    pre.extend_from_array(&[NODE_TAG]);
    pre.extend_from_array(&lo.to_array());
    pre.extend_from_array(&hi.to_array());
    env.crypto().sha256(&pre).into()
}

// ─── root computation ──────────────────────────────────────────────────────

/// Compute the Merkle root over an ordered slice of record IDs.
///
/// Empty set: returns `sha256("")` as a deterministic sentinel.
pub fn compute_merkle_root(env: &Env, record_ids: &Vec<u64>) -> BytesN<32> {
    let n = record_ids.len();
    if n == 0 {
        return env.crypto().sha256(&Bytes::new(env)).into();
    }

    // Build leaf layer
    let mut layer: Vec<BytesN<32>> = Vec::new(env);
    for id in record_ids.iter() {
        layer.push_back(hash_leaf(env, id));
    }

    // Reduce layer-by-layer until a single root remains
    while layer.len() > 1 {
        let len = layer.len();
        let mut next: Vec<BytesN<32>> = Vec::new(env);
        let mut i = 0u32;
        while i + 1 < len {
            next.push_back(hash_pair(
                env,
                layer.get(i).unwrap(),
                layer.get(i + 1).unwrap(),
            ));
            i += 2;
        }
        // Odd node: pair with itself
        if len % 2 == 1 {
            let last = layer.get(len - 1).unwrap();
            next.push_back(hash_pair(env, last.clone(), last));
        }
        layer = next;
    }

    layer.get(0).unwrap()
}

// ─── membership verification ───────────────────────────────────────────────

/// Verify that `record_id` belongs to the tree with the given `root`.
///
/// `proof` contains one sibling hash per tree level (leaf level → root).
/// Returns `true` iff the recomputed root matches `root`.
pub fn verify_membership(
    env: &Env,
    record_id: u64,
    proof: &Vec<BytesN<32>>,
    root: &BytesN<32>,
) -> bool {
    let mut current = hash_leaf(env, record_id);
    for sibling in proof.iter() {
        current = hash_pair(env, current, sibling);
    }
    &current == root
}
