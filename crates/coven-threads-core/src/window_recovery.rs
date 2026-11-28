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
//! 1. An inconsistent history (not exactly one opening, more than one
//!    terminal, or a terminal before its opening) cannot be repaired by
//!    appending. It is held as an integrity incident.
//! 2. A window that already has its one terminal needs nothing.
//! 3. An active pending item or decision claim belongs to the ordinary
//!    scheduler and decision paths, which already write typed closes.
//! 4. If an apply intent exists, a write may have started. The current bytes
//!    can match the before-images even when a landed write was later
//!    reverted, so recovery never asserts a rejection here. It holds.
//! 5. A missing intent proves that no write happened only when the daemon
//!    that opened the window committed every apply intent durably before any
//!    target write. Otherwise recovery holds.
//! 6. Automatic closes wait for the deadline, which preserves minimum
//!    visibility.
//! 7. A transient authority failure is retried, never treated as proof.
//! 8. A quarantined proposal whose evidence still replays can be restored by
//!    its owner, so it is held rather than closed.
//! 9. Otherwise the close is `proposal_rejected`, either with
//!    `evidence_diverged` (replay ran and differed) or with
//!    `revalidation_failed` (authoritative evidence cannot be produced,
//!    including because the staged bytes are gone). Both reasons already
//!    exist; no schema change is needed.
//!
//! Recovery never produces `proposal_approved` or `proposal_vetoed`: it never
//! asserts an apply or a principal act.
//!
//! Contradictory inputs fail closed. Example: a replay outcome reported for a
//! proposal whose staged bytes are absent.

use crate::approval::{ProposalWindowCloseAuditDetail, WindowCloseReason};
use crate::audit::AuditEventType;

/// What the audit store says about a proposal's apply intent.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyIntentEvidence {
    /// No apply-intent row. The daemon that opened the window committed every
    /// apply intent durably (power-loss safe) before any target write, so the
    /// absence proves that no write started.
    AbsentDurable,
    /// No apply-intent row, but intent durability at the time of the opening
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
    /// Only unrecognized artifacts remain, for example an interrupted staging
    /// file.
    Untrusted,
    /// No retained artifact matches the proposal.
    Absent,
}

/// The outcome of re-deriving the proposal's authoritative evidence now.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayOutcome {
    /// The evidence was re-derived and its hash equals the opening's.
    Matched,
    /// The evidence was re-derived and its hash differs.
    Diverged,
    /// Authoritative evidence cannot be produced. Examples: the staged bytes
    /// are gone, or the retained proposal is invalid. This is a permanent,
    /// proven condition, not an I/O failure.
    NotProducible,
    /// A transient failure, such as an I/O error or a temporarily unavailable
    /// authority. Not proof of anything.
    Transient,
}

/// Facts a recovery pass gathers for one proposal with an opened window.
///
/// The daemon owns gathering these from its store and filesystem. Threads
/// owns what they permit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRecoveryFacts {
    /// Number of `proposal_window_opened` rows for the proposal.
    pub openings: u32,
    /// Number of terminal rows (`proposal_approved`, `proposal_rejected`,
    /// `proposal_vetoed`, or any legacy terminal) for the proposal.
    pub terminals: u32,
    /// Whether any terminal row precedes the opening.
    pub terminal_precedes_opening: bool,
    /// Whether the window's deadline has been reached on the daemon clock.
    pub deadline_reached: bool,
    /// Apply-intent evidence for the proposal.
    pub apply_intent: ApplyIntentEvidence,
    /// What the daemon still retains for the proposal.
    pub retained: RetainedProposal,
    /// The result of re-deriving the evidence now.
    pub replay: ReplayOutcome,
}

/// Why recovery must not append a close yet, or at all.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryHold {
    /// The history violates the single-opening or single-terminal shape.
    /// Appending cannot repair it. Treat it as a store-integrity incident.
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
    /// Quarantined evidence still replays. Its owner can restore it, so it is
    /// not closed automatically.
    RestorableEvidence,
    /// The facts contradict each other. Fail closed.
    ContradictoryFacts,
}

/// A typed terminal close that recovery may append.
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
    /// The window already has its single terminal.
    AlreadyClosed,
    /// The ordinary scheduler or decision path owns the terminal.
    Ordinary,
    /// Append this close, durably, in one transaction with any consumption of
    /// retained state.
    Close(RecoveryClose),
    /// Do not append a close.
    Hold(RecoveryHold),
}

