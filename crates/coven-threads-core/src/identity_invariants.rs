//! Identity invariant `PatternPredicate` implementations (`threads-uqx.4`).
//!
//! This module carries forward the Phase-0/RFC-0001 identity invariants as
//! **predicate implementations**, not strings. The v0.1 textual invariant
//! syntax is dead; these are its typed successors.
//!
//! ## Design principle (spec §2.3, §3.2, decision 4)
//!
//! Identity invariants compile into [`PatternPredicate`] implementations.
//! The predicate is **authoritative**; its [`PatternDescriptor`] is **derived
//! and never enforced on**. This is the same source-authoritative discipline
//! that Phase 0 established for the thread/strand/weave spine — applied here
//! to semantic identity checks.
//!
//! ## Referent mapping (§3.2)
//!
//! | Coven type | Role in identity invariants |
//! |---|---|
//! | [`Thread`] | Authority relationship: protected identity surface → writer |
//! | [`Strand`] | Evidence: hashes, manifest entries, audit anchors |
//! | Weave | Identity predicates are part of the weave pattern, not side-table rows |
//! | [`Channel`] | Invariants must hold under `Mutation` and `Serialization` at minimum |
//!
//! ## Advisory probes (decision 4 / RFC-0001 §5.4–5.5)
//!
//! Model-judgment signals (confidence scores, semantic drift flags, pattern
//! matches) are **Gate-3 evidence only**. They are supplementary; they are
//! never the sole authority for any gated path. The structural/deterministic
//! predicate result is authoritative; probe output feeds `advisory_probes` on
//! the proposal surface, never `probes`.
//!
//! See [`AdvisoryProbe`] and [`AdvisoryProbeResult`] for the type shapes.
//!
//! ## Fail-closed ambiguity rule (decision 4)
//!
//! When deterministic extraction fails or is ambiguous — missing strand,
//! hash mismatch, unparseable manifest entry — the predicate MUST fail closed
//! (return `WeaveCoherence::Broken` or `WeaveCoherence::Degraded`). Silent
//! fallback to probe judgment is forbidden for gates; probes are supplementary
//! evidence, not fallback authorities.

use serde::{Deserialize, Serialize};

use crate::channel::Channel;
use crate::ids::SurfaceId;
use crate::pattern::{PatternDescriptor, PatternPredicate, StrandRequirement, WeaveCoherence};
use crate::strand::{HashAlgo, Strand, StrandKind};
use crate::thread::Thread;

// ---------------------------------------------------------------------------
// FamiliarNameInvariant
// ---------------------------------------------------------------------------

/// Invariant: the familiar's declared name has not changed from the pinned
/// value, as proven by the content hash of its primary identity surface.
///
/// ## How it works
///
/// At weave construction time, the principal records the `expected_hash` of the
/// identity surface (e.g. `IDENTITY.md`) using [`HashAlgo::Blake3`]. At gate
/// time the daemon extracts the `ContentHash` strand from the thread on that
/// surface and compares byte-for-byte. Mismatch → fail closed.
///
/// This is not a string comparison of the familiar name. It is a commitment to
/// the entire identity surface. If the name changes, so does the hash; the
/// invariant fails and the proposal must clear human review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamiliarNameInvariant {
    /// The identity surface that carries the name (typically `IDENTITY.md`).
    pub surface: SurfaceId,
    /// Expected Blake3 content hash (32 bytes) of the surface at weave
    /// construction time. The predicate fails closed if no thread for this
    /// surface carries a matching hash strand.
    pub expected_hash: [u8; 32],
    /// Human-readable description of what this invariant is protecting.
    pub description: String,
}

