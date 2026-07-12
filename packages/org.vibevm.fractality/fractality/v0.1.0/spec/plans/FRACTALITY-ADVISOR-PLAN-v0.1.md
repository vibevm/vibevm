# FRACTALITY-ADVISOR-PLAN v0.1 — Stage C: the advisor channel (D-C3-7) {#root}

_Drafted 2026-07-12 as the owner's follow-on to Stage B (PP-003). The
advisor is VISION §V4 — the fabric's fourth plane, on top of the descent
(V2) + ascent (V3) Stage B built. This plan records the mandate, the core
that shipped in this session, and the parts deliberately deferred to a
validated Stage C. Authority: the Stage B plan §4 (D-C3-7), RD-10/RD-11,
VISION §V4._

## 1. Mandate

Build the **advisor channel**: a run may consult a **bigger** model for
*judgment* and **keep ownership** of its task. Unlike escalation (V3, which
hands the task UP), advice moves **sideways-up**: only judgment crosses; the
caller's loop, task, and ownership are unchanged (VISION §V4 table). "An
advisor holds no loop, no task, no state" — it returns advice and is done.

The economics **invert** relative to descent: descent has a big model plan
and small models work; advice has a small(er) model own the loop and a
bigger model advise. The load-bearing constraint (RD-10) is the **caller
capability bar** — a weak caller is made WORSE by advice (the minRLM/nano
inversion; the SWE-1.5→1.6 threshold), so only a caller of class ≥ medium
may consult.

## 2. What shipped this session (the advisor core, D-C3-7)

Most of the seam already existed after Stage B (worker-run shape,
`context_from` access lists, `capability_class`, the routing policy's
`advisor_enabled` row — Ф2 pre-built the bar). This session added the call
shape + the enforcement:

- **`output.advice` packet marker** — a worker-shaped run that returns
  judgment, no ownership transfer (`packet.rs`; Default + snapshot updated).
- **`RunRecord.advice` denorm** — so `ps`/`show` mark advisor runs and the
  accounting can attribute advice to the caller (`run.rs`, register-time).
- **The advisor bar, enforced** — `admission::check_advisor_caller_class`
  refuses (400) an advice call whose CALLER (parent run) is below
  `advisor_enabled` for its capability class (RD-10). A boss-spawned advice
  call (no parent = the human at the top) is always above the bar. Wired
  into `register_run`.
