//! Conformance coverage for the public synthetic retired-Ward corpus generator.

#[path = "support/phase5_retired_ward_corpus.rs"]
mod corpus;

use std::time::Duration;

use coven_threads_core::approval::{ApprovalPath, ApprovalPathKind, VetoWindow};
use coven_threads_core::identity_invariants::{
    CandidateIdentityFact, CandidateIdentityFacts, IdentityFact, IdentityInvariantSet,
};
use coven_threads_core::ids::SurfaceId;
use coven_threads_core::pattern::WeaveCoherence;
use coven_threads_core::surface_regions::{MaterializedDiff, SurfaceDiff, SurfaceRegionRegistry};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn valid_cases() -> Vec<Value> {
    corpus::synthetic_retired_ward_corpus()["valid_cases"]
        .as_array()
        .expect("valid_cases is an array")
        .clone()
}

fn unsupported_cases() -> Vec<Value> {
    corpus::synthetic_retired_ward_corpus()["unsupported_cases"]
        .as_array()
        .expect("unsupported_cases is an array")
        .clone()
}

fn identity_fact(name: &str) -> IdentityFact {
    match name {
        "name" => IdentityFact::Name,
        "person" => IdentityFact::Person,
        "pronouns" => IdentityFact::Pronouns,
        "purpose" => IdentityFact::Purpose,
        "coven" => IdentityFact::Coven,
        other => panic!("unsupported synthetic identity fact {other}"),
    }
}

fn approval_path(case: &Value) -> Result<ApprovalPath, String> {
    let approval = &case["approval"];
    let veto = || {
        let veto = &approval["veto"];
        VetoWindow::try_new(
            Duration::from_secs(veto["duration_seconds"].as_u64().unwrap()),
            Duration::from_secs(veto["min_visible_seconds"].as_u64().unwrap()),
        )
    };
    match approval["kind"].as_str().unwrap() {
        "auto_regression" => Ok(ApprovalPath::AutoRegression {
            veto: approval["veto"].is_object().then(veto).transpose()?,
        }),
        "familiar_coherence" => Ok(ApprovalPath::FamiliarCoherence { veto: veto()? }),
        "human_approval" => Ok(ApprovalPath::HumanApproval),
        "human_approval_with_rationale" => Ok(ApprovalPath::HumanApprovalWithRationale),
        other => Err(format!("unsupported approval path kind {other}")),
    }
}

fn materialized_diff(case: &Value) -> Result<MaterializedDiff, String> {
    let surfaces = case["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| SurfaceDiff {
            surface: SurfaceId::new(entry["path"].as_str().unwrap()),
            before: entry["before"]
                .as_str()
                .map(|value| value.as_bytes().to_vec()),
            after: entry["after"]
                .as_str()
                .map(|value| value.as_bytes().to_vec()),
        })
        .collect();
    MaterializedDiff::try_new(surfaces)
}

#[test]
fn corpus_documents_synthetic_provenance_without_historical_data() {
    let corpus = corpus::synthetic_retired_ward_corpus();
    assert_eq!(corpus["schema_version"], "phase5-retired-ward-synthetic-v1");
    assert_eq!(corpus["provenance"]["kind"], "synthetic");
    assert_eq!(corpus["provenance"]["historical_data_used"], false);
    assert!(corpus["provenance"]["notice"]
        .as_str()
        .unwrap()
        .contains("repository-authored"));
}

