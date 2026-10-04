//! The approval lifecycle (`validateLifecycleEvent`, `applyLifecycleEvent` and
//! `verifyLifecycleChain` in the reference): an append-only chain of events,
//! signed by the Threads authority, that moves one approval from `required`
//! to `approved` and on to `consumed`, `rejected`, `expired` or `revoked`.

use serde_json::{Map, Value};

use super::approval::validate_approval;
use super::canonical::{canonical_digest, canonicalize};
use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::request::validate_integrity_shape;
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

const EVENT_VERSION: &str = "opencoven.automation-approval-event/v1";

const EVENT_FIELDS: [&str; 18] = [
    "schema_version",
    "event_id",
    "approval_id",
    "request_digest",
    "decision_digest",
    "approval_digest",
    "sequence",
    "previous_event_digest",
    "from_state",
    "to_state",
    "event",
    "occurred_at",
    "actor",
    "execution_phase",
    "dispatch_disposition",
    "consumption",
    "occurrence_disposition",
    "integrity",
];

const CONSUMPTION_FIELDS: [&str; 6] = [
    "request_digest",
    "decision_digest",
    "occurrence_id",
    "run_id",
    "attempt",
    "fence_generation",
];

/// `TRANSITIONS`: the target state of each `from_state:event` pair other than
/// `consume`, whose target depends on the approval's use.
const TRANSITIONS: [(&str, &str, &str); 7] = [
    ("required", "request", "requested"),
    ("requested", "approve", "approved"),
    ("requested", "reject", "rejected"),
    ("requested", "expire", "expired"),
    ("requested", "revoke", "revoked"),
    ("approved", "revoke", "revoked"),
    ("approved", "expire", "expired"),
];

/// Where an approval's lifecycle stands after a run of events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LifecycleState {
    /// The approval the chain concerns.
    pub approval_id: String,
    /// The request the approval answers, `sha256:`-prefixed.
    pub request_digest: String,
    /// The decision the approval answers, `sha256:`-prefixed.
    pub decision_digest: String,
    /// The current state, such as `approved` or `consumed`.
    pub state: String,
    /// The last applied event's sequence number.
    pub sequence: i64,
    /// The last applied event's digest, `sha256:`-prefixed.
    pub last_event_digest: String,
    /// How many `consume` events the chain holds.
    pub consumption_count: u64,
    /// The canonical text of each consumed per-run binding.
    pub consumed_occurrences: Vec<String>,
    /// The applied events' ids, in order.
    pub event_ids: Vec<String>,
}

