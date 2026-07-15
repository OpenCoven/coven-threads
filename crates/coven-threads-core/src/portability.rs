//! Portability — the C7 round-trip contract (§3.3 invariant #4, Phase 3).
//!
//! **C7:** every thread bound to `Channel::Serialization` MUST carry a
//! `SerializationMarker` strand whose survival is a round-trip invariant.
//! Export followed by import produces a weave with equivalent tension state,
//! **or fails visibly**. Silent downgrade on import is the `.af` failure mode
//! this module exists to refuse (§12 verification log: `CoreMemoryBlockSchema`
//! has no protection field; runtime `read_only` stripped at export).
//!
//! The unit of exchange is [`PortableWeave`]: a versioned envelope around
//! [`WeaveRecord`] stamped with the [`SerializationContract`] hash it was
//! exported under. The reference encoding is canonical JSON via serde; the
//! Phase 3 interchange *format* decision (Shape A `.af`-superset vs Shape B
//! net-new, `specs/PHASE-3-PORTABILITY.md`) wraps this envelope — it does not
//! change these semantics.
//!
//! Fail-visibly matrix enforced here:
//! - exporting a `Serialization`-covered thread with no marker → error (the
//!   contract cannot be stamped on material that never carried it);
//! - importing an unsupported format version → error;
//! - importing an artifact whose envelope contract hash is unknown → error;
//! - importing a thread whose marker committed to a different contract →
//!   error naming the thread and surface;
//! - importing a record whose threads were altered after export →
//!   [`WeaveError::HashMismatch`] (weave-hash recomputation);
//! - a clean import yields a weave with *equivalent tension state* and
//!   *identical authority topology* — never wider (RFC-0001 §5.1: import is
//!   not a write path around the gates).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::channel::Channel;
use crate::ids::{SurfaceId, ThreadId};
use crate::pattern::PatternPredicate;
use crate::strand::{HashAlgo, Strand, StrandKind};
use crate::weave::{Weave, WeaveError, WeaveRecord};

/// The portability format version this crate exports and accepts.
pub const PORTABILITY_FORMAT_VERSION: &str = "0.1.0";

/// The serialization contract: what a `SerializationMarker` strand's
/// `contract_hash` commits to.
///
/// The contract enumerates, per strand kind, that the kind is representable in
/// the portable form and survives the transform. Hashing the canonical
/// encoding makes contract drift *visible*: a runtime with a different strand
/// vocabulary produces a different hash and imports fail loudly instead of
/// silently dropping fibers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializationContract {
    /// Format version this contract belongs to.
    pub format_version: String,
    /// Strand kinds this format round-trips, in canonical order.
    pub strand_kinds: Vec<StrandKind>,
}

impl SerializationContract {
    /// The current contract: all five frozen strand kinds (§2.3) round-trip.
    pub fn current() -> Self {
        Self {
            format_version: PORTABILITY_FORMAT_VERSION.to_string(),
            strand_kinds: vec![
                StrandKind::ContentHash,
                StrandKind::Signature,
                StrandKind::ManifestEntry,
                StrandKind::AuditTrail,
                StrandKind::SerializationMarker,
            ],
        }
    }

    /// BLAKE3 over the canonical encoding — the value `SerializationMarker`
    /// strands commit to.
    pub fn contract_hash(&self) -> Vec<u8> {
        let mut h = blake3::Hasher::new();
        h.update(b"coven-threads:serialization-contract:v1\n");
        h.update(self.format_version.as_bytes());
        h.update(b"\n");
        for kind in &self.strand_kinds {
            h.update(format!("{kind:?}\n").as_bytes());
        }
        h.finalize().as_bytes().to_vec()
    }

    /// A `SerializationMarker` strand committing to this contract.
    pub fn marker_strand(&self) -> Strand {
        Strand::SerializationMarker {
            id: crate::ids::StrandId::new(),
            format_version: self.format_version.clone(),
            contract_hash: self.contract_hash(),
        }
    }
}

