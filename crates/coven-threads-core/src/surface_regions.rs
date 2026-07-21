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
#[serde(try_from = "MaterializedDiffWire")]
pub struct MaterializedDiff {
    /// The per-surface before/after pairs.
    surfaces: Vec<SurfaceDiff>,
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
    /// Construct a materialized diff, rejecting duplicate surface entries.
    pub fn try_new(surfaces: Vec<SurfaceDiff>) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        for surface in &surfaces {
            if !seen.insert(surface.surface.as_str()) {
                return Err(format!(
                    "materialized diff contains duplicate surface {:?}",
                    surface.surface.as_str()
                ));
            }
            if surface.before.is_none() && surface.after.is_none() {
                return Err(format!(
                    "materialized diff surface {:?} has neither before nor after content",
                    surface.surface.as_str()
                ));
            }
            if surface.before == surface.after {
                return Err(format!(
                    "materialized diff surface {:?} is unchanged",
                    surface.surface.as_str()
                ));
            }
        }
        Ok(Self { surfaces })
    }

    /// All surface entries in this materialized diff.
    pub fn surfaces(&self) -> &[SurfaceDiff] {
        &self.surfaces
    }

    /// Look up the diff for a specific surface.
    pub fn for_surface(&self, surface: &SurfaceId) -> Option<&SurfaceDiff> {
        self.surfaces.iter().find(|s| &s.surface == surface)
    }

    /// All surfaces that are modified (added, changed, or deleted).
    pub fn modified_surfaces(&self) -> impl Iterator<Item = &SurfaceDiff> {
        self.surfaces.iter().filter(|s| s.is_modified())
    }
}

#[derive(Deserialize)]
struct MaterializedDiffWire {
    surfaces: Vec<SurfaceDiff>,
}

impl TryFrom<MaterializedDiffWire> for MaterializedDiff {
    type Error = String;

    fn try_from(value: MaterializedDiffWire) -> Result<Self, Self::Error> {
        Self::try_new(value.surfaces)
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

/// Canonical Blake3 commitment over the materialized diff and region evidence.
///
/// Diff, evidence, and affected-surface ordering do not affect the result.
/// Human rationale is deliberately excluded because it is derived display
/// text, not authority. Every committed field is length-prefixed to prevent
/// boundary ambiguity.
#[must_use]
pub fn evidence_replay_hash(diff: &MaterializedDiff, evidence: &[RegionEvidence]) -> [u8; 32] {
    let mut entries: Vec<Vec<u8>> = evidence
        .iter()
        .map(|item| {
            let mut encoded = Vec::new();
            push_len_prefixed(&mut encoded, item.region_id.as_str().as_bytes());
            encoded.push(item.min_path_tier);

            let mut surfaces: Vec<&str> = item
                .affected_surfaces
                .iter()
                .map(SurfaceId::as_str)
                .collect();
            surfaces.sort_unstable();
            encoded.extend_from_slice(&(surfaces.len() as u64).to_le_bytes());
            for surface in surfaces {
                push_len_prefixed(&mut encoded, surface.as_bytes());
            }
            push_len_prefixed(&mut encoded, &item.replay_bytes);
            encoded
        })
        .collect();
    entries.sort();

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"coven-threads:proposal-evidence:v2");
    let diff_commitment = materialized_diff_commitment(diff);
    hasher.update(&(diff_commitment.len() as u64).to_le_bytes());
    hasher.update(&diff_commitment);
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update(&(entry.len() as u64).to_le_bytes());
        hasher.update(&entry);
    }
    *hasher.finalize().as_bytes()
}

fn materialized_diff_commitment(diff: &MaterializedDiff) -> Vec<u8> {
    let mut surfaces: Vec<_> = diff.surfaces().iter().collect();
    surfaces.sort_by(|left, right| left.surface.as_str().cmp(right.surface.as_str()));

    let mut commitment = Vec::new();
    push_len_prefixed(
        &mut commitment,
        b"coven-threads:materialized-diff-commitment:v1",
    );
    commitment.extend_from_slice(&(surfaces.len() as u64).to_le_bytes());
    for surface in surfaces {
        push_len_prefixed(&mut commitment, surface.surface.as_str().as_bytes());
        push_optional_content_commitment(&mut commitment, surface.before.as_deref());
        push_optional_content_commitment(&mut commitment, surface.after.as_deref());
    }
    commitment
}

