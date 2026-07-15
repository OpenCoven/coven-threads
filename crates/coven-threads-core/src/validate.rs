//! The gate-shaped receiver (§1, §5): `validate` — the single entry point the
//! `coven` daemon calls on identity-surface mutation requests.
//!
//! Enforcement flow (§5): untrusted client → `coven` daemon → daemon calls
//! `validate(weave, request)` → validator checks the affected thread's strands
//! under the request's channel → returns `Permit` / `DegradeToProposal` /
//! `Reject` → daemon acts and appends to `ward.audit` in `coven.sqlite3` (§3.4).
//!
//! **Fail-closed on every unknown** (Nova non-negotiable #2, RFC-0001 §5.4
//! Gate 4 conformance — line one, not hardening):
//! - Unknown surface path → `Reject`.
//! - Unknown thread for a protected surface → `Reject` (all protected surfaces
//!   MUST have threads).
//! - Unknown/uncovered channel → `Reject`.
//! - Validator panic → `Reject` with diagnostic ([`validate_fail_closed`];
//!   the daemon must also catch at its boundary — defense in depth).
//!
//! This module gates on `Weave::coherence()` — the predicate. It never reads a
//! `PatternDescriptor` (§2.2 anti-pattern: descriptors are for legibility,
//! predicates are for enforcement).
//!
//! This crate is reachable *only* by the privileged daemon, never by a
//! familiar-controlled process (RFC-0001 §5.1: F MUST NOT modify the Ward file,
//! restart the authority process, or bypass gates).

use serde::{Deserialize, Serialize};

use crate::channel::Channel;
use crate::fray::{FrayOrSnap, SnapReason};
use crate::ids::{SurfaceId, ThreadId, WriterId};
use crate::pattern::WeaveCoherence;
use crate::weave::Weave;

/// A mutation request as the daemon presents it to the gate layer (§5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRequest {
    /// The protected surface the mutation targets.
    pub surface: SurfaceId,
    /// The writer proposing the mutation.
    pub writer: WriterId,
    /// The channel the mutation arrives on (§2.4).
    pub channel: Channel,
}

/// The validator's verdict (§5). The daemon acts on this and audits it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// The thread holds and the weave is coherent at this surface: apply the
    /// mutation.
    Permit {
        /// The thread that carried the authority.
        thread: ThreadId,
    },

    /// Frayed thread (§5): write staged to `~/.coven/pending/`, notification to
    /// principal, no immediate write to the protected surface. Staging mechanics
    /// are the daemon's lane (Phase 2); this crate names the outcome.
    DegradeToProposal {
        /// The frayed thread.
        thread: ThreadId,
        /// The fray that triggered degradation.
        fray: FrayOrSnap,
    },

    /// The mutation must not happen. Always carries a named reason for
    /// `ward.audit`.
    Reject {
        /// Why.
        reason: RejectReason,
    },
}

/// Why a mutation was rejected (§5 failure modes, audit-legible).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    /// No thread binds this surface at all — unknown surface, or a protected
    /// surface missing its threads. Both fail closed (§5).
    UnknownSurface {
        /// The unknown surface.
        surface: SurfaceId,
    },
    /// The surface is known, but no thread extends authority to this writer.
    WriterNotBound {
        /// The surface.
        surface: SurfaceId,
        /// The writer with no thread.
        writer: WriterId,
    },
    /// The `(surface, writer)` thread exists but does not cover this channel.
    ChannelNotCovered {
        /// The thread.
        thread: ThreadId,
        /// The uncovered channel.
        channel: Channel,
    },
    /// The thread snapped (§5): surface read-only until repair.
    ThreadSnapped {
        /// The snapped thread.
        thread: ThreadId,
        /// The snap.
        reason: SnapReason,
    },
    /// The weave's pattern predicate ruled the weave broken: no authority can
    /// be exercised through it.
    WeaveBroken {
        /// The predicate's diagnostic.
        reason: String,
    },
    /// The weave is degraded at this surface (§5): read-only until repair.
    SurfaceDegraded {
        /// The degraded surface.
        surface: SurfaceId,
        /// The predicate's diagnostic.
        reason: String,
    },
    /// The validator panicked; fail-closed with diagnostic (§5).
    ValidatorPanic {
        /// Best-effort panic message.
        diagnostic: String,
    },
}

