# FAQ

Honest answers to the questions we expect. Sources cited inline; when in doubt, the order of authority is RFC-0001 §5, then `specs/PHASE-0-DESIGN.md` (frozen v0.2), then the `coven-grimoire` Ward Layer Spec Brief §9. Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Why not just make SOUL.md read-only at the filesystem level?

Because `chmod 444` answers the wrong question. Filesystem permissions gate *whether a process can write a path*; the problem this layer exists for is *whether the authority state of an identity surface permits a specific write, arriving on a specific channel, from a specific writer*.

Walk through what read-only-on-disk actually fails to cover:

- **Legitimate writes have to happen.** SOUL.md and MEMORY.md are living surfaces — the dreaming sweep promotes memory into them under principal consent (`Channel::Deliberate`). A read-only bit gives you a binary: either nobody writes (the surface is dead) or someone flips the bit to write (and while it's flipped, *anyone* with that privilege writes). There is no "writes permitted only through the gated proposal path" mode in POSIX permissions.
- **The file on disk isn't the only copy that matters.** Protected content is *materialized into the context window* to take effect, and forced compaction (`Channel::Forced`) rewrites the window, not the file. The disk copy can be pristine while the working identity is silently paraphrased away. WARD-C1–C6 exist precisely because the file's permission bits are irrelevant to that mutation path (Ward Layer Spec Brief §9).
- **Serialization leaves the filesystem entirely.** Export/import (`Channel::Serialization`) round-trips the identity through an artifact where no permission bit follows it. C7 exists because the protection must be a property of the *authority contract*, carried in the artifact and verified on import — not a property of one filesystem's metadata.
- **No audit, no legibility, no degradation.** A blocked write under `chmod` is an `EACCES` and nothing else. The gate layer gives you a typed verdict, a named reason, an append-only `ward.audit` entry (RFC-0001 §5.6), and — for the repairable middle ground — `DegradeToProposal` staging so a human can approve the write rather than the write simply vanishing.

**Where this now lives in shipped code (2026-07-15).** PR https://github.com/OpenCoven/coven/pull/382 wired the gate into the coven daemon. On the daemon's protected-edit path (`crates/coven-cli/src/api.rs`), each proposal goes through three ordered steps: (1) `Ward::evaluate` — pure adjudication of targets against tiers, no side effects; (2) if any target is `Blocked`, `Ward::apply` refuses the whole proposal as a unit (403) — a blocked target never rides into a staged write; (3) for Tier-0 (Protected) targets that survived, `threads_gate::gate_protected_edits` (`crates/coven-cli/src/threads_gate.rs`) runs — this is where `coven-threads-core` is called, and it returns a `GateOutcome` of `Permitted`, `Rejected`, `Staged`, or `Errored`. Only on `Permitted` does `Ward::apply` then run the actual write. `Staged` proposals land in `~/.coven/pending/`, and every outcome appends to the `ward_audit` sqlite table (schema from `coven_threads_core::WARD_AUDIT_SCHEMA_SQL`, RFC-0001 §5.6 field set plus coven-threads extensions). None of that is expressible with a filesystem permission bit alone: `chmod` has no verdict, no channel, no writer identity, no staged-repair path, and no audit row. The gate answers a question `chmod` cannot state.

Filesystem permissions are still fine as *defense-in-depth*. They are just not an authority model. RFC-0001 §5.1 is explicit that what's normative is a structurally separate authority layer, and "the OS says no unless somebody flips a bit" doesn't express `(surface, writer, channel)`-level authority at all.

## How is this different from Ward?

Ward is the **spec**; `coven-threads` is the **implementation receiver**. Precisely:

- **RFC-0001 §5** ("The Ward") defines the normative requirements: authority-layer separation (§5.1), the ward file format (§5.2), approval tiers (§5.3), the four enforcement gates (§5.4), regression/identity probes (§5.5), and the audit log (§5.6). It says **what** must be checked and what conformance means.
- **The `coven` daemon** is the shipped trust boundary those checks run behind (`coven/docs/SAFETY-MODEL.md`) — but today it validates *who* and *what action*, not *what the target surface's authority state permits*.
- **`coven-threads`** is the missing piece between them: the *gate-shaped receiver* the daemon calls, which loads the weave, checks each affected thread's strands under the request's channel, and returns a verdict (design doc §1, §5). Ward's four gates are, in weave vocabulary, the **loom** — the fixed structure threads run through — not something this repo replaces.

So the honest relationship: Ward specifies, the daemon hosts, coven-threads enforces. If coven-threads ever disagrees with RFC-0001, coven-threads is wrong by declaration (design doc §3.2: "RFC wins on any conflict").

## Why isn't it `.af`-compatible?

Because `.af`, as it exists, cannot carry the one thing this layer is for.

The claim is source-verified, not vibes (design doc §12, checked 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`): Letta's `CoreMemoryBlockSchema` has **no protection field**, and the runtime `read_only` flag is **stripped at export**. So an agent whose persona block is read-only at runtime exports to an artifact in which that fact does not exist. The importing runtime cannot restore what it cannot see — and cannot even *ask* what was lost.

That is, precisely, the failure mode WARD-C7 exists to refuse: **silent downgrade on import**. C7 requires that export followed by import produce a weave with equivalent tension state *or fail visibly* ([channels-and-strands.md](channels-and-strands.md#serialization)). A format that structurally cannot represent the protection contract cannot satisfy that, and adopting it anyway would falsify the external-authority thesis at the format layer.

To be fair to `.af`: it serializes what Letta's model is — an editable persona-as-memory-block — and it does that fine. The divergence is a mismatch of models, not a defect claim: a Coven familiar has a *typed protected surface*, and the format must carry the type. The decision landed 2026-07-15 (`specs/PHASE-3-PORTABILITY.md` §6, bead `threads-986.16`): **Shape B — a net-new `.weave` envelope — is canonical**, with a clearly-marked lossy one-way `.af` exporter for Letta handoff. `.af` is never a round-trip surface, and silent-downgrade-on-import remains non-conformant.

## What happens if the daemon crashes mid-check?

Short answer: nothing was written, so nothing needs unwinding.

The order of operations makes this work (design doc §5; [architecture.md](architecture.md)): the validator is a pure computation — `coven-threads-core` has no filesystem side effects, no audit writes, no staging I/O — and the daemon applies the mutation only *after* a `Permit` verdict returns. A crash before the verdict means the protected surface was never touched. The client's request dies with the connection; on retry, the entire check runs again from scratch. There is no partially-applied state to recover, because there is no state to partially apply.

Within the check itself, the same posture holds for softer failures: a validator **panic** is caught by the daemon and treated as `Reject` with a diagnostic (Gate 4 fail-closed, RFC-0001 §5.4 — an error is an unknown, and unknowns reject). Defense in depth runs both directions: the crate provides a panic-catching wrapper, and the daemon must also catch at its boundary.

One honest caveat: the crash-atomicity of the *post-verdict* steps — applying the write and appending to `ward.audit` as an effectively-atomic pair — is Phase 2 daemon territory (merged to coven `main` via PR #382; see [phases.md](phases.md)). The crate defines the contracts; the transactional discipline around them lives, and deserves scrutiny, in the daemon integration.

## Is this a policy engine?

No, and the design doc says so as an explicit anti-goal (§1): coven-threads is *"a typed authority layer for OpenCoven familiar surfaces; typed correctness is the goal, not general-purpose reusability."*

The distinction is worth spelling out. A policy engine (OPA, Cedar, and kin) gives you a general language for expressing arbitrary rules over arbitrary resources, evaluated against data you feed it. coven-threads gives you a *fixed, typed vocabulary* — threads over `(surface, writer)` pairs, five strand kinds, four channels, three verdicts — and makes illegal states hard to represent rather than making all states expressible. There is no rule language to misconfigure; the "policy" is the weave's pattern predicate, which is Rust code implementing a trait, reviewed like code, with a derived descriptor for legibility (and a named anti-pattern forbidding enforcement on the descriptor — [concepts.md](concepts.md#the-descriptor-vs-predicate-anti-pattern)).

Could you bend it into a general policy engine? Probably, and the design treats that as scope creep to refuse: the layer owns *authority over writes to the protected surface, gated by the weave* — not retrieval, not promotion, not dreaming, not arbitrary resource governance (§3.3.1's "anti-non-negotiable": hold the line).

## Can a familiar bypass the gate by writing files directly?

This is *the* right question, and the answer has two honest halves.

**Within the boundary the system defines:** no. RFC-0001 §5.1 requires that familiar-controlled processes have *no write path* to the daemon's authority surface, and the reference architecture runs familiars behind the daemon boundary — the untrusted client speaks over the socket, the daemon revalidates everything, and coven-threads is a crate inside the daemon, unreachable from familiar-controlled code (design doc §3.2). A familiar cannot modify the Ward file, restart the authority process, or bypass the gates, because there is nothing familiar-reachable to modify, restart, or bypass.

**Outside that boundary:** physics still applies. If a familiar-controlled process runs with raw OS write access to the protected files — same user, no sandbox — then yes, it can write the bytes, and no userspace gate can stop it. RFC-0001 is explicit that convention-based protection *is not* a protected surface (§5.1); the flip side is that the enforcement claim is only as strong as the actual process isolation underneath it. What the thread layer guarantees in that case is **detection, not prevention**: the out-of-band write breaks the `ContentHash` strand's commitment, the thread frays or snaps on next verification, the surface degrades to read-only through the gate, and the event is legible in `ward.audit` — tampering is caught by re-derivation from source, not silently absorbed (§3.3.1).

So the honest one-liner: the gate cannot be *cooperated* past, and out-of-band writes are *detected and quarantined* rather than prevented. Deployments that want prevention must supply real OS-level isolation between familiar processes and the protected paths — which the daemon's safety model assumes, and which is a deployment property, not a crate property.

## What's the difference between a Thread and a mutation request?

Different lifetimes, different roles — roughly the difference between a *standing relationship* and a *single event*.

A **Thread** is durable: the authority relationship from one protected surface to one writer, constructed once (with its strands committed), persisting across sessions, carrying tension state that evolves under load. It answers: *who may write this surface, under what channels, backed by what commitments, and is that relationship currently intact?*

A **MutationRequest** is momentary: one attempted write, described by exactly three facts — which surface, which writer, which channel (`validate.rs`; design doc §5). It exists for the duration of one gate check and resolves to one verdict.

The gate check is where they meet: the request *names* a `(surface, writer)` pair; the validator finds the thread bound to that pair and asks whether it holds under the request's channel. Request without a matching thread → `Reject` (fail-closed: all protected surfaces MUST have threads). Thread frayed → `DegradeToProposal`. Thread holds → `Permit`. The request never carries authority of its own — authority lives in the thread, and the request is merely tested against it. (This is also why staged proposals are "data, not authority": replaying one is just submitting a new request, which meets the thread again — [authority-model.md](authority-model.md#degradetoproposal).)

## Why call it a "weave" and not just a "policy set"?

Because the two names make different claims, and only one of them is true of this design.

A "policy set" is a *bag*: rules, individually evaluated, individually true or false, with no structure among them. Calling the weave a policy set would erase the three properties that do actual work here (design doc §2.2):

1. **The pattern is the unit of enforcement, not the members.** A weave is coherent iff its *pattern predicate* holds over the threads jointly — "these specific threads must all hold *together* for the identity to be coherent." Authority lives on the weave, not on the rows (§3.3.1). A bag has no "together."
2. **Failure has a location.** When a thread snaps, the weave degrades *at that thread's surface*, reports which surface, quarantines it read-only, and the familiar continues elsewhere. That partial-degradation shape — a hole in a fabric, not a false boolean — is structural, and the metaphor carries it precisely.
3. **The vocabulary decomposes along real joints.** Threads (relationships) are made of strands (survivability commitments) and hold under channels (loads); the gates are the loom the pattern is made on. Each term is bound to a referent at first use, per the binding rule (§2.5), and the design's history shows the discipline is real: Sage's v0.1 loaded the metaphor wrongly (threads-as-static-contracts), and Echo's v0.1.1 pass *rebound it to the correct referents* rather than letting pretty language float free.

The design doc names the failure mode of metaphor-without-referent explicitly — "beautiful language, unclear semantics" — and the answer isn't to retreat to bureaucratic vocabulary, it's to keep the metaphor **only where it carries semantic weight** and bolt every term to a referent. "Weave" survives that test; "policy set" would be both duller *and less accurate*.

---

*Something unanswered? The frozen design doc (`specs/PHASE-0-DESIGN.md`) is short and readable; RFC-0001 §5 is the upstream source of truth. If those disagree with this FAQ, they win — file an issue against the docs.*
