# fractality — WAL (project continuation state)

_Updated: 2026-07-10 ~14:10 (**Campaign 2 EXECUTING, paused at the Ф6
boundary** — owner: «сохранить и перезапустить сессию, дойди до
стабильной точки и остановись». Ф0–Ф5 executed and ledgered in one
session; the Ф6 trial is fully pre-registered and committed; the paid
arms have NOT fired — they are the next session's first move. Plan:
[`fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
(§14 ledger per phase); owner dashboards in
[`reports/`](reports/): started/state/paused plan files + per-phase
narratives Ф0–Ф5)._
_Prior: 2026-07-10 (IGNITION **CLOSED** — Phases 0–6, MT-01…05 signed
off, P1–P6/P8 confirmed, P7 falsified; reports in `reports/`)._

## Current state

- **Campaign 2 (initiative system): Ф0–Ф5 landed, floor green at
  every boundary.** Live surface added this campaign: boss sessions
  in MC (sibling `sessions.jsonl`, idempotent begin, initiative
  counters), run attribution via `FRACTALITY_BOSS_SESSION`
  (CLAUDE_ENV_FILE seam; I1-pinned out of workers), the pure
  `fractality-initiative` engine (scoreboard render, nudge policy,
  route calculus — strictly factual, D7), CLI verbs `session ·
  scoreboard · route · hook · statusline · harness · fetch`, the CC
  adapter (availability law: hooks exit 0 on any failure;
  connect-only, never autostart), threshold nudges + cooldown,
  stop-time parked-question alerts (once per question, folded acks),
  profile answer_rules (auto-answers with journaled provenance),
  matrix-as-data with 10/10 §worked goldens (P5 ✅), monthly quota
  rollup in `stats` (IGNITION leftover closed).
- **Ф6 pre-registered and frozen** (committed BEFORE any run):
  staging crate `spec/manual-tests/trial/staging/` (mini_logfmt, 8
  tasks), neutral `menu.md`, `run-arm.sh` (worker-shaped clean boss
  env, per-run scratch homes, secrets never echoed), MT-C2-01/-04
  with the scoring rules; snippet 75 v2 + skill v2 (route verb +
  scoreboard). **RP1 RESOLVED (owner verbatim in plan §13): GLM-served
  cold boss, 3 runs per arm, technical cap 8.** RP2/RP3 still open
  (recommendations recorded).
- Floor at the pause: conform 0 (7/7 gated), specmap 18 units / 63
  items / 63 edges / 0 orphans / 0 warnings, test-gate xfail-strict,
  ~155 tests. MC daemon **stopped**; real `~/.fractality` untouched
  (scratch homes throughout).
- Commit chain this session: `5242bd6` (contract: delegate cwd law) →
  `36c09aa` (barkain note) → `47412ad` (plan) → `a63a219` (language
  law) → `4f5bc04` (Ф0) → `1c9757b`+`ea6ed83` (Ф1) → `6f5788a`+
  `6d8397e`+`a979ea6` (Ф2) → `4e2c71c`+`5f7dd3b`+`82771bb` (Ф3 +
  reports practice) → `2b24288`+`09845de` (Ф4 + dashboards) →
  `337ea86` (Ф5) → the wind-down commits (boss surface v2, trial
  pre-registration, this checkpoint).

## Constraints (do not violate without discussion)

- Host Rules 1–4; the delegation law + live-observation protocol +
  two context scenarios (scoreboard in every checkpoint); clean-room
  law; I1 worker-env (now also pins FRACTALITY_BOSS_SESSION); I2 bus /
  files-as-persistence; I3 one telemetry store (the initiative engine
  is a consumer, never an accumulator); publish owner-word-only (RP3
  host). **Language law (owner, 2026-07-10): no Python in the shipped
  codebase** — Rust/TS + thin PS/Bash launchers; Python only in
  throwaway spikes/tests and named exceptional cases.
- **F15 + corollary (this session):** stop MC daemons before builds;
  hook smokes rebuild `--workspace` — hooks talk to the SIBLING
  daemon binary, a stale one folds session events with old rules.
- **Cwd law binds every launch:** pin the working directory inside
  the command itself — gates AND delegate launches (violated once
  this session on a delegate launch; killed and relaunched pinned).
- **opencode delegate law:** inputs must live UNDER the launch cwd
  (external_directory auto-rejects); heartbeats are `echo` commands.
- **Reports law (owner, 2026-07-10):** every phase ends with an
  owner-facing report in `reports/` (дата-время-кампания-фаза name);
  big plans mirror as stage-suffixed dashboards (drafted/started/
  changed/paused/resumed/completed/rejected + one -state-plan.md);
  specs/WAL stay the source of truth.

## Delegation scoreboard (session total)

Delegated 5 / delivered 5: barkain survey (5.2) · cc-docs hooks
extraction (5.2) · MC session integration tests (5.2) · fetch.rs
(turbo) · the route slice (5.2; one relaunch after the boss's own cwd
slip). Kept with cause: seam design, nudge policy, ownership
semantics, experiment design, every review (the never-delegate set).

## Next (the resumed session's recipe)

1. `cd packages/org.vibevm.fractality/fractality/v0.1.0 && cargo
   build --workspace` (one build for CLI + daemon + hooks).
2. **Fire the arms** (paid, RP1-authorized):
   `for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh a $n; done`
   then the same with `b`. Watch per the live-observation law
   (`target/trial-results/arm-*/boss-stderr.log`, run-info.txt).
   Expect env-nit friction on the first run — the runner has never
   fired live; the cap 8 covers technical repeats.
3. Score per MT-C2-01's frozen rules; fill both MTs' Recorded runs;
   verdicts for P1/P3 (+ P4 hook-latency bench if convenient).
4. Ф7: close the campaign — §2 execution record, P1–P8 verdicts,
   campaign-close report, completed-plan dashboard, WAL/CONTINUE/
   WORKSPACES, backlog entries (leftovers named in the reports:
   hook debug channel, session TTL reaping, per-packet answer rules,
   auto-answered counter in metrics, quota plan limits).
