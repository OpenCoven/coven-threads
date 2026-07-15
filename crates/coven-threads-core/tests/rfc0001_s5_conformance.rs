//! RFC-0001 §5 conformance suite, mirrored in Rust (bead threads-986.11).
//!
//! Upstream: `familiar-contract/tests/conformance/` — 5 positive + 9 negative
//! bash-driven cases against `validators/validate.js`. Each test here mirrors
//! one upstream case directory into gate-layer facts:
//!
//! - a **file/section present and intact** upstream ⇢ a woven, fully-strung
//!   thread on that surface;
//! - a **missing protected file** ⇢ no thread on that surface;
//! - a **content violation** on a protected file ⇢ a frayed commitment strand
//!   (content drift is a hash mismatch at this layer);
//! - a **missing ward declaration** ⇢ the corresponding authority topology
//!   absent (no threads / no principal writer / no commitment strands).
//!
//! Positive cases MUST produce `WeaveCoherence::Coherent` and a bound-writer
//! `Permit`. Negative cases MUST NOT produce that all-green outcome — with the
//! specific non-green shape asserted per case. Fail-closed throughout
//! (RFC-0001 §5.4 Gate 4: line-one conformance, not hardening).

use coven_threads_core::{
    validate, AllSurfacesHoldOnChannels, Channel, FamiliarId, FrayReason, HashAlgo, ManifestId,
    MutationRequest, RejectReason, Strand, StrandId, SurfaceId, TensionState, Thread, ThreadId,
    Verdict, Weave, WeaveCoherence, WeaveId, WriterId,
};
use time::OffsetDateTime;

const FLOOR: [&str; 4] = ["SOUL.md", "IDENTITY.md", "MEMORY.md", "ward.toml"];
const PRINCIPAL: &str = "principal:alex";
const FAMILIAR: &str = "familiar:lumen";

fn full_strands() -> Vec<Strand> {
    vec![
        Strand::ContentHash {
            id: StrandId::new(),
            algorithm: HashAlgo::Blake3,
            value: vec![0x11; 32],
        },
        Strand::ManifestEntry {
            id: StrandId::new(),
            manifest_id: ManifestId::new(),
            entry_hash: vec![0x22; 32],
        },
        Strand::SerializationMarker {
            id: StrandId::new(),
            format_version: "0.1.0".into(),
            contract_hash: vec![0x33; 32],
        },
    ]
}

fn floor_channels() -> Vec<Channel> {
    vec![Channel::Forced, Channel::Serialization, Channel::Mutation]
}

fn thread_on(
    surface: &str,
    writer: &str,
    strands: Vec<Strand>,
    holds_under: Vec<Channel>,
) -> Thread {
    Thread {
        id: ThreadId::new(),
        surface: SurfaceId::new(surface),
        writer: WriterId::new(writer),
        strands,
        holds_under,
        created_at: OffsetDateTime::now_utc(),
        tension: TensionState::Holds,
    }
}

fn floor_pattern() -> Box<AllSurfacesHoldOnChannels> {
    Box::new(AllSurfacesHoldOnChannels::rfc0001_floor())
}

fn weave(threads: Vec<Thread>) -> Weave {
    Weave::new(
        WeaveId::new(),
        FamiliarId::new(),
        threads,
        floor_pattern(),
        None,
    )
    .expect("conformance fixtures respect one-thread-per-(surface,writer)")
}

fn request(surface: &str, writer: &str, channel: Channel) -> MutationRequest {
    MutationRequest {
        surface: SurfaceId::new(surface),
        writer: WriterId::new(writer),
        channel,
    }
}

/// The all-green outcome positive cases must reach and negative cases must not:
/// coherent weave, and the principal writer permitted on every floor surface.
fn assert_conformant(w: &Weave) {
    assert_eq!(
        w.coherence(),
        WeaveCoherence::Coherent,
        "weave must be coherent"
    );
    for surface in FLOOR {
        let v = validate(w, &request(surface, PRINCIPAL, Channel::Mutation));
        assert!(
            v.permits_write(),
            "expected Permit for {PRINCIPAL} on {surface}, got {v:?}"
        );
    }
}

// ── Positive cases ───────────────────────────────────────────────────────────

/// Mirrors `tests/conformance/positive/01-minimal-compliant`: the smallest
/// known-good surface — all four floor files present, person binding intact.
#[test]
fn positive_01_minimal_compliant() {
    let w = weave(
        FLOOR
            .into_iter()
            .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
            .collect(),
    );
    assert_conformant(&w);
}

