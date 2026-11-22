//! Kani proof harnesses over the model. Compiled only under `cargo kani`
//! (`cfg(kani)`); ordinary builds and tests do not see this module.
//!
//! Each harness is an inductive step: assume an arbitrary chain satisfying
//! [`invariant`], apply an arbitrary event, and assert the step properties
//! and the invariant of the successor. The model has no loops other than
//! `count_ones`, so no unwind bound is needed.
//!
//! Run with `cargo kani -p coven-threads-authority-model`.

use crate::lifecycle::{apply, invariant, step_violation, Approval, Chain, Event, Use, Variant};

fn arbitrary_valid_approval() -> Approval {
    let approval: Approval = kani::any();
    kani::assume(approval.use_.is_valid());
    approval
}

fn arbitrary_invariant_chain(approval: Approval) -> Chain {
    let chain: Chain = kani::any();
    kani::assume(chain.sequence < u8::MAX);
    kani::assume(invariant(approval, chain));
    chain
}

/// Under the faithful relation, every accepted step from an invariant
/// chain satisfies the step properties and preserves the invariant.
#[kani::proof]
fn faithful_step_is_safe() {
    let approval = arbitrary_valid_approval();
    let chain = arbitrary_invariant_chain(approval);
    let event: Event = kani::any();
    if let Ok(next) = apply(Variant::Faithful, chain, approval, event) {
        assert!(step_violation(approval, chain, event, next).is_none());
        assert!(invariant(approval, next));
    }
}

/// A single-use approval is consumed at most once: no accepted step leaves
/// the count above one.
#[kani::proof]
fn single_use_consumed_at_most_once() {
    let approval = Approval {
        use_: Use::SingleUse,
    };
    let chain = arbitrary_invariant_chain(approval);
    let event: Event = kani::any();
    if let Ok(next) = apply(Variant::Faithful, chain, approval, event) {
        assert!(next.count <= 1);
    }
}

/// A recurring approval never exceeds `max_uses`, and never binds the same
/// occurrence twice.
#[kani::proof]
fn recurring_use_is_bounded_and_distinct() {
    let approval = arbitrary_valid_approval();
    kani::assume(matches!(approval.use_, Use::Recurring { .. }));
    let chain = arbitrary_invariant_chain(approval);
    let event: Event = kani::any();
    if let Ok(next) = apply(Variant::Faithful, chain, approval, event) {
        if let Use::Recurring { max_uses } = approval.use_ {
            assert!(next.count <= max_uses);
        }
        assert!(u8::try_from(next.consumed.count_ones()).unwrap() == next.count);
    }
}

/// Terminal states are absorbing.
#[kani::proof]
fn terminal_states_are_absorbing() {
    let approval = arbitrary_valid_approval();
    let chain = arbitrary_invariant_chain(approval);
    kani::assume(chain.state.is_terminal());
    let event: Event = kani::any();
    assert!(apply(Variant::Faithful, chain, approval, event).is_err());
}

/// Negative control: under erratum 1's relation, Kani must find a
/// counterexample to the step properties. This harness is *expected to
/// fail*; run it with `--harness erratum_1_is_caught` and read the trace.
#[kani::proof]
fn erratum_1_is_caught() {
    let approval = arbitrary_valid_approval();
    let chain = arbitrary_invariant_chain(approval);
    let event: Event = kani::any();
    if let Ok(next) = apply(
        Variant::Erratum1ConsumeChecksTargetOnly,
        chain,
        approval,
        event,
    ) {
        assert!(step_violation(approval, chain, event, next).is_none());
    }
}