impl PatternPredicate for FamiliarNameInvariant {
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence {
        let surface_threads: Vec<&Thread> = threads
            .iter()
            .filter(|t| t.surface == self.surface)
            .collect();

        if surface_threads.is_empty() {
            return WeaveCoherence::Broken {
                reason: format!(
                    "FamiliarNameInvariant: no thread found for identity surface {:?} — \
                     fail closed (§2.3: deterministic extraction failure)",
                    self.surface
                ),
            };
        }

        // Check each thread for a ContentHash strand that matches the expected
        // hash. Any single matching, intact thread is sufficient.
        for thread in &surface_threads {
            if thread.holds_under(Channel::Mutation).is_err() {
                // This thread is frayed/snapped; skip it.
                continue;
            }
            for strand in &thread.strands {
                if let Strand::ContentHash { value, .. } = strand {
                    if value.as_slice() == self.expected_hash.as_slice() {
                        // Hash matches on an intact thread — invariant holds.
                        return WeaveCoherence::Coherent;
                    } else {
                        // Hash mismatch → fail closed; the identity surface has
                        // been mutated without this invariant's knowledge.
                        return WeaveCoherence::Broken {
                            reason: format!(
                                "FamiliarNameInvariant: content hash mismatch on {:?} — \
                                 expected {:?}, found {:?}. Identity surface mutation \
                                 requires human review.",
                                self.surface,
                                &self.expected_hash[..4],
                                &value[..4.min(value.len())],
                            ),
                        };
                    }
                }
            }
        }

        // Surface thread exists but carries no ContentHash strand — fail closed.
        WeaveCoherence::Broken {
            reason: format!(
                "FamiliarNameInvariant: thread for {:?} carries no ContentHash strand — \
                 fail closed (decision 4: ambiguity → reject, not fallback to probe)",
                self.surface
            ),
        }
    }

    fn describe(&self) -> PatternDescriptor {
        PatternDescriptor {
            name: format!(
                "familiar-name-invariant({})",
                self.surface.as_str()
            ),
            protected_surfaces: vec![self.surface.clone()],
            channels_required: vec![Channel::Mutation, Channel::Serialization],
            strand_requirements: vec![StrandRequirement {
                kind: StrandKind::ContentHash,
                required_on_channels: vec![Channel::Mutation, Channel::Serialization],
            }],
        }
    }
}

// ---------------------------------------------------------------------------
// ManifestAnchoredInvariant
// ---------------------------------------------------------------------------

/// Invariant: a protected surface's entry in the familiar manifest has not
/// changed, proven by the manifest entry hash.
///
/// This is the manifest-layer complement to [`FamiliarNameInvariant`]:
/// rather than hashing the surface content directly, it anchors to the
/// principal's external manifest record. This is the channel `Forced` requires
/// (WARD-C1..C6) — the invariant must survive compaction without familiar
/// cooperation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestAnchoredInvariant {
    /// The surface protected by this invariant.
    pub surface: SurfaceId,
    /// Expected entry hash (32 bytes) as recorded in the manifest at weave
    /// construction time.
    pub expected_entry_hash: [u8; 32],
    /// Human-readable label.
    pub description: String,
}

impl PatternPredicate for ManifestAnchoredInvariant {
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence {
        let surface_threads: Vec<&Thread> = threads
            .iter()
            .filter(|t| t.surface == self.surface)
            .collect();

        if surface_threads.is_empty() {
            return WeaveCoherence::Broken {
                reason: format!(
                    "ManifestAnchoredInvariant: no thread for {:?} — fail closed",
                    self.surface
                ),
            };
        }

        for thread in &surface_threads {
            // Forced channel is the strictest; if the thread doesn't hold
            // under Forced, it cannot prove survival across compaction.
            if thread.holds_under(Channel::Forced).is_err() {
                continue;
            }
            for strand in &thread.strands {
                if let Strand::ManifestEntry { entry_hash, .. } = strand {
                    if entry_hash.as_slice() == self.expected_entry_hash.as_slice() {
                        return WeaveCoherence::Coherent;
                    } else {
                        return WeaveCoherence::Broken {
                            reason: format!(
                                "ManifestAnchoredInvariant: manifest entry hash mismatch \
                                 on {:?} — pinned {:?}, found {:?}. Requires principal review.",
                                self.surface,
                                &self.expected_entry_hash[..4],
                                &entry_hash[..4.min(entry_hash.len())],
                            ),
                        };
                    }
                }
            }
        }

        WeaveCoherence::Broken {
            reason: format!(
                "ManifestAnchoredInvariant: no Forced-intact ManifestEntry strand for \
                 {:?} — fail closed (WARD-C1..C6: must survive compaction)",
                self.surface
            ),
        }
    }

