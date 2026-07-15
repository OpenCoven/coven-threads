//! `Strand` — fibers inside a thread (`PHASE-0-DESIGN.md` §2.3, §4).
//!
//! Strands are the fibers that make a thread survive stress. A thread survives a
//! channel iff its strands survive that channel. Strands make failure legible:
//! instead of "thread snapped," you get "thread frayed at strand `ContentHash` —
//! SOUL.md hash mismatch, detected on channel `Forced`."
//!
//! Strand kinds are the frozen v0.1.1 set (§2.3): `ContentHash`, `Signature`,
//! `ManifestEntry`, `AuditTrail`, `SerializationMarker`.
//!
//! **The fourth invariant (survives serialization, WARD-C7) lives at the strand
//! level** (§2.3): a thread survives serialization iff *all its strands* survive the
//! lossy transform. `SerializationMarker` is the strand that carries the survival
//! contract explicitly.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{EventRef, ManifestId, StrandId};

/// A single fiber of authority commitment inside a thread (§2.3).
///
/// Enum-over-struct resolved by Cody's scoping read (§4 open question #1): enum wins
/// for extension — new strand kinds are new variants, and exhaustive matches force
/// every consumer to decide how the new kind fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strand {
    /// Content hash of the protected surface recorded at weave time (§4).
    ContentHash {
        /// This strand's id.
        id: StrandId,
        /// Which hash algorithm produced `value`.
        algorithm: HashAlgo,
        /// The recorded hash bytes.
        value: Vec<u8>,
    },

    /// Signature over the surface by an authority key (§4).
    Signature {
        /// This strand's id.
        id: StrandId,
        /// Key identifier — opaque to this crate; the daemon resolves it.
        key_id: String,
        /// The kind of signature.
        kind: SigKind,
        /// The signature bytes.
        value: Vec<u8>,
    },

    /// Membership entry in an external hash manifest (§4).
    ///
    /// This is the strand that lets a thread hold under `Channel::Forced` without
    /// agent-side cooperation (§2.4): the manifest lives outside the context window,
    /// so forced compaction cannot evict it.
    ManifestEntry {
        /// This strand's id.
        id: StrandId,
        /// Which manifest this entry belongs to.
        manifest_id: ManifestId,
        /// The entry's hash bytes within the manifest.
        entry_hash: Vec<u8>,
    },

    /// Audit-trail anchor into the daemon-owned `ward.audit` store (§4, §3.4).
    AuditTrail {
        /// This strand's id.
        id: StrandId,
        /// When this thread was first seen by the audit store.
        first_seen: OffsetDateTime,
        /// Reference to the audit event row (`ward.audit` in `coven.sqlite3`).
        event_log_ref: EventRef,
    },

    /// Serialization survival contract — the C7 strand (§2.3, §3.3 invariant #4).
    ///
    /// Every thread that must round-trip export/import carries this strand. Export
    /// followed by import produces a weave with equivalent tension state, or fails
    /// visibly (§3.3 C7).
    SerializationMarker {
        /// This strand's id.
        id: StrandId,
        /// Portable format version (semver string; strict parsing is a Phase 3 concern).
        format_version: String,
        /// Hash of the serialization contract this marker commits to.
        contract_hash: Vec<u8>,
    },
}

impl Strand {
    /// Every strand carries an id.
    pub fn id(&self) -> StrandId {
        match self {
            Strand::ContentHash { id, .. }
            | Strand::Signature { id, .. }
            | Strand::ManifestEntry { id, .. }
            | Strand::AuditTrail { id, .. }
            | Strand::SerializationMarker { id, .. } => *id,
        }
    }

    /// The kind discriminant for this strand (used by descriptors and requirements —
    /// derived introspection, never enforcement; see `pattern` module docs).
    pub fn kind(&self) -> StrandKind {
        match self {
            Strand::ContentHash { .. } => StrandKind::ContentHash,
            Strand::Signature { .. } => StrandKind::Signature,
            Strand::ManifestEntry { .. } => StrandKind::ManifestEntry,
            Strand::AuditTrail { .. } => StrandKind::AuditTrail,
            Strand::SerializationMarker { .. } => StrandKind::SerializationMarker,
        }
    }

    /// Canonical bytes of this strand's committed material, for weave/thread hashing.
    ///
    /// Any change to a strand's committed payload changes these bytes, which changes
    /// the containing thread's leaf hash, which changes `weave_hash` (§4). Timestamps
    /// participate for `AuditTrail` because first-seen is part of the commitment.
    pub fn commitment_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Strand::ContentHash {
                algorithm, value, ..
            } => {
                out.extend_from_slice(b"content-hash\n");
                out.extend_from_slice(algorithm.tag().as_bytes());
                out.push(b'\n');
                out.extend_from_slice(value);
            }
            Strand::Signature {
                key_id,
                kind,
                value,
                ..
            } => {
                out.extend_from_slice(b"signature\n");
                out.extend_from_slice(key_id.as_bytes());
                out.push(b'\n');
                out.extend_from_slice(kind.tag().as_bytes());
                out.push(b'\n');
                out.extend_from_slice(value);
            }
            Strand::ManifestEntry {
                manifest_id,
                entry_hash,
                ..
            } => {
                out.extend_from_slice(b"manifest-entry\n");
                out.extend_from_slice(manifest_id.0.as_bytes());
                out.extend_from_slice(entry_hash);
            }
            Strand::AuditTrail {
                first_seen,
                event_log_ref,
                ..
            } => {
                out.extend_from_slice(b"audit-trail\n");
                out.extend_from_slice(first_seen.unix_timestamp_nanos().to_be_bytes().as_slice());
                out.extend_from_slice(event_log_ref.as_str().as_bytes());
            }
            Strand::SerializationMarker {
                format_version,
                contract_hash,
                ..
            } => {
                out.extend_from_slice(b"serialization-marker\n");
                out.extend_from_slice(format_version.as_bytes());
                out.push(b'\n');
                out.extend_from_slice(contract_hash);
            }
        }
        out
    }
}

