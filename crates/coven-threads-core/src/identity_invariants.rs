//! Identity invariant compilation and predicate implementations (`threads-uqx.4`).
//!
//! [`IdentityInvariantSet`] compiles the retired Ward declaration strings into
//! typed, deterministic checks over candidate-harness identity facts. The
//! lower-level surface commitment predicates remain available as supporting
//! evidence, but they are not substitutes for semantic identity evaluation.
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

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::channel::Channel;
use crate::ids::SurfaceId;
use crate::pattern::{PatternDescriptor, PatternPredicate, StrandRequirement, WeaveCoherence};
use crate::strand::{Strand, StrandKind};
use crate::thread::Thread;

// ---------------------------------------------------------------------------
// Retired declaration compiler and candidate identity facts
// ---------------------------------------------------------------------------

/// Identity fact named by the retired Ward v0.1 invariant declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityFact {
    /// `familiar.name`.
    Name,
    /// `familiar.person`.
    Person,
    /// `familiar.pronouns`.
    Pronouns,
    /// `familiar.purpose`.
    Purpose,
    /// `familiar.coven`.
    Coven,
}

impl IdentityFact {
    fn from_declaration_name(value: &str) -> Option<Self> {
        match value {
            "familiar.name" => Some(Self::Name),
            "familiar.person" => Some(Self::Person),
            "familiar.pronouns" => Some(Self::Pronouns),
            "familiar.purpose" => Some(Self::Purpose),
            "familiar.coven" => Some(Self::Coven),
            _ => None,
        }
    }
}

/// Comparison operator supported by retired Ward v0.1 declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityInvariantOperator {
    /// Candidate value must equal the declared value byte-for-byte.
    Equals,
    /// Candidate value must contain the declared value byte-for-byte.
    Includes,
}

/// One compiled identity invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityInvariantDeclaration {
    /// Identity fact being constrained.
    pub fact: IdentityFact,
    /// Deterministic comparison operation.
    pub operator: IdentityInvariantOperator,
    /// Principal-declared expected value.
    pub expected: String,
}

/// One identity fact deterministically extracted from the complete candidate
/// familiar harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateIdentityFact {
    /// Extracted fact.
    pub fact: IdentityFact,
    /// Extracted candidate value.
    pub value: String,
}

/// Deterministic identity facts for a complete candidate familiar harness.
///
/// The daemon must derive these facts from the materialized candidate, not
/// infer them from the path that changed. Missing or ambiguous extraction must
/// be represented by omitting the fact; evaluation then fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateIdentityFacts {
    /// Commitment tying this extraction to the complete materialized candidate.
    pub candidate_commitment: [u8; 32],
    values: Vec<CandidateIdentityFact>,
}

/// Gate-4 identity evidence bound to the daemon's complete candidate harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateIdentityContext {
    /// Daemon-computed commitment over the complete materialized harness.
    pub candidate_commitment: [u8; 32],
    /// Deterministically extracted facts carrying their own source commitment.
    pub facts: CandidateIdentityFacts,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateIdentityFactsWire {
    candidate_commitment: [u8; 32],
    values: Vec<CandidateIdentityFact>,
}

impl CandidateIdentityFacts {
    /// Construct a validated candidate fact set.
    pub fn try_new(
        candidate_commitment: [u8; 32],
        mut values: Vec<CandidateIdentityFact>,
    ) -> Result<Self, String> {
        values.sort_by_key(|entry| entry.fact);
        let mut seen = BTreeSet::new();
        for entry in &values {
            if entry.value.trim().is_empty() {
                return Err(format!("{:?} identity fact must not be empty", entry.fact));
            }
            if !seen.insert(entry.fact) {
                return Err(format!("duplicate {:?} identity fact", entry.fact));
            }
        }
        Ok(Self {
            candidate_commitment,
            values,
        })
    }

    /// Return the extracted value for a fact.
    pub fn get(&self, fact: IdentityFact) -> Option<&str> {
        self.values
            .iter()
            .find(|entry| entry.fact == fact)
            .map(|entry| entry.value.as_str())
    }

