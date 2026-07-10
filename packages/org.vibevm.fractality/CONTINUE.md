# fractality — cold-resume checkpoint

_Written 2026-07-10 at the IGNITION campaign close. `WAL.md` (same
directory) is the canonical living state and supersedes this snapshot
wherever they diverge._

## TL;DR

**The IGNITION campaign is CLOSED — all seven phases (0–6) executed
and ledgered.** fractality is now a working agent OS seed: a boss
delegates packets to isolated GLM workers under mission-control;
swarms run in parallel worktrees with budgets and recursive kill; a
parked worker can ask the boss a question and resume on the answer;
a decidable policy matrix routes what gets delegated; and the boss
integration (boot snippet + skill + stats) closed the loop by pushing
a REAL host chore (the RP1 relicense) through the fabric itself.
Predictions: P1–P6, P8 confirmed (swarm ratio 1.00; kill-tree
1 025 ms; 11-minute park survived; matrix 10/10 decidable); P7
(commit count) falsified with every drift ledgered. All five manual
tests recorded green and **signed off by the owner (2026-07-10)**.

## Where work stands

- Branch `main`; the campaign landed as: dogfood worktree commits
  `c502a08`/`425ed3e` → merges `893e314`/`79938ab` → the feature
  series `d363405` (swarm/budgets/kill/metrics + F17/F19 hardening),
  `13f201c` (ask_boss broker + boss verbs), `c4dcdaf` (MT docs #2–#5),
  `cbc3c9a` (delegation-rules package), `cc69af6` (boss boot snippet +
  skill) → the close docs commit (ledgers §14, §2 execution record,
  reports/, WAL, this file, WORKSPACES row, backlog entries).
- Floor at close: **all green** — fmt · ~120 tests · clippy -D
  warnings · conform 0 findings (6/6 gated; two recorded
  `#[spec(deviates)]` testimonies on the F17 kernel32 FFI) · specmap
  16 units / 47 items / 47 edges / 0 orphans · test-gate.
- §12 whole-campaign acceptance ran green at close (live hello-glm on
  the real home rendering the D19 FileRef line; questions empty;
  stats; 5 MT procedures; host self-check).
- Real `~/.fractality` untouched by MTs (scratch homes); it now holds
  the Phase-2/3 runs + the acceptance run. **MC daemon: stopped.**

## The active blocker

None. The campaign is closed, the manual-test index is signed off,
both mirrors are synced.

## Next-steps recipe (cold start)

1. Read `WAL.md` → this file → the plan's §2/§14
   (`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`) →
   the per-phase narratives in [`reports/`](reports/).
2. Campaign 2 (initiative system) is seeded in the plan's §15 DEF-1:
   scoreboard-driven delegation nudges; the warm P6=100% is the floor
   a COLD session must be measured against; natural cargo:
   routing-as-data, question push-notifications, D18 layer 2 (dynamic
   permission brokering). Campaign 3 (RLM): DEF-2 with the owner's
   hypothesis quoted in §3.
3. Small named leftovers (§15 extension): server-side long-poll wait,
   F18 result-path knob, monthly quota rollup in stats, `wait
   --verbose`, POSIX fallback kill semantics, `vibe skill install`
   projection of fractality-delegate on this box.

## Non-obvious findings this campaign (do not rediscover)

- **F17:** a detached daemon auto-started from inside `$( … )` command
  substitution inherits the substitution pipe's write end on Windows
  (CreateProcess copies EVERY inheritable handle) — the shell hangs
  forever. Fixed by stripping `HANDLE_FLAG_INHERIT` from the client's
  std handles around the spawn; pinned by
  `autostart_does_not_capture_the_callers_pipes`.
- **F19:** `git worktree add` of a deep repo into a run dir overflows
  Windows MAX_PATH — provisioning passes `-c core.longpaths=true`.
- **F18:** every worktree worker writes `result.md` per the output
  contract → N branches collide at one path on merge; commit only the
  intended module (procedure), result-path knob deferred (product).
- **The cwd law binds the boss:** a floor run from a wrong directory
  silently gated the HOST tree and reported green (baselines differ:
  host 3, fractality 0). Pin cwd in every gate invocation.
- Acceptance predicates must assert what CHANGED (the EULA placeholder
  itself mentions the UPL — `findstr Universal` passed either way).
- A parked (waiting_on_boss) worker burns zero tokens — the CC process
  blocks on one MCP tool result; an 11-minute park resumed cleanly.
- Kill delivery rides the 1 s heartbeat answer; `taskkill` exit 128 =
  "already gone" = the goal state.
- vibe pilot: vendored files materialise CRLF on Windows; no
  re-materialise path for in-place content edits (both in
  `VIBEVM-BACKLOG.md`, 2026-07-10).

## Repository map (workspace)

`packages/org.vibevm.fractality/` — contract (CLAUDE.md), WAL.md, this
file, VIBEVM-BACKLOG.md, **reports/** (per-phase + campaign-close
narratives); `fractality/v0.1.0/` — the Cargo workspace: crates/
{core, mission-control (admission/kill/metrics cells), pod
(worker_env cell), mc-client (F17 guard), backend-claude-code, cli
(swarm/boss/broker cells)}, spec/ (PROP-001, the CLOSED plan,
manual-tests MT-01…05, examples, boot incl. snippet 75, skills/
fractality-delegate), vibedeps/ + discipline configs;
`delegation-rules/v0.1.0/` — the policy package (DECISION-MATRIX,
playbooks, boot snippet 77, its own vibedeps).

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
# the floor (cwd matters — see the cwd law above):
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
# a live smoke (paid, one turbo turn):
./target/debug/fractality.exe run --packet spec/examples/hello-glm.toml
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
