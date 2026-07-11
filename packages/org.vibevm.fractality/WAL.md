# fractality — WAL (project continuation state)

_Updated: 2026-07-11 ~19:20 (**Campaign 3 Stage B — Ф0, Ф1, Ф2
COMPLETE; the need-gate decision core is in**). One long session:
RP-C3-1 ruled Option B, Ф0 spikes closed, Ф1 (D-C3-2 packet + budget
surface) landed, Ф2 (D-C3-1 need-gate + D-C3-10 routing policy)
landed — all floor-green. Next is Ф3 (descent verbs + the gate
wiring). ~23 commits, pushed to both remotes._

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

## Next — Ф3 (descent verbs + gate wiring)

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md`
→ the state-plan tracker (carries the seam map) → plan §10 (BINDING) +
§9 (ledger + deferrals). Then Ф3:

1. **Descent verbs (D-C3-4, D-C3-5):** `await any|all|named` in
   mc-client + CLI; parallel siblings the default idiom; mid-task
   profile alternation; **sibling isolation by default** (visibility
   only via `context_from`); a designated **merge node** answering the
   parent goal; MC refuses near-duplicate child specs; single-writer.
2. **The deferred gate wiring (§9):** a `fractality gate` invocation
   surface + journal the decision tuple (D-C3-8); admission's
   spawn-past-cap **depth-guard enforcement** (D-C3-3, using
   `budget.max_depth` + the routing policy); **availability masking**
   (FD-8, route over usable profiles); retry-on-violation re-dispatch.
3. **To read for Ф3** (not yet read): mc `admission.rs`, `http.rs`,
   `registry.rs`, `journal_store.rs`, `state.rs`; `mc-client/lib.rs`;
   cli `mc_cmd.rs`, `swarm.rs`, `broker.rs`. Core already mapped.

Each slice = one commit, floor green after each; specmap re-mint
in-commit on drift. **Floor/test runs = backgrounded cargo** (opencode
unreliable this session).

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
