//! Exhaustive bounded exploration of [`crate::lifecycle`]: from the empty
//! chain, apply every event in a finite event space to every reachable
//! chain, record the reachable states with a witness path to each, and
//! check the safety properties on every accepted step and the invariant on
//! every reachable state.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::lifecycle::{
    apply, invariant, shape_ok, step_violation, Approval, Binding, Chain, DigestClaim, Disposition,
    Event, Kind, NoLaunch, OccurrenceDisposition, Phase, PreviousClaim, SequenceClaim, State,
    SubjectClaim, Variant,
};

/// The `(phase, disposition)` pair that is well formed for `kind`.
#[must_use]
pub const fn consistent_disposition(kind: Kind) -> (Phase, Disposition) {
    match kind {
        Kind::Request | Kind::Approve => (Phase::NotApplicable, Disposition::NotApplicable),
        Kind::Reject => (Phase::NotStarted, Disposition::NoLaunchRejected),
        Kind::Expire => (Phase::NotStarted, Disposition::NoLaunchExpired),
        Kind::Revoke => (Phase::NotStarted, Disposition::CancelBeforeLaunch),
        Kind::Consume => (Phase::Dispatching, Disposition::LaunchAuthorized),
    }
}

/// A `(phase, disposition)` pair that is malformed for every kind.
pub const INCONSISTENT_DISPOSITION: (Phase, Disposition) =
    (Phase::Running, Disposition::CancelBeforeLaunch);

/// The no-launch dispositions an event may carry, including none.
const OCCURRENCE_DISPOSITIONS: [Option<OccurrenceDisposition>; 5] = [
    None,
    Some(OccurrenceDisposition {
        kind: NoLaunch::Rejected,
        matches_approval: true,
    }),
    Some(OccurrenceDisposition {
        kind: NoLaunch::Rejected,
        matches_approval: false,
    }),
    Some(OccurrenceDisposition {
        kind: NoLaunch::Expired,
        matches_approval: true,
    }),
    Some(OccurrenceDisposition {
        kind: NoLaunch::Expired,
        matches_approval: false,
    }),
];

