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

_Executed 2026-07-10 (session resuming the paused Ф6; runner + build
as specified above; boss `glm-5.2[1m]`, workers `small` where
spawned). Per-run facts from `target/trial-results/arm-a-run-<n>/`
(run-info.txt, runs.json, boss-transcript.jsonl, proj-final/)._

- **Run 1** — `boss_exit=1` (`error_max_turns`, 51 turns),
  wall 1281 s, mc_runs=1. Attempted: all 8 (E: 6/6). **Delegated: 1**
  — Task 3 → worker `facts-md` (small, completed; the boss reviewed
  its 1/2 acceptance state but ran out of turns before merging
  FACTS.md). Distractors delegated: 0. Boss triaged via the matrix
  explicitly; kept 1/4/5/6 citing cargo-test verification coupling +
  MSVC-linker friction it had just hand-fixed. **Run metric: 1/6.**
- **Run 2** — `boss_exit=124` (wall timeout), 1500 s, mc_runs=2.
  Attempted: all 8 (E: 6/6). **Delegated: 2** — Task 1 → worker
  `parse_line-test-suite`, Task 3 → worker `facts-table-extract`
  (both small, both completed; collect/merge not landed before the
  wall). Distractors delegated: 0. Explicit matrix triage in the
  transcript (delegate 1+3; keep 2/4/5/6 with named reasons; 7/8
  never-delegate). **Run metric: 2/6.**
- **Run 3** — `boss_exit=0` (clean finish under both caps),
  mc_runs=0. Attempted: all 8, all completed by the boss itself
  (every artifact present incl. FACTS.md; 12+3 tests green).
  **Delegated: 0.** Distractors delegated: 0. **Run metric: 0/6.**

**Arm A pooled metric: (1+2+0)/(6+6+6) = 3/18 ≈ 16.7%.**
Distractor delegations: 0/6 run-opportunities — the matrix's KEEP
verdicts on 7/8 were respected in all runs.

**P1 verdict: CONFIRMED** — 16.7% < 50%. The cold-boss delegation
gap is real and measured (variance across runs: 17% / 33% / 0%).

**Validity notes (recorded, not excuses):** (a) the scratch env
(`env -i`) breaks rustc's MSVC toolchain auto-detection; every boss
hand-fixed the linker and two of three cited "workers can't
self-verify via cargo test here" as a keep reason — a staging defect
that depresses delegation in BOTH arms equally (the A↔B delta stays
interpretable); (b) GLM-5.2 proxies the Opus-class boss (RP1 caveat,
pre-registered); (c) N=3 per arm by owner ruling.

- _(owner sign-off with the Ф6 index: pending)_
