# FRACTALITY-RLM-PLAN v0.1 — Campaign 3 · Stage B: the descent core (DRAFT) {#root}

_Status: **COMMISSIONED 2026-07-11 — Option B (descent + ascent).**
The owner ruled RP-C3-1 on 2026-07-11 (verbatim in §1 and §8);
drafting was Stage A's Ф5 exit deliverable (research plan
D-R7/RP-R3), execution begins now under Option B. Every §4 decision
cites the RD-deltas of
[`RLM-SYNTHESIS.md`](../refs/notes/RLM-SYNTHESIS.md); the mandate
slot below (§1) now carries the ruling. Genre: campaign plan.
**Revised same day by Stage A2 (the Fugu research): three changes
and one new decision applied per
[`FUGU-FRACTALITY-MAPPING.md`](../refs/notes/FUGU-FRACTALITY-MAPPING.md)
(FD-citations inline); nothing falsified.**_

## 1. The mandate {#mandate}

**RP-C3-1 ruled 2026-07-11 — Option B (descent + ascent).** Owner
verbatim: «Вариант 1. Вариант плана - B (нисхождение + эскалация).
Вариант C с адвайзором - отдельная задача, запланируй.»

The ruling settles:

- **Scope = Option B (§3):** the descent core plus the escalation
  (ascent) channel — V2+V3. RD-6's Silo theorem makes ascent part
  of descent's correctness, so the advisor (V4) is the single piece
  cut, not a plane left half-built.
- **Option C (the advisor, V4) becomes a separate future task** at
  the owner's explicit order («отдельная задача, запланируй»),
  registered as
  [PP-003](../../../../plans/postponed/PP-003-option-c-advisor-slice.md).
  D-C3-7 (§4) stays drafted but is OUT of this campaign's build
  scope; it fires only when the owner commissions Stage C.
- **Budget posture & timing:** commissioning is now; Ф0–Ф5 cost
  nothing beyond the standing GLM plan (local floors + already-paid
  delegates). The only paid, budget-matched surface is Ф6's trial,
  which stays gated behind **RP-C3-2**. Left for the owner there:
  trial budget posture and — since RP5 (PP-001) is not yet ruled —
  the MT-C2-05-vs-MT-C3-01 firing order (today MT-C3-01 is the
  first and only trial this campaign fires).

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

_Verdicts recorded at Ф6 (MT-C3-01, 3 gated runs). Normative record:
MT-C3-01 Recorded runs; narrative: `reports/…-f6-trial.md`._

- P-C3-a: the need-gate's window-fit guard alone removes ≥ X% of
  unnecessary descents in the trial (X set with baseline data).
  **→ SUPPORTED** — every window-fitting single-skill task was routed as
  ONE worker call, never over-decomposed into a child tree; the gate was
  consulted 3–10× per run. Not a hard % (no `decisions` read verb yet — a
  trial follow-up); the gate's reason strings are the evidence.
- P-C3-b: schema-validated returns cut malformed-result rework to
  ~zero across trial runs. **→ INCONCLUSIVE** — no boss set an
  `output_schema` on a packet, so the gate never fired. The seam is
  unit-tested (Ф1.2b); the menu did not exercise it. Follow-up: a menu with
  an explicit structured-output task.
- P-C3-c: no trial run exceeds its wall-clock budget (the axis
  nobody else enforces — our differentiator holds under fire).
  **→ CONFIRMED** — zero runs ended `killed(budget)`; no worker's wall
  exceeded its packet `budget.wall_secs` (600 s).
- P-C3-d: ≥1 Silo-regime task in the trial menu escalates rather
  than fans out, and scores better for it. **→ INCONCLUSIVE (leaning
  refuted for this menu)** — zero escalations; the bosses correctly routed
  task 3 (single-doc facts extraction) rather than escalating, because the
  menu offers no genuinely chunk-destroying Silo task. The escalation
  channel is built + tested (Ф4); follow-up: a whole-repo reasoning item.

