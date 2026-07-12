# C2 Ф6 — the cold-boss initiative trial (MT-C2-01 / MT-C2-04)

_Fired 2026-10-07 (Campaign 2 Ф6). Six paid GLM cold-boss runs over the
`mini_logfmt` menu: arm A (snippet-in-CLAUDE.md baseline) ×3, and arm B
(A + `fractality harness install` — the initiative hooks + statusline) ×3.
Pre-registration: MT-C2-01 (cold baseline) + MT-C2-04 (initiative arm)._

## What this measured

Whether the initiative surface (mid-work nudges + the cold-start scoreboard,
installed by `harness install`) moves a cold GLM boss to delegate more of the
menu's eligible work, versus a bare snippet-only boss.

## Summary results

- **Arm A (baseline): 3/18 ≈ 16.7% delegation** — the historical naive
  comparator every later trial cites.
- **Arm B (+hooks): 0/18 = 0% delegation** — FALSIFIED the prediction that
  the hooks would raise delegation.
- The falsifier analysis named three mechanics: **F23** (the threshold nudge's
  only channel, UserPromptSubmit, never re-fires in a single-prompt headless
  boss), **F24** (the staging toolchain broke under `env -i`, handing every
  boss a rational "workers can't self-verify" keep-reason), **F25** (a fresh
  home rendered an all-zero scoreboard at the only moment the injection speaks).

## Follow-up

The DEF-C2 slice repaired F23/F24/F25; **MT-C2-05 (PP-001)** re-runs the
repaired surface (armed 2026-07-12, RP5 resolved) to answer the question this
trial could not. Canonical verdicts + the F-mechanics: the MT-C2-01/-04/-05
docs and `reports/2026-10-07-16-33-campaign2-f6-trial.md`. The raw evidence
(bus facts + gzipped transcripts) sits per-run beside this file.
