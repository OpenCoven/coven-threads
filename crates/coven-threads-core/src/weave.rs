//! `Weave` — the enforced pattern of threads across a familiar (§2.2, §4).
//!
//! Not the tapestry-as-visual-whole: the *structural pattern* that determines
//! which threads cross which, and where the weave breaks if a thread snaps.
//! Ward's four gates are the **loom** the weave is made on, not threads (§2.2).
//!
//! `weave_hash` is a Merkle root over the threads in canonical
//! `(surface_path, writer_id)` order (§4). Any strand change changes the hash.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{CovenId, FamiliarId, SurfaceId, ThreadId, WeaveId, WriterId};
use crate::manifest::merkle_root;
use crate::pattern::{PatternDescriptor, PatternPredicate, WeaveCoherence};
use crate::thread::Thread;

/// Errors constructing or importing a weave.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WeaveError {
    /// §2.1: one thread per `(surface, writer)` pair. Two threads binding the
    /// same pair is a construction error, rejected fail-closed.
    #[error("duplicate thread for (surface {surface}, writer {writer})")]
    DuplicateThread {
        /// The duplicated surface.
        surface: SurfaceId,
        /// The duplicated writer.
        writer: WriterId,
    },

    /// A record's recorded `weave_hash` disagrees with recomputation — the C7
    /// "fails visibly" arm (§3.3): import produces an equivalent weave or errors.
    #[error("weave hash mismatch on import: recorded {recorded:02x?}, computed {computed:02x?}")]
    HashMismatch {
        /// The hash carried by the record.
        recorded: Vec<u8>,
        /// The hash recomputed from the record's threads.
        computed: Vec<u8>,
    },

    /// An `update_threads` closure tried to add, remove, or rebind a
    /// `(surface, writer)` pair. State updates may not change the authority
    /// topology (§2.1); the weave was rolled back unchanged.
    #[error("update_threads changed the authority topology; weave rolled back")]
    TopologyChanged,
}

/// The pattern-bound bundle of threads for a familiar (§4).
///
/// Carries its `PatternPredicate` (§4 `Weave.pattern`) — the authoritative gate
/// over the weave. The predicate is not serializable (it is enforcement logic,
/// not data); [`WeaveRecord`] is the serializable projection, and
/// [`Weave::from_record`] rebinds a predicate on import, verifying the hash.
#[derive(Debug)]
pub struct Weave {
    /// This weave's id.
    pub id: WeaveId,
    /// Which familiar this weave belongs to.
    pub familiar_id: FamiliarId,
    /// The threads composing this weave, in canonical `(surface, writer)` order.
    threads: Vec<Thread>,
    /// The authoritative pattern predicate (§2.2). Enforcement gates on this,
    /// never on a descriptor.
    pub pattern: Box<dyn PatternPredicate>,
    /// Merkle root over threads in canonical order (§4).
    weave_hash: Vec<u8>,
    /// Optional Coven this weave participates in (§4).
    pub coven_ref: Option<CovenId>,
}

impl Weave {
    /// Construct a weave, canonically ordering threads and computing the hash.
    ///
    /// Rejects duplicate `(surface, writer)` pairs (§2.1: one thread per pair).
    pub fn new(
        id: WeaveId,
        familiar_id: FamiliarId,
        mut threads: Vec<Thread>,
        pattern: Box<dyn PatternPredicate>,
        coven_ref: Option<CovenId>,
    ) -> Result<Self, WeaveError> {
        threads.sort_by(|a, b| (&a.surface, &a.writer).cmp(&(&b.surface, &b.writer)));
        for pair in threads.windows(2) {
            if pair[0].surface == pair[1].surface && pair[0].writer == pair[1].writer {
                return Err(WeaveError::DuplicateThread {
                    surface: pair[0].surface.clone(),
                    writer: pair[0].writer.clone(),
                });
            }
        }
        let weave_hash = compute_weave_hash(&threads);
        Ok(Self {
            id,
            familiar_id,
            threads,
            pattern,
            weave_hash,
            coven_ref,
        })
    }

    /// The threads in canonical order.
    pub fn threads(&self) -> &[Thread] {
        &self.threads
    }