/// `validateLifecycleEvent`: a well-formed event whose dispositions fit its
/// kind, signed by a `threads_authority` key.
fn validate_lifecycle_event(
    event: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<()> {
    shape::object(Some(event), "$.event")?;
    exact_fields(Some(event), &EVENT_FIELDS, "$.event")?;
    let field = |name: &str| event.get(name);
    shape::exact(
        field("schema_version"),
        EVENT_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.event.schema_version",
    )?;
    shape::string(field("event_id"), "$.event.event_id")?;
    shape::string(field("approval_id"), "$.event.approval_id")?;
    shape::digest_hex(field("request_digest"), "$.event.request_digest", true)?;
    shape::digest_hex(field("decision_digest"), "$.event.decision_digest", true)?;
    if field("approval_digest") != Some(&Value::Null) {
        shape::digest_hex(field("approval_digest"), "$.event.approval_digest", true)?;
    }
    shape::integer(field("sequence"), "$.event.sequence", 1)?;
    if field("previous_event_digest") != Some(&Value::Null) {
        shape::digest_hex(
            field("previous_event_digest"),
            "$.event.previous_event_digest",
            true,
        )?;
    }
    shape::enumeration(
        field("from_state"),
        &[
            "required",
            "requested",
            "approved",
            "rejected",
            "expired",
            "revoked",
            "consumed",
        ],
        "$.event.from_state",
    )?;
    let kind = field("event").and_then(Value::as_str);
    let consumption = field("consumption");
    let has_consumption = consumption != Some(&Value::Null);
    if has_consumption {
        let path = "$.event.consumption";
        exact_fields(consumption, &CONSUMPTION_FIELDS, path)?;
        let member = |name: &str| consumption.and_then(|c| c.get(name));
        shape::digest_hex(
            member("request_digest"),
            &format!("{path}.request_digest"),
            true,
        )?;
        shape::digest_hex(
            member("decision_digest"),
            &format!("{path}.decision_digest"),
            true,
        )?;
        shape::string(member("occurrence_id"), &format!("{path}.occurrence_id"))?;
        shape::string(member("run_id"), &format!("{path}.run_id"))?;
        shape::integer(member("attempt"), &format!("{path}.attempt"), 1)?;
        shape::integer(
            member("fence_generation"),
            &format!("{path}.fence_generation"),
            1,
        )?;
    }
    if kind != Some("consume") && has_consumption {
        return fail(
            ErrorCode::LifecycleConsumptionUnexpected,
            "only consume events may carry per-run binding",
            "$",
        );
    }
    let disposition = field("occurrence_disposition");
    let has_disposition = disposition != Some(&Value::Null);
    if has_disposition {
        let path = "$.event.occurrence_disposition";
        exact_fields(
            disposition,
            &["occurrence_id", "run_id", "disposition"],
            path,
        )?;
        let member = |name: &str| disposition.and_then(|d| d.get(name));
        shape::string(member("occurrence_id"), &format!("{path}.occurrence_id"))?;
        shape::string(member("run_id"), &format!("{path}.run_id"))?;
        shape::enumeration(
            member("disposition"),
            &["rejected_no_launch", "expired_no_launch"],
            &format!("{path}.disposition"),
        )?;
    }
    if !matches!(kind, Some("reject" | "expire")) && has_disposition {
        return fail(
            ErrorCode::LifecycleOccurrenceDispositionUnexpected,
            "only rejection and expiry may carry a no-launch occurrence disposition",
            "$",
        );
    }
    shape::enumeration(
        field("to_state"),
        &[
            "requested",
            "approved",
            "rejected",
            "expired",
            "revoked",
            "consumed",
        ],
        "$.event.to_state",
    )?;
    let kind = shape::enumeration(
        field("event"),
        &[
            "request", "approve", "reject", "expire", "revoke", "consume",
        ],
        "$.event.event",
    )?;
    shape::timestamp(field("occurred_at"), "$.event.occurred_at")?;
    shape::exact(
        field("actor"),
        "threads_authority",
        ErrorCode::LifecycleActorForged,
        "$.event.actor",
    )?;
    let phase = shape::enumeration(
        field("execution_phase"),
        &[
            "not_applicable",
            "not_started",
            "queued",
            "dispatching",
            "running",
            "completed",
        ],
        "$.event.execution_phase",
    )?;
    let dispatch = shape::enumeration(
        field("dispatch_disposition"),
        &[
            "not_applicable",
            "launch_authorized",
            "cancel_before_launch",
            "request_cooperative_cancel",
            "external_effects_not_rolled_back",
            "no_launch_rejected",
            "no_launch_expired",
        ],
        "$.event.dispatch_disposition",
    )?;
    // `event.occurrence_disposition?.disposition`.
    let disposition_kind = disposition
        .and_then(|d| d.get("disposition"))
        .and_then(Value::as_str);
    match kind {
        "reject" => {
            if phase != "not_started" || dispatch != "no_launch_rejected" {
                return fail(
                    ErrorCode::RejectionDispositionInvalid,
                    "rejection must record an explicit no-launch disposition",
                    "$",
                );
            }
            if disposition_kind != Some("rejected_no_launch") {
                return fail(
                    ErrorCode::RejectionOccurrenceMissing,
                    "rejection must identify the no-launch occurrence",
                    "$",
                );
            }
        }
        "expire" => {
            if phase != "not_started" || dispatch != "no_launch_expired" {
                return fail(
                    ErrorCode::ExpirationDispositionInvalid,
                    "expiry must record an explicit no-launch disposition",
                    "$",
                );
            }
            if disposition_kind != Some("expired_no_launch") {
                return fail(
                    ErrorCode::ExpirationOccurrenceMissing,
                    "expiry must identify the no-launch occurrence",
                    "$",
                );
            }
        }
        "revoke" => {
            if matches!(phase, "not_started" | "queued" | "dispatching")
                && dispatch != "cancel_before_launch"
            {
                return fail(
                    ErrorCode::RevocationDispositionInvalid,
                    "pre-launch revocation must cancel before launch",
                    "$",
                );
            }
            if phase == "running"
                && !matches!(
                    dispatch,
                    "request_cooperative_cancel" | "external_effects_not_rolled_back"
                )
            {
                return fail(
                    ErrorCode::RevocationDispositionInvalid,
                    "running revocation must request cancellation or preserve completed external effects",
                    "$",
                );
            }
        }
        "consume" => {
            if !has_consumption {
                return fail(
                    ErrorCode::ConsumptionBindingMissing,
                    "consumption event must bind the exact per-run operation",
                    "$",
                );
            }
            if phase != "dispatching" || dispatch != "launch_authorized" {
                return fail(
                    ErrorCode::ConsumptionDispositionInvalid,
                    "consumption must be atomic with launch authorization",
                    "$",
                );
            }
        }
        _ => {
            if phase != "not_applicable"
                || dispatch != "not_applicable"
                || has_consumption
                || has_disposition
            {
                return fail(
                    ErrorCode::LifecycleDispositionInvalid,
                    "non-dispatch lifecycle event has a dispatch disposition",
                    "$",
                );
            }
        }
    }
    validate_integrity_shape(field("integrity"), "$.event.integrity")?;
    verify_signed_artifact(
        event,
        Domain::ApprovalEvent,
        keyring,
        Some("threads_authority"),
        verifier,
    )?;
    Ok(())
}

/// JavaScript `object.name`: `undefined` for a primitive or array without the
/// member, and a `TypeError` on `undefined` or `null`, which the reference
/// throws as no profile code and this port refuses as `schema_type`.
fn member<'a>(
    object: Option<&'a Value>,
    name: &str,
    path: &str,
) -> AuthorityResult<Option<&'a Value>> {
    match object {
        None | Some(Value::Null) => fail(
            ErrorCode::SchemaType,
            format!("cannot read {name} of a missing value"),
            path,
        ),
        Some(object) => Ok(object.get(name)),
    }
}

