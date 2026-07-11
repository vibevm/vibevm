# fractality — WAL (project continuation state)

_Updated: 2026-07-12 ~01:25 (**Campaign 3 Stage B — Ф3 IN PROGRESS: the
WHOLE gate wiring is in**). Ф0/Ф1/Ф2 CLOSED (the need-gate decision
core). Ф3 so far, each floor-green + committed + ledgered + pushed:
**Ф3.1** spawn depth-guard — D-C3-3 enforcement (`b23f3f1`); **Ф3.2**
gate invocation + decision journal — **D-C3-8 COMPLETE**: `fractality
gate` surfaces `decide` (`3b0b2d2`, also resolving the `max_depth=0`
overload via `GateInputs.can_spawn`), and `gate --record` journals the
`DecisionRecord` to a `/v0/decisions` stem end-to-end (`2c0a128` storage
+ `8d8960a` producer); **Ф3.4a** `fractality wait --any` descent
await-any race (`a1479f1`; `all`/`named` already existed); **Ф3.5a**
refuse near-duplicate child — `Packet::task_fingerprint` +
`admission::check_not_duplicate` (`1189b3c`, full-spec match so fan-out
passes). **Remaining Ф3:** Ф3.5b merge node (design-laden) + the
sibling-isolation pinning test, masking (FD-8, maybe defer), retry (3.6).
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

## Next — Ф3 (descent verbs + gate wiring) — IN PROGRESS

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker (carries the seam map + per-slice status) →
plan §10 (BINDING) + §9 (ledger + deferrals). **Done this session:**
Ф3.1 depth-guard (`b23f3f1`), Ф3.2a gate invocation + `can_spawn`
overload fix (`3b0b2d2`) — both floor-green. Remaining Ф3:

1. **Descent SEMANTICS (D-C3-4/5) — the hardest remaining slice:**
   sibling isolation (already true by construction — a child sees only
   its packet + `context_from` results, never a sibling transcript; wants
   a PINNING TEST, not new code); a designated **merge node** answering
   the parent goal; **MC refuses near-duplicate child specs** — NB needs
   a FULL-spec match (title+goal+context), NOT title-only (a fan-out
   legitimately spawns same-title children on different chunks; title-only
   would break the core idiom) → likely a task fingerprint on RunRecord
   (new field + journal event). `await any|all|named` in
   mc-client + CLI — **NB `fractality wait` already blocks on all ids
   (`swarm::wait`); extend it** with any/named; parallel siblings the
   default idiom; **sibling isolation by default** (visibility only via
   `context_from`); a designated **merge node**; MC refuses near-
   duplicate child specs; single-writer.
3. **Availability masking (FD-8):** route over usable profiles (token
   present). **NB dead-surface risk:** today packets name their profile
   and `preflight` already checks the token exists; masking needs a
   multi-profile *router* seam to consume it — build that consumer first
   or defer.
4. **retry-on-violation re-dispatch** (deferred from Ф1.2b §9).

**Still to read** (the deferred delegate failed — silent stall):
`mc-client/lib.rs`, cli `swarm.rs`/`mc_cmd.rs`/`broker.rs`, mc
`registry.rs`. Core + `admission.rs`/`http.rs`/`state.rs`/`journal.rs`
read this session.

Each slice = one commit, floor green after each; specmap re-mint
in-commit on drift. **Floor/test runs = backgrounded cargo, NO stdout
redirect** (the harness captures the task output file; a `> log` steals
it — lesson this session). opencode delegation stays unreliable this
session (read-inventory attempt stalled silently).

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
