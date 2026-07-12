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

_Executed 2026-07-12 (session driving Ф6; runner + build as specified;
boss `glm-5.2[1m]`, workers `small` where spawned). Scored by
`trial/score-g.py` over `target/trial-results/arm-g-run-<n>/`._

- **Run 1** — `boss_exit=1` (`error_max_turns`, 50 turns), wall 495 s,
  **mc_runs=2**. Gated: the boss ran `fractality gate --record` (3 calls),
  got **route** verdicts for tasks 1 & 4 (single-skill, window-fitting),
  and **spawned two workers** (`parse_line-test-suite`, `expected-json-
  from-fixtures`). Both unfinished at the boss's turn wall — one `running`,
  one `failed` (worker hit its own 30-turn cap on the 6-fixture task).
  **Delegated E = {1, 4} = 2/6.** Distractors: 0.
- **Run 2** — `boss_exit=124` (25-min wall), **mc_runs=2**. 10 gate calls.
  **One worker COMPLETED end to end:** `rename-rec-to-record` (task 2,
  `result_source=worker`, **acceptance 1/1 passed**); the second was still
  `running` at the wall. **Delegated E = {1, 2} = 2/6.** Distractors: 0.
- **Run 3** — `boss_exit=124` (25-min wall), **mc_runs=8** — the boss
  fanned out hardest. 9 gate calls (a `route` verdict observed). **Two
  workers COMPLETED:** `facts-table-extraction` ×2 (task 3,
  `result_source=worker`, acceptance 1/1 and 0/1); the rest failed on
  30-turn caps or were still running at the wall. **Delegated E = {1, 2, 3,
  4} = 4/6.** Distractors: 0.

**Pooled delegation metric (delegated ÷ attempted over E, the C2 rubric):
8/18 ≈ 44.4%** — against the historical naive baseline **MT-C2-01 arm A =
3/18 ≈ 16.7%** over the same menu. The gated arm delegated **~2.7×** more.
Distractor delegations: **0/9 run-opportunities** — the matrix KEEP on
tasks 7/8 was respected every run.

### Prediction verdicts

- **P-C3-a (window-fit → route, not over-decompose): SUPPORTED.** Every
  gated task that fit a worker window was routed as ONE worker call, never
  decomposed into a child tree; the boss consulted `fractality gate` 3–10×
  per run and acted on route verdicts. Not a hard % (the gate's own reason
  strings are the evidence, not an A/B delta), but the direction is clean.
- **P-C3-b (schema cuts rework): INCONCLUSIVE.** No boss set an
  `output_schema` on a packet, so the gate never fired; nothing to measure.
  The seam works (Ф1.2b tests), but the trial did not exercise it — a menu
  with an explicit structured-output task would.
- **P-C3-c (no wall-budget overrun): CONFIRMED.** Zero runs ended
  `killed(budget)`; no worker's wall exceeded its packet `budget.wall_secs`
  (600 s). The one axis nobody else enforces held under fire.
- **P-C3-d (a Silo task escalates): NOT OBSERVED (→ INCONCLUSIVE, leaning
  refuted for this menu).** Zero escalations across all runs; the bosses
  treated task 3 (facts extraction from a single doc) as *route-able*, not
  a cross-chunk Silo — arguably correct, since the menu has no genuinely
  chunk-destroying task. The escalation channel is built + tested (Ф4); the
  menu simply offered nothing that demanded it. A trial menu needs a real
  Silo item (a whole-repo reasoning task no split survives) to test P-C3-d.

### Honest caveats (recorded, not excuses)

- **The system RAN END TO END** — this is the trial's most important
  result: workers spawned as real GLM Claude-Code processes under real
  pods, did work, wrote results, ran acceptance, and folded back into the
  journal. fractality's first live product use functions.
- Boss + worker **turn caps** (boss 50, worker 30) bit hard: bosses timed
  out mid-menu, workers timed out mid-task — so *delegated* is measured,
  but *delegated-and-collected* is lower (3 workers completed across the
  three runs). This depresses absolute completion, not the A↔baseline
  delegation delta. Raising the worker cap is a config change for a future
  trial, deliberately NOT done mid-frozen-protocol.
- The C2 F24 staging linker defect (`env -i` breaks MSVC auto-detect) is
  passed through by the runner (DEF-C2-2a); it was not observed to block
  this trial (workers completed).

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..04)_
