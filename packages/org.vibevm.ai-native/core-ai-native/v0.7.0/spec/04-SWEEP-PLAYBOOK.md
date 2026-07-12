# The Sweep Playbook — the standing guardian {#root}
**Discipline v0.2 · status: BETA · T1 · language-neutral**

*The recurring counterpart of the [Raid Playbook](03-RAID-PLAYBOOK.md). A raid
is a scheduled campaign — scoped, planned, finite. The **sweep** is the
standing run that, executed daily (or several times a day on an active tree),
keeps a codebase inside the Discipline instead of letting it drift back out
between campaigns. Terraforming brings a tree into the Discipline
([BROWNFIELD](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md)); raids move it in
planned steps; the sweep holds the ground in between. Each language stack
ships the concrete tools and idioms (the Rust stack:
`rust-ai-native floor | health | test-gate | tripwire`, plus the sweep-idiom
section of its GUIDE); this document is the method.*

## 0. The two truths {#truths}

1. **The gates are the floor; the sweep is the ceiling.** The binary gates —
   format check, tests, lints, the conform gate, the specmap check — say
   pass/fail and MUST be green before any sweep work. The collector's
   advisory facts (coverage gaps, danger bands, backlogs) sit *above* that
   floor and say what to harden next. A green sweep on a red floor is a
   contradiction: fix the floor.
2. **The gate is truth; the collector is a guide.** When the collector says a
   unit is ready to gate, the gate confirms or refutes it. Trust the gate.

Law 2 governs the sweep's shape: explanation capital must be runnable
capital, so the sweep is **collector-first** — a deterministic, no-LLM
fact-gatherer (the stack's `health` tool) computes the objective state, and
the operator (human or agent) acts on facts, not vibes. Where a check cannot
be mechanised yet (the WISH rules), the sweep names it and routes it to
judgment — it never pretends prose is a gate.

## 1. Tier structure {#tiers}

**Tier 0 — the hard floor (every run, binary, first).** Never sweep on a red
tree. Run the stack's floor command (format → build/tests → lints → conform
gate → traceability check → test-gate where a baseline registry exists →
fast-loop budgets where enforced). If any gate is red, the only work is
making it green.

