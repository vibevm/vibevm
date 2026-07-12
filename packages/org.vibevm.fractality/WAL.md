# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~06:44 (**Campaign 3 Stage B — Ф6 COMPLETE: the trial
FIRED and fractality ran end to end as a product for the first time**).
Ф0–Ф5 CLOSED (need-gate → descent → ascent → acceptance). **Ф6 CLOSED this
session** (D-C3-9): MT-C3-01 pre-registered (`3c8ea76`) → harness `run-arm.sh
g` + `preamble-g.md` (`1c4a8f8`) → 3 paid GLM gated-boss runs → recorded +
`score-g.py` (`67a3e4a`). **Gated delegation 8/18 ≈ 44.4% vs C2 naive
baseline 16.7% (~2.7×); 3 GLM workers completed with a worker result, 1
acceptance 1/1.** P-C3-c CONFIRMED, P-C3-a SUPPORTED, P-C3-b/d inconclusive
(menu gap). **Next: Ф7 — close Stage B** (verdicts, deferrals, campaign
report, WAL), then **PP-003** (Option C advisor). Phase reports:
`…-f6-trial.md`, `…-f5-acceptance.md`._

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

## Next — Ф7 (close Stage B)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker → plan §10 (BINDING) + §9 (ledger) + §7/§8.

**Ф7 (from plan §6): close Stage B.** The code phases are all done and the
trial fired; Ф7 is the wrap-up:
- **Verdicts** — record each §7 prediction's verdict (P-C3-a SUPPORTED,
  P-C3-b INCONCLUSIVE, P-C3-c CONFIRMED, P-C3-d INCONCLUSIVE) in the plan
  §7 / §8 and mark the D-C3 decisions all landed in §9.
- **Deferrals ledger** — the trial follow-ups (worker turn caps, a menu
  with a schema task + a Silo task, a `fractality decisions` read verb) and
  the Ф4/Ф5 follow-ups (worker-stop enforcement, tree→verifier query,
  acceptance-feeds-routing) belong in `plans/postponed.md` if not already.
- **Campaign-close report** in `reports/` + a `-completed-plan.md` dashboard
  stage; refresh the fractality status line in the host `WORKSPACES.md`.
- **WAL** to the Stage-B-complete checkpoint.

Then the owner's follow-on goal (2026-07-12): **PP-003 — Option C, the
advisor slice (D-C3-7)**: read `plans/postponed/PP-003-…`, research it, and
execute it to completion. Advisor = a worker-shaped run with an `advice`
packet type, no ownership transfer; `advisor_enabled ⇐ caller_class ≥
medium`; uncertainty-triggered; accounting on the caller's budget.

**Standing follow-ups (non-blocking):** Ф4 — worker-stop after escalate is
cooperative not enforced; Ф5 — no tree→verifier query; Ф6 — worker turn
caps bit, menu exercised neither the schema gate (P-C3-b) nor a Silo task
(P-C3-d).

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
