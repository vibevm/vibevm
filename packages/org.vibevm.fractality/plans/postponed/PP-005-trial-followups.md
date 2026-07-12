# PP-005 — the three trial follow-ups (validated re-runs)

_Filed: 2026-07-12 · Status: **POSTPONED** (each needs an owner mandate +
paid runs) · Origin: the three paid trials fired this session (PP-004 gated
re-run MT-C3-03, PP-001 initiative re-run MT-C2-05, MT-C3-02 advisor
help/hurt). Each trial fired cleanly and produced a real finding; each left
exactly one follow-up that a *next* run would resolve. This collects all
three so a cold session can pick any of them up alone._

Common shape: all three are re-runs of an already-frozen protocol with a
small, specific change. None blocks anything — the five-task goal is done and
the machinery each exercises is built and floor-green. Each is a paid GLM run
under the live-observation law, and its evidence MUST be preserved
(`save-results.sh <group>` → committed `reports/trial-results/`, the standing
rule in workspace `CLAUDE.md`).

---

## FU-1 — the advisor forced-consult re-run (MT-C3-02)

**The task.** Re-run the advisor help/hurt trial with the consult *forced*,
so the RD-10 help/hurt effect can actually be measured.

**Why postponed / what the first run showed.** MT-C3-02 fired (alone×3 +
advised×3, caller glm-5-turbo, advisor glm-5.2) and returned a **null
mechanism**: the advised caller NEVER invoked `fractality advise` (0 advice
calls, 0 transcript mentions), so ADVISED = ALONE = 66.7% with no advice
effect to measure. PR-adv-2 falsified — a prose preamble invitation is not
enough to make a weak caller consult. The effect (PR-adv-1) is therefore
untested, not disproven.

**What unblocks it.** The owner commissions a re-run with a stronger consult
DELIVERY (ADVISOR-PLAN §6), cheapest first:
1. a **mandatory-step preamble** — "for each subtle case, sample your answer
   3× (self-consistency); if they disagree you MUST `fractality advise`
   before committing";
2. or a **forcing hook** — a PostToolUse-style check that injects the advise
   call / blocks the commit on a detected-uncertain step (makes the trigger
   the fabric's job, not the weak caller's discipline).

**First steps when unblocked.**
1. Fix the trial-design brittleness: **pin task 1's exact function name**
   (`dedup_keys`) in `menu-advise.md`, OR make `advise-assets/task1_order_test.rs`
   name-agnostic — task 1 failed all six runs on `error[E0432]`, a name
   mismatch, not a logic error.
2. Author `preamble-advised.md` v2 with the mandatory-consult step (or add the
   forcing hook); keep `preamble-alone.md` unchanged (paired-arm discipline).
3. For the full RD-10 *inversion* (advice helps medium, does NOT hurt weak),
   add a **third model tier** below glm-5-turbo to the `glm` profile — two
   tiers only serve one point (glm-5.2 advises glm-5-turbo).
4. Re-fire `run-advise.sh alone|advised {1,2,3}`; `save-results.sh
   c3-mt-c3-02-advisor-rerun`; `score-advise.py`; rule PR-adv-1/2/3; the
   measured self-consistency-spread / confidence thresholds go into
   ADVISOR-PLAN §6 (they are the deferred MEASUREMENT there).

**Canonical pointers.** `spec/manual-tests/MT-C3-02-advisor-help-hurt.md`
(§Recorded runs), `spec/plans/FRACTALITY-ADVISOR-PLAN-v0.1.md` §6, the group
`reports/trial-results/2026-12-07-13-12-c3-mt-c3-02-advisor-help-hurt/`, the
harness `trial/run-advise.sh` + `advise-assets/` + `score-advise.py`.

---

## FU-2 — the PP-001 PostToolUse nudge (MT-C2-05)

**The task.** Move the initiative mid-work nudge off `UserPromptSubmit` onto
a channel a single-prompt `-p` boss actually re-enters, then re-run MT-C2-05
to see whether a re-firing nudge moves cold delegation.

