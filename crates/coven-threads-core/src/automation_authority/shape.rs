//! Shape checks with the reference validator's codes and semantics
//! (`requireObject`, `required`, `closed`, `requireString`, …).
//!
//! A missing member is `None`, as `undefined` is in JavaScript.

use serde_json::{Map, Value};

use super::{fail, AuthorityResult, ErrorCode};

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `requireObject`.
pub(crate) fn object<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> AuthorityResult<&'a Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        _ => fail(ErrorCode::SchemaType, "expected object", path),
    }
}

/// `required`: each name must be an own member. A non-object has none.
pub(crate) fn required(value: Option<&Value>, names: &[&str], path: &str) -> AuthorityResult<()> {
    for name in names {
        if !value
            .and_then(Value::as_object)
            .is_some_and(|object| object.contains_key(*name))
        {
            return fail(
                ErrorCode::SchemaRequired,
                format!("missing required field {name}"),
                path,
            );
        }
    }
    Ok(())
}

/// `closed`: an object with no member outside `allowed`.
pub(crate) fn closed(value: Option<&Value>, allowed: &[&str], path: &str) -> AuthorityResult<()> {
    let object = object(value, path)?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return fail(
            ErrorCode::SchemaUnknownField,
            format!("unknown field {key}"),
            format!("{path}.{key}"),
        );
    }
    Ok(())
}

/// `required` then `closed`, as the reference pairs them.
pub(crate) fn exact_fields(
    value: Option<&Value>,
    names: &[&str],
    path: &str,
) -> AuthorityResult<()> {
    required(value, names, path)?;
    closed(value, names, path)
}

/// `requireString`: a non-empty string.
pub(crate) fn string<'a>(value: Option<&'a Value>, path: &str) -> AuthorityResult<&'a str> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => Ok(text),
        _ => fail(ErrorCode::SchemaType, "expected non-empty string", path),
    }
}

/// `requireInteger`: a safe integer at least `min`.
pub(crate) fn integer(value: Option<&Value>, path: &str, min: i64) -> AuthorityResult<i64> {
    match value.and_then(Value::as_f64) {
        Some(number)
            if number.fract() == 0.0
                && number.abs() <= MAX_SAFE_INTEGER
                && number >= min as f64 =>
        {
            Ok(number as i64)
        }
        _ => fail(
            ErrorCode::SchemaInteger,
            format!("expected safe integer >= {min}"),
            path,
        ),
    }
}

/// `requireBoolean`.
pub(crate) fn boolean(value: Option<&Value>, path: &str) -> AuthorityResult<bool> {
    match value {
        Some(Value::Bool(flag)) => Ok(*flag),
        _ => fail(ErrorCode::SchemaType, "expected boolean", path),
    }
}

/// `requireEnum`: one of `allowed`, compared as strings.
pub(crate) fn enumeration<'a>(
    value: Option<&'a Value>,
    allowed: &[&str],
    path: &str,
) -> AuthorityResult<&'a str> {
    match value.and_then(Value::as_str) {
        Some(text) if allowed.contains(&text) => Ok(text),
        _ => fail(
            ErrorCode::SchemaEnum,
            format!("expected one of {}", allowed.join(", ")),
            path,
        ),
    }
}

/// `exact`: the string `expected`, or `code`.
pub(crate) fn exact(
    value: Option<&Value>,
    expected: &str,
    code: ErrorCode,
    path: &str,
) -> AuthorityResult<()> {
    if value.and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        fail(code, format!("expected {expected}"), path)
    }
}

/// `requireArray`. `unique` follows JavaScript's `Set`: primitives compare by
/// value, objects and arrays never compare equal.
pub(crate) fn array<'a>(
    value: Option<&'a Value>,
    path: &str,
    nonempty: bool,
    unique: bool,
) -> AuthorityResult<&'a Vec<Value>> {
    let Some(Value::Array(items)) = value else {
        return fail(ErrorCode::SchemaType, "expected array", path);
    };
    if nonempty && items.is_empty() {
        return fail(ErrorCode::SchemaMinItems, "array must not be empty", path);
    }
    if unique {
        for (index, item) in items.iter().enumerate() {
            if items[..index]
                .iter()
                .any(|earlier| same_primitive(earlier, item))
            {
                return fail(
                    ErrorCode::SchemaDuplicateItem,
                    "array items must be unique",
                    path,
                );
            }
        }
    }
    Ok(items)
}

