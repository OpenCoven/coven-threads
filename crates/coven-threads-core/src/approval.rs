//! Phase-5 approval semantics — core types (`threads-uqx.3`).
//!
//! This module defines the authority-ceremony layer sitting *above* the load
//! axis ([`Channel`]) and *below* the daemon's apply logic. The key
//! decomposition (`specs/PHASE-5-APPROVAL-SEMANTICS.md` §3.1, decision 1):
//!
//! - **[`Channel`]** — *why* a thread is stressed (deliberate / forced /
//!   serialization / mutation). Phase-0 axis, not changed.
//! - **[`ApprovalPath`]** — *which promotion ceremony* is required before the
//!   daemon may apply a proposal. Phase-5 addition. These are orthogonal; never
//!   gate on `Channel` to infer `ApprovalPath`.
//!
//! ## Veto windows are delayed-apply only (decision 2)
//!
//! A [`VetoWindow`] means: the daemon stages the proposal, makes it *visibly
//! pending* for at least `min_visible`, then applies only if the window closes
//! without a veto. Provisional apply (apply-then-rollback) is explicitly
//! forbidden until Val/Nova accept rollback semantics.
//!
//! ## Proposal classification (decision 3 / 5 bead scope)
//!
//! [`ProposalClassification`] is the record the daemon produces at intake. It
//! carries:
//! - the channel the mutation arrived on;
//! - which surface regions were affected (region evidence, `threads-uqx.5`);
//! - the floor path tier (tier 0 = protected, tier 1 = reviewed, …);
//! - the required approval path — highest ceremony wins;
//! - an **`evidence_replay_hash`** (WARD-C7 generalised): the delayed-apply
//!   scheduler must prove at deadline that the evidence that gated the
//!   window-open decision can still be replayed to the same result.
//!
//! ## Audit event shape (decisions 2 + 8)
//!
//! The audit trail for a proposal lifecycle is: `proposal_submitted` →
//! `proposal_window_opened` (when delayed-apply applies) → close event. The
//! close event carries an explicit [`WindowCloseReason`] field — the window is a
//! first-class audit interval, not a gap between two rows.
//!
//! ## Label mapping is a daemon wire contract (decision 7)
//!
//! [`ApprovalPath::display_label`] is the canonical display string the daemon
//! emits over the wire. Cave has zero policy freedom over these labels: it
//! renders exactly what the daemon sends. The daemon MUST reject at load if a
//! variant has no corresponding display label, or if a received label string
//! cannot be mapped back to a variant (see [`ApprovalPath::from_display_label`]).
//!
//! [`Channel`]: crate::channel::Channel

use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::channel::Channel;
use crate::ids::{FamiliarId, ProposalId, SurfaceId};

// ---------------------------------------------------------------------------
// ApprovalPath
// ---------------------------------------------------------------------------

/// The promotion ceremony required before a proposal may be applied.
///
/// ## Ceremony ordering (highest wins)
///
/// When a proposal touches surfaces with different approval requirements, the
/// daemon uses the highest ceremony for the proposal as a unit (all-or-nothing,
/// matching the existing Ward behaviour). The ordering is:
///
/// `AutoRegression` < `FamiliarCoherence` < `HumanApproval` <
/// `HumanApprovalWithRationale`
///
/// ## Display labels (daemon wire contract — decision 7)
///
/// The daemon emits `{variant, label, veto_deadline}` over its wire protocol.
/// Clients (Cave and others) MUST render the label as received; they have zero
/// policy freedom over the string. The daemon MUST reject at load time if a
/// variant has no label ([`ApprovalPath::display_label`] is exhaustive) or if
/// a received label string is not recognised ([`ApprovalPath::from_display_label`]
/// returns `None` → load-time reject).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalPath {
    /// Deterministic regression gates pass; human principal may veto within the
    /// window. If the window closes without a veto, the proposal is applied
    /// automatically. RFC-0001 §5.3 auto tier.
    AutoRegression {
        /// Optional veto window. `None` means: apply as soon as regression gates
        /// clear, with no veto period (emergency / unattended automation path).
        veto: Option<VetoWindow>,
    },

    /// Familiar-coherence gate runs; a veto window follows before apply.
    /// Maps to RFC-0001 §5.3 familiar-review tier.
    FamiliarCoherence {
        /// The veto window (always required for this path; familiar review is
        /// meaningless without a chance for the familiar to surface a concern).
        veto: VetoWindow,
    },

    /// A human principal must explicitly approve before apply. No veto window;
    /// the write is blocked until approval arrives.
    HumanApproval,

    /// A human principal must approve *and* record a rationale. The rationale
    /// is stored in the audit row. Highest-ceremony path.
    HumanApprovalWithRationale,
}

