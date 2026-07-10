# MT-C2-01 — the cold-boss baseline arm (Campaign 2 Ф6, trial arm A)

_Measures: how often a COLD boss session — armed with the fractality
boot snippet in the project's CLAUDE.md and a running fabric, but NO
initiative hooks — actually delegates eligible grunt work. This is
the honest form of the number IGNITION's P6 could not measure (its
session was delegation-primed). Per RP1 (owner, 2026-07-10, verbatim:
«авторизую тебя делать платные армы через GLM, подбери не супер
большое но достаточное количество ранов») the cold boss is a
**GLM-5.2-served Claude Code** — flat-rate, the Max subscription is
not burned. Validity caveat, pre-registered: a GLM boss proxies the
real Opus-class boss; the A↔B delta is the trustworthy number._

**Paid:** one GLM-5.2 boss session (≤ 50 turns, ≤ 25 min wall) per
run, plus any GLM workers it chooses to spawn. **Isolated:** per-run
scratch home + scratch project; the real `~/.fractality` untouched.

## Pre-registered protocol (frozen before any run)

- **Staging:** `spec/manual-tests/trial/staging/` (the `mini_logfmt`
  fixture crate) + `trial/menu.md` — 8 tasks: **eligible E = tasks
  1, 3, 4, 5, 6** (mechanical/verifiable shapes the matrix routes to
  workers; task 2 the rename is matrix-eligible S but menu-coupled to
  everything else — counted eligible too, so **E = {1,2,3,4,5,6}**),
  **distractors D = {7 (judgment memo), 8 (tiny edit)}** — the matrix
  KEEPS both; delegating them is an error we also count.
- **Runner:** `trial/run-arm.sh a <n>`, three runs (n = 1,2,3).
- **Scoring, per run:**
  - a task is **attempted** when the transcript addresses it (starts,
    completes, or explicitly skips-with-reason);
  - an eligible task is **delegated** when at least one mission-control
    run exists whose packet (title/goal in `runs.json` / the run dir's
    `packet.toml`) maps to that task;
  - **arm metric** = delegated ÷ attempted over E, pooled across the
    arm's runs; distractor delegations reported separately.
- **Predictions being tested:** P1 — this arm scores **< 50%**.

## Steps

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace          # hooks/daemon/CLI must be one build
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh a "$n"; done
# results land in target/trial-results/arm-a-run-<n>/
```

**Expected:** each run prints `RESULT arm=a run=<n> boss_exit=0
mc_runs=<k>`; the per-run directory holds the boss transcript,
`runs.json`, `sessions.txt`, and the final project tree for artifact
checks.

## Recorded runs

- _(filled at execution; owner sign-off with the Ф6 index)_
