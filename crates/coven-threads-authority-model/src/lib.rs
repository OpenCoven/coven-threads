//! Bounded verification pilot for the automation-authority transitions
//! (coven-threads#95).
//!
//! This crate is research tooling, not a contract. It holds a small pure
//! model of the approval lifecycle and the at-most-once adoption and
//! decision-consumption stores, an exhaustive explorer over the model's
//! bounded state space with deliberately broken variants as negative
//! controls, and Kani proof harnesses over the same model. The tests under
//! `tests/` run the explorer and a model-to-implementation differential
//! that concretizes abstract events into real, Ed25519-signed artifacts and
//! applies them through `coven_threads_core::automation_authority`.
//!
//! The reference validator `profiles/automation-authority/v1/validator.mjs`
//! and the specification remain normative. Where this model and the
//! implementation disagree, the differential fails and the disagreement is
//! an owner-local regression to investigate; the model is not evidence of
//! correctness on its own. See `docs/reviews/2026-10-29-authority-model-pilot.md`.

pub mod decision;
pub mod explore;
pub mod lifecycle;

#[cfg(kani)]
mod kani;
