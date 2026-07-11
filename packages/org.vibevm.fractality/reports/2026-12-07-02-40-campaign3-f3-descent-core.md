# Campaign 3 · Ф3 — the descent core (COMPLETE)

_Owner-facing phase report. 2026-07-12. One long session, resumed from
the Ф2 checkpoint. Ф3 closed end to end: every D-C3 decision landed,
floor green after each slice, all pushed to both remotes._

## TL;DR

Ф3 turned the Ф2 need-gate from a pure, uncalled library into a wired
descent core. Nine floor-green slices this session (+ one real bug fix):
the gate now has **enforcement** (depth-guard), a **caller** (`fractality
gate`), a **decision journal** (`gate --record` → `/v0/decisions`), and
the descent **safety mechanisms** (await-any race, refuse-near-duplicate,
availability masking, retry-on-violation, merge-node designation). The
`~/.fractality` boundary was never touched; every change is an extension
at a named seam.

## What was done (the commit chain)

| Slice | Decision | Commit |
|---|---|---|
| Ф3.1 | depth-guard — spawn-past-cap refusal at the door | `b23f3f1` |
| Ф3.2a | `fractality gate` — the need-gate as a verb | `3b0b2d2` |
| Ф3.2b-i | decision-journal storage (records + sibling stem) | `2c0a128` |
| Ф3.2b-ii | decision-journal producer (`gate --record` → bus) | `8d8960a` |
| Ф3.4a | `fractality wait --any` — descent await-any race | `a1479f1` |
| Ф3.5a | refuse near-duplicate child (`task_fingerprint`) | `1189b3c` |
| Ф3.3 | availability masking (FD-8) | `b21a4c6` |
| Ф3.6 | retry-on-violation re-dispatch | `867afc2` |
| Ф3.5b | merge-node marker + at-most-one invariant | `9825f4e` |

Each feat has a paired `docs(fractality)` ledger commit; the WAL carries
two mid-session checkpoints. ~24 commits total, all on `main`, pushed via
`cargo xtask mirror` to GitVerse + GitHub.

## The decisions taken (the main thing)

1. **The `max_depth = 0` overload — found and fixed.** `routing.rs`
   reads a class cap of `0` as "no spawning" (weak); `needgate::decide`
   reads `max_depth = 0` as "unlimited". Feeding a weak class's policy cap
   straight into `GateInputs` would have spawned WITHOUT BOUND — the exact
   opposite of intent. Fix: `GateInputs.can_spawn` (derived `cap > 0`)
   gates the spawn arm, so a no-spawn class folds instead of reaching the
   ambiguous `max_depth`. `decide`'s pure semantics and its golden stand
   unchanged. This was the session's one genuine latent bug.

2. **Depth-guard is enforcement, independent of the gate's advice.** The
   need-gate *recommends* folding at the cap; admission *refuses* a spawn
   that ignored it. Defense in depth: a caller that bypasses the gate
   still cannot open an unbounded tree.

3. **The decision journal rides its own stem, not the run fold.** Every
   run-journal event folds into a `RunRecord` keyed by `run_id`, but an
   inline/escalate verdict has no run. So decisions append to a `decisions`
   sibling stem (the session-journal pattern) — the soft-label table
   (D-C3-8) is a replay-and-aggregate over it. The producer is `gate
   --record` (it holds the real task-shape signals); MC cannot synthesize
   a decision at spawn time (it lacks them), so a synthesized record would
   be a false entry — that ruling is in §9.

4. **Refuse-near-duplicate keys on FULL-spec identity, not title.** This
   is load-bearing: a legitimate fan-out spawns many same-titled children
   over different chunks, so a title-only match would break the core
   idiom. `Packet::task_fingerprint` hashes task + inputs + workspace, NOT
   execution params (routing/budget/output branch), so same-task/different-
   model still collides while fan-out passes.

5. **Retry-on-violation is a CLI sync-loop concern, no protocol change.**
   Verified the seam: the schema-gate verdict never reaches MC (`Collected`
   carries only result + acceptance; the pod writes `schema_gate` to
   `status.json` locally). Rather than a pod→MC protocol change, `fractality
   run` reads `status.json` and re-dispatches ONCE with the violations in
   the retry's `context.notes`. Bounded (checked only on the first attempt).
   `fractality spawn` (fire-and-forget) has no wait point and so no retry —
   correct.

