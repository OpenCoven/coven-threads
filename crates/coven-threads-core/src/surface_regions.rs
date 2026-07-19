//! Surface-region design — `SurfaceRegionPredicate` + Gate-4 replay (`threads-uqx.5`).
//!
//! ## Motivation (spec §2.2, §3.3, decision 3)
//!
//! Phase 2 protects files via `SurfaceId`. Phase 5 introduces a finer-grained
//! abstraction: a **`SurfaceRegion`** is a typed semantic area within a surface
//! (or across surfaces) that the daemon can classify from a materialized diff.
//!
//! The rule: **classify first; promote to thread later** (decision 3). By
//! default, regions attach their evidence to `ProposalClassification`, not to
//! new thread/strand entries. A region becomes a thread only when it has a
//! stable source-authoritative projection.
//!
//! ## The trait
//!
//! ```rust,ignore
//! pub trait SurfaceRegionPredicate {
//!     fn materialize(&self, proposal: &MaterializedDiff) -> RegionEvidence;
//!     fn describe(&self) -> SurfaceRegionDescriptor;
//! }
//! ```
//!
//! - `materialize` — authoritative. The daemon calls this to produce
//!   `RegionEvidence` that feeds `ProposalClassification.affected_regions` and
//!   Gate-4 deadline replay.
//! - `describe` — derived. Human-readable; Cave-renderable; never enforced on.
//!
//! ## Gate-4 replay constraint
//!
//! The predicate **cannot** depend on:
//! - Cave state
//! - Agent self-report
//! - Stale metadata cached outside the diff
//!
//! It must be a pure function of `MaterializedDiff`. The daemon replays it at
//! the Gate-4 deadline; if the output changes, the proposal is rejected
//! (WARD-C7 generalised — the `evidence_replay_hash` on
//! [`ProposalClassification`] covers region evidence).
//!
//! ## Forward-only reclassification
//!
//! If a region reclassifies mid-session (e.g. a proposal is amended), the new
//! classification applies forward only. Retroactive projection would corrupt
//! the authority trail.
//!
//! [`ProposalClassification`]: crate::approval::ProposalClassification

use serde::{Deserialize, Serialize};

use crate::approval::SurfaceRegionId;
use crate::ids::SurfaceId;

// ---------------------------------------------------------------------------
// MaterializedDiff
// ---------------------------------------------------------------------------

/// A materialized diff — the input to `SurfaceRegionPredicate::materialize`.
///
/// This represents the computed before/after state of every surface a proposal
/// would modify. The daemon materializes this from staged edits before calling
/// any predicate; predicates are pure over it.
///
/// ## Purity requirement
///
/// Predicates MUST be pure functions of `MaterializedDiff`. No Cave state,
/// no network calls, no agent self-report, no stale metadata. The daemon must
/// be able to replay the same computation at Gate-4 deadline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedDiff {
    /// The per-surface before/after pairs.
    pub surfaces: Vec<SurfaceDiff>,
}

/// Before/after for a single surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceDiff {
    /// The surface being modified.
    pub surface: SurfaceId,
    /// Content before the proposal (bytes). `None` if the surface is new.
    pub before: Option<Vec<u8>>,
    /// Content after the proposal (bytes). `None` if the surface is deleted.
    pub after: Option<Vec<u8>>,
}

impl SurfaceDiff {
    /// Whether this surface is being added (no prior content).
    pub fn is_addition(&self) -> bool {
        self.before.is_none() && self.after.is_some()
    }

    /// Whether this surface is being deleted.
    pub fn is_deletion(&self) -> bool {
        self.before.is_some() && self.after.is_none()
    }

    /// Whether this surface content actually changed.
    pub fn is_modified(&self) -> bool {
        self.before != self.after
    }
}

impl MaterializedDiff {
    /// Look up the diff for a specific surface.
    pub fn for_surface(&self, surface: &SurfaceId) -> Option<&SurfaceDiff> {
        self.surfaces.iter().find(|s| &s.surface == surface)
    }

    /// All surfaces that are modified (added, changed, or deleted).
    pub fn modified_surfaces(&self) -> impl Iterator<Item = &SurfaceDiff> {
        self.surfaces.iter().filter(|s| s.is_modified())
    }
}

// ---------------------------------------------------------------------------
// RegionEvidence
// ---------------------------------------------------------------------------

