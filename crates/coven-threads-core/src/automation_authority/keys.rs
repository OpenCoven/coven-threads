//! Keyrings, signatures and signing (`normalizeKeyring`,
//! `verifySignedArtifact` and `signArtifact` in the reference).
//!
//! An artifact's `integrity` is `{alg: "ed25519", key_id, signed_digest,
//! signature_b64}`: the signature is over the 32 raw bytes of the
//! domain-separated digest, in canonical padded base64.

use serde_json::{json, Map, Value};

use super::canonical::canonical_digest;
use super::{fail, shape, AuthorityResult, Domain, ErrorCode};

/// The DER prefix of an Ed25519 SubjectPublicKeyInfo.
#[cfg(test)]
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
const IDENTITY_ROLES: [&str; 3] = ["principal", "protected_owner", "auditor"];

/// Ed25519 verification, supplied by the caller so the core carries no crypto
/// backend.
pub trait SignatureVerifier {
    /// Whether `signature` by `public_key` verifies over `message`.
    fn verify_ed25519(
        &self,
        public_key: &[u8; 32],
        message: &[u8; 32],
        signature: &[u8; 64],
    ) -> bool;
}

/// An Ed25519 signer for one keyring key, supplied by the caller.
pub trait ArtifactSigner {
    /// The keyring id the signature is filed under.
    /// `sign_artifact` reads it once, validates it, and reuses that value.
    fn key_id(&self) -> &str;
    /// The Ed25519 signature over `message`.
    fn sign_ed25519(&self, message: &[u8; 32]) -> [u8; 64];
}

/// Keys trusted to sign profile artifacts, by key id. A record is the
/// reference's `{role, principal_id?, public_key_pem}`; roles are
/// `threads_authority` and the identity roles `principal`, `protected_owner`
/// and `auditor`, which must name the exact principal they speak for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keyring {
    records: Vec<(String, Value)>,
}

impl Keyring {
    /// A keyring from `(key_id, record)` pairs, validated when first used, as
    /// the reference validates on every verification. Record validation follows
    /// supplied order (as a JavaScript Map does). Duplicate ids are refused at
    /// validation before any record can supply authority.
    #[must_use]
    pub fn new(records: Vec<(String, Value)>) -> Self {
        Self { records }
    }

    /// A keyring from a JSON object of records, such as `keyring.json`'s
    /// `keys`. Validation follows this map's iteration order. To retain an
    /// external object's original order, pass its ordered pairs to [`Self::new`]
    /// before converting to a map that may sort keys.
    #[must_use]
    pub fn from_object(keys: &Map<String, Value>) -> Self {
        Self::new(
            keys.iter()
                .map(|(id, record)| (id.clone(), record.clone()))
                .collect(),
        )
    }

    /// The unique record for `key_id`, to adjust in place. An ambiguous id
    /// returns `None`; duplicate ids must be removed explicitly by the caller.
    pub fn record_mut(&mut self, key_id: &str) -> Option<&mut Value> {
        if self.records.iter().filter(|(id, _)| id == key_id).count() != 1 {
            return None;
        }
        self.records
            .iter_mut()
            .find(|(id, _)| id == key_id)
            .map(|(_, record)| record)
    }