impl ApprovalPath {
    /// Stable display label the daemon emits over the wire (decision 7).
    ///
    /// These strings are the canonical external representation. Do not change
    /// them without a migration plan; Cave and other clients will be
    /// hard-referencing the strings.
    pub fn display_label(&self) -> &'static str {
        match self {
            ApprovalPath::AutoRegression { .. } => "auto",
            ApprovalPath::FamiliarCoherence { .. } => "familiar_review",
            ApprovalPath::HumanApproval => "human_review",
            ApprovalPath::HumanApprovalWithRationale => "human_required",
        }
    }

    /// Parse a display label back to a variant **shape** (without inner fields,
    /// which must come from the full wire payload).
    ///
    /// The daemon MUST call this at load time and reject if the result is
    /// `None` — no unknown labels may pass through.
    pub fn from_display_label(label: &str) -> Option<ApprovalPathKind> {
        match label {
            "auto" => Some(ApprovalPathKind::AutoRegression),
            "familiar_review" => Some(ApprovalPathKind::FamiliarCoherence),
            "human_review" => Some(ApprovalPathKind::HumanApproval),
            "human_required" => Some(ApprovalPathKind::HumanApprovalWithRationale),
            _ => None,
        }
    }

    /// Ordinal for "highest ceremony wins" comparison.
    fn ceremony_ordinal(&self) -> u8 {
        match self {
            ApprovalPath::AutoRegression { .. } => 0,
            ApprovalPath::FamiliarCoherence { .. } => 1,
            ApprovalPath::HumanApproval => 2,
            ApprovalPath::HumanApprovalWithRationale => 3,
        }
    }

    /// Returns the higher-ceremony path of `self` and `other`.
    pub fn highest(self, other: ApprovalPath) -> ApprovalPath {
        if other.ceremony_ordinal() > self.ceremony_ordinal() {
            other
        } else {
            self
        }
    }

    /// Whether this path includes a veto window.
    pub fn has_veto_window(&self) -> bool {
        match self {
            ApprovalPath::AutoRegression { veto } => veto.is_some(),
            ApprovalPath::FamiliarCoherence { .. } => true,
            ApprovalPath::HumanApproval | ApprovalPath::HumanApprovalWithRationale => false,
        }
    }

    /// Extract the veto window, if present.
    pub fn veto_window(&self) -> Option<&VetoWindow> {
        match self {
            ApprovalPath::AutoRegression { veto } => veto.as_ref(),
            ApprovalPath::FamiliarCoherence { veto } => Some(veto),
            ApprovalPath::HumanApproval | ApprovalPath::HumanApprovalWithRationale => None,
        }
    }
}

/// Variant kind without inner fields — used for label-round-trip validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPathKind {
    AutoRegression,
    FamiliarCoherence,
    HumanApproval,
    HumanApprovalWithRationale,
}

impl ApprovalPathKind {
    /// The display label for this kind (decision 7 — must round-trip with
    /// `ApprovalPath::from_display_label`).
    pub fn display_label(&self) -> &'static str {
        match self {
            ApprovalPathKind::AutoRegression => "auto",
            ApprovalPathKind::FamiliarCoherence => "familiar_review",
            ApprovalPathKind::HumanApproval => "human_review",
            ApprovalPathKind::HumanApprovalWithRationale => "human_required",
        }
    }
}

// ---------------------------------------------------------------------------
// VetoWindow
// ---------------------------------------------------------------------------

