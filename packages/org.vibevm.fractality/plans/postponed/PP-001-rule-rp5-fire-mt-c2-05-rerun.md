# PP-001 — Rule RP5: authorize and fire the MT-C2-05 re-run

_Filed: 2026-07-10 · Status: **POSTPONED** (RP5 OPEN — awaiting the
owner's ruling) · Origin: Campaign 2 close — plan §15 (DEF-C2-4),
[MT-C2-05](../../fractality/v0.1.0/spec/manual-tests/MT-C2-05-initiative-rerun.md),
WAL §Next._

## The task

Fire the pre-registered re-run of the initiative trial on the
repaired surface, score it by the frozen rules, and rule its
predictions.

Campaign 2's Ф6 trial measured cold-boss delegation at arm A
(snippet only) **3/18 ≈ 16.7%** and arm B (+ initiative hooks)
**0/18 = 0%**, falsifying P3 and naming three mechanics: **F23** —
the threshold nudge's only channel (UserPromptSubmit) never re-fires
in single-prompt headless sessions; **F24** — the staging toolchain
broke under `env -i`, handing every boss a rational "workers can't
self-verify" keep-reason; **F25** — a fresh home rendered an
all-zero scoreboard at the only moment the injection speaks. The
DEF-C2 slice repaired all three (mid-work PostToolUse nudge with
shared cooldown; runner toolchain passthrough, repro-verified;
cold-start board leading with the route verb). MT-C2-05 asks the
question Ф6 could not answer: does the repaired initiative surface
move cold delegation?

- **Arms:** **A′** — repaired baseline (snippet 75 v2, fixed
  toolchain, NO hooks; the A′−A delta isolates the F24 confound).
  **B′** — A′ + `harness install` (now including the mid-work nudge
  channel and the cold board; B′−A′ is the initiative effect proper,
  B′−B the whole-repair effect).
- **Runs:** 3 per arm recommended (RP1 sizing precedent),
  technical-repeat cap 8, sequential, live-observation law
  throughout.
- **Boss:** GLM-5.2-served cold boss for Ф6 comparability; an
  Opus-class arm is a separate, additional ask.
- **Frozen predictions:** **PR1** — A′ ≥ A (16.7%); **PR2** — every
  B′ run fires ≥ 1 mid-work nudge; **PR3** — B′ ≥ A′ + 30 points.
- **Fatigue facts per B′ run (new):** nudges by journal reason
  (`work-tool-threshold-midwork` vs `work-tool-threshold` vs
  `parked-questions`), the slate at each nudge, the
  delegation-after-first-nudge proxy, total nudges (the cooldown
  should cap at ⌈wall/300 s⌉).

## Why postponed

The runs are paid (Ф6 precedent: ~2 h 10 m of GLM wall for six
runs), and MT-C2-05 hard-gates every paid arm on **RP5**: the
owner's explicit word on count, boss, and timing, recorded verbatim
in its §RP5. The protocol was frozen 2026-07-10; nothing has fired.

## Unblock

The owner rules RP5. Recommendation on file: 3+3 GLM (Ф6-comparable),
cap 8.

## First steps when unblocked

1. Record the ruling verbatim in MT-C2-05 §RP5 (OPEN → RESOLVED).
2. Archive or clear Ф6's `target/trial-results/arm-{a,b}-run-{1..3}/`
   (all six dirs still present as of filing) so runs do not
   interleave.
3. `cd fractality/v0.1.0 && cargo build --workspace` (stop MC
   daemons first — F15).
4. Fire `bash spec/manual-tests/trial/run-arm.sh {a|b} {1..3}`
   sequentially under the live-observation law.
5. Score per MT-C2-01's frozen rules; extract fatigue facts from
   each B′ run's `sessions.txt` + the session journal; fill
   MT-C2-05 §Recorded runs; rule PR1–PR3.

One session's work with the fixed toolchain.

## Canonical pointers

- Frozen protocol:
  [`fractality/v0.1.0/spec/manual-tests/MT-C2-05-initiative-rerun.md`](../../fractality/v0.1.0/spec/manual-tests/MT-C2-05-initiative-rerun.md)
- Scoring rules: MT-C2-01 (same directory); mechanics F23/F24/F25 and
  verdicts: plan §2 + §15 —
  [`FRACTALITY-INITIATIVE-PLAN-v0.1.md`](../../fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
- DEF-C2 slice record:
  [`reports/2026-10-07-21-30-defc2slice-report.md`](../../reports/2026-10-07-21-30-defc2slice-report.md)
