# PP-003 — Option C: the advisor slice (V4)

_Filed: 2026-07-11 · Status: **POSTPONED** (owner cut it from
Campaign 3 Stage B — «отдельная задача, запланируй») · Origin:
RP-C3-1 ruling (Stage B plan §1, §8); scope options §3; decision
D-C3-7._

## The task

Build the **advisor channel** (VISION §V4) — the fabric's fourth
plane, on top of Stage B's descent + ascent. An advisor is a
worker-shaped run that returns *advice*, not owned work: a weaker
caller asks a stronger profile a bounded question and keeps
ownership of its task.

Concretely, D-C3-7 (Stage B plan §4):

- an `advice` packet type — worker-shaped run, **no ownership
  transfer** (the caller stays responsible for the task);
- a capability bar on the CALLER: `advisor_enabled ⇐ caller_class
  ≥ medium` — routing by capability, not by task difficulty
  (RD-10: weak callers are made WORSE by advice; the SWE-1.5→1.6
  threshold, the minRLM-nano inversion);
- an **uncertainty trigger** — the caller asks only on intrinsic
  uncertainty signals (self-consistency spread, trace length,
  verbalized confidence — RD-10 / SRLM);
- context hand-off ≈ a fork of the caller's context (RD-11's
  clean-context posture applies);
- an accounting line: the advice call bills the **caller's**
  budget (revisit trigger: first field data — RD-10).

## Why postponed

The owner ruled RP-C3-1 as **Option B** (descent + ascent) and
cut Option C explicitly: «Вариант C с адвайзором - отдельная
задача, запланируй.» V4 is a real plane with its own economics
(inverted vs descent), its own capability bar, and its own trial
questions — folding it into Stage B would widen the campaign past
the descent core the owner wants first. D-C3-7 stays drafted in
the Stage B plan but out of its build scope.

## Unblock

The owner commissions **Stage C** (or folds the advisor into a
later mandate). Design questions to settle at commissioning:

- the caller-class threshold's exact boundary and how a caller
  measures its own class at call time;
- which uncertainty signal(s) fire the trigger, and their
  thresholds (measured, not guessed);
- whose budget line pays when advice recurses (caller's, per the
  draft — confirm against first field data);
- the trial that proves advice HELPS a medium caller and does not
  hurt a weak one (the RD-10 inversion is the falsifier).

## First steps when unblocked

1. Re-read D-C3-7, RD-10, RD-11 (RLM-SYNTHESIS §3), FD-6/FD-8
   (FUGU-SYNTHESIS §3), VISION §V4 — the advisor-ladder pillar.
2. Confirm Stage B shipped the plumbing D-C3-7 reuses (worker-run
   shape, `context_from` access lists, the acceptance/clean-context
   posture) — the advisor is "a worker run with an `advice` packet
   type", so most of the seam already exists after Stage B.
3. Draft the Stage C plan (or slice) with its own MT pre-registration
   for the help/hurt trial.

## Canonical pointers

- Decision + scope: Stage B plan §3 (Option C), §4 (D-C3-7), §1
  (RP-C3-1 ruling) —
  [`FRACTALITY-RLM-PLAN-v0.1.md`](../../fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md)
- Field evidence: RD-10, RD-11 —
  [`RLM-SYNTHESIS.md`](../../fractality/v0.1.0/spec/refs/notes/RLM-SYNTHESIS.md);
  FD-6, FD-8 —
  [`FUGU-SYNTHESIS.md`](../../fractality/v0.1.0/spec/refs/notes/FUGU-SYNTHESIS.md)
- Vision pillar: V4 advisor ladder —
  [`VISION-RECURSIVE-FABRIC.md`](../../fractality/v0.1.0/spec/VISION-RECURSIVE-FABRIC.md)