/// A delayed-apply veto window (decision 2).
///
/// ## Semantics
///
/// The daemon stages the proposal and makes it *visibly pending* for at least
/// [`min_visible`]. After `deadline` passes without a veto, the daemon
/// **revalidates** (using `evidence_replay_hash` on [`ProposalClassification`])
/// and applies only if revalidation succeeds.
///
/// Provisional apply (write-then-rollback) is **not** modelled here and must
/// not be inferred. Every apply happens *after* the deadline, never before.
///
/// ## `min_visible` enforcement
///
/// The daemon MUST NOT close a veto window before `staged_at + min_visible` has
/// elapsed, even if the deadline has technically passed (e.g. if the window was
/// created with a very short duration). This prevents race conditions where a
/// proposal is technically pending but never actually reachable by a human.
///
/// [`min_visible`]: VetoWindow::min_visible
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VetoWindow {
    /// How long the window stays open from the moment the proposal is staged.
    pub duration: Duration,
    /// Minimum time the proposal must be *visibly pending* before the window
    /// may close. Prevents proposals that are technically pending but
    /// unreachable in practice (same shape as the two-compaction contract's
    /// minimum-visibility requirement).
    ///
    /// MUST be ≤ `duration`.
    pub min_visible: Duration,
}

impl VetoWindow {
    /// Construct a veto window, panicking if `min_visible > duration`.
    pub fn new(duration: Duration, min_visible: Duration) -> Self {
        assert!(
            min_visible <= duration,
            "VetoWindow: min_visible ({min_visible:?}) must be ≤ duration ({duration:?})"
        );
        Self {
            duration,
            min_visible,
        }
    }

    /// Calculate the absolute deadline given a staged-at timestamp.
    pub fn deadline(&self, staged_at: OffsetDateTime) -> OffsetDateTime {
        staged_at
            + time::Duration::try_from(self.duration)
                .expect("VetoWindow duration too large for time::Duration")
    }

    /// The earliest time at which the window may close (staged_at + min_visible).
    pub fn earliest_close(&self, staged_at: OffsetDateTime) -> OffsetDateTime {
        staged_at
            + time::Duration::try_from(self.min_visible)
                .expect("VetoWindow min_visible too large for time::Duration")
    }

    /// Whether the window may now be closed given the current time and when it
    /// was staged.
    ///
    /// Returns `true` iff `now >= earliest_close(staged_at)`.  The scheduler
    /// should additionally check that `now >= deadline(staged_at)` before
    /// auto-applying; this method only answers the min-visible gate.
    pub fn is_min_visible_elapsed(&self, staged_at: OffsetDateTime, now: OffsetDateTime) -> bool {
        now >= self.earliest_close(staged_at)
    }
}

// ---------------------------------------------------------------------------
// ProposalClassification
// ---------------------------------------------------------------------------

/// The daemon's classification record produced at proposal intake.
///
/// Every field is set when the proposal is first received; none may be mutated
/// afterwards (the record is append-only, mirroring `ward.audit`). At
/// delayed-apply deadline, the daemon replays gate evidence against
/// `evidence_replay_hash` — if the result differs, the proposal is rejected
/// (WARD-C7 generalised: evidence must survive the time gap).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalClassification {
    /// The proposal this classification belongs to.
    pub proposal_id: ProposalId,
    /// The familiar whose weave is being mutated.
    pub familiar_id: FamiliarId,
    /// The channel the mutation arrived on (load axis — not the approval path).
    pub channel: Channel,
    /// Surface regions the diff touches (populated by region-extractor
    /// predicates, `threads-uqx.5`). May be empty while uqx.5 is unshipped.
    pub affected_regions: Vec<SurfaceRegionId>,
    /// Floor path tier for all touched surfaces (lowest tier = most protected).
    /// Tier 0 = protected; tier 1 = reviewed; tier 2 = logged; tier 3 = free.
    pub path_tier_floor: u8,
    /// The approval ceremony required (highest ceremony of all touched
    /// surfaces + regions).
    pub approval_path: ApprovalPath,
    /// Blake3 hash over the gate-evidence bundle that was evaluated when this
    /// classification was created. The delayed-apply scheduler MUST re-derive
    /// this hash at deadline and reject if it differs (WARD-C7 generalised).
    ///
    /// Encoding: raw 32-byte Blake3 digest.
    pub evidence_replay_hash: [u8; 32],
    /// When the classification was produced.
    pub classified_at: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// SurfaceRegionId
// ---------------------------------------------------------------------------

/// Opaque identifier for a typed semantic surface region (`threads-uqx.5`).
///
/// The full `SurfaceRegionPredicate` machinery lives in `uqx.5`; this module
/// only needs the identifier so `ProposalClassification` can reference regions
/// without a circular dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SurfaceRegionId(pub String);

