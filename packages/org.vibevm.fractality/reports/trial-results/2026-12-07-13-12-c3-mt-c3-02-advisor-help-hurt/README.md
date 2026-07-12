# C3 Stage C — the advisor help/hurt trial (MT-C3-02)

_Fired 2026-12-07 12:52–13:12 (Stage C, PP-003). Six paid runs, caller =
glm-5-turbo (the weaker tier): arm ALONE ×3 (no advisor) and arm ADVISED ×3
(preamble invites `fractality advise` to a glm-5.2 rung before committing).
Pre-registration: MT-C3-02. Scored by `score-advise.py` (drops each hidden
acceptance into the caller's proj-final and `cargo test`s it)._

## What this measured

The V4 advisor claim (RD-10): does consulting a stronger advisor improve a
weaker caller's result quality on genuinely-uncertain tasks? Three tasks over
mini_logfmt, each with a subtle correct answer (first-seen-order dedup, a
record count where a trailing newline is not a record, an empty-value `k=`
round trip), each with a hidden acceptance test.

## Summary results — the mechanism did not fire

- **Pass rate: ALONE 6/9 = 66.7%, ADVISED 6/9 = 66.7% — delta +0.0.**
- **PR-adv-1 (advice helps): NOT TESTED (null mechanism).** Advised = alone,
  but NOT because advice failed to help — because **the advised caller never
  consulted.** Zero `fractality advise` calls across all three advised runs
  (0 advice runs in `runs.json`, 0 "fractality advise" mentions in the
  transcripts). There was no advice effect to measure.
- **PR-adv-2 (every advised run consults): FALSIFIED — the key finding.** A
  glm-5-turbo caller, even with a preamble explicitly inviting it to consult a
  stronger advisor on the subtle case, **did not consult**. A preamble
  instruction is not enough to make a weak caller reach for the advisor.
- **PR-adv-3 (no-hurt floor): CONFIRMED (trivially).** Advised failures (3) =
  alone failures (3) — advice did not degrade anything, because it did not
  happen.

## The task-1 caveat (a trial-design artifact, not a quality signal)

Task 1 (first-seen dedup) FAILED in ALL six runs — but on `error[E0432]:
unresolved import mini_logfmt::dedup_keys`, a **name mismatch**, not a logic
error. The menu task said "add a public function… wire it into lib.rs"
without pinning the exact name, while the hidden test imports `dedup_keys`.
The callers named their function something else, so the test never linked.
Tasks 2 and 3 (whose expected surface the callers happened to match) passed
universally. A future run must pin the exact function name in the menu task,
or make the hidden test name-agnostic — the 66.7% is muddied by this, not a
clean quality read.

## What this teaches (the real value)

The advisor **machinery** is built and unit-tested (the `advise` verb, the
RD-10 caller-class bar, the ladder). This trial shows the **consult behaviour
is the gap**: a weak caller will not consult from a preamble alone. That is
exactly what the deferred **uncertainty trigger** (C-3) is for — the caller
must be driven to consult by a measured signal (self-consistency spread,
verbalized low confidence), and/or the consult must be more automatic than a
prose invitation. It also reaffirms the 2-tier limit (this measured one point,
glm-5.2 advising glm-5-turbo; the full RD-10 inversion needs a third tier).

Canonical verdicts: MT-C3-02 "Recorded runs". Raw evidence sits per-run
beside this file.
