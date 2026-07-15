//! C7 round-trip conformance suite (Phase 3, §3.3 invariant #4, RFC-0001 §5).
//!
//! "Export followed by import produces a weave with equivalent tension state,
//! or fails visibly." Positive arm: a weave carrying **every frozen strand
//! kind** and every tension state survives the transform with equivalent
//! tension, identical `weave_hash`, and byte-identical authority topology.
//! Negative arms: every tamper, version skew, contract skew, and missing
//! marker is a *typed, visible* refusal — never a silent downgrade.
//!
//! RFC-0001 §5.1 is asserted at the boundary: import is not a write path.
//! The imported weave's writers, surfaces, and channel coverage are exactly
//! the exported ones — an artifact cannot smuggle wider authority in.

use coven_threads_core::portability::{
    export_weave, from_json_bytes, import_weave, to_json_bytes, PortabilityError,
    SerializationContract, PORTABILITY_FORMAT_VERSION,
};
use coven_threads_core::{
    AllSurfacesHoldOnChannels, Channel, EventRef, FamiliarId, FrayReason, HashAlgo, ManifestId,
    PatternPredicate, SigKind, SnapReason, Strand, StrandId, SurfaceId, Thread, ThreadId, Weave,
    WeaveError, WriterId,
};
use time::OffsetDateTime;

fn contract_marker() -> Strand {
    SerializationContract::current().marker_strand()
}

fn all_five_strands() -> Vec<Strand> {
    vec![
        Strand::ContentHash {
            id: StrandId::new(),
            algorithm: HashAlgo::Blake3,
            value: vec![0x11; 32],
        },
        Strand::Signature {
            id: StrandId::new(),
            key_id: "val-ed25519-1".into(),
            kind: SigKind::Ed25519,
            value: vec![0x22; 64],
        },
        Strand::ManifestEntry {
            id: StrandId::new(),
            manifest_id: ManifestId::new(),
            entry_hash: vec![0x33; 32],
        },
        Strand::AuditTrail {
            id: StrandId::new(),
            first_seen: OffsetDateTime::UNIX_EPOCH,
            event_log_ref: EventRef::new("ward.audit/7"),
        },
        contract_marker(),
    ]
}

fn thread_on(surface: &str, writer: &str) -> Thread {
    Thread {
        id: ThreadId::new(),
        surface: SurfaceId::new(surface),
        writer: WriterId::new(writer),
        strands: all_five_strands(),
        holds_under: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
        created_at: OffsetDateTime::UNIX_EPOCH,
        tension: coven_threads_core::TensionState::Holds,
    }
}

fn floor_pattern() -> Box<dyn PatternPredicate> {
    Box::new(AllSurfacesHoldOnChannels::rfc0001_floor())
}

fn floor_weave() -> Weave {
    Weave::new(
        coven_threads_core::WeaveId::new(),
        FamiliarId::new(),
        ["SOUL.md", "IDENTITY.md", "MEMORY.md", "ward.toml"]
            .into_iter()
            .map(|s| thread_on(s, "principal:val"))
            .collect(),
        floor_pattern(),
        None,
    )
    .unwrap()
}

// ── Positive arm ─────────────────────────────────────────────────────────────

#[test]
fn all_five_strand_kinds_round_trip_with_equivalent_state() {
    let weave = floor_weave();
    let artifact = export_weave(&weave).expect("export");
    let bytes = to_json_bytes(&artifact).expect("encode");
    let decoded = from_json_bytes(&bytes).expect("decode");
    let imported = import_weave(decoded, floor_pattern()).expect("import");

    // C7: equivalent tension state — here, byte-equal threads.
    assert_eq!(imported.threads(), weave.threads());
    assert_eq!(imported.weave_hash(), weave.weave_hash());
    assert_eq!(imported.coherence(), weave.coherence());
}

#[test]
fn frayed_and_snapped_tension_survive_round_trip() {
    let mut weave = floor_weave();
    weave.update_threads(|threads| {
        let strand = threads[0].strands[0].id();
        threads[0].fray(
            Some(strand),
            Channel::Forced,
            FrayReason::ContentHashMismatch,
            OffsetDateTime::UNIX_EPOCH,
        );
        threads[1].snap(
            Channel::Mutation,
            SnapReason::Revoked,
            OffsetDateTime::UNIX_EPOCH,
        );
    });

    let bytes = to_json_bytes(&export_weave(&weave).unwrap()).unwrap();
    let imported = import_weave(from_json_bytes(&bytes).unwrap(), floor_pattern()).unwrap();

    // The frayed thread is still frayed, the snapped thread still snapped —
    // with the same channels and reasons. Nothing was healed or hidden.
    assert_eq!(imported.threads(), weave.threads());
    assert_eq!(imported.coherence(), weave.coherence());
}