impl SurfaceRegionId {
    /// Construct from any string-like value.
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SurfaceRegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// WindowCloseReason
// ---------------------------------------------------------------------------

/// The reason a veto window closed (decision 2 + 8 — explicit reason field on
/// every close event; the window is a first-class audit interval).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowCloseReason {
    /// The window expired without a veto and revalidation succeeded; proposal
    /// was applied.
    Applied,
    /// A principal vetoed the proposal before the deadline.
    Vetoed,
    /// The window expired and revalidation failed; proposal was rejected.
    Expired,
    /// A superseding proposal was submitted for the same surface before this
    /// window closed; this proposal is no longer pending.
    Superseded,
}

impl WindowCloseReason {
    /// Stable tag for the `decision` column in `ward.audit`.
    pub fn tag(&self) -> &'static str {
        match self {
            WindowCloseReason::Applied => "applied",
            WindowCloseReason::Vetoed => "vetoed",
            WindowCloseReason::Expired => "expired",
            WindowCloseReason::Superseded => "superseded",
        }
    }
}

// ---------------------------------------------------------------------------
// ProposalWindowAuditRecord
// ---------------------------------------------------------------------------

/// Audit record for the `proposal_window_opened` event (decision 8).
///
/// This is a *supplementary* shape for the detail payload in a
/// `WardAuditRecord` with `event_type = AuditEventType::ProposalWindowOpened`.
/// The top-level fields (`proposal_id`, `familiar_id`, `ward_hash`, …) come
/// from the `WardAuditRecord`; this struct is serialised into the `detail`
/// column.
///
/// Note: `AuditEventType::ProposalWindowOpened` will be added to
/// `AuditEventType` in issue #5 / the Cody implementation lane, which owns
/// the schema migration. This struct documents the shape so that migration
/// can be written correctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalWindowAuditDetail {
    /// The approval path that opened this window.
    pub approval_path_label: String,
    /// Absolute deadline (RFC 3339).
    pub deadline: OffsetDateTime,
    /// Earliest moment the window may close (RFC 3339).
    pub earliest_close: OffsetDateTime,
    /// Hex-encoded `evidence_replay_hash` from `ProposalClassification`.
    pub evidence_replay_hash_hex: String,
    /// Affected surface region ids, for audit legibility.
    pub affected_regions: Vec<String>,
}

/// Audit record for the window-close event (the paired close to
/// `proposal_window_opened`).
///
/// Serialised into the `detail` column of the close-event `WardAuditRecord`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalWindowCloseAuditDetail {
    /// Why the window closed.
    pub reason: WindowCloseReason,
    /// Whether the evidence replay hash matched at deadline (for `Applied` and
    /// `Expired` paths — `None` for `Vetoed` / `Superseded` where replay is not
    /// attempted).
    pub replay_hash_matched: Option<bool>,
    /// Rationale text (required for `HumanApprovalWithRationale` path; `None`
    /// for other paths unless the approver voluntarily adds one).
    pub rationale: Option<String>,
}

// ---------------------------------------------------------------------------
// Wire envelope (daemon → client)
// ---------------------------------------------------------------------------

/// The wire envelope the daemon sends to clients for a pending proposal
/// (decision 7 — label, variant, optional deadline).
///
/// Cave and other clients receive this and render it as-is. They MUST NOT
/// infer policy from the variant or label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPathWireEnvelope {
    /// Stable enum variant name (snake_case, for machine use).
    pub variant: ApprovalPathKind,
    /// Human-readable display label (daemon wire contract — never remapped
    /// by clients).
    pub label: String,
    /// Absolute veto deadline, if this path has a veto window.
    pub veto_deadline: Option<OffsetDateTime>,
    /// The surfaces affected by this proposal (for display).
    pub affected_surfaces: Vec<SurfaceId>,
}

impl ApprovalPathWireEnvelope {
    /// Construct from a classification and optional staged-at timestamp.
    pub fn from_classification(
        classification: &ProposalClassification,
        staged_at: Option<OffsetDateTime>,
    ) -> Self {
        let variant = match &classification.approval_path {
            ApprovalPath::AutoRegression { .. } => ApprovalPathKind::AutoRegression,
            ApprovalPath::FamiliarCoherence { .. } => ApprovalPathKind::FamiliarCoherence,
            ApprovalPath::HumanApproval => ApprovalPathKind::HumanApproval,
            ApprovalPath::HumanApprovalWithRationale => ApprovalPathKind::HumanApprovalWithRationale,
        };
        let label = classification.approval_path.display_label().to_string();
        let veto_deadline = staged_at.and_then(|at| {
            classification
                .approval_path
                .veto_window()
                .map(|w| w.deadline(at))
        });
        Self {
            variant,
            label,
            veto_deadline,
            affected_surfaces: vec![],
        }
    }

