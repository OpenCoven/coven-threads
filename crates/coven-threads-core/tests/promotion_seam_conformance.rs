//! Crate-local promotion seam conformance (threads-bv2).
//! Maps docs/seam-promotion-write-contract.md §§2–5 to public contracts.
//! Synthetic non-Tier-0 target only. No daemon admission, authorization,
//! provenance resolution, persistence, or end-to-end claim is made here.

use coven_threads_core::{
    validate_fail_closed, AllSurfacesHoldOnChannels, ApprovalPath, AuditEventType, Channel,
    FamiliarId, FrayOrSnap, FrayReason, MutationRequest, PatternDescriptor, PatternPredicate,
    ProposalClassification, ProposalId, RejectReason, SnapReason, SurfaceId, TensionState, Thread,
    ThreadId, Verdict, VetoWindow, WardAuditRecord, Weave, WeaveCoherence, WeaveId, WriterId,
};
use std::time::Duration;
use time::OffsetDateTime;

const SURFACE: &str = "notes/synthetic-entry.md";
const WRITER: &str = "familiar:synthetic-promotion";

fn thread() -> Thread {
    Thread {
        id: ThreadId::new(),
        surface: SurfaceId::new(SURFACE),
        writer: WriterId::new(WRITER),
        strands: vec![],
        holds_under: vec![Channel::Deliberate],
        created_at: OffsetDateTime::UNIX_EPOCH,
        tension: TensionState::Holds,
    }
}

fn request() -> MutationRequest {
    MutationRequest {
        surface: SurfaceId::new(SURFACE),
        writer: WriterId::new(WRITER),
        channel: Channel::Deliberate,
        identity_context: None,
    }
}

fn weave(thread: Thread, pattern: Box<dyn PatternPredicate>) -> Weave {
    Weave::new(
        WeaveId::new(),
        FamiliarId::new(),
        vec![thread],
        pattern,
        None,
    )
    .unwrap()
}

fn coherent_pattern() -> Box<dyn PatternPredicate> {
    Box::new(AllSurfacesHoldOnChannels {
        name: "synthetic-promotion".into(),
        surfaces: vec![SurfaceId::new(SURFACE)],
        channels: vec![Channel::Deliberate],
    })
}

fn assert_reject(weave: &Weave, request: &MutationRequest, reason: RejectReason) {
    let verdict = validate_fail_closed(weave, request);
    assert!(!verdict.permits_write());
    assert!(!verdict.requires_staging());
    assert_eq!(verdict, Verdict::Reject { reason });
}

#[test]
fn unknown_surface_is_rejected() {
    let w = weave(thread(), coherent_pattern());
    let mut r = request();
    r.surface = SurfaceId::new("notes/unregistered.md");
    assert_reject(
        &w,
        &r,
        RejectReason::UnknownSurface {
            surface: r.surface.clone(),
        },
    );
}

#[test]
fn unbound_writer_is_rejected() {
    let w = weave(thread(), coherent_pattern());
    let mut r = request();
    r.writer = WriterId::new("familiar:unbound-synthetic");
    assert_reject(
        &w,
        &r,
        RejectReason::WriterNotBound {
            surface: r.surface.clone(),
            writer: r.writer.clone(),
        },
    );
}

#[test]
fn deliberate_requires_explicit_thread_coverage() {
    let mut t = thread();
    t.holds_under = vec![Channel::Mutation];
    let id = t.id;
    assert_reject(
        &weave(t, coherent_pattern()),
        &request(),
        RejectReason::ChannelNotCovered {
            thread: id,
            channel: Channel::Deliberate,
        },
    );
}

#[test]
fn snapped_thread_is_rejected() {
    let mut t = thread();
    t.snap(
        Channel::Deliberate,
        SnapReason::Revoked,
        OffsetDateTime::UNIX_EPOCH,
    );
    let id = t.id;
    assert_reject(
        &weave(t, coherent_pattern()),
        &request(),
        RejectReason::ThreadSnapped {
            thread: id,
            reason: SnapReason::Revoked,
        },
    );
}