6. **Merge-node is a conservative v1 (§10.8).** `output.merge` marks the
   designated answer child; MC enforces at most one per parent. The
   await/collect integration that makes a parent's own result BE its merge
   node's is flagged `REVIEW` on the field as a follow-up — the marker +
   invariant are the minimal non-dead designation.

7. **Availability masking ships tested ahead of its consumer** — the Ф2
   precedent (the need-gate itself shipped uncalled). `usable_profiles`
   filters by token presence; the multi-profile router that consumes it is
   a later phase.

## What is left undone (non-blocking follow-ups — NOT D-C3 decisions)

- **Merge-node await/collect integration** — a parent's result BEING its
  merge node's. The marker + invariant are in; the wiring is future.
- **Mid-task profile alternation** (Fugu's per-step worker rotation) — a
  pod/worker feature, a later phase, not an Ф3 decision.
- **A sibling-isolation pinning test** — isolation is true BY
  CONSTRUCTION (a child sees only its packet + `context_from`; there is no
  field for a sibling transcript — the fold law). A test would document,
  not enforce; optional.

None of these gates Ф4.

## Unfixed bugs / honest caveats (висяки)

- **Sibling checks are best-effort under concurrency.** The depth-guard,
  refuse-duplicate, and merge invariant read state then record without
  atomicity — two simultaneous identical spawns could both slip past.
  Acceptable v1 (rare; the tree stays bounded by other means); noted in
  each function. A single-writer admission lock would close it.
- **`usable_profiles` / dup / merge re-read sibling `packet.toml` per
  call** (O(siblings) I/O). Fine for small trees; a fingerprint/flag on
  `RunRecord` would make it O(1) — deferred to avoid the RunRecord blast
  radius mid-session.
- **The retry's own result is not re-validated** — one retry, then its
  result stands whatever its schema verdict (still recorded on the plane).

## Global / strategic notes (косяки & friction)

- **The 600-line conform cell budget bit THREE times** this session
  (main.rs, http.rs ×2, mc-client/lib.rs). Each was resolved by a clean
  module-grain split (`run_packet`→swarm, `http_decisions.rs`, mc-client
  `decisions.rs`, dup-logic→admission), which is the discipline working as
  intended — but http.rs now sits at exactly 599/600 and mc-client at
  597/600. **The next MC/http slice will need a split up front.** Worth a
  deliberate pass to carve http.rs's handlers into `http_*` cells before
  it forces another mid-slice detour.
- **Two machine lessons, now in the WAL:** (1) backgrounded cargo must NOT
  redirect stdout (`> log` steals the harness's own capture — the output
  vanishes); read the harness task file instead. (2) the shell cwd is NOT
  reliably persisted between Bash calls — ALWAYS `cd <v0.1.0>` explicitly
  for cargo/floor/specmap, or they run against the host workspace and fail
  confusingly.
- **opencode/GLM delegation stayed unreliable** — the one attempt (a
  12-file seam inventory) stalled silently (0-byte log, exit 0), matching
  prior-session field data. Seam reconnaissance ran boss-side; it is a
  legitimate boss-keep (it anchors phase design), but the delegation law's
  scoreboard records the miss. When opencode stabilizes, the mechanical
  seam-inventory read is the first thing to hand back.
- **The decision journal has no QUERY surface yet** — `GET /v0/decisions`
  returns the raw log, but the soft-label aggregation (per worker-class ×
  task-shape) is not built. That is Phase-5 delegation-rules work; the raw
  rows are now being recorded, which is the prerequisite.

## Next

**Ф4 escalation (D-C3-6).** The Ф0 s4 spike already resolved the design: a
terminal `RunState::Escalated` + `EscalationRecord{reason, needs}` climbing
the `parent` edges to the human, generalizing the D18 park channel. Open
question: worker expresses escalation via an ask_boss-style MCP tool vs a
result status. Read the D18 machinery first (`http_questions.rs`, the
AnswerRule fold).

The source of truth is the plan/WAL/§9 ledger; this report is the
narrative.