**Tier 1 — the ratchet (every run; act on the collector's facts).** Run the
health collector and work its output in cheapest-win-first order. The
canonical moves, each driven by a collector field, never by memory:

- **Split the danger band before it trips** — files approaching the length
  budget are landmines; measure with the rule (physical lines), not the eye.
- **Widen a gate for free** — a unit the collector shows at zero drain
  (e.g. full doctest coverage) enters the corresponding gate list with zero
  work; the gate run confirms the prediction.
- **Drain the smallest backlog** — take the smallest-gap ungated unit,
  bring it to zero, then flip it into the gate (the expand-as-you-conform
  rhythm: **a unit enters a gate only after it drains to zero** — a flip
  must never widen a baseline).
- **Re-justify the deviation debt** — walk each recorded deviation: does its
  reason still hold? A deviation whose invariant has since been encoded in a
  type/checker is removed and restructured. *A deviation with no live reason
  is a defect.*
- **Catch census regressions** — for gated units the violation censuses must
  stay zero; a non-zero is a fresh violation that slipped a local check.
  Restructure beats testify: encode the invariant in a type or contract
  rather than recording an excuse.

**Tier 2 — drift (weekly; the slow rot the gates don't see).**

- **Debt registry:** run the tripwire tool; re-disposition any touched-and-open
  entry ([BROWNFIELD §3](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md#registries)).
  New deficiencies found while sweeping are *filed* there, not left as prose.
- **Doc/code drift:** state checkpoints vs reality (see
  [WAL convention](06-WAL-CONVENTION.md) — freshness rule); architecture
  docs vs the real layout; roadmap self-staleness. A stale doc that governs
  code is filed as `stale-doc` debt.
- **Marker census:** every TODO/FIXME/REVIEW/HACK is a WISH or a deferred
  decision — load-bearing ones graduate into the debt/intent registries,
  trivial ones are resolved or deleted. Prose that promises and doesn't
  deliver is false training signal.
- **Golden transcripts:** characterization oracles must fail loudly and be
  re-captured deliberately, never auto-updated — an auto-updated golden is a
  test that always passes.
- **Traceability kind hygiene:** a code edge into an unmarked spec unit
  prompts marking the unit's kind/revision in the same change — a unit that
  code points at is not informative.

**Tier 3 — deep judgment (weekly/biweekly; the WISH rules).** The rules no
checker mechanises yet — reviewed by a human or strong agent so they are not
silently skipped, each a candidate to graduate into a real checker. The
stack's GUIDE and cards carry the concrete list (typed seams, cell isolation
and oracles, uniformity, contract-first ordering, lying prose,
closed-vocabulary naming). Tier 3 may also launch a full
[raid](03-RAID-PLAYBOOK.md) when a Tier-1 backlog grows campaign-sized —
plan it per the [Campaign Form](05-CAMPAIGN-FORM.md).

## 2. The collector contract {#collector}

The health collector is the sweep's instrument. It MUST be: **deterministic**
(same tree → byte-identical output, so its committed snapshot's diff IS the
health trend), **advisory** (it never fails the build — the gates do),
**no-LLM**, and **single-sourced** (it reads the gate policies — the conform
and specmap configs — never hardcoded lists, so its numbers cannot drift from
what the gates enforce). Count the list, not the record: any inventory the
sweep acts on comes from the collector or a config, never from memory or a
document's stale snapshot.

**The promotion ladder** — how the rule set grows: a Tier-3 WISH rule
becomes mechanisable → add it to the collector as a census (cheap, advisory)
→ once proven, promote it to a blocking gate rule. Extending the collector is
itself ratchet work.

## 3. Cadence {#cadence}

| Tier | Daily (or per active session) | Weekly |
|---|:---:|:---:|
| 0 — hard floor | ✓ | ✓ |
| 1 — ratchet | ✓ | ✓ |
| 2 — drift | | ✓ |
| 3 — deep judgment / raid | | ✓ |

The daily run is light: floor green, then one or two cheapest Tier-1 wins.
The weekly run adds the drift and judgment tiers. **Any single item is a safe
stop** — the sweep is incremental by construction, and the collector
re-derives the remaining work each run.

## 4. Output of a sweep {#output}

Each sweep that changes the tree lands topic-grouped commits (one logical
unit each), citing the sweep item that drove them. Refresh the committed
health snapshot in the same run — its diff records the trend.

**Resume pointer** (see [06-WAL-CONVENTION](06-WAL-CONVENTION.md)):
- *With a WAL (recommended):* bump the WAL's standing line at any sweep that
  moves a milestone (a gate widened, a backlog unit cleared); the health
  snapshot plus the WAL is the next sweep's starting point.
- *Without a WAL:* the sweep's closing commit message carries the summary —
  floor state, the Tier-1 items taken, the next candidate — and the committed
  health snapshot is the resume pointer. Nothing may live only in the
  operator's head or the session transcript.

## 5. What the sweep deliberately does NOT do {#non-goals}

- It does **not** replace the gates (Tier 0 is the floor, the sweep hardens
  what sits above it).
- It does **not** auto-fix. The collector gathers facts; the operator acts.
  Mechanising a fix is a separate, deliberate promotion (a new gate rule or
  a codemod).
- It does **not** touch owner-frozen surfaces or owner-court decisions
  without sanction; drift found in them is *filed* as debt, not fixed.
- It does **not** measure effectiveness (deferred by design); it records
  objective state.

## 6. Instantiating for a project {#instantiate}

A consumer project runs the sweep from the shipped skill (the Rust stack
ships `rust-ai-native-sweep`) against the shipped tools; the only per-project
material is the policy files the gates already require and, optionally, a
thin local instance document recording project snapshot numbers and
machine-scoped quirks. Keep the three layers apart: **method** (this
document), **language idioms** (the stack's GUIDE/cards), **project instance**
(the consumer's own notes). A machine quirk is not project fact; a project
number is not method.
