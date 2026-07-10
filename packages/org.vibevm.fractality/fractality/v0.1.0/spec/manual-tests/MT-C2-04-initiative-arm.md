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

- _(filled at execution; owner sign-off with the Ф6 index)_
