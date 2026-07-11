# FRACTALITY-RLM-PLAN v0.1 — Campaign 3 · Stage B: the descent core (DRAFT) {#root}

_Status: **DRAFT 2026-07-11 — NOT COMMISSIONED.** Authored by
Stage A's Ф5 as its exit deliverable (research plan D-R7/RP-R3):
drafting is in scope, execution is a separate owner decision.
Every §4 decision cites the RD-deltas of
[`RLM-SYNTHESIS.md`](../refs/notes/RLM-SYNTHESIS.md); the mandate
slot below is empty until the owner speaks. Genre: campaign plan.
**Revised same day by Stage A2 (the Fugu research): three changes
and one new decision applied per
[`FUGU-FRACTALITY-MAPPING.md`](../refs/notes/FUGU-FRACTALITY-MAPPING.md)
(FD-citations inline); nothing falsified.**_

## 1. The mandate {#mandate}

_(empty — RP-C3-1. The owner rules: scope cut, budget posture,
timing. The three cut options are §3.)_

Standing context the mandate lands on: VISION §V2 («Должна быть
реализация RLM-спуска и всего RLM-процесса») + §V1/V3/V4 pillars;
the INITIATIVE §15 deferrals; PP-001 (MT-C2-05 re-run, RP5-gated)
and PP-002 (credibility facts) as interacting mandates.

## 2. Goal {#goal}

Give the fabric its descent core: packets that carry context by
reference and return symbols; a need-gate that decides inline /
fold / spawn / escalate on data, not vibes; budgets that make
recursion safe for untrained models; and the ascent (escalation)
channel that descent provably requires (Silo Effect). Exit with a
pre-registered trial proving the machinery on a cold boss under
budget-matched arms.

## 3. Scope options for the mandate (pick one) {#scope}

- **Option A — descent only (V2 core):** Ф1–Ф4 + trial. Smallest
  honest slice; escalation stubs but no advisor.
- **Option B — descent + ascent (V2+V3; RECOMMENDED):** adds the
  escalated-outcome channel (RD-6 says descent without ascent is
  incomplete by theorem — Silo tasks MUST go up). Advisor (V4)
  deferred to Stage C.
- **Option C — full fabric (V2+V3+V4 minimal):** adds the
  advisor call behind its capability bar (RD-10). Largest; only
  if the owner wants V4 field data this campaign.

