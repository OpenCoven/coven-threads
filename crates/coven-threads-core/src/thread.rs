//! `Thread` — an authority relationship: surface → writer (§2.1, §4).
//!
//! A directional line from a *protected surface* to the *authority that gates
//! writes to it*. One thread per `(surface, writer)` pair. A thread has
//! **tension**: it either holds under load, or it frays, or it snaps. Load
//! arrives on a `Channel` (§2.4). Every gate check becomes: **"does thread T
//! hold under channel C?"** — the enforcement vocabulary of the whole layer.
//!
//! Threads are first-class inspectable objects (§2.1): `tension()` returns the
//! current tension state.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::channel::Channel;
use crate::fray::{FrayOrSnap, FrayReason, SnapReason};
use crate::ids::{StrandId, SurfaceId, ThreadId, WriterId};
use crate::strand::Strand;

/// The tension state of a thread (§4 `TensionState`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TensionState {
    /// All strands hold; the thread carries its full authority contract.
    Holds,
    /// One strand failed but the thread has not snapped (§2.3). Repairable.
    /// Frayed threads MUST surface to the operator.
    Frayed {
        /// The strand that frayed — `None` when the fray is a missing required
        /// strand (nothing on the thread to point at).
        strand: Option<StrandId>,
        /// The channel the fray was detected on.
        channel: Channel,
        /// Why.
        reason: FrayReason,
        /// When the fray was detected.
        detected_at: OffsetDateTime,
    },
    /// Terminal severance. Repair requires a fresh authority ceremony.
    Snapped {
        /// The channel the snap is attributed to.
        channel: Channel,
        /// Why.
        reason: SnapReason,
        /// When.
        at: OffsetDateTime,
    },
}

/// An authority relationship binding one protected surface to one writer (§4).
///
/// The three axes are split per Echo v0.1.1: `writer` names *who* proposes the
/// mutation, `holds_under` names *what load* the thread covers, and the weave's
/// `PatternPredicate` names *the gate structure*. Gates are the loom, not thread
/// fields (§2.2) — a thread never carries a gate id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    /// This thread's id.
    pub id: ThreadId,
    /// The protected surface this thread binds (typed at construction — §3.3 #1).
    pub surface: SurfaceId,
    /// The writer this thread extends authority to.
    pub writer: WriterId,
    /// The strands (fibers of commitment) making up this thread (§2.3).
    pub strands: Vec<Strand>,
    /// The channels this thread must survive (§4). A channel not listed here is
    /// not covered by this thread — fail-closed (`FrayOrSnap::NotCovered`).
    pub holds_under: Vec<Channel>,
    /// When this thread was woven.
    pub created_at: OffsetDateTime,
    /// Current tension state.
    pub tension: TensionState,
}

impl Thread {
    /// Does this thread hold under this channel? The load-bearing question (§2.1).
    ///
    /// Fail-closed on every axis (§5, RFC-0001 §5.4 Gate 4):
    /// - a channel this thread does not cover → `NotCovered`;
    /// - a snapped thread → `Snapped`, whatever the channel;
    /// - a frayed thread → `Frayed`, whatever the channel;
    /// - a covered channel whose structural strand requirements are unmet
    ///   (e.g. `Serialization` without a `SerializationMarker`, C7 §2.4;
    ///   `Forced` without hash + external manifest, §2.4) → `Frayed` with
    ///   `RequiredStrandMissing`.
    ///
    /// This method checks *structural* state: strand presence, tension, coverage.
    /// Verifying strand *content* against the world (filesystem hashes, manifest
    /// stores, signatures) is the daemon verifier's lane (Phase 2) — it feeds
    /// results back by setting `tension`.
    pub fn holds_under(&self, channel: Channel) -> Result<(), FrayOrSnap> {
        // Terminal state first: a snapped thread holds nowhere.
        if let TensionState::Snapped {
            channel: c, reason, ..
        } = &self.tension
        {
            return Err(FrayOrSnap::Snapped {
                channel: *c,
                reason: reason.clone(),
            });
        }

        // Fail-closed coverage: authority extends only over listed channels (§4).
        if !self.holds_under.contains(&channel) {
            return Err(FrayOrSnap::NotCovered { channel });
        }

        // A frayed thread surfaces its fray on every covered channel (§2.3).
        if let TensionState::Frayed {
            strand,
            channel: c,
            reason,
            ..
        } = &self.tension
        {
            return Err(FrayOrSnap::Frayed {
                strand: *strand,
                channel: *c,
                reason: reason.clone(),
            });
        }

        // Structural per-channel strand requirements (§2.4).
        for kind in channel.required_strand_kinds() {
            if !self.strands.iter().any(|s| s.kind() == *kind) {
                return Err(FrayOrSnap::Frayed {
                    strand: None,
                    channel,
                    reason: FrayReason::RequiredStrandMissing { kind: *kind },
                });
            }
        }

        Ok(())
    }

