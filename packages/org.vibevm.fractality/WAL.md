# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~06:48 (**Campaign 3 STAGE B COMPLETE — the RLM is
built and it runs**). Ф0–Ф7 all CLOSED. The whole fabric: need-gate (Ф2) →
descent core (Ф3) → escalation ascent (Ф4, D-C3-6) → acceptance (Ф5,
FD-9) → the trial (Ф6, D-C3-9) → close (Ф7). **Ф6 fired MT-C3-01: 3 paid
GLM gated-boss runs → delegation 8/18 ≈ 44.4% vs C2 naive baseline 16.7%
(~2.7×), and fractality ran end to end as a product for the first time**
(3 workers completed with a worker result, 1 acceptance 1/1). P-C3-c
CONFIRMED, P-C3-a SUPPORTED, P-C3-b/d inconclusive (menu gaps → PP-004).
Delegation switched opencode→CC+z.ai (works — it IS the trial mechanism).
**PP-003 advisor CORE also landed this session** (D-C3-7): the `output.advice`
marker + the RD-10 caller-class bar (`check_advisor_caller_class` refuses an
advice call whose caller is below `advisor_enabled`) + denorm + surfacing +
tests (`40687ca`), floor green (test-gate 215). Plan:
`FRACTALITY-ADVISOR-PLAN-v0.1`. **Next: a validated Stage C** (the advisor
help/hurt trial + uncertainty trigger + ladder-data) when the owner
commissions it. Campaign-close: `reports/…-campaign3-close.md`._

## Current state

- **Stage B COMMISSIONED — Option B** (RP-C3-1, plan §1/§8). Advisor
  (Option C/V4) postponed → PP-003.
- **Goal (owner, standing):** the WHOLE Stage B plan (Ф0→Ф7), a working
  RLM with all patterns — then **PP-003** (Option C advisor slice, D-C3-7).
  **70%-context stop rule is LIFTED** (owner 2026-07-12): run straight
  through with context compaction, no checkpoint-and-ask at 70%.
- **Ф0–Ф2 CLOSED** — spikes (jsonschema 0.47.0 on rustc 1.93.1); the
  packet + budget surface (D-C3-2); the need-gate machinery (`needgate::
  decide`, routing policy, profile class — D-C3-1/D-C3-10). Reports under
  `reports/…-f0/f1/f2….md`.
- **Ф3 CLOSED — the descent core** (9 slices): depth-guard, gate
  invocation + decision journal (D-C3-8), await `--any`, refuse-near-dup,
  masking, retry-on-violation, merge-node marker + `max_depth=0` fix.
  Report `…-f3-descent-core.md`.
- **Ф4 CLOSED — escalation (D-C3-6)**, the ascent (see header). The
  open Ф0-s4 question resolved: worker escalates via an **MCP tool**
  (`escalate`), not a result-status exit (fits the state machine). Report
  `…-f4-escalation.md`.
- **Floor green:** test-gate 211 / conform 0 / specmap clean. Real
  `~/.fractality` untouched. Every extension at a named seam.
- **Live tracker:** `reports/2026-11-07-17-52-rlmplan-state-plan.md`
  (goal + operating contract, seam reconnaissance — read it, crates need
  not be re-read; per-slice status; delegation scoreboard). Plan §9 ledger
  = commit map + scoping decisions.

## Next — a validated Stage C (advisor trial + trigger)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker → `FRACTALITY-ADVISOR-PLAN-v0.1` (§3 deferred) →
`plans/postponed/PP-003-…` + PP-004.

**Stage B is DONE and the PP-003 advisor CORE landed** (`40687ca`): the
`output.advice` marker + the RD-10 caller-class bar (enforced at admission)
+ denorm + `show` surfacing + tests, all floor-green. The advisor is now "a
worker run with an `advice` packet type". **Deferred to a validated Stage C**
(FRACTALITY-ADVISOR-PLAN §3): the uncertainty trigger (caller behaviour,
measured thresholds), the ladder-as-routing-data, a `fractality advise`
verb, and the **help/hurt trial** (an MT-C3-02-shaped pre-registration —
prove advice helps a medium caller, does not hurt a weak one; the RD-10
inversion is the falsifier). Commissioned when the owner mandates Stage C.
The core built this session is the foundation; treat the trial as its own
mandate with its own pre-reg discipline.

**Standing follow-ups (non-blocking, all filed):** Ф4 — worker-stop after
escalate is cooperative not enforced; Ф5 — no tree→verifier query,
acceptance does not feed routing (FD-5); Ф6 — worker turn caps, menu
exercised neither the schema gate nor a Silo task, no `decisions` read verb
(all → PP-004).

Each code slice = one commit, floor green after each; specmap re-mint
in-commit on drift. Floor via backgrounded cargo; delegation via CC+z.ai
GLM (the proven mechanism — it is also what the trial arms run on).

## Constraints (do not violate without discussion)

- Host Rules 1–4; **plan §10 executor guide is BINDING**; clean-room
  §10.4 (never open refs/src|papers|articles while coding); the delegation
  law + live-observation; I1–I7; no Python in shipped code; cwd law;
  commit heredoc; editor-tool edits (PS 5.1 corrupts UTF-8-no-BOM);
  specmap re-mint law; scratch homes; no `*install*` test binaries; F15
  (stop MC before builds); domain code has no `unwrap`/`expect` (conform);
  600-line conform cell budget (carve before adding).
- **Ф6 paid trial arms — RP-C3-2 PRE-AUTHORIZED 2026-07-11** (owner:
  «я прямо сейчас разрешаю делать эти платные прогоны»). Arms fire only
  after MT-C3-01 pre-registration is committed (§10.7 pre-reg-first still
  binds); budget posture confirmed at Ф6. MT-C2-05 stays RP5-gated.
- Fugu benchmark numbers are Sakana-reported — mechanism evidence only.

## Delegation scoreboard (session)

**MECHANISM SWITCH — opencode → CC+z.ai (the phase's process win).**
opencode/GLM stalled again (booted, no tool output, killed ~3 min). The
owner: launch GLM the way fractality itself does — headless Claude Code
(`claude -p`) at the z.ai Anthropic-compatible gateway. Recipe (verified,
from this workspace's `backend-claude-code/envbuild.rs` +
`spec/examples/profiles.sample.toml`): `env -u ANTHROPIC_API_KEY
ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic ANTHROPIC_AUTH_TOKEN=$(cat
~/.vibevm/zai.api.token) API_TIMEOUT_MS=3000000
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 claude -p '<task>' --model
glm-5.2[1m] --dangerously-skip-permissions --output-format stream-json
--verbose` from the workspace cwd. Never echo the token (`$(cat …)` → env
var, value never hits stdout). Watch heuristics (owner 2026-07-12): silent
>5 min after the first line ⇒ kill; actively producing ⇒ wait for exit up
to 30 min.

- **Delegated & succeeded:** the http.rs pod-leg carve (GLM glm-5.2 removed
  4 functions, cleaned all unused imports via a clippy loop, self-verified;
  clean diff on review). This is the model for Ф5→Ф7 + PP-003: mechanical
  carves, bulk edits, run-and-report → CC+z.ai GLM.
- **Kept boss-side (legitimately):** all seam design (the escalation state
  machine, the escalate endpoint/DTO/verb, the broker tool); the mc-client
  twin carve (small, content already loaded); floors run backgrounded (the
  harness notification IS the "don't babysit" the law wants — token-cheap);
  review of every delegated diff (the boss's half of the bargain).
