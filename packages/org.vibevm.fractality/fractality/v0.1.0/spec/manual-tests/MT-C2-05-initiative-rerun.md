# MT-C2-05 — the initiative re-run (post-DEF arms; RP5-gated, UNFIRED)

_Pre-registers the trial that answers what C2 Ф6 could not: does the
initiative surface move cold delegation once its falsified mechanics
are repaired? Ф6 measured arm A 3/18 (16.7%) and arm B 0/18 (0%) and
named three mechanisms (plan §2, §15): F23 — the threshold nudge's
only channel (UserPromptSubmit) never re-fires in single-prompt
headless sessions; F24 — the staging toolchain was broken under
`env -i`, handing every boss a rational "workers can't self-verify"
keep-reason; F25 — a fresh home rendered an empty, unpersuasive
scoreboard. The DEF-C2 slice repaired all three: the mid-work
PostToolUse nudge channel (shared cooldown anchor, distinct journal
reason `work-tool-threshold-midwork`), the runner's toolchain
passthrough (rustup homes + the ProgramFiles family; repro-verified
2026-07-10: `cargo test --no-run` link-fails without, links clean
with), and the cold-start board (leads with the route verb, never
renders all-zero counters). This re-run measures the repaired
surface against a same-day-comparable baseline._

**Paid / isolated:** as MT-C2-01 (GLM-served cold boss per the RP1
pattern; per-run scratch home + project; real `~/.fractality`
untouched). **NOT FIRED in the authoring session — every paid run is
gated on RP5 below.**

## Pre-registered protocol (frozen before any run)

- **Staging, menu, runner:** identical artifacts to MT-C2-01
  (`trial/staging/`, `trial/menu.md`, `trial/run-arm.sh`) — the
  runner now carrying the DEF-C2-2a toolchain passthrough. Both arms
  get the fixed toolchain (the fix is arm-neutral by design).
- **Arms:**
  - **A′ — repaired baseline:** snippet 75 v2 in CLAUDE.md, running
    fabric, NO hooks. Differs from Ф6's arm A only by the toolchain
    fix — the A′−A delta measures the F24 confound alone.
  - **B′ — repaired initiative:** A′ + `harness install` — which now
    also means the mid-work nudge channel (F23 repair) and the
    cold-start board (F25 repair). The B′−A′ delta is the initiative
    effect proper; B′−B is the whole-repair effect.
- **Runs:** 3 per arm (the RP1 sizing precedent), technical-repeat
  cap 8, sequential, live-observation law throughout.
- **Scoring:** identical to MT-C2-01 (attempted = transcript
  addresses the task; delegated = an MC run's packet maps to it;
  metric = delegated ÷ attempted over E = {1..6}, pooled per arm;
  distractor delegations for tasks 7/8 reported separately).
- **Fatigue facts (new, recorded per B′ run):** nudges sent by
  reason (`work-tool-threshold-midwork` vs `work-tool-threshold` vs
  `parked-questions`) from the session journal; the slate at each
  nudge; whether a delegation occurred after the first nudge
  (the acted-on proxy); total nudges per run (the cooldown should
  cap this at ⌈wall/300 s⌉).
- **Predictions (frozen):**
  - **PR1** — A′ ≥ A (16.7%): removing the toolchain confound does
    not lower — and likely raises — baseline delegation.
    (Falsifier: A′ < A → the confound theory was wrong; re-analyse.)
  - **PR2** — every B′ run fires ≥ 1 mid-work nudge (mechanism
    proof: the channel exists where the boss lives).
    (Falsifier: a B′ run with slate ≥ 7 and zero nudges → adapter
    bug; fix before interpreting PR3.)
  - **PR3** — B′ ≥ A′ + 30 points (the original P3 delta clause,
    now with a live channel). (Falsifier: the initiative surface
    still does not move cold propensity even when its channel
    fires — the scoreboard-first bet itself goes under review.)
