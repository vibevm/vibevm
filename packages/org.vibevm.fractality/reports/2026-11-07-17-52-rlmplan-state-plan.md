# rlmplan — state plan (living tracker)

_Campaign 3 Stage B execution tracker. Updated in place between status
documents (big-plan dashboard rule — bulk stays out of status files).
Source of truth is the spec tree (plan, syntheses, WAL); this is the
owner-facing surface + the agent's own quick tracker. Last updated:
2026-07-12 (**Ф4 IN PROGRESS — escalation core outcome landed**). Ф3
CLOSED (descent core, 9 slices). Ф4.1 (`e13ddbf`) lands the D-C3-6 core:
terminal `RunState::Escalated` + `EscalationRecord`, typed `Event::
Escalated` + fold, `escalated` metrics counter — the journal fold carved
into `journal_fold.rs` on the way (600-line budget). Floor green. Next:
Ф4.2 (escalation climbs the parent edges) then Ф4.3 (worker expresses it).
Phase report: `reports/2026-12-07-02-40-campaign3-f3-descent-core.md`._

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
- [x] Ф1 packets & budgets — CLOSED on D-C3-2 (context_from,
      output_schema + validation, budget lattice); D-C3-3 → Ф2
- [x] Ф2 need-gate + delegation-rules — CLOSED (decide procedure +
      routing policy + profile class; goldens); gate wiring → Ф3
- [x] **Ф3 COMPLETE — the descent core** (every D-C3 decision landed,
      floor green, pushed). Minor non-blocking follow-ups noted below.
  - [x] Ф3.1 depth-guard — D-C3-3 spawn-past-cap refusal (`b23f3f1`)
  - [x] Ф3.2 gate invocation + decision journal (D-C3-8) — **COMPLETE**
    - [x] Ф3.2a `fractality gate` CLI + `can_spawn` overload fix (`3b0b2d2`)
    - [x] Ф3.2b decision journal — separate stem (soft-label table)
      - [x] Ф3.2b-i storage: `DecisionRecord`/`Envelope` + decisions
            stem (`record_decision`/`decisions`), tested (`2c0a128`)
      - [x] Ф3.2b-ii producer: `gate --record` → `/v0/decisions` +
            module splits (`8d8960a`) — end to end over the bus
  - [x] Ф3.3 availability masking (FD-8) — `usable_profiles`/`token_present`
        (`b21a4c6`); pure query, shipped tested ahead of the router that
        consumes it (Ф2 precedent)
  - [x] Ф3.4 descent verbs — await any|all|named (D-C3-4/5)
    - [x] Ф3.4a `fractality wait --any` race (`a1479f1`); `all`/`named`
          already existed (default-join / passing ids)
    - [ ] Ф3.4b (FOLLOW-UP, non-blocking): parallel-spawn is already the
          idiom (spawn+parent+await+dedup+merge); mid-task profile
          alternation is a pod/worker feature for a later phase
  - [x] Ф3.5 sibling isolation + merge node + refuse-near-duplicate
    - [x] Ф3.5a refuse-near-duplicate — `Packet::task_fingerprint` +
          `check_sibling_invariants` (`1189b3c`); full-spec match, not
          title-only, so fan-out passes
    - [x] Ф3.5b merge-node marker `output.merge` + at-most-one invariant
          (`9825f4e`); await/collect integration is a flagged follow-up
    - [~] sibling isolation is true BY CONSTRUCTION (a child sees only its
          packet + `context_from` results — the fold law); a pinning test
          would document, not enforce — optional follow-up
  - [x] Ф3.6 retry-on-violation re-dispatch (`867afc2`, D-C3-2) — the
        sync `fractality run` loop re-dispatches ONCE on a schema
        violation, reading `status.json` directly (no pod→MC protocol
        change, resolving the seam finding): `run_once` + `retry_report`
        in `swarm.rs`, violations folded into the retry's `context.notes`,
        bounded (gate checked only on the first attempt). `fractality
        spawn` has no wait point so no retry — correct.
- [~] **Ф4 escalation (D-C3-6)** — IN PROGRESS (s4 spike design now real)
  - [x] Ф4.1 escalation core outcome (`e13ddbf`) — terminal
        `RunState::Escalated` + `EscalationRecord{reason, needs}`; typed
        `Event::Escalated` + fold; `MetricsBucket.escalated` counter; MC
        validator target. Journal fold carved to `journal_fold.rs` (600
        budget), `journal::apply` re-exported (no caller churn); specmap
        re-minted. Edges: `running`/`waiting_on_boss → escalated` only
        (§10.8 minimal). Tested library — no producer yet. Floor green
        (test-gate 206 / conform 0 / specmap clean).
  - [ ] Ф4.2 escalation climbs the parent edges — surface escalated runs
        to the top (mirror D18 `runs(WaitingOnBoss)` triage with
        `runs(Escalated)`), attribute to the root via `parent` walk; a
        distinct `state_code` exit for the CLI. Terminal record climbs;
        run does not resume.
  - [ ] Ф4.3 worker expresses escalation — resolve open Q (ask_boss-style
        MCP tool vs result-status exit); wire the pod/backend + cli
        surface. May widen the `starting → escalated` edge if result-exit.
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
- **Ф3 attempt (2026-07-12):** the 12-file mc/cli seam inventory →
  GLM glm-5.2, live-observed (`--print-logs`, background, cwd pinned).
  **FAILED** — silent stall, 0-byte log, exit 0 after ~70 s (matches the
  prior-session field data: opencode read-delegation stalls silently on
  this box this session). Law honored (attempt made), then boss-read the
  seams itself — legitimate boss-keep (seam reconnaissance anchors phase
  design). **Field data:** opencode remains unreliable for reads this
  session; keep floor/test on backgrounded cargo. When it stabilizes,
  the seam-inventory read is the first thing to hand back.
