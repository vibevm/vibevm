# fractality — WAL (project continuation state)

_Updated: 2026-07-10 late (Phase 3 FIRST SLICE LANDED — collection +
metering live) — the backend gained the tolerant incremental
stream-json parser (D14/R2: unknown kinds counted, malformed counted,
never fatal; the result event is authoritative for totals — assistant
events under-report on this provider); the pod's transcript pump is now
a tee (file + parser + `watch`-channel live totals sampled by the
heartbeat → `PodEvent::Usage` snapshots — MC meters a run mid-flight);
at exit the pod settles the result contract (worker | extracted | none,
with the path) and writes `usage.json`; `run`/`show` print usage, cost,
and the result pointer with provenance. Live proof: run
`01KX4J7BNX5J7NK8CB86H77RPM` — summary printed in=16692 out=238
cache_r=23616 events=56, cost 0.101218, result (worker); usage.json ≡
bus record field for field. Commits: `799dba3` (feat: collection,
metering, sync run), `1fb9517` (test: goldens from the frozen Phase-2
transcript — event map, authoritative totals, tolerance pins). Floor:
**all green** (specmap now 11 units / 37 items / 37 edges / 0 orphans).
**Remaining for the Phase 3 boundary:** acceptance-command runner
(packet `acceptance` array → verdicts), result-as-FileRef in the
summary (D19), the pod_lost exit-code polish (3 → infra 2, D17 table),
and the Phase 3 exit E2E (real Rust-function packet on GLM-5.2 with
`cargo test` acceptance → manual-test #1). Ledger entry waits for that
boundary. Delegation scoreboard this session: **delegated 2,
delivered 2** — (1) worktree integration tests (scenario 1, cwd
pinned): green first landing, caught a factual error in the compiled
context (nonexistent constructor) by verifying the source, killed the
lock-holding daemon it collided with (F15); (2) the stream parser +
goldens (scenario 1, exact API + semantics + golden numbers compiled
in): green first landing, zero corrections needed beyond one
misleading doc sentence. Kept boss-side with reasons: E2E runs +
triage, the F14 fix, pod tee + collection + CLI (cross-crate seam
design), boundary docs. Campaign tally: delegated 4, delivered 3.
Prior status follows._
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
  — status `EXECUTING`; **Phases 0–2 in the §14 ledger with the E2E
  evidence.** Remaining: Phases 3 (collect-back), 4, 4b, 5, 6.
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

## Next (the cold-start recipe — Phase 3 remainder)

1. **Acceptance runner (plan Phase 3 step 2 tail):** after worker exit,
   the pod runs each `packet.acceptance` command in the workspace
   (shell form: `cmd /C` on Windows, `sh -c` on POSIX; pod's own env,
   worker's cwd), records per-command verdict + duration into
   status.json (and the exit report — decide: extend `PodEvent::Exit`
   vs a new `Collected` event; the boss holds this seam decision).
2. **Result as FileRef (D19)** in the `run` summary / `show` output,
   scope-relative — the rendering half; the FileRef type already lives
   in core.
3. **Exit-code polish:** `killed(pod_lost)` → infra family (2), not
   killed family (3); review the whole D17 table while there
   (`crates/fractality-cli`, exit-code mapping).
4. **Phase 3 exit E2E → manual-test #1:** packet "implement a small
   Rust function + test in a scratch repo", GLM-5.2 (`model = "big"`),
   acceptance `cargo test` green; record procedure + output under
   `fractality/v0.1.0/spec/manual-tests/`. P3 counting continues
   (currently 2/2 across live runs).
5. Then the Phase 3 ledger entry + WAL/WORKSPACES refresh, and on to
   Phase 4 (swarm).
6. Delegation candidates (law: delegate or record why not): the
   acceptance-runner unit tests (scenario 1 — fixture commands with
   known exit codes), the manual-test document draft from the run
   artifacts. Boss keeps: the Exit-vs-Collected API seam decision,
   exit-code semantics, the E2E itself.
