# fractality — cold-resume checkpoint

_Written 2026-07-12 ~02:45 at session save. `WAL.md` (same directory) is
the canonical living state and supersedes this snapshot wherever they
diverge. Resume with `восстанови сессию fractality` (report-then-wait)._

## TL;DR

**Campaign 3 · Stage B: Ф0–Ф3 COMPLETE — the descent core is in.** This
session resumed from the Ф2 checkpoint and drove the WHOLE of Ф3 end to
end: the Ф2 need-gate went from a pure, uncalled library to a wired
descent core across **9 floor-green slices** (+ one real bug fix). The
gate now has enforcement (depth-guard), a caller (`fractality gate`), a
decision journal (`gate --record` → `/v0/decisions`), and the descent
safety mechanisms (await-any, refuse-near-duplicate, availability masking,
retry-on-violation, merge-node designation). ~26 commits, all on `main`,
pushed to both remotes. Floor green (test-gate 203 / conform 0 / specmap
clean). `~/.fractality` never touched.

The `/goal` (“план должен быть выполнен до конца”) was **cleared** by the
owner at save time. **No active blocker.** The candidate next step is
**Ф4 escalation (D-C3-6)** — but resume is report-then-wait; the owner
steers.

## Where work stands

- Branch `main`, **in sync with origin** (0 ahead), working tree clean.
- Both remotes (GitVerse `origin` + GitHub `github`) at `9dd642b`.
- Floor last run green end to end: test-gate 203 passed, conform 0
  findings, specmap clean, clippy/fmt clean.

## Ф3 — what landed (the commit map)

| Decision | What | Commit |
|---|---|---|
| D-C3-3 | depth-guard — spawn-past-cap refusal at the door | `b23f3f1` |
| D-C3-8 | `fractality gate` invocation | `3b0b2d2` |
| D-C3-8 | decision-journal storage (records + `decisions` stem) | `2c0a128` |
| D-C3-8 | decision-journal producer (`gate --record` → bus) | `8d8960a` |
| D-C3-4 | `fractality wait --any` — descent await-any race | `a1479f1` |
| D-C3-4/5 | refuse near-duplicate child (`task_fingerprint`) | `1189b3c` |
| FD-8 | availability masking (`usable_profiles`) | `b21a4c6` |
| D-C3-2 | retry-on-violation re-dispatch (sync loop) | `867afc2` |
| D-C3-4/5 | merge-node marker + at-most-one invariant | `9825f4e` |

Full narrative: `reports/2026-12-07-02-40-campaign3-f3-descent-core.md`
(decisions, follow-ups, caveats, strategy). Per-slice status +
delegation scoreboard: `reports/2026-11-07-17-52-rlmplan-state-plan.md`.

## Next-steps recipe (cold start) — Ф4 escalation (D-C3-6)

1. Boot: workspace `CLAUDE.md` → this file → `WAL.md` → the state-plan
   tracker → plan `FRACTALITY-RLM-PLAN-v0.1.md` §10 (BINDING) + §9
   (ledger). The plan §10.7 process laws bind every slice.
2. **The design is already resolved (Ф0 spike s4):** a terminal
   `RunState::Escalated` + an `EscalationRecord{reason, needs}` on
   `RunRecord`, climbing the existing `parent` edges to the human at the
   top — generalizing the D18 question/answer park channel + AnswerRule.
   No new daemon.
3. **Open design question (s4):** a worker expresses escalation via an
   `ask_boss`-style MCP tool vs a result status. Decide conservatively
   (§10.8) or ask the owner.
4. **First slice (suggested):** the core outcome — `RunState::Escalated`
   in `run.rs` (enum + `can_transition_to` edges), `EscalationRecord` on
   `RunRecord`, and an `Event::Escalated` in `journal.rs` with its fold in
   `apply()`. Mirror the `Question`/`Answer` park-channel pattern. One
   D-C3 = one commit, floor green after each; specmap re-mint on drift.