    /// Borrow the canonical fact list.
    pub fn values(&self) -> &[CandidateIdentityFact] {
        &self.values
    }
}

impl<'de> Deserialize<'de> for CandidateIdentityFacts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CandidateIdentityFactsWire::deserialize(deserializer)?;
        Self::try_new(wire.candidate_commitment, wire.values).map_err(serde::de::Error::custom)
    }
}

/// Validated set of compiled identity invariants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct IdentityInvariantSet {
    declarations: Vec<IdentityInvariantDeclaration>,
}

impl IdentityInvariantSet {
    /// Compile retired Ward declaration strings such as
    /// `familiar.name == 'Example'` and `familiar.purpose includes 'review'`.
    ///
    /// `familiar.name` and `familiar.person` are mandatory. Unknown fields,
    /// unknown operators, duplicate declarations, and empty values fail closed.
    pub fn compile<I, S>(declarations: I) -> Result<Self, Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut compiled = Vec::new();
        let mut errors = Vec::new();
        for (index, declaration) in declarations.into_iter().enumerate() {
            match parse_identity_declaration(declaration.as_ref()) {
                Ok(declaration) => compiled.push(declaration),
                Err(error) => errors.push(format!("invariant[{index}]: {error}")),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Self::try_new(compiled).map_err(|error| vec![error])
    }

    /// Construct from already-parsed declarations and validate the complete set.
    pub fn try_new(mut declarations: Vec<IdentityInvariantDeclaration>) -> Result<Self, String> {
        declarations.sort_by_key(|declaration| declaration.fact);
        let mut seen = BTreeSet::new();
        for declaration in &declarations {
            if declaration.expected.is_empty() {
                return Err(format!(
                    "{:?} identity invariant expected value must not be empty",
                    declaration.fact
                ));
            }
            if !seen.insert(declaration.fact) {
                return Err(format!(
                    "duplicate {:?} identity invariant declaration",
                    declaration.fact
                ));
            }
        }
        for mandatory in [IdentityFact::Name, IdentityFact::Person] {
            if !seen.contains(&mandatory) {
                return Err(format!(
                    "missing mandatory {:?} identity invariant declaration",
                    mandatory
                ));
            }
        }
        Ok(Self { declarations })
    }

    /// Evaluate candidate-harness facts without consulting the changed path.
    ///
    /// Callers must run this for every candidate capable of changing familiar
    /// behaviour. A missing extraction, including an omitted optional declared
    /// fact, is a deterministic failure rather than an advisory result.
    pub fn evaluate(
        &self,
        expected_candidate_commitment: [u8; 32],
        candidate: Option<&CandidateIdentityFacts>,
    ) -> WeaveCoherence {
        let Some(candidate) = candidate else {
            return WeaveCoherence::Broken {
                reason: "candidate identity facts unavailable — fail closed".into(),
            };
        };
        if candidate.candidate_commitment != expected_candidate_commitment {
            return WeaveCoherence::Broken {
                reason: "candidate identity facts do not match the materialized harness".into(),
            };
        }

        let mut failures = Vec::new();
        for declaration in &self.declarations {
            let Some(actual) = candidate.get(declaration.fact) else {
                failures.push(format!(
                    "{:?} identity fact unavailable or ambiguous",
                    declaration.fact
                ));
                continue;
            };
            let holds = match declaration.operator {
                IdentityInvariantOperator::Equals => actual == declaration.expected,
                IdentityInvariantOperator::Includes => actual.contains(&declaration.expected),
            };
            if !holds {
                failures.push(format!(
                    "{:?} identity invariant did not hold",
                    declaration.fact
                ));
            }
        }

        if failures.is_empty() {
            WeaveCoherence::Coherent
        } else {
            WeaveCoherence::Broken {
                reason: failures.join("; "),
            }
        }
    }

    /// Borrow the canonical declaration list.
    pub fn declarations(&self) -> &[IdentityInvariantDeclaration] {
        &self.declarations
    }
}

impl<'de> Deserialize<'de> for IdentityInvariantSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let declarations = Vec::<IdentityInvariantDeclaration>::deserialize(deserializer)?;
        Self::try_new(declarations).map_err(serde::de::Error::custom)
    }
}

