# fractality — cold-resume checkpoint

_Written 2026-07-10 ~14:10, pausing mid-Campaign-2 at the Ф6 boundary
(owner: «сохранить и перезапустить сессию»). `WAL.md` (same
directory) is the canonical living state and supersedes this snapshot
wherever they diverge._

## TL;DR

**Campaign 2 (the initiative system) is EXECUTING: Ф0–Ф5 are done and
green; the Ф6 trial is fully pre-registered and committed; the paid
arms have NOT fired.** The owner authorized them (RP1: GLM-served
cold boss, 3 runs per arm, cap 8) — firing and scoring them is the
next session's first move, then Ф7 closes the campaign. Everything
the trial needs is frozen in the tree: staging crate, neutral menu,
runner script, MT-C2-01/-04 with pre-registered scoring.

## Where work stands

- Branch `main`, mirrors synced at the wind-down commits. This
  session's chain: campaign open `47412ad` → Ф0 `4f5bc04` → Ф1
  `1c9757b` → Ф2 `6f5788a`/`6d8397e` → Ф3 `4e2c71c` → Ф4 `2b24288` →
  Ф5 `337ea86` (+ ledgers, reports, dashboards, and the wind-down
  commits after each).
- Floor at the pause: **all green** — conform 0 (7/7 gated), specmap
  18 units / 63 items / 63 edges / 0 orphans, test-gate xfail-strict,
  ~155 tests. MC daemon stopped; real `~/.fractality` untouched.
- What Campaign 2 built so far: boss sessions + attribution
  (`FRACTALITY_BOSS_SESSION` via CLAUDE_ENV_FILE), the pure
  initiative engine (scoreboard/nudges/route — strictly factual), the
  CC adapter (`hook`/`statusline`/`harness install` with
  command-string ownership; availability law: any failure → silent
  exit 0), threshold nudges with cooldown, stop-time parked-question
  alerts with folded acks, profile auto-answer rules, matrix-as-data
  (10/10 goldens, P5 ✅), `fetch` (D12 repair), quota rollup in
  `stats`.

## The active blocker

None. RP1 is resolved (the arms are authorized); RP2/RP3 remain open
with recommendations in plan §13 (they do not block the trial).

## Next-steps recipe (cold start)

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo build --workspace        # ONE build: hooks talk to the sibling daemon exe
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh a "$n"; done
for n in 1 2 3; do bash spec/manual-tests/trial/run-arm.sh b "$n"; done
# results: target/trial-results/arm-{a,b}-run-{1..3}/
```

Watch the runs per the live-observation law (boss-stderr.log +
run-info.txt; expect first-run env friction — the runner has never
fired live; the RP1 cap of 8 exists for technical repeats). Then
score by MT-C2-01's frozen rules (attempted/delegated over the
eligible set E={1..6}; distractor delegations reported separately),
fill both MTs' Recorded runs, rule P1 (<50% arm A) and P3 (≥80% and
≥A+30 arm B), and close Ф7: §2 execution record, P1–P8 verdicts,
campaign-close report + completed-plan dashboard, WAL/CONTINUE/
WORKSPACES refresh, backlog entries.

## Non-obvious findings this session (do not rediscover)

- **opencode delegates cannot read outside their launch cwd**
  (external_directory auto-reject) — copy inputs under the scratch
  cwd; heartbeats must be `echo` commands.
- **Hook smokes need `cargo build --workspace`** — hooks talk to the
  sibling `fractality-mission-control.exe`; a stale daemon folds
  session events with old rules (the cooldown "didn't work" until
  the rebuild).
- **Stop-hook additionalContext CONTINUES the turn** (harness
  contract) — use only for deliberate interrupts (our parked-question
  alert), never for casual summaries.
- statusline does not run in `-p` (F20); PostToolUse carries
  `duration_ms` (F21); warm exe spawn ≈ 6 ms (F22).
- The conform file budget forced three real seams this campaign
  (mc_cmd, http_sessions, http_questions) — let the gate design.
- reqwest 0.13 renamed `rustls-tls` → `rustls` (D11's pin caught it).

## Repository map (workspace)

`packages/org.vibevm.fractality/` — contract (CLAUDE.md: delegation
law, language law, REPORTS law + plan dashboards), WAL.md, this file,
VIBEVM-BACKLOG.md, **reports/** (per-phase narratives + campaign2
started/state/paused plan dashboards); `fractality/v0.1.0/` — the
Cargo workspace: crates/{core, mission-control, pod, mc-client,
backend-claude-code, cli, **initiative**}, spec/ (PROP-001 +
§3b sessions, the CLOSED IGNITION plan, the EXECUTING
FRACTALITY-INITIATIVE plan, manual-tests MT-01…05 + MT-C2-01…04 +
**trial/** staging+menu+runner, examples incl. initiative.sample.toml,
boot snippet 75 v2, skills/fractality-delegate v2), vibedeps/ +
discipline configs (conform.toml env_roots incl. hook.rs; specmap.toml
external_specs incl. the sibling delegation-rules);
`delegation-rules/v0.1.0/` — the policy package (DECISION-MATRIX now
cites its executable form in the engine).

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
./target/debug/fractality.exe scoreboard   # the initiative surface, live
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