5. **Read for Ф4 (not yet read this session):** `http_questions.rs` (the
   D18 HTTP park channel), the `AnswerRule` fold in `profile.rs`/
   `state.rs`, the pod escalation path, `registry.rs`.

**Ф3 follow-ups (non-blocking, NOT D-C3 decisions):** merge-node
await/collect integration (the marker + invariant are in; making a
parent's result BE its merge node's is future — flagged `REVIEW` on the
`output.merge` field); mid-task profile alternation (a pod/worker
feature); a sibling-isolation pinning test (isolation is true by
construction — the fold law).

## Non-obvious findings this session (do not rediscover)

- **`max_depth = 0` was overloaded** — `routing.rs` reads a class cap of 0
  as "no spawning" (weak), `needgate::decide` reads it as "unlimited".
  Fixed via `GateInputs.can_spawn` (derived `cap > 0`). If you touch the
  gate, keep the two readings straight.
- **The schema-gate verdict does NOT reach MC** — `core::run::Collected`
  and `PodEvent::Collected` carry only result + acceptance; the pod writes
  `schema_gate` to `status.json` locally (Ф1.2b). Retry-on-violation
  therefore reads `status.json` in the CLI sync loop rather than changing
  the pod→MC protocol. If Ф4+ needs the schema verdict server-side, plumb
  it through `Collected` first.
- **cwd is NOT reliably persisted between Bash calls** — ALWAYS
  `cd <…/fractality/v0.1.0>` explicitly for `cargo` / the floor binary /
  `specmap`, or they run against the host workspace (which excludes
  `packages/`) and fail confusingly.
- **Backgrounded cargo must NOT redirect stdout** — a `> log 2>&1` steals
  the harness's own task-output capture (the file ends up empty); read the
  harness task file instead. `| tail`/`| head` on a live pipe also buffers.
- **The 600-line conform cell budget bit three times** — `main.rs`,
  `http.rs` (×2), `mc-client/lib.rs`. All resolved by module-grain splits.
  **`http.rs` now sits at 599/600 and `mc-client/lib.rs` at 597/600** —
  the next MC/http slice will need a split UP FRONT (carve http handlers
  into `http_*` cells before adding).
- **Adding a field to a Debug-snapshotted core type** (e.g. `OutputSpec`)
  changes the `hello_glm` insta snapshot regardless of
  `skip_serializing_if` — update `crates/fractality-core/src/snapshots/
  fractality_core__packet__tests__hello_glm_*.snap` and delete the
  `.snap.new`.
- **opencode/GLM delegation stayed unreliable** — the one attempt (a
  12-file seam inventory) stalled silently (0-byte log, exit 0). Seam
  reconnaissance ran boss-side. When it stabilizes, hand back the
  mechanical seam-inventory reads first.

## Honest caveats (висяки, all noted in code + the report)

- Sibling checks (depth-guard, refuse-dup, merge) are **best-effort under
  concurrency** — read-then-record is not atomic; two simultaneous
  identical spawns could slip past. A single-writer admission lock closes
  it. Acceptable v1.
- Those checks **re-read sibling `packet.toml` per call** (O(siblings)
  I/O). A fingerprint/flag on `RunRecord` would make it O(1) — deferred to
  avoid the RunRecord blast radius.
- The retry's own result is not re-validated (one retry, then it stands).
- The decision journal has **no aggregation/query surface** yet
  (`GET /v0/decisions` returns raw rows); the soft-label table is Phase-5.

## Repository map (workspace)

