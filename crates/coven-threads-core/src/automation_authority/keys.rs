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
    /// the reference validates on every verification.
    #[must_use]
    pub fn new(records: Vec<(String, Value)>) -> Self {
        Self { records }
    }

    /// A keyring from a JSON object of records, such as `keyring.json`'s
    /// `keys`.
    #[must_use]
    pub fn from_object(keys: &Map<String, Value>) -> Self {
        Self::new(
            keys.iter()
                .map(|(id, record)| (id.clone(), record.clone()))
                .collect(),
        )
    }

    /// The record for `key_id`, to adjust in place.
    pub fn record_mut(&mut self, key_id: &str) -> Option<&mut Value> {
        self.records
            .iter_mut()
            .find(|(id, _)| id == key_id)
            .map(|(_, record)| record)
    }

    /// `validateKeyring`.
    ///
    /// # Errors
    ///
    /// The first malformed record's `integrity_*` or `schema_*` code.
    pub fn validate(&self) -> AuthorityResult<()> {
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
    let Some(pem) = record.get("public_key_pem").and_then(Value::as_str) else {
        return fail(
            ErrorCode::IntegrityKeyUnknown,
            "key record does not contain a public key",
            "$",
        );
    };
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
/// `schema_type` when `value` is not an object, or a canonicalization error.
pub fn sign_artifact(
    value: &Value,
    domain: Domain,
    signer: &dyn ArtifactSigner,
) -> AuthorityResult<Value> {
    let mut signed = shape::object(Some(value), "$")?.clone();
    let digest = canonical_digest(value, domain.as_str())?;
    let message = decode_hex32(&digest).expect("a SHA-256 digest is 32 bytes");
    signed.insert(
        "integrity".to_owned(),
        json!({
            "alg": "ed25519",
            "key_id": signer.key_id(),
            "signed_digest": digest,
            "signature_b64": encode_base64(&signer.sign_ed25519(&message)),
        }),
    );
    Ok(Value::Object(signed))
}

/// The raw Ed25519 key in a `PUBLIC KEY` PEM, or `None` for anything else.
fn ed25519_from_pem(pem: &str) -> Option<[u8; 32]> {
    let body = pem
        .trim()
        .strip_prefix("-----BEGIN PUBLIC KEY-----")?
        .strip_suffix("-----END PUBLIC KEY-----")?;
    let base64: String = body
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
    let der = decode_base64(&base64)?;
    der.strip_prefix(&ED25519_SPKI_PREFIX)?.try_into().ok()
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
