//! The profile's strict I-JSON parser, ported from `StrictJsonParser`.
//!
//! Errors are raised in the reference's order, because the first one is the
//! one that counts:
//!
//! - a leading BOM is `json_non_ijson`;
//! - while scanning, a syntax fault is `json_invalid`, a repeated key is
//!   `json_duplicate_key` at its second occurrence, a lone surrogate is
//!   `json_non_ijson` at its string, a non-finite number is `json_non_ijson`,
//!   and an integer literal beyond 2^53 - 1 is `json_unsafe_integer`;
//! - once the whole document has parsed, an integer-valued number written
//!   with a fraction or exponent beyond 2^53 - 1 is `json_unsafe_integer`.
//!
//! Numbers become the double JavaScript holds, stored as an exact integer when
//! integer-valued.
//!
//! Nesting is limited to [`MAX_DEPTH`] levels, refused beyond that as
//! `json_invalid`. The reference states no limit, but its recursive parser
//! throws a non-profile `RangeError` at an engine-dependent depth, near 1,750
//! levels on Node 24. The limit sits above that, so everything the reference
//! parses parses here, and it keeps dropping the parsed value, which recurses,
//! well within any thread's stack. Profile documents nest under ten levels.

use serde_json::{Map, Number, Value};

use super::{fail, AuthorityResult, ErrorCode};

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// The deepest nesting accepted.
pub const MAX_DEPTH: usize = 2048;

/// Parses `text` as one strict I-JSON document.
///
/// # Errors
///
/// The profile's `json_*` codes, in the reference parser's order.
pub fn strict_parse_json(text: &str) -> AuthorityResult<Value> {
    if text.starts_with('\u{feff}') {
        return fail(
            ErrorCode::JsonNonIjson,
            "I-JSON input must not contain a BOM",
            "$",
        );
    }
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        at: 0,
        open: Vec::new(),
        unsafe_number: false,
    };
    let value = parser.document()?;
    if parser.unsafe_number {
        return fail(
            ErrorCode::JsonUnsafeInteger,
            "integer exceeds the interoperable I-JSON range",
            "$",
        );
    }
    Ok(value)
}

/// The double JavaScript holds for `value`, as an exact integer when it is
/// integer-valued and small enough to convert exactly.
pub(crate) fn js_number(value: f64) -> Option<Number> {
    if !value.is_finite() {
        return None;
    }
    if value.fract() == 0.0 && value.abs() < 9_223_372_036_854_775_808.0 {
        return Some(if value >= 0.0 {
            Number::from(value as u64)
        } else {
            Number::from(value as i64)
        });
    }
    Number::from_f64(value)
}

enum Frame {
    Array(Vec<Value>),
    Object {
        map: Map<String, Value>,
        key: String,
    },
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    open: Vec<Frame>,
    /// An integer-valued number beyond the safe range, written with a fraction
    /// or exponent: refused once the document has parsed.
    unsafe_number: bool,
}