PP-002 (acceptance plumbing) folds naturally into any option's Ф5
(RD-11 defines the acceptance packet's context posture); the
owner may instead keep it standalone.

## 4. Decisions (seeded from synthesis; finalized at commissioning) {#decisions}

- **D-C3-1 Need-gate verb** (RD-1, RD-2, RD-6, RD-16; FD-1): one
  auditable MC/boss call with typed verdict `inline | route |
  fold-local | spawn | escalate` + journaled reason — `route` is
  the cheap tier (dispatch ONE worker, no workflow ceremony,
  priced for latency; Fugu-standard's whole business); policy
  columns in `delegation-rules` (window-fit guard, O(1) guard,
  native-strength guard, regime triage, depth caps by model
  class × task class). Rejected: prompt-embedded judgment
  (unauditable, untrainable).
- **D-C3-2 Packet extensions** (RD-5, RD-4; FD-2): context by
  FileRef slice **plus explicit `context_from: [result-refs]` —
  the access-list contract: a child sees exactly the named prior
  results, never parent-gives-everything**; `output_schema`
  validated at the seam with one retry-on-violation; result =
  files + status, transcripts never cross upward; budget block
  gains the six axes + wall-clock; depth + parent ride the packet
  and the worker env (FRACTALITY_DEPTH).
- **D-C3-3 Boundary behaviors** (RD-3): spawn-past-cap → 
  structured refusal; at-cap profiles → capability removal +
  leaf surface; at-cap work → force-execute. Per-verb, recorded
  in profiles.
- **D-C3-4 Await verbs** (RD-9; FD-7): `await any|all|named` in
  mc-client + CLI; parallel siblings are the default idiom;
  **mid-task profile alternation is a first-class boss move** (the
  next packet of a run may go to a different profile — Fugu's
  per-step wins live exactly there).
- **D-C3-5 Aggregation, isolation & single-writer law** (RD-7,
  RD-8; FD-3, FD-4): **sibling isolation is the default — a child
  sees another child's work only if `context_from` grants it**
  (anti-"orchestration collapse"); shared memory persists ACROSS
  turns, never implicitly within a fan-out wave; designated merge
  node answering the parent's goal, **its profile chosen for the
  aggregation domain, never a fixture**; MC refuses
  near-duplicate child specs; briefs carry
  objective/format/tools/boundaries.
- **D-C3-6 Escalation channel** (RD-6; Option B+): packet
  outcome `escalated(reason, needs)` climbing the run tree to
  the human at the top; generalizes Ф5's answer channel from
  questions to tasks.
- **D-C3-7 Advisor slice** (RD-10, RD-11; Option C only):
  advisor = worker-shaped run with an `advice` packet type, no
  ownership transfer; `advisor_enabled ⇐ caller_class ≥ medium`;
  uncertainty-triggered; accounting line = caller's budget
  (revisit trigger: first field data).
- **D-C3-8 Journal schema** (RD-13, RD-17; FD-5, FD-13): decision
  tuples, delegation edges as events, snapshot+checksum, the
  plateau-explosion stall signature; sample_rate knob reserved;
  **the per-worker × task-class outcome table is a first-class
  query** (soft-label routing data — feeds delegation-rules now,
  a learned router later); **result metadata surfaces tree
  depth/spawn counts by default** (transparency as product edge).
- **D-C3-9 Trial protocol** (RD-21, RD-19; FD-3): MT-C3-01
  pre-registered before arms fire; budget-matched arms; surface
  wording as a controlled variable; GLM cold boss per RP1
  precedent; **an orchestration-collapse probe** (two isolated
  siblings, one seeded with a misleading early action — the
  fabric must keep them independent); interaction with PP-001's
  MT-C2-05 decided at commissioning (fire order matters for
  attribution).
- **D-C3-10 Routing policy data** (FD-8, FD-11, FD-16, FD-5; V4
  ladder, RD-2): profiles declare **availability**, and the
  need-gate routes over the available subset (mask the absent —
  also V4's effective-top fallback, made mechanical); policy rows
  address **capability classes, never model names** (pool churn
  is the design condition); the routing policy stays tabular
  data in v1, its features drawn from the journal's outcome
  table. Rejected: a learned router in v1 (RD-20 defers the
  lever; Trinity proves the brain can be tiny when features are
  right, which is an argument for better features first).

## 5. Current-state facts (verified at draft time) {#facts}

Floor green at `6c4ca62` (164 tests / conform 0 / specmap 63
units / 0 orphans). Crates: core, mission-control, pod,
mc-client, backend-claude-code, cli, initiative. Packets/runs/
budgets exist in v0.1 form (PROP-001 §2); FileRef exists; await
verbs do NOT; schema validation at the seam does NOT; escalation
exists only as parked questions + answer rules (Ф5 slice);
settings-injection machinery exists in the initiative adapter
(RD-12's promotion mechanics reuse it). Research corpus: 11
notes + synthesis under `spec/refs/notes/`; INVENTORY S1–S26 all
license-cleared; refs tree local and pinned.

## 6. Phases (shape; final counts at commissioning) {#phases}

- **Ф0 spikes (no commits):** s1 schema-validate-at-seam probe;
  s2 FileRef slice handoff probe; s3 settings-injection
  promotion probe on CC (RD-12); s4 escalated-outcome round-trip
  probe. Each green or its Decision is rewritten in place.
- **Ф1 packets & budgets** (D-C3-2, D-C3-3) → floor green.
- **Ф2 need-gate + delegation-rules columns** (D-C3-1) → goldens
  for the policy table.
- **Ф3 descent verbs** (D-C3-4, D-C3-5) → await-any/all + merge
  node + refuse-duplicate check.
- **Ф4 escalation** (D-C3-6; Option B+).
- **Ф5 acceptance/PP-002 fold-in** (RD-11; FD-9) — acceptance
  verdicts can gate run-tree completion (verifier-accept), and an
  acceptance packet on an empty/workless tree is refused
  (no cold verification) — or advisor slice (D-C3-7) under
  Option C.
- **Ф6 trial** (D-C3-9): pre-register → fire → score → fatigue +
  uncertainty facts.
- **Ф7 close:** verdicts, deferrals ledger, reports, WAL.

## 7. Prediction candidates (frozen at commissioning) {#predictions}

- P-C3-a: the need-gate's window-fit guard alone removes ≥ X% of
  unnecessary descents in the trial (X set with baseline data).
- P-C3-b: schema-validated returns cut malformed-result rework to
  ~zero across trial runs.
- P-C3-c: no trial run exceeds its wall-clock budget (the axis
  nobody else enforces — our differentiator holds under fire).
- P-C3-d: ≥1 Silo-regime task in the trial menu escalates rather
  than fans out, and scores better for it.

## 8. Review points {#review-points}

- **RP-C3-1 — mandate & scope cut (OPEN):** owner picks Option
  A/B/C, budget posture, timing; ruling verbatim here.
- **RP-C3-2 — trial arms authorization (opens at Ф6):** paid arms
  gated like RP1/RP5.
- Standing: PP-001's RP5 remains independent; firing order of
  MT-C2-05 vs MT-C3-01 is part of RP-C3-1.

## 9. Ledger {#ledger}

_(empty — DRAFT)_

## 10. Executor's guide — read this before any code {#executor-guide}

_Added 2026-07-11 at the owner's order, for the sessions that will
execute this plan on non-Fable models (Opus-class boss, GPT/GLM
workers). Everything here was implicit context for the authoring
model; for you it is LAW. When this section and your own judgment
disagree, this section wins; when it and the owner disagree, the
owner wins._

### 10.1 Reading order for the executing session {#eg-reading}

1. The workspace `CLAUDE.md`, end to end (its laws bind every
   commit, especially the delegation law and machine quirks).
2. This plan, whole.
3. [`RLM-SYNTHESIS.md §3`](../refs/notes/RLM-SYNTHESIS.md) and
   [`FUGU-SYNTHESIS.md §3`](../refs/notes/FUGU-SYNTHESIS.md) —
   every `RD-n`/`FD-n` cited in §4 resolves THERE, nowhere else.
4. A study note under `spec/refs/notes/` ONLY when the decision
   you are implementing cites it — never the whole shelf.
5. `PROP-001-foundation.md` §2–§4 (the system model you extend).

Do not boot the host repo tree. Do not re-read the research
papers or reference repos — see 10.4.

### 10.2 Glossary — our words, exact meanings {#eg-glossary}

- **packet** — a TOML task contract (goal, context refs, output
  contract, budgets, routing). **run** — one packet executed by
  one worker. **run tree** — runs spawned from within runs.
- **The five need-gate verdicts** (D-C3-1), operationally:
  - `inline` — the boss does it itself. No worker, no packet.
  - `route` — ONE worker call with the task as-is. No
    decomposition, no child tree, parent blocks. (Fugu-standard's
    entire business. Cheap. Default for single-skill tasks that
    fit the worker's window.)
  - `fold-local` — a bounded sub-session in the boss's OWN
    context space (e.g. a nested headless CC call) that returns
    a summary; no pod, no isolation. For context-heavy subtasks
    that need neither parallelism nor a fresh environment.
  - `spawn` — child packet(s) through MC, each with its own
    budgets, env, and run identity. The only verdict that
    creates a run tree.
  - `escalate` — return the task UP, annotated with
    `escalated(reason, needs)`. This is a first-class OUTCOME,
    not a failure. The top of every chain is the human.
- **context_from** — a packet field: a list of run-ids whose
  RESULT files become readable FileRefs for this child. Default
  `[]`. There is deliberately NO mechanism for a child to see a
  parent's or sibling's transcript — only named results.
- **the fold law** — worker transcripts never enter any parent
  context, ever. Parents consume `result.md` + `status.json`.
- **orchestration collapse** — when parallel siblings can see
  each other's early actions, the first mover steers everyone
  into one trajectory and the fan-out is wasted. Prevention:
  isolation is the default; visibility only via `context_from`.
- **cold verifier** — an acceptance/verification packet issued
  over a tree with no work output. Refuse it mechanically.
- **availability mask** — routing considers only profiles that
  are currently usable (credentials present, quota not
  exhausted); absent profiles are excluded before scoring.
- **soft-label table** — the journal query "per (worker-class ×
  task-class): attempts, successes, mean cost" (D-C3-8). It is
  DATA for delegation-rules, not a trained model.
- **Silo / Brain-Fog / trivial regimes** (RD-6) — task-noise
  dominant (cross-chunk reasoning dies under any split) /
  model-noise dominant (long input degrades one model) / O(1)
  lookup. Silo ⇒ escalate or route-to-biggest-window; Brain-Fog
  ⇒ spawn; trivial ⇒ inline or route.
- **boss surface / promotion** — capability grants written into
  the child's OWN harness config at spawn (settings/permissions
  injection). Promotion = spawning (or re-launching) a child
  WITH spawn rights + initiative hooks; the default child has
  none. There is no in-place promotion in v1.
- **floor** — the whole gate panel (`rust-ai-native floor`),
  run FROM `fractality/v0.1.0/`. Green = all six stages pass.

### 10.3 The need-gate decision procedure (v1, fixed order) {#eg-needgate}

Evaluate top-down; first match wins; journal the verdict + a
one-line reason verbatim:

1. Task is an O(1) lookup / single fact → `inline` (or `route`
   if it needs a tool the boss lacks). Never spawn for these.
2. Task + its context fit the candidate worker's window with
   ≥ 30% margin AND the task is single-skill → `route`. Do NOT
   decompose what fits — evidence says wrapping natively-capable
   models makes them WORSE (RD-2's guards; three independent
   sources).
3. Cross-chunk dependence dominates (the answer needs global
   reasoning over the whole context; chunking destroys it) →
   `escalate` — or `route` to the largest-window available
   profile if one exists. Do not fan out; it saturates below
   optimal regardless of worker quality (Silo theorem).
4. Decomposable, sub-results composable (you can write per-child
   goals whose outputs merge mechanically or via one designated
   merge node) → `spawn`. Depth cap from delegation-rules —
   default `max_depth = 1`; depth 2 exists only behind the
   experimental flag and only for provably super-linear tasks.
   When writing child goals, REWRITE them for composition (the
   "2nd smallest → per-chunk two smallest" planner trick, RD-8).
5. Otherwise (context-heavy, sequential, no isolation need) →
   `fold-local`.

### 10.4 Clean-room rules for executors — legally load-bearing {#eg-cleanroom}

- Implement ONLY from: this plan, the syntheses, the study notes,
  PROP-001. **Never open `refs/src/*`, `refs/papers/*`, or
  `refs/articles/*` during an implementation session.** Not to
  "check how they did it", not for a function name, not once.
- If a note under-specifies something you need — STOP that slice,
  write the question into this plan's §9 ledger, and ask the
  owner. Re-reading a source is a separate STUDY act with its
  own INVENTORY discipline, never part of coding.
- Never copy code from anywhere; never paraphrase code
  structure file-by-file. Ideas from the notes, expression
  entirely yours. One violation contaminates the codebase.

### 10.5 v1 minimalism — things you must NOT build {#eg-minimalism}

No learned router (no ES, no RL, no hidden-state features — the
routing policy is a TOML table in delegation-rules). No Python
REPL surface. No natural-language workflow grammar (packets are
TOML; the Conductor's NL plans are explicitly non-adopted). No
new daemons beyond mission-control. No string sentinels
(`FINAL()`-style) anywhere — outcomes are files + status.json.
No global rewrite of existing crates: every D-C3 lands as an
extension at named seams.

### 10.6 Where code lands {#eg-crates}

Packet/type/budget extensions → `fractality-core`. Enforcement,
spawn, await-verbs, kill, journal events → `fractality-mission-
control`. Client verbs + CLI surface → `fractality-mc-client` /
`fractality-cli`. Claude-Code specifics (settings injection,
promotion grants) → `fractality-backend-claude-code`. Nudge/board
text changes → `fractality-initiative`. Policy tables → the
`delegation-rules` package (its OWN version dir and Cargo
workspace — not inside `fractality/v0.1.0`).

### 10.7 Process laws, restated for you {#eg-process}

- One D-C3 decision = one commit-sized slice. Never batch two
  decisions into one commit. After every slice: run the floor
  (from `fractality/v0.1.0/`, cwd law). If the floor stays red
  longer than ~30 minutes of fixing: revert the slice, record
  what broke in the ledger, ask.
- Any commit that adds/renames an anchored spec section re-mints
  `specmap.json` IN THE SAME COMMIT (`rust-ai-native specmap`).
- Long runs (tests, builds, trials): background + log file +
  watcher; **first-output timeout ≤ 3 min** (a silent delegate
  is dead — kill and relaunch); never filter a live pipe through
  `head`; completion = notification, never a blind timeout.
- Git: commit messages via `git commit -F - <<'MSG'` heredoc
  only; Conventional Commits, scope `fractality`; NEVER any
  AI-attribution trailer (host Rule 1 — absolute).
- File edits via editor tools only (PowerShell 5.1 corrupts
  UTF-8-no-BOM round-trips on this box).
- Tests/trials NEVER touch the real `~/.fractality` — scratch
  homes always; never name a test binary `*install*` (Windows
  UAC). Stop MC daemons before workspace builds (F15).
- Paid trial arms fire only after pre-registration is committed
  AND the owner's RP word is recorded verbatim. No exceptions.
- Delegate mechanical work to GLM (`opencode run -m
  zai-coding-plan/glm-5.2 …`) per the workspace delegation law;
  the boss reviews every delegated diff. Never delegate: spec or
  plan authorship, seam design, anything touching secrets.

### 10.8 When you are unsure {#eg-unsure}

Follow the conflict-protocol ladder: re-read the governing spec
section → find the closest analog in the codebase → choose the
CONSERVATIVE interpretation (cheapest to reverse) → mark it
`<!-- REVIEW: ... -->` → proceed and report. Never invent
semantics silently; never "improve" a recorded decision without
naming its revisit trigger. The authority order is Human > Spec >
Tests > Code > WAL — recency is not authority.