    /// `validateKeyring`.
    ///
    /// # Errors
    ///
    /// `json_duplicate_key` for duplicate ids, otherwise the first malformed
    /// record's `integrity_*` or `schema_*` code in supplied iteration order.
    pub fn validate(&self) -> AuthorityResult<()> {
        let mut ids = std::collections::BTreeSet::new();
        for (id, _) in &self.records {
            if !ids.insert(id) {
                return fail(
                    ErrorCode::JsonDuplicateKey,
                    "duplicate keyring id",
                    "$.keyring",
                );
            }
        }
        for (key_id, record) in &self.records {
            if key_id.is_empty() {
                return fail(
                    ErrorCode::SchemaType,
                    "expected non-empty string",
                    "$.keyring.key_id",
                );
            }
            let path = format!("$.keyring.{key_id}");
            let object = shape::object(Some(record), &path)?;
            let role = shape::string(object.get("role"), &format!("{path}.role"))?;
            if role != "threads_authority" && !IDENTITY_ROLES.contains(&role) {
                return fail(
                    ErrorCode::IntegrityRoleUnknown,
                    format!("unknown keyring role {role}"),
                    format!("{path}.role"),
                );
            }
            let principal = object.get("principal_id");
            if IDENTITY_ROLES.contains(&role)
                && principal.and_then(Value::as_str).is_none_or(str::is_empty)
            {
                return fail(
                    ErrorCode::IntegrityPrincipalIdMissing,
                    format!("{role} key {key_id} must bind an exact principal_id"),
                    format!("{path}.principal_id"),
                );
            }
            if role == "threads_authority" && principal.is_some() {
                return fail(
                    ErrorCode::IntegrityPrincipalIdUnexpected,
                    format!("Threads authority key {key_id} cannot carry a principal_id"),
                    format!("{path}.principal_id"),
                );
            }
        }
        Ok(())
    }

    fn get(&self, key_id: &str) -> Option<&Value> {
        self.records
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, record)| record)
    }
}

/// The key that verified an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedKey {
    pub role: String,
    pub principal_id: Option<String>,
}