/// Validate one mutation request against a weave (§5). The gate-shaped receiver.
///
/// Check order is fail-closed at every step; nothing falls through to Permit:
/// 1. Surface unknown to the weave → `Reject(UnknownSurface)`.
/// 2. No thread for `(surface, writer)` → `Reject(WriterNotBound)`.
/// 3. `thread.holds_under(channel)`:
///    - `NotCovered` → `Reject(ChannelNotCovered)`,
///    - `Snapped` → `Reject(ThreadSnapped)`,
///    - `Frayed` → `DegradeToProposal` (§5: staged, principal notified),
/// 4. Weave coherence (the predicate — never the descriptor):
///    - `Broken` → `Reject(WeaveBroken)`,
///    - `Degraded` at this surface → `Reject(SurfaceDegraded)`,
///    - otherwise → `Permit`.
pub fn validate(weave: &Weave, request: &MutationRequest) -> Verdict {
    // 1. Unknown surface → Reject (§5).
    if !weave.covers_surface(&request.surface) {
        return Verdict::Reject {
            reason: RejectReason::UnknownSurface {
                surface: request.surface.clone(),
            },
        };
    }

    // 2. No thread for (surface, writer) → Reject. Authority is per-pair (§2.1);
    // a writer without a thread has no path, whoever else is bound.
    let Some(thread) = weave.thread_for(&request.surface, &request.writer) else {
        return Verdict::Reject {
            reason: RejectReason::WriterNotBound {
                surface: request.surface.clone(),
                writer: request.writer.clone(),
            },
        };
    };

    // 3. The load-bearing question (§2.1): does thread T hold under channel C?
    match thread.holds_under(request.channel) {
        Err(FrayOrSnap::NotCovered { channel }) => {
            return Verdict::Reject {
                reason: RejectReason::ChannelNotCovered {
                    thread: thread.id,
                    channel,
                },
            };
        }
        Err(FrayOrSnap::Snapped { reason, .. }) => {
            return Verdict::Reject {
                reason: RejectReason::ThreadSnapped {
                    thread: thread.id,
                    reason,
                },
            };
        }
        Err(fray @ FrayOrSnap::Frayed { .. }) => {
            // §5: frayed → stage as proposal, notify principal, no direct write.
            return Verdict::DegradeToProposal {
                thread: thread.id,
                fray,
            };
        }
        Ok(()) => {}
    }

    // 4. The weave-level gate: predicate authoritative (§2.2).
    match weave.coherence() {
        WeaveCoherence::Broken { reason } => Verdict::Reject {
            reason: RejectReason::WeaveBroken { reason },
        },
        WeaveCoherence::Degraded {
            degraded_surfaces,
            reason,
        } if degraded_surfaces.contains(&request.surface) => Verdict::Reject {
            reason: RejectReason::SurfaceDegraded {
                surface: request.surface.clone(),
                reason,
            },
        },
        // Degraded elsewhere: §5 — the familiar continues on other surfaces.
        WeaveCoherence::Degraded { .. } | WeaveCoherence::Coherent => {
            Verdict::Permit { thread: thread.id }
        }
    }
}

/// [`validate`], catching panics and converting them to `Reject` (§5:
/// "Validator panic → daemon catches and treats as Reject with diagnostic").
///
/// `PatternPredicate` implementations are externally definable (§4); a panicking
/// predicate must not become a bypass. The daemon should still catch at its own
/// boundary — this is defense in depth, not a substitute.
pub fn validate_fail_closed(weave: &Weave, request: &MutationRequest) -> Verdict {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| validate(weave, request)));
    match result {
        Ok(verdict) => verdict,
        Err(panic) => {
            let diagnostic = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "opaque panic payload".to_string());
            Verdict::Reject {
                reason: RejectReason::ValidatorPanic { diagnostic },
            }
        }
    }
}

