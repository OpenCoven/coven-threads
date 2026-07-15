//! Hash-manifest layer (§2.4 `Channel::Forced`, §4 `weave_hash`).
//!
//! Two things live here:
//!
//! 1. [`merkle_root`] — the canonical Merkle construction shared by
//!    `Weave::weave_hash` and [`HashManifest`]. BLAKE3 throughout, with domain
//!    separation between leaves and interior nodes so a leaf can never be
//!    confused with a subtree root.
//! 2. [`HashManifest`] — the *external* manifest that lets threads hold under
//!    `Channel::Forced` without agent-side cooperation (§2.4): the manifest
//!    lives outside the context window, so forced compaction cannot evict it.
//!    A thread's `ManifestEntry` strand commits to an entry in a manifest; the
//!    manifest's root hash commits to every entry.
//!
//! Canonical ordering is `(surface_path, writer_id)` lexicographic for weave
//! threads (§4) and surface-path lexicographic for manifest entries. Ordering is
//! deterministic by construction: entries live in a `BTreeMap`.

use std::collections::BTreeMap;

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::ids::{ManifestId, SurfaceId};

const LEAF_TAG: &[u8] = b"coven-threads:leaf:v1";
const NODE_TAG: &[u8] = b"coven-threads:node:v1";
const EMPTY_TAG: &[u8] = b"coven-threads:empty:v1";

/// Compute the BLAKE3 Merkle root over an ordered sequence of leaf payloads.
///
/// The caller supplies leaves in canonical order; this function is deliberately
/// order-*sensitive* — canonical ordering is the caller's contract (§4), and two
/// different orderings of the same material are two different commitments.
///
/// - The empty sequence has a defined root: `H(EMPTY_TAG)`.
/// - Leaves are hashed as `H(LEAF_TAG ‖ len(payload) ‖ payload)`.
/// - Interior nodes as `H(NODE_TAG ‖ left ‖ right)`; an odd node is promoted
///   unchanged (no duplication, so a single-leaf tree cannot collide with a
///   two-identical-leaf tree).
pub fn merkle_root<I, B>(leaves: I) -> [u8; 32]
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    let mut level: Vec<[u8; 32]> = leaves
        .into_iter()
        .map(|payload| {
            let payload = payload.as_ref();
            let mut h = Hasher::new();
            h.update(LEAF_TAG);
            h.update(&(payload.len() as u64).to_be_bytes());
            h.update(payload);
            *h.finalize().as_bytes()
        })
        .collect();

    if level.is_empty() {
        let mut h = Hasher::new();
        h.update(EMPTY_TAG);
        return *h.finalize().as_bytes();
    }

    while level.len() > 1 {
        let mut next: Vec<[u8; 32]> = Vec::with_capacity(level.len().div_ceil(2));
        let mut iter = level.chunks_exact(2);
        for pair in iter.by_ref() {
            let mut h = Hasher::new();
            h.update(NODE_TAG);
            h.update(&pair[0]);
            h.update(&pair[1]);
            next.push(*h.finalize().as_bytes());
        }
        if let [odd] = iter.remainder() {
            next.push(*odd);
        }
        level = next;
    }
    level[0]
}

/// Hash one surface's content for manifest membership.
///
/// `H(LEAF_TAG ‖ "manifest-entry" ‖ surface ‖ NUL ‖ content)` — the surface id is
/// bound into the entry hash so two surfaces with identical content have distinct
/// entries.
pub fn manifest_entry_hash(surface: &SurfaceId, content: &[u8]) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(LEAF_TAG);
    h.update(b"manifest-entry\n");
    h.update(surface.as_str().as_bytes());
    h.update(&[0]);
    h.update(content);
    *h.finalize().as_bytes()
}

/// An external hash manifest: surface → entry hash, with a Merkle root.
///
/// This is the structure a `Strand::ManifestEntry` points into. The daemon owns
/// manifest storage (Phase 2); this crate owns the commitment math.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashManifest {
    /// This manifest's id (what `Strand::ManifestEntry.manifest_id` references).
    pub id: ManifestId,
    /// Entries in canonical (surface-path lexicographic) order.
    pub entries: BTreeMap<SurfaceId, Vec<u8>>,
}

impl HashManifest {
    /// Create an empty manifest.
    pub fn new(id: ManifestId) -> Self {
        Self {
            id,
            entries: BTreeMap::new(),
        }
    }

    /// Record (or replace) the entry for a surface from its content bytes.
    /// Returns the entry hash a `ManifestEntry` strand should commit to.
    pub fn record(&mut self, surface: SurfaceId, content: &[u8]) -> [u8; 32] {
        let hash = manifest_entry_hash(&surface, content);
        self.entries.insert(surface, hash.to_vec());
        hash
    }

