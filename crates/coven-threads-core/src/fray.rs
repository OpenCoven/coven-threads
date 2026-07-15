//! `FrayOrSnap` — the failure taxonomy for `Thread::holds_under` (§2.3, §4, §5).
//!
//! Fraying is the intermediate state between "holds" and "snapped": one strand
//! failed but the thread has not been severed. Snapping is terminal. Making the
//! distinction first-class keeps partial failure legible in the type system —
//! §5 maps `Frayed → DegradeToProposal` and `Snapped → Reject`.
//!
//! `NotCovered` is the fail-closed answer for a channel the thread does not
//! extend authority over (§5: "Unknown channel → Reject"). It is neither a fray
//! nor a snap — the thread is healthy — but the mutation is still not permitted
//! by this thread. Fail-closed on every unknown is a Gate 4 conformance property
//! (RFC-0001 §5.4), stated at line one, not hardening.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::channel::Channel;
use crate::ids::StrandId;
use crate::strand::StrandKind;

/// Why a specific strand frayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrayReason {
    /// Surface content does not match the `ContentHash` strand's recorded hash.
    ContentHashMismatch,
    /// A `Signature` strand failed verification or its key is unresolvable.
    SignatureInvalid,
    /// A `ManifestEntry` strand's hash disagrees with the external manifest.
    ManifestEntryMismatch,
    /// An `AuditTrail` strand's event reference cannot be verified against
    /// `ward.audit` (§3.4).
    AuditTrailUnverifiable,
    /// A strand kind required on this channel is absent from the thread.
    /// Structural detection — e.g. `Channel::Serialization` without a
    /// `SerializationMarker` strand (§2.4, C7).
    RequiredStrandMissing {
        /// The missing kind.
        kind: StrandKind,
    },
    /// A `SerializationMarker` strand does not match the round-trip contract (C7).
    SerializationMarkerMismatch,
    /// Freeform diagnostic.
    Other(String),
}

/// Why a thread snapped (terminal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapReason {
    /// Revoked by external authority.
    Revoked,
    /// Multiple strands frayed simultaneously; in-place repair is not possible.
    MultipleStrandFray,
    /// The containing weave's `PatternPredicate` ruled this thread's pattern broken.
    PatternBroken,
    /// Freeform diagnostic.
    Other(String),
}

/// The failure result of `Thread::holds_under(channel)` (§4).
///
/// §4 freezes the signature `holds_under(&self, channel) -> Result<(), FrayOrSnap>`;
/// the variant set here is the Cody-lane ergonomics of that contract.
#[derive(Debug, Clone, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrayOrSnap {
    /// The thread does not cover this channel (`Thread::holds_under` field, §4).
    ///
    /// Fail-closed: a thread extends authority only over channels it explicitly
    /// covers. Not an error state on the thread — but the answer to "does this
    /// thread hold here?" is still *no*.
    #[error("thread does not cover channel {channel:?} — fail-closed")]
    NotCovered {
        /// The channel the thread does not cover.
        channel: Channel,
    },

    /// One or more strands failed but the thread is repairable. Frayed threads
    /// MUST surface to the operator (§2.3); §5 maps this to `DegradeToProposal`.
    #[error("thread frayed at strand {strand:?} on channel {channel:?}: {reason:?}")]
    Frayed {
        /// The strand that frayed — `None` when the fray is a *missing* required
        /// strand, which has no id to name.
        strand: Option<StrandId>,
        /// The channel the fray was detected on.
        channel: Channel,
        /// Why.
        reason: FrayReason,
    },

    /// The thread is terminally severed. The protected surface becomes read-only
    /// until a fresh authority ceremony repairs the weave (§5).
    #[error("thread snapped on channel {channel:?}: {reason:?}")]
    Snapped {
        /// The channel the snap is attributed to.
        channel: Channel,
        /// Why.
        reason: SnapReason,
    },
}
