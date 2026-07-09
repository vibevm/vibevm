# fractality — WAL (project continuation state)

_Updated: 2026-07-10 (Phase 2 CODE LANDED, floor green — E2E is the open
exit criterion) — the delegate-out path exists end to end in code:
profiles (D6, schema-1 TOML, Class-F errors, slot resolution), the D5
clean-slate env constructor as a pure function over an os-env snapshot
(I1 poisoned-parent test is a plain unit test), the headless invocation
builder (flags pinned live on CC 2.1.202), RunSpec + BackendSecrets
(Debug = [redacted]) + the widened WorkerBackend seam, the pod's
`--run-spec` product mode (profile + token resolved pod-side at spawn
time — plaintext exists only in pod memory), MC's spawn path
(profile/token-existence validation → D8 workspace provisioning incl.
git worktrees → run-spec → detached pod launch with pod.log), and
`fractality run --packet` (sync loop, one-screen summary, semantic exit
codes). Floor: **all green** (fmt · tests incl. profile goldens +
poisoned-parent · clippy -D warnings · conform 0 findings 6/6 gated ·
specmap 0 orphans · test-gate). **NOT yet run: the live GLM E2E** —
hello-glm through a real pod+worker is the Phase 2 exit criterion and
opens the next session (needs ~/.fractality/profiles.toml on this box;
sample: `fractality/v0.1.0/spec/examples/profiles.sample.toml`).
Delegation scoreboard: **delegated 2, delivered 1** — the error-contract
drain (4 enums, 23 rule-hits) was GLM-executed and boss-accepted
earlier; the profiles module was launched twice at GLM and delivered
nothing (launch 1 poisoned by an inherited cwd — it roamed the HOST
root for 12 minutes; launch 2 stuck in silent planning) and the boss
wrote it from the ready spec when the owner called the wind-down.
Field data & new contract law from those failures: live-observation
protocol item 6 (pin the cwd in the launch command), delegate context
economics measured (~15k cached prefix, targets-only reads, stdout
end-buffered ⇒ mtime telemetry), and the two context scenarios (small
task = compile the discipline into the prompt; big task = delegate
boots the corpus). Session bugs fixed live: clippy large_enum_variant
(Registered boxed); Windows path-separator assert in the backend test.
Commit map this phase so far: `b15bd02` (profiles + clean-slate worker
env), `10bc4b9` (spawn path — mc-owned workers and worktrees); Phase 2
ledger entry deliberately waits for the E2E evidence. Prior status
follows._
_Prior: 2026-07-10 (Phase 1 EXECUTED) — six-crate workspace + MC + pod
skeleton live; P8 restart-survival proven with a real process kill;
AI-Native floor adopted from birth (D15 rewritten, DEF-9 early); the
workspace became a vibe consumer (redbook ^0.2.0 + rust-ai-native
^0.7.0, 26 pkgs in its own vibedeps, boot lane = contract step 6);
pilot posture + delegation law + live-observation protocol entered the
contract; findings F11 (Notify lost-wakeup → watch) and F12 (in-process
abort ≠ crash: pooled connections survive) in the plan §14._
_Prior: 2026-07-09 (Phase 0 EXECUTED) — spikes green, F1–F10 folded into
the plan; interim opencode+GLM paradigm verified live._

## Current state

- **The plan (canonical for campaign detail):**
  [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md)
  — status `EXECUTING`; Phases 0–1 in the §14 ledger; **Phase 2 code
  landed (b15bd02, 10bc4b9), its ledger entry + P2/P3 checks land with
  the E2E evidence.** Remaining: E2E, then Phases 3, 4, 4b, 5, 6.
- **Code:** six crates, three binaries, floor = `rust-ai-native floor`
  (all green at this checkpoint). Conform 6/6 gated, empty baseline;
  specmap 12 units / 38 items / 38 edges / 0 orphans (namespace
  `fractality`); pub-doctest gate = named next ratchet.
- **vibe wiring:** workspace vibe.toml requires redbook ^0.2.0 +
  rust-ai-native ^0.7.0 (standing rule for every future fractality
  package); own vibedeps/ (26 pkgs) committed; boot lane bound as
  contract boot step 6. Pilot findings + fix-list + verification plan:
  [`VIBEVM-BACKLOG.md`](VIBEVM-BACKLOG.md); operating recipe: contract
  §"Driving vibevm here".
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

## Next (the cold-start recipe)

1. `cp fractality/v0.1.0/spec/examples/profiles.sample.toml
   ~/.fractality/profiles.toml` (values are already correct for this
   box; token file `~/.vibevm/zai.api.token` exists).
2. **The Phase 2 exit E2E:** from `fractality/v0.1.0/`:
   `cargo build --workspace`, then
   `target/debug/fractality.exe run --packet spec/examples/hello-glm.toml`
   — expect: pod launches, GLM-5-turbo worker runs headless, exit 0,
   run dir holds packet.toml, run-spec.toml, worker-stdout.jsonl
   (stream-json with usage fields → P2), status.json, pod.log, and the
   worker-written hello.txt + result.md in `work/`. Check
   `fractality ps` / `show` along the way.
3. On green: write the Phase 2 ledger entry (P2 verdict from the real
   transcript; P3 counting starts), refresh WAL/WORKSPACES, commit
   `docs(fractality): phase 2 ledger — E2E evidence`.
4. Then delegate (scenario 1, cwd-pinned!) the worktree-manager
   integration tests to GLM: scratch git repo fixture → provision
   worktree (branch fractality/<id>, wt dir, base ref) → remove_worktree
   → dir/none modes; file
   `crates/fractality-mission-control/tests/workspace.rs`.
5. Then Phase 3 (collect-back): stream-json parser (fixture from the
   E2E transcript), result.md/usage.json/status.json collection,
   acceptance runner, `run` summary upgrade.
