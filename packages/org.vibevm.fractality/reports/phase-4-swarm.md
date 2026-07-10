# Phase 4 report — swarm: async lifecycle, budgets, kill-tree, metrics

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 4 · executed 2026-07-10.
This report is the phase's narrative record for the owner; the plan's
§14 ledger stays the canonical commit map._

## What the phase delivered

- **Async verbs.** `fractality spawn` (fire-and-return, prints the run
  id on stdout — `id=$(fractality spawn …)` composes), `wait <id>…`
  (shell-wait semantics: blocks on all, exit code mirrors the last id's
  outcome), `tree [<id>]` (one tree or the whole forest), `kill <id>
  [--tree]`.
- **Admission control.** Registration always lands `queued`;
  per-profile `max_concurrent` slots; FIFO by ULID order; slots free on
  any terminal event; the sweeper re-ticks admission as self-heal. The
  launch claim (`queued → starting`) is journaled BEFORE any side
  effect, atomically (`claim_queued`), so restarts and concurrent ticks
  can never double-spawn one run.
- **Budgets are mechanism now.** Wall-clock watchdog and cumulative
  output-token cap → `killed(budget)`; the token cap also fires inline
  the moment a usage snapshot crosses it. `--max-turns` already rode
  the invocation since Phase 2. A budget field of 0 means "unlimited"
  on that axis (documented on `BudgetSpec`).
- **Kill delivery, layered.** MC journals `killed(reason)` at decision
  time (authoritative); a fresh pod receives the kill on its next
  heartbeat (interval lowered 2 s → 1 s so P5's two-second bound has
  headroom) and closes its Job Object — the F5-proven tree reap; a
  stale/silent pod escalates to the OS fallback (`taskkill /PID <pod>
  /T /F`); the sweeper re-arms undelivered kills. Delivery is
  at-least-once; the pod-side kill is idempotent.
- **Nesting is real.** `RunRecord.parent`/`depth` land at registration
  (parent must be alive); the pod injects `FRACTALITY_HOME` and
  prepends its own directory to the worker PATH, so a worker's
  `fractality spawn` hits the same mission-control and attaches to its
  own run automatically (FRACTALITY_RUN_ID → default parent).
- **Collected rides the bus.** The pod ships result provenance +
  acceptance verdicts as a `Collected` event; MC mints the D19 FileRef
  (scope-relative path + size/mtime etag) and folds everything into
  `RunRecord.collected` — `show`/`run` render from the record and only
  fall back to plane files for pre-Phase-4 runs.
- **Metrics.** `GET /v0/metrics`: totals + by_state / by_profile /
  by_model / by_day buckets (runs, outcomes, tokens, cost, wall time of
  terminal runs, and the D12 web-tool counter). The stream parser now
  counts `tool_use` blocks by name; web-ish names (WebFetch, WebSearch,
  web_search/web_reader/zread MCP shapes) accumulate into
  `usage.web_tool_calls`.
- **The allowlist layer arrived early.** `profile.permissions.
  allow_tools` → `--allowed-tools` passthrough. Pulled forward from
  Phase 4b because the nesting demo (MT-03) needs a worker that may
  call `fractality spawn` via Bash; it is also the first third of the
  D18 stack.

## Important decisions taken (and why)

1. **MC records `killed` at decision time, not at confirmation.** The
   run is dead the moment the operator/budget says so; the pod's later
   exit report on a closed run is acknowledged as the kill's tail, not
   an error (`pod_event` is terminal-tolerant; usage and collection
   still land as facts). Alternative — wait for pod confirmation —
   leaves a "dying" state that every reader must special-case, and
   breaks when the pod is already gone.
2. **Kill rides the heartbeat answer; there is no MC→pod push.** Keeps
   D3's one-direction connection model (pods dial in), survives daemon
   restarts trivially, and costs at most one heartbeat interval (now
   1 s) of latency. Escalation to `taskkill` covers the dead-pod case.
3. **The launch claim is a journaled state transition.** After a crash,
   only `queued` runs are launch candidates — a run mid-launch is
   `starting` and is never re-launched; if its pod never registers, the
   sweeper fails it loudly after 60 s pointing at pod.log.
4. **FileRef minting is MC-side.** The pod ships a plain path; MC owns
   scopes and stamps etags (D19 says MC-stamped). Keeps the pod free of
   scope bookkeeping.
5. **`spawn_requested` is a persisted record field.** Raw registrations
   (tests, manual pod driving) must never be auto-launched by
   admission; the flag survives restarts with the journal.

## Bugs found and fixed in flight

