# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~02:40 (**Campaign 3 Stage B — Ф3 COMPLETE: the
descent core is in**). Ф0/Ф1/Ф2 CLOSED (the need-gate decision core); Ф3
CLOSED this session across 9 slices, each floor-green + committed +
ledgered + pushed. The whole gate wiring AND descent semantics landed:
**Ф3.1** depth-guard (D-C3-3, `b23f3f1`); **Ф3.2** gate invocation +
decision journal (D-C3-8 end-to-end: `3b0b2d2` gate + `2c0a128`/`8d8960a`
journal; also fixed the `max_depth=0` overload via `GateInputs.can_spawn`);
**Ф3.4a** await `--any` race (`a1479f1`); **Ф3.5a** refuse-near-duplicate
(`1189b3c`); **Ф3.3** availability masking (FD-8, `b21a4c6`); **Ф3.6**
retry-on-violation (D-C3-2, `867afc2`); **Ф3.5b** merge-node marker +
at-most-one invariant (`9825f4e`). **Next phase: Ф4 escalation (D-C3-6).**
Phase report: `reports/2026-12-07-02-40-campaign3-f3-descent-core.md`.
Per-slice status + design notes in the state-plan tracker._

## Current state

- **Stage B COMMISSIONED — Option B** (RP-C3-1, plan §1/§8). Advisor
  (Option C/V4) postponed → PP-003.
- **Goal (owner, standing):** the WHOLE Stage B plan (Ф0→Ф7), a working
  RLM with all patterns. **70%-context rule:** at ~70% consumed,
  checkpoint + ask restart.
- **Ф0 spikes CLOSED** — all seams green; s1 confirmed **jsonschema
  0.47.0 on rustc 1.93.1** (validate + `at <ptr>: <msg>` shape).
- **Ф1 CLOSED on D-C3-2** — the packet + budget surface:
  `ContextSpec.context_from` access-list (`35a378c`); `output_schema`
  field (`d91780d`) + collect-seam validation (`12b9824`, verdict →
  `status.json schema_gate`); six-axis budget lattice (`19c33e9`,
  RD-4). D-C3-3 (boundary behaviors) deferred to Ф2/Ф3; retry-on-
  violation deferred to Ф3 re-dispatch (§9). Report:
  `reports/2026-11-07-18-54-campaign3-f1-packets-budgets.md`.
- **Ф2 CLOSED — the need-gate machinery** (D-C3-1 + D-C3-10):
  `needgate::decide` (`5adcceb`, the §10.3 procedure: inline | route |
  fold-local | spawn | escalate + journaled reason); `RoutingPolicy`
  capability-class table (`011ef6c`, data in delegation-rules
  `routing-policy.toml` + compiled default); `profile.capability_class`
  (`14f97b8`). Goldens present. The gate is a **pure, tested library —
  no caller yet** (wiring → Ф3). Report:
  `reports/2026-11-07-19-17-campaign3-f2-needgate.md`.
- **Floor green:** 184 tests / conform 0 / specmap clean. Real
  `~/.fractality` untouched. No product-code from the boundary — all
  extensions at named seams.
- **Live tracker:** `reports/2026-11-07-17-52-rlmplan-state-plan.md`
  (goal, seam reconnaissance — read it, the crates need not be
  re-read; per-slice status; delegation scoreboard). §9 ledger in the
  plan is the commit map + scoping decisions.

## Next — Ф4 (escalation, D-C3-6)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker → plan §10 (BINDING) + §9 (ledger). **Ф3 is
CLOSED** (descent core complete). Next is **Ф4 escalation (D-C3-6):**

- `escalated(reason, needs)` as a first-class OUTCOME (not a failure)
  that climbs the `parent` edges to the human at the top — generalizing
  the D18 question/answer park channel + AnswerRule.
- **Ф0 spike s4 already resolved the DESIGN:** a terminal
  `RunState::Escalated` + an `EscalationRecord{reason, needs}` on
  RunRecord, climbing via existing `parent` edges. No new daemon. Open Q
  (s4): a worker expresses escalation via an ask_boss-style MCP tool vs a
  result status.
- **Still to read for Ф4:** the D18 machinery (`http_questions.rs`, the
  AnswerRule fold in `profile.rs`/`state.rs`), pod escalation path,
  `registry.rs`.

**Ф3 follow-ups (non-blocking, NOT D-C3 decisions — do not gate Ф4):**
merge-node await/collect integration (marker + invariant are in; making a
parent's result BE its merge node's is future); mid-task profile
alternation (a pod/worker feature); a sibling-isolation pinning test
(isolation is true by construction — a child sees only its packet +
`context_from`, the fold law).

Each slice = one commit, floor green after each; specmap re-mint
in-commit on drift. **Floor/test/specmap runs = backgrounded cargo with
an EXPLICIT `cd <v0.1.0>`, NO stdout redirect** (the harness captures the
task output file; a `> log` steals it; the shell cwd is not reliably
persisted — both lessons this session). opencode delegation stayed
unreliable (read-inventory attempt stalled silently).

## Constraints (do not violate without discussion)

- Host Rules 1–4; **plan §10 executor guide is BINDING**; clean-room
  §10.4 (never open refs/src|papers|articles while coding); delegation
  law + live-observation (first-output ≤3 min); I1–I7; no Python in
  shipped code; cwd law; commit heredoc; editor-tool edits; specmap
  re-mint law; scratch homes; no `*install*` test binaries; F15 (stop
  MC before builds); domain code has no `unwrap`/`expect` (conform).
- **Ф6 paid trial arms — RP-C3-2 PRE-AUTHORIZED 2026-07-11** (owner:
  «я прямо сейчас разрешаю делать эти платные прогоны»). Arms fire
  after MT-C3-01 pre-registration is committed (§10.7 pre-reg-first
  still binds); budget posture confirmed at Ф6. MT-C2-05 stays
  RP5-gated (unruled) — MT-C3-01 is this campaign's first trial.
- Fugu benchmark numbers are Sakana-reported — mechanism evidence only.

## Delegation scoreboard (session)

Delegated 1 (attempted): s1 schema spike → opencode/GLM. FAILED twice
(external_directory reject on a nested cargo project, then a silent
launch stall). Killed per the live-observation law; s1 + all Ф1/Ф2 code
done boss-side. **Field data (Phase-5):** opencode is unreliable for
cargo spikes on this box today; floor/test go through backgrounded
cargo (reliable notification). Kept (boss): every slice — discipline-
bound seam work is a legitimate boss-keep, and the delegate proved
unreliable. When it stabilizes, mechanical edits + test triage are the
first things to hand back.