/// `verifySignedArtifact`, in the reference's order of checks.
pub(crate) fn verify_signed_artifact(
    value: &Value,
    domain: Domain,
    keyring: &Keyring,
    expected_role: Option<&str>,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<VerifiedKey> {
    shape::object(Some(value), "$")?;
    let integrity = value.get("integrity");
    shape::object(integrity, "$.integrity")?;
    shape::closed(
        integrity,
        &["alg", "key_id", "signed_digest", "signature_b64"],
        "$.integrity",
    )?;
    let integrity = integrity.expect("checked above");
    shape::exact(
        integrity.get("alg"),
        "ed25519",
        ErrorCode::IntegrityAlgorithmUnknown,
        "$.integrity.alg",
    )?;
    let key_id = shape::string(integrity.get("key_id"), "$.integrity.key_id")?;
    let signed_digest = shape::digest_hex(
        integrity.get("signed_digest"),
        "$.integrity.signed_digest",
        false,
    )?;
    let signature_b64 = shape::string(integrity.get("signature_b64"), "$.integrity.signature_b64")?;
    let canonical_shape = signature_b64.len() == 88
        && signature_b64.ends_with("==")
        && signature_b64[..86]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'+' || byte == b'/');
    if !canonical_shape {
        return fail(
            ErrorCode::IntegritySignatureNoncanonical,
            "signature_b64 must be canonical padded Base64",
            "$.integrity.signature_b64",
        );
    }
    keyring.validate()?;
    let Some(record) = keyring.get(key_id) else {
        return fail(
            ErrorCode::IntegrityKeyUnknown,
            format!("unknown key {key_id}"),
            "$.integrity.key_id",
        );
    };
    let role = record["role"].as_str().unwrap_or_default().to_owned();
    if expected_role.is_some_and(|expected| expected != role) {
        return fail(
            ErrorCode::IntegrityRoleMismatch,
            format!(
                "key {key_id} is not a {} key",
                expected_role.unwrap_or_default()
            ),
            "$",
        );
    }
    let digest = canonical_digest(value, domain.as_str())?;
    if digest != signed_digest {
        return fail(
            ErrorCode::IntegrityDigestMismatch,
            "signed digest does not match canonical artifact bytes",
            "$",
        );
    }
    let signature = decode_base64(signature_b64).unwrap_or_default();
    if encode_base64(&signature) != signature_b64 {
        return fail(
            ErrorCode::IntegritySignatureNoncanonical,
            "signature Base64 does not round-trip canonically",
            "$",
        );
    }
    let pem = public_key_text(record)?;
    let verified = match (
        ed25519_from_pem(pem),
        <[u8; 64]>::try_from(signature.as_slice()),
        decode_hex32(&digest),
    ) {
        (Some(key), Ok(signature), Some(message)) => {
            verifier.verify_ed25519(&key, &message, &signature)
        }
        _ => false,
    };
    if !verified {
        return fail(
            ErrorCode::IntegritySignatureInvalid,
            "Ed25519 signature verification failed",
            "$",
        );
    }
    Ok(VerifiedKey {
        role,
        principal_id: record
            .get("principal_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

/// `signArtifact`: `value` with an `integrity` member signed by `signer` over
/// its `domain` digest.
///
/// # Errors
///
/// `integrity_signer_required` when the signer's key id is empty: the
/// reference's `signArtifact` refuses a falsy `key_id` or private key before
/// it signs, and while a missing private key is a type matter here, an empty
/// id is representable. Then `schema_type` when `value` is not an object, or
/// a canonicalization error.
pub fn sign_artifact(
    value: &Value,
    domain: Domain,
    signer: &dyn ArtifactSigner,
) -> AuthorityResult<Value> {
    let key_id = signer.key_id();
    if key_id.is_empty() {
        return fail(
            ErrorCode::IntegritySignerRequired,
            "a key_id and Ed25519 private_key are required",
            "$",
        );
    }
    let mut signed = shape::object(Some(value), "$")?.clone();
    let digest = canonical_digest(value, domain.as_str())?;
    let message = decode_hex32(&digest).expect("a SHA-256 digest is 32 bytes");
    signed.insert(
        "integrity".to_owned(),
        json!({
            "alg": "ed25519",
            "key_id": key_id,
            "signed_digest": digest,
            "signature_b64": encode_base64(&signer.sign_ed25519(&message)),
        }),
    );
    Ok(Value::Object(signed))
}

/// The reference's `keyObject`: a truthy `public_key` wins over a truthy
/// `public_key_pem`. A JSON record can't hold a `KeyObject`, so `public_key`
/// is text that Node reads as PEM, and anything else makes Node throw, which
/// the port refuses.
fn public_key_text(record: &Value) -> AuthorityResult<&str> {
    for member in ["public_key", "public_key_pem"] {
        match record.get(member) {
            None | Some(Value::Null | Value::Bool(false)) => {}
            Some(Value::String(text)) if text.is_empty() => {}
            Some(Value::Number(number)) if number.as_f64() == Some(0.0) => {}
            Some(Value::String(text)) => return Ok(text),
            Some(_) => {
                return fail(
                    ErrorCode::IntegrityKeyUnknown,
                    format!("key record's {member} is not PEM text"),
                    "$",
                )
            }
        }
    }
    fail(
        ErrorCode::IntegrityKeyUnknown,
        "key record does not contain a public key",
        "$",
    )
}

/// The raw Ed25519 key in a `PUBLIC KEY` PEM, or `None` for anything else.
fn ed25519_from_pem(pem: &str) -> Option<[u8; 32]> {
    // PEM markers occupy their own lines. Search preamble lines, preserve the
    // first matching public block, and never merge adjacent containers.
    let mut lines = pem.strip_prefix('\u{feff}').unwrap_or(pem).split('\n');
    fn clean(line: &str) -> &str {
        line.trim_end_matches(|ch: char| ch.is_ascii() && ch <= ' ')
    }
    lines.find(|line| clean(line) == "-----BEGIN PUBLIC KEY-----")?;
    let mut base64 = String::new();
    let mut separator = false;
    let mut short_line = false;
    let mut closed = false;
    for line in lines {
        let line = clean(line);
        if line == "-----END PUBLIC KEY-----" {
            closed = true;
            break;
        }
        if line.is_empty() {
            if separator || !base64.is_empty() {
                return None;
            }
            separator = true;
            continue;
        }
        if line
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && !b"+/= \t\r".contains(&byte))
        {
            return None;
        }
        if separator {
            if short_line || line.len() > 64 {
                return None;
            }
            short_line = line.len() < 64;
        }
        base64.extend(line.chars().filter(|ch| !matches!(ch, ' ' | '\t' | '\r')));
    }
    if !closed || base64.is_empty() {
        return None;
    }
    let der = decode_base64(&base64)?;
    // OpenSSL accepts BER lengths and constructed BIT STRINGs in public SPKI.
    // Decode only this public Ed25519 structure; private containers never enter.
    let mut input = der.as_slice();
    let (0x30, mut spki) = public_ber_element(&mut input, 0)? else {
        return None;
    };
    let (0x30, mut algorithm) = public_ber_element(&mut spki, 0)? else {
        return None;
    };
    let (0x06, oid) = public_ber_element(&mut algorithm, 0)? else {
        return None;
    };
    if oid != [0x2b, 0x65, 0x70] || !algorithm.is_empty() {
        return None;
    }
    let mut key = Vec::new();
    public_ber_bits(&mut spki, &mut key, 0)?;
    if !spki.is_empty() {
        return None;
    }
    key.try_into().ok()
}

/// Read one bounded BER element, retaining exact container boundaries.
fn public_ber_element<'a>(input: &mut &'a [u8], depth: usize) -> Option<(u8, &'a [u8])> {
    if depth > 30 {
        return None;
    }
    let (&tag, rest) = input.split_first()?;
    let (&length, rest) = rest.split_first()?;
    if length == 0x80 {
        if tag & 0x20 == 0 {
            return None;
        }
        let mut cursor = rest;
        while !cursor.starts_with(&[0, 0]) {
            public_ber_element(&mut cursor, depth + 1)?;
        }
        let content = &rest[..rest.len().checked_sub(cursor.len())?];
        *input = &cursor[2..];
        return Some((tag, content));
    }
    let (length, rest) = if length & 0x80 == 0 {
        (usize::from(length), rest)
    } else {
        let count = usize::from(length & 0x7f);
        if count > std::mem::size_of::<usize>() {
            return None;
        }
        let bytes = rest.get(..count)?;
        let length = bytes.iter().try_fold(0_usize, |length, byte| {
            length.checked_mul(256)?.checked_add(usize::from(*byte))
        })?;
        (length, &rest[count..])
    };
    let content = rest.get(..length)?;
    *input = &rest[length..];
    Some((tag, content))
}

fn public_ber_bits(input: &mut &[u8], output: &mut Vec<u8>, depth: usize) -> Option<()> {
    if depth > 6 {
        return None;
    }
    let (tag, mut content) = public_ber_element(input, depth)?;
    match tag {
        0x03 => {
            let (&unused, bytes) = content.split_first()?;
            if unused > 7 || bytes.is_empty() || output.len().checked_add(bytes.len())? > 32 {
                return None;
            }
            output.extend_from_slice(bytes);
            *output.last_mut()? &= 0xff_u8 << unused;
        }
        0x23 => {
            public_ber_bits(&mut content, output, depth + 1)?;
            if !content.is_empty() {
                return None;
            }
        }
        _ => return None,
    }
    Some(())
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Padded standard base64.
pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let block = chunk.iter().enumerate().fold(0_u32, |sum, (index, byte)| {
            sum | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(
                    ALPHABET[(block >> (18 - 6 * index) & 63) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Padded standard base64, or `None` for anything malformed.
pub(crate) fn decode_base64(text: &str) -> Option<Vec<u8>> {
    if text.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let bytes = text.as_bytes();
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index == bytes.len() / 4 - 1;
        let padding = chunk.iter().rev().take_while(|byte| **byte == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut block = 0_u32;
        for (position, byte) in chunk.iter().enumerate() {
            let sextet = if position >= 4 - padding {
                0
            } else {
                u32::try_from(ALPHABET.iter().position(|letter| letter == byte)?).ok()?
            };
            block = block << 6 | sextet;
        }
        let decoded = [(block >> 16) as u8, (block >> 8) as u8, block as u8];
        out.extend_from_slice(&decoded[..3 - padding]);
    }
    Some(out)
}

fn decode_hex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(2 * index..2 * index + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_and_refuses_malformed_input() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(decode_base64(&encode_base64(bytes)).as_deref(), Some(bytes));
        }
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        for malformed in ["Zm9", "Zm=v", "Z===", "Zm9v!A==", "Zm8==A=="] {
            assert_eq!(decode_base64(malformed), None, "{malformed}");
        }
        // Non-zero trailing bits decode, but do not round-trip.
        assert_ne!(encode_base64(&decode_base64("Zm9=").unwrap()), "Zm9=");
    }

    struct Ring;

    impl SignatureVerifier for Ring {
        fn verify_ed25519(&self, key: &[u8; 32], message: &[u8; 32], signature: &[u8; 64]) -> bool {
            ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
                .verify(message, signature)
                .is_ok()
        }
    }

    struct TestSigner {
        id: &'static str,
        pair: ring::signature::Ed25519KeyPair,
    }

    impl TestSigner {
        fn new(id: &'static str) -> Self {
            let document =
                ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
                    .unwrap();
            Self {
                id,
                pair: ring::signature::Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap(),
            }
        }

        fn pem(&self) -> String {
            use ring::signature::KeyPair as _;
            let mut der = ED25519_SPKI_PREFIX.to_vec();
            der.extend_from_slice(self.pair.public_key().as_ref());
            format!(
                "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
                encode_base64(&der)
            )
        }
    }

    impl ArtifactSigner for TestSigner {
        fn key_id(&self) -> &str {
            self.id
        }

        fn sign_ed25519(&self, message: &[u8; 32]) -> [u8; 64] {
            self.pair.sign(message).as_ref().try_into().unwrap()
        }
    }

    struct ObservedSigner {
        inner: TestSigner,
        key_reads: std::cell::Cell<usize>,
        signing_calls: std::cell::Cell<usize>,
        change_id_after_first_read: bool,
    }

    impl ArtifactSigner for ObservedSigner {
        fn key_id(&self) -> &str {
            let reads = self.key_reads.get();
            self.key_reads.set(reads + 1);
            if self.change_id_after_first_read && reads != 0 {
                ""
            } else {
                self.inner.key_id()
            }
        }

        fn sign_ed25519(&self, message: &[u8; 32]) -> [u8; 64] {
            self.signing_calls.set(self.signing_calls.get() + 1);
            self.inner.sign_ed25519(message)
        }
    }

    #[test]
    fn signing_uses_the_same_key_id_that_passed_the_producer_guard() {
        let signer = ObservedSigner {
            inner: TestSigner::new("key:test"),
            key_reads: std::cell::Cell::new(0),
            signing_calls: std::cell::Cell::new(0),
            change_id_after_first_read: true,
        };
        let value = json!({"kind": "test", "value": 1});
        let expected = sign_artifact(&value, Domain::Decision, &signer.inner).unwrap();
        let actual = sign_artifact(&value, Domain::Decision, &signer).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(signer.key_reads.get(), 1);
        assert_eq!(signer.signing_calls.get(), 1);
        let keys = Keyring::new(vec![(
            "key:test".into(),
            json!({"role": "threads_authority", "public_key_pem": signer.inner.pem()}),
        )]);
        verify_signed_artifact(
            &actual,
            Domain::Decision,
            &keys,
            Some("threads_authority"),
            &Ring,
        )
        .unwrap();
    }

    #[test]
    fn empty_signer_refusal_precedes_body_validation_and_crypto() {
        let signer = ObservedSigner {
            inner: TestSigner::new(""),
            key_reads: std::cell::Cell::new(0),
            signing_calls: std::cell::Cell::new(0),
            change_id_after_first_read: false,
        };
        for value in [json!(null), json!([]), json!({"kind": "test"})] {
            let error = sign_artifact(&value, Domain::Decision, &signer).unwrap_err();
            assert_eq!(error.code, ErrorCode::IntegritySignerRequired);
            assert_eq!(
                error.message,
                "a key_id and Ed25519 private_key are required"
            );
            assert_eq!(error.path, "$");
            assert_eq!(signer.signing_calls.get(), 0);
        }
    }

    #[test]
    fn an_empty_signer_key_id_is_refused_before_signing_like_the_reference() {
        // The reference's `signArtifact` refuses a falsy `key_id` with
        // `integrity_signer_required` before it signs anything. A missing
        // private key is a type matter in Rust, since `ArtifactSigner`
        // supplies the signature, but an empty id is representable and must
        // get the same refusal rather than a signed artifact filed under "".
        assert_eq!(
            ErrorCode::IntegritySignerRequired.as_str(),
            "integrity_signer_required"
        );
        let artifact = json!({"kind": "test", "value": 1});
        let refused = sign_artifact(&artifact, Domain::Decision, &TestSigner::new(""))
            .map_err(|error| (error.code, error.path));
        assert_eq!(
            refused,
            Err((ErrorCode::IntegritySignerRequired, "$".to_owned()))
        );

        // The receiving side already fails closed on its own: the same real
        // signature filed under an empty id is refused at `$.integrity.key_id`
        // before any keyring lookup, so an escaped artifact never verifies.
        let signer = TestSigner::new("key:test");
        let mut emptied = sign_artifact(&artifact, Domain::Decision, &signer).unwrap();
        emptied["integrity"]["key_id"] = json!("");
        let error = verify_signed_artifact(
            &emptied,
            Domain::Decision,
            &Keyring::new(vec![(
                "key:test".into(),
                json!({"role": "threads_authority", "public_key_pem": signer.pem()}),
            )]),
            Some("threads_authority"),
            &Ring,
        )
        .unwrap_err();
        assert_eq!(
            (error.code, error.path.as_str()),
            (ErrorCode::SchemaType, "$.integrity.key_id")
        );
    }

    #[test]
    fn signatures_verify_only_under_the_named_key_and_unchanged_bytes() {
        // No conformance vector carries a well-formed signature that fails to
        // verify, so this is the guard that verification actually happens.
        let signer = TestSigner::new("key:test");
        let other = TestSigner::new("key:other");
        let keyring = |pem: String| {
            Keyring::new(vec![(
                "key:test".into(),
                json!({"role": "threads_authority", "public_key_pem": pem}),
            )])
        };
        let artifact = json!({"kind": "test", "value": 1});
        let signed = sign_artifact(&artifact, Domain::Decision, &signer).unwrap();
        let verify = |value: &Value, keyring: &Keyring| {
            verify_signed_artifact(
                value,
                Domain::Decision,
                keyring,
                Some("threads_authority"),
                &Ring,
            )
            .map_err(|error| error.code)
        };
        assert!(verify(&signed, &keyring(signer.pem())).is_ok());
        assert_eq!(
            verify(&signed, &keyring(other.pem())),
            Err(ErrorCode::IntegritySignatureInvalid),
            "a different key"
        );
        let mut tampered = signed.clone();
        let signature = tampered["integrity"]["signature_b64"]
            .as_str()
            .unwrap()
            .to_owned();
        let flipped = if &signature[10..11] == "A" { "B" } else { "A" };
        tampered["integrity"]["signature_b64"] =
            json!(format!("{}{flipped}{}", &signature[..10], &signature[11..]));
        assert_eq!(
            verify(&tampered, &keyring(signer.pem())),
            Err(ErrorCode::IntegritySignatureInvalid),
            "a canonical signature over something else"
        );
        let mut changed = signed.clone();
        changed["value"] = json!(2);
        assert_eq!(
            verify(&changed, &keyring(signer.pem())),
            Err(ErrorCode::IntegrityDigestMismatch),
            "a changed body"
        );
        assert_eq!(
            verify(
                &signed,
                &Keyring::new(vec![(
                    "key:test".into(),
                    json!({"role": "principal", "principal_id": "p:x", "public_key_pem": signer.pem()})
                )])
            ),
            Err(ErrorCode::IntegrityRoleMismatch)
        );
    }

    #[test]
    fn a_truthy_public_key_wins_over_public_key_pem() {
        // Expected codes are the reference's; where Node throws, the port
        // must refuse.
        let signer = TestSigner::new("key:test");
        let other = TestSigner::new("key:other");
        let signed = sign_artifact(&json!({"kind": "test"}), Domain::Decision, &signer).unwrap();
        let verify = |record: Value| {
            verify_signed_artifact(
                &signed,
                Domain::Decision,
                &Keyring::new(vec![("key:test".into(), record)]),
                Some("threads_authority"),
                &Ring,
            )
            .map(|_| ())
            .map_err(|error| error.code)
        };
        let with_pem = |public_key: Value| json!({"role": "threads_authority", "public_key_pem": signer.pem(), "public_key": public_key});
        assert_eq!(
            verify(with_pem(json!(other.pem()))),
            Err(ErrorCode::IntegritySignatureInvalid)
        );
        assert_eq!(
            verify(json!({"role": "threads_authority", "public_key": signer.pem()})),
            Ok(())
        );
        for falsy in [json!(null), json!(false), json!(0), json!("")] {
            assert_eq!(verify(with_pem(falsy.clone())), Ok(()), "{falsy}");
        }
        for thrown in [
            json!({}),
            json!([]),
            json!(1),
            json!(true),
            json!("not pem"),
        ] {
            assert!(verify(with_pem(thrown.clone())).is_err(), "{thrown}");
        }
        assert_eq!(
            verify(json!({"role": "threads_authority", "public_key_pem": ""})),
            Err(ErrorCode::IntegrityKeyUnknown)
        );
    }

    #[test]
    fn public_pem_boundaries_require_marker_lines_and_preserve_first_key() {
        let signer = TestSigner::new("key:test");
        let other = TestSigner::new("key:other");
        let signed =
            sign_artifact(&json!({"kind": "pem-boundary"}), Domain::Decision, &signer).unwrap();
        let pem = signer.pem();
        let verify = |pem: String| {
            let keys = Keyring::new(vec![(
                "key:test".into(),
                json!({"role": "threads_authority", "public_key_pem": pem}),
            )]);
            verify_signed_artifact(
                &signed,
                Domain::Decision,
                &keys,
                Some("threads_authority"),
                &Ring,
            )
            .map(|_| ())
            .map_err(|error| error.code)
        };
        for accepted in [
            format!("preamble\n{pem}postamble\n"),
            format!("{pem}{}", other.pem()),
            format!("\u{feff}{pem}"),
            pem.replace("-----\n", "----- \n"),
        ] {
            assert_eq!(verify(accepted), Ok(()));
        }
        assert_eq!(
            verify(format!("{}{pem}", other.pem())),
            Err(ErrorCode::IntegritySignatureInvalid)
        );
        for refused in [
            pem.replace("KEY-----\n", "KEY-----"),
            pem.replace("\n-----END", "-----END"),
            format!(" {pem}"),
            format!("\t{pem}"),
            pem.replace('\n', "\r"),
            pem.replace("MCow", "M\u{c}Cow"),
        ] {
            assert_eq!(verify(refused), Err(ErrorCode::IntegritySignatureInvalid));
        }
    }

    #[test]
    fn public_spki_accepts_long_form_lengths() {
        let signer = TestSigner::new("key:test");
        let pem = signer.pem();
        let base64 = pem.lines().nth(1).unwrap();
        let der = decode_base64(base64).unwrap();
        let mut encoded = vec![0x30, 0x81, der[1]];
        encoded.extend_from_slice(&der[2..]);
        let long_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            encode_base64(&encoded)
        );
        assert_eq!(ed25519_from_pem(&long_pem), ed25519_from_pem(&pem));
    }

    #[test]
    fn private_pem_is_outside_the_public_key_contract() {
        // Synthetic seed and private encoding exist only in this test's memory.
        let mut der = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        der.extend_from_slice(&[88; 32]);
        let signer = TestSigner {
            id: "key:boundary",
            pair: ring::signature::Ed25519KeyPair::from_pkcs8_maybe_unchecked(&der).unwrap(),
        };
        let signed = sign_artifact(
            &json!({"kind": "synthetic-public-key-boundary"}),
            Domain::Decision,
            &signer,
        )
        .unwrap();
        let label = ["PRIVATE", "KEY"].join(" ");
        let private = format!(
            "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
            encode_base64(&der)
        );
        for (pem, expected) in [
            (signer.pem(), Ok(())),
            (private, Err(ErrorCode::IntegritySignatureInvalid)),
        ] {
            let keys = Keyring::new(vec![(
                "key:boundary".into(),
                json!({"role": "threads_authority", "public_key_pem": pem}),
            )]);
            let result = verify_signed_artifact(
                &signed,
                Domain::Decision,
                &keys,
                Some("threads_authority"),
                &Ring,
            )
            .map(|_| ())
            .map_err(|error| error.code);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn duplicate_constructor_ids_are_refused_before_record_validation() {
        for second in [
            json!({"role": "threads_authority"}),
            json!({"role": "principal"}),
        ] {
            let mut keys = Keyring::new(vec![
                ("same".into(), json!({"role": "threads_authority"})),
                ("same".into(), second),
            ]);
            assert_eq!(
                keys.validate().unwrap_err().code,
                ErrorCode::JsonDuplicateKey
            );
            assert!(keys.record_mut("same").is_none());
        }
        let keys = Keyring::new(vec![
            ("same".into(), json!(null)),
            ("same".into(), json!({"role": "unknown"})),
        ]);
        assert_eq!(
            keys.validate().unwrap_err().code,
            ErrorCode::JsonDuplicateKey
        );
        let signer = TestSigner::new("key:test");
        let signed = sign_artifact(&json!({"kind": "test"}), Domain::Decision, &signer).unwrap();
        let record = json!({"role": "threads_authority", "public_key_pem": signer.pem()});
        let keys = Keyring::new(vec![
            ("key:test".into(), record.clone()),
            ("key:test".into(), record),
        ]);
        assert_eq!(
            verify_signed_artifact(
                &signed,
                Domain::Decision,
                &keys,
                Some("threads_authority"),
                &Ring
            )
            .unwrap_err()
            .code,
            ErrorCode::JsonDuplicateKey
        );
    }

    #[test]
    fn malformed_records_follow_supplied_order_without_sorting_ids() {
        let role = ("z".into(), json!({"role": "unknown"}));
        let principal = ("a".into(), json!({"role": "principal"}));
        for (records, expected) in [
            (
                vec![role.clone(), principal.clone()],
                ErrorCode::IntegrityRoleUnknown,
            ),
            (
                vec![principal, role],
                ErrorCode::IntegrityPrincipalIdMissing,
            ),
        ] {
            assert_eq!(Keyring::new(records).validate().unwrap_err().code, expected);
        }
    }

    #[test]
    fn pem_uses_the_first_complete_spki_like_the_reference_crypto_backend() {
        let signer = TestSigner::new("key:test");
        let signed = sign_artifact(&json!({"kind": "test"}), Domain::Decision, &signer).unwrap();
        let pem = signer.pem();
        let body = pem
            .trim()
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .unwrap()
            .strip_suffix("-----END PUBLIC KEY-----")
            .unwrap();
        let mut der = decode_base64(&body.split_whitespace().collect::<String>()).unwrap();
        der.push(0);
        let pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            encode_base64(&der)
        );
        let keys = Keyring::new(vec![(
            "key:test".into(),
            json!({"role": "threads_authority", "public_key_pem": pem}),
        )]);
        assert!(verify_signed_artifact(
            &signed,
            Domain::Decision,
            &keys,
            Some("threads_authority"),
            &Ring
        )
        .is_ok());
    }

    #[test]
    fn keyring_records_are_validated_in_order() {
        let keyring = |record: Value| Keyring::new(vec![("key:test".into(), record)]);
        assert_eq!(
            keyring(json!({"role": "threads_authority"})).validate(),
            Ok(())
        );
        assert_eq!(
            keyring(json!({"role": "principal"}))
                .validate()
                .unwrap_err()
                .code,
            ErrorCode::IntegrityPrincipalIdMissing
        );
        assert_eq!(
            keyring(json!({"role": "threads_authority", "principal_id": "p:x"}))
                .validate()
                .unwrap_err()
                .code,
            ErrorCode::IntegrityPrincipalIdUnexpected
        );
        assert_eq!(
            keyring(json!({"role": "admin"}))
                .validate()
                .unwrap_err()
                .code,
            ErrorCode::IntegrityRoleUnknown
        );
    }
}