fn same_primitive(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        _ => false,
    }
}

/// JavaScript `===` on two members, for the strings and numbers the profile
/// compares. Two absent members are equal; objects never are.
pub(crate) fn strict_equal(left: Option<&Value>, right: Option<&Value>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => same_primitive(left, right),
        _ => false,
    }
}

/// `exact` against a non-string constant, such as `null` or `false`: the
/// member must be `===` to `expected`.
pub(crate) fn exact_value(
    value: Option<&Value>,
    expected: &Value,
    code: ErrorCode,
    path: &str,
) -> AuthorityResult<()> {
    if strict_equal(value, Some(expected)) {
        Ok(())
    } else {
        fail(code, format!("expected {expected}"), path)
    }
}

/// JavaScript truthiness of a member: `undefined`, `null`, `false`, `0`, and
/// `""` are falsy; every object and array is truthy.
pub(crate) fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|number| number != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// JavaScript `String(value)` for a member: `undefined` is `"undefined"`, an
/// array joins its items with commas (a `null` item as nothing), and an
/// object is `"[object Object]"`.
pub(crate) fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_owned(),
        Some(Value::Null) => "null".to_owned(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => {
            super::canonical::ecmascript_number(number.as_f64().unwrap_or(f64::NAN))
        }
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                item => js_string(Some(item)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_owned(),
    }
}

/// JavaScript `Number(value)` for a member, as a relational comparison
/// coerces it: `undefined` and objects are `NaN`, `null` is `0`, and strings
/// and arrays go through `StringToNumber`.
///
/// Radix literals accumulate in a double, so a value beyond 2^53 may round
/// differently from JavaScript; the profile only compares the result with
/// small counts.
pub(crate) fn js_number(value: Option<&Value>) -> f64 {
    match value {
        None | Some(Value::Object(_)) => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        Some(Value::Number(number)) => number.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(text)) => string_to_number(text),
        Some(array @ Value::Array(_)) => string_to_number(&js_string(Some(array))),
    }
}

/// ECMAScript `StringToNumber`.
fn string_to_number(text: &str) -> f64 {
    let is_space = |ch: char| {
        matches!(
            ch,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    };
    let text = text.trim_matches(is_space);
    if text.is_empty() {
        return 0.0;
    }
    let radix = match text.get(..2) {
        Some("0x" | "0X") => 16,
        Some("0o" | "0O") => 8,
        Some("0b" | "0B") => 2,
        _ => 10,
    };
    if radix != 10 {
        let digits = &text[2..];
        if digits.is_empty() {
            return f64::NAN;
        }
        return digits
            .chars()
            .try_fold(0.0, |sum, ch| {
                ch.to_digit(radix)
                    .map(|digit| sum * f64::from(radix) + f64::from(digit))
            })
            .unwrap_or(f64::NAN);
    }
    let (negative, unsigned) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let magnitude = if unsigned == "Infinity" {
        f64::INFINITY
    } else if is_decimal_literal(unsigned) {
        unsigned.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    };
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// `StrUnsignedDecimalLiteral` without `Infinity`: digits with an optional
/// fraction, or a fraction alone, then an optional exponent.
fn is_decimal_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes
            .get(from..)
            .unwrap_or_default()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count()
    };
    let integer = digits(0);
    let mut at = integer;
    let mut fraction = 0;
    if bytes.get(at) == Some(&b'.') {
        fraction = digits(at + 1);
        at += 1 + fraction;
    }
    if integer == 0 && fraction == 0 {
        return false;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let exponent = digits(at);
        if exponent == 0 {
            return false;
        }
        at += exponent;
    }
    at == bytes.len()
}