fn push_len_prefixed(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    output.extend_from_slice(bytes);
}

fn materialized_surface_replay(proposal: &MaterializedDiff, affected: &[SurfaceId]) -> Vec<u8> {
    let mut surfaces = affected.to_vec();
    surfaces.sort_by(|left, right| left.as_str().cmp(right.as_str()));

    let mut replay = Vec::new();
    push_len_prefixed(&mut replay, b"coven-threads:surface-region-replay:v1");
    replay.extend_from_slice(&(surfaces.len() as u64).to_le_bytes());
    for surface in surfaces {
        push_len_prefixed(&mut replay, surface.as_str().as_bytes());
        let diff = proposal
            .for_surface(&surface)
            .expect("affected surfaces are selected from the materialized diff");
        push_optional_content_commitment(&mut replay, diff.before.as_deref());
        push_optional_content_commitment(&mut replay, diff.after.as_deref());
    }
    replay
}

fn push_optional_content_commitment(output: &mut Vec<u8>, content: Option<&[u8]>) {
    match content {
        Some(bytes) => {
            output.push(1);
            output.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            output.extend_from_slice(blake3::hash(bytes).as_bytes());
        }
        None => output.push(0),
    }
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

        let replay_bytes = materialized_surface_replay(proposal, &affected);

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

        let replay_bytes = materialized_surface_replay(proposal, &affected);

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
    /// Surfaces that define heartbeat behavior.
    pub heartbeat_surfaces: Vec<SurfaceId>,
}

impl HeartbeatBehaviorRegion {
    /// Default heartbeat behavior surface.
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

        let replay_bytes = materialized_surface_replay(proposal, &affected);

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
        MaterializedDiff::try_new(vec![SurfaceDiff {
            surface: SurfaceId::new(surface),
            before: before.map(|b| b.to_vec()),
            after: after.map(|b| b.to_vec()),
        }])
        .unwrap()
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

    #[test]
    fn materialized_diff_rejects_duplicate_surfaces() {
        let duplicate = vec![
            SurfaceDiff {
                surface: SurfaceId::new("SOUL.md"),
                before: Some(b"old".to_vec()),
                after: Some(b"unchanged".to_vec()),
            },
            SurfaceDiff {
                surface: SurfaceId::new("SOUL.md"),
                before: Some(b"old".to_vec()),
                after: Some(b"changed".to_vec()),
            },
        ];

        assert!(MaterializedDiff::try_new(duplicate).is_err());
    }

    #[test]
    fn materialized_diff_rejects_absent_before_and_after() {
        let error = MaterializedDiff::try_new(vec![SurfaceDiff {
            surface: SurfaceId::new("SOUL.md"),
            before: None,
            after: None,
        }])
        .unwrap_err();

        assert!(error.contains("neither before nor after content"));
    }

    #[test]
    fn materialized_diff_rejects_unchanged_surfaces() {
        let error = MaterializedDiff::try_new(vec![SurfaceDiff {
            surface: SurfaceId::new("SOUL.md"),
            before: Some(b"same".to_vec()),
            after: Some(b"same".to_vec()),
        }])
        .unwrap_err();

        assert!(error.contains("unchanged"));
    }

