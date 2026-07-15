//! `PatternPredicate` — the enforced pattern of a weave (§2.2, §4).
//!
//! Ward validates the *pattern*, not individual threads: the weave carries
//! authority; individual threads are just where it's expressed (§3.3.1).
//!
//! ## The descriptor-vs-predicate anti-pattern (§2.2, prose — Echo third turn)
//!
//! The pattern is defined by a *predicate* (`PatternPredicate::coherent`), and that
//! predicate is **authoritative**. The predicate also carries a *derived* structural
//! summary (`PatternPredicate::describe -> PatternDescriptor`) for humans, tools, and
//! CovenCave rendering.
//!
//! **The descriptor MUST NOT become authoritative.** If any downstream component ever
//! gates enforcement on the descriptor instead of the predicate, we have reinvented
//! the derived-index problem one layer up — the exact failure mode
//! Ward-authority-over-indexer-not-rows was designed to avoid. Descriptors are for
//! legibility; predicates are for enforcement. Same source-authoritative discipline
//! as the memory retrieval substrate (source authoritative, index derived), applied
//! to the authority layer itself. Do not collapse it.

use serde::{Deserialize, Serialize};

use crate::channel::Channel;
use crate::ids::SurfaceId;
use crate::strand::StrandKind;
use crate::thread::Thread;

/// The result of evaluating a pattern against a weave's threads (§4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaveCoherence {
    /// Every thread required by the pattern is intact (§2.2).
    Coherent,
    /// One or more surfaces degraded, but the pattern still partially holds. The
    /// weave surfaces *which surface* degraded, not just "something is wrong"
    /// (§2.2). Degraded surfaces become read-only until repair (§5); the familiar
    /// continues on other surfaces.
    Degraded {
        /// The surfaces that degraded, in stable order.
        degraded_surfaces: Vec<SurfaceId>,
        /// Human-readable diagnostic.
        reason: String,
    },
    /// The pattern fundamentally does not hold — no required surface retains an
    /// intact thread. Nothing to continue on.
    Broken {
        /// Human-readable diagnostic.
        reason: String,
    },
}

/// The authoritative gate predicate on a weave (§4).
///
/// Implementations name a specific pattern of authority. Trait-over-enum resolved
/// v0.1.1 (Echo second turn): patterns are what Ward defines, and Ward's vocabulary
/// of authority patterns must be externally definable. Introspection cost is paid
/// by `describe()`.
///
/// **`coherent` is authoritative. `describe` is derived.** See module docs.
pub trait PatternPredicate: std::fmt::Debug {
    /// Authoritative gate: does this set of threads, in current tension state,
    /// satisfy the pattern?
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence;

    /// Derived, non-authoritative structural summary. For humans, tools, and
    /// CovenCave rendering. MUST NOT be gated on downstream — if anything ever
    /// enforces on the descriptor instead of the predicate, that is the
    /// derived-index problem reinvented one layer up (§2.2).
    fn describe(&self) -> PatternDescriptor;
}

/// Serializable, stable structural summary of a `PatternPredicate` (§4).
///
/// Introspection surface only; not authoritative. Do not enforce on this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternDescriptor {
    /// Human-readable name of the pattern.
    pub name: String,
    /// The surfaces this pattern protects.
    pub protected_surfaces: Vec<SurfaceId>,
    /// The channels this pattern requires threads to hold on.
    pub channels_required: Vec<Channel>,
    /// The strand requirements this pattern names.
    pub strand_requirements: Vec<StrandRequirement>,
}

/// A single strand-kind requirement inside a `PatternDescriptor` (§4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrandRequirement {
    /// Which kind of strand.
    pub kind: StrandKind,
    /// On which channels the strand kind is required.
    pub required_on_channels: Vec<Channel>,
}

/// The default identity-surface pattern: every listed surface must have at least
/// one thread that holds under every required channel.
///
/// This is the pattern shape RFC-0001 §4.1's protected floor uses (SOUL.md,
/// IDENTITY.md, MEMORY.md, ward.toml). An empty `surfaces` list is vacuously
/// coherent *as a predicate*; the §5 validator still fail-closes on any request
/// whose surface has no thread, so an empty pattern protects nothing but permits
/// nothing extra either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllSurfacesHoldOnChannels {
    /// Human-readable name for this instance.
    pub name: String,
    /// The surfaces that must be covered.
    pub surfaces: Vec<SurfaceId>,
    /// The channels each surface must hold on.
    pub channels: Vec<Channel>,
}