- **Surfacing** — `fractality show` marks an advice run ("consultation;
  the caller keeps ownership").
- **Tests** — a weak caller's advice call is refused 400; a strong caller's
  and a boss's are admitted and marked.

The advisor is now "a worker run with an `advice` packet type," exactly as
PP-003 framed it — the machinery is real and floor-green.

## 3. Deliberately deferred (a validated Stage C)

Stage B's rhythm — ship the machinery, then trial it — applies here. The
core is in; the following need field data or a trial and are NOT built:

- **The uncertainty trigger** — the caller consults only on intrinsic
  uncertainty (self-consistency spread, trace length, verbalized
  confidence — RD-10 / SRLM). WHEN to consult is caller behaviour (like
  "when to escalate" is), documented for the caller, not core-enforced.
  Thresholds must be measured, not guessed.
- **The ladder as policy data** — "advise with a rung above the caller;
  at the effective top, a slightly-smaller/same-size model" (VISION §V4).
  Today the caller names the advisor profile in the advice packet's
  `routing.profile`; promoting the ladder to a routing-policy table (which
  profile a class-X caller advises to) is a data slice.
- **A `fractality advise` CLI surface** — a first-class verb for a caller
  to make an advice call (today it is an advice-marked packet through
  `run`/`spawn`). Small, cosmetic.
- **The help/hurt trial (its own MT pre-registration)** — the RD-10
  inversion is the falsifier: prove advice HELPS a medium caller and does
  NOT hurt a weak one. Needs a menu with genuinely uncertain tasks + a
  paired-arm design (caller-with-advisor vs caller-alone). This is the
  Stage C equivalent of Ф6.

## 4. Decisions taken

1. **The bar is on the CALLER, enforced at admission** — not on the advice
   packet's own profile. The check reads the PARENT run's capability class
   (the caller), because RD-10 is about who ASKS, not who answers. A
   boss-spawned advice call has no parent → the caller is the human/top →
   always admitted.
2. **`advice` lives in `OutputSpec`** next to `merge`/`verifier` — the
   established home for run-role markers, even though "advice" is a call
   shape more than an output. Consistency over purity (§10.8).
3. **No ownership transfer is a semantic property, enforced by absence** —
   an advice run is a normal worker run whose result the caller reads via
   `context_from`; nothing merges it into the caller's task or completes the
   caller's work. There is no code that COULD transfer ownership from an
   advice run, which is the point.
4. **The trigger + trial are deferred, not designed-and-skipped** — like
   Stage B, the machinery ships first; the behavioural trigger and the
   validating trial follow with real data (PP-003 unblock questions stand).

## 5. Stage C executed (2026-07-12)

The owner commissioned Stage C; it ran end to end:

- **The `fractality advise` verb** (`a1a9403`) — the V4 CLI surface (marks
  `output.advice`, sync-runs like `run`, MC applies the RD-10 caller-class bar).
- **The ladder as routing data** (`0740bc3`) — `ClassPolicy.advisor_class`
  (medium→strong, strong→strong, weak→none) + `advisor_class_for`.
- **The help/hurt trial** (MT-C3-02, `40382b4`) — fired alone×3 + advised×3
  (caller glm-5-turbo, advisor glm-5.2). **Result: a null MECHANISM.** The
  advised caller NEVER consulted (0 advice calls), so ADVISED = ALONE = 66.7%
  with no advice effect to measure. PR-adv-2 falsified — a preamble invitation
  is not enough to make a weak caller reach for the advisor. This is the
  finding that shapes §6.
- **The uncertainty trigger** — documented in §6 below (C-3).

The machinery is real and floor-green; the trial isolated the remaining gap as
the consult BEHAVIOUR, which §6 addresses.

## 6. The uncertainty trigger — the consult protocol (C-3)

The MT-C3-02 finding is unambiguous: **a weak caller will not consult from a
prose invitation.** Advice is only ever useful if the consult actually
happens, so WHEN and HOW a caller consults is the load-bearing behaviour — not
the packet format, which is done.

**The signal — WHEN to consult (RD-10 / SRLM).** A caller consults on
*intrinsic uncertainty*, measured, not on a vibe:

- **self-consistency spread** — sample the same sub-decision k times (k≈3–5);
  disagreement among the samples is the strongest cheap uncertainty signal.
  Wide spread → consult.
- **verbalized confidence** — ask the caller to state a 0–1 confidence on the
  subtle case; below a threshold → consult. (Weakly calibrated on small models;
  use as a secondary signal.)
- **trace length / branchiness** — an unusually short or thrashing trace on a
  task that should take work is a proxy for "guessed" → consult.

Only a caller of class ≥ medium consults (the RD-10 bar, already enforced at
admission); a weak caller is barred because advice makes it worse.

**The delivery — HOW to make it fire.** MT-C3-02 proves the open question is
delivery, not signal. Three escalating options, cheapest first:

1. **A stronger, structural preamble** — not "you MAY consult" but a required
   step: "for each subtle case, FIRST sample your answer 3× (self-consistency);
   if they disagree, you MUST `fractality advise` before committing." Re-run
   MT-C3-02 with this to test whether *mandated* consult moves quality.
2. **A harness/hook that forces it** — a PostToolUse-style check that, on a
   detected-uncertain step, injects the advise call or blocks the commit until
   the caller consults. This makes the trigger the fabric's job, not the weak
   caller's discipline — the same lesson PP-001 taught for the initiative nudge.
3. **Core-side, later** — a gate that, given an uncertainty score on a run,
   recommends `advise` the way the need-gate recommends `route`/`escalate`.

**Thresholds — still to be MEASURED (not guessed).** MT-C3-02 could not
measure them because no consult fired. The re-run under delivery-option 1 or 2
is what produces them: the self-consistency spread and confidence values at
which advice HELPS a medium caller and does NOT hurt a weak one (the RD-10
inversion), and it needs a third model tier for the weak-caller arm. Until
then this section is the *method*, and the thresholds are the deferred
measurement — filed honestly, not invented.
