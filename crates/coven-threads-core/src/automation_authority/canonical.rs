//! Canonical bytes and domain-separated digests (`canonicalize` and
//! `canonicalDigest` in the reference).
//!
//! Object keys sort by UTF-16 code unit; strings and numbers print as
//! `JSON.stringify` prints them. A digest is
//! `SHA-256(UTF-8(domain) || 0x00 || canonical(artifact without integrity))`,
//! as lowercase hex.

use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::{fail, AuthorityResult, ErrorCode};

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// The profile's digest domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// `opencoven:automation-request:v1`
    Request,
    /// `opencoven:automation-decision:v1`
    Decision,
    /// `opencoven:automation-approval:v1`
    Approval,
    /// `opencoven:automation-approval-event:v1`
    ApprovalEvent,
    /// `opencoven:automation-consumption-snapshot:v1`
    ConsumptionSnapshot,
    /// `opencoven:automation-proposal:v1`
    Proposal,
    /// `opencoven:automation-evidence-read:v1`
    EvidenceRead,
}

impl Domain {
    /// The domain string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Request => "opencoven:automation-request:v1",
            Self::Decision => "opencoven:automation-decision:v1",
            Self::Approval => "opencoven:automation-approval:v1",
            Self::ApprovalEvent => "opencoven:automation-approval-event:v1",
            Self::ConsumptionSnapshot => "opencoven:automation-consumption-snapshot:v1",
            Self::Proposal => "opencoven:automation-proposal:v1",
            Self::EvidenceRead => "opencoven:automation-evidence-read:v1",
        }
    }
}

/// The canonical text of `value`.
///
/// # Errors
///
/// `json_unsafe_integer` for an integer-valued number beyond 2^53 - 1, as the
/// reference's I-JSON check raises before canonicalizing.
pub fn canonicalize(value: &Value) -> AuthorityResult<String> {
    write_canonical(value, false)
}

/// `canonicalDigest`: the lowercase hex SHA-256 of `domain`, a zero byte, and
/// the canonical text of `value` without its top-level `integrity` member.
///
/// # Errors
///
/// `integrity_domain_invalid` for an empty domain, or a canonicalization
/// error.
pub fn canonical_digest(value: &Value, domain: &str) -> AuthorityResult<String> {
    if domain.is_empty() {
        return fail(
            ErrorCode::IntegrityDomainInvalid,
            "digest domain must be a non-empty string",
            "$",
        );
    }
    let canonical = write_canonical(value, true)?;
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0_u8]);
    hasher.update(canonical.as_bytes());
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