impl AllSurfacesHoldOnChannels {
    /// The RFC-0001 §4.1 protected-surface floor: `SOUL.md`, `IDENTITY.md`,
    /// `MEMORY.md`, `ward.toml`, holding under `Forced`, `Serialization`, and
    /// `Mutation` — the three channels where load arrives without familiar
    /// cooperation (§2.4). `Deliberate` coverage is a per-thread choice, not a
    /// floor requirement.
    pub fn rfc0001_floor() -> Self {
        Self {
            name: "rfc0001-protected-floor".to_string(),
            surfaces: vec![
                SurfaceId::new("SOUL.md"),
                SurfaceId::new("IDENTITY.md"),
                SurfaceId::new("MEMORY.md"),
                SurfaceId::new("ward.toml"),
            ],
            channels: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
        }
    }
}

impl PatternPredicate for AllSurfacesHoldOnChannels {
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence {
        let mut degraded: Vec<SurfaceId> = Vec::new();
        let mut reasons: Vec<String> = Vec::new();

        for surface in &self.surfaces {
            let mut surface_reason: Option<String> = None;
            for channel in &self.channels {
                let covered = threads
                    .iter()
                    .any(|t| &t.surface == surface && t.holds_under(*channel).is_ok());
                if !covered && surface_reason.is_none() {
                    // Name the most specific failure for legibility (§2.3): a
                    // fraying strand beats "no thread".
                    let detail = threads
                        .iter()
                        .filter(|t| &t.surface == surface)
                        .find_map(|t| t.holds_under(*channel).err())
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "no thread on surface".to_string());
                    surface_reason = Some(format!("{surface} on {channel:?}: {detail}"));
                }
            }
            if let Some(r) = surface_reason {
                degraded.push(surface.clone());
                reasons.push(r);
            }
        }