// Synthetic external predicates exercise the public validator boundary for
// outcomes not supplied by the structural predicate. Descriptors never decide.
#[derive(Debug)]
enum FixturePredicate {
    Broken,
    Degraded,
    Panics,
}
impl PatternPredicate for FixturePredicate {
    fn coherent(&self, _: &[Thread]) -> WeaveCoherence {
        match self {
            Self::Broken => WeaveCoherence::Broken {
                reason: "synthetic broken pattern".into(),
            },
            Self::Degraded => WeaveCoherence::Degraded {
                degraded_surfaces: vec![SurfaceId::new(SURFACE)],
                reason: "synthetic degraded surface".into(),
            },
            Self::Panics => panic!("synthetic predicate panic"),
        }
    }
    fn describe(&self) -> PatternDescriptor {
        panic!("validator must not use a descriptor as authority")
    }
}

#[test]
fn broken_pattern_is_rejected() {
    assert_reject(
        &weave(thread(), Box::new(FixturePredicate::Broken)),
        &request(),
        RejectReason::WeaveBroken {
            reason: "synthetic broken pattern".into(),
        },
    );
}

#[test]
fn degraded_target_is_rejected() {
    assert_reject(
        &weave(thread(), Box::new(FixturePredicate::Degraded)),
        &request(),
        RejectReason::SurfaceDegraded {
            surface: SurfaceId::new(SURFACE),
            reason: "synthetic degraded surface".into(),
        },
    );
}

#[test]
fn panicking_validator_is_rejected() {
    assert_reject(
        &weave(thread(), Box::new(FixturePredicate::Panics)),
        &request(),
        RejectReason::ValidatorPanic {
            diagnostic: "synthetic predicate panic".into(),
        },
    );
}

#[test]
fn healthy_proposal_eligible_thread_permits_at_the_crate_boundary() {
    let t = thread();
    let id = t.id;
    assert_eq!(
        validate_fail_closed(&weave(t, coherent_pattern()), &request()),
        Verdict::Permit { thread: id }
    );
}

#[test]
fn frayed_proposal_eligible_thread_stages_without_permitting() {
    let mut t = thread();
    t.fray(
        None,
        Channel::Deliberate,
        FrayReason::ContentHashMismatch,
        OffsetDateTime::UNIX_EPOCH,
    );
    let id = t.id;
    let verdict = validate_fail_closed(&weave(t, coherent_pattern()), &request());
    assert!(!verdict.permits_write());
    assert!(verdict.requires_staging());
    assert_eq!(
        verdict,
        Verdict::DegradeToProposal {
            thread: id,
            fray: FrayOrSnap::Frayed {
                strand: None,
                channel: Channel::Deliberate,
                reason: FrayReason::ContentHashMismatch,
            },
        }
    );
}

#[test]
fn admission_record_round_trips_and_distinguishes_deliberate_channel() {
    let familiar = FamiliarId::new();
    let records: Vec<_> = [Some(Channel::Deliberate), Some(Channel::Mutation), None]
        .into_iter()
        .map(|channel| {
            let record = WardAuditRecord::for_memory_entry_admitted(
                familiar,
                &[0x11; 32],
                &[0x22; 32],
                "ward_updated:synthetic-anchor",
                channel,
                OffsetDateTime::UNIX_EPOCH,
            );
            let json = serde_json::to_string(&record).unwrap();
            let decoded: WardAuditRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, record);
            assert_eq!(decoded.event_type, AuditEventType::MemoryEntryAdmitted);
            assert_eq!(decoded.channel, channel);
            record
        })
        .collect();
    assert_ne!(records[0], records[1]);
    assert_ne!(records[0], records[2]);
}

#[test]
fn deliberate_channel_does_not_select_an_approval_ceremony() {
    let veto = VetoWindow::new(Duration::from_secs(60), Duration::from_secs(10));
    let paths = [
        ApprovalPath::AutoRegression { veto: None },
        ApprovalPath::AutoRegression {
            veto: Some(veto.clone()),
        },
        ApprovalPath::FamiliarCoherence { veto },
        ApprovalPath::HumanApproval,
        ApprovalPath::HumanApprovalWithRationale,
    ];
    for path in paths {
        let classification = ProposalClassification {
            proposal_id: ProposalId::new(),
            familiar_id: FamiliarId::new(),
            channel: Channel::Deliberate,
            affected_surfaces: vec![SurfaceId::new(SURFACE)],
            affected_regions: vec![],
            path_tier_floor: 2,
            approval_path: path.clone(),
            evidence_replay_hash: [0x33; 32],
            classified_at: OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_value(&classification).unwrap();
        let decoded: ProposalClassification = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, classification);
        assert_eq!(decoded.channel, Channel::Deliberate);
        assert_eq!(decoded.approval_path, path);
    }
}