/// The portable envelope: what leaves one runtime and enters another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableWeave {
    /// Format version of this artifact.
    pub format_version: String,
    /// Hash of the [`SerializationContract`] the artifact was exported under.
    pub contract_hash: Vec<u8>,
    /// The weave record (threads, tension, weave hash, derived descriptor).
    pub record: WeaveRecord,
    /// Optional protected-surface content payloads, keyed by typed surface.
    ///
    /// Legacy `.weave` artifacts without this field still import after envelope
    /// verification. When present, every payload is checked against the
    /// `ContentHash` strands for its surface after the weave itself verifies.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub surfaces: BTreeMap<SurfaceId, PortableSurfaceContent>,
}

/// A surface payload carried inside a Shape B `.weave` envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableSurfaceContent {
    /// Payload encoding. This crate currently exports and verifies `utf8`.
    pub encoding: String,
    /// The encoded content bytes as text for `utf8` payloads.
    pub data: String,
}

impl PortableSurfaceContent {
    /// Construct a UTF-8 surface payload.
    pub fn utf8<S: Into<String>>(data: S) -> Self {
        Self {
            encoding: "utf8".to_string(),
            data: data.into(),
        }
    }
}

/// Lossy one-way Letta `.af` export for handoff.
///
/// This type intentionally implements `Serialize` only. There is no `.af` import
/// path: Coven re-entry is via the original `.weave` artifact only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LossyAfExport {
    /// Letta-facing agent type marker.
    pub agent_type: String,
    /// Explicitly false: the protection layer does not survive `.af`.
    pub round_trippable: bool,
    /// Letta core-memory blocks, deterministically sorted by surface id.
    pub core_memory: Vec<AfCoreMemoryBlock>,
    /// Deterministic creation timestamp for this lossy artifact.
    pub created_at: String,
    /// Handoff description warning that this is not a Coven re-entry artifact.
    pub description: Option<String>,
    /// Minimal Letta embedding config for schema-compatible handoff.
    pub embedding_config: serde_json::Value,
    /// Minimal Letta LLM config for schema-compatible handoff.
    pub llm_config: serde_json::Value,
    /// Letta message-buffer setting.
    pub message_buffer_autoclear: bool,
    /// In-context message indices; empty for a content-only handoff.
    pub in_context_message_indices: Vec<usize>,
    /// Letta messages; empty for a content-only handoff.
    pub messages: Vec<serde_json::Value>,
    /// Letta metadata field.
    pub metadata_: Option<serde_json::Value>,
    /// Letta multi-agent group field.
    pub multi_agent_group: Option<serde_json::Value>,
    /// Deterministic agent name derived from the weave's familiar id.
    pub name: String,
    /// System prompt for the lossy handoff.
    pub system: String,
    /// Letta tags.
    pub tags: Vec<AfTag>,
    /// Letta tool environment variables; empty for a content-only handoff.
    pub tool_exec_environment_variables: Vec<serde_json::Value>,
    /// Letta tool rules; empty for a content-only handoff.
    pub tool_rules: Vec<serde_json::Value>,
    /// Letta tools; empty for a content-only handoff.
    pub tools: Vec<serde_json::Value>,
    /// Deterministic update timestamp for this lossy artifact.
    pub updated_at: String,
    /// Letta `.af` schema version marker.
    pub version: String,
    /// Coven warning metadata, not an import surface.
    pub x_coven_threads: AfNonRoundTrippableMarker,
}

/// Letta core-memory block in the lossy `.af` handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfCoreMemoryBlock {
    /// Deterministic creation timestamp for this memory block.
    pub created_at: String,
    /// Optional Letta block description.
    pub description: Option<String>,
    /// Letta template marker; false for concrete exported surfaces.
    pub is_template: bool,
    /// Surface id rendered as Letta's block label.
    pub label: String,
    /// Letta block character budget.
    pub limit: usize,
    /// Letta block metadata.
    pub metadata_: Option<serde_json::Value>,
    /// Optional Letta template name.
    pub template_name: Option<String>,
    /// Deterministic update timestamp for this memory block.
    pub updated_at: String,
    /// Surface content rendered as Letta's block value.
    pub value: String,
}