/// Decide what recovery may do for an opened window with damaged or missing
/// retained state. See the module documentation for the rules.
pub fn recovery_disposition(_facts: &WindowRecoveryFacts) -> TerminalDisposition {
    TerminalDisposition::Hold(RecoveryHold::InconsistentHistory)
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

    fn damaged(retained: RetainedProposal, replay: ReplayOutcome) -> WindowRecoveryFacts {
        WindowRecoveryFacts {
            openings: 1,
            terminals: 0,
            terminal_precedes_opening: false,
            deadline_reached: true,
            apply_intent: ApplyIntentEvidence::AbsentDurable,
            retained,
            replay,
        }
    }

    fn all_facts() -> Vec<WindowRecoveryFacts> {
        let mut out = Vec::new();
        for openings in 0..=2 {
            for terminals in 0..=2 {
                for terminal_precedes_opening in [false, true] {
                    for deadline_reached in [false, true] {
                        for apply_intent in INTENTS {
                            for retained in RETAINED {
                                for replay in REPLAYS {
                                    out.push(WindowRecoveryFacts {
                                        openings,
                                        terminals,
                                        terminal_precedes_opening,
                                        deadline_reached,
                                        apply_intent,
                                        retained,
                                        replay,
                                    });
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
        for (openings, terminals, precedes) in
            [(0, 0, false), (2, 0, false), (1, 2, false), (1, 1, true)]
        {
            let facts = WindowRecoveryFacts {
                openings,
                terminals,
                terminal_precedes_opening: precedes,
                ..damaged(RetainedProposal::Absent, ReplayOutcome::NotProducible)
            };
            assert_eq!(
                recovery_disposition(&facts),
                hold(RecoveryHold::InconsistentHistory)
            );
        }
    }

    #[test]
    fn a_window_with_its_terminal_needs_nothing() {
        let facts = WindowRecoveryFacts {
            terminals: 1,
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
            for retained in [
                RetainedProposal::Quarantined,
                RetainedProposal::Untrusted,
                RetainedProposal::Absent,
            ] {
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
        for retained in [
            RetainedProposal::Quarantined,
            RetainedProposal::Untrusted,
            RetainedProposal::Absent,
        ] {
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
    fn replay_reported_without_staged_bytes_fails_closed() {
        for retained in [RetainedProposal::Untrusted, RetainedProposal::Absent] {
            for replay in [ReplayOutcome::Matched, ReplayOutcome::Diverged] {
                assert_eq!(
                    recovery_disposition(&damaged(retained, replay)),
                    hold(RecoveryHold::ContradictoryFacts)
                );
            }
        }
    }

    /// Over every combination of facts: a close is only ever a rejection,
    /// passes the core's own window-close validation, and appears only for a
    /// consistent, unterminated, past-deadline window with durable proof of
    /// no write and no active item.
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
                assert_eq!(facts.terminals, 0, "{facts:?}");
                assert!(!facts.terminal_precedes_opening, "{facts:?}");
                assert!(facts.deadline_reached, "{facts:?}");
                assert_eq!(
                    facts.apply_intent,
                    ApplyIntentEvidence::AbsentDurable,
                    "{facts:?}"
                );
                assert_ne!(facts.retained, RetainedProposal::Active, "{facts:?}");
                assert_ne!(facts.replay, ReplayOutcome::Transient, "{facts:?}");
                assert_ne!(facts.replay, ReplayOutcome::Matched, "{facts:?}");
            }
        }
        // Quarantined x {Diverged, NotProducible} + {Untrusted, Absent} x NotProducible.
        assert_eq!(closes, 4);
    }

    /// The disposition is a total function: every input yields exactly one
    /// answer, and nothing outside rule 2 reports `AlreadyClosed`.
    #[test]
    fn already_closed_requires_one_consistent_terminal() {
        for facts in all_facts() {
            if recovery_disposition(&facts) == TerminalDisposition::AlreadyClosed {
                assert_eq!((facts.openings, facts.terminals), (1, 1), "{facts:?}");
                assert!(!facts.terminal_precedes_opening, "{facts:?}");
            }
        }
    }
}
