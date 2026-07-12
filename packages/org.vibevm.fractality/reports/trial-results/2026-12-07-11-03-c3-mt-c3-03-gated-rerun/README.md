# C3 — the PP-004 gated re-run (MT-C3-03)

_Fired 2026-12-07 10:09–11:03 (Campaign 3, PP-004). Three paid GLM-5.2
cold-boss runs, arm `g2`: the RLM gated arm over `mini_logfmt` + the extended
menu (`preamble-g.md` + `menu.md` + `menu-g2-extra.md` — the frozen 8 tasks
plus task 9 schema and task 10 Silo), with the raised caps (worker 80 / boss
100) and the `decisions` journal collected. Pre-registration: MT-C3-03. Scored
by `score-g2.py`._

## What this measured

Whether the two PP-004 menu additions make MT-C3-01's inconclusive
predictions measurable: **P-C3-b** (a structured-output task exercises the
schema gate) and **P-C3-d** (a Silo task escalates rather than fans out); plus
**P-C3-a** re-measured as a hard count now that `fractality decisions` reads
the need-gate journal back.

## Runs

| run | boss_exit | workers | delegated E | note |
|---|---|---|---|---|
| 1 | 0 (clean) | 4 (all failed) | {1,3,4,9} = 4/7 | delegated the schema task; workers hit the staging linker / caps |
| 2 | 124 (25-min wall) | 5 (3 completed, 2 running) | {1,3,4,9} = 4/7 | best collection; timed out mid-menu |
| 3 | 1 (early) | 0 | ∅ = 0/7 | **technical failure** ~3.5 min in, no workers — drags the pool |

## Summary results

- **Delegation 8/21 = 38.1%** pooled (vs the 16.7% naive baseline, ~2.3×).
  Runs 1+2 alone = **8/14 ≈ 57%**; run 3's early failure is the pool's drag,
  not a menu effect.
- **P-C3-a (window-fit → route): CONFIRMED, as a hard count.** The decision
  journal shows **80% route/inline** (8 of 10 gated verdicts: route 7, inline
  1, spawn 0), the `decisions` verb turning MT-C3-01's soft "SUPPORTED" into a
  number.
- **P-C3-b (schema cuts rework): SUPPORTED — the mechanism engaged.** The boss
  set `output_schema` on the task-9 delegation in 2 of 3 runs (9 and 14
  transcript mentions), where MT-C3-01 had **zero**. Not fully CONFIRMED only
  because no worker *completed* under a schema (workers failed/timed out), so
  the validate-and-retry gate had nothing to grade — a completed schema worker
  closes it.
- **P-C3-c (no wall-budget overrun): CONFIRMED.** Zero budget kills.
- **P-C3-d (Silo escalates): CONFIRMED.** The new Silo task 10 drew **two
  `escalate` gate verdicts** (reason: "cross-chunk dependence dominates (Silo
  task)"), not a fan-out — measured directly from the decision journal.
  MT-C3-01 had zero escalations; the missing menu item was the whole gap.

## Takeaways

The PP-004 fixes landed: both previously-inconclusive predictions moved
(P-C3-b to supported, P-C3-d to confirmed), and P-C3-a is now a number. Open
follow-ups: run 3's early failure (re-run for a clean N=3); a completed
schema-worker to turn P-C3-b CONFIRMED (the worker turn cap / staging linker
still bite). Canonical verdicts: MT-C3-03 "Recorded runs". Raw evidence (bus
facts + decision journals + gzipped transcripts) sits per-run beside this file.