/// Letta tag object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfTag {
    /// Tag text.
    pub tag: String,
}

/// Explicit warning that `.af` is not a Coven round-trip format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfNonRoundTrippableMarker {
    /// Always true for this exporter.
    pub non_round_trippable: bool,
    /// Human-readable loss reason.
    pub loss_reason: String,
    /// Re-entry rule for operators and tools.
    pub reentry: String,
    /// Canonical source format.
    pub source_format: String,
    /// Source weave hash for operator correlation only; not an import verifier.
    pub source_weave_hash: Vec<u8>,
}

/// Why an export or import failed — every arm is a *visible* refusal (C7).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PortabilityError {
    /// A `Serialization`-covered thread carries no `SerializationMarker`
    /// strand: the survival contract cannot be stamped. Fail-closed at export
    /// — shipping the thread anyway would manufacture the silent-downgrade
    /// import this module refuses.
    #[error("thread {thread} on surface {surface} holds under Serialization but carries no SerializationMarker strand")]
    MarkerMissing {
        /// The offending thread.
        thread: ThreadId,
        /// Its surface.
        surface: SurfaceId,
    },

    /// The artifact's format version is not supported by this runtime.
    #[error(
        "unsupported portability format version {found:?}; this runtime supports {supported:?}"
    )]
    UnsupportedFormatVersion {
        /// Version carried by the artifact.
        found: String,
        /// Version this runtime supports.
        supported: String,
    },

    /// The artifact's envelope contract hash does not match this runtime's
    /// contract: the two runtimes disagree about what survives serialization.
    #[error(
        "serialization contract mismatch: artifact contract differs from this runtime's contract"
    )]
    ContractMismatch,

    /// A thread's `SerializationMarker` committed to a different contract
    /// than the artifact claims — the strand did not survive the transform
    /// with its commitment intact.
    #[error("thread {thread} on surface {surface}: SerializationMarker does not match the artifact contract")]
    MarkerContractMismatch {
        /// The offending thread.
        thread: ThreadId,
        /// Its surface.
        surface: SurfaceId,
    },

    /// The artifact is not decodable as a `PortableWeave`.
    #[error("artifact is not a valid PortableWeave: {detail}")]
    Malformed {
        /// Decoder diagnostic.
        detail: String,
    },

    /// Structural verification failed on import (weave-hash mismatch or
    /// duplicate `(surface, writer)` pair).
    #[error("weave verification failed on import: {0}")]
    Weave(#[from] WeaveError),

    /// A surface payload uses an encoding this runtime cannot verify.
    #[error("surface {surface} uses unsupported content encoding {encoding:?}")]
    UnsupportedSurfaceEncoding {
        /// The surface whose payload could not be decoded.
        surface: SurfaceId,
        /// The unsupported encoding string.
        encoding: String,
    },

    /// A surface payload was present but no thread `ContentHash` strand commits
    /// to that surface.
    #[error("surface {surface} content has no matching thread ContentHash strand")]
    SurfaceContentUnbound {
        /// The unbound surface.
        surface: SurfaceId,
    },

    /// A non-empty surfaces map omitted a content payload needed to verify a
    /// thread `ContentHash` strand.
    #[error("surface {surface} has a ContentHash strand but no content payload")]
    SurfaceContentMissing {
        /// The missing surface.
        surface: SurfaceId,
    },

    /// A surface payload does not match a thread's `ContentHash` strand.
    #[error("surface {surface} content hash mismatch for {algorithm:?}")]
    SurfaceContentHashMismatch {
        /// The surface whose content failed verification.
        surface: SurfaceId,
        /// Hash algorithm named by the strand.
        algorithm: HashAlgo,
        /// Hash bytes recorded in the strand.
        expected: Vec<u8>,
        /// Hash bytes computed from the payload.
        actual: Vec<u8>,
    },

    /// A Letta `.af` handoff requires actual surface content.
    #[error("cannot export lossy .af: no surface content is present in the .weave artifact")]
    AfExportMissingSurfaces,
}

