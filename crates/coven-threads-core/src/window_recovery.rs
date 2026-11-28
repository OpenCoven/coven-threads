//! Recovery disposition for an opened veto window that has no terminal event.
//!
//! **Research prototype (coven-threads#13 criterion 4).** Nothing in this
//! crate or in the daemon calls it. It encodes one candidate answer to a
//! question the Phase-5 contract leaves to recovery: when a window was opened
//! but its retained proposal state is damaged, which typed terminal close, if
//! any, may a recovery pass *truthfully* append?
//!
//! The contract stays as written: every opened veto window receives exactly
//! one typed terminal close (`specs/PHASE-5-APPROVAL-SEMANTICS.md` §4 and §6
//! decision 2), and a census classification is not a close. The schema
//! already enforces *at most one* terminal. This module addresses the other
//! half by telling the daemon when appending a close is safe and when it is
//! not.
//!
//! Rules, in order:
//!
//! 1. An inconsistent history cannot be repaired by appending. That means not
//!    exactly one opening, more than one typed terminal, any untyped or legacy
//!    terminal, any census issue (invalid record, scope mismatch, intent
//!    order), or a terminal before its opening. It is held as an integrity
//!    incident.
//! 2. A window that already has its one typed terminal needs nothing.
//! 3. An active pending item or decision claim belongs to the ordinary
//!    scheduler and decision paths, which already write typed closes.
//! 4. Any retained applying state (a claim sidecar) or apply intent means a
//!    write may have started. The current bytes can match the before-images
//!    even when a landed write was later reverted, so recovery never asserts a
//!    rejection here. It holds.
//! 5. A missing intent proves that no write happened only when the proposal
//!    was staged by a daemon that commits every apply intent durably before
//!    any target write. Otherwise recovery holds.
//! 6. The proposal's own submission receipt must match the opening, or the
//!    opening is unprovable and recovery holds.
//! 7. Automatic closes wait for the deadline, which preserves minimum
//!    visibility.
//! 8. A transient authority failure is retried, never treated as proof.
//! 9. A quarantined proposal whose evidence still replays is held rather than
//!    closed.
//! 10. Otherwise the close is `proposal_rejected`, either with
//!     `evidence_diverged` (replay ran and differed) or with
//!     `revalidation_failed` (authoritative evidence cannot be produced). Both
//!     reasons already exist; no schema change is needed.
//!
//! Recovery never produces `proposal_approved` or `proposal_vetoed`: it never
//! asserts an apply or a principal act. Contradictory inputs fail closed.
//!
//! # Caller obligations
//!
//! - Gather the facts under the daemon's single Ward write lock, and append any
//!   returned close in the same durable transaction that re-checks for an
//!   existing terminal. Filename observations are otherwise non-atomic.
//! - Start from [`WindowRecoveryFacts::conservative`]. Every field defaults to
//!   the value that cannot produce a close, so a field added later fails
//!   closed for callers that do not set it.
//! - [`TerminalDisposition`] and [`RecoveryHold`] are `#[non_exhaustive]`. A
//!   wildcard match arm must hold, never close.
//! - Preserve every retained artifact after a close, for investigation.

use crate::approval::{ProposalWindowCloseAuditDetail, WindowCloseReason};
use crate::audit::AuditEventType;

/// What the audit store says about a proposal's apply intent.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyIntentEvidence {
    /// No apply-intent row, and the proposal was **staged** by a daemon that
    /// commits every apply intent durably (power-loss safe) before any target
    /// write, so the absence proves that no write started.
    ///
    /// Establish this from a durable store marker that such a daemon wrote
    /// before staging the proposal. Do not use row timestamps or the opening
    /// row: a later daemon can write an opening lazily for a proposal an
    /// earlier daemon staged.
    AbsentDurable,
    /// No apply-intent row, but intent durability when the proposal was staged
    /// is unknown or was not guaranteed. Legacy histories fall here.
    AbsentUnproven,
    /// A valid apply-intent row exists, so a write may have started.
    Present,
    /// An apply-intent row exists but is malformed or diverges from retained
    /// state.
    Unverifiable,
}