/// The output of `SurfaceRegionPredicate::materialize` — evidence that a
/// specific semantic region was affected by a proposal.
///
/// This feeds `ProposalClassification.affected_regions` and is replayed at
/// Gate-4 deadline. The daemon hashes all `RegionEvidence` entries into
/// `ProposalClassification.evidence_replay_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionEvidence {
    /// The region this evidence belongs to.
    pub region_id: SurfaceRegionId,
    /// The surfaces on which this region was detected as affected.
    pub affected_surfaces: Vec<SurfaceId>,
    /// The minimum approval path tier this region requires.
    /// Feeds `ProposalClassification.path_tier_floor` (highest floor wins).
    pub min_path_tier: u8,
    /// Stable bytes that the daemon can hash for `evidence_replay_hash`.
    /// Must be deterministic over the same `MaterializedDiff`.
    pub replay_bytes: Vec<u8>,
    /// Human-readable explanation of why this region was triggered.
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// SurfaceRegionDescriptor
// ---------------------------------------------------------------------------

/// Derived, human-readable description of a `SurfaceRegionPredicate`.
///
/// **Never enforced on.** This is for Cave rendering, tool output, and audit
/// legibility only. The predicate output is authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceRegionDescriptor {
    /// Stable region id.
    pub region_id: SurfaceRegionId,
    /// Human-readable label (e.g. "execution_prompt", "output_formats").
    pub label: String,
    /// Which surfaces this region can span.
    pub candidate_surfaces: Vec<SurfaceId>,
    /// Minimum approval tier floor this region typically requires.
    pub typical_min_tier: u8,
    /// Free-text description for audit / UI.
    pub description: String,
}

// ---------------------------------------------------------------------------
// SurfaceRegionPredicate trait
// ---------------------------------------------------------------------------

/// Authoritative extractor predicate for a typed semantic surface region.
///
/// ## Contract
///
/// - `materialize` MUST be a pure function of `MaterializedDiff`. No external
///   state, no agent self-report, no Cave queries.
/// - `describe` is derived and non-authoritative.
/// - If extraction fails or is ambiguous, `materialize` MUST return evidence
///   with `min_path_tier` elevated (fail-conservative), or the daemon must
///   treat the missing evidence as a block (fail-closed, decision 3).
/// - A region reclassification applies forward only — never retroactively.
pub trait SurfaceRegionPredicate: std::fmt::Debug + Send + Sync {
    /// Authoritative materialization: extract region evidence from the diff.
    /// Returns `None` if this region is not affected by the diff at all.
    fn materialize(&self, proposal: &MaterializedDiff) -> Option<RegionEvidence>;

    /// Derived, non-authoritative descriptor. Cave-renderable. Never enforced
    /// on.
    fn describe(&self) -> SurfaceRegionDescriptor;
}

// ---------------------------------------------------------------------------
// Built-in region predicates
// ---------------------------------------------------------------------------

/// Region: execution prompt surface (e.g. `SOUL.md`, `AGENTS.md` system prompt
/// sections). Highest-protection semantic region — any modification requires at
/// minimum human review (tier floor 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPromptRegion {
    /// The surfaces that carry execution prompt content.
    pub prompt_surfaces: Vec<SurfaceId>,
}

impl ExecutionPromptRegion {
    /// Default: `SOUL.md` and `AGENTS.md` as execution prompt surfaces.
    pub fn default_protected() -> Self {
        Self {
            prompt_surfaces: vec![SurfaceId::new("SOUL.md"), SurfaceId::new("AGENTS.md")],
        }
    }
}