/// Export a weave as a portable artifact.
///
/// Refuses (visibly) if any `Serialization`-covered thread lacks its marker
/// strand — C7 is enforced at the boundary in both directions.
pub fn export_weave(weave: &Weave) -> Result<PortableWeave, PortabilityError> {
    let contract = SerializationContract::current();
    for thread in weave.threads() {
        if thread.holds_under.contains(&Channel::Serialization) {
            let has_marker = thread
                .strands
                .iter()
                .any(|s| s.kind() == StrandKind::SerializationMarker);
            if !has_marker {
                return Err(PortabilityError::MarkerMissing {
                    thread: thread.id,
                    surface: thread.surface.clone(),
                });
            }
        }
    }
    Ok(PortableWeave {
        format_version: contract.format_version.clone(),
        contract_hash: contract.contract_hash(),
        record: weave.to_record(),
        surfaces: BTreeMap::new(),
    })
}

/// Export a weave with UTF-8 surface content payloads.
///
/// The resulting Shape B `.weave` envelope carries surface content keyed by
/// [`SurfaceId`]. The map is verified against the weave's `ContentHash` strands
/// before it leaves this runtime, so a mismatched payload fails visibly at export
/// instead of producing an invalid artifact.
pub fn export_weave_with_surfaces<I>(
    weave: &Weave,
    surfaces: I,
) -> Result<PortableWeave, PortabilityError>
where
    I: IntoIterator<Item = (SurfaceId, String)>,
{
    let mut portable = export_weave(weave)?;
    portable.surfaces = surfaces
        .into_iter()
        .map(|(surface, data)| (surface, PortableSurfaceContent::utf8(data)))
        .collect();
    verify_surface_content(weave.threads(), &portable.surfaces)?;
    Ok(portable)
}

