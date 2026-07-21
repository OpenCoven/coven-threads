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
        ///
        /// The wire key MUST be present (`"veto": null` for the no-window
        /// path). A payload with the key absent is rejected: because `None` is
        /// the *most permissive* configuration, a truncated or hand-built
        /// payload must not silently resolve to it. The daemon always emits
        /// the key (serde serializes `None` as `null`).
        #[serde(deserialize_with = "deserialize_required_veto")]
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
    #[must_use]
    pub fn highest(self, other: ApprovalPath) -> ApprovalPath {
        match (self, other) {
            (
                ApprovalPath::AutoRegression { veto: left },
                ApprovalPath::AutoRegression { veto: right },
            ) => ApprovalPath::AutoRegression {
                veto: Self::strongest_optional_window(left, right),
            },
            (
                ApprovalPath::FamiliarCoherence { veto: left },
                ApprovalPath::FamiliarCoherence { veto: right },
            ) => ApprovalPath::FamiliarCoherence {
                veto: left.strongest(right),
            },
            (left, right) => {
                let (winner, loser) = if right.ceremony_ordinal() > left.ceremony_ordinal() {
                    (right, left)
                } else {
                    (left, right)
                };
                // Elevating ceremony must not shrink a veto window a lower
                // path demanded: a FamiliarCoherence winner merges with the
                // loser's window, mirroring the equal-variant arms. Human
                // paths carry no window — blocked-until-explicit-approval is
                // strictly stronger than any veto period, so nothing is lost.
                match (winner, loser) {
                    (
                        ApprovalPath::FamiliarCoherence { veto },
                        ApprovalPath::AutoRegression {
                            veto: Some(other_window),
                        },
                    ) => ApprovalPath::FamiliarCoherence {
                        veto: veto.strongest(other_window),
                    },
                    (winner, _) => winner,
                }
            }
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

    fn strongest_optional_window(
        left: Option<VetoWindow>,
        right: Option<VetoWindow>,
    ) -> Option<VetoWindow> {
        match (left, right) {
            (Some(left), Some(right)) => Some(left.strongest(right)),
            (Some(window), None) | (None, Some(window)) => Some(window),
            (None, None) => None,
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
    /// Deterministic regression path with an optional veto window.
    AutoRegression,
    /// Familiar-coherence review followed by a veto window.
    FamiliarCoherence,
    /// Explicit human approval.
    HumanApproval,
    /// Explicit human approval with a recorded rationale.
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
#[serde(try_from = "VetoWindowWire")]
pub struct VetoWindow {
    /// How long the window stays open from the moment the proposal is staged.
    duration: Duration,
    /// Minimum time the proposal must be *visibly pending* before the window
    /// may close. Prevents proposals that are technically pending but
    /// unreachable in practice (same shape as the two-compaction contract's
    /// minimum-visibility requirement).
    ///
    /// MUST be ≤ `duration`.
    min_visible: Duration,
}

impl VetoWindow {
    /// Construct a veto window, panicking if `min_visible > duration`.
    pub fn new(duration: Duration, min_visible: Duration) -> Self {
        Self::try_new(duration, min_visible).expect("invalid VetoWindow")
    }

    /// Construct a validated veto window.
    pub fn try_new(duration: Duration, min_visible: Duration) -> Result<Self, String> {
        if min_visible > duration {
            return Err(format!(
                "VetoWindow: min_visible ({min_visible:?}) must be ≤ duration ({duration:?})"
            ));
        }
        time::Duration::try_from(duration)
            .map_err(|_| format!("VetoWindow duration {duration:?} is not representable"))?;
        time::Duration::try_from(min_visible)
            .map_err(|_| format!("VetoWindow min_visible {min_visible:?} is not representable"))?;
        Ok(Self {
            duration,
            min_visible,
        })
    }

    /// Total time the veto window stays open.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Minimum time the proposal must remain visibly pending.
    pub fn min_visible(&self) -> Duration {
        self.min_visible
    }

    fn strongest(self, other: Self) -> Self {
        Self::new(
            self.duration.max(other.duration),
            self.min_visible.max(other.min_visible),
        )
    }

    /// Calculate the absolute deadline given a staged-at timestamp.
    pub fn deadline(&self, staged_at: OffsetDateTime) -> Result<OffsetDateTime, String> {
        let duration = time::Duration::try_from(self.duration)
            .map_err(|_| "VetoWindow duration is not representable".to_string())?;
        staged_at.checked_add(duration).ok_or_else(|| {
            "VetoWindow deadline is outside the representable timestamp range".into()
        })
    }

    /// The earliest time at which the window may close (staged_at + min_visible).
    pub fn earliest_close(&self, staged_at: OffsetDateTime) -> Result<OffsetDateTime, String> {
        let min_visible = time::Duration::try_from(self.min_visible)
            .map_err(|_| "VetoWindow min_visible is not representable".to_string())?;
        staged_at.checked_add(min_visible).ok_or_else(|| {
            "VetoWindow earliest close is outside the representable timestamp range".into()
        })
    }

    /// Whether the window may now be closed given the current time and when it
    /// was staged.
    ///
    /// Returns `Ok(true)` iff `now >= earliest_close(staged_at)`. The scheduler
    /// should additionally check that `now >= deadline(staged_at)` before
    /// auto-applying; this method only answers the min-visible gate.
    pub fn is_min_visible_elapsed(
        &self,
        staged_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<bool, String> {
        Ok(now >= self.earliest_close(staged_at)?)
    }
}

#[derive(Deserialize)]
struct VetoWindowWire {
    duration: Duration,
    min_visible: Duration,
}

impl TryFrom<VetoWindowWire> for VetoWindow {
    type Error = String;

    fn try_from(value: VetoWindowWire) -> Result<Self, Self::Error> {
        Self::try_new(value.duration, value.min_visible)
    }
}

/// Field-level deserializer that makes the `veto` key REQUIRED on
/// [`ApprovalPath::AutoRegression`] while still accepting an explicit `null`.
///
/// serde's derive treats `Option` fields as implicitly optional; here an
/// absent key would resolve to the most permissive no-window path, so absence
/// must reject instead (fail-closed).
fn deserialize_required_veto<'de, D>(deserializer: D) -> Result<Option<VetoWindow>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<VetoWindow>::deserialize(deserializer)
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
    /// Materialized surfaces affected by the proposal.
    pub affected_surfaces: Vec<SurfaceId>,
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
    /// Deadline replay produced different evidence; proposal was rejected.
    EvidenceDiverged,
    /// Deadline replay could not produce authoritative evidence; proposal was
    /// rejected fail-closed.
    RevalidationFailed,
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
            WindowCloseReason::EvidenceDiverged => "evidence_diverged",
            WindowCloseReason::RevalidationFailed => "revalidation_failed",
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
    #[serde(with = "time::serde::rfc3339")]
    pub deadline: OffsetDateTime,
    /// Earliest moment the window may close (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
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
    /// Whether the evidence replay hash matched at deadline. This is `None` for
    /// `Vetoed` and `Superseded`, where replay is not attempted.
    pub replay_hash_matched: Option<bool>,
    /// Rationale text (required for `HumanApprovalWithRationale` path; `None`
    /// for other paths unless the approver voluntarily adds one).
    pub rationale: Option<String>,
}

/// Audit detail for every successful proposal approval.
///
/// Human-required approvals carry their mandatory rationale here. Delayed
/// approval paths additionally carry the window-close evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalApprovalAuditDetail {
    /// Canonical [`ApprovalPath::display_label`] value.
    pub approval_path_label: String,
    /// Required and non-empty for `human_required`; optional otherwise.
    pub rationale: Option<String>,
    /// Delayed-apply close evidence, when this approval closed a veto window.
    pub window_close: Option<ProposalWindowCloseAuditDetail>,
}

// ---------------------------------------------------------------------------
// Wire envelope (daemon → client)
// ---------------------------------------------------------------------------

/// The wire envelope the daemon sends to clients for a pending proposal
/// (decision 7 — label, variant, optional deadline).
///
/// Cave and other clients receive this and render it as-is. They MUST NOT
/// infer policy from the variant or label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApprovalPathWireEnvelope {
    /// Stable enum variant name (snake_case, for machine use).
    pub variant: ApprovalPathKind,
    /// Human-readable display label (daemon wire contract — never remapped
    /// by clients).
    pub label: String,
    /// Absolute veto deadline, if this path has a veto window.
    #[serde(with = "time::serde::rfc3339::option")]
    pub veto_deadline: Option<OffsetDateTime>,
    /// The surfaces affected by this proposal (for display).
    pub affected_surfaces: Vec<SurfaceId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedApprovalPathWireEnvelope {
    variant: ApprovalPathKind,
    label: String,
    #[serde(with = "time::serde::rfc3339::option")]
    veto_deadline: Option<OffsetDateTime>,
    affected_surfaces: Vec<SurfaceId>,
}

impl<'de> Deserialize<'de> for ApprovalPathWireEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let unchecked = UncheckedApprovalPathWireEnvelope::deserialize(deserializer)?;
        let envelope = Self {
            variant: unchecked.variant,
            label: unchecked.label,
            veto_deadline: unchecked.veto_deadline,
            affected_surfaces: unchecked.affected_surfaces,
        };
        envelope
            .validate_label_round_trip()
            .map_err(serde::de::Error::custom)?;
        Ok(envelope)
    }
}

impl ApprovalPathWireEnvelope {
    /// Construct from a classification and optional staged-at timestamp.
    pub fn from_classification(
        classification: &ProposalClassification,
        staged_at: Option<OffsetDateTime>,
    ) -> Result<Self, String> {
        let variant = match &classification.approval_path {
            ApprovalPath::AutoRegression { .. } => ApprovalPathKind::AutoRegression,
            ApprovalPath::FamiliarCoherence { .. } => ApprovalPathKind::FamiliarCoherence,
            ApprovalPath::HumanApproval => ApprovalPathKind::HumanApproval,
            ApprovalPath::HumanApprovalWithRationale => {
                ApprovalPathKind::HumanApprovalWithRationale
            }
        };
        let label = classification.approval_path.display_label().to_string();
        let veto_deadline = match classification.approval_path.veto_window() {
            Some(window) => {
                let staged_at = staged_at.ok_or_else(|| {
                    "veto-bearing approval path requires a staged_at timestamp".to_string()
                })?;
                Some(window.deadline(staged_at)?)
            }
            None => None,
        };
        let envelope = Self {
            variant,
            label,
            veto_deadline,
            affected_surfaces: classification.affected_surfaces.clone(),
        };
        envelope.validate_label_round_trip()?;
        Ok(envelope)
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
        match self.variant {
            ApprovalPathKind::FamiliarCoherence if self.veto_deadline.is_none() => {
                return Err("familiar_review requires a veto deadline".into())
            }
            ApprovalPathKind::HumanApproval | ApprovalPathKind::HumanApprovalWithRationale
                if self.veto_deadline.is_some() =>
            {
                return Err("human approval paths must not carry a veto deadline".into())
            }
            _ => {}
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

    #[test]
    fn equal_auto_paths_preserve_the_strongest_veto_window() {
        let without_veto = ApprovalPath::AutoRegression { veto: None };
        let with_veto = ApprovalPath::AutoRegression {
            veto: Some(VetoWindow::new(
                Duration::from_secs(3600),
                Duration::from_secs(900),
            )),
        };

        assert_eq!(
            without_veto.highest(with_veto.clone()),
            with_veto,
            "aggregation must not drop a required veto window"
        );
    }

    #[test]
    fn equal_familiar_paths_merge_to_the_strongest_window() {
        let shorter = ApprovalPath::FamiliarCoherence {
            veto: VetoWindow::new(Duration::from_secs(1800), Duration::from_secs(300)),
        };
        let stronger = ApprovalPath::FamiliarCoherence {
            veto: VetoWindow::new(Duration::from_secs(7200), Duration::from_secs(900)),
        };

        assert_eq!(shorter.highest(stronger.clone()), stronger);
    }

    #[test]
    fn cross_variant_highest_merges_the_losing_paths_veto_window() {
        // Elevating AutoRegression{long window} to FamiliarCoherence{short
        // window} must not shrink the window a touched surface demanded.
        let auto_long = ApprovalPath::AutoRegression {
            veto: Some(VetoWindow::new(
                Duration::from_secs(7 * 24 * 3600),
                Duration::from_secs(24 * 3600),
            )),
        };
        let familiar_short = ApprovalPath::FamiliarCoherence {
            veto: VetoWindow::new(Duration::from_secs(300), Duration::from_secs(60)),
        };
        let merged = ApprovalPath::FamiliarCoherence {
            veto: VetoWindow::new(
                Duration::from_secs(7 * 24 * 3600),
                Duration::from_secs(24 * 3600),
            ),
        };

        assert_eq!(auto_long.clone().highest(familiar_short.clone()), merged);
        // Symmetric.
        assert_eq!(familiar_short.highest(auto_long), merged);
    }

    #[test]
    fn human_paths_drop_lower_windows_by_design() {
        // Blocked-until-explicit-approval is strictly stronger than any veto
        // period; the window is dropped, not merged, when a human path wins.
        let auto_with_window = ApprovalPath::AutoRegression {
            veto: Some(VetoWindow::new(
                Duration::from_secs(3600),
                Duration::from_secs(600),
            )),
        };
        let familiar = ApprovalPath::FamiliarCoherence {
            veto: VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(600)),
        };

        assert_eq!(
            auto_with_window.highest(ApprovalPath::HumanApproval),
            ApprovalPath::HumanApproval
        );
        assert_eq!(
            familiar.highest(ApprovalPath::HumanApprovalWithRationale),
            ApprovalPath::HumanApprovalWithRationale
        );
    }

    #[test]
    fn auto_regression_requires_the_veto_key_on_the_wire() {
        // Absent key must reject: None is the MOST permissive configuration,
        // so a truncated payload must not silently resolve to it.
        assert!(serde_json::from_str::<ApprovalPath>(r#"{"kind":"auto_regression"}"#).is_err());

        // Explicit null is the valid spelling of the no-window path.
        let none: ApprovalPath =
            serde_json::from_str(r#"{"kind":"auto_regression","veto":null}"#).unwrap();
        assert_eq!(none, ApprovalPath::AutoRegression { veto: None });

        // A full window still round-trips.
        let some: ApprovalPath = serde_json::from_str(
            r#"{"kind":"auto_regression","veto":{"duration":{"secs":3600,"nanos":0},"min_visible":{"secs":600,"nanos":0}}}"#,
        )
        .unwrap();
        assert!(some.has_veto_window());
    }

    #[test]
    fn auto_regression_serialization_always_emits_the_veto_key() {
        // The daemon serializes through this type, so every emitted payload
        // carries the key — which is what lets deserialization require it.
        let json = serde_json::to_string(&ApprovalPath::AutoRegression { veto: None }).unwrap();
        assert!(json.contains("\"veto\":null"), "{json}");
        let back: ApprovalPath = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ApprovalPath::AutoRegression { veto: None });
    }

    // ---- VetoWindow ----

    #[test]
    fn veto_window_deadline_is_staged_at_plus_duration() {
        let w = VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(300));
        let staged = OffsetDateTime::UNIX_EPOCH;
        let dl = w.deadline(staged).unwrap();
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
        let ec = w.earliest_close(staged).unwrap();
        assert_eq!(ec, staged + time::Duration::minutes(10));
    }

    #[test]
    fn veto_window_min_visible_gate() {
        let w = VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(600));
        let staged = OffsetDateTime::UNIX_EPOCH;
        // Before min_visible elapses → not eligible to close.
        let too_early = staged + time::Duration::seconds(599);
        assert!(!w.is_min_visible_elapsed(staged, too_early).unwrap());
        // After min_visible → eligible.
        let after = staged + time::Duration::seconds(600);
        assert!(w.is_min_visible_elapsed(staged, after).unwrap());
    }

    #[test]
    fn veto_window_timestamp_overflow_fails_closed() {
        let window = VetoWindow::new(Duration::from_secs(1), Duration::from_secs(1));
        let max = time::macros::datetime!(9999-12-31 23:59:59 UTC);
        assert!(window.deadline(max).is_err());
        assert!(window.earliest_close(max).is_err());
        assert!(window.is_min_visible_elapsed(max, max).is_err());
    }

    #[test]
    #[should_panic(expected = "min_visible")]
    fn veto_window_rejects_min_visible_greater_than_duration() {
        // min_visible > duration is a construction error.
        let _ = VetoWindow::new(Duration::from_secs(300), Duration::from_secs(600));
    }

    #[test]
    fn veto_window_deserialization_rejects_invalid_visibility() {
        let json = r#"{
            "duration":{"secs":300,"nanos":0},
            "min_visible":{"secs":600,"nanos":0}
        }"#;
        assert!(serde_json::from_str::<VetoWindow>(json).is_err());
    }

    #[test]
    fn veto_window_deserialization_rejects_unrepresentable_duration() {
        let json = r#"{
            "duration":{"secs":18446744073709551615,"nanos":999999999},
            "min_visible":{"secs":0,"nanos":0}
        }"#;
        assert!(serde_json::from_str::<VetoWindow>(json).is_err());
    }

    // ---- WindowCloseReason ----

    #[test]
    fn window_close_reason_tags_are_stable() {
        let pairs = [
            (WindowCloseReason::Applied, "applied"),
            (WindowCloseReason::Vetoed, "vetoed"),
            (WindowCloseReason::EvidenceDiverged, "evidence_diverged"),
            (WindowCloseReason::RevalidationFailed, "revalidation_failed"),
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
            affected_surfaces: vec![SurfaceId::new("SOUL.md")],
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
            affected_surfaces: vec![SurfaceId::new("SOUL.md")],
            affected_regions: vec![],
            path_tier_floor: 0,
            approval_path: ApprovalPath::HumanApprovalWithRationale,
            evidence_replay_hash: [0xaa; 32],
            classified_at: OffsetDateTime::UNIX_EPOCH,
        };
        let env = ApprovalPathWireEnvelope::from_classification(
            &classification,
            Some(OffsetDateTime::UNIX_EPOCH),
        )
        .unwrap();
        assert_eq!(env.label, "human_required");
        assert_eq!(env.variant, ApprovalPathKind::HumanApprovalWithRationale);
        assert_eq!(env.affected_surfaces, vec![SurfaceId::new("SOUL.md")]);
        assert!(env.validate_label_round_trip().is_ok());
    }

    #[test]
    fn wire_envelope_refuses_invalid_veto_staging_time() {
        use crate::ids::{FamiliarId, ProposalId};

        let classification = ProposalClassification {
            proposal_id: ProposalId::new(),
            familiar_id: FamiliarId::new(),
            channel: Channel::Mutation,
            affected_surfaces: vec![SurfaceId::new("SOUL.md")],
            affected_regions: vec![],
            path_tier_floor: 1,
            approval_path: ApprovalPath::FamiliarCoherence {
                veto: VetoWindow::new(Duration::from_secs(3600), Duration::from_secs(300)),
            },
            evidence_replay_hash: [0xaa; 32],
            classified_at: OffsetDateTime::UNIX_EPOCH,
        };

        assert!(ApprovalPathWireEnvelope::from_classification(&classification, None).is_err());
        let max = time::macros::datetime!(9999-12-31 23:59:59 UTC);
        assert!(ApprovalPathWireEnvelope::from_classification(&classification, Some(max)).is_err());
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
    fn wire_envelope_deserialization_rejects_invalid_contract() {
        let json = r#"{
            "variant":"familiar_coherence",
            "label":"auto",
            "veto_deadline":null,
            "affected_surfaces":[]
        }"#;
        assert!(serde_json::from_str::<ApprovalPathWireEnvelope>(json).is_err());
    }

    #[test]
    fn window_timestamps_use_rfc3339_on_the_wire() {
        let deadline = time::macros::datetime!(2026-07-21 12:00:00 UTC);
        let envelope = ApprovalPathWireEnvelope {
            variant: ApprovalPathKind::FamiliarCoherence,
            label: "familiar_review".into(),
            veto_deadline: Some(deadline),
            affected_surfaces: vec![],
        };
        let envelope_json = serde_json::to_value(&envelope).unwrap();
        assert!(envelope_json["veto_deadline"].is_string());
        let decoded: ApprovalPathWireEnvelope = serde_json::from_value(envelope_json).unwrap();
        assert_eq!(decoded, envelope);

        let detail = ProposalWindowAuditDetail {
            approval_path_label: "familiar_review".into(),
            deadline,
            earliest_close: deadline - time::Duration::minutes(5),
            evidence_replay_hash_hex: "ab".repeat(32),
            affected_regions: vec![],
        };
        let detail_json = serde_json::to_value(&detail).unwrap();
        assert!(detail_json["deadline"].is_string());
        assert!(detail_json["earliest_close"].is_string());
        let decoded: ProposalWindowAuditDetail = serde_json::from_value(detail_json).unwrap();
        assert_eq!(decoded, detail);
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
