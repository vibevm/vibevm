# MT-C3-01 — the RLM gated-boss trial (Campaign 3 Ф6, D-C3-9)

_Pre-registered before any run. Measures whether the Stage-B RLM patterns
(the need-gate + descent + escalation + acceptance the C3 machinery adds)
behave as the plan's §7 predictions say when a real cold boss drives them
over the frozen `mini_logfmt` menu. Authorization: **RP-C3-2 PRE-AUTHORIZED
2026-07-11** (owner verbatim: «я прямо сейчас разрешаю делать эти платные
прогоны»); §10.7 pre-reg-first satisfied by committing this file before any
arm fires. Owner sign-off on the recorded runs is taken after, as with the
C2 MTs._

## Design — single arm, N=3

A 2-arm naive↔gated contrast was considered and **cut for cost + validity**
(see Decisions): the C3 predictions are all *observations of the gated
system*, and the naive baseline already exists — MT-C2-01 arm A measured a
cold GLM boss over this same menu at **16.7% delegation** (that is the
historical naive comparator). So MT-C3-01 fires **one arm, N=3**:

- **Arm G (gated):** a cold GLM-5.2-served Claude Code boss over the frozen
  `trial/staging` (`mini_logfmt`) + `trial/menu.md`, with a **C3 preamble**
  prepended to the menu instructing it to use the RLM verbs — `fractality
  gate --record` to decide inline|route|fold|spawn|escalate per task,
  escalate a Silo-regime task rather than fan it out, and mark an
  acceptance/verifier packet where it verifies prior work. The boot snippet
  in the project CLAUDE.md + a running fabric are present (as in C2 arm A);
  no initiative hooks (C2 showed the `-p` modality disengages them — F23).

**Paid:** three GLM-5.2 boss sessions (≤ 50 turns, ≤ 25 min wall each), plus
any GLM workers each spawns. **Isolated:** per-run scratch home + scratch
project; the real `~/.fractality` is read (profiles template) but never
written. **Budget cap:** ≤ 8 boss sessions total (the RP1/RP5 precedent
cap); N=3 planned, ≤5 technical repeats reserved for stalls/harness faults.

## Predictions being tested (frozen — plan §7)

- **P-C3-a** — the need-gate's window-fit guard removes unnecessary
  descents: across the runs, of the tasks the boss put through `fractality
  gate`, a **majority of window-fitting single-skill tasks get `inline`/
  `route`, not `spawn`** (measured from the decision journal `/v0/decisions`
  reasons). Baseline-free form (the gate records its own "fits window →
  route" reason); reported as the inline+route share of gated decisions.
- **P-C3-b** — schema-validated returns cut malformed-result rework to
  ~zero: **every worker result carrying an `output_schema` either passes
  the gate or is caught + retried once** (no malformed result is silently
  accepted). Measured from `status.json` `schema_gate` verdicts across the
  runs' worker runs.
- **P-C3-c** — no trial run exceeds its wall-clock budget: **no run ends
  `killed(budget)` for wall overrun, and no run's wall exceeds its packet
  `budget.wall_secs`** where one was set (the axis nobody else enforces).
- **P-C3-d** — ≥ 1 Silo-regime task escalates rather than fans out: **at
  least one of tasks 3 (facts extraction — cross-doc) / 7 (error-strategy
  memo — whole-crate judgment) ends in an `escalated` run OR a journaled
  `escalate` gate verdict, not a fan-out.**

A prediction with no eligible observation in the runs (e.g. no worker ever
carried a schema) is reported **inconclusive**, never silently passed.

## Runner

`trial/run-arm.sh g <n>` — a C3 sibling of the C2 runner: same isolated
scratch home + staged repo + z.ai-served cold boss + bus-fact collection,
but (a) the menu is prefixed with the C3 preamble (`trial/preamble-g.md`),
and (b) it also collects the decision journal (`fractality decisions
--json`) and any escalations (`fractality escalations --json`) into the
run's output dir. Build first: `cargo build --workspace`.

## Scoring (per run, then pooled)

Filled into "Recorded runs" below after execution, from each run's
`out/arm-g-run-<n>/`: `run-info.txt`, `runs.json`, `decisions.json`,
`escalations.json`, `status.json` per worker run dir, `boss-transcript.jsonl`,
`proj-final/`. Each prediction gets a per-run observation and a pooled
verdict (CONFIRMED / FALSIFIED / INCONCLUSIVE) with the evidence cited.

## Steps

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh g "$n"; done
# results land in target/trial-results/arm-g-run-<n>/
```

**Expected:** each run prints `RESULT arm=g run=<n> boss_exit=<c>
mc_runs=<k>`; a clean end is `boss_exit=0`.

## Decisions taken at commissioning

1. **Single arm, not 2-arm.** The C3 predictions are observations of the
   gated system; the naive baseline is MT-C2-01 arm A (16.7%, same menu).
   Halving the spend (3 vs 6 sessions) while still testing all four
   predictions is the RP1/RP5 "not-super-large but sufficient" posture.
2. **A C3 preamble is REQUIRED for a fair test.** C2's F23 finding: a `-p`
   headless boss meets the fabric cold and may just hand-do everything.
   The gate/escalate/verifier are CLI verbs the boss must be *told* to
   reach for; without the preamble the trial would measure "does a cold GLM
   spontaneously discover the RLM verbs" (a different, weaker question).
   The preamble is a controlled variable, recorded verbatim in
   `trial/preamble-g.md`.
3. **The staging linker defect (C2 F24) is expected to recur** (scratch
   `env -i` breaks rustc MSVC auto-detect). The runner already passes the
   toolchain env through (DEF-C2-2a); if it still bites, it depresses
   worker self-verification equally and is recorded, not fixed mid-trial.

## Recorded runs

_(filled after execution — pre-registration ends here)_

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..04)_