/// The event space the explorer enumerates: every combination of the
/// authority-relevant claims, with the kind's consistent `(phase,
/// disposition)` pair and one pair that is malformed for every kind. The
/// full `(phase, disposition)` product is a shape concern, which
/// `tests/differential.rs` checks separately against the implementation.
#[must_use]
pub fn authority_core_space() -> Vec<Event> {
    let mut events = Vec::new();
    for kind in Kind::ALL {
        for dispositions in [consistent_disposition(kind), INCONSISTENT_DISPOSITION] {
            for from in State::ALL {
                for to in State::ALL {
                    for sequence in [
                        SequenceClaim::Expected,
                        SequenceClaim::Stale,
                        SequenceClaim::Skipped,
                    ] {
                        for previous in [PreviousClaim::Head, PreviousClaim::Other] {
                            for subject in [SubjectClaim::Same, SubjectClaim::Different] {
                                for digest in
                                    [DigestClaim::Null, DigestClaim::Matching, DigestClaim::Other]
                                {
                                    for approval_supplied in [false, true] {
                                        for consumption in
                                            std::iter::once(None).chain(Binding::ALL.map(Some))
                                        {
                                            for occurrence_disposition in OCCURRENCE_DISPOSITIONS {
                                                events.push(Event {
                                                    kind,
                                                    from,
                                                    to,
                                                    sequence,
                                                    previous,
                                                    subject,
                                                    digest,
                                                    approval_supplied,
                                                    phase: dispositions.0,
                                                    disposition: dispositions.1,
                                                    consumption,
                                                    occurrence_disposition,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    events
}

/// A property violated by one explored step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The violated property.
    pub property: &'static str,
    /// The chain the event was applied to.
    pub chain: Chain,
    /// The event.
    pub event: Event,
    /// What the model returned: the accepted successor, or `None` when the
    /// violation is of the invariant on a reachable state.
    pub next: Option<Chain>,
    /// The accepted events that reach `chain` from [`Chain::EMPTY`].
    pub witness: Vec<Event>,
}

/// The result of one exploration.
#[derive(Debug, Clone, Default)]
pub struct Exploration {
    /// Every reachable chain, in discovery order, with a witness path.
    pub reachable: Vec<(Chain, Vec<Event>)>,
    /// How many `(chain, event)` pairs were applied.
    pub transitions: u64,
    /// How many of them were accepted.
    pub accepted: u64,
    /// How many were refused for shape alone.
    pub malformed: u64,
    /// Every violation found. Empty means every property held within the
    /// bounds.
    pub violations: Vec<Violation>,
    /// How long the exploration took.
    pub elapsed: Duration,
}

impl Exploration {
    /// A witness path to `chain`, if it was reached.
    #[must_use]
    pub fn witness(&self, chain: Chain) -> Option<&[Event]> {
        self.reachable
            .iter()
            .find(|(reached, _)| *reached == chain)
            .map(|(_, path)| path.as_slice())
    }
}

/// Explores every reachable chain for `approval` under `variant`, applying
/// each event in `space` to each.
#[must_use]
pub fn explore(variant: Variant, approval: Approval, space: &[Event]) -> Exploration {
    let started = Instant::now();
    let mut exploration = Exploration::default();
    let mut index: HashMap<Chain, usize> = HashMap::new();
    let mut frontier = vec![Chain::EMPTY];
    index.insert(Chain::EMPTY, 0);
    exploration.reachable.push((Chain::EMPTY, Vec::new()));

    while let Some(chain) = frontier.pop() {
        let witness = exploration.reachable[index[&chain]].1.clone();
        if !invariant(approval, chain) {
            exploration.violations.push(Violation {
                property: "the reachable-state invariant holds",
                chain,
                event: witness.last().copied().unwrap_or_else(|| space[0]),
                next: None,
                witness: witness.clone(),
            });
        }
        for &event in space {
            exploration.transitions += 1;
            if !shape_ok(chain, event) {
                exploration.malformed += 1;
            }
            let Ok(next) = apply(variant, chain, approval, event) else {
                continue;
            };
            exploration.accepted += 1;
            if let Some(property) = step_violation(approval, chain, event, next) {
                exploration.violations.push(Violation {
                    property,
                    chain,
                    event,
                    next: Some(next),
                    witness: witness.clone(),
                });
            }
            if let std::collections::hash_map::Entry::Vacant(slot) = index.entry(next) {
                let mut path = witness.clone();
                path.push(event);
                slot.insert(exploration.reachable.len());
                exploration.reachable.push((next, path));
                frontier.push(next);
            }
        }
    }
    exploration.elapsed = started.elapsed();
    exploration
}

/// A one-line rendering of an event, for counterexamples.
#[must_use]
pub fn describe(event: Event) -> String {
    format!(
        "{} {}->{} seq={:?} prev={:?} subject={:?} digest={:?} approval={} phase={} disposition={} consumption={:?} occurrence_disposition={:?}",
        event.kind.as_str(),
        event.from.as_str(),
        event.to.as_str(),
        event.sequence,
        event.previous,
        event.subject,
        event.digest,
        if event.approval_supplied { "supplied" } else { "absent" },
        event.phase.as_str(),
        event.disposition.as_str(),
        event.consumption.map(|b| b.occurrence),
        event
            .occurrence_disposition
            .map(|d| (d.kind, d.matches_approval)),
    )
}

/// A multi-line rendering of a violation: the witness path, the offending
/// event and the states around it.
#[must_use]
pub fn render(violation: &Violation) -> String {
    let mut text = format!("violated: {}\n", violation.property);
    text.push_str("witness from the empty chain:\n");
    for (i, event) in violation.witness.iter().enumerate() {
        text.push_str(&format!("  {}. {}\n", i + 1, describe(*event)));
    }
    text.push_str(&format!("at chain: {:?}\n", violation.chain));
    text.push_str(&format!("event:    {}\n", describe(violation.event)));
    match violation.next {
        Some(next) => text.push_str(&format!("accepted: {next:?}\n")),
        None => text.push_str("the chain itself violates the invariant\n"),
    }
    text
}