impl SurfaceRegionPredicate for ExecutionPromptRegion {
    fn materialize(&self, proposal: &MaterializedDiff) -> Option<RegionEvidence> {
        let affected: Vec<SurfaceId> = self
            .prompt_surfaces
            .iter()
            .filter(|s| {
                proposal
                    .for_surface(s)
                    .map(|d| d.is_modified())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if affected.is_empty() {
            return None;
        }

        // Deterministic replay bytes: sorted surface ids + content hashes.
        let mut replay_bytes = Vec::new();
        let mut sorted = affected.clone();
        sorted.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for surface in &sorted {
            replay_bytes.extend_from_slice(surface.as_str().as_bytes());
            replay_bytes.push(b':');
            if let Some(diff) = proposal.for_surface(surface) {
                if let Some(after) = &diff.after {
                    replay_bytes.extend_from_slice(after);
                }
            }
            replay_bytes.push(b'\n');
        }

        Some(RegionEvidence {
            region_id: SurfaceRegionId::new("execution_prompt"),
            affected_surfaces: affected,
            min_path_tier: 0, // execution prompt = highest protection = tier floor 0
            replay_bytes,
            rationale: "Execution prompt surface modified — human review required".into(),
        })
    }

    fn describe(&self) -> SurfaceRegionDescriptor {
        SurfaceRegionDescriptor {
            region_id: SurfaceRegionId::new("execution_prompt"),
            label: "Execution Prompt".into(),
            candidate_surfaces: self.prompt_surfaces.clone(),
            typical_min_tier: 0,
            description: "System-prompt and soul surfaces that define familiar behavior. \
                           Any modification triggers human review."
                .into(),
        }
    }
}

/// Region: tool defaults / skill config (e.g. `TOOLS.md`, `skills/*.toml`).
/// Requires at least familiar review (tier floor 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefaultsRegion {
    /// Surfaces that carry tool or skill config content.
    pub config_surfaces: Vec<SurfaceId>,
}

impl ToolDefaultsRegion {
    /// Default: `TOOLS.md`.
    pub fn default_protected() -> Self {
        Self {
            config_surfaces: vec![SurfaceId::new("TOOLS.md")],
        }
    }
}

impl SurfaceRegionPredicate for ToolDefaultsRegion {
    fn materialize(&self, proposal: &MaterializedDiff) -> Option<RegionEvidence> {
        let affected: Vec<SurfaceId> = self
            .config_surfaces
            .iter()
            .filter(|s| {
                proposal
                    .for_surface(s)
                    .map(|d| d.is_modified())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if affected.is_empty() {
            return None;
        }

        let mut replay_bytes = Vec::new();
        let mut sorted = affected.clone();
        sorted.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for surface in &sorted {
            replay_bytes.extend_from_slice(surface.as_str().as_bytes());
            replay_bytes.push(b':');
            if let Some(diff) = proposal.for_surface(surface) {
                if let Some(after) = &diff.after {
                    replay_bytes.extend_from_slice(after);
                }
            }
            replay_bytes.push(b'\n');
        }

        Some(RegionEvidence {
            region_id: SurfaceRegionId::new("tool_defaults"),
            affected_surfaces: affected,
            min_path_tier: 1, // familiar review
            replay_bytes,
            rationale: "Tool config surface modified — familiar review required".into(),
        })
    }

    fn describe(&self) -> SurfaceRegionDescriptor {
        SurfaceRegionDescriptor {
            region_id: SurfaceRegionId::new("tool_defaults"),
            label: "Tool Defaults".into(),
            candidate_surfaces: self.config_surfaces.clone(),
            typical_min_tier: 1,
            description: "Tool allowlists, skill config, and capability defaults. \
                           Modifications require familiar coherence review."
                .into(),
        }
    }
}

/// Region: heartbeat behavior (e.g. `HEARTBEAT.md`).
/// Requires at least familiar review (tier floor 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatBehaviorRegion {
    pub heartbeat_surfaces: Vec<SurfaceId>,
}

impl HeartbeatBehaviorRegion {
    pub fn default_protected() -> Self {
        Self {
            heartbeat_surfaces: vec![SurfaceId::new("HEARTBEAT.md")],
        }
    }
}

impl SurfaceRegionPredicate for HeartbeatBehaviorRegion {
    fn materialize(&self, proposal: &MaterializedDiff) -> Option<RegionEvidence> {
        let affected: Vec<SurfaceId> = self
            .heartbeat_surfaces
            .iter()
            .filter(|s| {
                proposal
                    .for_surface(s)
                    .map(|d| d.is_modified())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if affected.is_empty() {
            return None;
        }

        let mut replay_bytes = Vec::new();
        for surface in &affected {
            replay_bytes.extend_from_slice(surface.as_str().as_bytes());
            if let Some(diff) = proposal.for_surface(surface) {
                if let Some(after) = &diff.after {
                    replay_bytes.extend_from_slice(after);
                }
            }
        }

        Some(RegionEvidence {
            region_id: SurfaceRegionId::new("heartbeat_behavior"),
            affected_surfaces: affected,
            min_path_tier: 1,
            replay_bytes,
            rationale: "Heartbeat behavior surface modified".into(),
        })
    }

    fn describe(&self) -> SurfaceRegionDescriptor {
        SurfaceRegionDescriptor {
            region_id: SurfaceRegionId::new("heartbeat_behavior"),
            label: "Heartbeat Behavior".into(),
            candidate_surfaces: self.heartbeat_surfaces.clone(),
            typical_min_tier: 1,
            description: "Surfaces controlling familiar heartbeat schedule and checks.".into(),
        }
    }
}

/// A registry of region predicates. The daemon holds one of these and calls
/// `classify_all` at proposal intake to populate
/// `ProposalClassification.affected_regions`.
pub struct SurfaceRegionRegistry {
    predicates: Vec<Box<dyn SurfaceRegionPredicate>>,
}

impl std::fmt::Debug for SurfaceRegionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfaceRegionRegistry")
            .field("predicate_count", &self.predicates.len())
            .finish()
    }
}

impl SurfaceRegionRegistry {
    /// Construct with an explicit list of predicates.
    pub fn new(predicates: Vec<Box<dyn SurfaceRegionPredicate>>) -> Self {
        Self { predicates }
    }

