//! Closed output-format regions never confer authority over other content.

use coven_threads_core::{MaterializedDiff, SurfaceDiff, SurfaceId, SurfaceRegionRegistry};

const BEFORE: &[u8] = br#"{"schema":"coven.output-format/v1","indent":2,"final_newline":true}"#;
const AFTER: &[u8] = br#"{"schema":"coven.output-format/v1","indent":4,"final_newline":false}"#;

fn evidence(
    before: Option<&[u8]>,
    after: Option<&[u8]>,
) -> Vec<coven_threads_core::RegionEvidence> {
    let diff = MaterializedDiff::try_new(vec![SurfaceDiff {
        surface: SurfaceId::new("output-format.json"),
        before: before.map(ToOwned::to_owned),
        after: after.map(ToOwned::to_owned),
    }])
    .unwrap();
    SurfaceRegionRegistry::default_registry().classify_all(&diff)
}

#[test]
fn output_format_replacement_has_a_bounded_logged_region() {
    let regions = evidence(Some(BEFORE), Some(AFTER));
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].region_id.as_str(), "output_format");
    assert_eq!(regions[0].min_path_tier, 2);
    assert!(!regions[0].replay_bytes.is_empty());
}

#[test]
fn output_format_invalid_content_remains_blocking_region_evidence() {
    let oversized = [b' '; 257];
    let invalid: &[&[u8]] = &[
        b"{}",
        b"null",
        b"[]",
        b"\xff",
        b"{",
        br#"{"schema":"coven.output-format/v2","indent":2,"final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","indent":3,"final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","indent":2.0,"final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","indent":"2","final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","indent":2,"final_newline":"true"}"#,
        br#"{"schema":"coven.output-format/v1","indent":2,"final_newline":true,"prompt":"run"}"#,
        br#"{"schema":"coven.output-format/v1","indent":2,"indent":4,"final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","schema":"coven.output-format/v1","indent":2,"final_newline":true}"#,
        br#"{"schema":"coven.output-format/v1","indent":2,"final_newline":true,"final_newline":false}"#,
        br#"{"schema":"coven.output-format/v1","indent":2}"#,
        br#"{"schema":"coven.output-format/v1","indent":2,"final_newline":true} {}"#,
        &oversized,
    ];
    for bytes in invalid {
        for (before, after) in [(Some(BEFORE), Some(*bytes)), (Some(*bytes), Some(AFTER))] {
            let regions = evidence(before, after);
            assert_eq!(regions.len(), 1, "invalid bytes must not lose coverage");
            assert_eq!(regions[0].min_path_tier, 0);
        }
    }
    for (before, after) in [(None, Some(AFTER)), (Some(BEFORE), None)] {
        let regions = evidence(before, after);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].min_path_tier, 0);
    }
}

#[test]
fn output_format_all_four_settings_are_finite_and_replayable() {
    for indent in [2, 4] {
        for newline in [false, true] {
            let after = format!(
                "{{ \"schema\":\"coven.output-format/v1\",\"indent\":{indent},\"final_newline\":{newline}}}"
            );
            let first = evidence(Some(BEFORE), Some(after.as_bytes()));
            let second = evidence(Some(BEFORE), Some(after.as_bytes()));
            assert_eq!(first, second);
            assert_eq!(first.len(), 1);
            assert_eq!(first[0].min_path_tier, 2);
        }
    }
}

#[test]
fn output_format_does_not_cover_memory_or_arbitrary_paths() {
    for path in ["MEMORY.md", "notes.json", "nested/output-format.json"] {
        let diff = MaterializedDiff::try_new(vec![SurfaceDiff {
            surface: SurfaceId::new(path),
            before: Some(BEFORE.to_vec()),
            after: Some(AFTER.to_vec()),
        }])
        .unwrap();
        assert!(SurfaceRegionRegistry::default_registry()
            .classify_all(&diff)
            .is_empty());
    }
}

#[test]
fn output_format_limit_counts_exact_utf8_image_bytes() {
    let mut image = AFTER.to_vec();
    image.resize(256, b' ');
    assert_eq!(evidence(Some(BEFORE), Some(&image))[0].min_path_tier, 2);
    image.push(b' ');
    assert_eq!(evidence(Some(BEFORE), Some(&image))[0].min_path_tier, 0);
}

#[test]
fn output_format_review_array_before_is_not_an_object() {
    let regions = evidence(Some(br#"["coven.output-format/v1",4,false]"#), Some(AFTER));
    assert_eq!(regions[0].min_path_tier, 0);
}

#[test]
fn output_format_review_array_after_is_not_an_object() {
    let regions = evidence(Some(BEFORE), Some(br#"["coven.output-format/v1",4,false]"#));
    assert_eq!(regions[0].min_path_tier, 0);
}

#[test]
fn output_format_review_tagged_schema_before_is_not_a_string() {
    let regions = evidence(
        Some(br#"{"schema":{"coven.output-format/v1":null},"indent":4,"final_newline":false}"#),
        Some(AFTER),
    );
    assert_eq!(regions[0].min_path_tier, 0);
}

#[test]
fn output_format_review_tagged_schema_after_is_not_a_string() {
    let regions = evidence(
        Some(BEFORE),
        Some(br#"{"schema":{"coven.output-format/v1":null},"indent":4,"final_newline":false}"#),
    );
    assert_eq!(regions[0].min_path_tier, 0);
}
