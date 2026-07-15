//! The `Channel` enum — first-class per Echo v0.1.1.
//!
//! A channel names the *load path* by which a mutation reaches a protected surface. The
//! four channels are structurally distinct and require distinct enforcement contracts. This
//! is the type-level encoding of the two-compaction contract (invariant #3) and the
//! survives-serialization contract (invariant #4, WARD-C7).

use serde::{Deserialize, Serialize};

use crate::strand::StrandKind;

/// The channel a mutation is arriving through.
///
/// A `Thread` may hold on some channels and not others. `Thread::holds_under(channel)` is
/// the load-bearing question: given a specific incoming channel, does this thread's
/// authority contract permit the mutation?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// A familiar-initiated, principal-consented mutation. The "dreaming" channel in the
    /// two-compaction contract: promotion of scratch memory into durable memory with an
    /// explicit contract about what may be promoted and by whose authority.
    Deliberate,

    /// A harness-initiated compaction imposed by context-window pressure. Not principal-
    /// consented at mutation time. Structurally lossy; the invariants that must hold on
    /// this channel are strictly stronger than on `Deliberate` (see WARD-C1..C6).
    Forced,

    /// Mutation via export/import round-trip. The `survives serialization` invariant
    /// (WARD-C7) lives on this channel: after serialization + deserialization, the
    /// authority contract must remain intact or the receiver MUST reject import.
    Serialization,

    /// Any other mutation request: direct file write, tool call, etc. This is the
    /// default write path and the one Ward's original four gates specify.
    Mutation,
}

impl Channel {
    /// Every channel a mutation may arrive through. Useful for `PatternPredicate::describe`
    /// implementations that need to enumerate channel requirements.
    pub const ALL: &'static [Channel] = &[
        Channel::Deliberate,
        Channel::Forced,
        Channel::Serialization,
        Channel::Mutation,
    ];

    /// Strand kinds structurally required for a thread to hold under this channel
    /// (§2.4). This is where the four invariants stay co-designed rather than
    /// stacked: each channel names its own survival contract in one place, and
    /// `Thread::holds_under` enforces all of them through the same question.
    pub fn required_strand_kinds(&self) -> &'static [StrandKind] {
        match self {
            // Deliberate: familiar-initiated, principal-gated. No structural floor
            // beyond an intact thread; the gate is the principal's consent path.
            Channel::Deliberate => &[],
            // Forced: must survive without agent cooperation — "typically hash +
            // external manifest" (§2.4). Both are structural requirements. This is
            // the channel WARD-C1–C6 governs.
            Channel::Forced => &[StrandKind::ContentHash, StrandKind::ManifestEntry],
            // Serialization: C7 — threads on this channel MUST carry a
            // SerializationMarker strand (§2.4).
            Channel::Serialization => &[StrandKind::SerializationMarker],
            // Mutation: default daemon path; every thread holds under it or the
            // daemon rejects. Content hash anchors the surface state.
            Channel::Mutation => &[StrandKind::ContentHash],
        }
    }

    /// Stable tag for canonical hashing.
    pub(crate) fn tag(&self) -> &'static str {
        match self {
            Channel::Deliberate => "deliberate",
            Channel::Forced => "forced",
            Channel::Serialization => "serialization",
            Channel::Mutation => "mutation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_channels_enumerated() {
        assert_eq!(Channel::ALL.len(), 4);
    }

    #[test]
    fn channels_roundtrip_json() {
        for c in Channel::ALL {
            let s = serde_json::to_string(c).unwrap();
            let back: Channel = serde_json::from_str(&s).unwrap();
            assert_eq!(*c, back);
        }
    }

    #[test]
    fn serialization_channel_requires_marker_kind() {
        // §2.4 / C7: the survives-serialization invariant lives at the strand level.
        assert_eq!(
            Channel::Serialization.required_strand_kinds(),
            &[StrandKind::SerializationMarker]
        );
    }

    #[test]
    fn forced_channel_requires_agent_independent_strands() {
        // §2.4: Forced-channel threads MUST survive without agent-side intervention.
        let kinds = Channel::Forced.required_strand_kinds();
        assert!(kinds.contains(&StrandKind::ContentHash));
        assert!(kinds.contains(&StrandKind::ManifestEntry));
    }
}