    /// Default registry: execution prompt + tool defaults + heartbeat.
    pub fn default_registry() -> Self {
        Self::new(vec![
            Box::new(ExecutionPromptRegion::default_protected()),
            Box::new(ToolDefaultsRegion::default_protected()),
            Box::new(HeartbeatBehaviorRegion::default_protected()),
        ])
    }

    /// Classify all affected regions for a diff. Returns evidence for every
    /// region that `materialize` returns `Some` for.
    ///
    /// The results are in predicate registration order (stable for replay).
    pub fn classify_all(&self, diff: &MaterializedDiff) -> Vec<RegionEvidence> {
        self.predicates
            .iter()
            .filter_map(|p| p.materialize(diff))
            .collect()
    }

    /// Derive descriptors for all registered regions (for Cave / display).
    pub fn descriptors(&self) -> Vec<SurfaceRegionDescriptor> {
        self.predicates.iter().map(|p| p.describe()).collect()
    }

    /// The highest (lowest number = most protected) path tier floor across all
    /// evidence. Returns `u8::MAX` if no evidence (no floor imposed).
    pub fn path_tier_floor(evidence: &[RegionEvidence]) -> u8 {
        evidence
            .iter()
            .map(|e| e.min_path_tier)
            .min()
            .unwrap_or(u8::MAX)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn diff_with(surface: &str, before: Option<&[u8]>, after: Option<&[u8]>) -> MaterializedDiff {
        MaterializedDiff {
            surfaces: vec![SurfaceDiff {
                surface: SurfaceId::new(surface),
                before: before.map(|b| b.to_vec()),
                after: after.map(|b| b.to_vec()),
            }],
        }
    }

    // ---- SurfaceDiff helpers ----

    #[test]
    fn surface_diff_flags_modification_correctly() {
        let modified = SurfaceDiff {
            surface: SurfaceId::new("SOUL.md"),
            before: Some(b"old".to_vec()),
            after: Some(b"new".to_vec()),
        };
        assert!(modified.is_modified());
        assert!(!modified.is_addition());
        assert!(!modified.is_deletion());

        let added = SurfaceDiff {
            surface: SurfaceId::new("new.md"),
            before: None,
            after: Some(b"content".to_vec()),
        };
        assert!(added.is_addition());

        let deleted = SurfaceDiff {
            surface: SurfaceId::new("old.md"),
            before: Some(b"content".to_vec()),
            after: None,
        };
        assert!(deleted.is_deletion());
    }

    // ---- ExecutionPromptRegion ----

    #[test]
    fn execution_prompt_region_triggers_on_soul_md_change() {
        let region = ExecutionPromptRegion::default_protected();
        let diff = diff_with("SOUL.md", Some(b"old soul"), Some(b"new soul"));
        let evidence = region.materialize(&diff);
        assert!(evidence.is_some());
        let ev = evidence.unwrap();
        assert_eq!(ev.region_id.as_str(), "execution_prompt");
        assert_eq!(ev.min_path_tier, 0, "execution prompt is tier-0 protected");
        assert!(ev.affected_surfaces.contains(&SurfaceId::new("SOUL.md")));
        assert!(!ev.replay_bytes.is_empty());
    }

    #[test]
    fn execution_prompt_region_returns_none_for_unrelated_surface() {
        let region = ExecutionPromptRegion::default_protected();
        let diff = diff_with("README.md", Some(b"old"), Some(b"new"));
        assert!(region.materialize(&diff).is_none());
    }

    #[test]
    fn execution_prompt_region_replay_bytes_are_deterministic() {
        // Gate-4 replay: same diff → same replay_bytes.
        let region = ExecutionPromptRegion::default_protected();
        let diff = MaterializedDiff {
            surfaces: vec![
                SurfaceDiff {
                    surface: SurfaceId::new("SOUL.md"),
                    before: Some(b"A".to_vec()),
                    after: Some(b"B".to_vec()),
                },
                SurfaceDiff {
                    surface: SurfaceId::new("AGENTS.md"),
                    before: Some(b"X".to_vec()),
                    after: Some(b"Y".to_vec()),
                },
            ],
        };
        let ev1 = region.materialize(&diff).unwrap();
        let ev2 = region.materialize(&diff).unwrap();
        assert_eq!(ev1.replay_bytes, ev2.replay_bytes, "replay must be deterministic");
    }

    #[test]
    fn execution_prompt_region_describe_is_derived_and_non_authoritative() {
        let region = ExecutionPromptRegion::default_protected();
        let d = region.describe();
        assert_eq!(d.region_id.as_str(), "execution_prompt");
        assert_eq!(d.typical_min_tier, 0);
        // Descriptor roundtrips (for Cave rendering).
        let json = serde_json::to_string(&d).unwrap();
        let back: SurfaceRegionDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    // ---- ToolDefaultsRegion ----

    #[test]
    fn tool_defaults_region_triggers_on_tools_md() {
        let region = ToolDefaultsRegion::default_protected();
        let diff = diff_with("TOOLS.md", Some(b"old tools"), Some(b"new tools"));
        let ev = region.materialize(&diff).unwrap();
        assert_eq!(ev.region_id.as_str(), "tool_defaults");
        assert_eq!(ev.min_path_tier, 1);
    }

    // ---- SurfaceRegionRegistry ----

    #[test]
    fn registry_classify_all_returns_only_triggered_regions() {
        let registry = SurfaceRegionRegistry::default_registry();
        // Only HEARTBEAT.md changed → only heartbeat region triggers.
        let diff = diff_with("HEARTBEAT.md", Some(b"old"), Some(b"new"));
        let evidence = registry.classify_all(&diff);
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].region_id.as_str(), "heartbeat_behavior");
    }

