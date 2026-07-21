use serde_json::{json, Value};

fn declarations(name: &str, person: &str, purpose: &str) -> Value {
    json!([
        format!("familiar.name == {name:?}"),
        format!("familiar.person == {person:?}"),
        "familiar.pronouns == \"they/them\"",
        format!("familiar.purpose includes {purpose:?}"),
        "familiar.coven == \"ExampleCoven\""
    ])
}

fn candidate_facts(name: &str, person: &str, purpose: &str) -> Value {
    json!([
        {"fact": "name", "value": name},
        {"fact": "person", "value": person},
        {"fact": "pronouns", "value": "they/them"},
        {"fact": "purpose", "value": format!("This synthetic familiar exists to {purpose}.")},
        {"fact": "coven", "value": "ExampleCoven"}
    ])
}

fn valid_case(
    id: &str,
    commitment_byte: u8,
    approval: Value,
    surfaces: Value,
    regions: Value,
    path_tier_floor: u8,
) -> Value {
    let name = format!("Synthetic-{id}");
    let person = format!("Example principal {commitment_byte}");
    let purpose = format!("exercise {id} migration fidelity");
    json!({
        "id": id,
        "declarations": declarations(&name, &person, &purpose),
        "candidate_commitment_byte": commitment_byte,
        "candidate_facts": candidate_facts(&name, &person, &purpose),
        "approval": approval,
        "surfaces": surfaces,
        "expected": {
            "regions": regions,
            "path_tier_floor": path_tier_floor
        }
    })
}

pub fn synthetic_retired_ward_corpus() -> Value {
    json!({
        "schema_version": "phase5-retired-ward-synthetic-v1",
        "provenance": {
            "kind": "synthetic",
            "historical_data_used": false,
            "authorization_basis": "Repository-authored synthetic values require no retired or private source corpus.",
            "notice": "This repository-authored corpus is fictional and must not be represented as recovered Ward v0.1 user data.",
            "generator": "crates/coven-threads-core/tests/support/phase5_retired_ward_corpus.rs"
        },
        "valid_cases": [
            valid_case(
                "auto",
                0x11,
                json!({
                    "kind": "auto_regression",
                    "label": "auto",
                    "veto": {
                        "duration_seconds": 3600,
                        "min_visible_seconds": 900
                    }
                }),
                json!([
                    {
                        "path": "MEMORY.md",
                        "before": "synthetic memory v1",
                        "after": "synthetic memory v2"
                    }
                ]),
                json!([]),
                u8::MAX
            ),
            valid_case(
                "auto-no-veto",
                0x55,
                json!({
                    "kind": "auto_regression",
                    "label": "auto",
                    "veto": null
                }),
                json!([
                    {
                        "path": "MEMORY.md",
                        "before": "synthetic memory v2",
                        "after": "synthetic memory v3"
                    }
                ]),
                json!([]),
                u8::MAX
            ),
            valid_case(
                "familiar-review",
                0x22,
                json!({
                    "kind": "familiar_coherence",
                    "label": "familiar_review",
                    "veto": {
                        "duration_seconds": 7200,
                        "min_visible_seconds": 1800
                    }
                }),
                json!([
                    {
                        "path": "TOOLS.md",
                        "before": "synthetic tool defaults v1",
                        "after": "synthetic tool defaults v2"
                    },
                    {
                        "path": "HEARTBEAT.md",
                        "before": "synthetic heartbeat v1",
                        "after": "synthetic heartbeat v2"
                    }
                ]),
                json!(["tool_defaults", "heartbeat_behavior"]),
                1
            ),
            valid_case(
                "human-review",
                0x33,
                json!({
                    "kind": "human_approval",
                    "label": "human_review"
                }),
                json!([
                    {
                        "path": "SOUL.md",
                        "before": "synthetic execution prompt v1",
                        "after": "synthetic execution prompt v2"
                    }
                ]),
                json!(["execution_prompt"]),
                0
            ),
            valid_case(
                "human-required",
                0x44,
                json!({
                    "kind": "human_approval_with_rationale",
                    "label": "human_required"
                }),
                json!([
                    {
                        "path": "AGENTS.md",
                        "before": "synthetic agent prompt v1",
                        "after": "synthetic agent prompt v2"
                    },
                    {
                        "path": "TOOLS.md",
                        "before": "synthetic tool defaults v1",
                        "after": "synthetic tool defaults v3"
                    }
                ]),
                json!(["execution_prompt", "tool_defaults"]),
                0
            )
        ],
        "unsupported_cases": [
            {
                "id": "unknown-identity-fact",
                "kind": "identity_declaration",
                "declarations": [
                    "familiar.name == \"Synthetic-Unknown\"",
                    "familiar.person == \"Example principal\"",
                    "familiar.favorite_color == \"purple\""
                ],
                "expected_error_contains": "unsupported identity fact"
            },
            {
                "id": "missing-candidate-pronouns",
                "kind": "candidate_facts",
                "candidate_facts": [
                    {"fact": "name", "value": "Synthetic-auto"},
                    {"fact": "person", "value": "Example principal 17"},
                    {"fact": "purpose", "value": "This synthetic familiar exists to exercise auto migration fidelity."},
                    {"fact": "coven", "value": "ExampleCoven"}
                ],
                "expected_error_contains": "Pronouns identity fact"
            },
            {
                "id": "invalid-veto-visibility",
                "kind": "veto_window",
                "approval": {
                    "kind": "familiar_coherence",
                    "label": "familiar_review",
                    "veto": {
                        "duration_seconds": 60,
                        "min_visible_seconds": 61
                    }
                },
                "expected_error_contains": "min_visible"
            },
            {
                "id": "duplicate-materialized-surface",
                "kind": "materialized_diff",
                "surfaces": [
                    {
                        "path": "TOOLS.md",
                        "before": "synthetic v1",
                        "after": "synthetic v2"
                    },
                    {
                        "path": "TOOLS.md",
                        "before": "synthetic v2",
                        "after": "synthetic v3"
                    }
                ],
                "expected_error_contains": "duplicate surface"
            },
            {
                "id": "absent-before-and-after",
                "kind": "materialized_diff",
                "surfaces": [
                    {
                        "path": "SOUL.md",
                        "before": null,
                        "after": null
                    }
                ],
                "expected_error_contains": "neither before nor after content"
            },
            {
                "id": "unchanged-materialized-surface",
                "kind": "materialized_diff",
                "surfaces": [
                    {
                        "path": "SOUL.md",
                        "before": "synthetic unchanged",
                        "after": "synthetic unchanged"
                    }
                ],
                "expected_error_contains": "unchanged"
            }
        ]
    })
}

pub fn canonical_corpus_json() -> String {
    serde_json::to_string(&synthetic_retired_ward_corpus()).expect("synthetic corpus serializes")
}