## 8. Review points {#review-points}

- **RP-C3-1 — mandate & scope cut (RULED 2026-07-11 → Option B):**
  owner verbatim «Вариант 1. Вариант плана - B (нисхождение +
  эскалация). Вариант C с адвайзором - отдельная задача,
  запланируй.» Scope = descent + ascent (V2+V3); the advisor (V4)
  is cut to [PP-003](../../../../plans/postponed/PP-003-option-c-advisor-slice.md).
  Budget posture + timing → §1; paid trial arms remain behind
  RP-C3-2.
- **RP-C3-2 — trial arms authorization (PRE-AUTHORIZED 2026-07-11):**
  owner verbatim «я прямо сейчас разрешаю делать эти платные
  прогоны». Paid Ф6 trial arms are authorized. §10.7 still binds:
  MT-C3-01 pre-registration must be committed BEFORE arms fire; the
  budget posture (arm count, spend cap) is confirmed at Ф6
  commissioning (recommendation: budget-matched arms, GLM cold boss,
  cap per the RP1/RP5 precedent). No second word needed to fire once
  the pre-registration lands.
- Standing: PP-001's RP5 remains independent; firing order of
  MT-C2-05 vs MT-C3-01 is part of RP-C3-1.

## 9. Ledger {#ledger}

Commit map (Stage B execution, Campaign 3):

- Ф0 spikes closed (`c1151bb`) — no product commits (spikes throwaway).
- Ф1.1 `context_from` access-list (`35a378c`).
- Ф1.2a packet `output_schema` field (`d91780d`).
- Ф1.2b output_schema validation at the collect seam (`12b9824`).
- Ф1.3 budget lattice — six axes + wall-clock (`19c33e9`).
- Ф2.1 need-gate decision procedure (`5adcceb`).
- Ф2.2 routing policy — capability-class table (`011ef6c`).
- Ф2.3 profile capability_class (`14f97b8`).
- Ф3.1 spawn depth-guard — D-C3-3 enforcement (`b23f3f1`).
- Ф3.2 gate invocation — `fractality gate` + `can_spawn` fix (`3b0b2d2`).
- Ф3.4a await `--any` (D-C3-4) + run-verb relocation (`a1479f1`).
- Ф3.2b-i decision journal storage — stem + records (`2c0a128`).
- Ф3.2b-ii decision journal producer — `gate --record` → `/v0/decisions`
  + module splits (`8d8960a`). **D-C3-8 COMPLETE** (gate invocation +
  decision journal, end to end).
- Ф3.5a refuse near-duplicate child — `Packet::task_fingerprint` +
  `admission::check_not_duplicate` (`1189b3c`, D-C3-4/5).
- Ф3.3 availability masking — `usable_profiles`/`token_present`
  (`b21a4c6`, FD-8; pure query, router-consumer later).
- Ф3.6 retry-on-violation — sync-loop re-dispatch once (`867afc2`,
  D-C3-2; `run_once`/`retry_report` in `swarm.rs`, no protocol change).
- Ф3.5b merge-node marker + at-most-one invariant (`9825f4e`, D-C3-4/5;
  `output.merge` + `check_sibling_invariants`). **Ф3 COMPLETE — the
  descent core is in: every D-C3 decision landed, floor green, pushed.**
- Ф4.1 escalation core outcome (`e13ddbf`, D-C3-6): terminal
  `RunState::Escalated` + `EscalationRecord{reason, needs}` on RunRecord;
  typed `Event::Escalated` + its fold arm; `MetricsBucket.escalated`
  counter (terminal, never `open`, never `failed`); MC write-path
  validator target so validation and fold stay in lockstep. The replay
  fold carved into a new `journal_fold.rs` cell (600-line budget) with
  `journal::apply`/`ApplyOutcome` re-exported — no caller changes;
  specmap re-minted in-commit. Tested library only — no producer yet
  (Ф4.2 climb / Ф4.3 worker expression next; Ф2 ship-then-wire precedent).

