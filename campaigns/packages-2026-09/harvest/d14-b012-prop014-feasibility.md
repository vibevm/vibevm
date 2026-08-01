# B-012 · PROP-014's ten unbuilt mechanisms — the feasibility study {#root}

**Date:** 2026-08-01 · **HEAD at synthesis:** `3710ae2f` (+ the uncommitted F-133
repair, landed the same day) · **Owner directive:** `BACKLOG.md` B-012 —
«провести исследование, можно ли реализовать» — before any Phase E scheduling.

**What this document is.** The boss synthesis over two evidence passes —
[`d14-b012-part-A.md`](d14-b012-part-A.md) (runtime/distribution family) and
[`d14-b012-part-B.md`](d14-b012-part-B.md) (graph/check/explain family) — both
evidence-only, both boss-reviewed against the tree (spot-checks re-run:
`#[cfg(test)]` echo, the three MCP rosters, `codegen.rs:50-52`, the 9-line
`vibe-llm`, self-check's specmap steps). The **verdicts and recommendations
below are the boss's**, marked as proposals: nothing here is scheduled until
the owner rules. Genre: campaign harvest, non-binding.

**The one-line answer to «можно ли реализовать»:** yes for all ten — nothing is
architecturally impossible — but they divide sharply into **four cheap,
unblocked slices a Phase E could take now**, and **six that are gated on
exactly three keys**, two of which are the owner's to turn and one of which is
spec-authoring, not code.

## SUPERSEDED IN PART — owner rulings, 2026-08-01, same day {#rulings}

The owner ruled on the whole set the day the study landed; **the
recommendation column of the verdict table below is superseded by these
rulings** and survives only as the reasoning trail. The rulings, executed as
`BACKLOG.md` entries with full plain-language descriptions:

- **Security (key 1) → [B-015](../../../BACKLOG.md#b-015), parked**: protocol
  the tasks in detail, build nothing until the owner's explicit notice — usage
  of the product is observable only from the owner's seat, not from the tree,
  so re-opening is his call alone; no code-observable trigger is assigned.
  The «build the channel before the signature» re-sequencing this implies is
  recorded there (task 6: the PROP's ships-signed-or-not-at-all sentences get
  an owner-approved amendment when B-018 is built).
- **Labels (key 2)**: the recommendation stands as ruled — the ~80-section
  targeted set + label-new-sections-forward, and only when the dependent
  features are revived; folded into B-019's (a) as its spec-side twin.
- **Thresholds (key 3) + the multiplicity lint → [B-021](../../../BACKLOG.md#b-021),
  build both warnings**: zero findings on this corpus is not an argument —
  «мы пишем систему для всех, а не только для нас»; thresholds configurable,
  placeholders until the warnings themselves collect real statistics.
- **M1 → [B-016](../../../BACKLOG.md#b-016), build** (both halves, ordered).
- **M3 → [B-017](../../../BACKLOG.md#b-017), build.**
- **M4 → [B-018](../../../BACKLOG.md#b-018), build — high priority, wide form**
  (including answers about installed/foreign packages).
- **M7 → [B-019](../../../BACKLOG.md#b-019), build all three parts,
  algorithmic, no LLM**; the (c) system-boundary question is written out there
  and is the mandatory first step of that implementation.
- **M8 → [B-020](../../../BACKLOG.md#b-020), build** — the owner's direction:
  a light client for external non-local LLMs, possibly through fractality,
  settled at build time.
- **B-012 itself → `done`**: the research question is answered; the backlog
  entries above are the drain.

## The three keys that gate everything else {#keys}

1. **The signing-scheme decision** (`#OPEN-SIGNING-SCHEME`, PROP-014 :415 —
   «sigstore vs. minisign-class vs. registry-native git signatures; blocking
   for §2.8»). The PROP's own standing position — *the trust layer ships WITH
   the runtime channel, not after it* (:242, :297, :369) — is currently doing
   its job: it is what keeps M4 (MCP tools), and therefore M3 (profiles) and
   M1-half-1's only named consumer, unbuilt. **This is an owner decision and
   Rule-4 territory (CI/signing); no work starts before the ruling.**
2. **Kind-line authoring in the host corpus.** Measured twice independently:
   **0 of 5266** host spec units carry a `kind` or `revision`. The parser is
   complete (`mdspec.rs:63`), demo projects populate it — the gap is authored
   data, not code. It blocks M2's revision half, M5(c)'s guide carve-out, makes
   M7(a)'s twin discipline inert, and leaves M11 with no `prop` node to point
   at. **This is spec-authoring work, schedulable independently of any build.**
3. **Phase 3 metrics** (`#OPEN-THRESHOLDS` :414, `#PHASE-3` :363 — unrun).
   Both numeric thresholds in the set (3 edges/item for M6, 120 lines/unit for
   M9) are placeholders by the PROP's own words until that phase runs.

## Verdict table {#verdicts}

| # | mechanism | feasible? | effort | blocked on | recommendation (proposal) |
|---|---|---|---|---|---|
| M1 | index ships in package + fetch-by-hash | yes | S + L | consumer (M4); «what is a code fragment» | **defer both halves**; trigger: M4 ships, or `[[external_specs]]` re-derivation measured painful |
| M2 | error-rendering index lookup + hint | yes | S (+M data) | nothing (S slice); kind lines (revision); naming (`vibe explain`) | **build the S slice in Phase E**; revision waits for key 2; hint waits for `#PHASE-4-PROMOTE-XTASK-TO-VIBE` |
| M3 | `[metamodel]` profiles | yes | S key / L meaning | M4, M1; `#OPEN-NON-OSS-CONTRACT-PROFILE` | **defer whole** — do not land an inert key into a `deny_unknown_fields` manifest |
| M4 | `specmap_query` / `specmap_source` / channel | yes | S/M/L/L | **key 1** (its own :369: «ships signed or not at all»); query grammar undefined; M1 | **defer all four pieces**; the unlock is key 1 |
| M5 | trust layer (sign / frame / lint) | yes | L / S / M | key 1 (sign); key 2 (lint) | **(b) frame now in Phase E**; (a) = key 1, owner-only; (c) waits for key 2 |
| M6 | edge-multiplicity lint | yes | S | key 3 (threshold); placement seam | **defer** — fires on zero items (max fan-out 2); build when Phase 3 instruments |
| M7 | `CodeItem.content_hash` + `Command`/`ErrorVariant` views | yes | M / L / M | broken codegen route (B-013); «what is hashed»; seam | **defer (a)** — ride the first forced schema bump; **(b) stays annotated** (no consumer anywhere); **(c) defer** to the channel |
| M8 | LLM prose producer | yes | M relay / L in-process | M10 in substance (input too thin); v1.5 milestone (`vibe-llm` is a 9-line stub) | **defer**; the honest order is M10 first, then judge |
| M9 | spec-unit length warning | yes | S | grain decision (owner); key 3 (number) | **build leaf-scoped, host-side (`vibe-spec`), in Phase E**; engine-side serialised warning waits for the grain ruling |
| M10 | rustdoc composition in `explain` | yes | S query-time / M serialised | volume/truncation (serialised form only) | **build the query-time form in Phase E** (no schema change — the orphan table's own posture); defer the serialised field |
| — | rider: `decides` verb (M11) | (a) yes / (b) no as modelled | S / L | key 2 (no `prop` nodes exist); edge model has no spec→spec representation | **no build**; the cell is already annotated in place — the sanctioned form; revisit at key 2 |

## The Phase-E-ready shortlist (S-class, unblocked, four items) {#phase-e-ready}

1. **M2's doorway slice.** One chokepoint (`vibe-cli/src/output.rs:212`), one
   existing enrichment hook (`stamp_structured_error`, `:238` — its own doc
   frames it as the extension point), one existing tolerant loader
   (`progress_evidence.rs:39`), and a measured **81/81** hit rate — every
   `violates spec://` URI in `crates/**` resolves in the committed index.
   Failure posture: absent/stale index degrades silently to today's output.
   The `run: …` hint text is NOT in this slice (the command it would name does
   not exist; promotion is Phase 4's own item).
2. **M5(b) framing.** The exposure §2.8.4 warns about **already ships** by two
   routes — `read_subskill` (`vibe-mcp/src/tools.rs:158`) pipes installed-package
   prose into agent context, `[boot_snippet]` (`package.rs:500`) injects it at
   session boot — unsigned and unframed. Description-string edits are S.
   One boundary to state in the diff: `agentic_explain`'s «follow it» framing
   is PROP-018's **consented instruction relay working as designed** — the
   framing rule applies to *reference* surfaces, not to the relay.
3. **M9 leaf-scoped, host-side.** `vibe-spec`'s IR already carries the length
   (`doctree.rs:59`, `span: Range<usize>`); leaf-scoped it fires on **3** real
   units today (vs 72 under the literal span rule, 43 of which are `#root`
   document anchors — a genre measurement, not a unit one). Host-side placement
   avoids the engine's serialised-warning churn entirely.
4. **M10 query-time.** The doc comment is already in the scanner's hand and
   dropped by one match arm (`rscan.rs:120`); `explain` already rebuilds
   in-memory per invocation (`trace.rs:9-12`), so composition at query time
   needs **no schema bump, no index growth, no vendor propagation** — the same
   deliberately-not-serialised posture the orphan table records
   (`ratchet.rs:22-24`). This is also the substantive input M8 was specified to
   consume.

## Owner decision points, exactly four {#owner-points}

1. **Key 1 — the signing scheme** (unlocks the M4→M3→M1 chain; Rule-4 topic).
   The study takes no position on the scheme; it records that the «ships
   signed or not at all» position is currently load-bearing.
2. **M9's grain** — leaf-scoped (3 findings, recommended) vs literal span
   (72, of which 43 are h1 document anchors; top offenders are campaign logs
   and the owner-frozen `VIBEVM-SPEC.md#root`, genres a lint cannot ask to
   change).
3. **Key 2 scheduling** — whether kind-line authoring over the host corpus
   (the enabler for three mechanisms) is worth a Phase E work-slice of its own.
4. **M7(b) `Command` view** — stays annotated (recommended) or is removed
   outright under `#UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC` (:425);
   nothing in the tree asks the index about commands, and the cell is already
   in the ruled annotate-in-place form.

## What the study itself surfaced and filed {#filed}

- **B-013 (P2, BACKLOG):** the schema-bump path is broken before anyone needs
  it — `xtask/src/codegen.rs:50-52` routes specmap codegen to a
  `rust-ai-native-lang/v0.5.0` slot that does not exist (only `v0.7.0` does);
  `schemas/specmap.jtd.json`'s own metadata still names the pre-relocation
  `crates/specmap-core/...`; and the engine header names a package-local
  `schemas/specmap.jtd.json` that is absent. Three coordinates of one stale
  relocation, met first by whoever attempts M7(a)/M10-serialised/M3-contract.
- **B-014 (P2, BACKLOG):** the committed host `specmap.json` has drifted
  ungated — **599 of 5266** spec units' recorded `line` no longer lands on its
  anchor at HEAD (the code side holds: 898/912 edges land on a marker line).
  No gate lies: `self-check.sh`'s specmap steps are the *packages'* `--gate`
  self-traces (`tools/self-check.sh:366-375`), and no host-index freshness
  check exists anywhere — which is exactly the out-of-gate-drift class the
  health-audit owns. The A–D inventory at the exit gate should see it.
- **F-133 (closed same day):** the партія-1a annotation at PROP-014 :56
  carried one false clause («only shipped tool is echo» — echo is a
  `#[cfg(test)]` fixture; the transport ships zero tools). Repaired
  verdict-first under §3.6(a): re-judged drift → registry minted F-133 →
  one-clause fix + scoping sentence → re-judged confirmed → sealed. Registry
  returned to 108 / 232 with F-133 resolved to history.

## Method and honesty {#method}

Everything above traces to the two parts' file:line evidence; the parts state
their perimeters per absence claim and their own freshness caveat (all
index-derived distributions are properties of the committed artefact, which
B-014 records as stale on the spec side). Numbers quoted here were re-measured
by the workers at `ed0abbab` and spot-checked by the boss at `3710ae2f`; anyone
re-running should take HEAD's own measurements. The workers were told to gather
evidence only; every verdict, effort class synthesis, and recommendation above
is the boss's judgment over their evidence, per the campaign's delegation law
(the decision is never delegated).
