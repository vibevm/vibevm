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

## 5. Next

A validated Stage C: the help/hurt trial (MT-C3-02-shaped), the uncertainty
trigger with measured thresholds, and the ladder-as-data. Commissioned by
the owner when Stage C is mandated; the core built here is the foundation.