`packages/org.vibevm.fractality/` — `CLAUDE.md` (contract), `WAL.md`
(canonical state), this file, `VIBEVM-BACKLOG.md`, `WORKSPACES.md` row in
the host; **`plans/`** (postponed.md + PP-001/002/003); **`reports/`**
(IGNITION, C2, rlm/fugu research dashboards, the state-plan tracker, the
Ф1/Ф2/**Ф3** phase reports). `fractality/v0.1.0/` — the Cargo workspace:
`crates/{core, mission-control, pod, mc-client, backend-claude-code, cli,
initiative}`; `spec/` (PROP-001, VISION, plans/**RLM-PLAN v0.1**,
manual-tests, refs/ notes); `delegation-rules/v0.1.0/` — the policy
package (`routing-policy.toml`). Ф3 touched: core `packet.rs`/
`needgate.rs`/`api.rs`/`lib.rs`; mc `admission.rs`/`http.rs`/`state.rs` +
new `http_decisions.rs`; mc-client `lib.rs` + new `decisions.rs`; cli
`main.rs`/`gate_cmd.rs`(new)/`swarm.rs`.

## Decisions / policy in force (long form)

- Host Rules 1–4; **plan §10 executor guide is BINDING**; clean-room
  §10.4 (never open `refs/src|papers|articles` while coding); the
  delegation law + live-observation; no Python in shipped code; commit via
  `git commit -F - <<'MSG'` heredoc; editor-tool edits only (PS 5.1
  corrupts UTF-8-no-BOM); scratch homes; no `*install*` test binaries;
  F15 (stop MC before builds); domain code has no `unwrap`/`expect`
  (conform); specmap re-mint in-commit on drift.
- **Ф6 paid trial arms — RP-C3-2 PRE-AUTHORIZED 2026-07-11**; arms fire
  only after MT-C3-01 pre-registration is committed (§10.7 pre-reg-first
  binds).
- Floor is the gate panel run FROM `fractality/v0.1.0/`: fmt → test →
  clippy → conform → specmap → test-gate; green at every phase boundary.

## Recent commit chain (last ~24, newest first)

```
9dd642b docs(workspaces): fractality Ф3 complete — descent core
9192528 docs(fractality): Ф3 COMPLETE — descent core, phase report
9825f4e feat(fractality): Ф3.5b merge-node marker + invariant (D-C3-4/5)
d745a09 docs(fractality): Ф3.6 ledger — retry-on-violation
867afc2 feat(fractality): Ф3.6 retry-on-violation re-dispatch (D-C3-2)
4172827 docs(fractality): Ф3.3 ledger — availability masking
b21a4c6 feat(fractality): Ф3.3 availability masking (FD-8)
4d0f565 docs(fractality): Ф3.6 retry — record the schema-result seam finding
e072909 docs(fractality): Ф3.5a ledger — refuse near-duplicate child
1189b3c feat(fractality): Ф3.5a refuse near-duplicate child (D-C3-4/5)
94eae73 docs(fractality): D-C3-8 complete — ledger, tracker, WAL
8d8960a feat(fractality): Ф3.2b-ii decision journal producer (D-C3-8)
4a97b22 docs(fractality): Ф3.2b-i ledger — decision journal storage
2c0a128 feat(fractality): Ф3.2b-i decision journal storage (D-C3-8)
2020b57 docs(wal): Ф3.4a — await --any in the mid-session checkpoint
0e43257 docs(fractality): Ф3.4a ledger — await --any
a1479f1 feat(fractality): Ф3.4a await --any (D-C3-4)
800f29c docs(wal): Ф3 mid-session checkpoint — 3.1 + 3.2a landed
8fbb10c docs(fractality): Ф3.2a ledger — gate invocation + overload resolved
3b0b2d2 feat(fractality): Ф3.2 gate invocation — fractality gate (D-C3-8)
0441206 docs(fractality): Ф3.1 ledger — depth-guard + max_depth overload
b23f3f1 feat(fractality): Ф3.1 spawn depth-guard (D-C3-3)
71c749c docs(fractality): Ф2 checkpoint — WAL + host status
77351e7 docs(fractality): Ф2 closed — need-gate machinery, wiring → Ф3
```

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -20 WAL.md
cd fractality/v0.1.0
# floor (ALWAYS from v0.1.0; explicit cd):
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
# tests (backgrounded, NO stdout redirect — read the harness task file)
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