- **Ф4.1 (2026-07-12):** no delegation. Work = seam design (the escalation
  state machine + journal fold + the forced `journal_fold.rs` module
  split) — all never-delegate/boss-keep (§10.7: seam design, and a
  discipline-critical split touching scope-marks + the cfg-test conform
  exemption). Floor ran via backgrounded cargo (the reliable path);
  first-output well under the 3-min law. Kept: legitimate; delegate
  remains unproven this session.

## Next action

Ф0 CLOSED. Executing Ф1 (D-C3-2, D-C3-3). Blast radius small: no external
`BudgetSpec{}`/`OutputSpec{}`/`ContextSpec{}` literals exist (grep), so
new `#[serde(default)]` fields touch only the `impl Default`s + the
hello_glm golden snapshot (insta accept). Slice plan (each = one commit,
floor green after each):
- **Ф1.1** `ContextSpec.context_from: Vec<RunId>` (access-list; default [])
  — ✅ LANDED (`35a378c`, floor green 165/0, specmap 170/63/63/0)
- **Ф1.2** `OutputSpec.output_schema` — ✅ Ф1.2a field (`d91780d`) +
  Ф1.2b validation at collect seam (`12b9824`, jsonschema 0.47.0, verdict
  → `status.json schema_gate`). Auto-retry deferred to Ф3 re-dispatch
  (plan §9); pumps extracted to `pump.rs` for the file budget.
- **Ф1.3** budget lattice — ✅ LANDED (`19c33e9`, six axes + wall-clock;
  new axes 0=unlimited, enforcement in Ф2/Ф4)
- **Ф1.4** D-C3-3 boundary behaviors → DEFERRED to Ф2 (needs the gate's
  verbs + caps; §9 ledger). **Ф1 CLOSED on D-C3-2.**

**Ф2 CLOSED** — need-gate machinery shipped: decision procedure (Ф2.1
`5adcceb`, §10.3), routing policy (Ф2.2 `011ef6c`, capability classes),
profile class (Ф2.3 `14f97b8`). Report:
`2026-11-07-19-17-campaign3-f2-needgate.md`.

**Ф3 IN PROGRESS.** Ф3.1 depth-guard landed (`b23f3f1`, floor green):
`register_run` refuses spawn-past-cap using the routing policy per parent
class (0=no-spawn), tightened by `budget.max_depth`. Read this session:
core `state.rs`/`admission.rs`/`http.rs`/`needgate.rs`/`routing.rs`/
`profile.rs`/`packet.rs`/`journal.rs`/`lib.rs`, cli `route_cmd.rs`,
delegation-rules `routing-policy.toml`. Seams for the rest are mapped.

**Ф3.2a DONE (`3b0b2d2`):** `fractality gate` CLI surfaces
`needgate::decide` (pattern: `route_cmd.rs`), and the `max_depth=0`
overload is resolved — `GateInputs.can_spawn` (derived from `cap > 0`)
gates the spawn arm, so a no-spawn class folds instead of spawning.

**Ф3.2b-i DONE (`2c0a128`):** the decision-journal STORAGE — core
`DecisionRecord`/`DecisionEnvelope` (owned, serde-flat one line) + a third
`decisions` sibling stem in `state.rs` (`open_stem`/`replay_stem`, no fold),
`record_decision`/`decisions`, tested. Design resolved: decisions ride
their own stem (a gate decision may have no run), NOT the run fold.

**Ф3.2b-ii DONE (`8d8960a`) — D-C3-8 COMPLETE.** `gate --record` (now
async + daemon-aware) POSTs its `DecisionRecord` to `/v0/decisions`; the
stem stores it; `GET /v0/decisions` reads it back. Http-level test over
the bus. Two files crossed the 600-line conform budget → split along
their seams (`http_decisions.rs`, mc-client `decisions.rs`).

**Next: the descent SEMANTICS (Ф3.5, D-C3-4/5) — the hardest remaining
slice.** Three parts: (1) **sibling isolation** by default — already true
by construction (a child sees only its packet + `context_from` results,
never a sibling transcript, the fold law); likely wants a PINNING TEST,
not new code. (2) a designated **merge node** answering the parent goal.
(3) **MC refuses near-duplicate child specs** — NB this needs a FULL-spec
match (title + goal + context), NOT title-only: a fan-out legitimately
spawns same-title children on different chunks, so title-only would break
the core parallel idiom. Likely needs a task fingerprint on the RunRecord
(new field + journal event). Design-laden — best started fresh; read
`mc-client/lib.rs`, cli `mc_cmd.rs`/`swarm.rs`/`broker.rs`, mc
`registry.rs` first. Then Ф3.3 masking (FD-8, dead-surface risk — needs a
multi-profile router consumer first), Ф3.6 retry.
(read `mc-client/lib.rs`, cli `mc_cmd.rs`/`swarm.rs`/`broker.rs` first),
Ф3.6 retry-on-violation. Each = one commit, floor green after each.
Floor runs = backgrounded cargo, NO redirect (harness captures the task
output file; a `> log` steals it — lesson this session).