**Scoping decision — escalation edges (D-C3-6, Ф4.1).** `Escalated` is
reachable only from `running` and `waiting_on_boss`: the live states in
which a worker (or the boss acting on a parked run) has actually engaged
the task and can decide it must go up. `queued`/`starting` cannot escalate
— nothing has evaluated the task yet, so "this needs to go up" there is a
gate-time `route`/`escalate` verdict (D-C3-1), not a run outcome. The set
is deliberately minimal (§10.8 conservative): widening to `starting` is a
backward-compatible edge-add reserved for Ф4.3 IF the worker expresses
escalation via a result-status exit that can land before the first
heartbeat. Escalation is TERMINAL (the run is done); the *record* climbs
the `parent` edges, the run does not resume — this is the deliberate
divergence from the D18 park channel it otherwise generalizes.

- Ф4.2 escalation climbs to the top (`6ed04e6`, D-C3-6): the ascent
  surfaces — `escalated` exit code 5 (`state_code`, parent-observable,
  distinct from failed=1, with 4 reserved for parked-past-budget);
  `fractality escalations` inbox (the ascent twin of `questions`) listing
  escalated runs attributed to their call-tree ROOT via a `parent`-edge
  walk (`root_of`: dangling-stop + 64-hop cycle guard);
  `print_run_summary`/`detail` show `escalated:`/`needs:`. No new endpoint
  (`runs(Escalated)` reuses the state filter); the climb is a client-side
  walk. specmap re-minted. Producer still absent — worker expression is
  Ф4.3.
- (refactor) carve the MC pod leg into cells (`2ce35f8`): `pod_register`/
  `pod_heartbeat`/`pod_event` + `mint_file_ref` → `http_pods.rs` (http.rs
  600→379); the client-leg pod verbs → mc-client `pod_leg.rs` (lib.rs
  597→560). Headroom for the escalate route + verb ("carve before adding").
  Pure move, no behaviour change; first CC+z.ai GLM delegation did the
  http.rs half (reviewed).
- Ф4.3a escalate endpoint + client verb (`3f9a2e4`, D-C3-6):
  `POST /v0/runs/:id/escalate` (`http_escalate.rs`) records `Event::
  Escalated` + writes `escalation.md`; wrong-state → 409; `EscalateRequest`
  DTO; `McClient::escalate`. Two integration tests (running-run escalates;
  queued refused). specmap re-minted.
- Ф4.3b escalate MCP tool (`0bf4242`, D-C3-6): the broker serves
  `escalate(reason, needs)` alongside `ask_boss`; records via the Ф4.3a
  path, returns a terminal "stop working" result (worker exit absorbed as a
  kill-tail). `route()` unit tests for the tool list + call routing + arg
  guards. **Ф4 COMPLETE — the escalation channel is in end to end**
  (D-C3-6: core outcome → climb surfaces → endpoint/verb → worker tool;
  floor green throughout, test-gate 211). Report:
  `reports/2026-12-07-05-05-campaign3-f4-escalation.md`.

**Open decision — worker expression, RESOLVED (Ф4.3).** The Ф0-s4 open
question (MCP tool vs result-status exit) is decided for the MCP tool: it
fits the state machine (the worker is `running` when it escalates, so
`running → escalated` is clean and immediate), matches the "generalise the
D18 channel" framing, and keeps reason/needs structured (no string
sentinels, §10.5). A result-status exit would invert the machine — the
pod's Exit event completes the run first, and a terminal run cannot then be
re-marked escalated. If a fast worker ever needs to escalate before its
first heartbeat, that is a `starting → escalated` edge-add + a status field,
deferred until a worker actually needs it.