impl Parser<'_> {
    fn invalid<T>(&self, message: &str) -> AuthorityResult<T> {
        fail(
            ErrorCode::JsonInvalid,
            format!("{message} at byte {}", self.at),
            "$",
        )
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    fn document(&mut self) -> AuthorityResult<Value> {
        loop {
            self.whitespace();
            let Some(mut value) = self.value()? else {
                continue;
            };
            loop {
                self.whitespace();
                match self.open.last_mut() {
                    None => {
                        if self.at != self.bytes.len() {
                            return self.invalid("trailing data after JSON value");
                        }
                        return Ok(value);
                    }
                    Some(Frame::Array(items)) => {
                        items.push(value);
                        match self.peek() {
                            Some(b',') => {
                                self.at += 1;
                                break;
                            }
                            Some(b']') => {
                                self.at += 1;
                                let Some(Frame::Array(items)) = self.open.pop() else {
                                    unreachable!("the innermost frame is an array");
                                };
                                value = Value::Array(items);
                            }
                            _ => return self.invalid("missing comma in array"),
                        }
                    }
                    Some(Frame::Object { map, key }) => {
                        map.insert(std::mem::take(key), value);
                        match self.peek() {
                            Some(b',') => {
                                self.at += 1;
                                self.member_key()?;
                                break;
                            }
                            Some(b'}') => {
                                self.at += 1;
                                let Some(Frame::Object { map, .. }) = self.open.pop() else {
                                    unreachable!("the innermost frame is an object");
                                };
                                value = Value::Object(map);
                            }
                            _ => return self.invalid("missing comma in object"),
                        }
                    }
                }
            }
        }
    }

    /// A scalar or empty container as a finished value; a non-empty container
    /// opens a frame and yields `None`.
    fn value(&mut self) -> AuthorityResult<Option<Value>> {
        // Every container counts toward the depth, empty ones included.
        if matches!(self.peek(), Some(b'{' | b'[')) && self.open.len() >= MAX_DEPTH {
            return self.invalid("nesting is too deep");
        }
        match self.peek() {
            Some(b'{') => {
                self.at += 1;
                self.whitespace();
                if self.peek() == Some(b'}') {
                    self.at += 1;
                    return Ok(Some(Value::Object(Map::new())));
                }
                self.open.push(Frame::Object {
                    map: Map::new(),
                    key: String::new(),
                });
                self.member_key()?;
                Ok(None)
            }
            Some(b'[') => {
                self.at += 1;
                self.whitespace();
                if self.peek() == Some(b']') {
                    self.at += 1;
                    return Ok(Some(Value::Array(Vec::new())));
                }
                self.open.push(Frame::Array(Vec::new()));
                Ok(None)
            }
            Some(b'"') => self.string().map(|text| Some(Value::String(text))),
            Some(b't') if self.bytes[self.at..].starts_with(b"true") => {
                self.at += 4;
                Ok(Some(Value::Bool(true)))
            }
            Some(b'f') if self.bytes[self.at..].starts_with(b"false") => {
                self.at += 5;
                Ok(Some(Value::Bool(false)))
            }
            Some(b'n') if self.bytes[self.at..].starts_with(b"null") => {
                self.at += 4;
                Ok(Some(Value::Null))
            }
            Some(b'-' | b'0'..=b'9') => self.number().map(Some),
            _ => self.invalid("unexpected token"),
        }
    }

    /// Reads `"key":` for the innermost object.
    fn member_key(&mut self) -> AuthorityResult<()> {
        self.whitespace();
        if self.peek() != Some(b'"') {
            return self.invalid("object key must be a string");
        }
        let key = self.string()?;
        let Some(Frame::Object { map, key: slot }) = self.open.last_mut() else {
            unreachable!("a key is read only inside an object");
        };
        if map.contains_key(&key) {
            return fail(
                ErrorCode::JsonDuplicateKey,
                format!("duplicate object key {key:?}"),
                "$",
            );
        }
        // JavaScript drops `__proto__` silently; refusing it keeps the parsed
        // value equal to the document.
        if key == "__proto__" {
            return fail(
                ErrorCode::JsonInvalid,
                "an object key named __proto__ is not portable",
                "$",
            );
        }
        // Reserve the key now, so a later duplicate is caught before its value.
        map.insert(key.clone(), Value::Null);
        *slot = key;
        self.whitespace();
        if self.peek() != Some(b':') {
            return self.invalid("missing colon after object key");
        }
        self.at += 1;
        Ok(())
    }

    fn number(&mut self) -> AuthorityResult<Value> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return self.invalid("invalid JSON number"),
        }
        let mut integer_literal = true;
        if self.peek() == Some(b'.') && self.bytes.get(self.at + 1).is_some_and(u8::is_ascii_digit)
        {
            integer_literal = false;
            self.at += 1;
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mut end = self.at + 1;
            if matches!(self.bytes.get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            if self.bytes.get(end).is_some_and(u8::is_ascii_digit) {
                integer_literal = false;
                self.at = end;
                self.digits();
            }
        }
        let token = &self.text[start..self.at];
        let value: f64 =
            token
                .parse()
                .map_err(|_| crate::automation_authority::AuthorityError {
                    code: ErrorCode::JsonInvalid,
                    message: "invalid JSON number".to_owned(),
                    path: "$".to_owned(),
                })?;
        if !value.is_finite() {
            return fail(
                ErrorCode::JsonNonIjson,
                "non-finite number is not I-JSON",
                "$",
            );
        }
        let unsafe_integer = value.fract() == 0.0 && value.abs() > MAX_SAFE_INTEGER;
        if unsafe_integer && integer_literal {
            return fail(
                ErrorCode::JsonUnsafeInteger,
                "integer exceeds the interoperable I-JSON range",
                "$",
            );
        }
        self.unsafe_number |= unsafe_integer;
        Ok(Value::Number(js_number(value).expect("a finite number")))
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
    }

    /// One string: a raw control character or bad escape is `json_invalid`,
    /// then a lone surrogate is `json_non_ijson`.
    fn string(&mut self) -> AuthorityResult<String> {
        self.at += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let Some(byte) = self.peek() else {
                return self.invalid("unterminated JSON string");
            };
            match byte {
                b'"' => {
                    self.at += 1;
                    break;
                }
                b'\\' => {
                    self.at += 1;
                    let unit = match self.peek() {
                        Some(b'"') => 0x22,
                        Some(b'\\') => 0x5c,
                        Some(b'/') => 0x2f,
                        Some(b'b') => 0x08,
                        Some(b'f') => 0x0c,
                        Some(b'n') => 0x0a,
                        Some(b'r') => 0x0d,
                        Some(b't') => 0x09,
                        Some(b'u') => {
                            let digits = self
                                .text
                                .get(self.at + 1..self.at + 5)
                                .filter(|digits| digits.bytes().all(|b| b.is_ascii_hexdigit()));
                            let Some(digits) = digits else {
                                return self.invalid("invalid JSON string");
                            };
                            self.at += 4;
                            u16::from_str_radix(digits, 16).expect("four hex digits")
                        }
                        _ => return self.invalid("invalid JSON string"),
                    };
                    self.at += 1;
                    units.push(unit);
                }
                0x00..=0x1f => return self.invalid("control character in string"),
                _ => {
                    let ch = self.text[self.at..]
                        .chars()
                        .next()
                        .expect("valid UTF-8 continues here");
                    self.at += ch.len_utf8();
                    let mut buffer = [0_u16; 2];
                    units.extend_from_slice(ch.encode_utf16(&mut buffer));
                }
            }
        }
        String::from_utf16(&units).or_else(|_| {
            fail(
                ErrorCode::JsonNonIjson,
                "unpaired surrogate is not I-JSON",
                "$",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn code(text: &str) -> Option<&'static str> {
        strict_parse_json(text)
            .err()
            .map(|error| error.code.as_str())
    }

    #[test]
    fn parses_strict_json_with_javascript_numbers() {
        assert_eq!(
            strict_parse_json(" {\"a\": [1, 7.0, 1e0, -0, 0.5, true, null, \"\\u00e9\"]} ")
                .unwrap(),
            json!({"a": [1, 7, 1, 0, 0.5, true, null, "é"]})
        );
    }

    #[test]
    fn errors_follow_the_reference_order() {
        assert_eq!(code("\u{feff}{}"), Some("json_non_ijson"));
        assert_eq!(code("{\"a\":1,\"a\":2}"), Some("json_duplicate_key"));
        assert_eq!(code("{\"a\":1,\"\\u0061\":2}"), Some("json_duplicate_key"));
        assert_eq!(code("[\"\\ud800\"]"), Some("json_non_ijson"));
        assert_eq!(code("[\"\\udc00x\"]"), Some("json_non_ijson"));
        assert_eq!(code("[1e400]"), Some("json_non_ijson"));
        assert_eq!(code("[9007199254740992]"), Some("json_unsafe_integer"));
        assert_eq!(code("[-9007199254740992]"), Some("json_unsafe_integer"));
        assert_eq!(code("[9007199254740991, -9007199254740991]"), None);
        // An exponent-form unsafe integer is refused only after the document
        // parses, so an earlier syntax fault wins.
        assert_eq!(code("[1e16]"), Some("json_unsafe_integer"));
        assert_eq!(code("[1e16, ]"), Some("json_invalid"));
        assert_eq!(code("[1.5e300]"), Some("json_unsafe_integer"));
        // A lone surrogate is caught at its string, before a later duplicate.
        assert_eq!(
            code("{\"\\ud800\":1,\"a\":1,\"a\":2}"),
            Some("json_non_ijson")
        );
        assert_eq!(code("{\"__proto__\":{}}"), Some("json_invalid"));
        for invalid in [
            "",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "01",
            "1.",
            ".5",
            "+1",
            "1e",
            "-",
            "tru",
            "\"a",
            "\"\\x\"",
            "\"\\u12\"",
            "\"\t\"",
            "{a:1}",
            "[1 2]",
            "1 2",
            "\u{a0}{}",
            "NaN",
        ] {
            assert_eq!(code(invalid), Some("json_invalid"), "{invalid:?}");
        }
    }

    #[test]
    fn nesting_is_bounded() {
        let nested = |depth: usize| "[".repeat(depth) + &"]".repeat(depth);
        assert!(strict_parse_json(&nested(MAX_DEPTH)).is_ok());
        assert_eq!(code(&nested(MAX_DEPTH + 1)), Some("json_invalid"));
        // Far deeper input is refused without exhausting the stack.
        assert_eq!(code(&nested(1_000_000)), Some("json_invalid"));
    }
}