**Why postponed / what the first run showed.** MT-C2-05 fired (A′×3 + B′×3)
and **replicated the Ф6 finding**: A′ = 11.1%, B′ (with hooks) = 0% — the
DEF-C2 repairs did not move cold delegation. The residual mechanic is **F23**:
a `-p` headless boss never re-enters `UserPromptSubmit`, so the mid-work nudge
channel is structurally dead in that modality (F24 toolchain and F25 cold
board ARE repaired — arm A′ run 2's workers spawned and ran). So the nudge
never re-fires; the delegation-decision gap is unaddressed.

**What unblocks it.** The owner commissions the channel move. The initiative
DEF-C2 slice already added a mid-work PostToolUse nudge for the *interactive*
path; this extends it to the `-p` path (or picks whatever event a `-p` boss
re-enters mid-session).

**First steps when unblocked.**
1. In `fractality-initiative` / the harness hook wiring, route the mid-work
   threshold nudge through PostToolUse (or an equivalent re-entered event) for
   the `-p` modality; unit-test the shared cooldown still caps at ⌈wall/300s⌉.
2. Re-fire `run-arm.sh {a,b} {1,2,3}`; `save-results.sh c2-mt-c2-05-rerun2`;
   score by the MT-C2-01 rubric; rule PR1–PR3 (esp. PR2: does each B′ run now
   fire ≥ 1 mid-work nudge?).

**Canonical pointers.** `spec/manual-tests/MT-C2-05-initiative-rerun.md`
(§Recorded runs — the F23 conclusion), the group
`reports/trial-results/2026-12-07-12-27-c2-mt-c2-05-initiative-rerun/`,
`spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md` §2/§15 (F23/F24/F25),
`crates/fractality-initiative/src/nudge.rs`, `crates/fractality-cli/src/hook.rs`.

---

## FU-3 — the PP-004 clean-N=3 re-fire (MT-C3-03)

**The task.** Re-fire the one failed gated re-run so P-C3-a/b/d rest on a
clean N=3, and land a completed schema-worker to turn P-C3-b CONFIRMED.

**Why postponed / what the first run showed.** MT-C3-03 fired (arm g2 ×3) and
moved both previously-inconclusive predictions — P-C3-a CONFIRMED as a hard
count (80% route/inline via `decisions`), P-C3-d CONFIRMED (Silo task → 2
escalate verdicts), P-C3-b SUPPORTED (boss set `output_schema`). But **run 3
was a technical failure** (0 workers, exit 1, ~3.5 min), dragging the pool to
38.1% (runs 1+2 alone = 57%), and **no worker completed under a schema**, so
P-C3-b is SUPPORTED, not CONFIRMED (the validate-and-retry gate had no return
to grade).

**What unblocks it.** The owner commissions the re-fire (≤ 5 technical repeats
were reserved in MT-C3-03). Cheap — one re-fire of run 3, plus enough worker
headroom for a schema worker to finish.

**First steps when unblocked.**
1. Re-fire `run-arm.sh g2 3` (and 1–2 more if a clean N=3 is wanted);
   `save-results.sh c3-mt-c3-03-refire`; `score-g2.py`; update MT-C3-03
   §Recorded runs.
2. If the schema worker still fails to complete (staging linker / turn cap),
   raise the worker cap further or simplify task 9 so one worker finishes
   under an `output_schema` — that single completed run turns P-C3-b CONFIRMED.

**Canonical pointers.** `spec/manual-tests/MT-C3-03-gated-rerun.md`
(§Recorded runs), the group
`reports/trial-results/2026-12-07-11-03-c3-mt-c3-03-gated-rerun/`,
`trial/menu-g2-extra.md` (task 9 schema), `trial/score-g2.py`.

---

## Minor, related (not one of the three; noted for completeness)

**PP-002 credibility on the mid-work nudge.** The credibility line
(`worker_credibility` → `CredibilityFact`) is surfaced on the cold board +
`fractality scoreboard`; the mid-work nudge could cite it too, so a boss
mid-session sees "workers self-verify here" at the moment it hesitates to
delegate. Small; wire `render` of the fact into the nudge path. Pointer:
`crates/fractality-core/src/credibility.rs`, `crates/fractality-initiative/src/nudge.rs`.
