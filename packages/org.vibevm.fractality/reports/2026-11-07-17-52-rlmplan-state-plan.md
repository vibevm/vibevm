# rlmplan — state plan (living tracker)

_Campaign 3 Stage B execution tracker. Updated in place between status
documents (big-plan dashboard rule — bulk stays out of status files).
Source of truth is the spec tree (plan, syntheses, WAL); this is the
owner-facing surface + the agent's own quick tracker. Last updated:
2026-07-11 18:20 (70%-context checkpoint — Ф1.1 landed, paused for
restart)._

## Goal & operating contract (owner, 2026-07-11)

- **Goal (verbatim):** «нам нужно по факту довести кампанию 3 и получить
  рабочий RLM со всеми паттернами … сделай весь план … проектирование,
  кодирование, тестирование, нагрузка — сделай это всё.»
- **70%-context stop rule (verbatim):** «Если контекста останется мало
  (допустим мы израсходовали 70%) — остановись, запланируй следующие
  шаги, и попроси меня перезапустить сессию.» → at ~70% consumed: clean
  stop — rewrite WAL, write a `-paused-plan.md` (done / checklist ✅ /
  exact stop point), ask the owner to restart from a fresh context.
- **Hard stop (§10.7, no exceptions):** Ф6 paid trial arms fire only
  after MT-C3-01 pre-registration is committed AND the owner's verbatim
  RP-C3-2 word + budget posture are recorded. Everything else is routine
  (Rule 4).
- **Posture:** delegate grunt to GLM (spikes, build/test runs, mechanical
  edits, bulk reads) under live observation; boss keeps seam design,
  architecture, and review of every diff. One D-C3 = one commit-sized
  slice; floor green at every boundary (safe-stop law).

## Reconnaissance — existing seams (verified 2026-07-11, do not re-read)

Crates: core, mission-control, pod, mc-client, backend-claude-code, cli,
initiative. Floor is run FROM `fractality/v0.1.0/`.

**fractality-core:**
- `packet.rs` — `Packet{schema=1, task, context, workspace, output,
  budget, routing}`, `deny_unknown_fields`. Extension points:
  - `ContextSpec{files, notes}` ← **context_from** (D-C3-2 access-list)
  - `OutputSpec{result, branch}` ← **output_schema** (D-C3-2 / s1)
  - `BudgetSpec{wall_secs, max_turns, max_output_tokens}` ← **six axes +
    wall-clock** (RD-4): depth / per-agent-calls / per-call-token-ceiling
    / cumulative-tokens / currency / global-calls. `0 = unlimited` per
    axis is the existing convention.
  - `RoutingSpec{profile, model}` ← need-gate verdict/verb (D-C3-1)
  - schema stays `1`; new fields `#[serde(default, skip_serializing_if)]`
    to preserve the golden snapshot (`hello_glm` insta) + back-compat.
- `run.rs` — `RunState{Queued,Starting,Running,WaitingOnBoss,Completed,
  Failed,Killed}`; `RunRecord` ALREADY has `parent`, `depth`,
  `question`/`answer` (D18 park channel), `Collected{result_source,
  result:FileRef, acceptance_passed/total, acceptance_skipped}`.
  - **escalated(reason,needs)** (D-C3-6) generalizes the question/answer
    park channel from questions to task-outcomes — likely a new RunState
    (or Collected variant) that climbs via the `parent` edges.
- `fileref.rs` — `FileRef{fs:ScopeId, path, range, etag, sha256}`,
  `RefRange{Whole|Slice{offset,len}|Trim{skip_head,skip_tail}}`,
  `resolve_against(size)` (RFC 7233). **s2 handoff machinery already
  exists** — the spike only proves end-to-end child-reads-slice.

**fractality-pod:**
- `collect.rs` — `collect_result` (worker|extracted|none), `run_acceptance`
  (per-cmd verdicts → acceptance.log), `write_status`, `write_usage_json`.
  **s1 seam: output_schema validation folds in after `collect_result`;
  retry-on-violation is an MC/boss re-spawn decision, not pod-local.**

**Not yet read (delegate or read at slice time):** mc admission / http /
journal_store, backend-claude-code env / envbuild (s3), initiative route /
nudge (RD-12 settings-writes precedent), mc-client, cli surfaces.