    #[test]
    fn materialized_diff_deserialization_rejects_duplicate_surfaces() {
        let json = r#"{"surfaces":[
            {"surface":"SOUL.md","before":null,"after":[97]},
            {"surface":"SOUL.md","before":[97],"after":[98]}
        ]}"#;
        assert!(serde_json::from_str::<MaterializedDiff>(json).is_err());
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
        let diff = MaterializedDiff::try_new(vec![
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
        ])
        .unwrap();
        let ev1 = region.materialize(&diff).unwrap();
        let ev2 = region.materialize(&diff).unwrap();
        assert_eq!(
            ev1.replay_bytes, ev2.replay_bytes,
            "replay must be deterministic"
        );
    }

    #[test]
    fn execution_prompt_replay_commits_the_before_state() {
        let region = ExecutionPromptRegion::default_protected();
        let first = diff_with("SOUL.md", Some(b"old-a"), Some(b"same-result"));
        let second = diff_with("SOUL.md", Some(b"old-b"), Some(b"same-result"));

        assert_ne!(
            region.materialize(&first).unwrap().replay_bytes,
            region.materialize(&second).unwrap().replay_bytes,
            "intervening live-state drift must change replay evidence"
        );
    }

    #[test]
    fn execution_prompt_replay_distinguishes_delete_from_empty_replace() {
        let region = ExecutionPromptRegion::default_protected();
        let deleted = diff_with("SOUL.md", Some(b"old"), None);
        let emptied = diff_with("SOUL.md", Some(b"old"), Some(b""));

        assert_ne!(
            region.materialize(&deleted).unwrap().replay_bytes,
            region.materialize(&emptied).unwrap().replay_bytes
        );
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
        let diff = MaterializedDiff::try_new(vec![
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
        ])
        .unwrap();
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

    #[test]
    fn evidence_replay_hash_is_independent_of_evidence_order() {
        let first = RegionEvidence {
            region_id: SurfaceRegionId::new("execution_prompt"),
            affected_surfaces: vec![SurfaceId::new("SOUL.md")],
            min_path_tier: 0,
            replay_bytes: b"alpha".to_vec(),
            rationale: "display only".into(),
        };
        let second = RegionEvidence {
            region_id: SurfaceRegionId::new("tool_defaults"),
            affected_surfaces: vec![SurfaceId::new("TOOLS.md")],
            min_path_tier: 1,
            replay_bytes: b"beta".to_vec(),
            rationale: "display only".into(),
        };

        assert_eq!(
            evidence_replay_hash(
                &diff_with("README.md", Some(b"a"), Some(b"b")),
                &[first.clone(), second.clone()]
            ),
            evidence_replay_hash(
                &diff_with("README.md", Some(b"a"), Some(b"b")),
                &[second, first]
            )
        );
    }

    #[test]
    fn evidence_replay_hash_commits_field_boundaries() {
        let split = RegionEvidence {
            region_id: SurfaceRegionId::new("ab"),
            affected_surfaces: vec![SurfaceId::new("c")],
            min_path_tier: 1,
            replay_bytes: b"d".to_vec(),
            rationale: String::new(),
        };
        let joined = RegionEvidence {
            region_id: SurfaceRegionId::new("a"),
            affected_surfaces: vec![SurfaceId::new("bc")],
            min_path_tier: 1,
            replay_bytes: b"d".to_vec(),
            rationale: String::new(),
        };

        assert_ne!(
            evidence_replay_hash(&diff_with("README.md", Some(b"a"), Some(b"b")), &[split]),
            evidence_replay_hash(&diff_with("README.md", Some(b"a"), Some(b"b")), &[joined])
        );
    }

    #[test]
    fn evidence_replay_hash_commits_unclassified_surfaces() {
        let first = MaterializedDiff::try_new(vec![
            SurfaceDiff {
                surface: SurfaceId::new("SOUL.md"),
                before: Some(b"old".to_vec()),
                after: Some(b"new".to_vec()),
            },
            SurfaceDiff {
                surface: SurfaceId::new("README.md"),
                before: Some(b"a".to_vec()),
                after: Some(b"b".to_vec()),
            },
        ])
        .unwrap();
        let second = MaterializedDiff::try_new(vec![
            SurfaceDiff {
                surface: SurfaceId::new("SOUL.md"),
                before: Some(b"old".to_vec()),
                after: Some(b"new".to_vec()),
            },
            SurfaceDiff {
                surface: SurfaceId::new("README.md"),
                before: Some(b"a".to_vec()),
                after: Some(b"c".to_vec()),
            },
        ])
        .unwrap();
        let region = ExecutionPromptRegion::default_protected();
        let first_evidence = vec![region.materialize(&first).unwrap()];
        let second_evidence = vec![region.materialize(&second).unwrap()];

        assert_ne!(
            evidence_replay_hash(&first, &first_evidence),
            evidence_replay_hash(&second, &second_evidence)
        );
    }
}
