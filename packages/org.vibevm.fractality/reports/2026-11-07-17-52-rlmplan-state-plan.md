# rlmplan — state plan (living tracker)

_Campaign 3 Stage B execution tracker. Updated in place between status
documents (big-plan dashboard rule — bulk stays out of status files).
Source of truth is the spec tree (plan, syntheses, WAL); this is the
owner-facing surface + the agent's own quick tracker. Last updated:
2026-07-12 01:45 (Ф3 IN PROGRESS — gate wiring complete (D-C3-3 +
D-C3-8), await `--any` (3.4a), and refuse-near-duplicate (3.5a) all in,
each floor green + pushed. Remaining Ф3: Ф3.5b merge node (design-laden)
+ the sibling-isolation pinning test, masking (FD-8, maybe defer), retry
(3.6))._

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
- [~] Ф3 gate wiring + descent verbs — IN PROGRESS
  - [x] Ф3.1 depth-guard — D-C3-3 spawn-past-cap refusal (`b23f3f1`)
  - [x] Ф3.2 gate invocation + decision journal (D-C3-8) — **COMPLETE**
    - [x] Ф3.2a `fractality gate` CLI + `can_spawn` overload fix (`3b0b2d2`)
    - [x] Ф3.2b decision journal — separate stem (soft-label table)
      - [x] Ф3.2b-i storage: `DecisionRecord`/`Envelope` + decisions
            stem (`record_decision`/`decisions`), tested (`2c0a128`)
      - [x] Ф3.2b-ii producer: `gate --record` → `/v0/decisions` +
            module splits (`8d8960a`) — end to end over the bus
  - [ ] Ф3.3 availability masking (FD-8) — dead-surface risk, may defer
  - [~] Ф3.4 descent verbs — await any|all|named (D-C3-4/5)
    - [x] Ф3.4a `fractality wait --any` race (`a1479f1`); `all`/`named`
          already existed (default-join / passing ids)
    - [ ] Ф3.4b parallel-spawn idiom + mid-task profile alternation
  - [~] Ф3.5 sibling isolation + merge node + refuse-near-duplicate
    - [x] Ф3.5a refuse-near-duplicate — `Packet::task_fingerprint` +
          `admission::check_not_duplicate` (`1189b3c`); full-spec match,
          not title-only, so fan-out passes
    - [ ] Ф3.5b merge node (design-laden — what it IS / how designated)
          + sibling-isolation pinning test (isolation already true by
          construction; the test would document, not enforce)
  - [ ] Ф3.6 retry-on-violation re-dispatch (deferred from Ф1.2b)
        **SEAM FINDING (this session, verified):** the schema-gate result
        does NOT reach MC — `core::run::Collected` (and the `PodEvent::
        Collected` it rides) carry only result + acceptance, never the
        schema verdict; the pod writes `schema_gate` to `status.json`
        locally (Ф1.2b). So retry needs the schema result PLUMBED first:
        either add `schema_valid`/violations to `Collected` (pod→MC
        protocol change) OR have MC read `status.json` in the Collected
        handler. Then re-dispatch once with the violation report in the
        retry child's `context.notes`, bounded by a retry marker to
        prevent loops. Protocol-touching + multi-crate → best fresh.
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
- **Ф3 attempt (2026-07-12):** the 12-file mc/cli seam inventory →
  GLM glm-5.2, live-observed (`--print-logs`, background, cwd pinned).
  **FAILED** — silent stall, 0-byte log, exit 0 after ~70 s (matches the
  prior-session field data: opencode read-delegation stalls silently on
  this box this session). Law honored (attempt made), then boss-read the
  seams itself — legitimate boss-keep (seam reconnaissance anchors phase
  design). **Field data:** opencode remains unreliable for reads this
  session; keep floor/test on backgrounded cargo. When it stabilizes,
  the seam-inventory read is the first thing to hand back.

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