/// The length JavaScript reports for a string: its UTF-16 code units.
pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `requireDigestHex`: lowercase SHA-256 hex, with a `sha256:` prefix when
/// `prefixed`.
pub(crate) fn digest_hex<'a>(
    value: Option<&'a Value>,
    path: &str,
    prefixed: bool,
) -> AuthorityResult<&'a str> {
    let text = string(value, path)?;
    let hex = if prefixed {
        text.strip_prefix("sha256:")
    } else {
        Some(text)
    };
    if hex.is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }) {
        Ok(text)
    } else {
        fail(
            ErrorCode::SchemaDigest,
            "expected lowercase SHA-256 digest",
            path,
        )
    }
}

/// `requireTimestamp`: an RFC 3339 UTC calendar instant with `Z` and any
/// number of fraction digits. Returns `Date.parse`'s milliseconds, which keep
/// only the first three fraction digits.
pub(crate) fn timestamp(value: Option<&Value>, path: &str) -> AuthorityResult<i64> {
    let refused = |message: &str| fail(ErrorCode::SchemaTimestamp, message.to_owned(), path);
    let Some(Value::String(text)) =
        value.filter(|value| value.as_str().is_some_and(|t| !t.is_empty()))
    else {
        return refused("expected an RFC 3339 UTC timestamp");
    };
    let Some(parts) = parse_timestamp(text) else {
        return refused("expected an RFC 3339 UTC timestamp");
    };
    let [year, month, day, hour, minute, second, millis] = parts;
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return refused("expected a real RFC 3339 UTC calendar instant");
    }
    Ok(days_from_civil(year, month, day) * 86_400_000
        + hour * 3_600_000
        + minute * 60_000
        + second * 1_000
        + millis)
}

/// `Date.parse` of a timestamp already accepted by [`timestamp`].
pub(crate) fn millis(value: Option<&Value>) -> i64 {
    timestamp(value, "$").unwrap_or(i64::MIN)
}

