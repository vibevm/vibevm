# MT-C2-04 — the initiative arm (Campaign 2 Ф6, trial arm B)

_Measures: the same cold GLM-served boss over the same staging and
menu as MT-C2-01, with exactly one difference — `fractality harness
install claude-code` ran against the project, so the session gets the
live scoreboard at SessionStart, threshold nudges at prompts, and
parked-question stop alerts. The A↔B delta on identical arms is the
campaign's headline number (P3)._

**Paid / isolated:** as MT-C2-01. **Pre-registered protocol:**
identical to MT-C2-01 in staging, menu, run count (3), scoring, and
metric — the arm differs only in the harness install (see
`run-arm.sh`, arm `b`).

- **Predictions being tested:** P3 — this arm scores **≥ 80%** AND
  **≥ arm A + 30 points**. Additional facts recorded: nudges sent vs
  acted on (bus counters), work-tool slates, and — headless caveat
  F20 — the statusline does not render in `-p`, so arm B exercises
  SessionStart + UserPromptSubmit + Stop channels only. One
  SessionStart injection rides every run by construction; the arm
  therefore measures "greeted with the scoreboard + nudged on
  grinding", not the full interactive ambient experience.

## Steps

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh b "$n"; done
# results land in target/trial-results/arm-b-run-<n>/
```

**Expected:** as MT-C2-01, plus `harness-status.txt` reporting all
five events + statusLine `installed`, and the session counters in
`sessions.txt` showing non-zero activity (work-tools and/or
delegations and/or nudges).

## Recorded runs

_Executed 2026-07-10, same session/runner/build as MT-C2-01. Every
run's `harness-status.txt` reported all five events + statusLine
`installed`; every run's `sessions.txt` carries a session record —
the adapter demonstrably worked end to end in `-p`._

- **Run 1** — `boss_exit=0` (subtype `success`, 81 turns), mc_runs=0.
  Session `01KX5ZSA…` ended, **deleg=0 slate=34** (PostToolUse
  counted 34 work-tools). SessionStart scoreboard injection reached
  the model (board text present in the transcript). Attempted: all 8
  (E: 6/6), all completed by the boss (18 parse tests; every
  artifact). **Delegated: 0.** The boss explicitly argued delegation
  DOWN: "small, judgment-heavy edits where the delegation+review
  overhead exceeds the work" — citing the matrix's own never-delegate
  vocabulary. **Run metric: 0/6.**
- **Run 2** — `boss_exit=124` (wall), mc_runs=0. Session `01KX60V8…`
  open at collection, **deleg=0 slate=45**. Attempted: all 8
  (E: 6/6). **Delegated: 0.** The boss consulted matrix + scoreboard
  proactively, then discovered the sandbox linker defect and
  reasoned: workers cannot self-verify (`cargo test` broken without
  the hand-built vcvars wrapper) → the delegation premise fails →
  keep everything. **Run metric: 0/6.**
- **Run 3** — `boss_exit=1` (`error_max_turns`, 51 turns), mc_runs=0.
  Session `01KX629S…` ended, **deleg=0 slate=44**. Attempted: all 8
  (E: 6/6; max-turns hit during final review). **Delegated: 0.** The
  boss ran `route` ("it returns 'delegate' for small mechanical
  work") and overrode it: coupling via the Task-2 rename + per-task
  packet/review overhead. **Run metric: 0/6.**

**Arm B pooled metric: 0/18 = 0%.** Distractor delegations: 0.

**P3 verdict: FALSIFIED** on both clauses — 0% < 80% and
0% < A (16.7%) + 30. The initiative surface as exercised in `-p`
did not raise cold delegation; it did not lower attempted coverage
either (6/6 across all runs, work quality high).

**Channel analysis (P3's falsifier obligation):**
- **SessionStart scoreboard: delivered, ineffective.** Every run
  greeted the boss with an EMPTY board ("0 runs · 0 completed
  all-time" — fresh scratch home). A zero board carries no social
  proof; it may even read as "nobody uses this here".
- **UserPromptSubmit nudge: structurally silent in `-p`.** The only
  user prompt is the menu itself, arriving at slate=0 — below every
  threshold. The threshold crossings (slates 34/44/45 ≫ 7) all
  happened AFTER the last prompt; no second prompt ever came, so the
  nudge teeth never engaged. MT pre-registered the channel as
  present; the trial proves it never fires in single-prompt
  headless sessions — a design fact, not a bug in the hook.
- **Stop question-push: correctly silent** (no parked questions —
  nothing was delegated).
- **Confound shared with arm A:** the broken MSVC auto-detect gave
  a rational keep reason; it suppresses B the same way it
  suppresses A.

- **Owner sign-off: RECORDED 2026-07-10** (verbatim: «подписываю
  MT-C2-01…04») — the recorded runs above stand as the accepted
  result of this test.