/// Discriminant over `Strand` variants (§4 `StrandKind`).
///
/// What a pattern *names* as required, distinct from what a thread *carries*.
/// Used in `StrandRequirement` and `PatternDescriptor` — introspection surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrandKind {
    /// A `ContentHash` strand.
    ContentHash,
    /// A `Signature` strand.
    Signature,
    /// A `ManifestEntry` strand.
    ManifestEntry,
    /// An `AuditTrail` strand.
    AuditTrail,
    /// A `SerializationMarker` strand.
    SerializationMarker,
}

/// Which hash algorithm a `ContentHash` strand uses (§4 `HashAlgo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgo {
    /// BLAKE3 — default for OpenCoven.
    Blake3,
    /// SHA-256 — accepted for interop.
    Sha256,
}

impl HashAlgo {
    fn tag(self) -> &'static str {
        match self {
            HashAlgo::Blake3 => "blake3",
            HashAlgo::Sha256 => "sha256",
        }
    }
}

/// Kind of signature carried by a `Signature` strand (§4 `SigKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigKind {
    /// Ed25519 detached signature.
    Ed25519,
    /// A named-principal attestation resolved by the daemon (no cryptographic key).
    PrincipalAttestation,
}

impl SigKind {
    fn tag(self) -> &'static str {
        match self {
            SigKind::Ed25519 => "ed25519",
            SigKind::PrincipalAttestation => "principal-attestation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_hash(value: Vec<u8>) -> Strand {
        Strand::ContentHash {
            id: StrandId::new(),
            algorithm: HashAlgo::Blake3,
            value,
        }
    }

    #[test]
    fn strand_id_returns_own_id() {
        let id = StrandId::new();
        let s = Strand::ContentHash {
            id,
            algorithm: HashAlgo::Blake3,
            value: vec![0; 32],
        };
        assert_eq!(s.id(), id);
    }

    #[test]
    fn all_five_frozen_kinds_constructible_and_roundtrip() {
        // §2.3 frozen v0.1.1 strand set. If a kind is added or removed, this test —
        // and the design doc — must change together.
        let strands = vec![
            content_hash(vec![1, 2, 3]),
            Strand::Signature {
                id: StrandId::new(),
                key_id: "val-ed25519-1".into(),
                kind: SigKind::Ed25519,
                value: vec![9; 64],
            },
            Strand::ManifestEntry {
                id: StrandId::new(),
                manifest_id: ManifestId::new(),
                entry_hash: vec![4; 32],
            },
            Strand::AuditTrail {
                id: StrandId::new(),
                first_seen: OffsetDateTime::UNIX_EPOCH,
                event_log_ref: EventRef::new("ward.audit/42"),
            },
            Strand::SerializationMarker {
                id: StrandId::new(),
                format_version: "0.1.0".into(),
                contract_hash: vec![7; 32],
            },
        ];
        let kinds: Vec<StrandKind> = strands.iter().map(Strand::kind).collect();
        assert_eq!(
            kinds,
            vec![
                StrandKind::ContentHash,
                StrandKind::Signature,
                StrandKind::ManifestEntry,
                StrandKind::AuditTrail,
                StrandKind::SerializationMarker,
            ]
        );
        for s in &strands {
            let json = serde_json::to_string(s).unwrap();
            let back: Strand = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn commitment_bytes_change_when_payload_changes() {
        let a = content_hash(vec![1, 2, 3]);
        let b = content_hash(vec![1, 2, 4]);
        assert_ne!(a.commitment_bytes(), b.commitment_bytes());
    }

    #[test]
    fn commitment_bytes_stable_across_strand_id() {
        // The strand id is identity, not commitment: two strands committing the same
        // material produce the same bytes.
        let a = content_hash(vec![5; 32]);
        let b = content_hash(vec![5; 32]);
        assert_eq!(a.commitment_bytes(), b.commitment_bytes());
    }

    #[test]
    fn commitment_bytes_distinct_across_kinds() {
        // Domain-separation tags keep different kinds with similar payloads distinct.
        let hash = content_hash(vec![7; 32]);
        let marker = Strand::SerializationMarker {
            id: StrandId::new(),
            format_version: "0.1.0".into(),
            contract_hash: vec![7; 32],
        };
        assert_ne!(hash.commitment_bytes(), marker.commitment_bytes());
    }
}
