# tcg-bench — the CONTROL-arm baseline (pre-oracle)

_Recorded 2026-07-07. Twelve tasks, one throwaway ts-demo copy each,
mechanical verification (tsc `--pretty false`, `node --test` TAP over an
explicit file list, the real `conform-typescript check` against the
demo's frozen ratchet baseline, per-task completion checks). Arm:
**control — no oracle tools available**. The with-tools arm re-runs the
same tasks after Phase 5 and is compared row-by-row against THIS file._

## Subject

- Agent: opencode **1.17.14**, headless `opencode run --auto`,
  timeout 300 s/task.
- Model: **`openrouter/z-ai/glm-5-turbo`** — the owner's fallback
  directive engaged: the primary (`openrouter/openai/gpt-oss-20b:free`)
  degraded mid-campaign into truncated half-runs (streams cut off
  mid-step at exit 0, 11-read-no-write sessions) and produced no usable
  arm. The owner named "GLM-5-Turbo (Z.AI Coding Plan)"; no Z.AI
  provider credential exists on this box, so the SAME model rides
  through the configured OpenRouter (`z-ai/glm-5-turbo`). Both arms
  must use this model.
- Raw rows: `control-2026-07-07-0634.jsonl` (committed alongside).

## Result: 10 PASS / 2 FAIL

| task | verdict | wall s | tsc errs | halluc | tests | conform new |
|---|---|---|---|---|---|---|
| 01-farewell-variant | PASS | 52 | 0 | 0 | 6p/0f | 0 |
| 02-greet-many | PASS | 54 | 0 | 0 | 9p/0f | 0 |
| 03-new-cell-announce | PASS | 48 | 0 | 0 | 8p/0f | 0 |
| 04-reserved-name | **FAIL** | 54 | 0 | 0 | 9p/0f | **1** |
| 05-truncate-core | PASS | 79 | 0 | 0 | 9p/0f | 0 |
| 06-greet-warmly | PASS | 41 | 0 | 0 | 7p/0f | 0 |
| 07-digits-only | **FAIL** | 33 | 0 | 0 | 9p/0f | **1** |
| 08-farewell-all | PASS | 101 | 0 | 0 | 7p/0f | 0 |
| 09-try-greet | PASS | 32 | 0 | 0 | 9p/0f | 0 |
| 10-farewell-count | PASS | 46 | 0 | 0 | 6p/0f | 0 |
| 11-polite-farewell | PASS | 49 | 0 | 0 | 6p/0f | 0 |
| 12-greet-raw-string | PASS | 90 | 0 | 0 | 9p/0f | 0 |

Aggregates: completion 12/12 (`done=1` everywhere), mean wall 56.6 s,
tsc errors 0/12 tasks, hallucination-class codes 0, test failures 0,
**conform regressions 2/12**.

## Reading

- **The failure mode is DISCIPLINE, not types.** GLM-5-Turbo keeps
  `tsc --noEmit` clean on every task and never hallucinates an
  identifier — but both parseGuestName-extension tasks (04, 07)
  introduced a NEW unsafe-set finding against the frozen baseline, and
  did so REPRODUCIBLY (the same two tasks failed the same way in the
  interrupted earlier run). This is precisely the gap the oracle's
  enrichment targets: `tcg_validate` returns those findings (flagged
  non-baselined, with the guide-citing advice) BEFORE the edit lands.
- Task 12 (the brand-discipline trap) passed clean — the model used
  `parseGuestName` rather than casting. The trap stays in the battery
  as a regression canary.
- **Primary-model note:** gpt-oss-20b:free produced two unusable arms
  (first: five do-nothing "PASS" rows that motivated the completion
  checks; second: truncated sessions with zero writes). Free-tier
  routing quality is a real threat to run validity — the RUNBOOK now
  pins the fallback and requires arms to share one model.
- **Harness lessons burned in this session:** per-task completion
  checks (a do-nothing run must not PASS on a pre-green tree); ANSI-free
  verifier output (`--pretty false`, TAP) so counters count; the
  conform binary is copied to a battery-local toolcache because a slot
  refresh mid-run yanked it (three conform=127 rows in the discarded
  run).

## Predictions check (plan §4.3, first half)

The claim under test is DIRECTIONAL: with-tools must lower the
discipline-regression count (2) and/or wall time on the failing tasks.
n=12 supports direction only, not magnitude — as posted.
