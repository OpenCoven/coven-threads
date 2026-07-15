//! Portability — the C7 round-trip contract (§3.3 invariant #4, Phase 3).
//!
//! **C7:** every thread bound to `Channel::Serialization` MUST carry a
//! `SerializationMarker` strand whose survival is a round-trip invariant.
//! Export followed by import produces a weave with equivalent tension state,
//! **or fails visibly**. Silent downgrade on import is the `.af` failure mode
//! this module exists to refuse (§12 verification log: `CoreMemoryBlockSchema`
//! has no protection field; runtime `read_only` stripped at export).
//!
//! The unit of exchange is [`PortableWeave`]: a versioned envelope around
//! [`WeaveRecord`] stamped with the [`SerializationContract`] hash it was
//! exported under. The reference encoding is canonical JSON via serde; the
//! Phase 3 interchange *format* decision (Shape A `.af`-superset vs Shape B
//! net-new, `specs/PHASE-3-PORTABILITY.md`) wraps this envelope — it does not
//! change these semantics.
//!
//! Fail-visibly matrix enforced here:
//! - exporting a `Serialization`-covered thread with no marker → error (the
//!   contract cannot be stamped on material that never carried it);
//! - importing an unsupported format version → error;
//! - importing an artifact whose envelope contract hash is unknown → error;
//! - importing a thread whose marker committed to a different contract →
//!   error naming the thread and surface;
//! - importing a record whose threads were altered after export →
//!   [`WeaveError::HashMismatch`] (weave-hash recomputation);
//! - a clean import yields a weave with *equivalent tension state* and
//!   *identical authority topology* — never wider (RFC-0001 §5.1: import is
//!   not a write path around the gates).

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::channel::Channel;
use crate::ids::{SurfaceId, ThreadId};
use crate::pattern::PatternPredicate;
use crate::strand::{Strand, StrandKind};
use crate::weave::{Weave, WeaveError, WeaveRecord};

/// The portability format version this crate exports and accepts.
pub const PORTABILITY_FORMAT_VERSION: &str = "0.1.0";

/// The serialization contract: what a `SerializationMarker` strand's
/// `contract_hash` commits to.
///
/// The contract enumerates, per strand kind, that the kind is representable in
/// the portable form and survives the transform. Hashing the canonical
/// encoding makes contract drift *visible*: a runtime with a different strand
/// vocabulary produces a different hash and imports fail loudly instead of
/// silently dropping fibers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializationContract {
    /// Format version this contract belongs to.
    pub format_version: String,
    /// Strand kinds this format round-trips, in canonical order.
    pub strand_kinds: Vec<StrandKind>,
}

impl SerializationContract {
    /// The current contract: all five frozen strand kinds (§2.3) round-trip.
    pub fn current() -> Self {
        Self {
            format_version: PORTABILITY_FORMAT_VERSION.to_string(),
            strand_kinds: vec![
                StrandKind::ContentHash,
                StrandKind::Signature,
                StrandKind::ManifestEntry,
                StrandKind::AuditTrail,
                StrandKind::SerializationMarker,
            ],
        }
    }

    /// BLAKE3 over the canonical encoding — the value `SerializationMarker`
    /// strands commit to.
    pub fn contract_hash(&self) -> Vec<u8> {
        let mut h = blake3::Hasher::new();
        h.update(b"coven-threads:serialization-contract:v1\n");
        h.update(self.format_version.as_bytes());
        h.update(b"\n");
        for kind in &self.strand_kinds {
            h.update(format!("{kind:?}\n").as_bytes());
        }
        h.finalize().as_bytes().to_vec()
    }

    /// A `SerializationMarker` strand committing to this contract.
    pub fn marker_strand(&self) -> Strand {
        Strand::SerializationMarker {
            id: crate::ids::StrandId::new(),
            format_version: self.format_version.clone(),
            contract_hash: self.contract_hash(),
        }
    }
}

/// The portable envelope: what leaves one runtime and enters another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableWeave {
    /// Format version of this artifact.
    pub format_version: String,
    /// Hash of the [`SerializationContract`] the artifact was exported under.
    pub contract_hash: Vec<u8>,
    /// The weave record (threads, tension, weave hash, derived descriptor).
    pub record: WeaveRecord,
}