    /// Current tension state (§2.1: threads are first-class inspectable).
    pub fn tension(&self) -> &TensionState {
        &self.tension
    }

    /// Mark this thread frayed at a strand. Used by the daemon verifier lane when
    /// strand-content verification fails against the world.
    pub fn fray(
        &mut self,
        strand: Option<StrandId>,
        channel: Channel,
        reason: FrayReason,
        detected_at: OffsetDateTime,
    ) {
        // A snap is terminal; fraying cannot resurrect it.
        if matches!(self.tension, TensionState::Snapped { .. }) {
            return;
        }
        self.tension = TensionState::Frayed {
            strand,
            channel,
            reason,
            detected_at,
        };
    }

    /// Mark this thread snapped (terminal).
    pub fn snap(&mut self, channel: Channel, reason: SnapReason, at: OffsetDateTime) {
        self.tension = TensionState::Snapped {
            channel,
            reason,
            at,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ManifestId;
    use crate::strand::{HashAlgo, StrandKind};

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn content_hash() -> Strand {
        Strand::ContentHash {
            id: StrandId::new(),
            algorithm: HashAlgo::Blake3,
            value: vec![0xab; 32],
        }
    }

    fn manifest_entry() -> Strand {
        Strand::ManifestEntry {
            id: StrandId::new(),
            manifest_id: ManifestId::new(),
            entry_hash: vec![0xcd; 32],
        }
    }

    fn serialization_marker() -> Strand {
        Strand::SerializationMarker {
            id: StrandId::new(),
            format_version: "0.1.0".into(),
            contract_hash: vec![0xef; 32],
        }
    }

    fn thread(strands: Vec<Strand>, holds_under: Vec<Channel>) -> Thread {
        Thread {
            id: ThreadId::new(),
            surface: SurfaceId::new("SOUL.md"),
            writer: WriterId::new("sage"),
            strands,
            holds_under,
            created_at: now(),
            tension: TensionState::Holds,
        }
    }

    #[test]
    fn holds_on_covered_channel_with_required_strands() {
        let t = thread(vec![content_hash()], vec![Channel::Mutation]);
        assert!(t.holds_under(Channel::Mutation).is_ok());
    }

    #[test]
    fn uncovered_channel_fails_closed() {
        // §4: holds_under lists the channels this thread must survive. Anything
        // else is NotCovered — never a silent pass. This is the fail-open trap the
        // v0.1.1 review caught: absence of coverage must read as "no", not "n/a".
        let t = thread(vec![content_hash()], vec![Channel::Mutation]);
        let err = t.holds_under(Channel::Forced).unwrap_err();
        assert_eq!(
            err,
            FrayOrSnap::NotCovered {
                channel: Channel::Forced
            }
        );
    }

    #[test]
    fn forced_channel_requires_hash_and_manifest() {
        // §2.4: Forced-channel threads MUST have strands surviving without
        // agent-side intervention (typically hash + external manifest).
        let missing_manifest = thread(vec![content_hash()], vec![Channel::Forced]);
        match missing_manifest.holds_under(Channel::Forced) {
            Err(FrayOrSnap::Frayed {
                reason: FrayReason::RequiredStrandMissing { kind },
                channel: Channel::Forced,
                ..
            }) => assert_eq!(kind, StrandKind::ManifestEntry),
            other => panic!("expected RequiredStrandMissing manifest, got {other:?}"),
        }

        let complete = thread(
            vec![content_hash(), manifest_entry()],
            vec![Channel::Forced],
        );
        assert!(complete.holds_under(Channel::Forced).is_ok());
    }

    #[test]
    fn serialization_channel_requires_marker_strand() {
        // §2.4 / C7: threads on Channel::Serialization MUST carry a
        // SerializationMarker strand.
        let missing = thread(vec![content_hash()], vec![Channel::Serialization]);
        match missing.holds_under(Channel::Serialization) {
            Err(FrayOrSnap::Frayed {
                reason: FrayReason::RequiredStrandMissing { kind },
                ..
            }) => assert_eq!(kind, StrandKind::SerializationMarker),
            other => panic!("expected RequiredStrandMissing marker, got {other:?}"),
        }

        let complete = thread(vec![serialization_marker()], vec![Channel::Serialization]);
        assert!(complete.holds_under(Channel::Serialization).is_ok());
    }

    #[test]
    fn deliberate_channel_has_no_structural_floor() {
        let t = thread(vec![], vec![Channel::Deliberate]);
        assert!(t.holds_under(Channel::Deliberate).is_ok());
    }

    #[test]
    fn frayed_thread_surfaces_fray_on_every_covered_channel() {
        let mut t = thread(
            vec![content_hash(), manifest_entry()],
            vec![Channel::Mutation, Channel::Forced],
        );
        let s_id = t.strands[0].id();
        t.fray(
            Some(s_id),
            Channel::Forced,
            FrayReason::ContentHashMismatch,
            now(),
        );
        for c in [Channel::Mutation, Channel::Forced] {
            match t.holds_under(c) {
                Err(FrayOrSnap::Frayed { strand, .. }) => assert_eq!(strand, Some(s_id)),
                other => panic!("expected fray on {c:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn snapped_thread_holds_nowhere_even_uncovered_channels() {
        let mut t = thread(vec![content_hash()], vec![Channel::Mutation]);
        t.snap(Channel::Mutation, SnapReason::Revoked, now());
        for c in Channel::ALL {
            match t.holds_under(*c) {
                Err(FrayOrSnap::Snapped {
                    reason: SnapReason::Revoked,
                    ..
                }) => {}
                other => panic!("expected snap on {c:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn snap_is_terminal_fray_cannot_resurrect() {
        let mut t = thread(vec![content_hash()], vec![Channel::Mutation]);
        t.snap(Channel::Mutation, SnapReason::Revoked, now());
        t.fray(
            None,
            Channel::Mutation,
            FrayReason::ContentHashMismatch,
            now(),
        );
        assert!(matches!(t.tension(), TensionState::Snapped { .. }));
    }

    #[test]
    fn holds_fray_snap_exercised_under_all_four_channels() {
        // Phase 1 exit criterion: hold/fray/snap semantics under all four channels.
        for channel in Channel::ALL {
            let strands = || -> Vec<Strand> {
                vec![content_hash(), manifest_entry(), serialization_marker()]
            };
            // Hold.
            let healthy = thread(strands(), vec![*channel]);
            assert!(
                healthy.holds_under(*channel).is_ok(),
                "fully-strung thread must hold under {channel:?}"
            );
            // Fray.
            let mut frayed = thread(strands(), vec![*channel]);
            frayed.fray(None, *channel, FrayReason::Other("test".into()), now());
            assert!(
                matches!(frayed.holds_under(*channel), Err(FrayOrSnap::Frayed { .. })),
                "frayed thread must surface fray under {channel:?}"
            );
            // Snap.
            let mut snapped = thread(strands(), vec![*channel]);
            snapped.snap(*channel, SnapReason::Revoked, now());
            assert!(
                matches!(
                    snapped.holds_under(*channel),
                    Err(FrayOrSnap::Snapped { .. })
                ),
                "snapped thread must surface snap under {channel:?}"
            );
        }
    }

    #[test]
    fn thread_roundtrips_json_with_tension_intact() {
        // §3.3 invariant #4 groundwork: a thread's full state — strands, coverage,
        // tension — survives serde round-trip byte-for-byte equal.
        let mut t = thread(
            vec![content_hash(), serialization_marker()],
            vec![Channel::Mutation, Channel::Serialization],
        );
        t.fray(
            Some(t.strands[0].id()),
            Channel::Mutation,
            FrayReason::ContentHashMismatch,
            now(),
        );
        let json = serde_json::to_string(&t).unwrap();
        let back: Thread = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