/// The proposal state the daemon still retains for the window.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedProposal {
    /// An active pending item or decision claim. The ordinary scheduler and
    /// decision paths own its terminal.
    Active,
    /// The pending item or claim was moved to quarantine.
    Quarantined,
    /// Only artifacts that are never authoritative by policy remain, for
    /// example an unknown suffix or an interrupted staging file, even when the
    /// file holds a complete body.
    Untrusted,
    /// No retained artifact matches the proposal.
    Absent,
}

/// The outcome of re-deriving the proposal's authoritative evidence now.
///
/// With several retained copies, apply these in order: any copy that
/// re-derives to the opening's hash gives `Matched`; otherwise any transient
/// failure gives `Transient`; otherwise any copy that re-derived to a
/// different hash gives `Diverged`; otherwise `NotProducible`. Untrusted or
/// absent artifacts are never replayed, so they give `NotProducible`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayOutcome {
    /// The evidence was re-derived and its hash equals the opening's.
    Matched,
    /// The evidence was re-derived and its hash differs.
    Diverged,
    /// Authoritative evidence cannot be produced. This is a permanent, proven
    /// condition, not an I/O failure.
    NotProducible,
    /// A transient failure, such as an I/O error or a temporarily unavailable
    /// authority. Not proof of anything.
    Transient,
}

/// Facts a recovery pass gathers for one proposal with an opened window.
///
/// The daemon owns gathering these from its store and filesystem. Threads
/// owns what they permit. Construct with [`WindowRecoveryFacts::conservative`]
/// and set each field the daemon has established.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRecoveryFacts {
    /// Number of `proposal_window_opened` rows for the proposal.
    pub openings: u32,
    /// Number of typed terminal rows (`proposal_approved`,
    /// `proposal_rejected`, `proposal_vetoed`) for the proposal.
    pub typed_terminals: u32,
    /// Number of untyped or legacy terminal rows, for example
    /// `proposal_expired`, or a terminal without its window-close detail.
    pub untyped_terminals: u32,
    /// Whether the history has any other census issue: an invalid record,
    /// familiar, target or channel scope mismatch, or an apply intent outside
    /// its opening-to-terminal interval.
    pub history_issues: bool,
    /// Whether any terminal row precedes the opening.
    pub terminal_precedes_opening: bool,
    /// Whether the proposal's durable submission receipt exists and matches
    /// the opening.
    pub submission_receipt_matches: bool,
    /// Whether the window's deadline has been reached on the daemon clock.
    pub deadline_reached: bool,
    /// Apply-intent evidence for the proposal.
    pub apply_intent: ApplyIntentEvidence,
    /// Whether any retained claim or sidecar, quarantined or not, carries an
    /// applying state.
    pub applying_state_retained: bool,
    /// What the daemon still retains for the proposal.
    pub retained: RetainedProposal,
    /// The result of re-deriving the evidence now.
    pub replay: ReplayOutcome,
}

impl WindowRecoveryFacts {
    /// Facts that cannot produce a close. Callers set each field they have
    /// established; anything left unset fails closed.
    pub fn conservative() -> Self {
        Self {
            openings: 0,
            typed_terminals: 0,
            untyped_terminals: 0,
            history_issues: true,
            terminal_precedes_opening: true,
            submission_receipt_matches: false,
            deadline_reached: false,
            apply_intent: ApplyIntentEvidence::AbsentUnproven,
            applying_state_retained: true,
            retained: RetainedProposal::Absent,
            replay: ReplayOutcome::Transient,
        }
    }
}

/// Why recovery must not append a close yet, or at all.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryHold {
    /// The history is malformed (rule 1). Appending cannot repair it. Treat it
    /// as a store-integrity incident.
    InconsistentHistory,
    /// The deadline has not been reached. Minimum visibility is preserved.
    AwaitDeadline,
    /// Retry later. A transient failure is not proof.
    TransientAuthority,
    /// A missing apply intent is not proof of no write for this history.
    NoWriteUnproven,
    /// A write may have started. No existing close reason is truthful without
    /// a completed, verified apply.
    InterruptedApply,
    /// No matching submission receipt, so the opening itself is unprovable.
    UnprovenOpening,
    /// Quarantined evidence still replays. At Coven `9415c309` no path
    /// restores from quarantine, so this hold can be indefinite.
    RestorableEvidence,
    /// The facts contradict each other. Fail closed.
    ContradictoryFacts,
}

