//! **Gate 4 fail-closed is a line-one conformance property (RFC-0001 §5.4): an
//! implementation that allows Gate 4 to be bypassed DOES NOT conform to RFC-0001.
//! Every unknown in this crate — surface, writer, channel, panic — resolves to
//! Reject.**
//!
//! # coven-threads-core
//!
//! OpenCoven's authority-boundary gate layer: the *gate-shaped receiver* the
//! `coven` daemon calls to validate identity-surface mutation requests
//! (`specs/PHASE-0-DESIGN.md`, FROZEN v0.2; RFC-0001 §5 is the upstream
//! normative anchor — RFC wins on any conflict).
//!
//! This crate sits *above* the `coven` Rust daemon and *underneath* every
//! familiar's protected memory surface. It is reachable only by the privileged
//! daemon, never by a familiar-controlled process (RFC-0001 §5.1: F MUST NOT
//! modify the Ward file, restart the authority process, or bypass gates).
//!
//! ## Vocabulary (§2 — metaphor terms bound to concrete referents)
//!
//! - **Thread** (*authority relationship: surface → writer*) — a directional
//!   line from a protected surface to the authority gating writes to it. One
//!   thread per `(surface, writer)` pair. Threads have **tension**: they hold,
//!   fray, or snap under load.
//! - **Weave** (*enforced pattern of threads across a familiar or Coven*) — the
//!   invariant that these threads must all hold together for identity to be
//!   coherent. Ward's four gates are the **loom**, not threads.
//! - **Strand** (*fiber inside a thread: hash | sig | manifest entry | audit
//!   trail | serialization marker*) — the fibers that make a thread survive
//!   stress. A thread survives a channel iff its strands survive that channel.
//! - **Channel** (*the axis threads must hold under*) — `Deliberate`, `Forced`,
//!   `Serialization`, `Mutation`. Every gate check is: **"does thread T hold
//!   under channel C?"**
//!
//! ## The four invariants, co-designed (§3.3)
//!
//! Channels are how the four ship as one build, not stacked features:
//! identity-as-memory-property (threads bind typed surfaces at construction),
//! structural mutation authority (the gate is external, daemon-called),
//! two-compaction contract (`Deliberate` vs `Forced` are distinct channels with
//! distinct survival floors — WARD-C1–C6 governs `Forced`), and survives
//! serialization (C7: `Serialization`-channel threads carry a
//! `SerializationMarker` strand that round-trips or fails visibly).
//!
//! ## Predicate authoritative, descriptor derived (§2.2)
//!
//! A weave's [`PatternPredicate`] is the authority. [`PatternDescriptor`] is a
//! derived summary for humans and tools. **Nothing may gate enforcement on a
//! descriptor** — that is the derived-index problem reinvented one layer up.
//!
//! ## What this crate is not
//!
//! No enforcement side effects live here: no filesystem verification, no audit
//! writes, no staging. Those are the daemon's lane (Phase 2). This crate answers
//! the gate question and names the verdict.

pub mod approval;
pub mod audit;
pub mod channel;
pub mod identity_invariants;
pub mod fray;
pub mod ids;
pub mod manifest;
pub mod pattern;
pub mod portability;
pub mod staging;
pub mod strand;
pub mod thread;
pub mod validate;
pub mod weave;

pub use identity_invariants::{
    AdvisoryProbeResult, AdvisoryProbes, CompositeIdentityInvariant, FamiliarNameInvariant,
    ManifestAnchoredInvariant,
};
pub use approval::{
    ApprovalPath, ApprovalPathKind, ApprovalPathWireEnvelope, ProposalClassification,
    ProposalWindowAuditDetail, ProposalWindowCloseAuditDetail, SurfaceRegionId, VetoWindow,
    WindowCloseReason,
};
pub use audit::{AuditEventType, WardAuditRecord, WARD_AUDIT_SCHEMA_SQL};
pub use channel::Channel;
pub use fray::{FrayOrSnap, FrayReason, SnapReason};
pub use ids::{
    CovenId, EventRef, FamiliarId, ManifestId, ProposalId, StrandId, SurfaceId, ThreadId, WeaveId,
    WriterId,
};
pub use manifest::{manifest_entry_hash, merkle_root, HashManifest};
pub use pattern::{
    AllSurfacesHoldOnChannels, PatternDescriptor, PatternPredicate, StrandRequirement,
    WeaveCoherence,
};
pub use portability::{
    export_af, export_weave, export_weave_with_surfaces, from_json_bytes, import_weave,
    to_json_bytes, AfCoreMemoryBlock, AfNonRoundTrippableMarker, AfTag, LossyAfExport,
    PortabilityError, PortableSurfaceContent, PortableWeave, SerializationContract,
    PORTABILITY_FORMAT_VERSION,
};
pub use staging::{PendingProposal, StagedContents, StagedEdit};
pub use strand::{HashAlgo, SigKind, Strand, StrandKind};
pub use thread::{TensionState, Thread};
pub use validate::{validate, validate_fail_closed, MutationRequest, RejectReason, Verdict};
pub use weave::{Weave, WeaveError, WeaveRecord};
