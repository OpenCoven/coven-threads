//! Newtype IDs for the authority layer (`PHASE-0-DESIGN.md` §4).
//!
//! Every ID is a newtype wrapper. This is deliberate: in an authority layer,
//! passing a `ThreadId` where a `WeaveId` is expected is a correctness bug,
//! not a lint. The type system rejects it at compile time.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! uuid_newtype {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh id.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

macro_rules! string_newtype {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            /// Construct from anything string-like.
            pub fn new<S: Into<String>>(s: S) -> Self {
                Self(s.into())
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

uuid_newtype!(/// Identifier for a `Strand` (§4).
    StrandId);
uuid_newtype!(/// Identifier for a `Thread` (§4).
    ThreadId);
uuid_newtype!(/// Identifier for a `Weave` (§4).
    WeaveId);
uuid_newtype!(/// Identifier for a familiar (§4 `Weave.familiar_id`).
    FamiliarId);
uuid_newtype!(/// Identifier for a Coven (§4 `Weave.coven_ref`).
    CovenId);
uuid_newtype!(/// Identifier for a manifest referenced by a `ManifestEntry` strand (§4).
    ManifestId);

string_newtype!(
    /// Identifier for a protected surface (§4 `SurfaceId`).
    ///
    /// A *typed surface, not a raw path* — §3.3 invariant #1 (identity-as-memory-property):
    /// threads bind to typed surfaces at construction time. The inner string is a
    /// workspace-relative path (e.g. `SOUL.md`), human-readable in audit logs, with a
    /// stable `Ord` because it participates in `weave_hash` canonical ordering
    /// (`(surface_path, writer_id)` lexicographic, §4).
    SurfaceId);

string_newtype!(
    /// Identifier for a writer proposing mutations on a surface (§4 `WriterId`).
    ///
    /// Opaque. Split per Echo v0.1.1 (§4 change log): `WriterId` names the writer,
    /// `Channel` names the load, `PatternPredicate` names the gate structure —
    /// three independent axes, never collapsed.
    WriterId);

string_newtype!(
    /// Reference to an event in the daemon-owned audit store (§4 `EventRef`).
    ///
    /// Points into the `ward.audit` table inside `coven.sqlite3` (§3.4: single
    /// daemon-owned audit store, never a sidecar). Opaque to this crate; the daemon
    /// resolves it.
    EventRef);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtype_ids_are_distinct_types() {
        // ThreadId and WeaveId share a Uuid representation but must not interconvert.
        // Enforced at compile time; the test documents the intent.
        let t = ThreadId::new();
        let w = WeaveId::new();
        assert_ne!(t.0, w.0);
    }

    #[test]
    fn surface_id_orders_stable() {
        // SurfaceId participates in weave_hash canonical ordering (§4); stable Ord is
        // a correctness requirement, not a nicety.
        let mut v = [
            SurfaceId::new("SOUL.md"),
            SurfaceId::new("IDENTITY.md"),
            SurfaceId::new("MEMORY.md"),
        ];
        v.sort();
        assert_eq!(v[0].as_str(), "IDENTITY.md");
        assert_eq!(v[1].as_str(), "MEMORY.md");
        assert_eq!(v[2].as_str(), "SOUL.md");
    }

    #[test]
    fn ids_roundtrip_json() {
        let s = SurfaceId::new("SOUL.md");
        let json = serde_json::to_string(&s).unwrap();
        let back: SurfaceId = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);

        let t = ThreadId::new();
        let json = serde_json::to_string(&t).unwrap();
        let back: ThreadId = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
