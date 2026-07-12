# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~05:05 (**Campaign 3 Stage B — Ф4 COMPLETE: the
escalation channel is in end to end**). Ф0/Ф1/Ф2/Ф3 CLOSED; **Ф4 CLOSED
this session** (D-C3-6) across 4 slices + 1 carve, each floor-green +
committed. The ascent landed: terminal `RunState::Escalated` + record +
fold + `escalated` metric (**Ф4.1** `e13ddbf`); exit code 5 + `fractality
escalations` inbox with call-tree-root attribution (**Ф4.2** `6ed04e6`);
`POST /v0/runs/:id/escalate` + `McClient::escalate` + `escalation.md`
(**Ф4.3a** `3f9a2e4`); broker `escalate(reason, needs)` MCP tool (**Ф4.3b**
`0bf4242`). Pod leg carved to `http_pods.rs`/`pod_leg.rs` for headroom
(`2ce35f8`). **Next: Ф5 — acceptance / PP-002 fold-in.** Phase report:
`reports/2026-12-07-05-05-campaign3-f4-escalation.md`._

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

## Next — Ф5 (acceptance / PP-002 fold-in)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker → plan §10 (BINDING) + §9 (ledger) + §6 (phases).

**Ф5 (RD-11, FD-9), from plan §6:** acceptance verdicts can gate run-tree
completion (verifier-accept), and an acceptance packet on an
empty/workless tree is refused (no cold verification — the "cold verifier"
§10.2 that must be refused mechanically). Fold in PP-002 (acceptance).
Seams to read at slice time: `pod/collect.rs` (`run_acceptance`, already
writes per-cmd verdicts → `acceptance.log`), `core::run::Collected`
(`acceptance_passed`/`total`/`skipped` already exist), the await/collect
completion path (`swarm.rs`), and how the tree's completion is judged.
The acceptance plumbing (verdicts on the record) is ALREADY present from
Phase 4 — Ф5 adds the GATE (verdicts decide tree completion) + the
cold-verifier refusal.

**Ф4 follow-ups (non-blocking, NOT gating Ф5):** worker-stop after escalate
is cooperative not enforced (pod-reap is a future pod feature); no
automatic re-dispatch of an escalation (`fractality escalations` shows,
the boss acts — an `on_escalation` policy is Phase-5 delegation-rules);
`escalation.md` has no renderer yet (symmetric with question.md).

Each slice = one commit, floor green after each; specmap re-mint in-commit
on drift (ANY code change in a scoped file shifts tagged-item locations →
drift; re-mint every slice). **Floor via backgrounded cargo** (harness
captures the task file — NO `> log` redirect; explicit `cd <v0.1.0>`).

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