- **Acceptance runner leaked timed-out commands.** `tokio::time::
  timeout` dropping `cmd.output()` does NOT kill the child by default —
  a hung acceptance command would outlive its "killed: exceeded cap"
  verdict. Fixed with `kill_on_drop(true)` before the delegated test
  suite pinned the behavior.
- **Admission double-claim race.** `record()`'s idempotent same-state
  path (`starting → starting` is a legal re-assertion) meant two
  concurrent admission ticks could both "claim" one queued run and
  spawn two pods. Fixed with an atomic `claim_queued` (check + journal
  under one lock acquisition) plus a per-tick attempted-set so a
  journal fault cannot spin the loop.
- **(Inherited, noted)** `Cargo.lock` tail from commit `799dba3` —
  the serde_json dependency edit landed without its lock line; landed
  as the phase-opening chore commit.

## Strange things / surprises

- **Delegate vs my in-flight breakage.** Delegation #1 (acceptance
  tests, glm-5-turbo) self-verifies with `cargo test -p
  fractality-pod`, whose dev-dependency is the mission-control crate —
  which I was actively rewriting. The delegate correctly diagnosed the
  breakage as pre-existing ("not my errors"), then resolved its
  contradictory constraints (self-verify vs frozen manifest) by
  TEMPORARILY removing the broken dev-dep and restoring it at the end.
  Verdict: reasonable under contradiction, but the constraint conflict
  was my fault — do not hand out self-verify targets that transit
  crates under live edit. Field data for the Phase 5 playbooks.
- **cwd pinning near-miss, twice.** Both delegate launches relied on
  the shell's inherited cwd instead of `cd <ws> && opencode …` as the
  contract demands. Verified correct empirically both times, but the
  law exists because "probably right" once cost 12 minutes; the launch
  template needs the `cd` baked in.
- **taskkill exit 128 = "not found" = success.** The fallback kill
  treats a missing pod pid as the goal state, not an error — worth
  knowing when reading mc.log.
- **`git worktree` deliverables are uncommitted.** Workers on the
  default permission posture cannot run git, so their files sit
  uncommitted in the worktree; the branch itself carries only base
  commits until the boss commits/merges. MT-02 therefore has the boss
  commit each worktree before merging — an honest reflection of the
  RP4 posture, recorded so nobody expects worker-authored commits.

## Deliberately deferred / unfinished (named)

- **Server-side long-poll `wait`** — CLI polls at 500 ms; an
  event-driven wait endpoint joins the SSE family (DEF-6).
- **Monthly quota windows** — `web_tool_calls` accumulates per run and
  aggregates by day; the "N of 4000 this month" view is a consumer-side
  sum over `by_day` (Phase 6 `stats` can render it), not an MC concept
  yet.
- **Admission tick blocks the async handler thread** (fs + process
  spawn under axum) — inherited shape from Phase 2's spawn path;
  fine at v0.1 scale, a tokio::task::spawn_blocking refactor is future
  work.
- **POSIX fallback kill** is `kill -9 <pod-pid>` (no process-group
  sweep) — the POSIX path overall awaits the CI matrix (DEF-8).
- **Unreadable profiles pause admission with only a warn log** — a
  `fractality doctor` verb is the natural future surface.
- **Restart command for pods** (D3's "restart a crashed worker with
  amended parameters") — PodCommand has room; not built in v0.1.

## Delegation scoreboard (the law's ledger)

- **Delegated 2, delivered 2** (running campaign total: 6/5 counting
  Phases 0–3):
  1. Acceptance-runner unit tests — glm-5-turbo, scenario 1 (exact
     API + platform-split commands compiled into the prompt). Green
     first landing (5/5), one manifest excursion under contradictory
     constraints (see above), restored by itself. Review: accepted
     verbatim.
  2. Admission/kill state-primitive unit tests — glm-5.2, scenario 1
     (full 22-field RunRecord constructor + exact helper APIs in the
     prompt). Green first landing (5/5), "no other files modified"
     confirmed by diff review.
- **Kept boss-side, with reasons:** cross-crate seam design (kill
  delivery, Collected minting, admission claim atomicity) — judgment;
  MT-02/03 fixture packets + procedure docs — smaller than their
  prompts and genre-bound (manual-tests law wants the boss's pen);
  the race fix — found during self-review of my own lock discipline.

## Evidence (filled at the boundary)

- Floor: see §14 ledger entry (this run: all gates green — fmt, tests
  including 10 delegated ones, clippy -D warnings, conform, specmap,
  test-gate).
- MT-02 (3-worker swarm, P4) and MT-03 (recursive kill, P5): pre-run
  transcripts appended to the manual-test documents; verdicts recorded
  in the §14 ledger entry.