/// A typed terminal close that recovery may append.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryClose {
    /// Always [`AuditEventType::ProposalRejected`].
    pub event_type: AuditEventType,
    /// The window-close detail to store on the terminal row.
    pub detail: ProposalWindowCloseAuditDetail,
}

/// What a recovery pass may do for one opened window.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalDisposition {
    /// The window already has its single typed terminal.
    AlreadyClosed,
    /// The ordinary scheduler or decision path owns the terminal. That path can
    /// itself be blocked; this answer does not certify that it will close.
    Ordinary,
    /// Append this close, durably, in one transaction with any consumption of
    /// retained state.
    Close(RecoveryClose),
    /// Do not append a close.
    Hold(RecoveryHold),
}

/// Decide what recovery may do for an opened window with damaged or missing
/// retained state. See the module documentation for the rules and the
/// caller's obligations.
pub fn recovery_disposition(facts: &WindowRecoveryFacts) -> TerminalDisposition {
    use RecoveryHold as H;
    use TerminalDisposition as D;

    if facts.openings != 1 || facts.typed_terminals > 1 || facts.terminal_precedes_opening {
        return D::Hold(H::InconsistentHistory);
    }
    if facts.typed_terminals == 1 {
        return D::AlreadyClosed;
    }
    if facts.retained == RetainedProposal::Active {
        return D::Ordinary;
    }
    match facts.apply_intent {
        ApplyIntentEvidence::AbsentDurable => {}
        ApplyIntentEvidence::AbsentUnproven => return D::Hold(H::NoWriteUnproven),
        ApplyIntentEvidence::Present | ApplyIntentEvidence::Unverifiable => {
            return D::Hold(H::InterruptedApply)
        }
    }
    if !facts.deadline_reached {
        return D::Hold(H::AwaitDeadline);
    }
    let reason = match (facts.retained, facts.replay) {
        (_, ReplayOutcome::Transient) => return D::Hold(H::TransientAuthority),
        (RetainedProposal::Quarantined, ReplayOutcome::Matched) => {
            return D::Hold(H::RestorableEvidence)
        }
        (RetainedProposal::Quarantined, ReplayOutcome::Diverged) => {
            WindowCloseReason::EvidenceDiverged
        }
        (_, ReplayOutcome::NotProducible) => WindowCloseReason::RevalidationFailed,
        (RetainedProposal::Untrusted | RetainedProposal::Absent, _) => {
            return D::Hold(H::ContradictoryFacts)
        }
        (RetainedProposal::Active, _) => return D::Ordinary,
    };
    D::Close(RecoveryClose {
        event_type: AuditEventType::ProposalRejected,
        detail: ProposalWindowCloseAuditDetail {
            reason,
            replay_hash_matched: Some(false),
            rationale: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTENTS: [ApplyIntentEvidence; 4] = [
        ApplyIntentEvidence::AbsentDurable,
        ApplyIntentEvidence::AbsentUnproven,
        ApplyIntentEvidence::Present,
        ApplyIntentEvidence::Unverifiable,
    ];
    const RETAINED: [RetainedProposal; 4] = [
        RetainedProposal::Active,
        RetainedProposal::Quarantined,
        RetainedProposal::Untrusted,
        RetainedProposal::Absent,
    ];
    const REPLAYS: [ReplayOutcome; 4] = [
        ReplayOutcome::Matched,
        ReplayOutcome::Diverged,
        ReplayOutcome::NotProducible,
        ReplayOutcome::Transient,
    ];
    const DAMAGED: [RetainedProposal; 3] = [
        RetainedProposal::Quarantined,
        RetainedProposal::Untrusted,
        RetainedProposal::Absent,
    ];

    /// Facts for a consistent, unterminated, past-deadline window with durable
    /// proof of no write, varied only by what is retained and the replay.
    fn damaged(retained: RetainedProposal, replay: ReplayOutcome) -> WindowRecoveryFacts {
        let mut facts = WindowRecoveryFacts::conservative();
        facts.openings = 1;
        facts.history_issues = false;
        facts.terminal_precedes_opening = false;
        facts.submission_receipt_matches = true;
        facts.deadline_reached = true;
        facts.apply_intent = ApplyIntentEvidence::AbsentDurable;
        facts.applying_state_retained = false;
        facts.retained = retained;
        facts.replay = replay;
        facts
    }

    fn all_facts() -> Vec<WindowRecoveryFacts> {
        let mut out = Vec::new();
        for openings in 0..=2 {
            for typed_terminals in 0..=2 {
                for untyped_terminals in 0..=1 {
                    for bits in 0u8..32 {
                        for apply_intent in INTENTS {
                            for retained in RETAINED {
                                for replay in REPLAYS {
                                    let mut facts = WindowRecoveryFacts::conservative();
                                    facts.openings = openings;
                                    facts.typed_terminals = typed_terminals;
                                    facts.untyped_terminals = untyped_terminals;
                                    facts.history_issues = bits & 1 != 0;
                                    facts.terminal_precedes_opening = bits & 2 != 0;
                                    facts.submission_receipt_matches = bits & 4 != 0;
                                    facts.deadline_reached = bits & 8 != 0;
                                    facts.applying_state_retained = bits & 16 != 0;
                                    facts.apply_intent = apply_intent;
                                    facts.retained = retained;
                                    facts.replay = replay;
                                    out.push(facts);
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn close(reason: WindowCloseReason) -> TerminalDisposition {
        TerminalDisposition::Close(RecoveryClose {
            event_type: AuditEventType::ProposalRejected,
            detail: ProposalWindowCloseAuditDetail {
                reason,
                replay_hash_matched: Some(false),
                rationale: None,
            },
        })
    }

    fn hold(reason: RecoveryHold) -> TerminalDisposition {
        TerminalDisposition::Hold(reason)
    }

    #[test]
    fn inconsistent_histories_are_held_not_repaired() {
        let base = damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible);
        let mut cases = Vec::new();
        for openings in [0, 2] {
            cases.push(WindowRecoveryFacts { openings, ..base });
        }
        cases.push(WindowRecoveryFacts {
            typed_terminals: 2,
            ..base
        });
        cases.push(WindowRecoveryFacts {
            typed_terminals: 1,
            terminal_precedes_opening: true,
            ..base
        });
        for facts in cases {
            assert_eq!(
                recovery_disposition(&facts),
                hold(RecoveryHold::InconsistentHistory),
                "{facts:?}"
            );
        }
    }

    #[test]
    fn a_legacy_untyped_terminal_is_not_a_typed_close() {
        let base = damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible);
        for typed_terminals in [0, 1] {
            let facts = WindowRecoveryFacts {
                typed_terminals,
                untyped_terminals: 1,
                ..base
            };
            assert_eq!(
                recovery_disposition(&facts),
                hold(RecoveryHold::InconsistentHistory),
                "{facts:?}"
            );
        }
    }

    #[test]
    fn census_issues_are_held_as_inconsistent() {
        let facts = WindowRecoveryFacts {
            history_issues: true,
            ..damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible)
        };
        assert_eq!(
            recovery_disposition(&facts),
            hold(RecoveryHold::InconsistentHistory)
        );
    }

    #[test]
    fn a_window_with_its_typed_terminal_needs_nothing() {
        let facts = WindowRecoveryFacts {
            typed_terminals: 1,
            ..damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible)
        };
        assert_eq!(
            recovery_disposition(&facts),
            TerminalDisposition::AlreadyClosed
        );
    }

    #[test]
    fn active_items_stay_with_the_ordinary_paths() {
        for replay in REPLAYS {
            let facts = damaged(RetainedProposal::Active, replay);
            assert_eq!(recovery_disposition(&facts), TerminalDisposition::Ordinary);
        }
    }

    #[test]
    fn an_apply_intent_is_never_closed_as_a_rejection() {
        for intent in [
            ApplyIntentEvidence::Present,
            ApplyIntentEvidence::Unverifiable,
        ] {
            for retained in DAMAGED {
                for replay in REPLAYS {
                    let facts = WindowRecoveryFacts {
                        apply_intent: intent,
                        ..damaged(retained, replay)
                    };
                    assert_eq!(
                        recovery_disposition(&facts),
                        hold(RecoveryHold::InterruptedApply)
                    );
                }
            }
        }
    }

    /// A pre-durability daemon can lose an intent row while the claim's
    /// applying sidecar survives; a later daemon can then open the window
    /// lazily and quarantine the claim.
    #[test]
    fn a_retained_applying_state_is_never_closed_as_a_rejection() {
        for retained in DAMAGED {
            for replay in REPLAYS {
                let unproven = WindowRecoveryFacts {
                    apply_intent: ApplyIntentEvidence::AbsentUnproven,
                    applying_state_retained: true,
                    ..damaged(retained, replay)
                };
                assert_eq!(
                    recovery_disposition(&unproven),
                    hold(RecoveryHold::InterruptedApply),
                    "{unproven:?}"
                );
                // Under durable intents a sidecar always follows its intent row.
                let durable = WindowRecoveryFacts {
                    applying_state_retained: true,
                    ..damaged(retained, replay)
                };
                assert_eq!(
                    recovery_disposition(&durable),
                    hold(RecoveryHold::ContradictoryFacts),
                    "{durable:?}"
                );
            }
        }
    }

    #[test]
    fn missing_intent_without_durable_journal_is_not_proof() {
        let facts = WindowRecoveryFacts {
            apply_intent: ApplyIntentEvidence::AbsentUnproven,
            ..damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible)
        };
        assert_eq!(
            recovery_disposition(&facts),
            hold(RecoveryHold::NoWriteUnproven)
        );
    }

    #[test]
    fn an_opening_without_its_submission_receipt_is_unproven() {
        for retained in DAMAGED {
            let facts = WindowRecoveryFacts {
                submission_receipt_matches: false,
                ..damaged(retained, ReplayOutcome::NotProducible)
            };
            assert_eq!(
                recovery_disposition(&facts),
                hold(RecoveryHold::UnprovenOpening)
            );
        }
    }

    #[test]
    fn automatic_closes_wait_for_the_deadline() {
        let facts = WindowRecoveryFacts {
            deadline_reached: false,
            ..damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible)
        };
        assert_eq!(
            recovery_disposition(&facts),
            hold(RecoveryHold::AwaitDeadline)
        );
    }

    #[test]
    fn transient_failures_are_retried() {
        for retained in DAMAGED {
            let facts = damaged(retained, ReplayOutcome::Transient);
            assert_eq!(
                recovery_disposition(&facts),
                hold(RecoveryHold::TransientAuthority)
            );
        }
    }

    #[test]
    fn quarantined_windows_close_by_replay_outcome() {
        assert_eq!(
            recovery_disposition(&damaged(
                RetainedProposal::Quarantined,
                ReplayOutcome::Matched
            )),
            hold(RecoveryHold::RestorableEvidence)
        );
        assert_eq!(
            recovery_disposition(&damaged(
                RetainedProposal::Quarantined,
                ReplayOutcome::Diverged
            )),
            close(WindowCloseReason::EvidenceDiverged)
        );
        assert_eq!(
            recovery_disposition(&damaged(
                RetainedProposal::Quarantined,
                ReplayOutcome::NotProducible
            )),
            close(WindowCloseReason::RevalidationFailed)
        );
    }

    #[test]
    fn orphaned_and_untrusted_windows_close_as_revalidation_failed() {
        for retained in [RetainedProposal::Untrusted, RetainedProposal::Absent] {
            assert_eq!(
                recovery_disposition(&damaged(retained, ReplayOutcome::NotProducible)),
                close(WindowCloseReason::RevalidationFailed)
            );
        }
    }

    #[test]
    fn replay_reported_for_unreplayable_artifacts_fails_closed() {
        for retained in [RetainedProposal::Untrusted, RetainedProposal::Absent] {
            for replay in [ReplayOutcome::Matched, ReplayOutcome::Diverged] {
                assert_eq!(
                    recovery_disposition(&damaged(retained, replay)),
                    hold(RecoveryHold::ContradictoryFacts)
                );
            }
        }
    }

    /// The conservative starting point holds, and so does every fact set that
    /// leaves any one close precondition at its conservative value.
    #[test]
    fn conservative_facts_never_close() {
        assert!(matches!(
            recovery_disposition(&WindowRecoveryFacts::conservative()),
            TerminalDisposition::Hold(_)
        ));
        let closing = damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible);
        assert!(matches!(
            recovery_disposition(&closing),
            TerminalDisposition::Close(_)
        ));
        let conservative = WindowRecoveryFacts::conservative();
        let one_field_reverted = [
            WindowRecoveryFacts {
                openings: conservative.openings,
                ..closing
            },
            WindowRecoveryFacts {
                history_issues: conservative.history_issues,
                ..closing
            },
            WindowRecoveryFacts {
                terminal_precedes_opening: conservative.terminal_precedes_opening,
                ..closing
            },
            WindowRecoveryFacts {
                submission_receipt_matches: conservative.submission_receipt_matches,
                ..closing
            },
            WindowRecoveryFacts {
                deadline_reached: conservative.deadline_reached,
                ..closing
            },
            WindowRecoveryFacts {
                apply_intent: conservative.apply_intent,
                ..closing
            },
            WindowRecoveryFacts {
                applying_state_retained: conservative.applying_state_retained,
                ..closing
            },
            WindowRecoveryFacts {
                replay: conservative.replay,
                ..closing
            },
        ];
        for facts in one_field_reverted {
            assert!(
                matches!(recovery_disposition(&facts), TerminalDisposition::Hold(_)),
                "{facts:?}"
            );
        }
    }

    /// Over every combination of facts, a close is only ever a rejection,
    /// passes the core's own window-close validation, and appears only when
    /// every precondition holds.
    #[test]
    fn every_close_is_a_validated_rejection_under_its_preconditions() {
        let mut closes = 0;
        for facts in all_facts() {
            if let TerminalDisposition::Close(close) = recovery_disposition(&facts) {
                closes += 1;
                assert_eq!(
                    close.event_type,
                    AuditEventType::ProposalRejected,
                    "{facts:?}"
                );
                crate::audit::validate_window_close_detail(&close.event_type, &close.detail)
                    .unwrap_or_else(|error| panic!("{facts:?}: {error}"));
                assert_eq!(facts.openings, 1, "{facts:?}");
                assert_eq!(facts.typed_terminals, 0, "{facts:?}");
                assert_eq!(facts.untyped_terminals, 0, "{facts:?}");
                assert!(!facts.history_issues, "{facts:?}");
                assert!(!facts.terminal_precedes_opening, "{facts:?}");
                assert!(facts.submission_receipt_matches, "{facts:?}");
                assert!(facts.deadline_reached, "{facts:?}");
                assert_eq!(
                    facts.apply_intent,
                    ApplyIntentEvidence::AbsentDurable,
                    "{facts:?}"
                );
                assert!(!facts.applying_state_retained, "{facts:?}");
                assert_ne!(facts.retained, RetainedProposal::Active, "{facts:?}");
                assert_ne!(facts.replay, ReplayOutcome::Transient, "{facts:?}");
                assert_ne!(facts.replay, ReplayOutcome::Matched, "{facts:?}");
            }
        }
        // Quarantined x {Diverged, NotProducible} + {Untrusted, Absent} x NotProducible.
        assert_eq!(closes, 4);
    }

    /// `AlreadyClosed` occurs, and only for one consistent typed terminal.
    #[test]
    fn already_closed_requires_one_consistent_typed_terminal() {
        let mut already_closed = 0;
        for facts in all_facts() {
            if recovery_disposition(&facts) == TerminalDisposition::AlreadyClosed {
                already_closed += 1;
                assert_eq!((facts.openings, facts.typed_terminals), (1, 1), "{facts:?}");
                assert_eq!(facts.untyped_terminals, 0, "{facts:?}");
                assert!(!facts.history_issues, "{facts:?}");
                assert!(!facts.terminal_precedes_opening, "{facts:?}");
            }
        }
        assert!(already_closed > 0);
    }
}