## Phase checklist

- [x] **Ф0 spikes** (no commits) — CLOSED, all seams green (report:
      `2026-11-07-18-12-campaign3-f0-spikes.md`)
  - [x] s1 schema-validate-at-seam — GREEN (boss-side after 2 opencode
        failures). jsonschema 0.47.0 compiles on rustc 1.93.1; validate
        works; violation shape = `at <JSON-Pointer>: <message>`. API:
        validator_for → is_valid / iter_errors + err.instance_path().
  - [x] s2 FileRef slice handoff — GREEN by inspection: `RefRange::Slice`
        + `resolve_against` (RFC7233) exist and are unit-tested; FileRef
        carries scope+path+range+etag; handoff = pass a FileRef in the
        new `context_from` field (Ф1). No unknowns.
  - [x] s3 settings-injection promotion (CC) — GREEN by inspection: the
        capability surface is argv (--permission-mode / --allowed-tools /
        --disallowed-tools from `profile.permissions`) + --mcp-config
        broker + per-worker CLAUDE_CONFIG_DIR. Promotion = spawn a child
        whose profile `allow_tools` carries `Bash(fractality *)` (the
        nesting seam already named in profile.rs) + broker. No in-place
        promotion (§10.2); worker-side hooks out-of-scope (I5).
  - [~] s4 escalated-outcome round-trip — DESIGN resolved: add a terminal
        `RunState::Escalated` + `EscalationRecord{reason, needs}` on
        RunRecord; the run climbs via existing `parent` edges to the
        human at the top (generalizing the D18 question/answer park
        channel + AnswerRule). Open Q for Ф4: worker expresses escalation
        via an ask_boss-style MCP tool vs result status. Seam viability
        proven; no new daemon.
- [ ] Ф1 packets & budgets (D-C3-2, D-C3-3)
- [ ] Ф2 need-gate + delegation-rules (D-C3-1, D-C3-10)
- [ ] Ф3 descent verbs (D-C3-4, D-C3-5)
- [ ] Ф4 escalation (D-C3-6)
- [ ] Ф5 acceptance / PP-002 (RD-11, FD-9)
- [ ] Ф6 trial (D-C3-9) — STOP at RP-C3-2
- [ ] Ф7 close

## Delegation scoreboard (running)

- **Kept (boss):** core-seam reconnaissance (packet / run / fileref /
  collect / envbuild / env / invocation / profile / lib — 10 files) —
  architecture, anchors all downstream phase design. Ruling records,
  PP-003, dashboards, commissioning, all spike DESIGN — plan/spec
  authorship + seam design (never-delegate).
- **Delegated:** s1 schema spike → GLM glm-5.2 (library de-risk + build +
  run). **Field data (Phase-5 playbook):** opencode `run` auto-rejects
  `cargo build` inside a nested cargo project that owns its own `.git`
  (external_directory) — delegate cargo spikes IN-PLACE in the launch
  cwd with `cargo init --vcs none .`, never a subdir.

## Next action

Ф0 CLOSED. Executing Ф1 (D-C3-2, D-C3-3). Blast radius small: no external
`BudgetSpec{}`/`OutputSpec{}`/`ContextSpec{}` literals exist (grep), so
new `#[serde(default)]` fields touch only the `impl Default`s + the
hello_glm golden snapshot (insta accept). Slice plan (each = one commit,
floor green after each):
- **Ф1.1** `ContextSpec.context_from: Vec<RunId>` (access-list; default [])
  — ✅ LANDED (`35a378c`, floor green 165/0, specmap 170/63/63/0)
- **Ф1.2** `OutputSpec.output_schema` (raw JSON string in dep-light core;
  validated at pod/collect with jsonschema 0.47.0 + one retry)
- **Ф1.3** `BudgetSpec` six axes + wall-clock (RD-4): depth /
  per_agent_calls / per_call_token_ceiling / cumulative_tokens /
  currency / global_calls (0 = unlimited holds)
- **Ф1.4** D-C3-3 boundary behaviors per verb (MC + profiles)
Floor runs = backgrounded cargo (opencode unreliable today, Ф0 field data).