/// Mirrors `positive/02-full-compliant`: richer identity — extra commitment
/// strands (signature + audit trail) and Deliberate-channel coverage on top of
/// the floor. Richness must not break conformance.
#[test]
fn positive_02_full_compliant() {
    use coven_threads_core::{EventRef, SigKind};
    let rich_strands = || {
        let mut s = full_strands();
        s.push(Strand::Signature {
            id: StrandId::new(),
            key_id: "alex-ed25519-0".into(),
            kind: SigKind::Ed25519,
            value: vec![0x44; 64],
        });
        s.push(Strand::AuditTrail {
            id: StrandId::new(),
            first_seen: OffsetDateTime::UNIX_EPOCH,
            event_log_ref: EventRef::new("ward.audit/1"),
        });
        s
    };
    let mut channels = floor_channels();
    channels.push(Channel::Deliberate);

    let w = weave(
        FLOOR
            .into_iter()
            .map(|s| thread_on(s, PRINCIPAL, rich_strands(), channels.clone()))
            .collect(),
    );
    assert_conformant(&w);
    // Deliberate is additionally covered for this richer familiar.
    let v = validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Deliberate));
    assert!(v.permits_write(), "rich familiar covers Deliberate: {v:?}");
}

/// Mirrors `positive/03-multi-role`: role scaffolding under `roles/` on the
/// editable surface, familiar-bound. The protected identity contract is
/// unchanged and still validates; the familiar writer gets its own lane.
#[test]
fn positive_03_multi_role() {
    let mut threads: Vec<Thread> = FLOOR
        .into_iter()
        .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
        .collect();
    // Editable role file: familiar-writable, all lanes strung.
    threads.push(thread_on(
        "roles/researcher.md",
        FAMILIAR,
        full_strands(),
        vec![
            Channel::Mutation,
            Channel::Deliberate,
            Channel::Forced,
            Channel::Serialization,
        ],
    ));
    let w = weave(threads);
    assert_conformant(&w);
    // The familiar writes its role file...
    let v = validate(
        &w,
        &request("roles/researcher.md", FAMILIAR, Channel::Mutation),
    );
    assert!(
        v.permits_write(),
        "familiar edits its editable surface: {v:?}"
    );
    // ...but has no path to the protected identity contract (RFC-0001 §5.1).
    let v = validate(&w, &request("SOUL.md", FAMILIAR, Channel::Mutation));
    assert!(
        matches!(
            v,
            Verdict::Reject {
                reason: RejectReason::WriterNotBound { .. }
            }
        ),
        "familiar must have no write path to SOUL.md: {v:?}"
    );
}

/// Mirrors `positive/04-with-user-md`: optional `USER.md` carried and treated
/// as protected-if-present. Adding it must not break compliance.
#[test]
fn positive_04_with_user_md() {
    let mut surfaces: Vec<&str> = FLOOR.to_vec();
    surfaces.push("USER.md");
    let threads = surfaces
        .iter()
        .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
        .collect();

    // USER.md joins the protected pattern for this familiar.
    let mut pattern = AllSurfacesHoldOnChannels::rfc0001_floor();
    pattern.surfaces.push(SurfaceId::new("USER.md"));
    let w = Weave::new(
        WeaveId::new(),
        FamiliarId::new(),
        threads,
        Box::new(pattern),
        None,
    )
    .unwrap();

    assert_eq!(w.coherence(), WeaveCoherence::Coherent);
    for surface in surfaces {
        let v = validate(&w, &request(surface, PRINCIPAL, Channel::Mutation));
        assert!(v.permits_write(), "expected Permit on {surface}, got {v:?}");
    }
}

/// Mirrors `positive/05-tier-rich-ward`: all four approval tiers declared. At
/// the gate layer, tiers are distinct writers with distinct channel coverage —
/// `auto` (daemon) gets an editable-surface lane; `human_review` (principal)
/// keeps the protected lanes. Both coexist per (surface, writer) pairing.
#[test]
fn positive_05_tier_rich_ward() {
    let mut threads: Vec<Thread> = FLOOR
        .into_iter()
        .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
        .collect();
    // auto-tier lane: daemon writer on the editable surface, all lanes strung.
    threads.push(thread_on(
        "TOOLS.md",
        "daemon:auto-tier",
        full_strands(),
        vec![
            Channel::Deliberate,
            Channel::Forced,
            Channel::Serialization,
            Channel::Mutation,
        ],
    ));
    let w = weave(threads);
    assert_conformant(&w);

    let v = validate(
        &w,
        &request("TOOLS.md", "daemon:auto-tier", Channel::Deliberate),
    );
    assert!(v.permits_write(), "auto tier promotes on Deliberate: {v:?}");
    // The auto tier never touches the protected surface (§5.3: no tier may
    // auto-promote proposals targeting the protected surface).
    let v = validate(
        &w,
        &request("SOUL.md", "daemon:auto-tier", Channel::Deliberate),
    );
    assert!(
        matches!(v, Verdict::Reject { .. }),
        "auto tier must have no path to SOUL.md: {v:?}"
    );
}

