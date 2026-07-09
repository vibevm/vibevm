# fractality — WAL (project continuation state)

_Updated: 2026-07-10 late (Phase 3 EXECUTED — collect-back proven live,
manual-test #1 recorded) — the full loop now closes: tolerant
stream-json parser (D14/R2; the result event is authoritative — this
provider's assistant events under-report), pod tee pump with
watch-channel live metering (`PodEvent::Usage` snapshots — MC meters a
run mid-flight), result provenance (worker | extracted | none, with
path) + `usage.json`, pod-side **acceptance runner** (packet
`task.acceptance` → per-command verdicts in status.json + evidence in
acceptance.log; 600 s per-command cap; skipped-with-reason on failed
workers), exit-code families (killed(pod_lost) → 2 infra; policy kills
keep 3), and `run`/`show` rendering usage + cost + result + acceptance.
**MT-01 pre-run green** (scratch home, live GLM-5.2): run
`01KX4JRBNQ774N0G9VYG218TKD`, 36 s, events=599, cost 0.1336, result
(worker), acceptance 1/1 — the worker's four unit tests green in
366 ms; human sign-off pending per the manual-tests law. Phase 3
commits: `799dba3`, `1fb9517`, `01b22d3`, `eb8e7d9`; ledger entry in
plan §14 (findings F16 — profiles are home-scoped, D14 error contract
field-proven; the Collected-event bus promotion + FileRef rendering
deferred by name to Phase 4). Floor: **all green** (specmap 11/38/38/0;
conform forced the pod's `collect` cell split at the 600-line budget).
P3 running count 3/3. Delegation scoreboard this session: **delegated
2, delivered 2** — (1) worktree integration tests (scenario 1, cwd
pinned): green first landing, caught a factual error in the compiled
context by verifying the source, killed the lock-holding daemon it
collided with (F15); (2) the stream parser + goldens (scenario 1,
exact API + golden numbers compiled in): green first landing, one
misleading doc sentence fixed at review. Deferred delegable work,
recorded per the law: acceptance-runner unit tests (fixture commands
with known exit codes — scenario 1, glm-5-turbo candidate; the live
MT-01 exercised the runner end to end meanwhile). Kept boss-side with
reasons: E2E runs + triage, the F14 fix, pod tee + collection + CLI
(cross-crate seam design), the Exit-vs-Collected seam decision,
boundary docs. Campaign tally: delegated 4, delivered 3. Prior status
follows._
_Prior: 2026-07-10 (Phase 2 EXECUTED — exit E2E green on a live GLM
worker) — run `01KX4H4KESV9ADN6S0AJMWQHFW`, exit 0 in 29 s, hello.txt
byte-exact, worker-authored result.md, transcript with usage fields
(**P2 CONFIRMED**; **P3 opens 1/1**). The first firing
(`01KX4GD3C5RQ54YREHPRES6N2F`, kept as autopsy) exposed the
three-defect Windows spawn seam, fixed as **F14** (`38d78bc`): PATHEXT
resolver against the worker's PATH (npm ships only `claude.cmd`),
prompt moved from argv to `WorkerSpec::stdin` (cmd.exe escaping rejects
newline args; 32 KiB cap), case-insensitive D5 whitelist canonicalized
(stock Windows spells `Path`/`ComSpec`). **F15**: a running MC daemon
holds the .exe lock against cargo rebuilds. Worktree tests `9996f74`;
boundary docs `784fbda`._
_Prior: 2026-07-10 (Phase 2 CODE LANDED, floor green) — profiles (D6),
D5 clean-slate env as a pure function (I1 as a unit test), headless
invocation builder (flags pinned on CC 2.1.202), RunSpec +
BackendSecrets ([redacted] Debug) + widened WorkerBackend seam, pod
`--run-spec` product mode (token read pod-side at spawn), MC spawn path
(validation → D8 workspaces incl. git worktrees → run-spec → detached
pod launch), `fractality run --packet` sync loop; commits `b15bd02`,
`10bc4b9`. Delegation field data: profiles module failed twice at GLM
(poisoned cwd; silent planning) — cwd pinning + two context scenarios
entered the contract._
_Prior: 2026-07-10 (Phase 1 EXECUTED) — six-crate workspace + MC + pod
skeleton; P8 restart-survival proven with a real process kill;
AI-Native floor from birth; workspace became a vibe consumer (redbook
^0.2.0 + rust-ai-native ^0.7.0, 26 pkgs); findings F11 (Notify
lost-wakeup → watch) and F12 (in-process abort ≠ crash)._
_Prior: 2026-07-09 (Phase 0 EXECUTED) — spikes green, F1–F10 folded
into the plan; interim opencode+GLM paradigm verified live._

## Current state

- **The plan (canonical for campaign detail):**
  [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md)
  — status `EXECUTING`; **Phases 0–3 in the §14 ledger with live
  evidence; manual-test #1 recorded.** Remaining: Phases 4 (swarm),
  4b (interaction layer), 5 (delegation-rules), 6 (boss integration).
- **Code:** six crates, three binaries; the delegate-out path is proven
  live end to end (MC autostart → spawn → worktree/dir provisioning →
  pod → resolver → clean-slate env → stdin prompt → GLM worker →
  artifacts on disk). Floor all green at this checkpoint; conform 6/6
  gated, empty baseline; specmap 11/36/36/0 (namespace `fractality`);
  pub-doctest gate = next ratchet.
- **This box:** `~/.fractality/profiles.toml` installed (copy of
  `spec/examples/profiles.sample.toml`); two runs on disk under
  `~/.fractality/runs/` — the green `01KX4H4KESV9ADN6S0AJMWQHFW` (the
  Phase 3 golden-fixture source) and the F14 autopsy
  `01KX4GD3C5RQ54YREHPRES6N2F`. MC daemon **stopped** (F15: stop it
  before builds).
- **vibe wiring:** workspace requires redbook ^0.2.0 + rust-ai-native
  ^0.7.0 (standing rule for every future fractality package); own
  vibedeps/ (26 pkgs); boot lane = contract boot step 6. Pilot
  findings: [`VIBEVM-BACKLOG.md`](VIBEVM-BACKLOG.md).
- **Host side:** two redbook members vendored at
  `packages/org.vibevm/{atomic-commits,sync-from-code}/v0.1.0/`
  (tag-pinned mirrors — do not edit).

## Constraints (do not violate without discussion)

- Host Rules 1–4 bind every commit.
- The delegation law + live-observation protocol (incl. cwd pinning) +
  two context scenarios (contract §"THE DELEGATION LAW"); scoreboard in
  every WAL checkpoint.
- Clean-room law for refs; worker env never inherits
  `ANTHROPIC_*`/`CLAUDE_*` (I1 — structural + tested); token files:
  existence checks MC-side, content pod-side at spawn only, never
  logged.
- MC is the bus; files are the persistence plane (I2). Publish is
  owner-word-only. Floor green at phase boundaries; never wait blind on
  long runs.
- **F15 dev law:** stop the MC daemon before any build that touches its
  binary (`fractality mc stop`); a running daemon holds the .exe lock.

## Next (the cold-start recipe — Phase 4, swarm)

1. Re-read plan §8 Phase 4 (async verbs, budgets, nesting, kill-tree,
   metrics) and §14's Phase 3 deferred items — the `Collected` pod
   event (verdicts + result FileRef onto the bus / `RunRecord`) slots
   naturally into Phase 4's metrics/record work.
2. **Async verbs:** `spawn` (register + return id), `wait <id>…`
   (shell semantics), `tree`; per-profile `max_concurrent` admission +
   queueing MC-side.
3. **Budget enforcement in MC:** wall-clock watchdog, `--max-turns`
   passthrough exists, cumulative token cap → `killed(budget)`; the
   live Usage snapshots (already flowing) are the input.
4. **Nesting:** `FRACTALITY_RUN_ID`/`FRACTALITY_DEPTH` already ride
   the worker env; a worker calling `fractality spawn` registers a
   child — depth-2 tree demo (P4 target: 3-worker swarm, manual-test
   #2; recursive kill, manual-test #3).
5. **Kill:** `kill --tree` delegated to the pod (F5 Job Objects);
   orphan-sweep assertion; pod-loss fallback MC-side.
6. Delegation candidates: acceptance-runner unit tests (deferred from
   Phase 3, scenario 1, glm-5-turbo), admission-queue unit tests,
   `wait` verb CLI plumbing (scenario 1 with exact API). Boss keeps:
   budget semantics, tree/kill correctness, the Collected event
   design.
7. Machine note: stop the MC daemon before builds (F15); profiles are
   home-scoped (F16) — scratch homes need their own copy.