/// `approval.use.kind`, which the reference reads from an approval it has not
/// necessarily validated.
fn use_kind(approval: Option<&Value>) -> AuthorityResult<Option<&Value>> {
    member(
        member(approval, "use", "$.approval")?,
        "kind",
        "$.approval.use",
    )
}

/// `applyLifecycleEvent`: `event` validated and appended to the chain whose
/// head is `state` (`None` before the first event). A transition that needs
/// approval evidence, `approve` or anything from `approved`, validates
/// `approval` and requires the event to bind it exactly.
///
/// # Errors
///
/// An event or approval validation code, then the first failed chain check
/// (`lifecycle_*`, `approval_replayed`, `approval_consumption_mismatch`,
/// `approval_occurrence_out_of_scope` or `approval_usage_exhausted`), in the
/// reference's order. Where the reference reads a member of a missing value
/// and throws a `TypeError`, this port refuses with `schema_type`.
pub fn apply_lifecycle_event(
    state: Option<&LifecycleState>,
    event: &Value,
    approval: Option<&Value>,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<LifecycleState> {
    validate_lifecycle_event(event, keyring, verifier)?;
    let kind = event["event"].as_str().unwrap_or_default();
    let from_state = event["from_state"].as_str().unwrap_or_default();
    if kind == "approve" || from_state == "approved" {
        if !shape::truthy(approval) {
            return fail(
                ErrorCode::LifecycleApprovalMissing,
                "transition requires immutable approval evidence",
                "$",
            );
        }
        let evidence = approval.expect("a truthy approval is present");
        validate_approval(evidence, keyring, verifier, None)?;
        let digest = format!(
            "sha256:{}",
            canonical_digest(evidence, Domain::Approval.as_str())?
        );
        if event["approval_id"] != evidence["approval_id"]
            || event["approval_digest"].as_str() != Some(digest.as_str())
            || event["request_digest"] != evidence["request_digest"]
            || event["decision_digest"] != evidence["decision_digest"]
        {
            return fail(
                ErrorCode::LifecycleApprovalMismatch,
                "lifecycle event binds a different approval",
                "$",
            );
        }
    } else if !event["approval_digest"].is_null() {
        return fail(
            ErrorCode::LifecycleApprovalPremature,
            "pre-approval transition cannot claim approval evidence",
            "$",
        );
    }
    // `approval?.use.kind`: only a missing or null approval short-circuits.
    let present = approval.filter(|approval| !approval.is_null());
    if matches!(kind, "reject" | "expire") && present.is_some() {
        let single_use = use_kind(present)?.and_then(Value::as_str) == Some("single_use");
        let disposition = &event["occurrence_disposition"];
        let evidence = present.expect("checked above");
        if single_use
            && (!shape::strict_equal(
                disposition.get("occurrence_id"),
                evidence.get("occurrence_id"),
            ) || !shape::strict_equal(disposition.get("run_id"), evidence.get("run_id")))
        {
            return fail(
                ErrorCode::LifecycleOccurrenceMismatch,
                "no-launch disposition changed the occurrence",
                "$",
            );
        }
    }
    let sequence = event["sequence"].as_f64().unwrap_or(f64::NAN);
    let expected_sequence = state.map_or(1, |state| state.sequence + 1);
    if sequence != expected_sequence as f64 {
        return fail(
            ErrorCode::LifecycleReplay,
            "event sequence is stale or skipped",
            "$",
        );
    }
    let expected_state = state.map_or("required", |state| state.state.as_str());
    if let Some(state) = state {
        if event["approval_id"].as_str() != Some(state.approval_id.as_str())
            || event["request_digest"].as_str() != Some(state.request_digest.as_str())
            || event["decision_digest"].as_str() != Some(state.decision_digest.as_str())
        {
            return fail(
                ErrorCode::LifecycleSubjectMismatch,
                "event changes the approval subject",
                "$",
            );
        }
    }
    if from_state != expected_state {
        return fail(
            ErrorCode::LifecycleStateForged,
            "event from_state is not current",
            "$",
        );
    }
    let previous = event["previous_event_digest"].as_str();
    if previous != state.map(|state| state.last_event_digest.as_str()) {
        return fail(
            ErrorCode::LifecycleChainMismatch,
            "event previous digest does not match append-only head",
            "$",
        );
    }
    let to_state = event["to_state"].as_str().unwrap_or_default();
    let allowed = if kind == "consume" {
        let target = if use_kind(present)?.and_then(Value::as_str) == Some("recurring") {
            "approved"
        } else {
            "consumed"
        };
        to_state == target
    } else {
        TRANSITIONS
            .iter()
            .any(|(from, via, to)| *from == from_state && *via == kind && *to == to_state)
    };
    if !allowed {
        return fail(
            ErrorCode::LifecycleTransitionInvalid,
            "approval lifecycle transition is not allowed",
            "$",
        );
    }
    let previous_count = state.map_or(0, |state| state.consumption_count);
    let consumption_count = previous_count + u64::from(kind == "consume");
    let mut consumed_occurrences = state
        .map(|state| state.consumed_occurrences.clone())
        .unwrap_or_default();
    if kind == "consume" {
        let consumption = &event["consumption"];
        let key = canonicalize(consumption)?;
        if consumed_occurrences.contains(&key) {
            return fail(
                ErrorCode::ApprovalReplayed,
                "approval already consumed for this exact occurrence/run",
                "$",
            );
        }
        // The use and its kind were read above, so neither is missing now.
        let evidence = present.expect("approval.use.kind was read");
        let use_ = &evidence["use"];
        let use_kind = use_["kind"].as_str();
        if use_kind == Some("single_use") && previous_count > 0 {
            return fail(
                ErrorCode::ApprovalReplayed,
                "single-use approval was already consumed",
                "$",
            );
        }
        if use_kind == Some("single_use") {
            let mut expected = Map::new();
            let mut undefined = false;
            for name in CONSUMPTION_FIELDS {
                match evidence.get(name) {
                    Some(value) => {
                        expected.insert(name.to_owned(), value.clone());
                    }
                    None => undefined = true,
                }
            }
            // `canonicalize` checks I-JSON over the whole object before it
            // reaches an undefined member, where it throws a `TypeError`.
            let expected = canonicalize(&Value::Object(expected))?;
            if undefined {
                return fail(
                    ErrorCode::SchemaType,
                    "cannot canonicalize an undefined approval member",
                    "$.approval",
                );
            }
            if key != expected {
                return fail(
                    ErrorCode::ApprovalConsumptionMismatch,
                    "single-use consumption changed the approved operation",
                    "$",
                );
            }
        }
        if use_kind == Some("recurring") {
            // `String.prototype.startsWith` coerces its argument to a string.
            let prefix = shape::js_string(use_.get("occurrence_prefix"));
            if !consumption["occurrence_id"]
                .as_str()
                .unwrap_or_default()
                .starts_with(prefix.as_str())
            {
                return fail(
                    ErrorCode::ApprovalOccurrenceOutOfScope,
                    "recurring approval occurrence is outside its pattern",
                    "$",
                );
            }
            if consumption_count as f64 > shape::js_number(use_.get("max_uses")) {
                return fail(
                    ErrorCode::ApprovalUsageExhausted,
                    "recurring approval usage bound exceeded",
                    "$",
                );
            }
        }
        consumed_occurrences.push(key);
    }
    let mut event_ids = state
        .map(|state| state.event_ids.clone())
        .unwrap_or_default();
    event_ids.push(event["event_id"].as_str().unwrap_or_default().to_owned());
    Ok(LifecycleState {
        approval_id: event["approval_id"].as_str().unwrap_or_default().to_owned(),
        request_digest: event["request_digest"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        decision_digest: event["decision_digest"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        state: to_state.to_owned(),
        sequence: expected_sequence,
        last_event_digest: format!(
            "sha256:{}",
            canonical_digest(event, Domain::ApprovalEvent.as_str())?
        ),
        consumption_count,
        consumed_occurrences,
        event_ids,
    })
}

/// `verifyLifecycleChain`: a non-empty chain of events applied in order from
/// the start, against `approval` validated at `now` when given.
///
/// # Errors
///
/// `schema_type` or `schema_min_items` for a malformed chain, an approval
/// validation code, or the first event's refusal.
pub fn verify_lifecycle_chain(
    events: &Value,
    approval: Option<&Value>,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<LifecycleState> {
    let events = shape::array(Some(events), "$.lifecycle_events", true, false)?;
    validate_approval(approval.unwrap_or(&Value::Null), keyring, verifier, now)?;
    let mut state: Option<LifecycleState> = None;
    for event in events {
        state = Some(apply_lifecycle_event(
            state.as_ref(),
            event,
            approval,
            keyring,
            verifier,
        )?);
    }
    Ok(state.expect("the chain is not empty"))
}