    /// Thread ids in canonical order.
    pub fn thread_ids(&self) -> impl Iterator<Item = ThreadId> + '_ {
        self.threads.iter().map(|t| t.id)
    }

    /// Find the thread binding `(surface, writer)`, if woven.
    pub fn thread_for(&self, surface: &SurfaceId, writer: &WriterId) -> Option<&Thread> {
        self.threads
            .iter()
            .find(|t| &t.surface == surface && &t.writer == writer)
    }

    /// Any threads bound to a surface?
    pub fn covers_surface(&self, surface: &SurfaceId) -> bool {
        self.threads.iter().any(|t| &t.surface == surface)
    }

    /// The current Merkle hash.
    pub fn weave_hash(&self) -> &[u8] {
        &self.weave_hash
    }

    /// Update thread *state* (tension, strands) in place — the daemon verifier
    /// lane. The weave re-sorts and rehashes afterwards: tension is part of the
    /// commitment.
    ///
    /// The authority topology is frozen: the closure must not add, remove, or
    /// rebind `(surface, writer)` pairs. A closure that changes topology gets
    /// `WeaveError::TopologyChanged` and the weave is rolled back, unchanged —
    /// otherwise this method would be a safe-API bypass of the one-thread-per-
    /// pair construction rule (§2.1).
    pub fn update_threads<F: FnOnce(&mut [Thread])>(&mut self, f: F) -> Result<(), WeaveError> {
        let snapshot = self.threads.clone();
        f(&mut self.threads);
        self.threads
            .sort_by(|a, b| (&a.surface, &a.writer).cmp(&(&b.surface, &b.writer)));
        let topology_unchanged = self.threads.len() == snapshot.len()
            && self
                .threads
                .iter()
                .zip(snapshot.iter())
                .all(|(a, b)| a.surface == b.surface && a.writer == b.writer);
        if !topology_unchanged {
            self.threads = snapshot;
            return Err(WeaveError::TopologyChanged);
        }
        self.weave_hash = compute_weave_hash(&self.threads);
        Ok(())
    }

    /// Evaluate coherence via the weave's own pattern — the authoritative
    /// question (§2.2). A weave is coherent iff its pattern predicate holds.
    pub fn coherence(&self) -> WeaveCoherence {
        self.pattern.coherent(&self.threads)
    }

    /// The derived, non-authoritative descriptor of this weave's pattern.
    /// For rendering and audit only — never enforcement (§2.2).
    pub fn describe_pattern(&self) -> PatternDescriptor {
        self.pattern.describe()
    }

    /// Serializable projection of this weave (predicate excluded; its derived
    /// descriptor travels for legibility).
    pub fn to_record(&self) -> WeaveRecord {
        WeaveRecord {
            id: self.id,
            familiar_id: self.familiar_id,
            threads: self.threads.clone(),
            weave_hash: self.weave_hash.clone(),
            coven_ref: self.coven_ref,
            pattern_descriptor: self.pattern.describe(),
        }
    }

    /// Rebind a record to a predicate — the import half of the C7 round-trip
    /// (§3.3 invariant #4): produces a weave with equivalent tension state, or
    /// fails visibly.
    ///
    /// Verifies the recorded `weave_hash` against recomputation and re-checks the
    /// one-thread-per-pair rule. Deliberately does **not** compare the record's
    /// `pattern_descriptor` to `pattern.describe()` — the descriptor is derived
    /// and non-authoritative (§2.2); binding enforcement to it would be the
    /// derived-index problem. The authoritative serialization contract is the
    /// `SerializationMarker` strand's `contract_hash`, enforced per-thread.
    pub fn from_record(
        record: WeaveRecord,
        pattern: Box<dyn PatternPredicate>,
    ) -> Result<Self, WeaveError> {
        let weave = Self::new(
            record.id,
            record.familiar_id,
            record.threads,
            pattern,
            record.coven_ref,
        )?;
        if weave.weave_hash != record.weave_hash {
            return Err(WeaveError::HashMismatch {
                recorded: record.weave_hash,
                computed: weave.weave_hash,
            });
        }
        Ok(weave)
    }
}

/// Serializable projection of a [`Weave`] (§4, Phase 3 groundwork).
///
/// Carries the derived `pattern_descriptor` for legibility. On import the
/// predicate is rebound explicitly ([`Weave::from_record`]); nothing enforces
/// on the descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaveRecord {
    /// The weave's id.
    pub id: WeaveId,
    /// The familiar this weave belongs to.
    pub familiar_id: FamiliarId,
    /// Threads in canonical order.
    pub threads: Vec<Thread>,
    /// The Merkle hash at export time.
    pub weave_hash: Vec<u8>,
    /// Optional Coven reference.
    pub coven_ref: Option<CovenId>,
    /// Derived pattern summary — non-authoritative (§2.2).
    pub pattern_descriptor: PatternDescriptor,
}

