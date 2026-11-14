//! Exhaustive bounded exploration of the lifecycle model (coven-threads#95).
//!
//! The faithful relation must satisfy every property on every reachable
//! state and accepted step; each deliberately broken variant must yield a
//! counterexample, which is printed with its witness path.

use coven_threads_authority_model::decision::{
    adopt, consume, Adoption, Consumption, Outcome, Refusal,
};
use coven_threads_authority_model::explore::{authority_core_space, explore, render};
use coven_threads_authority_model::lifecycle::{Approval, State, Use, Variant, MAX_USES_BOUND};

fn approvals() -> Vec<Approval> {
    let mut approvals = vec![Approval {
        use_: Use::SingleUse,
    }];
    for max_uses in 1..=MAX_USES_BOUND {
        approvals.push(Approval {
            use_: Use::Recurring { max_uses },
        });
    }
    approvals
}

#[test]
fn faithful_relation_satisfies_every_property_within_bounds() {
    let space = authority_core_space();
    let mut total_transitions = 0;
    let mut total_reachable = 0;
    for approval in approvals() {
        let result = explore(Variant::Faithful, approval, &space);
        if let Some(violation) = result.violations.first() {
            panic!(
                "{:?}: {} violations; first:\n{}",
                approval.use_,
                result.violations.len(),
                render(violation)
            );
        }
        // The model reaches every state its use allows.
        let states: Vec<State> = result.reachable.iter().map(|(c, _)| c.state).collect();
        let expected: &[State] = match approval.use_ {
            Use::SingleUse => &State::ALL,
            Use::Recurring { .. } => &[
                State::Required,
                State::Requested,
                State::Approved,
                State::Rejected,
                State::Expired,
                State::Revoked,
            ],
        };
        for state in expected {
            assert!(
                states.contains(state),
                "{:?} never reaches {state:?}",
                approval.use_
            );
        }
        if let Use::Recurring { max_uses } = approval.use_ {
            let most = result.reachable.iter().map(|(c, _)| c.count).max().unwrap();
            assert_eq!(
                most, max_uses,
                "the usage bound is reached, not just respected"
            );
        }
        eprintln!(
            "{:?}: events={} reachable={} transitions={} accepted={} malformed={} elapsed={:?}",
            approval.use_,
            space.len(),
            result.reachable.len(),
            result.transitions,
            result.accepted,
            result.malformed,
            result.elapsed
        );
        total_transitions += result.transitions;
        total_reachable += result.reachable.len();
    }
    assert!(total_transitions > 0 && total_reachable > 0);
}

/// Each broken variant, the property it must violate, and an approval use
/// under which the violation is reachable.
const NEGATIVE_CONTROLS: [(Variant, &str, Use); 3] = [
    (
        Variant::Erratum1ConsumeChecksTargetOnly,
        "consumption is valid only from approved",
        Use::Recurring { max_uses: 2 },
    ),
    (
        Variant::RecurringBoundOffByOne,
        "the reachable-state invariant is preserved",
        Use::Recurring { max_uses: 1 },
    ),
    (
        Variant::RevokeAfterConsumed,
        "terminal states are absorbing",
        Use::SingleUse,
    ),
];

#[test]
fn every_broken_variant_yields_a_counterexample() {
    let space = authority_core_space();
    for (variant, property, use_) in NEGATIVE_CONTROLS {
        let result = explore(variant, Approval { use_ }, &space);
        let found = result
            .violations
            .iter()
            .find(|v| v.property == property)
            .unwrap_or_else(|| {
                panic!(
                    "{variant:?} under {use_:?} found no counterexample to {property:?}; found {:?}",
                    result.violations.iter().map(|v| v.property).collect::<Vec<_>>()
                )
            });
        eprintln!(
            "--- negative control {variant:?} under {use_:?}\n{}",
            render(found)
        );
    }
}

#[test]
fn single_use_count_guard_is_defense_in_depth() {
    // Dropping `single_use && consumption_count > 0` leaves every property
    // intact: a single-use consume lands in terminal `consumed`, so no
    // second consume reaches the guard. The guard is kept as defense in
    // depth; this test records that it is not load-bearing today.
    let space = authority_core_space();
    let result = explore(
        Variant::SingleUseCountNotChecked,
        Approval {
            use_: Use::SingleUse,
        },
        &space,
    );
    assert!(
        result.violations.is_empty(),
        "{}",
        render(&result.violations[0])
    );
}

#[test]
fn erratum_1_counterexample_consumes_without_any_approve_event() {
    // The shortest counterexample the explorer finds for erratum 1 is a
    // recurring consume appended to the empty chain: authority spent with
    // no `approve` event at all. Vector
    // `16-revoked-approval-cannot-be-consumed` covers the longer
    // revoked-then-consumed path.
    let space = authority_core_space();
    let result = explore(
        Variant::Erratum1ConsumeChecksTargetOnly,
        Approval {
            use_: Use::Recurring { max_uses: 2 },
        },
        &space,
    );
    let shortest = result
        .violations
        .iter()
        .filter(|v| v.property == "consumption is valid only from approved")
        .min_by_key(|v| v.witness.len())
        .expect("a counterexample");
    assert!(shortest.witness.is_empty(), "{}", render(shortest));
    assert_eq!(
        shortest.chain.state,
        State::Required,
        "{}",
        render(shortest)
    );
    assert!(!shortest.next.unwrap().approved, "{}", render(shortest));
}

#[test]
fn decision_and_adoption_layers_are_at_most_once() {
    // Exhaustive over the one-bit stores and every outcome.
    for outcome in Outcome::ALL {
        for consumed in [false, true] {
            let store = Consumption { consumed };
            match consume(store, outcome) {
                Ok(next) => {
                    assert!(outcome.is_consumable() && !consumed && next.consumed);
                }
                Err(Refusal::NotConsumable) => assert!(!outcome.is_consumable()),
                Err(Refusal::Replayed) => assert!(outcome.is_consumable() && consumed),
            }
        }
    }
    assert!(adopt(Adoption { adopted: false }).is_ok());
    assert_eq!(adopt(Adoption { adopted: true }), Err(Refusal::Replayed));
}