/// Export a lossy, one-way Letta `.af` handoff.
///
/// The output is deliberately marked non-round-trippable. It contains surface
/// text for Letta's `core_memory`, but not the Coven protection layer; no `.af`
/// import path exists in this crate.
pub fn export_af(portable: &PortableWeave) -> Result<LossyAfExport, PortabilityError> {
    if portable.surfaces.is_empty() {
        return Err(PortabilityError::AfExportMissingSurfaces);
    }
    verify_surface_content(&portable.record.threads, &portable.surfaces)?;

    let timestamp = "1970-01-01T00:00:00Z";
    let core_memory = portable
        .surfaces
        .iter()
        .map(|(surface, content)| {
            if content.encoding != "utf8" {
                return Err(PortabilityError::UnsupportedSurfaceEncoding {
                    surface: surface.clone(),
                    encoding: content.encoding.clone(),
                });
            }
            Ok(AfCoreMemoryBlock {
                created_at: timestamp.to_string(),
                description: Some(
                    "Lossy Coven surface handoff; not a protection boundary".to_string(),
                ),
                is_template: false,
                label: surface.as_str().to_string(),
                limit: content.data.len(),
                metadata_: None,
                template_name: None,
                updated_at: timestamp.to_string(),
                value: content.data.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LossyAfExport {
        agent_type: "coven_familiar_lossy_handoff".to_string(),
        round_trippable: false,
        core_memory,
        created_at: timestamp.to_string(),
        description: Some(
            "Lossy one-way Letta .af handoff exported from Coven .weave; not round-trippable"
                .to_string(),
        ),
        embedding_config: json!({
            "embedding_endpoint_type": "openai",
            "embedding_endpoint": "https://api.openai.com/v1",
            "embedding_model": "text-embedding-3-small",
            "embedding_dim": 1536,
            "embedding_chunk_size": 300,
            "handle": null,
            "batch_size": 32,
            "azure_endpoint": null,
            "azure_version": null,
            "azure_deployment": null
        }),
        llm_config: json!({
            "model": "gpt-4o-mini",
            "display_name": null,
            "model_endpoint_type": "openai",
            "model_endpoint": "https://api.openai.com/v1",
            "provider_name": null,
            "provider_category": null,
            "model_wrapper": null,
            "context_window": 128000,
            "put_inner_thoughts_in_kwargs": false,
            "handle": null,
            "temperature": 1.0,
            "max_tokens": null,
            "enable_reasoner": true,
            "reasoning_effort": null,
            "max_reasoning_tokens": 0,
            "effort": null,
            "frequency_penalty": null,
            "compatibility_type": null,
            "verbosity": null,
            "tier": null,
            "parallel_tool_calls": false,
            "response_format": null,
            "strict": false,
            "return_logprobs": false,
            "top_logprobs": null,
            "return_token_ids": false,
            "tool_call_parser": null
        }),
        message_buffer_autoclear: false,
        in_context_message_indices: Vec::new(),
        messages: Vec::new(),
        metadata_: None,
        multi_agent_group: None,
        name: format!("coven-familiar-{}", portable.record.familiar_id),
        system: "Lossy Coven familiar handoff. Use the original .weave for Coven re-entry."
            .to_string(),
        tags: vec![
            AfTag {
                tag: "coven".to_string(),
            },
            AfTag {
                tag: "lossy-af-export".to_string(),
            },
        ],
        tool_exec_environment_variables: Vec::new(),
        tool_rules: Vec::new(),
        tools: Vec::new(),
        updated_at: timestamp.to_string(),
        version: "0.1.0".to_string(),
        x_coven_threads: AfNonRoundTrippableMarker {
            non_round_trippable: true,
            loss_reason: "Coven protection layer is stripped in Letta .af handoff".to_string(),
            reentry: "re-entry requires the original .weave; .af import is unsupported".to_string(),
            source_format: "coven-threads PortableWeave .weave".to_string(),
            source_weave_hash: portable.record.weave_hash.clone(),
        },
    })
}

/// Serialize a portable weave to canonical JSON bytes (reference encoding).
pub fn to_json_bytes(portable: &PortableWeave) -> Result<Vec<u8>, PortabilityError> {
    serde_json::to_vec_pretty(portable).map_err(|err| PortabilityError::Malformed {
        detail: err.to_string(),
    })
}

/// Decode a portable weave from JSON bytes. Decoding is not acceptance —
/// [`import_weave`] performs the contract and structural verification.
pub fn from_json_bytes(bytes: &[u8]) -> Result<PortableWeave, PortabilityError> {
    serde_json::from_slice(bytes).map_err(|err| PortabilityError::Malformed {
        detail: err.to_string(),
    })
}

/// Import a portable artifact, rebinding the (non-serializable, authoritative)
/// pattern predicate. Every check fails visibly; nothing degrades silently.
///
/// A clean import yields a weave whose tension state, thread set, and
/// `weave_hash` are equivalent to the exported weave's (C7), and whose
/// authority topology is byte-identical — import never widens authority
/// (RFC-0001 §5.1).
pub fn import_weave(
    portable: PortableWeave,
    pattern: Box<dyn PatternPredicate>,
) -> Result<Weave, PortabilityError> {
    let contract = SerializationContract::current();

    if portable.format_version != contract.format_version {
        return Err(PortabilityError::UnsupportedFormatVersion {
            found: portable.format_version,
            supported: contract.format_version,
        });
    }
    let expected_hash = contract.contract_hash();
    if portable.contract_hash != expected_hash {
        return Err(PortabilityError::ContractMismatch);
    }

    // Per-thread marker verification: every Serialization-covered thread must
    // carry a marker committing to the artifact's contract.
    for thread in &portable.record.threads {
        if !thread.holds_under.contains(&Channel::Serialization) {
            continue;
        }
        let marker_matches = thread.strands.iter().any(|s| {
            matches!(
                s,
                Strand::SerializationMarker { format_version, contract_hash, .. }
                    if format_version == &contract.format_version
                        && contract_hash == &expected_hash
            )
        });
        if !marker_matches {
            return Err(PortabilityError::MarkerContractMismatch {
                thread: thread.id,
                surface: thread.surface.clone(),
            });
        }
    }

    // Structural verification: recomputed weave hash must equal the recorded
    // one; duplicate (surface, writer) pairs are refused (§2.1).
    let surfaces = portable.surfaces;
    let weave = Weave::from_record(portable.record, pattern)?;

    // Shape B content-map verification happens only after the envelope itself
    // verifies. Legacy artifacts without a surfaces map remain valid C7
    // round-trip artifacts; non-empty maps are fail-closed.
    verify_surface_content(weave.threads(), &surfaces)?;

    Ok(weave)
}

fn verify_surface_content(
    threads: &[crate::thread::Thread],
    surfaces: &BTreeMap<SurfaceId, PortableSurfaceContent>,
) -> Result<(), PortabilityError> {
    if surfaces.is_empty() {
        return Ok(());
    }

    for (surface, content) in surfaces {
        let bytes = surface_content_bytes(surface, content)?;
        let mut matched = false;
        for thread in threads.iter().filter(|thread| &thread.surface == surface) {
            for strand in &thread.strands {
                if let Strand::ContentHash {
                    algorithm, value, ..
                } = strand
                {
                    matched = true;
                    let actual = hash_surface_bytes(*algorithm, bytes);
                    if &actual != value {
                        return Err(PortabilityError::SurfaceContentHashMismatch {
                            surface: surface.clone(),
                            algorithm: *algorithm,
                            expected: value.clone(),
                            actual,
                        });
                    }
                }
            }
        }
        if !matched {
            return Err(PortabilityError::SurfaceContentUnbound {
                surface: surface.clone(),
            });
        }
    }

    for thread in threads {
        let has_content_hash = thread
            .strands
            .iter()
            .any(|strand| matches!(strand, Strand::ContentHash { .. }));
        if has_content_hash && !surfaces.contains_key(&thread.surface) {
            return Err(PortabilityError::SurfaceContentMissing {
                surface: thread.surface.clone(),
            });
        }
    }

    Ok(())
}

fn surface_content_bytes<'a>(
    surface: &SurfaceId,
    content: &'a PortableSurfaceContent,
) -> Result<&'a [u8], PortabilityError> {
    if content.encoding == "utf8" {
        Ok(content.data.as_bytes())
    } else {
        Err(PortabilityError::UnsupportedSurfaceEncoding {
            surface: surface.clone(),
            encoding: content.encoding.clone(),
        })
    }
}

fn hash_surface_bytes(algorithm: HashAlgo, bytes: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgo::Blake3 => blake3::hash(bytes).as_bytes().to_vec(),
        HashAlgo::Sha256 => Sha256::digest(bytes).to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_hash_is_stable_and_version_sensitive() {
        let a = SerializationContract::current();
        let b = SerializationContract::current();
        assert_eq!(a.contract_hash(), b.contract_hash());

        let mut c = SerializationContract::current();
        c.format_version = "9.9.9".into();
        assert_ne!(a.contract_hash(), c.contract_hash());

        let mut d = SerializationContract::current();
        d.strand_kinds.pop();
        assert_ne!(a.contract_hash(), d.contract_hash());
    }

    #[test]
    fn marker_strand_commits_to_contract() {
        let contract = SerializationContract::current();
        match contract.marker_strand() {
            Strand::SerializationMarker {
                format_version,
                contract_hash,
                ..
            } => {
                assert_eq!(format_version, PORTABILITY_FORMAT_VERSION);
                assert_eq!(contract_hash, contract.contract_hash());
            }
            other => panic!("expected marker strand, got {other:?}"),
        }
    }
}
