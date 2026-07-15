# PHASE-3 — Coven Familiar Portability Format: Shape A vs Shape B

**Status:** DRAFT for Val decision (bead `threads-986.16` — both shapes drafted, Val selects; per MEMORY.md 2026-07-14 escalation policy, the decision is made from concrete drafts, not abstract framing)
**Date:** 2026-07-15
**Upstream:** RFC-0001 §5 (round-trip anchor); `PHASE-0-DESIGN.md` §2.4 `Channel::Serialization`, §3.3 C7
**Implemented substrate (shape-agnostic):** `coven-threads-core::portability` — `PortableWeave` envelope, `SerializationContract`, `export_weave` / `import_weave` with the fail-visibly matrix; C7 round-trip conformance suite green (11 tests, `tests/c7_roundtrip.rs`)

---

## 0. What is already decided (and out of scope for this decision)

The **semantics** are frozen and implemented, whatever the envelope:

- C7: every `Serialization`-covered thread carries a `SerializationMarker` strand; export→import yields equivalent tension state **or fails visibly**.
- The five-kind strand vocabulary round-trips (`ContentHash`, `Signature`, `ManifestEntry`, `AuditTrail`, `SerializationMarker`).
- Import never widens authority (RFC-0001 §5.1) — verified in tests.
- Tamper, version skew, contract skew, duplicate pairs: typed refusals.

This decision selects the **interchange encoding** wrapped around those semantics.

## 1. Starting constraint (do not drift)

`.af` non-adoption was source-verified 2026-07-14 (`PHASE-0-DESIGN.md` §12): Letta's `CoreMemoryBlockSchema` has **no protection field**; runtime `read_only` is stripped at export. Whatever shape wins, silent-downgrade-on-import remains non-conformant. Shape A must not become adoption-by-erosion.

## 2. Shape A — `.af` superset

**One artifact, two audiences:** a valid `.af` agent file with the authority layer riding in optional, namespaced fields that vanilla Letta ignores.

```jsonc
// agent.af (abridged) — Shape A
{
  "agent_type": "coven_familiar",
  "core_memory": [
    {
      "label": "SOUL.md",
      "value": "…surface content…",
      // ── coven-threads extension (optional fields, x_-prefixed) ──
      "x_coven_thread": {
        "surface": "SOUL.md",
        "writer": "principal:val",
        "holds_under": ["forced", "serialization", "mutation"],
        "tension": { "state": "holds" },
        "strands": [ /* five-kind strand records, serde JSON */ ]
      }
    }
  ],
  "x_coven_weave": {
    "format_version": "0.1.0",
    "contract_hash": "b3:…",          // SerializationContract::contract_hash
    "weave_hash": "b3:…",             // Merkle root, §4 canonical ordering
    "familiar_id": "…", "coven_ref": null,
    "pattern_descriptor": { /* derived, non-authoritative */ }
  }
}
```

**Import rule (both directions of the asymmetry):**
- Coven runtime importing: `x_coven_weave` present → full C7 verification (the implemented `import_weave` path). Absent → **not a warded familiar**; import as unwarded material only with explicit operator acknowledgment (never silently as-if-warded).
- Vanilla Letta importing: reads the `.af` fields, ignores `x_*` — the familiar *runs* but the authority layer is inert. This is precisely the `.af` failure mode: acceptable only because the Coven side re-verifies on return (a round-trip through Letta that drops `x_*` fields changes the artifact → contract/weave-hash checks fail → visible refusal on re-import).

**Wins:** interop gravity (Letta ecosystem tooling opens the file); one-file story for "move my familiar."
**Loses:** the protection layer is structurally optional in the schema — the format itself cannot state "this MUST verify"; drift pressure every time `.af` upstream changes; the untyped `value`/`label` pairing duplicates surface content outside the thread commitment (two sources of truth inside one file).

## 3. Shape B — net-new format (`.weave`)

**Designed from the typed protected surface out.** The `PortableWeave` envelope, as implemented, *is* the format; surfaces ride as content entries keyed by `SurfaceId`, committed by the same strands the gate enforces.

```jsonc
// familiar.weave (abridged) — Shape B == PortableWeave + surfaces
{
  "format_version": "0.1.0",
  "contract_hash": "b3:…",
  "record": {
    "id": "…", "familiar_id": "…", "coven_ref": null,
    "weave_hash": "b3:…",
    "threads": [ /* full Thread records: surface, writer, holds_under, tension, strands */ ],
    "pattern_descriptor": { /* derived, non-authoritative */ }
  },
  "surfaces": {                        // content payloads, hash-committed
    "SOUL.md": { "encoding": "utf8", "data": "…" }
  }
}
```

**Import rule:** there is exactly one: `import_weave` verification, then per-surface `ContentHash` check against `surfaces`. No unwarded fallback exists in-format; an artifact without a verifiable weave is not a `.weave` file.

**Wins:** the format *is* the contract — protection is structural, not optional; zero upstream drift surface; already implemented (the envelope in `portability.rs` is Shape B minus the `surfaces` map); simplest possible conformance story ("valid iff `import_weave` accepts").
**Loses:** zero ecosystem gravity; needs its own `.af` *exporter* anyway for one-way Letta handoff (acknowledged lossy, marked as such).

## 4. Comparison table

| Axis | Shape A (`.af` superset) | Shape B (net-new `.weave`) |
|---|---|---|
| C7 enforceable in-format | No (fields optional by design) | Yes (structurally) |
| Silent-downgrade risk | Present at every non-Coven hop; caught only on re-import | Absent in-format |
| Upstream drift exposure | Tracks Letta schema forever | None |
| Ecosystem interop | High (Letta tooling reads it) | Low; explicit lossy exporter possible |
| Implementation distance from today | Envelope→`.af` mapping layer + dual-read rules | `surfaces` map + file extension (envelope already shipped) |
| Two-sources-of-truth inside artifact | Yes (`value` vs strand commitment) | No (surfaces hash-committed) |
| Conformance story | "valid `.af`" ∧ "valid weave" (two validators) | "accepted by `import_weave`" (one) |

## 5. Sage-lane read (recommendation, not decision)

Shape B for the canonical artifact, plus a clearly-marked **lossy one-way** `.af` exporter for Letta handoff (Shape A's interop win without letting `.af` become a round-trip surface). The C7 suite already proves Shape B's spine; Shape A's dual-read rules add a second validator whose failure modes are exactly the ones §1 refuses. Val decides.

## 6. Decision record

- [ ] Val selects: ☐ Shape A ☐ Shape B ☐ B + lossy A exporter
- [ ] On selection: bead `threads-986.16` closes with the round-trip suite passing on the selected shape; format doc graduates from DRAFT.