enum Step<'a> {
    Value(&'a Value),
    Key(&'a str),
    Text(char),
}

/// Writes `value` canonically, leaving out a top-level `integrity` member when
/// `unsigned`, without recursion.
fn write_canonical(root: &Value, unsigned: bool) -> AuthorityResult<String> {
    check_ijson(root)?;
    let mut out = String::new();
    let mut steps = vec![Step::Value(root)];
    let mut first = true;
    while let Some(step) = steps.pop() {
        let value = match step {
            Step::Text(text) => {
                out.push(text);
                continue;
            }
            Step::Key(key) => {
                write_string(key, &mut out);
                out.push(':');
                continue;
            }
            Step::Value(value) => value,
        };
        let top = std::mem::replace(&mut first, false);
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
            Value::Number(number) => {
                out.push_str(&ecmascript_number(number.as_f64().unwrap_or(f64::NAN)));
            }
            Value::String(text) => write_string(text, &mut out),
            Value::Array(items) => {
                out.push('[');
                steps.push(Step::Text(']'));
                for (index, item) in items.iter().enumerate().rev() {
                    steps.push(Step::Value(item));
                    if index > 0 {
                        steps.push(Step::Text(','));
                    }
                }
            }
            Value::Object(object) => {
                let mut keys: Vec<&String> = object
                    .keys()
                    .filter(|key| !(top && unsigned && key.as_str() == "integrity"))
                    .collect();
                keys.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
                out.push('{');
                steps.push(Step::Text('}'));
                for (index, key) in keys.into_iter().enumerate().rev() {
                    steps.push(Step::Value(&object[key]));
                    steps.push(Step::Key(key));
                    if index > 0 {
                        steps.push(Step::Text(','));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The reference's `validateIJsonValue` for values already in Rust: strings
/// are always Unicode scalar values here, so only numbers can fail.
fn check_ijson(root: &Value) -> AuthorityResult<()> {
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        match value {
            Value::Number(number) => {
                let float = number.as_f64().unwrap_or(f64::NAN);
                if !float.is_finite() {
                    return fail(ErrorCode::JsonNonIjson, "number must be finite", "$");
                }
                if float.fract() == 0.0 && float.abs() > MAX_SAFE_INTEGER {
                    return fail(
                        ErrorCode::JsonUnsafeInteger,
                        "integer exceeds the interoperable I-JSON range",
                        "$",
                    );
                }
            }
            Value::Array(items) => pending.extend(items),
            Value::Object(object) => pending.extend(object.values()),
            _ => {}
        }
    }
    Ok(())
}

/// `JSON.stringify` for a string: only `"`, `\` and C0 controls escape.
fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0}'..='\u{1f}' => out.push_str(&format!("\\u{:04x}", u32::from(ch))),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

/// ECMAScript `Number::toString` for a finite double, from the shortest
/// round-trip digits Rust's formatter produces.
pub(crate) fn ecmascript_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("Rust formats an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let exponent: i64 = exponent.parse().expect("an integer exponent");
    // ECMAScript names the decimal point position `n`: value = 0.digits * 10^n.
    let k = digits.len() as i64;
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let tail = if k == 1 {
            String::new()
        } else {
            format!(".{}", &digits[1..])
        };
        format!("{}{tail}e{sign}{}", &digits[..1], (n - 1).abs())
    };
    if value < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_print_as_ecmascript_does() {
        // Expected strings are `String(value)` in Node.
        for (value, expected) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (100.0, "100"),
            (0.5, "0.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (1e-7, "1e-7"),
            (0.000_001, "0.000001"),
            (123.456, "123.456"),
            (5e-324, "5e-324"),
            (1.797_693_134_862_315_7e308, "1.7976931348623157e+308"),
            (9_007_199_254_740_991.0, "9007199254740991"),
            (-42.0, "-42"),
            (1.5e-10, "1.5e-10"),
            (-2.5e30, "-2.5e+30"),
        ] {
            assert_eq!(ecmascript_number(value), expected, "{value:e}");
        }
    }

    #[test]
    fn canonical_text_sorts_keys_by_utf16_and_drops_top_level_integrity() {
        let value = json!({"b": 1, "a": {"z": [true, null, "x\n"], "integrity": 1}, "\u{1f600}": 2, "\u{ff61}": 3, "integrity": {}});
        // U+1F600 is the surrogate pair D83D DE00, so it sorts before U+FF61.
        assert_eq!(
            canonicalize(&value).unwrap(),
            "{\"a\":{\"integrity\":1,\"z\":[true,null,\"x\\n\"]},\"b\":1,\"integrity\":{},\"\u{1f600}\":2,\"\u{ff61}\":3}"
        );
        assert_eq!(
            canonical_digest(&value, "d").unwrap(),
            canonical_digest(&json!({"b": 1, "a": {"z": [true, null, "x\n"], "integrity": 1}, "\u{1f600}": 2, "\u{ff61}": 3}), "d").unwrap()
        );
        assert_eq!(
            canonical_digest(&json!({}), "").unwrap_err().code,
            ErrorCode::IntegrityDomainInvalid
        );
        assert_eq!(
            canonicalize(&json!([9_007_199_254_740_992_u64]))
                .unwrap_err()
                .code,
            ErrorCode::JsonUnsafeInteger
        );
    }
}