/// Authoritative pattern wrapper that joins structural thread coherence with
/// complete-candidate semantic identity invariants.
pub struct IdentityAwarePattern {
    /// Structural authority predicate evaluated first.
    pub structural: Box<dyn PatternPredicate + Send + Sync>,
    /// Semantic identity declarations evaluated for every Gate-4 candidate.
    pub invariants: IdentityInvariantSet,
}

impl std::fmt::Debug for IdentityAwarePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityAwarePattern")
            .field("structural", &self.structural)
            .field("invariants", &self.invariants)
            .finish()
    }
}

impl PatternPredicate for IdentityAwarePattern {
    fn coherent(&self, _threads: &[Thread]) -> WeaveCoherence {
        WeaveCoherence::Broken {
            reason: "identity-aware pattern requires complete candidate evidence".into(),
        }
    }

    fn coherent_with_context(
        &self,
        threads: &[Thread],
        identity: Option<&CandidateIdentityContext>,
    ) -> WeaveCoherence {
        let structural = self.structural.coherent_with_context(threads, identity);
        if matches!(structural, WeaveCoherence::Broken { .. }) {
            return structural;
        }
        let identity = self.invariants.evaluate(
            identity.map_or([0; 32], |context| context.candidate_commitment),
            identity.map(|context| &context.facts),
        );
        match identity {
            WeaveCoherence::Coherent => structural,
            broken => broken,
        }
    }

    fn describe(&self) -> PatternDescriptor {
        let mut descriptor = self.structural.describe();
        descriptor.name = format!("identity-aware({})", descriptor.name);
        descriptor
    }
}

fn parse_identity_declaration(value: &str) -> Result<IdentityInvariantDeclaration, String> {
    let value = value.trim();
    let equals = value.find(" == ");
    let includes = value.find(" includes ");
    let (left, operator, right) = match (equals, includes) {
        (Some(equals), Some(includes)) if equals < includes => (
            &value[..equals],
            IdentityInvariantOperator::Equals,
            &value[equals + 4..],
        ),
        (Some(_), Some(includes)) | (None, Some(includes)) => (
            &value[..includes],
            IdentityInvariantOperator::Includes,
            &value[includes + 10..],
        ),
        (Some(equals), None) => (
            &value[..equals],
            IdentityInvariantOperator::Equals,
            &value[equals + 4..],
        ),
        (None, None) => return Err("expected `==` or `includes` operator".into()),
    };
    let fact = IdentityFact::from_declaration_name(left.trim())
        .ok_or_else(|| format!("unsupported identity fact {:?}", left.trim()))?;
    let right = right.trim();
    if right.is_empty() {
        return Err("expected value must not be empty".into());
    }
    let expected = if right.starts_with('"') {
        serde_json::from_str::<String>(right)
            .map_err(|error| format!("invalid quoted expected value: {error}"))?
    } else if right.starts_with('\'') {
        if right.len() < 2 || !right.ends_with('\'') {
            return Err("invalid single-quoted expected value".into());
        }
        right[1..right.len() - 1].to_string()
    } else {
        right.to_string()
    };
    if expected.is_empty() {
        return Err("expected value must not be empty".into());
    }
    Ok(IdentityInvariantDeclaration {
        fact,
        operator,
        expected,
    })
}

// ---------------------------------------------------------------------------
// FamiliarNameInvariant
// ---------------------------------------------------------------------------