    #[test]
    fn registry_classify_all_multiple_regions() {
        let registry = SurfaceRegionRegistry::default_registry();
        let diff = MaterializedDiff {
            surfaces: vec![
                SurfaceDiff {
                    surface: SurfaceId::new("SOUL.md"),
                    before: Some(b"old".to_vec()),
                    after: Some(b"new".to_vec()),
                },
                SurfaceDiff {
                    surface: SurfaceId::new("TOOLS.md"),
                    before: Some(b"old".to_vec()),
                    after: Some(b"new".to_vec()),
                },
            ],
        };
        let evidence = registry.classify_all(&diff);
        assert_eq!(evidence.len(), 2);
    }

    #[test]
    fn registry_path_tier_floor_is_minimum_across_evidence() {
        let evidence = vec![
            RegionEvidence {
                region_id: SurfaceRegionId::new("execution_prompt"),
                affected_surfaces: vec![],
                min_path_tier: 0,
                replay_bytes: vec![],
                rationale: "".into(),
            },
            RegionEvidence {
                region_id: SurfaceRegionId::new("tool_defaults"),
                affected_surfaces: vec![],
                min_path_tier: 1,
                replay_bytes: vec![],
                rationale: "".into(),
            },
        ];
        // Floor = min = 0 (most protective wins).
        assert_eq!(SurfaceRegionRegistry::path_tier_floor(&evidence), 0);
    }

    #[test]
    fn registry_path_tier_floor_max_when_no_evidence() {
        // No regions triggered → no floor imposed.
        assert_eq!(
            SurfaceRegionRegistry::path_tier_floor(&[]),
            u8::MAX,
            "no evidence = no floor"
        );
    }

    #[test]
    fn registry_descriptors_covers_all_registered_regions() {
        let registry = SurfaceRegionRegistry::default_registry();
        let descs = registry.descriptors();
        assert_eq!(descs.len(), 3);
        let ids: Vec<&str> = descs.iter().map(|d| d.region_id.as_str()).collect();
        assert!(ids.contains(&"execution_prompt"));
        assert!(ids.contains(&"tool_defaults"));
        assert!(ids.contains(&"heartbeat_behavior"));
    }

    #[test]
    fn classify_all_returns_empty_when_nothing_matches() {
        let registry = SurfaceRegionRegistry::default_registry();
        let diff = diff_with("CHANGELOG.md", Some(b"old"), Some(b"new"));
        assert!(registry.classify_all(&diff).is_empty());
    }
}
