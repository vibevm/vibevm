# fractality — WAL (project continuation state)

_Updated: 2026-07-10 (Phase 2 EXECUTED — the exit E2E ran green on a
live GLM worker) — `fractality run --packet spec/examples/hello-glm.toml`
completed end to end: run `01KX4H4KESV9ADN6S0AJMWQHFW`, exit 0 in 29 s,
`work/hello.txt` byte-exact, worker-authored `result.md`, transcript
with usage fields (**P2 CONFIRMED** on a real product transcript:
usage ×6, result/success, num_turns 3, model glm-5-turbo; **P3 opens
1/1**). The first E2E firing (`01KX4GD3C5RQ54YREHPRES6N2F`, kept as
autopsy) exposed the three-defect Windows spawn seam, fixed as **F14**
(`38d78bc`): PATHEXT resolver against the worker's PATH (npm ships only
`claude.cmd`; CreateProcess finds only `.exe`), prompt moved from argv
to `WorkerSpec::stdin` (cmd.exe escaping rejects newline args; 32 KiB
command-line cap — fatal to big one-shot goals), case-insensitive D5
whitelist matching canonicalized to the whitelist spelling (a stock
Windows env spells `Path`/`ComSpec`; a PowerShell-launched pod handed
its worker no PATH). **F15**: a running MC daemon holds the .exe lock
and blocks `cargo test` rebuilds — stop the daemon before builds that
touch its binary (the arbitration class fractality exists to own).
Worktree-manager integration tests landed (`9996f74`, five tests, real
scratch repo). Floor: **all green** (fmt · tests · clippy -D warnings ·
conform 0 findings 6/6 gated · specmap 11 units / 36 items / 36 edges /
0 orphans · test-gate). Delegation scoreboard this session:
**delegated 1, delivered 1** — the worktree tests ran as a scenario-1
one-shot (discipline compiled into the task, cwd pinned, live-observed
by log+file telemetry): delivered green on first landing, self-verified
(test/fmt/clippy), caught a factual error in the compiled context
(a nonexistent constructor) by verifying against the source, and killed
the locked daemon it collided with (F15). Kept boss-side with reasons:
the E2E run + triage and the F14 fix (spawn-seam correctness discovered
in triage; the tree already carried an active delegate writer).
Campaign tally: delegated 3, delivered 2. Prior status follows._
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

## Next (the cold-start recipe — Phase 3, collect-back)

1. **First golden fixture:** copy the green run's transcript into the
   repo — `~/.fractality/runs/01KX4H4KESV9ADN6S0AJMWQHFW/worker-stdout.jsonl`
   → `fractality/v0.1.0/crates/fractality-core/tests/fixtures/` (or the
   crate the parser lands in). 16.8 KB, 20 stream events, usage ×6.
2. Stream-json incremental parser (plan Phase 3 step 1; tolerant per
   D14 — unknown event kinds preserved as `Other`, never fatal): state
   transitions, usage accumulation, final-result extraction.
3. Collection (step 2): `result.md` (fall back to final-message
   extraction, record which happened), `usage.json`, `status.json`;
   packet acceptance commands run in the workspace, verdicts recorded.
4. `run` one-screen summary upgrade + `show` parity (step 3): state,
   wall, tokens, result as a FileRef (D19), branch, acceptance
   verdicts. Fold in the exit-code polish (pod_lost → infra family 2,
   not killed family 3 — D17 table review).
5. E2E exit for Phase 3: packet "implement a small Rust function + test
   in a scratch repo", GLM-5.2 worker, acceptance `cargo test` green;
   recorded as manual-test #1 (house manual-tests flow).
6. Delegation candidates for this phase (law: delegate or record why
   not): parser skeleton from the frozen fixture (scenario 1 — compile
   the event-shape table into the task), fixture sanitization, golden
   assertions. Boss keeps: parser tolerance semantics (D14 judgment),
   collection state machine, anything touching run-dir layout.