/// Coarse commitment to an identity surface's complete content.
///
/// ## How it works
///
/// At weave construction time, the principal records the `expected_hash` of the
/// identity surface (e.g. `IDENTITY.md`) using
/// [`crate::strand::HashAlgo::Blake3`]. At gate
/// time the daemon extracts the `ContentHash` strand from the thread on that
/// surface and compares byte-for-byte. Mismatch → fail closed.
///
/// This is not a semantic implementation of `familiar.name`: edits to other
/// execution surfaces can alter identity behaviour while this hash remains
/// unchanged. Use [`IdentityInvariantSet`] over complete candidate-harness facts
/// for RFC-0001 identity enforcement; this predicate is supporting evidence.
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

        if surface_threads.len() != 1 {
            return WeaveCoherence::Broken {
                reason: format!(
                    "FamiliarNameInvariant: expected exactly one authority thread for {:?}, \
                     found {} — fail closed",
                    self.surface,
                    surface_threads.len()
                ),
            };
        }
        let thread = surface_threads[0];
        for channel in [Channel::Mutation, Channel::Serialization] {
            if let Err(error) = thread.holds_under(channel) {
                return WeaveCoherence::Broken {
                    reason: format!(
                        "FamiliarNameInvariant: thread for {:?} does not hold under \
                         {channel:?}: {error:?}",
                        self.surface
                    ),
                };
            }
        }
        let hashes: Vec<&[u8]> = thread
            .strands
            .iter()
            .filter_map(|strand| match strand {
                Strand::ContentHash { value, .. } => Some(value.as_slice()),
                _ => None,
            })
            .collect();
        if hashes.len() != 1 {
            return WeaveCoherence::Broken {
                reason: format!(
                    "FamiliarNameInvariant: expected exactly one ContentHash strand for {:?}, \
                     found {} — fail closed",
                    self.surface,
                    hashes.len()
                ),
            };
        }
        if hashes[0] != self.expected_hash {
            return WeaveCoherence::Broken {
                reason: format!(
                    "FamiliarNameInvariant: content hash mismatch on {:?} — \
                     expected {:?}, found {:?}. Identity surface mutation \
                     requires human review.",
                    self.surface,
                    &self.expected_hash[..4],
                    &hashes[0][..4.min(hashes[0].len())],
                ),
            };
        }
        WeaveCoherence::Coherent
    }

    fn describe(&self) -> PatternDescriptor {
        PatternDescriptor {
            name: format!("familiar-name-invariant({})", self.surface.as_str()),
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

        if surface_threads.len() != 1 {
            return WeaveCoherence::Broken {
                reason: format!(
                    "ManifestAnchoredInvariant: expected exactly one authority thread for {:?}, \
                     found {} — fail closed",
                    self.surface,
                    surface_threads.len()
                ),
            };
        }
        let thread = surface_threads[0];
        for channel in [Channel::Forced, Channel::Serialization] {
            if let Err(error) = thread.holds_under(channel) {
                return WeaveCoherence::Broken {
                    reason: format!(
                        "ManifestAnchoredInvariant: thread for {:?} does not hold under \
                         {channel:?}: {error:?}",
                        self.surface
                    ),
                };
            }
        }
        let entries: Vec<&[u8]> = thread
            .strands
            .iter()
            .filter_map(|strand| match strand {
                Strand::ManifestEntry { entry_hash, .. } => Some(entry_hash.as_slice()),
                _ => None,
            })
            .collect();
        if entries.len() != 1 {
            return WeaveCoherence::Broken {
                reason: format!(
                    "ManifestAnchoredInvariant: expected exactly one ManifestEntry strand for \
                     {:?}, found {} — fail closed",
                    self.surface,
                    entries.len()
                ),
            };
        }
        if entries[0] != self.expected_entry_hash {
            return WeaveCoherence::Broken {
                reason: format!(
                    "ManifestAnchoredInvariant: manifest entry hash mismatch on {:?} — \
                     pinned {:?}, found {:?}. Requires principal review.",
                    self.surface,
                    &self.expected_entry_hash[..4],
                    &entries[0][..4.min(entries[0].len())],
                ),
            };
        }
        WeaveCoherence::Coherent
    }

    fn describe(&self) -> PatternDescriptor {
        PatternDescriptor {
            name: format!("manifest-anchored-invariant({})", self.surface.as_str()),
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
        let mut saw_degraded = false;

        for component in &self.components {
            match component.coherent(threads) {
                WeaveCoherence::Coherent => {}
                WeaveCoherence::Degraded {
                    degraded_surfaces: mut s,
                    reason,
                } => {
                    saw_degraded = true;
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
        } else if saw_degraded {
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
#[serde(try_from = "AdvisoryProbeResultWire")]
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
    pub is_authoritative: bool,
}

#[derive(Deserialize)]
struct AdvisoryProbeResultWire {
    kind: String,
    label: String,
    confidence: Option<f64>,
    signal: Option<String>,
    flagged: bool,
    #[serde(default)]
    is_authoritative: bool,
}

impl TryFrom<AdvisoryProbeResultWire> for AdvisoryProbeResult {
    type Error = String;

    fn try_from(wire: AdvisoryProbeResultWire) -> Result<Self, Self::Error> {
        let result = Self {
            kind: wire.kind,
            label: wire.label,
            confidence: wire.confidence,
            signal: wire.signal,
            flagged: wire.flagged,
            is_authoritative: wire.is_authoritative,
        };
        result.validate()?;
        Ok(result)
    }
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
    /// - is_authoritative is false (enforced at construction and again at the
    ///   deserialize boundary — ill-formed wire values never materialize)
    pub fn validate(&self) -> Result<(), String> {
        if self.is_authoritative {
            return Err("AdvisoryProbeResult.is_authoritative must be false — \
                 advisory probes are never authoritative (decision 4)"
                .into());
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
            holds_under: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
            created_at: OffsetDateTime::UNIX_EPOCH,
            tension: TensionState::Holds,
        }
    }

    fn full_invariant_set() -> IdentityInvariantSet {
        IdentityInvariantSet::compile([
            r#"familiar.name == "Nova""#,
            r#"familiar.person == "Val""#,
            r#"familiar.pronouns == "she/her""#,
            r#"familiar.purpose includes "authority""#,
            r#"familiar.coven == "OpenCoven""#,
        ])
        .unwrap()
    }

    fn full_candidate_facts() -> CandidateIdentityFacts {
        CandidateIdentityFacts::try_new(
            [0x42; 32],
            vec![
                CandidateIdentityFact {
                    fact: IdentityFact::Name,
                    value: "Nova".into(),
                },
                CandidateIdentityFact {
                    fact: IdentityFact::Person,
                    value: "Val".into(),
                },
                CandidateIdentityFact {
                    fact: IdentityFact::Pronouns,
                    value: "she/her".into(),
                },
                CandidateIdentityFact {
                    fact: IdentityFact::Purpose,
                    value: "protect the authority boundary".into(),
                },
                CandidateIdentityFact {
                    fact: IdentityFact::Coven,
                    value: "OpenCoven".into(),
                },
            ],
        )
        .unwrap()
    }

    #[derive(Debug)]
    struct EmptyDegradedPredicate;

    impl PatternPredicate for EmptyDegradedPredicate {
        fn coherent(&self, _threads: &[Thread]) -> WeaveCoherence {
            WeaveCoherence::Degraded {
                degraded_surfaces: vec![],
                reason: "ambiguous degradation".into(),
            }
        }

        fn describe(&self) -> PatternDescriptor {
            PatternDescriptor {
                name: "empty-degraded".into(),
                protected_surfaces: vec![],
                channels_required: vec![],
                strand_requirements: vec![],
            }
        }
    }

    #[test]
    fn retired_invariant_declarations_compile_with_full_fidelity() {
        let invariants = full_invariant_set();
        assert_eq!(invariants.declarations().len(), 5);
        assert_eq!(
            invariants.declarations()[0].fact,
            IdentityFact::Name,
            "declarations are canonicalized"
        );
        assert!(matches!(
            invariants.evaluate([0x42; 32], Some(&full_candidate_facts())),
            WeaveCoherence::Coherent
        ));

        let encoded = serde_json::to_string(&invariants).unwrap();
        let decoded: IdentityInvariantSet = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, invariants);
    }

    #[test]
    fn normative_single_quoted_declarations_compile_and_evaluate() {
        let invariants = IdentityInvariantSet::compile([
            "familiar.name == 'Nova'",
            "familiar.person == 'Val'",
            "familiar.pronouns == 'she/her'",
            "familiar.purpose includes 'authority'",
            "familiar.coven == 'OpenCoven'",
        ])
        .unwrap();
        assert!(matches!(
            invariants.evaluate([0x42; 32], Some(&full_candidate_facts())),
            WeaveCoherence::Coherent
        ));
    }

    #[test]
    fn compiler_requires_name_and_person() {
        let errors = IdentityInvariantSet::compile([
            r#"familiar.pronouns == "she/her""#,
            r#"familiar.purpose includes "authority""#,
        ])
        .unwrap_err();
        assert!(errors.iter().any(|error| error.contains("Name")));
    }

    #[test]
    fn compiler_uses_the_first_declared_operator_not_rhs_text() {
        let invariants = IdentityInvariantSet::compile([
            r#"familiar.name == "Nova includes review""#,
            r#"familiar.person == "Val""#,
        ])
        .unwrap();
        assert_eq!(
            invariants.declarations()[0].expected,
            "Nova includes review"
        );
    }

    #[test]
    fn candidate_fact_drift_fails_even_without_a_path_specific_check() {
        let invariants = full_invariant_set();
        let mut facts = full_candidate_facts();
        facts
            .values
            .iter_mut()
            .find(|entry| entry.fact == IdentityFact::Person)
            .unwrap()
            .value = "someone else".into();

        assert!(matches!(
            invariants.evaluate([0x42; 32], Some(&facts)),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn missing_candidate_extraction_fails_closed() {
        assert!(matches!(
            full_invariant_set().evaluate([0x42; 32], None),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn stale_candidate_fact_extraction_fails_closed() {
        assert!(matches!(
            full_invariant_set().evaluate([0x99; 32], Some(&full_candidate_facts())),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn candidate_fact_deserialization_rejects_duplicates() {
        let json = format!(
            r#"{{"candidate_commitment":[{}],"values":[
                {{"fact":"name","value":"Nova"}},
                {{"fact":"name","value":"Other"}}
            ]}}"#,
            vec!["0"; 32].join(",")
        );
        assert!(serde_json::from_str::<CandidateIdentityFacts>(&json).is_err());
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
        thread.snap(
            Channel::Mutation,
            SnapReason::Revoked,
            OffsetDateTime::now_utc(),
        );
        // After snap, holds_under(Mutation) fails → thread is skipped → no
        // valid evidence → Broken.
        match inv.coherent(&[thread]) {
            WeaveCoherence::Broken { .. } => {}
            other => panic!("expected Broken with snapped thread, got {other:?}"),
        }
    }

    #[test]
    fn familiar_name_invariant_requires_serialization_survival() {
        let expected = [0xab; 32];
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: expected,
            description: "name must survive serialization".into(),
        };
        let mut thread = hash_thread("IDENTITY.md", expected);
        thread
            .strands
            .retain(|strand| !matches!(strand, Strand::SerializationMarker { .. }));

        assert!(matches!(
            inv.coherent(&[thread]),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn familiar_name_invariant_rejects_conflicting_threads() {
        let expected = [0xab; 32];
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: expected,
            description: "name must not conflict".into(),
        };
        let mut conflicting = hash_thread("IDENTITY.md", [0xcd; 32]);
        conflicting.writer = WriterId::new("principal:other");

        assert!(matches!(
            inv.coherent(&[hash_thread("IDENTITY.md", expected), conflicting]),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn familiar_name_invariant_describe_is_derived() {
        let inv = FamiliarNameInvariant {
            surface: SurfaceId::new("IDENTITY.md"),
            expected_hash: [0x00; 32],
            description: "test".into(),
        };
        let d = inv.describe();
        assert!(d
            .protected_surfaces
            .contains(&SurfaceId::new("IDENTITY.md")));
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

    #[test]
    fn manifest_anchored_invariant_requires_serialization_survival() {
        let expected = [0x11; 32];
        let inv = ManifestAnchoredInvariant {
            surface: SurfaceId::new("SOUL.md"),
            expected_entry_hash: expected,
            description: "manifest must survive serialization".into(),
        };
        let mut thread = hash_thread("SOUL.md", expected);
        thread
            .strands
            .retain(|strand| !matches!(strand, Strand::SerializationMarker { .. }));

        assert!(matches!(
            inv.coherent(&[thread]),
            WeaveCoherence::Broken { .. }
        ));
    }

    #[test]
    fn manifest_anchored_invariant_rejects_conflicting_threads() {
        let expected = [0x11; 32];
        let inv = ManifestAnchoredInvariant {
            surface: SurfaceId::new("SOUL.md"),
            expected_entry_hash: expected,
            description: "manifest must not conflict".into(),
        };
        let mut conflicting = hash_thread("SOUL.md", [0x22; 32]);
        conflicting.writer = WriterId::new("principal:other");

        assert!(matches!(
            inv.coherent(&[hash_thread("SOUL.md", expected), conflicting]),
            WeaveCoherence::Broken { .. }
        ));
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
    fn composite_preserves_degraded_result_with_no_surfaces() {
        let composite = CompositeIdentityInvariant {
            name: "degraded".into(),
            components: vec![Box::new(EmptyDegradedPredicate)],
        };
        assert!(matches!(
            composite.coherent(&[]),
            WeaveCoherence::Degraded { .. }
        ));
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
        assert!(d
            .protected_surfaces
            .contains(&SurfaceId::new("IDENTITY.md")));
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
        assert!(
            !probe.is_authoritative,
            "advisory probes must never be authoritative"
        );
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
    fn advisory_probe_result_roundtrips_json() {
        let probe = AdvisoryProbeResult::new(
            "semantic_drift",
            "Semantic drift detector",
            Some(0.82),
            Some("name field appears changed".into()),
            true,
        );
        let json = serde_json::to_string(&probe).unwrap();
        let back: AdvisoryProbeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(probe, back);
    }

    #[test]
    fn advisory_probe_deserialization_defaults_is_authoritative_false() {
        let json =
            r#"{"kind":"test","label":"test","confidence":0.5,"signal":null,"flagged":true}"#;
        let probe: AdvisoryProbeResult = serde_json::from_str(json).unwrap();
        assert!(!probe.is_authoritative);
    }

    #[test]
    fn advisory_probe_deserialization_rejects_out_of_range_confidence() {
        // Wire values must not bypass validate() (decision 4 fail-closed).
        let json =
            r#"{"kind":"test","label":"test","confidence":9.9,"signal":null,"flagged":false}"#;
        assert!(serde_json::from_str::<AdvisoryProbeResult>(json).is_err());
    }

    #[test]
    fn advisory_probe_deserialization_rejects_authoritative_flag() {
        let json = r#"{"kind":"test","label":"test","confidence":null,"signal":null,
            "flagged":false,"is_authoritative":true}"#;
        assert!(serde_json::from_str::<AdvisoryProbeResult>(json).is_err());
    }

    #[test]
    fn advisory_probes_block_deserialization_rejects_ill_formed_result() {
        // Element-boundary validation covers the block shape too.
        let json = r#"{"results":[{"kind":"test","label":"test","confidence":null,
            "signal":null,"flagged":false,"is_authoritative":true}]}"#;
        assert!(serde_json::from_str::<AdvisoryProbes>(json).is_err());
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
                AdvisoryProbeResult::new(
                    "semantic_drift",
                    "Drift",
                    Some(0.75),
                    Some("shifted".into()),
                    false,
                ),
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