/// Convenience mirror of §5's fray outcome for audit writers: whether a verdict
/// requires staging to `~/.coven/pending/`.
impl Verdict {
    /// Does this verdict permit the write to proceed immediately?
    pub fn permits_write(&self) -> bool {
        matches!(self, Verdict::Permit { .. })
    }

    /// Does this verdict require staging as a proposal?
    pub fn requires_staging(&self) -> bool {
        matches!(self, Verdict::DegradeToProposal { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fray::FrayReason;
    use crate::ids::{FamiliarId, ManifestId, StrandId, WeaveId};
    use crate::pattern::{AllSurfacesHoldOnChannels, PatternDescriptor, PatternPredicate};
    use crate::strand::{HashAlgo, Strand};
    use crate::thread::{TensionState, Thread};
    use time::OffsetDateTime;

    fn full_strands() -> Vec<Strand> {
        vec![
            Strand::ContentHash {
                id: StrandId::new(),
                algorithm: HashAlgo::Blake3,
                value: vec![1; 32],
            },
            Strand::ManifestEntry {
                id: StrandId::new(),
                manifest_id: ManifestId::new(),
                entry_hash: vec![2; 32],
            },
            Strand::SerializationMarker {
                id: StrandId::new(),
                format_version: "0.1.0".into(),
                contract_hash: vec![3; 32],
            },
        ]
    }

    fn floor_thread(surface: &str, writer: &str) -> Thread {
        Thread {
            id: ThreadId::new(),
            surface: SurfaceId::new(surface),
            writer: WriterId::new(writer),
            strands: full_strands(),
            holds_under: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
            created_at: OffsetDateTime::now_utc(),
            tension: TensionState::Holds,
        }
    }

    fn floor_weave() -> Weave {
        Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            ["SOUL.md", "IDENTITY.md", "MEMORY.md", "ward.toml"]
                .into_iter()
                .map(|s| floor_thread(s, "principal:val"))
                .collect(),
            Box::new(AllSurfacesHoldOnChannels::rfc0001_floor()),
            None,
        )
        .unwrap()
    }

    fn request(surface: &str, writer: &str, channel: Channel) -> MutationRequest {
        MutationRequest {
            surface: SurfaceId::new(surface),
            writer: WriterId::new(writer),
            channel,
        }
    }

    #[test]
    fn bound_writer_on_covered_channel_permits() {
        let w = floor_weave();
        let v = validate(&w, &request("SOUL.md", "principal:val", Channel::Mutation));
        assert!(v.permits_write(), "expected Permit, got {v:?}");
    }

    #[test]
    fn unknown_surface_rejects() {
        // §5: Unknown surface path → Reject.
        let w = floor_weave();
        let v = validate(&w, &request("EVIL.md", "principal:val", Channel::Mutation));
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::UnknownSurface { .. }
                }
            ),
            "got {v:?}"
        );
    }

    #[test]
    fn unbound_writer_rejects() {
        // RFC-0001 §5.1: familiar-controlled processes have no write path. A
        // writer with no thread gets Reject, not a downgrade.
        let w = floor_weave();
        let v = validate(&w, &request("SOUL.md", "familiar:sage", Channel::Mutation));
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::WriterNotBound { .. }
                }
            ),
            "got {v:?}"
        );
    }

    #[test]
    fn uncovered_channel_rejects() {
        // §5: Unknown channel → Reject. Deliberate is not in the floor threads'
        // holds_under set.
        let w = floor_weave();
        let v = validate(
            &w,
            &request("SOUL.md", "principal:val", Channel::Deliberate),
        );
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::ChannelNotCovered { .. }
                }
            ),
            "got {v:?}"
        );
    }

    #[test]
    fn frayed_thread_degrades_to_proposal() {
        // §5: Frayed thread → DegradeToProposal.
        let mut w = floor_weave();
        w.update_threads(|threads| {
            let strand_id = threads[0].strands[0].id();
            let channel = Channel::Mutation;
            threads[0].fray(
                Some(strand_id),
                channel,
                FrayReason::ContentHashMismatch,
                OffsetDateTime::now_utc(),
            );
        })
        .unwrap();
        let frayed_surface = w.threads()[0].surface.clone();
        let v = validate(
            &w,
            &request(frayed_surface.as_str(), "principal:val", Channel::Mutation),
        );
        assert!(
            v.requires_staging(),
            "expected DegradeToProposal, got {v:?}"
        );
    }

    #[test]
    fn snapped_thread_rejects_and_other_surfaces_continue() {
        // §5: Snapped thread → Reject; familiar continues on other surfaces.
        let mut w = floor_weave();
        w.update_threads(|threads| {
            threads[0].snap(
                Channel::Mutation,
                SnapReason::Revoked,
                OffsetDateTime::now_utc(),
            );
        })
        .unwrap();
        let snapped_surface = w.threads()[0].surface.clone();

        let v = validate(
            &w,
            &request(snapped_surface.as_str(), "principal:val", Channel::Mutation),
        );
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::ThreadSnapped { .. }
                }
            ),
            "got {v:?}"
        );

        // Any other floor surface still permits: degradation is local (§2.2).
        let other = w
            .threads()
            .iter()
            .find(|t| t.surface != snapped_surface)
            .map(|t| t.surface.clone())
            .unwrap();
        let v = validate(
            &w,
            &request(other.as_str(), "principal:val", Channel::Mutation),
        );
        assert!(v.permits_write(), "expected Permit on {other}, got {v:?}");
    }

    #[test]
    fn broken_weave_rejects_everything() {
        // A weave whose pattern is Broken exercises no authority at all.
        #[derive(Debug)]
        struct AlwaysBroken;
        impl PatternPredicate for AlwaysBroken {
            fn coherent(&self, _threads: &[Thread]) -> WeaveCoherence {
                WeaveCoherence::Broken {
                    reason: "test: always broken".into(),
                }
            }
            fn describe(&self) -> PatternDescriptor {
                PatternDescriptor {
                    name: "always-broken".into(),
                    protected_surfaces: vec![],
                    channels_required: vec![],
                    strand_requirements: vec![],
                }
            }
        }

        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![floor_thread("SOUL.md", "principal:val")],
            Box::new(AlwaysBroken),
            None,
        )
        .unwrap();
        let v = validate(&w, &request("SOUL.md", "principal:val", Channel::Mutation));
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::WeaveBroken { .. }
                }
            ),
            "got {v:?}"
        );
    }

    #[test]
    fn panicking_predicate_fails_closed() {
        // §5: Validator panic → Reject with diagnostic. A panicking predicate
        // must never become a bypass (Gate 4 conformance).
        #[derive(Debug)]
        struct Panics;
        impl PatternPredicate for Panics {
            fn coherent(&self, _threads: &[Thread]) -> WeaveCoherence {
                panic!("predicate exploded");
            }
            fn describe(&self) -> PatternDescriptor {
                PatternDescriptor {
                    name: "panics".into(),
                    protected_surfaces: vec![],
                    channels_required: vec![],
                    strand_requirements: vec![],
                }
            }
        }

        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![floor_thread("SOUL.md", "principal:val")],
            Box::new(Panics),
            None,
        )
        .unwrap();
        let v = validate_fail_closed(&w, &request("SOUL.md", "principal:val", Channel::Mutation));
        match v {
            Verdict::Reject {
                reason: RejectReason::ValidatorPanic { diagnostic },
            } => assert!(diagnostic.contains("predicate exploded")),
            other => panic!("expected ValidatorPanic reject, got {other:?}"),
        }
    }

    #[test]
    fn verdict_serializes_for_audit() {
        // Verdicts land in ward.audit (§3.4); they must serialize stably.
        let v = Verdict::Reject {
            reason: RejectReason::UnknownSurface {
                surface: SurfaceId::new("EVIL.md"),
            },
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: Verdict = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }
}
