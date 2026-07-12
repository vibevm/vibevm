# MT-C3-02 — the advisor help/hurt trial (Stage C, PP-003 / D-C3-7)

_Pre-registered before any run (§10.7 pre-reg-first). Measures the V4
advisor channel's load-bearing claim (RD-10 / VISION §V4): does consulting
a **stronger advisor** improve a **weaker caller's** result quality on
genuinely-uncertain tasks — and, the falsifier, does it ever make it
**worse**? Authorization: the owner pre-authorized all paid trial runs for
this goal (2026-07-12, verbatim «Авторизую все платные прогоны и автономию
до конца текущего goal»). Owner sign-off on the recorded runs is taken
after, as with every prior MT._

## The claim under test (RD-10, VISION §V4)

Advice moves judgment **sideways-up** without moving ownership: a caller
consults a bigger model, reads its judgment, and keeps its own task. The
economics invert relative to descent — a smaller model owns the loop, a
bigger one advises. The **load-bearing constraint** (RD-10) is a caller
**capability bar**: advice helps a capable-enough caller but makes a caller
*below a threshold* WORSE, not better (the minRLM/nano inversion; the
SWE-1.5→1.6 threshold in the research). The advisor bar
(`check_advisor_caller_class`) refuses a sub-`medium` caller for exactly
this reason. This trial asks whether the *help* half is real where the bar
allows it.

## Design — paired arms, one caller tier, N=3

**The tier constraint (recorded honestly).** The fabric's `glm` profile
exposes two capability tiers — `big = glm-5.2[1m]` and `small =
glm-5-turbo`. A full two-point inversion (advice HELPS a medium caller AND
does NOT hurt a weak one) needs three tiers: weak < medium < strong-advisor.
With two, this trial fires the single point the fabric can serve cleanly:

- **Caller** = `glm-5-turbo` (the weaker tier), the top-level agent that
  owns each task.
- **Advisor** = `glm-5.2` (a rung above — the ladder's `advisor_class_for`
  answer for a caller a rung below the top).

So MT-C3-02 tests **does glm-5.2's advice improve glm-5-turbo's output** —
the advisor value proposition proper. The second point (a genuinely *weak*
caller that advice should NOT help — the falsifier's other arm) is
**deferred to a third-tier trial** (a weaker model below glm-5-turbo, e.g.
a glm-4-class slot added to the profile), noted in Decisions.

**Two arms, same caller model, same menu, N=3 each (paired):**

- **Arm ALONE** — a `glm-5-turbo` caller works the uncertain-task menu with
  no advisor: the baseline quality the caller reaches by itself.
- **Arm ADVISED** — the same `glm-5-turbo` caller, its preamble instructing
  it to consult `fractality advise` (routed to the `big`/glm-5.2 rung)
  **before committing** each uncertain task, and factor the judgment in.

The caller is top-level (no parent run), so its advice call clears the bar
as a human-at-the-top consultation (`check_advisor_caller_class` admits a
parentless advice call) — this trial measures the *effect* of advice, not
the bar (the bar is unit-tested in `advisor.rs`).

**Paid:** ≤ 8 caller sessions total (RP1/RP5 precedent cap); 3 ALONE + 3
ADVISED planned, ≤ 2 technical repeats reserved for stalls. Each caller
session ≤ 100 turns, ≤ 25 min wall; the ADVISED arm additionally spends the
glm-5.2 advice calls it issues. **Isolated:** per-run scratch home + scratch
project; the real `~/.fractality` is read (profiles template) but never
written.

## The menu — genuinely-uncertain tasks with ground truth

The gated trial's `menu.md` measures *delegation* (delegable vs keep); this
trial needs tasks where a weaker model **plausibly errs** and stronger
judgment could **correct or mislead**, each with a **checkable** right
answer so quality is measured, not vibed. Authored in
`trial/menu-advise.md` (a fresh `mini_logfmt`-shaped micro-repo under
`trial/staging-advise/`), each task carrying a hidden acceptance the scorer
runs. Task shapes (the design intent; exact text frozen in the menu file):

1. **A correctness trap** — a plausible-looking implementation choice that
   fails a subtle acceptance (e.g. dedup that must preserve first-seen
   order, where the obvious `HashSet` reorders). Advice from a stronger
   model should steer to the order-preserving design.
2. **An off-by-one / boundary judgment** — a parser boundary (empty input,
   trailing separator) the acceptance pins; the naive path trips it.
3. **A design-with-a-right-answer** — two APIs, one composes and one dead-
   ends a later requirement stated in the task; the acceptance exercises the
   later requirement.
4. **A "looks-done" refactor** — a change that passes the happy path but
   breaks an edge the acceptance covers.

Each task's acceptance is a `cargo test` (hidden test file the scorer moves
in) or a deterministic check. Quality per task = acceptance pass/fail.

## Predictions being tested (frozen)

- **PR-adv-1 (help)** — pooled ACCEPTANCE-PASS rate is **higher in ADVISED
  than in ALONE**. The falsifier: ADVISED ≤ ALONE means advice did not help
  glm-5-turbo (it sits at/below the RD-10 threshold for these tasks) — a
  real, recordable result, not a trial failure.
