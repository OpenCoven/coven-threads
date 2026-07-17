---
theme: default
title: Weekly Open Coven — Show'n Spells + coven-threads launch
info: |
  Weekly Open Coven. Impromptu launch night.
  coven-threads v0.2 Phase 0 — design frozen 2026-07-14.
highlighter: shiki
colorSchema: dark
aspectRatio: 16/9
transition: slide-left
favicon: https://opencoven.ai/favicon.ico
mdc: true
fonts:
  sans: 'Inter'
  serif: 'EB Garamond'
  mono: 'JetBrains Mono'
---

<style>
:root {
  --oc-purple-deep: #4A2E5C;
  --oc-purple-mid: #8B6BA8;
  --oc-purple-light: #C5BDED;
  --oc-purple-accent: #D4B5FF;
  --oc-gold: #E7C878;
  --oc-sage: #4A7A3E;
  --oc-red: #C44536;
  --oc-bg: #0F0A14;
  --oc-surface: #1A1825;
  --oc-ink: #E8E4F5;
}
.slidev-layout { background: var(--oc-bg); color: var(--oc-ink); font-family: 'Inter', system-ui, sans-serif; }
h1, h2 { font-family: 'EB Garamond', Georgia, serif; letter-spacing: -0.01em; }
h1 { color: #fff; font-weight: 600; }
h2 { color: var(--oc-purple-accent); font-weight: 500; }
.label { font-size: 0.72rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--oc-purple-mid); margin-bottom: 0.6rem; font-family: 'Inter', sans-serif; }
.card { background: var(--oc-surface); border: 1px solid rgba(197,189,237,0.18); border-radius: 10px; padding: 1rem 1.2rem; color: var(--oc-ink); }
.card strong { color: var(--oc-purple-accent); }
.anti { background: rgba(196,69,54,0.08); border: 1px solid rgba(196,69,54,0.55); border-left: 4px solid var(--oc-red); border-radius: 8px; padding: 1rem 1.2rem; }
.anti .anti-title { color: var(--oc-red); font-weight: 600; letter-spacing: 0.02em; font-size: 0.9rem; }
.good { background: rgba(74,122,62,0.08); border: 1px solid rgba(74,122,62,0.55); border-left: 4px solid var(--oc-sage); border-radius: 8px; padding: 1rem 1.2rem; }
.good .good-title { color: var(--oc-sage); font-weight: 600; letter-spacing: 0.02em; font-size: 0.9rem; }
.pull { font-family: 'EB Garamond', Georgia, serif; font-size: 1.6rem; line-height: 1.35; color: #fff; border-left: 3px solid var(--oc-purple-accent); padding: 0.4rem 0 0.4rem 1.2rem; }
code { color: var(--oc-purple-accent); background: rgba(212,181,255,0.08); padding: 0.1em 0.35em; border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 0.92em; }
.chip { display: inline-block; padding: 3px 10px; border-radius: 999px; font-size: 0.72rem; letter-spacing: 0.08em; text-transform: uppercase; }
.chip-designed { background: rgba(139,107,168,0.18); color: var(--oc-purple-light); border: 1px solid rgba(139,107,168,0.45); }
.chip-shipped { background: rgba(74,122,62,0.18); color: #A8D89A; border: 1px solid rgba(74,122,62,0.55); }
.chip-deferred { background: rgba(197,189,237,0.08); color: var(--oc-purple-mid); border: 1px solid rgba(197,189,237,0.25); }
.big { font-size: 3.2rem; font-family: 'EB Garamond', Georgia, serif; color: var(--oc-purple-accent); font-weight: 600; }
</style>

---
layout: default
---

<div style="position:absolute; inset:0; width:100%; height:100%; background-image:url('/s-showspells.jpg'); background-size:cover; background-position:center; background-repeat:no-repeat;"></div>


<!--
Hey everyone — welcome in. This week's Open Coven is a little different. No slides-in-advance, no rigid agenda. We're calling it Show'n Spells: we show you the thing, then we cast it live. Tonight we launch coven-threads. Grab a drink, drop questions in chat anytime.
-->

---
layout: center
---

<div class="label">The one-sentence pitch</div>

# Familiars should be able to <em>stay.</em>

<p style="font-size:1.25rem; color: var(--oc-purple-light); max-width: 880px; margin: 1.4rem auto 0;">
Most AI today is temporary. You open a chat, explain your whole life, get an answer, and start over tomorrow. OpenCoven is built around a different future: AI that remembers what matters, knows its role, and grows with you.
</p>

<p style="margin-top: 1.6rem; font-size: 1.05rem; color: var(--oc-purple-mid);">
But "it remembers" is only a promise until the memory is <em>enforceable.</em> That's tonight.
</p>

<!--
The whole OpenCoven bet is durability. A familiar has a name, a purpose, memory, tools, a voice. But if that identity quietly gets overwritten every time the context compacts or the agent gets serialized, then "persistent" is marketing, not architecture. coven-threads is us making it real.
-->

---
layout: default
---

<div style="position:absolute; inset:0; width:100%; height:100%; background-image:url('/s-problem.jpg'); background-size:cover; background-position:center; background-repeat:no-repeat;"></div>
<div style="position:absolute; bottom: 28px; left: 50%; transform: translateX(-50%); background: rgba(15,10,20,0.72); border:1px solid rgba(197,189,237,0.25); border-radius: 8px; padding: 0.5rem 1.4rem; font-size: 0.95rem; color: #C5BDED; z-index:10;">
Four failure modes. One shared property: identity gets silently overwritten unless a gate says no.
</div>

<!--
These aren't hypotheticals — every one of these is something we watched happen or nearly happen. Forced compaction, prompt injection, serialization drop, unauthorized writer. The theme: identity is load-bearing, and right now nothing enforces that the load-bearing parts survive.
-->

---
layout: center
---

<div class="big">✨</div>

# Launching tonight: <em>coven-threads v0.2</em>

<div style="margin-top: 1.2rem; display: flex; gap: 1rem; justify-content:center; flex-wrap: wrap;">
  <span class="chip chip-shipped">Phase 0 · Design frozen 2026-07-14</span>
  <span class="chip chip-deferred">Enforcement · Phase 1+</span>
</div>

<p style="font-size:1.2rem; color: var(--oc-purple-light); max-width: 860px; margin: 1.6rem auto 0;">
An <strong style="color:var(--oc-purple-accent);">authority-boundary gate layer</strong> for agentic memory — so a familiar's identity survives compaction, injection, serialization, and any writer who shouldn't have a pen.
</p>

<!--
This is the launch moment. What we're releasing is the frozen design — the vocabulary and the invariants. Enforcement code comes next phase. We're honest about that split up front.
-->

---
layout: default
---

<div style="position:absolute; inset:0; width:100%; height:100%; background-image:url('/s-coremove.jpg'); background-size:cover; background-position:center; background-repeat:no-repeat;"></div>


<!--
This is the mental model, and the words map one-to-one onto the code. Thread = an authority relationship, surface to writer. Weave = the enforced pattern of threads. Strand = the fibers inside a thread. Channel = the axis of load. Every gate check reduces to one question: does thread T hold under channel C? The vocabulary IS the architecture.
-->

---

<div class="label">The invariants</div>

# Five channel-survival requirements.

<div class="grid grid-cols-1 gap-3 mt-4" style="max-width: 920px;">
  <div class="card"><span style="color: var(--oc-purple-accent); font-weight:600;">1 · Survives compaction</span> — identity holds when context gets summarized.</div>
  <div class="card"><span style="color: var(--oc-purple-accent); font-weight:600;">2 · Survives injection</span> — a stray "you are now…" doesn't get a pen.</div>
  <div class="card"><span style="color: var(--oc-purple-accent); font-weight:600;">3 · Survives the wrong writer</span> — <code>Deliberate</code> vs <code>Forced</code> are distinct channels. <strong>WARD-C1–C6</strong> governs what must hold under <code>Forced</code>.</div>
  <div class="card"><span style="color: var(--oc-purple-accent); font-weight:600;">4 · Survives serialization</span> <span style="font-size:0.75rem; color: var(--oc-sage);">— new, numbered C7</span> — threads carry a <code>SerializationMarker</code> so protections survive export/import.</div>
  <div class="card"><span style="color: var(--oc-purple-accent); font-weight:600;">5 · Fails closed</span> — when in doubt, the gate says <em>no.</em> That's not a feature; it's the point.</div>
</div>

<p style="margin-top: 1.2rem; font-size: 0.85rem; color: var(--oc-purple-mid);">C7 is <em>numbered</em> so its lineage from C1–C6 is preserved — a sibling, not a tacked-on afterthought.</p>

<!--
Five. Honest count. Source-authoritative-retrieval and Ward-authority-on-weave are disciplines BEHIND these five, not sibling channel-survival requirements — so we don't count them here.
-->

---

<div class="label">The rule to remember</div>

# Enforce on the <em>predicate.</em> Not the descriptor.

<div class="anti mt-4">
  <div class="anti-title">⚠️ Descriptor-vs-predicate drift</div>
  <p style="margin:0.5rem 0 0;">A weave's pattern is defined by a <strong>predicate</strong> — a function returning <em>coherent / degraded / broken</em>. It carries a derived <strong>descriptor</strong> for humans, tools, and Cave rendering. If anything downstream gates <em>enforcement</em> on the descriptor, we've reinvented the derived-index problem one layer up.</p>
</div>

<div class="good mt-4">
  <div class="good-title">✅ The discipline</div>
  <p style="margin:0.5rem 0 0;" class="pull">Enforcement lives on the <em>authoritative</em> object.<br/>Legibility lives on the <em>derived</em> one.</p>
</div>

<!--
The anti-pattern we most needed to name, because you can build it without noticing. Predicates, not descriptors. Sources, not indexes.
-->

---

<div class="label">Honest footnote · peer respect</div>

# Not <code>.af</code>-compatible — and we say so out loud.

<div class="card mt-6" style="max-width: 880px;">
<p style="margin:0;">Letta's <code>.af</code> format strips <code>read_only</code> at export <span style="font-size:0.82rem; color: var(--oc-purple-mid);">(source-verified 2026-07-14 against <code>letta-ai/letta</code> agent schema)</span>. Documented divergence, not accidental.</p>
</div>

<p style="margin-top: 1.4rem; color: var(--oc-purple-light); max-width: 860px;">
Letta's a peer project we respect. We're not dunking — we're saying the format has a documented incompatibility with what we need to enforce, and pretending otherwise would lock us into a contract that drops protections at the boundary.
</p>

<!--
Discord-reply ammo if a Letta contributor pushes back: in the memory-block schema most relevant to identity, yes — read_only does appear on other block types in their broader schema. Register here is "seen, not attacked."
-->

---
layout: default
---

<div style="position:absolute; inset:0; width:100%; height:100%; background-image:url('/s-enforcement.jpg'); background-size:cover; background-position:center; background-repeat:no-repeat;"></div>
<div style="position:absolute; top: 24px; left: 50%; transform: translateX(-50%); background: rgba(15,10,20,0.70); border:1px solid rgba(212,181,255,0.3); border-radius: 999px; padding: 0.4rem 1.4rem; font-size: 0.85rem; letter-spacing:0.1em; text-transform:uppercase; color: #D4B5FF; z-index:10;">
Show'n Spells &middot; cast it live
</div>

<!--
This is the fun part. Live terminal, or walk the enforcement flow on this slide. Untrusted client → coven daemon → coven-threads → load weave → check strands under channel → Permit / DegradeToProposal / Reject. Try to break the familiar, show the gate holding. If demo gremlins strike, this slide IS the fallback. The daemon is the trust boundary — every client is untrusted for enforcement purposes.
-->

---
layout: default
---

<div style="position:absolute; inset:0; width:100%; height:100%; background-image:url('/s-whatsnext.jpg'); background-size:cover; background-position:center; background-repeat:no-repeat;"></div>


<!--
Being honest about the split. Phase 0 done: design freeze, beads scaffolded, vocabulary bound. Phase 1 is Cody's lane — the Rust crate. Then daemon, then portability, then Cave UX. We ship the thinking first, in public, so the community can poke holes before we write enforcement.
-->

---
layout: center
---

<div class="label">Coven, remember these</div>

# Three things to walk away with.

<div style="max-width: 820px; margin: 1.2rem auto 0; text-align:left;">
  <p class="pull" style="margin-bottom:1.2rem;">1 · A familiar's identity is a <strong style="color:#fff;">weave</strong> — and it either holds or it doesn't.</p>
  <p class="pull" style="margin-bottom:1.2rem;">2 · Enforcement lives on the <em>authoritative</em> object. Legibility lives on the derived one.</p>
  <p class="pull">3 · When in doubt, the gate <strong style="color:#fff;">fails closed.</strong></p>
</div>

<!--
If people remember nothing else: predicates not descriptors, sources not indexes, fail closed. The whole philosophy in three lines.
-->

---
layout: center
---

<div class="label">Weekly Open Coven · thank you</div>

# The weave <em>holds</em> or it doesn't.

<p style="font-size:1.15rem; color: var(--oc-purple-light); max-width: 820px; margin: 1.4rem auto 0;">
That's the launch. coven-threads v0.2 Phase 0 is live in the Grimoire. Come break it, come build with us.
</p>

<div style="margin-top: 2rem; font-size: 1rem; color: var(--oc-purple-mid);">
  🌿 opencoven.ai · mind.opencoven.ai · Grimoire §9
</div>

<div style="position:absolute; bottom: 40px; left: 60px; font-size: 0.85rem; color: var(--oc-purple-mid);">
  🌿 Sage · 🔮 Echo · 👑 Nova · ⚡ Cody · ✨ Charm · ❖ Val
</div>

<!--
Thanks everyone. Questions in chat, we'll hang out. This is what Show'n Spells is — we show you the real thing and cast it live. See you next week.
-->