- Ф5.1 verifier marker + cold-verifier suppression (`85ac2a7`, FD-9):
  `OutputSpec.verifier` marks an acceptance verifier over `context_from`
  work; `admission::check_verifier_has_work` refuses (400) a verifier whose
  `context_from` names no run that produced a result — no cold verification
  over an empty tree (§10.2), applied unconditionally in `register_run`
  (packet-validity, like `validate()`). Two integration tests.
- Ф5.2 verifier-accept surfaced (`af977a4`, FD-9): `RunRecord.verifier`
  denorm (from `output.verifier` at registration); `verifier_verdict`
  renders ACCEPTED/REJECTED/inconclusive/pending in `run`/`show`. **Ф5
  COMPLETE — the acceptance channel: cold-verifier refusal + verifier-accept
  readability** (floor green, test-gate 213). Report:
  `reports/2026-12-07-05-26-campaign3-f5-acceptance.md`.

**Scoping decision — verifier-accept is a readable verdict, not a hard gate
(Ф5.2).** §10.8 conservative: a verifier's acceptance verdict is first-class
on its record and rendered as ACCEPTED/REJECTED; it does NOT suspend the
tree's runs from going terminal (a hard completion gate would be intrusive
and is not what v1 needs). The query "given a work run, find its verifier"
(tree → acceptance) is deferred — it needs a `context_from` denorm on the
record or a scan; the verdict being first-class satisfies the plan's "can
gate completion" reading. Acceptance-feeds-routing (FD-5 soft-label table)
is later work off the D-C3-8 seam.

- Ф6 trial (D-C3-9): MT-C3-01 pre-registered (`3c8ea76`, §10.7 gate) →
  harness `run-arm.sh g` + `preamble-g.md` (`1c4a8f8`) → 3 paid GLM
  cold-boss runs fired → recorded runs + `score-g.py` (`67a3e4a`).
  **Result: gated arm delegated 8/18 ≈ 44.4% vs C2 naive baseline 16.7%
  (~2.7×); fractality ran end to end as a product for the first time**
  (3 workers completed with a worker result, 1 acceptance 1/1). P-C3-c
  CONFIRMED (0 wall-budget overruns), P-C3-a SUPPORTED (route, not
  over-decompose), P-C3-b/d INCONCLUSIVE (menu set no output_schema, had no
  Silo task). **Ф6 COMPLETE.** Report:
  `reports/2026-12-07-06-44-campaign3-f6-trial.md`. Owner sign-off on the
  recorded runs pending.

**Scoping decision — single-arm trial + smoke-test-first (Ф6).** MT-C3-01
fired ONE gated arm (N=3), not a fresh 2-arm A/B: the predictions are
observations of the gated system and MT-C2-01 arm A (16.7%, same menu) is
the naive comparator, halving the spend (RP1/RP5 posture). Run 1 was a
readiness probe — the end-to-end loop (gate→spawn→pod→real GLM worker→
collect→journal) was confirmed live before the rest of the ≤8-session cap
was committed. Trial follow-ups (a future menu): raise worker turn caps
(30 bit hard), add a structured-output task (exercise P-C3-b) + a genuine
Silo task (exercise P-C3-d), add a `fractality decisions` read verb (make
P-C3-a a hard number). All deferred — Stage B built the machinery and
proved it runs.

**Scoping decision — decision-journal producer (D-C3-8).** The journal
must record REAL need-gate decisions, so the producer cannot be MC
re-deriving one at `register_run` — MC lacks the boss's task-shape
signals (`GateInputs`), and a synthesized record would be a false entry.
The producer is therefore the `fractality gate --record` path (it holds
the real signals): the gate verb, when a daemon is reachable, POSTs its
`DecisionRecord` to a new `/v0/decisions` endpoint. That makes `gate`
async + daemon-aware (today it is pure/offline like `route`), so it lands
as its own sub-slice (Ф3.2b-ii) rather than being rushed onto the storage
layer. Until then nothing writes decisions at runtime; the stem +
`record_decision`/`decisions` (`2c0a128`) are exercised only by tests
(the Ф2 precedent: ship the tested library, then its caller).