    fn describe(&self) -> PatternDescriptor {
        PatternDescriptor {
            name: format!(
                "manifest-anchored-invariant({})",
                self.surface.as_str()
            ),
            protected_surfaces: vec![self.surface.clone()],
            channels_required: vec![Channel::Forced, Channel::Serialization],
            strand_requirements: vec![
                StrandRequirement {
                    kind: StrandKind::ContentHash,
                    required_on_channels: vec![Channel::Forced],
                },
                StrandRequirement {
                    kind: StrandKind::ManifestEntry,
                    required_on_channels: vec![Channel::Forced],
                },
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// CompositeIdentityInvariant
// ---------------------------------------------------------------------------

/// A composite identity predicate that requires ALL component predicates to
/// be coherent. Used to assemble the full familiar identity invariant from
/// individual hash/manifest anchors.
///
/// ## Fail-closed semantics
///
/// If any component predicate returns `Degraded` or `Broken`, the composite
/// returns the worst outcome across all components. There is no partial-pass
/// for identity — the identity invariant holds as a unit or not at all.
pub struct CompositeIdentityInvariant {
    /// Label for this composite (used in descriptor and error messages).
    pub name: String,
    /// Component predicates, evaluated in order.
    pub components: Vec<Box<dyn PatternPredicate + Send + Sync>>,
}

impl std::fmt::Debug for CompositeIdentityInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeIdentityInvariant")
            .field("name", &self.name)
            .field("component_count", &self.components.len())
            .finish()
    }
}

impl PatternPredicate for CompositeIdentityInvariant {
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence {
        let mut degraded_surfaces: Vec<SurfaceId> = Vec::new();
        let mut broken_reasons: Vec<String> = Vec::new();
        let mut degraded_reasons: Vec<String> = Vec::new();

        for component in &self.components {
            match component.coherent(threads) {
                WeaveCoherence::Coherent => {}
                WeaveCoherence::Degraded {
                    degraded_surfaces: mut s,
                    reason,
                } => {
                    degraded_surfaces.append(&mut s);
                    degraded_reasons.push(reason);
                }
                WeaveCoherence::Broken { reason } => {
                    broken_reasons.push(reason);
                }
            }
        }

        if !broken_reasons.is_empty() {
            WeaveCoherence::Broken {
                reason: broken_reasons.join("; "),
            }
        } else if !degraded_surfaces.is_empty() {
            WeaveCoherence::Degraded {
                degraded_surfaces,
                reason: degraded_reasons.join("; "),
            }
        } else {
            WeaveCoherence::Coherent
        }
    }

    fn describe(&self) -> PatternDescriptor {
        // Merge component descriptors into one. Surfaces and requirements union.
        let mut all_surfaces: Vec<SurfaceId> = Vec::new();
        let mut all_channels: Vec<Channel> = Vec::new();
        let mut all_requirements: Vec<StrandRequirement> = Vec::new();

        for component in &self.components {
            let d = component.describe();
            for s in d.protected_surfaces {
                if !all_surfaces.contains(&s) {
                    all_surfaces.push(s);
                }
            }
            for c in d.channels_required {
                if !all_channels.contains(&c) {
                    all_channels.push(c);
                }
            }
            for r in d.strand_requirements {
                match all_requirements.iter_mut().find(|x| x.kind == r.kind) {
                    Some(existing) => {
                        for ch in r.required_on_channels {
                            if !existing.required_on_channels.contains(&ch) {
                                existing.required_on_channels.push(ch);
                            }
                        }
                    }
                    None => all_requirements.push(r),
                }
            }
        }

        PatternDescriptor {
            name: self.name.clone(),
            protected_surfaces: all_surfaces,
            channels_required: all_channels,
            strand_requirements: all_requirements,
        }
    }
}

// ---------------------------------------------------------------------------
// Advisory probes (Gate-3 only — decision 4 / RFC-0001 §5.4–5.5)
// ---------------------------------------------------------------------------

/// A model-judgment advisory probe result (Gate-3 supplementary evidence).
///
/// Advisory probes are **not authoritative**. They are produced by model-based
/// evaluation (confidence scoring, semantic drift detection, pattern matching)
/// and feed the `advisory_probes` block on the proposal surface. They MUST NOT
/// be placed in the deterministic `probes` block, and they MUST NOT be the
/// sole authority for any gated decision path (decision 4, Q4 answer).
///
/// ## Structural separation (Q4 decision)
///
/// Deterministic gate evidence → `probes` block.
/// Model-judgment signals → `advisory_probes` block.
///
/// This is a schema-level distinction, not a display hint. Rules gating on
/// `probes` must not fire on `advisory_probes`, and vice versa. An empty
/// `advisory_probes` means "no advisory signals ran" — not "no probes at all."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisoryProbeResult {
    /// Probe kind identifier (e.g. "semantic_drift", "confidence_score",
    /// "pattern_match"). Not used for gating — for display and audit only.
    pub kind: String,
    /// Human-readable label for this probe.
    pub label: String,
    /// Numeric confidence in [0.0, 1.0], if the probe produces one.
    pub confidence: Option<f64>,
    /// Freeform signal text (a summary, a matched pattern, a drift description).
    pub signal: Option<String>,
    /// Whether this probe flagged a potential issue.
    pub flagged: bool,
    /// IMPORTANT: advisory probe results are NEVER gated on directly.
    /// This field is always `false` in a well-formed probe result and is
    /// included here as a documentation anchor.
    #[serde(default)]
    pub is_authoritative: bool,
}

impl AdvisoryProbeResult {
    /// Construct a new advisory probe result.
    ///
    /// `is_authoritative` is always forced to `false` — advisory probes are
    /// structurally non-authoritative (decision 4).
    pub fn new(
        kind: impl Into<String>,
        label: impl Into<String>,
        confidence: Option<f64>,
        signal: Option<String>,
        flagged: bool,
    ) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            confidence,
            signal,
            flagged,
            is_authoritative: false,
        }
    }

    /// Validate that this probe result is structurally well-formed:
    /// - confidence, if present, is in [0.0, 1.0]
    /// - is_authoritative is false (enforced at construction; checked here for
    ///   wire-received values)
    pub fn validate(&self) -> Result<(), String> {
        if self.is_authoritative {
            return Err(
                "AdvisoryProbeResult.is_authoritative must be false — \
                 advisory probes are never authoritative (decision 4)"
                    .into(),
            );
        }
        if let Some(c) = self.confidence {
            if !(0.0..=1.0).contains(&c) {
                return Err(format!(
                    "AdvisoryProbeResult.confidence must be in [0.0, 1.0], got {c}"
                ));
            }
        }
        Ok(())
    }
}