    /// Validate that the label round-trips back to the variant (daemon load-time
    /// check — decision 7). Returns `Err` with the offending label if not.
    pub fn validate_label_round_trip(&self) -> Result<(), String> {
        let parsed = ApprovalPath::from_display_label(&self.label).ok_or_else(|| {
            format!(
                "unknown display label {:?}: daemon must reject at load time",
                self.label
            )
        })?;
        if parsed != self.variant {
            return Err(format!(
                "label {:?} maps to {:?} but envelope carries {:?}",
                self.label, parsed, self.variant
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use time::OffsetDateTime;

    // ---- ApprovalPath display labels ----

    #[test]
    fn all_variants_have_display_labels() {
        let paths = [
            ApprovalPath::AutoRegression { veto: None },
            ApprovalPath::FamiliarCoherence {
                veto: VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(300)),
            },
            ApprovalPath::HumanApproval,
            ApprovalPath::HumanApprovalWithRationale,
        ];
        let expected = ["auto", "familiar_review", "human_review", "human_required"];
        for (path, label) in paths.iter().zip(expected.iter()) {
            assert_eq!(path.display_label(), *label, "label mismatch for {path:?}");
        }
    }

    #[test]
    fn display_labels_round_trip_to_kind() {
        // decision 7: every label maps to a known kind; unknown labels fail.
        for label in ["auto", "familiar_review", "human_review", "human_required"] {
            let kind = ApprovalPath::from_display_label(label);
            assert!(
                kind.is_some(),
                "label {label:?} must round-trip to a known kind"
            );
            // kind → label must also agree.
            assert_eq!(kind.unwrap().display_label(), label);
        }
        // Unknown label → None (daemon rejects at load).
        assert_eq!(ApprovalPath::from_display_label("tier_0"), None);
        assert_eq!(ApprovalPath::from_display_label(""), None);
        assert_eq!(ApprovalPath::from_display_label("AUTO"), None); // case-sensitive
    }

    #[test]
    fn approval_path_serializes_with_kind_tag() {
        // serde tag = "kind" so the wire payload carries the discriminant.
        let path = ApprovalPath::HumanApproval;
        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("\"kind\":\"human_approval\""), "{json}");
        let back: ApprovalPath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, back);
    }

    // ---- ApprovalPath::highest ----

    #[test]
    fn highest_ceremony_wins() {
        let auto = ApprovalPath::AutoRegression { veto: None };
        let human = ApprovalPath::HumanApproval;
        let rationale = ApprovalPath::HumanApprovalWithRationale;

        assert_eq!(auto.clone().highest(human.clone()), human.clone());
        assert_eq!(human.clone().highest(auto.clone()), human.clone());
        assert_eq!(human.clone().highest(rationale.clone()), rationale.clone());
        assert_eq!(rationale.clone().highest(auto.clone()), rationale.clone());
        // Symmetric.
        assert_eq!(auto.clone().highest(rationale.clone()), rationale.clone());
    }

    // ---- VetoWindow ----

    #[test]
    fn veto_window_deadline_is_staged_at_plus_duration() {
        let w = VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(300));
        let staged = OffsetDateTime::UNIX_EPOCH;
        let dl = w.deadline(staged);
        assert_eq!(
            dl,
            staged + time::Duration::hours(1),
            "deadline should be staged_at + 1h"
        );
    }

