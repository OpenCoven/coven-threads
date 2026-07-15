//! `DegradeToProposal` staging records (§5).
//!
//! When a frayed thread degrades a mutation to a proposal, the daemon stages
//! the write at `~/.coven/pending/` and notifies the principal — no immediate
//! write to the protected surface. This module owns the *record shape* and its
//! filename convention; the daemon owns the directory, the file I/O, and the
//! notification (Phase 2).
//!
//! A staged proposal is data, not authority: replaying it later still goes
//! back through `validate` — staging never becomes a bypass (RFC-0001 §5.4
//! Gate 4).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::channel::Channel;
use crate::fray::FrayOrSnap;
use crate::ids::{FamiliarId, ProposalId, SurfaceId, ThreadId, WriterId};

/// One staged edit inside a pending proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedEdit {
    /// The surface the edit targets.
    pub surface: SurfaceId,
    /// The proposed full contents, UTF-8 lossless or base64 (tagged).
    pub contents: StagedContents,
}

/// Proposed contents, kept legible in the staged JSON when they are UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "encoding", content = "data")]
pub enum StagedContents {
    /// UTF-8 text, stored as-is for principal legibility.
    Utf8(String),
    /// Non-UTF-8 payload, base64-encoded.
    Base64(String),
}

impl StagedContents {
    /// Wrap raw bytes, preferring legible UTF-8.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(s) => StagedContents::Utf8(s.to_string()),
            Err(_) => StagedContents::Base64(base64_encode(bytes)),
        }
    }

    /// Recover the raw bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        match self {
            StagedContents::Utf8(s) => Ok(s.as_bytes().to_vec()),
            StagedContents::Base64(b) => base64_decode(b),
        }
    }
}

/// A proposal staged at `~/.coven/pending/` after `DegradeToProposal` (§5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingProposal {
    /// This proposal's id (also its filename stem).
    pub id: ProposalId,
    /// The familiar whose weave degraded the write.
    pub familiar_id: FamiliarId,
    /// The writer whose mutation was degraded.
    pub writer: WriterId,
    /// The channel the mutation arrived on.
    pub channel: Channel,
    /// The thread that frayed.
    pub thread_id: ThreadId,
    /// The fray that triggered degradation, verbatim for the principal.
    pub fray: FrayOrSnap,
    /// The staged edits (full desired contents, never diffs).
    pub edits: Vec<StagedEdit>,
    /// When the proposal was staged.
    pub staged_at: OffsetDateTime,
}

impl PendingProposal {
    /// Conventional filename inside `~/.coven/pending/`:
    /// `<familiar-uuid>-<proposal-uuid>.json`.
    pub fn file_name(&self) -> String {
        format!("{}-{}.json", self.familiar_id.0, self.id.0)
    }
}

// Minimal base64 (standard alphabet, padded) — a staging record must not pull
// an encoding dependency into the trust boundary for one field.
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok(u32::from(c - b'A')),
            b'a'..=b'z' => Ok(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Ok(u32::from(c - b'0') + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            other => Err(format!("invalid base64 byte {other:#04x}")),
        }
    }
    let raw: Vec<u8> = s.bytes().filter(|b| *b != b'\n').collect();
    if raw.len() % 4 != 0 {
        return Err("base64 length not a multiple of 4".into());
    }
    let mut out = Vec::with_capacity(raw.len() / 4 * 3);
    for chunk in raw.chunks(4) {
        let pad = chunk.iter().filter(|c| **c == b'=').count();
        if pad > 2 || chunk[..4 - pad].contains(&b'=') {
            return Err("malformed base64 padding".into());
        }
        let mut n: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            let v = if *c == b'=' { 0 } else { val(*c)? };
            n |= v << (18 - 6 * i as u32);
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fray::FrayReason;

    #[test]
    fn utf8_contents_stay_legible() {
        let c = StagedContents::from_bytes(b"# SOUL\nI am Sage.\n");
        assert!(matches!(&c, StagedContents::Utf8(s) if s.contains("I am Sage.")));
        assert_eq!(c.to_bytes().unwrap(), b"# SOUL\nI am Sage.\n");
    }

    #[test]
    fn binary_contents_roundtrip_base64() {
        let payload: Vec<u8> = (0..=255).collect();
        let c = StagedContents::from_bytes(&payload);
        assert!(matches!(c, StagedContents::Base64(_)));
        assert_eq!(c.to_bytes().unwrap(), payload);
    }

    #[test]
    fn base64_padding_edges() {
        for len in 0..8 {
            let payload: Vec<u8> = (0..len as u8).collect();
            let enc = base64_encode(&payload);
            assert_eq!(base64_decode(&enc).unwrap(), payload, "len {len}");
        }
    }

    #[test]
    fn pending_proposal_roundtrips_and_names_itself() {
        let p = PendingProposal {
            id: ProposalId::new(),
            familiar_id: FamiliarId::new(),
            writer: WriterId::new("principal:val"),
            channel: Channel::Mutation,
            thread_id: ThreadId::new(),
            fray: FrayOrSnap::Frayed {
                strand: None,
                channel: Channel::Mutation,
                reason: FrayReason::ContentHashMismatch,
            },
            edits: vec![StagedEdit {
                surface: SurfaceId::new("SOUL.md"),
                contents: StagedContents::from_bytes(b"proposed"),
            }],
            staged_at: OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_string_pretty(&p).unwrap();
        let back: PendingProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        assert!(p.file_name().ends_with(".json"));
        assert!(p.file_name().contains(&p.id.0.to_string()));
    }
}