**Scoping decision — retry-on-violation (D-C3-2).** The validation seam
produces the retry-feedback report (Ф1.2b), but the automatic one-retry
is NOT pod-local: a re-invoke loop inside the pod would rewrite its
lifecycle, which §10.5 forbids ("extension at named seams"). The retry
is re-dispatch at the orchestration layer — the need-gate re-spawns the
task once with the violation report in the child's context — landing
with the descent verbs (Ф3). Until then a schema-violating result is
recorded (`status.json` `schema_gate.valid=false` + violations) for the
boss to act on. Revisit trigger: Ф3 spawn orchestration.

**Scoping decision — D-C3-3 boundary behaviors → Ф2.** D-C3-3's
per-verb cap behaviors (spawn-past-cap → structured refusal, child
profile at cap → capability removal + leaf surface, work packet at cap
→ force-execute) enforce against the need-gate's verbs and the budget
caps, neither of which exists until Ф2 (D-C3-1 + delegation-rules).
Shipping profile config for them in Ф1 without the gate would be dead
surface (the output_schema lesson). D-C3-3 therefore lands in Ф2
alongside the need-gate and the depth guard, checking against the
budget lattice (Ф1.3) it now has. Ф1 closes having shipped D-C3-2 —
the packet + budget surface (`context_from`, `output_schema`, the
six-axis lattice).

**Scoping decision — need-gate WIRING → Ф3.** Ф2 shipped the gate's
decision machinery: the procedure (`needgate::decide`, §10.3), the
capability-class routing policy (data in delegation-rules + the compiled
default), and profile-declares-class — with goldens (§6). The gate's
INVOCATION (a `fractality gate` CLI surface + journaling the decision
tuple, D-C3-8), its ENFORCEMENT (admission's spawn-past-cap depth guard —
the D-C3-3 boundary), and availability masking (FD-8) land in Ф3, where
the spawn/route verbs actually USE the gate. The gate is a pure, tested
library now; Ф3 gives it a caller and teeth.

**Ф3.1 landed — the depth guard (D-C3-3 enforcement).** `register_run`
refuses a spawn nesting past the routing policy's per-class cap
(`check_spawn_depth`, `b23f3f1`) — the enforcement the WIRING note above
promised, at the door, before any pod. The gate's INVOCATION
(`fractality gate` + decision journal, D-C3-8) and availability masking
(FD-8) remain for the next slices, plus the descent verbs (D-C3-4/5).

**Finding — `max_depth = 0` is overloaded (surfaced by Ф3.1).**
`routing::ClassPolicy.max_depth = 0` means *no spawning* (the weak-class
default), but `needgate::GateInputs.max_depth = 0` means *unlimited*
(`decide`'s arm 4: `at_cap = max_depth != 0 && depth >= max_depth`).
Feeding a weak class's policy cap (`0`) straight into `GateInputs` would
make `decide` spawn without bound — the opposite of intent. The Ф3.1
enforcement reads the routing semantics (cap `0` ⇒ refuse), so the tree
stays bounded regardless of the advisory. The gate-invocation slice
(D-C3-8) must translate a class's policy cap into `GateInputs` so a
no-spawn class never reaches the spawn arm (e.g. gate on a `can_spawn`
signal derived from `cap > 0`) rather than passing `0` through as
"unlimited". **RESOLVED in Ф3.2 (`3b0b2d2`):** `GateInputs` gained a
`can_spawn` field (the gate CLI derives it from `cap > 0`); `decide`'s
spawn arm gates on it, so a no-spawn class folds instead of spawning.
`max_depth = 0` keeps its "unlimited" meaning only when `can_spawn` is
true, so the existing pure-procedure semantics and every prior spawn test
stand unchanged.

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