    #[test]
    fn veto_window_earliest_close_is_staged_at_plus_min_visible() {
        let w = VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(600));
        let staged = OffsetDateTime::UNIX_EPOCH;
        let ec = w.earliest_close(staged);
        assert_eq!(ec, staged + time::Duration::minutes(10));
    }

    #[test]
    fn veto_window_min_visible_gate() {
        let w = VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(600));
        let staged = OffsetDateTime::UNIX_EPOCH;
        // Before min_visible elapses → not eligible to close.
        let too_early = staged + time::Duration::seconds(599);
        assert!(!w.is_min_visible_elapsed(staged, too_early));
        // After min_visible → eligible.
        let after = staged + time::Duration::seconds(600);
        assert!(w.is_min_visible_elapsed(staged, after));
    }

    #[test]
    #[should_panic(expected = "min_visible")]
    fn veto_window_rejects_min_visible_greater_than_duration() {
        // min_visible > duration is a construction error.
        let _ = VetoWindow::new(Duration::from_secs(300), Duration::from_secs(600));
    }

    // ---- WindowCloseReason ----

    #[test]
    fn window_close_reason_tags_are_stable() {
        let pairs = [
            (WindowCloseReason::Applied, "applied"),
            (WindowCloseReason::Vetoed, "vetoed"),
            (WindowCloseReason::Expired, "expired"),
            (WindowCloseReason::Superseded, "superseded"),
        ];
        for (reason, expected_tag) in pairs {
            assert_eq!(reason.tag(), expected_tag);
            // serde round-trip.
            let json = serde_json::to_string(&reason).unwrap();
            let back: WindowCloseReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, back);
        }
    }

    // ---- SurfaceRegionId ----

    #[test]
    fn surface_region_id_roundtrips() {
        let id = SurfaceRegionId::new("execution_prompt");
        assert_eq!(id.as_str(), "execution_prompt");
        let json = serde_json::to_string(&id).unwrap();
        let back: SurfaceRegionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    // ---- ProposalClassification ----

    #[test]
    fn proposal_classification_roundtrips_json() {
        use crate::ids::{FamiliarId, ProposalId};

        let c = ProposalClassification {
            proposal_id: ProposalId::new(),
            familiar_id: FamiliarId::new(),
            channel: Channel::Mutation,
            affected_regions: vec![SurfaceRegionId::new("output_formats")],
            path_tier_floor: 1,
            approval_path: ApprovalPath::FamiliarCoherence {
                veto: VetoWindow::new(Duration::from_secs(7200), Duration::from_secs(900)),
            },
            evidence_replay_hash: [0xde; 32],
            classified_at: OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_string_pretty(&c).unwrap();
        let back: ProposalClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    // ---- ApprovalPathWireEnvelope ----

    #[test]
    fn wire_envelope_label_round_trip_validates() {
        use crate::ids::{FamiliarId, ProposalId};

        let classification = ProposalClassification {
            proposal_id: ProposalId::new(),
            familiar_id: FamiliarId::new(),
            channel: Channel::Mutation,
            affected_regions: vec![],
            path_tier_floor: 0,
            approval_path: ApprovalPath::HumanApprovalWithRationale,
            evidence_replay_hash: [0xaa; 32],
            classified_at: OffsetDateTime::UNIX_EPOCH,
        };
        let env = ApprovalPathWireEnvelope::from_classification(&classification, None);
        assert_eq!(env.label, "human_required");
        assert_eq!(env.variant, ApprovalPathKind::HumanApprovalWithRationale);
        assert!(env.validate_label_round_trip().is_ok());
    }

    #[test]
    fn wire_envelope_rejects_unknown_label() {
        let mut env = ApprovalPathWireEnvelope {
            variant: ApprovalPathKind::AutoRegression,
            label: "auto".into(),
            veto_deadline: None,
            affected_surfaces: vec![],
        };
        // Corrupt the label.
        env.label = "tier_0".into();
        assert!(env.validate_label_round_trip().is_err());
    }

    #[test]
    fn wire_envelope_rejects_mismatched_label_and_variant() {
        let env = ApprovalPathWireEnvelope {
            variant: ApprovalPathKind::FamiliarCoherence,
            label: "auto".into(), // mismatch: auto != familiar_review
            veto_deadline: None,
            affected_surfaces: vec![],
        };
        assert!(env.validate_label_round_trip().is_err());
    }

    #[test]
    fn auto_path_veto_window_present_and_absent() {
        let with_veto = ApprovalPath::AutoRegression {
            veto: Some(VetoWindow::new(
                Duration::from_secs(1800),
                Duration::from_secs(60),
            )),
        };
        let without_veto = ApprovalPath::AutoRegression { veto: None };
        assert!(with_veto.has_veto_window());
        assert!(!without_veto.has_veto_window());
        assert!(with_veto.veto_window().is_some());
        assert!(without_veto.veto_window().is_none());
    }
}
