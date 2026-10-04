//! Consumption snapshots (`validateConsumptionSnapshot` in the reference):
//! the Threads authority's signed record of adopted requests, consumed
//! decisions and approval lifecycle heads.

use serde_json::Value;

use super::canonical::canonicalize;
use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::request::validate_integrity_shape;
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

const CONSUMPTION_VERSION: &str = "opencoven.automation-consumption-snapshot/v1";

const SNAPSHOT_FIELDS: [&str; 8] = [
    "schema_version",
    "snapshot_id",
    "recorded_at",
    "store_revision",
    "request_adoptions",
    "decision_consumptions",
    "approval_heads",
    "integrity",
];

/// `validateConsumptionSnapshot`: a well-formed consumption snapshot, recorded
/// no later than `now` when given, signed by a `threads_authority` key.
///
/// # Errors
///
/// The first failed check's code, in the reference's order:
/// `consumption_snapshot_from_future`, `consumption_snapshot_duplicate`, or a
/// shape or integrity code.
pub fn validate_consumption_snapshot(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<()> {
    let path = "$.consumption_snapshot";
    shape::object(Some(value), path)?;
    exact_fields(Some(value), &SNAPSHOT_FIELDS, path)?;
    let field = |name: &str| value.get(name);
    shape::exact(
        field("schema_version"),
        CONSUMPTION_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.consumption_snapshot.schema_version",
    )?;
    shape::string(field("snapshot_id"), "$.consumption_snapshot.snapshot_id")?;
    let recorded = shape::timestamp(field("recorded_at"), "$.consumption_snapshot.recorded_at")?;
    shape::integer(
        field("store_revision"),
        "$.consumption_snapshot.store_revision",
        1,
    )?;
    if now.is_some() {
        let now = shape::timestamp(now, "$.trusted_now")?;
        if recorded > now {
            return fail(
                ErrorCode::ConsumptionSnapshotFromFuture,
                "consumption snapshot is newer than trusted now",
                "$",
            );
        }
    }

    let adoptions = shape::array(
        field("request_adoptions"),
        "$.consumption_snapshot.request_adoptions",
        false,
        false,
    )?;
    let mut adoption_keys = Vec::with_capacity(adoptions.len());
    for (index, adoption) in adoptions.iter().enumerate() {
        let path = format!("$.consumption_snapshot.request_adoptions[{index}]");
        exact_fields(
            Some(adoption),
            &["request_digest", "nonce", "adoption_key"],
            &path,
        )?;
        shape::digest_hex(
            adoption.get("request_digest"),
            &format!("{path}.request_digest"),
            true,
        )?;
        shape::string(adoption.get("nonce"), &format!("{path}.nonce"))?;
        shape::string(
            adoption.get("adoption_key"),
            &format!("{path}.adoption_key"),
        )?;
        let key = canonicalize(adoption)?;
        if adoption_keys.contains(&key) {
            return fail(
                ErrorCode::ConsumptionSnapshotDuplicate,
                "duplicate request adoption",
                "$",
            );
        }
        adoption_keys.push(key);
    }

    let consumptions = shape::array(
        field("decision_consumptions"),
        "$.consumption_snapshot.decision_consumptions",
        false,
        true,
    )?;
    for (index, digest) in consumptions.iter().enumerate() {
        shape::digest_hex(
            Some(digest),
            &format!("$.consumption_snapshot.decision_consumptions[{index}]"),
            true,
        )?;
    }

    let heads = shape::array(
        field("approval_heads"),
        "$.consumption_snapshot.approval_heads",
        false,
        false,
    )?;
    let mut approval_ids: Vec<&str> = Vec::with_capacity(heads.len());
    for (index, head) in heads.iter().enumerate() {
        let path = format!("$.consumption_snapshot.approval_heads[{index}]");
        exact_fields(
            Some(head),
            &["approval_id", "head_event_digest", "usage_count"],
            &path,
        )?;
        let approval_id = shape::string(head.get("approval_id"), &format!("{path}.approval_id"))?;
        shape::digest_hex(
            head.get("head_event_digest"),
            &format!("{path}.head_event_digest"),
            true,
        )?;
        shape::integer(head.get("usage_count"), &format!("{path}.usage_count"), 0)?;
        if approval_ids.contains(&approval_id) {
            return fail(
                ErrorCode::ConsumptionSnapshotDuplicate,
                "duplicate approval head",
                "$",
            );
        }
        approval_ids.push(approval_id);
    }
    validate_integrity_shape(field("integrity"), "$.consumption_snapshot.integrity")?;
    verify_signed_artifact(
        value,
        Domain::ConsumptionSnapshot,
        keyring,
        Some("threads_authority"),
        verifier,
    )?;
    Ok(())
}
