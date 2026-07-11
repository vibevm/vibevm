# rlmplan — state plan (living tracker)

_Campaign 3 Stage B execution tracker. Updated in place between status
documents (big-plan dashboard rule — bulk stays out of status files).
Source of truth is the spec tree (plan, syntheses, WAL); this is the
owner-facing surface + the agent's own quick tracker. Last updated:
2026-07-11 17:52._

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

- [~] **Ф0 spikes** (no commits) — IN PROGRESS
  - [ ] s1 schema-validate-at-seam — RISK: JSON-Schema lib choice + retry
        flow. Probe: validate structured output vs schema in Rust, show
        pass/fail + the violation-feedback shape. → delegate to GLM.
  - [ ] s2 FileRef slice handoff — RISK: low (machinery exists). Probe:
        child resolves + reads a Slice FileRef end-to-end under a proven
        scope.
  - [ ] s3 settings-injection promotion (CC) — RISK: CC settings format +
        what CC honors. Probe: write a capability grant into the child
        harness config at spawn; confirm CC reads it. → read env +
        initiative first.
  - [ ] s4 escalated-outcome round-trip — RISK: generalizing the D18
        channel. Probe: escalated(reason,needs) climbs `parent` edges to
        the boss/human.
- [ ] Ф1 packets & budgets (D-C3-2, D-C3-3)
- [ ] Ф2 need-gate + delegation-rules (D-C3-1, D-C3-10)
- [ ] Ф3 descent verbs (D-C3-4, D-C3-5)
- [ ] Ф4 escalation (D-C3-6)
- [ ] Ф5 acceptance / PP-002 (RD-11, FD-9)
- [ ] Ф6 trial (D-C3-9) — STOP at RP-C3-2
- [ ] Ф7 close

## Delegation scoreboard (running)

- **Kept (boss):** core-seam reconnaissance (packet / run / fileref /
  collect) — architecture, anchors all downstream phase design. Ruling
  records, PP-003, dashboards, commissioning — plan/spec authorship
  (never-delegate).
- **Delegated:** (none yet — spikes are the first delegation).

## Next action

Design + delegate s1 (schema-validate-at-seam) to GLM under live
observation; in parallel read backend-claude-code env + initiative route
for s3 seam design.