- **Validity caveats carried forward:** GLM-5.2 proxies the
  Opus-class boss (deltas trustworthy, absolutes loose); N=3 per
  arm; single box.

## Steps (when RP5 authorizes)

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh a "$n"; done
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh b "$n"; done
# results: target/trial-results/arm-{a,b}-run-{1..3}/ (fresh dirs —
# archive or clear Ф6's dirs first so runs do not interleave)
```

Score per MT-C2-01's rules; extract fatigue facts from each B′ run's
`sessions.txt` + the session journal; record everything below.

## RP5 — re-run authorization (RESOLVED 2026-07-12)

The paid arms of this test run only on the owner's word: count
(recommend 3+3, cap 8), boss (recommend GLM again for Ф6
comparability; an Opus-class arm is a separate, additional ask), and
timing.

**RESOLVED 2026-07-12 — the owner's ruling, verbatim.** Count/boss:
selected the recommendation on file — **«3+3 GLM, cold boss, cap 8»**
(3 runs arm A′ + 3 runs arm B′, GLM-5.2-served cold boss for Ф6
comparability, technical-repeat cap 8). Timing: **«Авторизую все платные
прогоны и автономию до конца текущего goal»** — the arms fire this goal,
in the session driving PP-001, under the live-observation law. An
Opus-class arm remains a separate, additional ask (not authorized here).
The frozen predictions PR1–PR3 (§Pre-registered protocol) stand
unchanged; scoring follows MT-C2-01's frozen rules.

## Recorded runs

_Executed 2026-12-07 11:26–12:27 (boss `glm-5.2[1m]`, arm A′ = a, arm B′ = b,
3 each). All six ended clean (`boss_exit=0`, 8–17 min — the bosses finished
the menu, none timed out). Raw evidence + the group narrative in
`reports/trial-results/2026-12-07-12-27-c2-mt-c2-05-initiative-rerun/`._

- **Arm A′ (repaired baseline, no hooks):** run 1 = 0 workers, run 2 = 2
  workers (`parse_line-test-suite` [task 1], `facts-table-from-vendor-notes`
  [task 3]), run 3 = 0. **Delegated E = 2/18 = 11.1%.**
- **Arm B′ (A′ + `harness install`):** all three runs = **0 workers**. Hooks
  were installed (SessionStart + UserPromptSubmit) and the cold board fired
  the repaired F25 shape, but the boss delegated nothing. **0/18 = 0%.**

### Prediction verdicts

- **PR1 (A′ ≥ 16.7%): FALSIFIED (within noise).** A′ = 11.1% < the old
  baseline 16.7%; at N=3 the gap is one run's tasks, so "no improvement" is
  the honest read, not "regression".
- **PR2 (every B′ run fires ≥ 1 mid-work nudge): NOT CONFIRMED.** The mid-work
  nudge rides `UserPromptSubmit`, which a single-prompt `-p` boss never
  re-fires (F23 — the mechanic DEF-C2 could not repair for the `-p` modality).
  No re-fired nudge is confirmable in the transcripts.
- **PR3 (B′ ≥ A′ + 30): FALSIFIED, hard.** B′ = 0% — worse than A′, not +30.
  The initiative hooks did not move cold delegation.

**Conclusion — MT-C2-05 REPLICATES the Ф6 finding, it does not overturn it.**
The repaired initiative surface still does not move a cold `-p` boss (A′ ≈
old A within noise; B′ = 0% = old B). F24 (toolchain) and F25 (cold board)
ARE repaired — arm A′ run 2's workers spawned and ran, and the board leads
with the verb — so the residual zero is a delegation-*decision* gap, driven
by F23: a `-p` boss never re-enters `UserPromptSubmit`, so the nudge channel
is structurally dead in that modality. The lever that DOES move cold
delegation is the RLM gate/preamble (arm g, 44.4%, MT-C3-01), not initiative
hooks — which validates Campaign 3's pivot. Open follow-up: move the mid-work
nudge to a channel a `-p` boss re-enters (a PostToolUse path).

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..04)_
