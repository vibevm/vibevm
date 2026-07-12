# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~05:26 (**Campaign 3 Stage B — Ф5 COMPLETE: the
acceptance channel is in**). Ф0–Ф4 CLOSED (need-gate → descent → ascent);
**Ф5 CLOSED this session** (FD-9): `output.verifier` marker + cold-verifier
suppression (**Ф5.1** `85ac2a7` — `check_verifier_has_work` refuses a
verifier over an empty/resultless tree); `RunRecord.verifier` denorm +
verifier-accept verdict (ACCEPTED/REJECTED) in `run`/`show` (**Ф5.2**
`af977a4`). Floor green (test-gate 213). **Next: Ф6 — trial (D-C3-9):
pre-register MT-C3-01 FIRST (§10.7 BINDING), then RP-C3-2 paid arms.**
Phase reports: `…-f5-acceptance.md`, `…-f4-escalation.md`._

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

## Next — Ф6 (trial, D-C3-9)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker → plan §10 (BINDING) + §9 (ledger) + §6/§7/§8.

**Ф6 (D-C3-9), from plan §6:** pre-register **MT-C3-01** — write + COMMIT
the pre-registration FIRST (§10.7 pre-reg-first is BINDING, NO exceptions)
— then fire the budget-matched paid arms (**RP-C3-2 PRE-AUTHORIZED**
2026-07-11: «я прямо сейчас разрешаю делать эти платные прогоны» — no
second word needed once the pre-reg lands), score, record fatigue +
uncertainty facts. GLM cold boss (RP1 precedent); an orchestration-collapse
probe (two isolated siblings, one seeded with a misleading early action).
Budget posture (arm count, spend cap) chosen at commissioning per the
RP1/RP5 precedent.

**⚠ CAUTION — Ф6 spends REAL money and is the first true end-to-end paid
use of fractality.** Before firing scored arms, verify the system actually
runs a real GLM worker end to end (spawn → work → collect) — expect
integration surprises (this is the pilot's first live product run). Seams
to read at slice time: `spec/manual-tests/` MT files, the Campaign-2 trial
reports (RP1/RP5 precedent + arm design), the initiative/trial harness,
and the CC+z.ai launch recipe (delegation scoreboard) — the arms ARE GLM
workers under Claude Code, the same mechanism now used for delegation.

**Ф5 follow-ups (non-blocking):** no "given a work run, find its verifier"
query (tree→acceptance) — needs `context_from` denorm or a scan; acceptance
does not yet feed routing (FD-5 soft-label table, off the D-C3-8 seam).

Each slice = one commit, floor green after each; specmap re-mint in-commit
on drift (ANY code change in a scoped file drifts specmap; re-mint every
slice). Floor via backgrounded cargo; delegation via CC+z.ai GLM for
mechanical/bulk/run-and-report.

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