- **PR-adv-2 (mechanism fires)** — every ADVISED run issues **≥ 1 `advise`
  call** (an advice-marked run appears in that run's `ps --json`), so a null
  result is "advice didn't help", never "advice never happened".
- **PR-adv-3 (no-hurt floor)** — ADVISED produces **no more hard acceptance
  failures than ALONE** on the tasks both attempted (advice must not
  actively degrade a task the caller would otherwise have passed).

A prediction with no eligible observation is reported **inconclusive**,
never silently passed.

## Runner

`trial/run-advise.sh alone|advised <n>` — a Stage-C sibling of
`run-arm.sh`: same isolated scratch home + staged repo + z.ai-served agent +
bus-fact collection, but (a) the top-level agent is served by the **small**
model (`glm-5-turbo`), not big; (b) the menu is `menu-advise.md` prefixed
with `preamble-alone.md` or `preamble-advised.md`; (c) it collects each
task's acceptance verdict and (advised) the `advise` runs from `ps --json`.
Build first: `cargo build --workspace`.

## Scoring (per run, then pooled)

`trial/score-advise.py` over `target/trial-results/advise-<arm>-run-<n>/`:
per task, the acceptance pass/fail; per arm, the pooled pass rate; the
ADVISED−ALONE delta (PR-adv-1); the advise-call count per advised run
(PR-adv-2); the failure-count comparison (PR-adv-3). Facts cite their
source file, the score-g.py rule.

## Steps

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace
for n in 1 2 3; do bash spec/manual-tests/trial/run-advise.sh alone   "$n"; done
for n in 1 2 3; do bash spec/manual-tests/trial/run-advise.sh advised "$n"; done
python spec/manual-tests/trial/score-advise.py
```

## Decisions taken at commissioning

1. **One caller tier, not two.** The fabric has two GLM tiers; the clean
   two-point inversion needs three. Rather than fake a middle tier, this
   trial fires the point it can serve — glm-5.2 advising glm-5-turbo — and
   defers the weak-caller falsifier to a third-tier trial. Honest partial
   evidence beats an unsound three-tier design over two models.
2. **Paired arms, same caller model.** ALONE and ADVISED differ only in the
   preamble's consult instruction, so the delta isolates the advice effect,
   not a model difference (the C2 A′/B′ paired-arm discipline).
3. **Ground-truth acceptance, not judgement.** Every uncertain task carries
   a hidden checkable acceptance so "quality" is a pass rate, not a rubric
   the scorer argues — the RD-10 inversion is only falsifiable against a
   real right answer.
4. **The caller is top-level, so the bar is not what's tested.** A
   parentless advice call clears `check_advisor_caller_class` by design;
   this trial measures whether advice HELPS, which the bar assumes but does
   not prove. The bar itself is unit-tested (`advisor.rs`).

## Recorded runs

_Executed 2026-12-07 12:52–13:12 (caller `glm-5-turbo`, alone×3 + advised×3,
all `boss_exit=0`). Scored by `score-advise.py`. Raw evidence + the group
narrative in `reports/trial-results/2026-12-07-13-12-c3-mt-c3-02-advisor-help-hurt/`._

**Pass rate: ALONE 6/9 = 66.7%, ADVISED 6/9 = 66.7% (delta +0.0).** Tasks 2
(count) and 3 (empty-value) passed in all six runs; task 1 (order dedup)
failed in all six — but on `error[E0432]: unresolved import
mini_logfmt::dedup_keys`, a **name mismatch** (the menu did not pin the exact
function name the hidden test imports), not a logic error. Task 1 is therefore
excluded from the quality read as a trial-design artifact.

### Prediction verdicts

- **PR-adv-1 (advice helps): NOT TESTED — null mechanism.** ADVISED = ALONE,
  but because **the advised caller never consulted**, not because advice
  failed to help. There was no advice effect to measure.
- **PR-adv-2 (every advised run consults ≥ 1×): FALSIFIED — the key finding.**
  Zero `fractality advise` calls across all three advised runs (0 advice runs
  in `runs.json`; 0 mentions in the transcripts). A `glm-5-turbo` caller, even
  with a preamble explicitly inviting it to consult a stronger advisor on the
  subtle case, **did not reach for the advisor**. A prose invitation is not
  enough to make a weak caller consult.
- **PR-adv-3 (no-hurt floor): CONFIRMED (trivially).** ADVISED failures (3) =
  ALONE failures (3) — advice degraded nothing, because it did not happen.

**Conclusion.** The advisor MACHINERY is real and unit-tested (the `advise`
verb, the RD-10 caller-class bar, the ladder — all floor-green). This trial
isolates the remaining gap as the **consult BEHAVIOUR**: a weak caller will
not consult from a preamble alone. This is precisely the case for the deferred
**uncertainty trigger** (C-3) — the consult must be driven by a measured
signal (self-consistency spread, verbalized low confidence) and/or made more
automatic than a prose invitation. The 2-tier limit stands (one point measured,
glm-5.2 advising glm-5-turbo). Follow-ups for a re-run: pin the exact function
name in task 1 (or make the hidden test name-agnostic); strengthen the consult
trigger; a third tier for the full RD-10 inversion.

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..05 and MT-C3-01)_

## Owner sign-off

_(recorded after the runs, as with MT-C2-01..05 and MT-C3-01)_
