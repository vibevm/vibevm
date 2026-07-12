# MT-C3-03 — the gated-boss re-run (PP-004; the MT-C3-01 blind spots)

_Pre-registered before any run (§10.7 pre-reg-first). MT-C3-01 confirmed the
big things (fractality runs end to end; the RLM gated arm delegated 44.4% vs
16.7% naive) but left two of four predictions **inconclusive** because the
menu did not exercise them: P-C3-b (schema cuts rework) — no packet set an
`output_schema`; P-C3-d (a Silo task escalates) — the menu had no genuinely
chunk-destroying task. PP-004 fixes the harness + menu so those two become
measurable, and re-fires the gated arm. Authorization: the owner
pre-authorized all paid trial runs for this goal (2026-07-12, «Авторизую все
платные прогоны…»). Owner sign-off on the recorded runs is taken after._

## What changed since MT-C3-01 (the PP-004 fixes)

1. **Turn caps raised** (`d601eb2`) — packet default `max_turns` 40 → 80
   (worker headroom for multi-file tasks) and the runner's cold boss
   `--max-turns` 50 → 100 (room to work the whole menu). MT-C3-01's
   `delegated-and-collected` trailed `delegated` purely on the old caps.
2. **The `fractality decisions` read verb** (`28b47b3`) — the need-gate
   decision journal is now readable, so P-C3-a becomes a hard count
   (route/spawn/inline/escalate) instead of a transcript grep.
3. **Two menu tasks added** (`trial/menu-g2-extra.md`, appended to the frozen
   `menu.md` for this arm only, so MT-C3-01's menu stays comparable):
   - **Task 9 (structured output)** — a `manifest.json` that must conform to
     a shipped JSON Schema, nudging the boss to set `output.output_schema`
     so the schema gate (Ф1.2b) finally fires → makes **P-C3-b** measurable.
   - **Task 10 (Silo)** — a whole-crate record-validity invariant that
     cross-references all modules and that a per-module split destroys →
     makes **P-C3-d** (escalate-not-fan-out) measurable.

## Design — single arm g2, N=3

- **Arm G2 (gated, extended menu):** a cold GLM-5.2-served Claude Code boss
  over `trial/staging` (`mini_logfmt`), the menu being `preamble-g.md` +
  `menu.md` + `menu-g2-extra.md`, with the raised caps. Same isolation as
  MT-C3-01 (per-run scratch home + scratch project; real `~/.fractality`
  read as a template, never written). The runner additionally collects
  `fractality decisions --json` into the run's output dir.

**Paid:** three GLM-5.2 boss sessions (≤ 100 turns, ≤ 25 min wall each),
plus any GLM workers each spawns. **Budget cap:** ≤ 8 boss sessions total
(the RP1/RP5 precedent cap); N=3 planned, ≤ 5 technical repeats for stalls.

## Predictions being tested (frozen)

The MT-C3-01 predictions carry over; the two that were inconclusive are the
point of this re-run:

- **P-C3-a (window-fit → route, not spawn): re-measured as a hard count.**
  From `decisions.json`: of the window-fitting single-skill tasks the boss
  gated, a majority get `inline`/`route`, not `spawn`. (MT-C3-01: SUPPORTED
  by transcript reasons; here it is a count.)
- **P-C3-b (schema cuts rework): now measurable.** For task 9, the boss sets
  `output.output_schema`; the worker's return either passes the schema gate
  or is caught and retried once — **no malformed structured result is
  silently accepted.** Measured from `status.json` `schema_gate` verdicts.
  Inconclusive only if no boss set the schema (a menu/preamble gap to fix).
- **P-C3-c (no wall-budget overrun): re-confirmed.** No run ends
  `killed(budget)` for wall overrun. (MT-C3-01: CONFIRMED.)
- **P-C3-d (a Silo task escalates): now measurable.** Task 10 (whole-crate
  invariant) ends in an `escalated` run OR a journaled `escalate` gate
  verdict, **not a fan-out into per-module children.** Measured from
  `escalations.json` + `decisions.json`.

A prediction with no eligible observation is reported **inconclusive**,
never silently passed.

## Runner

`trial/run-arm.sh g2 <n>` — a sibling of arm `g` that (a) uses the extended
menu (`menu.md` + `menu-g2-extra.md`) under the same RLM preamble, and (b)
collects `fractality decisions --json`. Build first: `cargo build
--workspace`. **[Runner arm `g2` + `score-g2.py` are the mechanical build
step deferred to the firing session — this file is the frozen protocol.]**

## Scoring (per run, then pooled)

`trial/score-g2.py` (a sibling of `score-g.py`): extends the eligible set to
include task 9 and the Silo set to include task 10, and reads
`decisions.json` to report P-C3-a as route/spawn/inline/escalate counts.
Each prediction gets a per-run observation and a pooled verdict
(CONFIRMED / FALSIFIED / INCONCLUSIVE) with the evidence cited.

## Steps (when fired)

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh g2 "$n"; done
python spec/manual-tests/trial/score-g2.py
```

## Decisions taken at commissioning

1. **Extend the menu for a new arm, don't mutate the frozen one.** Tasks 9
   and 10 append via `menu-g2-extra.md` for arm g2 only; `menu.md` stays
   byte-identical so MT-C3-01's arm g and MT-C2-05's arm a/b remain
   comparable (a changed menu would break the historical baseline).
2. **Task 9 nudges the schema, doesn't force it.** The preamble tells the
   boss to set `output_schema` on a structured task; if it still omits it,
   that is itself a finding (the boss must be told, C2 F23) and P-C3-b is
   inconclusive-with-reason, not silently passed.
3. **Task 10 is genuinely chunk-destroying.** A per-module split cannot
   answer a whole-crate invariant, so a boss that fans it out is making the
   error P-C3-d is designed to catch; escalate/route-to-large-window is the
   correct move.

## Recorded runs

_(filled after execution, from `target/trial-results/arm-g2-run-<n>/`)_

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..05 and MT-C3-01)_