/// Merkle root over threads in canonical `(surface_path, writer_id)` order (§4).
///
/// Each thread's leaf commits to: surface, writer, covered channels (as a sorted
/// set), the **full tension state** (variant, blamed strand, channel, reason,
/// timestamp — C7 equivalence covers all of it), and every strand's commitment
/// bytes in a canonical (sorted) order. All variable-length fields are
/// length-prefixed. Strand *ids* are identity, not commitment — two threads
/// committing the same material hash identically.
fn compute_weave_hash(threads: &[Thread]) -> Vec<u8> {
    merkle_root(threads.iter().map(thread_leaf_bytes)).to_vec()
}

fn thread_leaf_bytes(t: &Thread) -> Vec<u8> {
    use crate::manifest::put_field;
    let mut leaf = Vec::new();
    put_field(&mut leaf, b"thread:v2");
    put_field(&mut leaf, t.surface.as_str().as_bytes());
    put_field(&mut leaf, t.writer.as_str().as_bytes());

    let mut channel_tags: Vec<&'static str> = t.holds_under.iter().map(|c| c.tag()).collect();
    channel_tags.sort_unstable();
    channel_tags.dedup();
    put_field(&mut leaf, &(channel_tags.len() as u64).to_be_bytes());
    for tag in channel_tags {
        put_field(&mut leaf, tag.as_bytes());
    }

    // Full tension commitment — variant, blame, channel, reason, timestamp.
    put_field(&mut leaf, &t.tension.commitment_bytes());

    let mut strand_bytes: Vec<Vec<u8>> = t.strands.iter().map(|s| s.commitment_bytes()).collect();
    strand_bytes.sort_unstable();
    put_field(&mut leaf, &(strand_bytes.len() as u64).to_be_bytes());
    for sb in strand_bytes {
        put_field(&mut leaf, &sb);
    }
    leaf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::Channel;
    use crate::fray::SnapReason;
    use crate::ids::{ManifestId, StrandId};
    use crate::pattern::AllSurfacesHoldOnChannels;
    use crate::strand::{HashAlgo, Strand};
    use crate::thread::TensionState;
    use time::OffsetDateTime;

    fn strands() -> Vec<Strand> {
        vec![
            Strand::ContentHash {
                id: StrandId::new(),
                algorithm: HashAlgo::Blake3,
                value: vec![1; 32],
            },
            Strand::ManifestEntry {
                id: StrandId::new(),
                manifest_id: ManifestId(uuid::Uuid::nil()),
                entry_hash: vec![2; 32],
            },
            Strand::SerializationMarker {
                id: StrandId::new(),
                format_version: "0.1.0".into(),
                contract_hash: vec![3; 32],
            },
        ]
    }

    fn thread(surface: &str, writer: &str) -> Thread {
        Thread {
            id: ThreadId::new(),
            surface: SurfaceId::new(surface),
            writer: WriterId::new(writer),
            strands: strands(),
            holds_under: vec![Channel::Forced, Channel::Serialization, Channel::Mutation],
            created_at: OffsetDateTime::UNIX_EPOCH,
            tension: TensionState::Holds,
        }
    }

    fn floor_pattern() -> Box<dyn PatternPredicate> {
        Box::new(AllSurfacesHoldOnChannels::rfc0001_floor())
    }

    fn floor_threads() -> Vec<Thread> {
        ["SOUL.md", "IDENTITY.md", "MEMORY.md", "ward.toml"]
            .into_iter()
            .map(|s| thread(s, "principal:val"))
            .collect()
    }

    #[test]
    fn empty_weave_has_defined_hash() {
        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![],
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_eq!(w.weave_hash().len(), 32);
    }

    #[test]
    fn weave_hash_deterministic_across_input_order() {
        let a = thread("SOUL.md", "principal:val");
        let b = thread("IDENTITY.md", "principal:val");
        let c = thread("MEMORY.md", "familiar:sage");

        let w1 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![a.clone(), b.clone(), c.clone()],
            floor_pattern(),
            None,
        )
        .unwrap();
        let w2 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![c, a, b],
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_eq!(w1.weave_hash(), w2.weave_hash());
        // Canonical order is (surface, writer) lexicographic (§4).
        let surfaces: Vec<&str> = w1.threads().iter().map(|t| t.surface.as_str()).collect();
        assert_eq!(surfaces, vec!["IDENTITY.md", "MEMORY.md", "SOUL.md"]);
    }

    #[test]
    fn any_strand_change_changes_weave_hash() {
        // Bead threads-986.10 acceptance: strand-level change must be visible at
        // the weave commitment.
        let mut threads = floor_threads();
        let w1 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            threads.clone(),
            floor_pattern(),
            None,
        )
        .unwrap();

        if let Strand::ContentHash { value, .. } = &mut threads[0].strands[0] {
            value[0] ^= 0xff;
        } else {
            panic!("fixture: first strand should be ContentHash");
        }
        let w2 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            threads,
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_ne!(w1.weave_hash(), w2.weave_hash());
    }

    #[test]
    fn tension_change_changes_weave_hash() {
        let w1 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            None,
        )
        .unwrap();
        let mut w2 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_eq!(w1.weave_hash(), w2.weave_hash());

        w2.update_threads(|threads| {
            threads[0].snap(
                Channel::Mutation,
                SnapReason::Revoked,
                OffsetDateTime::now_utc(),
            );
        })
        .unwrap();
        assert_ne!(w1.weave_hash(), w2.weave_hash());
    }

    #[test]
    fn duplicate_surface_writer_pair_rejected() {
        // §2.1: one thread per (surface, writer) pair — fail-closed at
        // construction.
        let err = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![
                thread("SOUL.md", "principal:val"),
                thread("SOUL.md", "principal:val"),
            ],
            floor_pattern(),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, WeaveError::DuplicateThread { .. }));
    }

    #[test]
    fn same_surface_distinct_writers_allowed() {
        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![
                thread("SOUL.md", "principal:val"),
                thread("SOUL.md", "daemon"),
            ],
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_eq!(w.threads().len(), 2);
    }

    #[test]
    fn coherence_gates_on_predicate() {
        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_eq!(w.coherence(), WeaveCoherence::Coherent);
    }

    #[test]
    fn property_hash_stable_across_shuffles_and_sensitive_to_edits() {
        // Property-style test (bead threads-986.10): for pseudo-random thread
        // sets, (a) any input permutation hashes identically, (b) any single
        // strand edit changes the hash. Deterministic xorshift keeps CI stable.
        let mut state: u64 = 0x5EED_C0DE_D00D_F00D;
        let mut rng = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        for round in 0..25 {
            let n = 1 + (rng() % 7) as usize;
            let mut threads: Vec<Thread> = (0..n)
                .map(|i| {
                    let mut t = thread(
                        &format!("surface-{:02}.md", rng() % 10),
                        &format!("writer-{i}"),
                    );
                    if let Strand::ContentHash { value, .. } = &mut t.strands[0] {
                        *value = rng().to_be_bytes().to_vec();
                    }
                    t
                })
                .collect();
            let base = Weave::new(
                WeaveId::new(),
                FamiliarId::new(),
                threads.clone(),
                floor_pattern(),
                None,
            )
            .unwrap();

            // (a) Fisher-Yates shuffle → identical hash.
            for i in (1..threads.len()).rev() {
                let j = (rng() % (i as u64 + 1)) as usize;
                threads.swap(i, j);
            }
            let shuffled = Weave::new(
                WeaveId::new(),
                FamiliarId::new(),
                threads.clone(),
                floor_pattern(),
                None,
            )
            .unwrap();
            assert_eq!(
                base.weave_hash(),
                shuffled.weave_hash(),
                "round {round}: shuffle changed hash"
            );

            // (b) Single strand edit → different hash.
            let victim = (rng() % threads.len() as u64) as usize;
            if let Strand::ContentHash { value, .. } = &mut threads[victim].strands[0] {
                value[0] ^= 0x01;
            }
            let edited = Weave::new(
                WeaveId::new(),
                FamiliarId::new(),
                threads,
                floor_pattern(),
                None,
            )
            .unwrap();
            assert_ne!(
                base.weave_hash(),
                edited.weave_hash(),
                "round {round}: strand edit did not change hash"
            );
        }
    }

    #[test]
    fn fray_details_change_weave_hash() {
        // Review finding: the leaf must commit the full tension state, not
        // just the variant. Same variant, different reason/channel/timestamp
        // must hash differently.
        let base = floor_threads();
        let fray_at = OffsetDateTime::UNIX_EPOCH;

        let mut a = base.clone();
        a[0].fray(
            None,
            Channel::Mutation,
            crate::fray::FrayReason::ContentHashMismatch,
            fray_at,
        );
        let mut b = base.clone();
        b[0].fray(
            None,
            Channel::Mutation,
            crate::fray::FrayReason::SignatureInvalid,
            fray_at,
        );
        let mut c = base.clone();
        c[0].fray(
            None,
            Channel::Forced,
            crate::fray::FrayReason::ContentHashMismatch,
            fray_at,
        );
        let mut d = base.clone();
        d[0].fray(
            None,
            Channel::Mutation,
            crate::fray::FrayReason::ContentHashMismatch,
            fray_at + time::Duration::seconds(1),
        );

        let hash = |threads: Vec<Thread>| {
            Weave::new(
                WeaveId::new(),
                FamiliarId::new(),
                threads,
                floor_pattern(),
                None,
            )
            .unwrap()
            .weave_hash()
            .to_vec()
        };
        let (ha, hb, hc, hd) = (hash(a), hash(b), hash(c), hash(d));
        assert_ne!(ha, hb, "reason must be committed");
        assert_ne!(ha, hc, "channel must be committed");
        assert_ne!(ha, hd, "timestamp must be committed");
    }

    #[test]
    fn delimiter_bytes_in_ids_cannot_forge_leaves() {
        // Review finding: delimiter-based framing let (surface="a", writer
        // containing a delimiter) collide with a shifted split. With
        // length-prefixed fields the two topologies must hash differently.
        let make = |surface: &str, writer: &str| {
            let mut t = thread(surface, writer);
            t.strands.clear();
            t
        };
        let w1 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![make("a", "b\0c")],
            floor_pattern(),
            None,
        )
        .unwrap();
        let w2 = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            vec![make("a\0b", "c")],
            floor_pattern(),
            None,
        )
        .unwrap();
        assert_ne!(w1.weave_hash(), w2.weave_hash());
    }

    #[test]
    fn update_threads_rejects_topology_change_and_rolls_back() {
        // Review finding: update_threads must not be a safe-API bypass of the
        // one-thread-per-(surface, writer) construction rule.
        let mut w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            None,
        )
        .unwrap();
        let hash_before = w.weave_hash().to_vec();
        let threads_before = w.threads().to_vec();

        // Rebinding a writer is a topology change.
        let err = w
            .update_threads(|threads| {
                threads[0].writer = WriterId::new("familiar:mallory");
            })
            .unwrap_err();
        assert_eq!(err, WeaveError::TopologyChanged);
        assert_eq!(w.threads(), threads_before.as_slice(), "rolled back");
        assert_eq!(w.weave_hash(), hash_before.as_slice(), "hash unchanged");

        // Duplicating a pair by rebinding onto an existing one is rejected too.
        let existing = w.threads()[1].clone();
        let err = w
            .update_threads(move |threads| {
                threads[0].surface = existing.surface.clone();
                threads[0].writer = existing.writer.clone();
            })
            .unwrap_err();
        assert_eq!(err, WeaveError::TopologyChanged);
        assert_eq!(w.threads(), threads_before.as_slice());

        // A pure tension update still succeeds.
        w.update_threads(|threads| {
            threads[0].snap(
                Channel::Mutation,
                crate::fray::SnapReason::Revoked,
                OffsetDateTime::UNIX_EPOCH,
            );
        })
        .unwrap();
        assert_ne!(w.weave_hash(), hash_before.as_slice());
    }

    #[test]
    fn record_roundtrip_preserves_tension_and_hash() {
        // C7 groundwork (§3.3 #4): export → import produces a weave with
        // equivalent tension state.
        let mut w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            Some(CovenId::new()),
        )
        .unwrap();
        w.update_threads(|threads| {
            threads[0].snap(
                Channel::Mutation,
                SnapReason::Revoked,
                OffsetDateTime::UNIX_EPOCH,
            );
        })
        .unwrap();

        let json = serde_json::to_string(&w.to_record()).unwrap();
        let record: WeaveRecord = serde_json::from_str(&json).unwrap();
        let back = Weave::from_record(record, floor_pattern()).unwrap();

        assert_eq!(back.weave_hash(), w.weave_hash());
        assert_eq!(back.threads(), w.threads());
        assert_eq!(back.coherence(), w.coherence());
    }

    #[test]
    fn tampered_record_fails_visibly() {
        // C7: ...or fails visibly. A record whose threads were altered after
        // export must not import silently.
        let w = Weave::new(
            WeaveId::new(),
            FamiliarId::new(),
            floor_threads(),
            floor_pattern(),
            None,
        )
        .unwrap();
        let mut record = w.to_record();
        if let Strand::ContentHash { value, .. } = &mut record.threads[0].strands[0] {
            value[0] ^= 0xff;
        } else {
            panic!("fixture: first strand should be ContentHash");
        }
        let err = Weave::from_record(record, floor_pattern()).unwrap_err();
        assert!(matches!(err, WeaveError::HashMismatch { .. }));
    }
}