#[test]
fn import_never_widens_authority() {
    // RFC-0001 §5.1: import is not a write path around the gates. The
    // authority topology after import is exactly the exported one.
    let weave = floor_weave();
    let bytes = to_json_bytes(&export_weave(&weave).unwrap()).unwrap();
    let imported = import_weave(from_json_bytes(&bytes).unwrap(), floor_pattern()).unwrap();

    let exported_topology: Vec<_> = weave
        .threads()
        .iter()
        .map(|t| (t.surface.clone(), t.writer.clone(), t.holds_under.clone()))
        .collect();
    let imported_topology: Vec<_> = imported
        .threads()
        .iter()
        .map(|t| (t.surface.clone(), t.writer.clone(), t.holds_under.clone()))
        .collect();
    assert_eq!(exported_topology, imported_topology);
}

// ── Negative arms: every failure is visible ──────────────────────────────────

#[test]
fn export_refuses_serialization_thread_without_marker() {
    let mut thread = thread_on("SOUL.md", "principal:val");
    thread
        .strands
        .retain(|s| s.kind() != coven_threads_core::StrandKind::SerializationMarker);
    let weave = Weave::new(
        coven_threads_core::WeaveId::new(),
        FamiliarId::new(),
        vec![thread],
        floor_pattern(),
        None,
    )
    .unwrap();

    let err = export_weave(&weave).unwrap_err();
    assert!(
        matches!(err, PortabilityError::MarkerMissing { .. }),
        "got {err:?}"
    );
}

#[test]
fn tampered_artifact_fails_visibly_on_import() {
    let weave = floor_weave();
    let mut artifact = export_weave(&weave).unwrap();
    // Tamper a strand payload after export.
    if let Strand::ContentHash { value, .. } = &mut artifact.record.threads[0].strands[0] {
        value[0] ^= 0xff;
    } else {
        panic!("fixture: first strand should be ContentHash");
    }
    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    assert!(
        matches!(
            err,
            PortabilityError::Weave(WeaveError::HashMismatch { .. })
        ),
        "got {err:?}"
    );
}

#[test]
fn tension_tamper_fails_visibly_on_import() {
    // Flipping a snapped thread back to Holds in the artifact must not import:
    // tension is part of the weave commitment.
    let mut weave = floor_weave();
    weave.update_threads(|threads| {
        threads[0].snap(
            Channel::Mutation,
            SnapReason::Revoked,
            OffsetDateTime::UNIX_EPOCH,
        );
    });
    let mut artifact = export_weave(&weave).unwrap();
    artifact.record.threads[0].tension = coven_threads_core::TensionState::Holds;

    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    assert!(
        matches!(
            err,
            PortabilityError::Weave(WeaveError::HashMismatch { .. })
        ),
        "resurrecting a snapped thread must fail visibly, got {err:?}"
    );
}

#[test]
fn unsupported_format_version_fails_visibly() {
    let weave = floor_weave();
    let mut artifact = export_weave(&weave).unwrap();
    artifact.format_version = "999.0.0".into();
    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    match err {
        PortabilityError::UnsupportedFormatVersion { found, supported } => {
            assert_eq!(found, "999.0.0");
            assert_eq!(supported, PORTABILITY_FORMAT_VERSION);
        }
        other => panic!("expected UnsupportedFormatVersion, got {other:?}"),
    }
}

#[test]
fn contract_hash_mismatch_fails_visibly() {
    let weave = floor_weave();
    let mut artifact = export_weave(&weave).unwrap();
    artifact.contract_hash[0] ^= 0xff;
    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    assert!(
        matches!(err, PortabilityError::ContractMismatch),
        "got {err:?}"
    );
}

#[test]
fn marker_committing_to_foreign_contract_fails_visibly() {
    // A thread whose marker was minted under a different contract (e.g. a
    // foreign runtime's strand vocabulary) must be refused by name.
    let mut thread = thread_on("SOUL.md", "principal:val");
    for strand in &mut thread.strands {
        if let Strand::SerializationMarker { contract_hash, .. } = strand {
            contract_hash[0] ^= 0xff;
        }
    }
    let weave = Weave::new(
        coven_threads_core::WeaveId::new(),
        FamiliarId::new(),
        vec![thread],
        floor_pattern(),
        None,
    )
    .unwrap();
    // Export succeeds structurally (a marker is present)…
    let artifact = export_weave(&weave).unwrap();
    // …but import verifies the commitment and refuses, naming the surface.
    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    match err {
        PortabilityError::MarkerContractMismatch { surface, .. } => {
            assert_eq!(surface, SurfaceId::new("SOUL.md"));
        }
        other => panic!("expected MarkerContractMismatch, got {other:?}"),
    }
}

#[test]
fn malformed_bytes_fail_visibly() {
    let err = from_json_bytes(b"not json at all").unwrap_err();
    assert!(
        matches!(err, PortabilityError::Malformed { .. }),
        "got {err:?}"
    );
}

#[test]
fn duplicate_pair_in_artifact_fails_visibly() {
    // §2.1: one thread per (surface, writer). An artifact that duplicates a
    // pair — however it was produced — is refused at import.
    let weave = floor_weave();
    let mut artifact = export_weave(&weave).unwrap();
    let duplicate = artifact.record.threads[0].clone();
    artifact.record.threads.push(duplicate);
    let err = import_weave(artifact, floor_pattern()).unwrap_err();
    assert!(
        matches!(
            err,
            PortabilityError::Weave(WeaveError::DuplicateThread { .. })
        ),
        "got {err:?}"
    );
}