/// Why an export or import failed — every arm is a *visible* refusal (C7).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PortabilityError {
    /// A `Serialization`-covered thread carries no `SerializationMarker`
    /// strand: the survival contract cannot be stamped. Fail-closed at export
    /// — shipping the thread anyway would manufacture the silent-downgrade
    /// import this module refuses.
    #[error("thread {thread} on surface {surface} holds under Serialization but carries no SerializationMarker strand")]
    MarkerMissing {
        /// The offending thread.
        thread: ThreadId,
        /// Its surface.
        surface: SurfaceId,
    },

    /// The artifact's format version is not supported by this runtime.
    #[error(
        "unsupported portability format version {found:?}; this runtime supports {supported:?}"
    )]
    UnsupportedFormatVersion {
        /// Version carried by the artifact.
        found: String,
        /// Version this runtime supports.
        supported: String,
    },

    /// The artifact's envelope contract hash does not match this runtime's
    /// contract: the two runtimes disagree about what survives serialization.
    #[error(
        "serialization contract mismatch: artifact contract differs from this runtime's contract"
    )]
    ContractMismatch,

    /// A thread's `SerializationMarker` committed to a different contract
    /// than the artifact claims — the strand did not survive the transform
    /// with its commitment intact.
    #[error("thread {thread} on surface {surface}: SerializationMarker does not match the artifact contract")]
    MarkerContractMismatch {
        /// The offending thread.
        thread: ThreadId,
        /// Its surface.
        surface: SurfaceId,
    },

    /// The artifact is not decodable as a `PortableWeave`.
    #[error("artifact is not a valid PortableWeave: {detail}")]
    Malformed {
        /// Decoder diagnostic.
        detail: String,
    },

    /// Structural verification failed on import (weave-hash mismatch or
    /// duplicate `(surface, writer)` pair).
    #[error("weave verification failed on import: {0}")]
    Weave(#[from] WeaveError),
}

/// Export a weave as a portable artifact.
///
/// Refuses (visibly) if any `Serialization`-covered thread lacks its marker
/// strand — C7 is enforced at the boundary in both directions.
pub fn export_weave(weave: &Weave) -> Result<PortableWeave, PortabilityError> {
    let contract = SerializationContract::current();
    for thread in weave.threads() {
        if thread.holds_under.contains(&Channel::Serialization) {
            let has_marker = thread
                .strands
                .iter()
                .any(|s| s.kind() == StrandKind::SerializationMarker);
            if !has_marker {
                return Err(PortabilityError::MarkerMissing {
                    thread: thread.id,
                    surface: thread.surface.clone(),
                });
            }
        }
    }
    Ok(PortableWeave {
        format_version: contract.format_version.clone(),
        contract_hash: contract.contract_hash(),
        record: weave.to_record(),
    })
}

/// Serialize a portable weave to canonical JSON bytes (reference encoding).
pub fn to_json_bytes(portable: &PortableWeave) -> Result<Vec<u8>, PortabilityError> {
    serde_json::to_vec_pretty(portable).map_err(|err| PortabilityError::Malformed {
        detail: err.to_string(),
    })
}

/// Decode a portable weave from JSON bytes. Decoding is not acceptance —
/// [`import_weave`] performs the contract and structural verification.
pub fn from_json_bytes(bytes: &[u8]) -> Result<PortableWeave, PortabilityError> {
    serde_json::from_slice(bytes).map_err(|err| PortabilityError::Malformed {
        detail: err.to_string(),
    })
}

/// Import a portable artifact, rebinding the (non-serializable, authoritative)
/// pattern predicate. Every check fails visibly; nothing degrades silently.
///
/// A clean import yields a weave whose tension state, thread set, and
/// `weave_hash` are equivalent to the exported weave's (C7), and whose
/// authority topology is byte-identical — import never widens authority
/// (RFC-0001 §5.1).
pub fn import_weave(
    portable: PortableWeave,
    pattern: Box<dyn PatternPredicate>,
) -> Result<Weave, PortabilityError> {
    let contract = SerializationContract::current();

    if portable.format_version != contract.format_version {
        return Err(PortabilityError::UnsupportedFormatVersion {
            found: portable.format_version,
            supported: contract.format_version,
        });
    }
    let expected_hash = contract.contract_hash();
    if portable.contract_hash != expected_hash {
        return Err(PortabilityError::ContractMismatch);
    }

    // Per-thread marker verification: every Serialization-covered thread must
    // carry a marker committing to the artifact's contract.
    for thread in &portable.record.threads {
        if !thread.holds_under.contains(&Channel::Serialization) {
            continue;
        }
        let marker_matches = thread.strands.iter().any(|s| {
            matches!(
                s,
                Strand::SerializationMarker { format_version, contract_hash, .. }
                    if format_version == &contract.format_version
                        && contract_hash == &expected_hash
            )
        });
        if !marker_matches {
            return Err(PortabilityError::MarkerContractMismatch {
                thread: thread.id,
                surface: thread.surface.clone(),
            });
        }
    }

    // Structural verification: recomputed weave hash must equal the recorded
    // one; duplicate (surface, writer) pairs are refused (§2.1).
    Ok(Weave::from_record(portable.record, pattern)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_hash_is_stable_and_version_sensitive() {
        let a = SerializationContract::current();
        let b = SerializationContract::current();
        assert_eq!(a.contract_hash(), b.contract_hash());

        let mut c = SerializationContract::current();
        c.format_version = "9.9.9".into();
        assert_ne!(a.contract_hash(), c.contract_hash());

        let mut d = SerializationContract::current();
        d.strand_kinds.pop();
        assert_ne!(a.contract_hash(), d.contract_hash());
    }

    #[test]
    fn marker_strand_commits_to_contract() {
        let contract = SerializationContract::current();
        match contract.marker_strand() {
            Strand::SerializationMarker {
                format_version,
                contract_hash,
                ..
            } => {
                assert_eq!(format_version, PORTABILITY_FORMAT_VERSION);
                assert_eq!(contract_hash, contract.contract_hash());
            }
            other => panic!("expected marker strand, got {other:?}"),
        }
    }
}
