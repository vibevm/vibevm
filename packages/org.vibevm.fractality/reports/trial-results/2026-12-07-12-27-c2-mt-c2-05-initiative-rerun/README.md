# C2 — the MT-C2-05 initiative re-run (PP-001)

_Fired 2026-12-07 11:26–12:27 (Campaign 2 debt, PP-001). Six paid GLM-5.2
cold-boss runs on the DEF-C2-repaired surface: arm A′ (repaired baseline,
snippet + fixed toolchain, NO hooks) ×3 and arm B′ (A′ + `harness install` —
the initiative hooks + cold board) ×3. Pre-registration: MT-C2-05 (RP5
resolved 2026-07-12: 3+3 GLM cold boss, cap 8)._

## What this measured

Whether the DEF-C2 slice's repairs of the three Ф6 mechanics (F23 the
nudge channel, F24 the staging toolchain, F25 the all-zero cold board) move
a cold `-p` boss to delegate — the question the original Ф6 could not answer
because those defects confounded it. PR1: A′ ≥ old A (16.7%). PR2: every B′
run fires ≥ 1 mid-work nudge. PR3: B′ ≥ A′ + 30 points.

## Runs

All six ended clean (`boss_exit=0`, 8–17 min each — the bosses finished the
menu, they did not time out). Workers spawned:

| arm | run 1 | run 2 | run 3 | delegated E |
|---|---|---|---|---|
| A′ (a) | 0 | 2 ({1,3}) | 0 | 2/18 = 11.1% |
| B′ (b) | 0 | 0 | 0 | 0/18 = 0% |

Arm B′'s hooks WERE installed (harness-status: SessionStart + UserPromptSubmit)
and its cold board fired the repaired F25 shape ("fabric ready — no delegated
runs… first delegation: `fractality route`…"), yet the boss delegated nothing.

## Summary results — the repairs did NOT move cold delegation

- **PR1 (A′ ≥ 16.7%): FALSIFIED (within noise).** A′ = 11.1% < the old
  baseline 16.7% — the repaired baseline delegated *less*, though at N=3 the
  gap is a single run's worth of tasks (both are "≈1 of 3 runs delegated").
- **PR2 (every B′ run fires a mid-work nudge): NOT CONFIRMED.** The hooks are
  installed, but the mid-work nudge rides `UserPromptSubmit`, which a
  single-prompt `-p` boss never re-fires — F23, the very mechanic DEF-C2 could
  not fully repair for the `-p` modality. The transcripts show engagement but
  no re-fired nudge is confirmable.
- **PR3 (B′ ≥ A′ + 30): FALSIFIED, hard.** B′ = 0% delegation — *worse* than
  A′, not +30. The initiative hooks did not help; they coincided with zero
  delegation, exactly the original Ф6 arm-B result (0%).

**Headline:** even repaired, the initiative-hooks surface does not move a
cold `-p` boss to delegate (A′ ≈ old A within noise; B′ = 0% = old B). This
**replicates** the C2 finding rather than overturning it, and it stands in
sharp contrast to the RLM gated arm (arm g, **44.4%**, MT-C3-01): the RLM
gate/preamble is the lever for cold delegation; initiative hooks are not.
The result validates Campaign 3's pivot to the RLM approach.

## Takeaways

- The F23 `-p` re-fire limit is the load-bearing constraint: a single-prompt
  headless boss never re-enters `UserPromptSubmit`, so the mid-work nudge
  channel is structurally dead in the `-p` modality. Moving the nudge to a
  channel a `-p` boss DOES re-enter (e.g. a PostToolUse hook, which the DEF
  slice added for interactive but not the `-p` path) is the open lever.
- F24 (toolchain) IS repaired — arm A′ run 2's two workers spawned and ran
  (the linker no longer blocks); F25 IS repaired — the cold board leads with
  the verb. The remaining zero is a delegation-*decision* gap, not a
  mechanics gap.

Canonical verdicts: MT-C2-05 "Recorded runs". Raw evidence (bus facts +
gzipped transcripts) sits per-run beside this file.