        if degraded.is_empty() {
            WeaveCoherence::Coherent
        } else if degraded.len() == self.surfaces.len() && !self.surfaces.is_empty() {
            // No required surface holds: the pattern fundamentally fails.
            WeaveCoherence::Broken {
                reason: format!("no required surface holds: {}", reasons.join("; ")),
            }
        } else {
            // §2.2: a snapped/frayed/missing thread degrades the weave *at that
            // thread's surface*. The weave names which surfaces degraded.
            WeaveCoherence::Degraded {
                degraded_surfaces: degraded,
                reason: reasons.join("; "),
            }
        }
    }

    fn describe(&self) -> PatternDescriptor {
        // Derived from the predicate's own requirements (§2.4 channel floors) —
        // never the other way around.
        let mut strand_requirements: Vec<StrandRequirement> = Vec::new();
        for channel in &self.channels {
            for kind in channel.required_strand_kinds() {
                match strand_requirements.iter_mut().find(|r| r.kind == *kind) {
                    Some(req) => {
                        if !req.required_on_channels.contains(channel) {
                            req.required_on_channels.push(*channel);
                        }
                    }
                    None => strand_requirements.push(StrandRequirement {
                        kind: *kind,
                        required_on_channels: vec![*channel],
                    }),
                }
            }
        }
        PatternDescriptor {
            name: self.name.clone(),
            protected_surfaces: self.surfaces.clone(),
            channels_required: self.channels.clone(),
            strand_requirements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fray::SnapReason;
    use crate::ids::{ManifestId, StrandId, ThreadId, WriterId};
    use crate::strand::{HashAlgo, Strand};
    use crate::thread::TensionState;
    use time::OffsetDateTime;

    fn full_strands() -> Vec<Strand> {
        vec![
            Strand::ContentHash {
                id: StrandId::new(),
                algorithm: HashAlgo::Blake3,
                value: vec![1; 32],
            },
            Strand::ManifestEntry {
                id: StrandId::new(),
                manifest_id: ManifestId::new(),
                entry_hash: vec![2; 32],
            },
            Strand::SerializationMarker {
                id: StrandId::new(),
                format_version: "0.1.0".into(),
                contract_hash: vec![3; 32],
            },
        ]
    }

    fn floor_thread(surface: &str) -> Thread {
        Thread {
            id: ThreadId::new(),
            surface: SurfaceId::new(surface),
            writer: WriterId::new("principal:val"),
            strands: full_strands(),
            holds_under: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
            created_at: OffsetDateTime::now_utc(),
            tension: TensionState::Holds,
        }
    }

    fn floor_threads() -> Vec<Thread> {
        ["SOUL.md", "IDENTITY.md", "MEMORY.md", "ward.toml"]
            .into_iter()
            .map(floor_thread)
            .collect()
    }

    #[test]
    fn floor_pattern_coherent_when_fully_covered() {
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        assert_eq!(pattern.coherent(&floor_threads()), WeaveCoherence::Coherent);
    }

    #[test]
    fn missing_surface_degrades_at_that_surface() {
        // §2.2: degradation is localized and named, not "something is wrong".
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        let threads: Vec<Thread> = floor_threads()
            .into_iter()
            .filter(|t| t.surface.as_str() != "SOUL.md")
            .collect();
        match pattern.coherent(&threads) {
            WeaveCoherence::Degraded {
                degraded_surfaces, ..
            } => assert_eq!(degraded_surfaces, vec![SurfaceId::new("SOUL.md")]),
            other => panic!("expected Degraded at SOUL.md, got {other:?}"),
        }
    }

    #[test]
    fn snapped_thread_degrades_weave_at_its_surface_only() {
        // §2.2 + §5: snapped thread → that surface read-only; familiar continues
        // on other surfaces. The weave degrades locally, it does not break.
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        let mut threads = floor_threads();
        threads[0].snap(
            Channel::Mutation,
            SnapReason::Revoked,
            OffsetDateTime::now_utc(),
        );
        let snapped_surface = threads[0].surface.clone();
        match pattern.coherent(&threads) {
            WeaveCoherence::Degraded {
                degraded_surfaces,
                reason,
            } => {
                assert_eq!(degraded_surfaces, vec![snapped_surface]);
                assert!(
                    reason.contains("snapped"),
                    "reason should name the snap: {reason}"
                );
            }
            other => panic!("expected localized degradation, got {other:?}"),
        }
    }

    #[test]
    fn all_surfaces_failing_is_broken() {
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        match pattern.coherent(&[]) {
            WeaveCoherence::Broken { reason } => {
                assert!(reason.contains("no required surface holds"));
            }
            other => panic!("expected Broken with nothing covered, got {other:?}"),
        }
    }

    #[test]
    fn thread_not_covering_required_channel_is_not_coverage() {
        // Fail-closed: a Mutation-only thread does not satisfy a pattern that
        // requires Forced+Serialization+Mutation. NotCovered means "no".
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        let mut threads = floor_threads();
        threads[0].holds_under = vec![Channel::Mutation];
        let narrowed_surface = threads[0].surface.clone();
        match pattern.coherent(&threads) {
            WeaveCoherence::Degraded {
                degraded_surfaces, ..
            } => assert_eq!(degraded_surfaces, vec![narrowed_surface]),
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn describe_is_derived_and_deterministic() {
        let pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
        let d1 = pattern.describe();
        let d2 = pattern.describe();
        assert_eq!(d1, d2);
        // Descriptor reflects the predicate's channel floors (§2.4).
        assert!(d1
            .strand_requirements
            .iter()
            .any(|r| r.kind == StrandKind::SerializationMarker
                && r.required_on_channels == vec![Channel::Serialization]));
        // Round-trip through JSON: descriptors are for tools and rendering.
        let json = serde_json::to_string(&d1).unwrap();
        let back: PatternDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(d1, back);
    }

    #[test]
    fn empty_pattern_is_vacuously_coherent() {
        // Documented: an empty pattern protects nothing. The §5 validator still
        // fail-closes on surfaces without threads, so this cannot widen authority.
        let pattern = AllSurfacesHoldOnChannels {
            name: "empty".into(),
            surfaces: vec![],
            channels: vec![Channel::Mutation],
        };
        assert_eq!(pattern.coherent(&[]), WeaveCoherence::Coherent);
    }
}