// ── Negative cases ───────────────────────────────────────────────────────────

/// Mirrors `negative/01-missing-soul`: SOUL.md absent (Named Identity violated).
/// No thread can bind a missing surface → weave degraded at SOUL.md, mutation
/// rejected fail-closed.
#[test]
fn negative_01_missing_soul() {
    let w = weave(
        FLOOR
            .into_iter()
            .filter(|s| *s != "SOUL.md")
            .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
            .collect(),
    );
    match w.coherence() {
        WeaveCoherence::Degraded {
            degraded_surfaces, ..
        } => {
            assert_eq!(degraded_surfaces, vec![SurfaceId::new("SOUL.md")]);
        }
        other => panic!("expected Degraded at SOUL.md, got {other:?}"),
    }
    let v = validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Mutation));
    assert!(
        matches!(
            v,
            Verdict::Reject {
                reason: RejectReason::UnknownSurface { .. }
            }
        ),
        "got {v:?}"
    );
}

/// Mirrors `negative/02-missing-ward`: ward.toml absent (Bounded Authority +
/// Human Belonging violated). Same fail-closed shape at ward.toml.
#[test]
fn negative_02_missing_ward() {
    let w = weave(
        FLOOR
            .into_iter()
            .filter(|s| *s != "ward.toml")
            .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
            .collect(),
    );
    match w.coherence() {
        WeaveCoherence::Degraded {
            degraded_surfaces, ..
        } => {
            assert_eq!(degraded_surfaces, vec![SurfaceId::new("ward.toml")]);
        }
        other => panic!("expected Degraded at ward.toml, got {other:?}"),
    }
    let v = validate(&w, &request("ward.toml", PRINCIPAL, Channel::Mutation));
    assert!(matches!(v, Verdict::Reject { .. }), "got {v:?}");
}

/// Mirrors `negative/03-no-person-binding`: `[meta].person` missing (Human
/// Belonging violated). The authority topology has no principal writer — every
/// principal-path request rejects `WriterNotBound`, leaving the familiar with
/// no human-belonging lane at all.
#[test]
fn negative_03_no_person_binding() {
    // Threads woven by a bootstrap process with no person bound.
    let w = weave(
        FLOOR
            .into_iter()
            .map(|s| thread_on(s, "daemon:bootstrap", full_strands(), floor_channels()))
            .collect(),
    );
    assert!(
        !w.threads()
            .iter()
            .any(|t| t.writer.as_str().starts_with("principal:")),
        "fixture: no principal binding anywhere"
    );
    for surface in FLOOR {
        let v = validate(&w, &request(surface, PRINCIPAL, Channel::Mutation));
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::WriterNotBound { .. }
                }
            ),
            "no person binding must reject principal on {surface}: {v:?}"
        );
    }
}

/// Mirrors `negative/04-no-protected-section`: `[protected]` missing entirely
/// (Bounded Authority violated). Nothing was declared protected → no threads
/// woven → the floor pattern is fundamentally broken and everything rejects.
#[test]
fn negative_04_no_protected_section() {
    let w = weave(vec![]);
    assert!(
        matches!(w.coherence(), WeaveCoherence::Broken { .. }),
        "no protected surface at all must be Broken, got {:?}",
        w.coherence()
    );
    for surface in FLOOR {
        let v = validate(&w, &request(surface, PRINCIPAL, Channel::Mutation));
        assert!(
            matches!(
                v,
                Verdict::Reject {
                    reason: RejectReason::UnknownSurface { .. }
                }
            ),
            "got {v:?}"
        );
    }
}

/// Mirrors `negative/05-protected-missing-soul`: `[protected].files` omits
/// SOUL.md (Bounded Authority violated). Degradation is *localized* (§2.2):
/// SOUL.md rejects while the still-protected surfaces continue.
#[test]
fn negative_05_protected_missing_soul() {
    let w = weave(
        FLOOR
            .into_iter()
            .filter(|s| *s != "SOUL.md")
            .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
            .collect(),
    );
    match w.coherence() {
        WeaveCoherence::Degraded {
            degraded_surfaces, ..
        } => {
            assert_eq!(degraded_surfaces, vec![SurfaceId::new("SOUL.md")]);
        }
        other => panic!("expected Degraded at SOUL.md, got {other:?}"),
    }
    // SOUL.md fail-closed…
    let v = validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Mutation));
    assert!(matches!(v, Verdict::Reject { .. }), "got {v:?}");
    // …while the familiar continues on other surfaces (§5).
    let v = validate(&w, &request("MEMORY.md", PRINCIPAL, Channel::Mutation));
    assert!(v.permits_write(), "MEMORY.md must continue: {v:?}");
}