/// A collection of advisory probe results for a single proposal.
///
/// This is the `advisory_probes` block shape. It is separate from and must
/// never be mixed with the deterministic `probes` block (Q4 decision).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdvisoryProbes {
    /// The probe results, in evaluation order.
    pub results: Vec<AdvisoryProbeResult>,
}

impl AdvisoryProbes {
    /// An empty advisory probes block — means "no advisory signals ran."
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether any probe flagged an issue.
    pub fn any_flagged(&self) -> bool {
        self.results.iter().any(|r| r.flagged)
    }

    /// Validate all results.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let errors: Vec<String> = self
            .results
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.validate().err().map(|e| format!("probe[{i}]: {e}")))
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fray::SnapReason;
    use crate::ids::{ManifestId, StrandId, ThreadId, WriterId};
    use crate::strand::HashAlgo;
    use crate::thread::TensionState;
    use time::OffsetDateTime;

    fn hash_thread(surface: &str, hash: [u8; 32]) -> Thread {
        Thread {
            id: ThreadId::new(),
            surface: SurfaceId::new(surface),
            writer: WriterId::new("principal:val"),
            strands: vec![
                Strand::ContentHash {
                    id: StrandId::new(),
                    algorithm: HashAlgo::Blake3,
                    value: hash.to_vec(),
                },
                Strand::ManifestEntry {
                    id: StrandId::new(),
                    manifest_id: ManifestId::new(),
                    entry_hash: hash.to_vec(),
                },
                Strand::SerializationMarker {
                    id: StrandId::new(),
                    format_version: "0.1.0".into(),
                    contract_hash: vec![0xcc; 32],
                },
            ],
            holds_under: vec![
                Channel::Forced,
                Channel::Serialization,
                Channel::Mutation,
            ],
            created_at: OffsetDateTime::UNIX_EPOCH,
            tension: TensionState::Holds,
        }
    }

    // ---- FamiliarNameInvariant ----

    #[test]
    fn familiar_name_invariant_coherent_when_hash_matches() {
        let expected = [0xab; 32];
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: expected,
            description: "name must not change".into(),
        };
        let threads = vec![hash_thread("IDENTITY.md", expected)];
        assert_eq!(inv.coherent(&threads), WeaveCoherence::Coherent);
    }

    #[test]
    fn familiar_name_invariant_broken_on_hash_mismatch() {
        let expected = [0xab; 32];
        let actual = [0xcd; 32];
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: expected,
            description: "name must not change".into(),
        };
        let threads = vec![hash_thread("IDENTITY.md", actual)];
        match inv.coherent(&threads) {
            WeaveCoherence::Broken { reason } => {
                assert!(reason.contains("mismatch"), "{reason}");
            }
            other => panic!("expected Broken on hash mismatch, got {other:?}"),
        }
    }

    #[test]
    fn familiar_name_invariant_broken_when_no_thread() {
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: [0xab; 32],
            description: "name must not change".into(),
        };
        match inv.coherent(&[]) {
            WeaveCoherence::Broken { reason } => {
                assert!(reason.contains("no thread"), "{reason}");
            }
            other => panic!("expected Broken with no thread, got {other:?}"),
        }
    }

    #[test]
    fn familiar_name_invariant_broken_when_thread_snapped() {
        // decision 4: fail-closed — a snapped thread is not valid evidence.
        let expected = [0xab; 32];
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: expected,
            description: "name must not change".into(),
        };
        let mut thread = hash_thread("IDENTITY.md", expected);
        thread.snap(Channel::Mutation, SnapReason::Revoked, OffsetDateTime::now_utc());
        // After snap, holds_under(Mutation) fails → thread is skipped → no
        // valid evidence → Broken.
        match inv.coherent(&[thread]) {
            WeaveCoherence::Broken { .. } => {}
            other => panic!("expected Broken with snapped thread, got {other:?}"),
        }
    }

    #[test]
    fn familiar_name_invariant_describe_is_derived() {
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: [0x00; 32],
            description: "test".into(),
        };
        let d = inv.describe();
        assert!(d.protected_surfaces.contains(&SurfaceId::new("IDENTITY.md")));
        assert!(d.channels_required.contains(&Channel::Mutation));
        assert!(d
            .strand_requirements
            .iter()
            .any(|r| r.kind == StrandKind::ContentHash));
        // Descriptor never enforced on — just round-trip it.
        let json = serde_json::to_string(&d).unwrap();
        let back: PatternDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    // ---- ManifestAnchoredInvariant ----

    #[test]
    fn manifest_anchored_invariant_coherent_when_entry_hash_matches() {
        let expected = [0x11; 32];
        let inv = ManifestAnchoredInvariant {
            surface: SurfaceId::new("SOUL.md"),
            expected_entry_hash: expected,
            description: "soul surface must remain anchored".into(),
        };
        let threads = vec![hash_thread("SOUL.md", expected)];
        assert_eq!(inv.coherent(&threads), WeaveCoherence::Coherent);
    }

    #[test]
    fn manifest_anchored_invariant_broken_on_mismatch() {
        let inv = ManifestAnchoredInvariant {
            surface: SurfaceId::new("SOUL.md"),
            expected_entry_hash: [0x11; 32],
            description: "soul surface must remain anchored".into(),
        };
        let threads = vec![hash_thread("SOUL.md", [0x22; 32])];
        match inv.coherent(&threads) {
            WeaveCoherence::Broken { reason } => {
                assert!(reason.contains("mismatch"), "{reason}");
            }
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    // ---- CompositeIdentityInvariant ----

    #[test]
    fn composite_coherent_when_all_components_coherent() {
        let hash = [0xaa; 32];
        let composite = CompositeIdentityInvariant {
            name: "full-identity".into(),
            components: vec![
                Box::new(FamiliarNameInvariant {
                    surface: SurfaceId::new("IDENTITY.md"),
                    expected_hash: hash,
                    description: "name".into(),
                }),
                Box::new(ManifestAnchoredInvariant {
                    surface: SurfaceId::new("SOUL.md"),
                    expected_entry_hash: hash,
                    description: "soul".into(),
                }),
            ],
        };
        let threads = vec![
            hash_thread("IDENTITY.md", hash),
            hash_thread("SOUL.md", hash),
        ];
        assert_eq!(composite.coherent(&threads), WeaveCoherence::Coherent);
    }

    #[test]
    fn composite_broken_when_any_component_broken() {
        let hash = [0xaa; 32];
        let composite = CompositeIdentityInvariant {
            name: "full-identity".into(),
            components: vec![
                Box::new(FamiliarNameInvariant {
                    surface: SurfaceId::new("IDENTITY.md"),
                    expected_hash: hash,
                    description: "name".into(),
                }),
                Box::new(ManifestAnchoredInvariant {
                    surface: SurfaceId::new("SOUL.md"),
                    expected_entry_hash: hash,
                    description: "soul".into(),
                }),
            ],
        };
        // Only IDENTITY.md thread present, SOUL.md missing → composite Broken.
        let threads = vec![hash_thread("IDENTITY.md", hash)];
        match composite.coherent(&threads) {
            WeaveCoherence::Broken { .. } => {}
            other => panic!("expected Broken when component missing, got {other:?}"),
        }
    }

    #[test]
    fn composite_describe_merges_all_components() {
        let hash = [0x00; 32];
        let composite = CompositeIdentityInvariant {
            name: "full-identity".into(),
            components: vec![
                Box::new(FamiliarNameInvariant {
                    surface: SurfaceId::new("IDENTITY.md"),
                    expected_hash: hash,
                    description: "name".into(),
                }),
                Box::new(ManifestAnchoredInvariant {
                    surface: SurfaceId::new("SOUL.md"),
                    expected_entry_hash: hash,
                    description: "soul".into(),
                }),
            ],
        };
        let d = composite.describe();
        assert!(d.protected_surfaces.contains(&SurfaceId::new("IDENTITY.md")));
        assert!(d.protected_surfaces.contains(&SurfaceId::new("SOUL.md")));
        assert!(d.channels_required.contains(&Channel::Mutation));
        assert!(d.channels_required.contains(&Channel::Forced));
    }

    // ---- AdvisoryProbeResult / AdvisoryProbes ----

    #[test]
    fn advisory_probe_result_is_never_authoritative() {
        // decision 4: advisory probes are Gate-3 supplementary evidence only.
        let probe = AdvisoryProbeResult::new(
            "semantic_drift",
            "Semantic drift detector",
            Some(0.82),
            Some("name field appears changed".into()),
            true,
        );
        assert!(!probe.is_authoritative, "advisory probes must never be authoritative");
        assert!(probe.validate().is_ok());
    }

    #[test]
    fn advisory_probe_rejects_out_of_range_confidence() {
        let mut probe = AdvisoryProbeResult::new("test", "test", Some(1.5), None, false);
        assert!(probe.validate().is_err());
        probe.confidence = Some(-0.1);
        assert!(probe.validate().is_err());
        probe.confidence = Some(1.0);
        assert!(probe.validate().is_ok());
    }

    #[test]
    fn advisory_probe_rejects_authoritative_flag() {
        let probe = AdvisoryProbeResult {
            kind: "test".into(),
            label: "test".into(),
            confidence: None,
            signal: None,
            flagged: false,
            is_authoritative: true, // wire-received violation
        };
        assert!(probe.validate().is_err());
    }

    #[test]
    fn advisory_probes_block_empty_means_no_signals_ran() {
        // Q4 decision: empty advisory_probes != no probes at all.
        // It means "no advisory signals ran."
        let block = AdvisoryProbes::empty();
        assert!(block.results.is_empty());
        assert!(!block.any_flagged());
        assert!(block.validate().is_ok());
    }

    #[test]
    fn advisory_probes_block_any_flagged() {
        let mut block = AdvisoryProbes::empty();
        block.results.push(AdvisoryProbeResult::new(
            "confidence_score",
            "Low confidence",
            Some(0.3),
            None,
            true,
        ));
        assert!(block.any_flagged());
        assert!(block.validate().is_ok());
    }

    #[test]
    fn advisory_probes_roundtrips_json() {
        let block = AdvisoryProbes {
            results: vec![
                AdvisoryProbeResult::new("semantic_drift", "Drift", Some(0.75), Some("shifted".into()), false),
                AdvisoryProbeResult::new("pattern_match", "Pattern", None, None, true),
            ],
        };
        let json = serde_json::to_string_pretty(&block).unwrap();
        let back: AdvisoryProbes = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
        // Confirm is_authoritative is never true after round-trip.
        for r in &back.results {
            assert!(!r.is_authoritative);
        }
    }
}