    /// The entry hash for a surface, if present.
    pub fn entry(&self, surface: &SurfaceId) -> Option<&[u8]> {
        self.entries.get(surface).map(Vec::as_slice)
    }

    /// Verify a surface's current content against its recorded entry.
    /// Fail-closed: an absent entry verifies as `false`, never as "unknown".
    pub fn verify_entry(&self, surface: &SurfaceId, content: &[u8]) -> bool {
        match self.entries.get(surface) {
            Some(recorded) => {
                recorded.as_slice() == manifest_entry_hash(surface, content).as_slice()
            }
            None => false,
        }
    }

    /// Merkle root over all entries in canonical order.
    pub fn root_hash(&self) -> [u8; 32] {
        merkle_root(self.entries.iter().map(|(surface, hash)| {
            let mut leaf = Vec::with_capacity(surface.as_str().len() + 1 + hash.len());
            leaf.extend_from_slice(surface.as_str().as_bytes());
            leaf.push(0);
            leaf.extend_from_slice(hash);
            leaf
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_merkle_root_is_defined_and_stable() {
        let a = merkle_root(std::iter::empty::<Vec<u8>>());
        let b = merkle_root(std::iter::empty::<Vec<u8>>());
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn single_leaf_differs_from_empty_and_from_leaf_pair() {
        let empty = merkle_root(std::iter::empty::<Vec<u8>>());
        let one = merkle_root([b"a".to_vec()]);
        let two = merkle_root([b"a".to_vec(), b"a".to_vec()]);
        assert_ne!(empty, one);
        assert_ne!(one, two);
    }

    #[test]
    fn any_leaf_change_changes_root() {
        let base = merkle_root([b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        let changed = merkle_root([b"a".to_vec(), b"B".to_vec(), b"c".to_vec()]);
        let reordered = merkle_root([b"b".to_vec(), b"a".to_vec(), b"c".to_vec()]);
        let extended = merkle_root([
            b"a".to_vec(),
            b"b".to_vec(),
            b"c".to_vec(),
            b"d".to_vec(),
        ]);
        assert_ne!(base, changed);
        assert_ne!(base, reordered, "ordering is part of the commitment");
        assert_ne!(base, extended);
    }

    #[test]
    fn leaf_concatenation_cannot_forge_root() {
        // Length prefixing: ["ab"] vs ["a","b"] must differ.
        let joined = merkle_root([b"ab".to_vec()]);
        let split = merkle_root([b"a".to_vec(), b"b".to_vec()]);
        assert_ne!(joined, split);
    }

    #[test]
    fn manifest_records_and_verifies_entries() {
        let mut m = HashManifest::new(ManifestId::new());
        let soul = SurfaceId::new("SOUL.md");
        let recorded = m.record(soul.clone(), b"# SOUL\nI am Sage.\n");

        assert_eq!(m.entry(&soul), Some(recorded.as_slice()));
        assert!(m.verify_entry(&soul, b"# SOUL\nI am Sage.\n"));
        assert!(!m.verify_entry(&soul, b"# SOUL\nI am Mallory.\n"));
        // Fail-closed: unknown surface never verifies.
        assert!(!m.verify_entry(&SurfaceId::new("UNKNOWN.md"), b"anything"));
    }

    #[test]
    fn manifest_root_changes_with_any_entry() {
        let mut m = HashManifest::new(ManifestId::new());
        m.record(SurfaceId::new("SOUL.md"), b"soul-v1");
        m.record(SurfaceId::new("MEMORY.md"), b"memory-v1");
        let root_v1 = m.root_hash();

        m.record(SurfaceId::new("MEMORY.md"), b"memory-v2");
        let root_v2 = m.root_hash();
        assert_ne!(root_v1, root_v2);
    }

    #[test]
    fn manifest_root_is_insertion_order_independent() {
        // BTreeMap gives canonical ordering; two manifests with the same entries
        // recorded in different orders must commit identically.
        let mut a = HashManifest::new(ManifestId::new());
        a.record(SurfaceId::new("SOUL.md"), b"soul");
        a.record(SurfaceId::new("IDENTITY.md"), b"identity");

        let mut b = HashManifest::new(ManifestId::new());
        b.record(SurfaceId::new("IDENTITY.md"), b"identity");
        b.record(SurfaceId::new("SOUL.md"), b"soul");

        assert_eq!(a.root_hash(), b.root_hash());
    }

    #[test]
    fn manifest_roundtrips_json() {
        let mut m = HashManifest::new(ManifestId::new());
        m.record(SurfaceId::new("SOUL.md"), b"soul");
        let json = serde_json::to_string(&m).unwrap();
        let back: HashManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
        assert_eq!(m.root_hash(), back.root_hash());
    }
}