/// `^(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(?:\.(\d+))?Z$`, with ASCII
/// digits, as `[year, month, day, hour, minute, second, millis]`.
fn parse_timestamp(text: &str) -> Option<[i64; 7]> {
    let bytes = text.as_bytes();
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = bytes.get(range)?;
        digits.iter().all(u8::is_ascii_digit).then(|| {
            digits
                .iter()
                .fold(0, |sum, digit| sum * 10 + i64::from(digit - b'0'))
        })
    };
    let at = |index: usize, expected: u8| bytes.get(index) == Some(&expected);
    if !(at(4, b'-') && at(7, b'-') && at(10, b'T') && at(13, b':') && at(16, b':')) {
        return None;
    }
    let mut millis = 0;
    let mut index = 19;
    if at(19, b'.') {
        let start = 20;
        let mut end = start;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == start {
            return None;
        }
        let kept = &text[start..end.min(start + 3)];
        millis = kept.parse::<i64>().ok()? * 10_i64.pow(3 - kept.len() as u32);
        index = end;
    }
    if !(at(index, b'Z') && index + 1 == bytes.len()) {
        return None;
    }
    Some([
        number(0..4)?,
        number(5..7)?,
        number(8..10)?,
        number(11..13)?,
        number(14..16)?,
        number(17..19)?,
        millis,
    ])
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn timestamps_match_date_parse() {
        // Expected values are `Date.parse(value)` in Node 24.
        for (text, expected) in [
            ("2026-10-04T00:00:00.123456789Z", 1_791_072_000_123),
            ("2026-10-04T00:00:00.5Z", 1_791_072_000_500),
            ("2026-10-04T00:00:00.9999Z", 1_791_072_000_999),
            ("2026-10-04T00:00:00Z", 1_791_072_000_000),
            ("0099-01-01T00:00:00Z", -59_042_995_200_000),
            ("0000-01-01T00:00:00Z", -62_167_219_200_000),
            ("2024-02-29T23:59:59Z", 1_709_251_199_000),
        ] {
            assert_eq!(timestamp(Some(&json!(text)), "$"), Ok(expected), "{text}");
        }
        for text in [
            "2026-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:60Z",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00.Z",
            "2026-01-01 00:00:00Z",
            "",
        ] {
            assert!(timestamp(Some(&json!(text)), "$").is_err(), "{text}");
        }
        assert!(timestamp(None, "$").is_err());
        assert!(timestamp(Some(&json!(1)), "$").is_err());
    }

    #[test]
    fn coercions_match_javascript() {
        // Expected values are `Boolean(v)`, `String(v)` and `Number(v)` in
        // Node 24.
        for (value, truth) in [
            (None, false),
            (Some(json!(null)), false),
            (Some(json!(false)), false),
            (Some(json!(0)), false),
            (Some(json!(-0.0)), false),
            (Some(json!("")), false),
            (Some(json!("0")), true),
            (Some(json!(0.5)), true),
            (Some(json!([])), true),
            (Some(json!({})), true),
        ] {
            assert_eq!(truthy(value.as_ref()), truth, "{value:?}");
        }
        for (value, text) in [
            (None, "undefined"),
            (Some(json!(null)), "null"),
            (Some(json!(true)), "true"),
            (Some(json!(1.5)), "1.5"),
            (Some(json!(100)), "100"),
            (Some(json!("ab")), "ab"),
            (Some(json!(["a", null, 1, ["b", "c"]])), "a,,1,b,c"),
            (Some(json!([])), ""),
            (Some(json!({"a": 1})), "[object Object]"),
        ] {
            assert_eq!(js_string(value.as_ref()), text, "{value:?}");
        }
        for (value, number) in [
            (Some(json!(null)), 0.0),
            (Some(json!(true)), 1.0),
            (Some(json!(7)), 7.0),
            (Some(json!("")), 0.0),
            (Some(json!(" 12 ")), 12.0),
            (Some(json!("\u{a0}12\u{feff}")), 12.0),
            (Some(json!("1.")), 1.0),
            (Some(json!(".5")), 0.5),
            (Some(json!("-1e3")), -1000.0),
            (Some(json!("0x10")), 16.0),
            (Some(json!("0o17")), 15.0),
            (Some(json!("0b11")), 3.0),
            (Some(json!("-Infinity")), f64::NEG_INFINITY),
            (Some(json!([])), 0.0),
            (Some(json!([3])), 3.0),
            (Some(json!(["4"])), 4.0),
        ] {
            assert_eq!(js_number(value.as_ref()), number, "{value:?}");
        }
        for value in [
            None,
            Some(json!({})),
            Some(json!("x")),
            Some(json!("-0x10")),
            Some(json!("1_0")),
            Some(json!("infinity")),
            Some(json!("\u{85}1")),
            Some(json!("1e")),
            Some(json!(".")),
            Some(json!("0x")),
            Some(json!([1, 2])),
        ] {
            assert!(js_number(value.as_ref()).is_nan(), "{value:?}");
        }
        assert_eq!(utf16_len("ab\u{1f600}"), 4);
        assert!(exact_value(Some(&json!(null)), &json!(null), ErrorCode::SchemaType, "$").is_ok());
        assert!(exact_value(None, &json!(null), ErrorCode::SchemaType, "$").is_err());
        assert!(exact_value(Some(&json!(0)), &json!(false), ErrorCode::SchemaType, "$").is_err());
    }

    #[test]
    fn arrays_are_unique_as_javascript_sets_are() {
        assert!(array(Some(&json!(["a", "b"])), "$", true, true).is_ok());
        assert!(array(Some(&json!(["a", "a"])), "$", true, true).is_err());
        assert!(array(Some(&json!([1, 1.0])), "$", true, true).is_err());
        // Distinct objects never collide, even when equal.
        assert!(array(Some(&json!([{}, {}])), "$", true, true).is_ok());
        assert_eq!(
            array(Some(&json!([])), "$", true, false).unwrap_err().code,
            ErrorCode::SchemaMinItems
        );
    }
}