/// Mirrors `negative/06-no-core-work`: SOUL.md's `## Core Work` section missing
/// (Defined Purpose violated). Content drift on a protected surface is a
/// `ContentHash` fray at this layer → mutation degrades to proposal, never a
/// silent Permit.
#[test]
fn negative_06_no_core_work() {
    let mut threads: Vec<Thread> = FLOOR
        .into_iter()
        .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
        .collect();
    assert_eq!(
        threads[0].surface,
        SurfaceId::new("SOUL.md"),
        "fixture order"
    );
    let soul_strand = threads[0].strands[0].id();
    threads[0].fray(
        Some(soul_strand),
        Channel::Mutation,
        FrayReason::ContentHashMismatch,
        OffsetDateTime::now_utc(),
    );
    let w = weave(threads);

    assert_ne!(w.coherence(), WeaveCoherence::Coherent);
    let v = validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Mutation));
    assert!(
        v.requires_staging(),
        "content-drifted SOUL.md must degrade to proposal: {v:?}"
    );
}

/// Mirrors `negative/07-no-what-i-am-not`: SOUL.md's `## What I Am Not` section
/// missing (Defined Purpose violated). Same protected-content-drift shape as
/// 06, exercised through the external-manifest strand (`Channel::Forced`'s
/// agent-independent commitment).
#[test]
fn negative_07_no_what_i_am_not() {
    let mut threads: Vec<Thread> = FLOOR
        .into_iter()
        .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
        .collect();
    assert_eq!(
        threads[0].surface,
        SurfaceId::new("SOUL.md"),
        "fixture order"
    );
    let manifest_strand = threads[0]
        .strands
        .iter()
        .find(|s| matches!(s, Strand::ManifestEntry { .. }))
        .map(Strand::id);
    threads[0].fray(
        manifest_strand,
        Channel::Forced,
        FrayReason::ManifestEntryMismatch,
        OffsetDateTime::now_utc(),
    );
    let w = weave(threads);

    assert_ne!(w.coherence(), WeaveCoherence::Coherent);
    let v = validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Forced));
    assert!(v.requires_staging(), "got {v:?}");
    assert!(
        !validate(&w, &request("SOUL.md", PRINCIPAL, Channel::Mutation)).permits_write(),
        "a frayed thread must not permit on any covered channel"
    );
}

/// Mirrors `negative/08-no-invariants`: `[protected].invariants` empty (Bounded
/// Authority + Human Belonging violated). Invariants are what commit content at
/// this layer — threads woven without commitment strands fail every channel's
/// structural floor (§2.4), so nothing holds and nothing permits.
#[test]
fn negative_08_no_invariants() {
    let w = weave(
        FLOOR
            .into_iter()
            .map(|s| thread_on(s, PRINCIPAL, vec![], floor_channels()))
            .collect(),
    );
    assert!(
        matches!(w.coherence(), WeaveCoherence::Broken { .. }),
        "no commitment strands anywhere: every floor surface fails, got {:?}",
        w.coherence()
    );
    for surface in FLOOR {
        let v = validate(&w, &request(surface, PRINCIPAL, Channel::Mutation));
        assert!(
            !v.permits_write(),
            "strand-less thread must not permit on {surface}: {v:?}"
        );
    }
}

/// Mirrors `negative/09-missing-memory`: MEMORY.md absent (Persistent Memory
/// violated). Upstream's current validator only warns; v0.2 conformance expects
/// a failure, and this layer fails closed accordingly.
#[test]
fn negative_09_missing_memory() {
    let w = weave(
        FLOOR
            .into_iter()
            .filter(|s| *s != "MEMORY.md")
            .map(|s| thread_on(s, PRINCIPAL, full_strands(), floor_channels()))
            .collect(),
    );
    match w.coherence() {
        WeaveCoherence::Degraded {
            degraded_surfaces, ..
        } => {
            assert_eq!(degraded_surfaces, vec![SurfaceId::new("MEMORY.md")]);
        }
        other => panic!("expected Degraded at MEMORY.md, got {other:?}"),
    }
    let v = validate(&w, &request("MEMORY.md", PRINCIPAL, Channel::Mutation));
    assert!(
        matches!(
            v,
            Verdict::Reject {
                reason: RejectReason::UnknownSurface { .. }
            }
        ),
        "got {v:?}"
    );
}
