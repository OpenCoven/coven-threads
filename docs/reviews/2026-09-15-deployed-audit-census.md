# Deployed audit-census disposition — 2026-09-15

Scope: the actual deployed Coven profile on the maintainer workstation. This
record addresses, for that profile, the item OpenCoven/coven#886 named as its
remaining blocker — "no real deployed census/disposition was performed" — and
nothing else. The root's global deployed-history scope is wider than one
profile and stays open. It closes no root, grants no authority, and simulates
no human gate.

## Why this was performed

`threads-980` (OpenCoven/coven#886) requires that every opened veto window in
real deployed history carry exactly one typed terminal close. The supported
nine-case terminal/recovery matrix and the read-only global census landed at
accepted daemon `226bfcc8`, and CI `34784497791` proved the five terminal
families over synthetic corpora. What had never been done was running that
census against a real deployed profile. Until that happened, the root could not
distinguish "deployed history is clean" from "deployed history was never
looked at."

## Method

Two independent read-only methods were used so that neither depends on the
other's correctness.

1. **Accepted census tooling.** `coven ward audit-census`, whose source
   `crates/coven-cli/src/ward_audit_census.rs` is byte-identical to the
   accepted daemon `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`
   (SHA-256 `0a72d346e1ef8904477abb8a41d449534aa2fd81794e35b5a5ba42e5ea6db772`).
   The command is routed ahead of profile-lock and state initialization and
   opens the store through `open_existing_store_read_only`.
2. **Direct read-only SQL.** `sqlite3 'file:...?mode=ro'` against the same
   store file, querying the Ward tables directly.

Both were run against the path the daemon itself resolves. `coven config paths
--json` reports `coven.home` = `~/.coven` and
`store.session_ledger` = `~/.coven/coven.sqlite3`. `COVEN_HOME` is
unset, and no second `coven.sqlite3` profile exists under the user tree.

No write, repair, migration, or authority action was taken. The profile was
live and actively written throughout: the store grew from 5,369,208,832 to
5,377,527,808 bytes between the first and last observation while the Ward
tables stayed empty.

## Result

`coven ward audit-census --json`
(SHA-256 `371e0875788debcd4489ffa8c5e97e406b672b09234ade11d167091863310c95`):

```json
{
  "format": "coven.ward-window-census.v1",
  "complete": true,
  "throughAuditId": 0,
  "scannedHistoryRows": 0,
  "unresolvedHistories": 0,
  "artifactObservation": "non_atomic_unverified_names_only",
  "unattributedArtifactEntries": 0,
  "windows": []
}
```

Corroborating direct SQL against the same store:

| Surface | Observed |
|---|---|
| `ward_audit` rows | 0 |
| `ward_manifest` rows | 0 |
| `coven_ward_audit_reservations` rows | 0 |
| `coven_ward_audit_capacity.used_bytes` | 0 |
| `ward_schema_meta` | `ward_audit` = version 20 |
| `<home>/pending` | does not exist |
| `<home>/pending/quarantine` | does not exist |

The `ward_audit` schema carries the v20 `CHECK` constraints and the append-only
`UPDATE`/`DELETE` triggers, so the table is the enforced one, not a placeholder.
The 5.4 GB of store content is the session ledger (`events`, 10,232,199 rows),
which is unrelated to the Ward estate.

The census was run twice with identical output.

## Disposition

The accessible deployed profile has **zero** opened veto-window histories.
Therefore, in this profile:

- there are no unresolved or orphaned open intervals;
- there is no missing original proposal evidence;
- there is no unprovable or interrupted apply;
- there is nothing in quarantine, and no terminal receipt was invented for
  anything.

The disposition is complete and empty. It is a verified negative, not a
fabricated closure — the distinction OpenCoven/coven#886 explicitly demands.

## What this does not establish

- It is **not** evidence about deployments other than this workstation. It
  states the disposition of the profile that was actually inspected.
- An empty estate means the Ward/Threads proposal path was never exercised in
  this profile. That is weaker than auditing a busy deployment and finding it
  clean, and it must not be reported as the stronger claim.
- It does not supply the separate J6 five-family evidence, which already landed
  at `226bfcc8` under CI `34784497791`.
- It closes no root issue. It grants no sign-off. `threads-uqx.9` and
  `threads-uqx.10` remain untouched and are Nova's and Val's alone.
- Native reliability follow-ups OpenCoven/coven#1047, #1050, and #1051 remain
  open and are unaffected.