#[test]
fn valid_cases_cover_identity_approval_veto_and_region_fidelity() {
    let cases = valid_cases();
    assert_eq!(cases.len(), 4);

    let labels: Vec<_> = cases
        .iter()
        .map(|case| case["approval"]["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        ["auto", "familiar_review", "human_review", "human_required"]
    );

    let mut covered_regions = Vec::new();
    for case in cases {
        let declarations: Vec<_> = case["declarations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        let invariants = IdentityInvariantSet::compile(declarations).unwrap();
        assert_eq!(invariants.declarations().len(), 5);

        let commitment = [case["candidate_commitment_byte"].as_u64().unwrap() as u8; 32];
        let facts = case["candidate_facts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| CandidateIdentityFact {
                fact: identity_fact(entry["fact"].as_str().unwrap()),
                value: entry["value"].as_str().unwrap().to_string(),
            })
            .collect();
        let facts = CandidateIdentityFacts::try_new(commitment, facts).unwrap();
        assert!(matches!(
            invariants.evaluate(commitment, Some(&facts)),
            WeaveCoherence::Coherent
        ));

        let approval = approval_path(&case).unwrap();
        assert_eq!(approval.display_label(), case["approval"]["label"]);
        assert_eq!(
            ApprovalPath::from_display_label(approval.display_label()),
            Some(match case["approval"]["kind"].as_str().unwrap() {
                "auto_regression" => ApprovalPathKind::AutoRegression,
                "familiar_coherence" => ApprovalPathKind::FamiliarCoherence,
                "human_approval" => ApprovalPathKind::HumanApproval,
                "human_approval_with_rationale" => {
                    ApprovalPathKind::HumanApprovalWithRationale
                }
                _ => unreachable!(),
            })
        );

        let diff = materialized_diff(&case).unwrap();
        let evidence = SurfaceRegionRegistry::default_registry().classify_all(&diff);
        let regions: Vec<_> = evidence
            .iter()
            .map(|entry| entry.region_id.as_str())
            .collect();
        assert_eq!(
            regions,
            case["expected"]["regions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            SurfaceRegionRegistry::path_tier_floor(&evidence),
            case["expected"]["path_tier_floor"].as_u64().unwrap() as u8
        );
        covered_regions.extend(regions.into_iter().map(str::to_string));
    }

    covered_regions.sort();
    covered_regions.dedup();
    assert_eq!(
        covered_regions,
        ["execution_prompt", "heartbeat_behavior", "tool_defaults"]
    );
}

#[test]
fn unsupported_cases_fail_closed_for_each_authority_input() {
    let cases = unsupported_cases();
    assert_eq!(cases.len(), 6);

    for case in cases {
        let error = match case["kind"].as_str().unwrap() {
            "identity_declaration" => {
                let declarations = case["declarations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap());
                IdentityInvariantSet::compile(declarations)
                    .unwrap_err()
                    .join("; ")
            }
            "candidate_facts" => {
                let source = valid_cases().remove(0);
                let declarations = source["declarations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap());
                let invariants = IdentityInvariantSet::compile(declarations).unwrap();
                let commitment = [0x41; 32];
                let facts = case["candidate_facts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| CandidateIdentityFact {
                        fact: identity_fact(entry["fact"].as_str().unwrap()),
                        value: entry["value"].as_str().unwrap().to_string(),
                    })
                    .collect();
                let facts = CandidateIdentityFacts::try_new(commitment, facts).unwrap();
                match invariants.evaluate(commitment, Some(&facts)) {
                    WeaveCoherence::Broken { reason, .. } => reason,
                    other => panic!("expected fail-closed identity result, got {other:?}"),
                }
            }
            "veto_window" => approval_path(&case).unwrap_err(),
            "materialized_diff" => materialized_diff(&case).unwrap_err(),
            other => panic!("unsupported synthetic failure kind {other}"),
        };
        assert!(
            error.contains(case["expected_error_contains"].as_str().unwrap()),
            "{error:?} did not match {:?}",
            case["expected_error_contains"]
        );
    }
}

#[test]
fn canonical_generator_output_has_a_pinned_sha256_digest() {
    let digest = Sha256::digest(corpus::canonical_corpus_json().as_bytes());
    assert_eq!(
        format!("{digest:x}"),
        "b3c5f156896ed4ef03b3f57bb8e65a33a5cf6fe52582ccd8403972a20299db44"
    );
}
