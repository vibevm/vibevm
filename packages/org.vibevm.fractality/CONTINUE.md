# fractality — cold-resume checkpoint

_Written 2026-07-10 at the Phase-2-code wind-down. `WAL.md` (same
directory) is the canonical living state and supersedes this snapshot
wherever they diverge._

## TL;DR

fractality is an agent OS incubated in the vibevm repo as a workspace:
an expensive boss delegates to swarms of cheap Claude Code workers
(GLM via z.ai) supervised by pods under a mission-control daemon; all
content exchange rides files + the localhost bus. **Phases 0–1 are
executed and ledgered; Phase 2 (delegate-out) code is fully landed and
floor-green — the one open exit criterion is the live GLM E2E, which
deliberately opens the next session.** The workspace runs the full
AI-Native discipline (conform 6/6 gated, specmap 0 orphans) and is a
real vibe consumer (redbook + rust-ai-native in its own vibedeps).

## Where work stands

- Branch `main`, synced to both mirrors after the wind-down push; tree
  clean. Last code commits: `b15bd02` (profiles + clean-slate worker
  env), `10bc4b9` (spawn path — mc-owned workers and worktrees).
- Floor `rust-ai-native floor`: **all green** (fmt · tests incl. the D6
  profile goldens and the I1 poisoned-parent test · clippy -D warnings ·
  conform 0 findings · specmap 0 orphans · test-gate).
- What exists in code: profiles (D6) → env constructor (D5, pure over an
  os-env snapshot) → invocation builder (CC 2.1.202 flags pinned live) →
  pod `--run-spec` product mode (token read pod-side at spawn; plaintext
  only in pod memory) → MC spawn path (validation, D8 workspaces incl.
  git worktrees, run-spec, detached pod launch, pod.log) →
  `fractality run --packet` sync loop with the one-screen summary.
- **NOT done:** the live E2E (hello-glm through a real GLM worker) — the
  Phase 2 exit criterion; the Phase 2 ledger entry (waits for that
  evidence); worktree-manager integration tests (queued for GLM);
  pub-doctest ratchet.

## Active blocker

None. The E2E needs only: `~/.fractality/profiles.toml` (copy the
sample, step 1 below) and one paid GLM-5-turbo worker turn.

## Next-steps recipe (cold start)

1. `cp fractality/v0.1.0/spec/examples/profiles.sample.toml ~/.fractality/profiles.toml`
   (values already match this box; `~/.vibevm/zai.api.token` exists).
2. From `fractality/v0.1.0/`: `cargo build --workspace`, then
   `target/debug/fractality.exe run --packet spec/examples/hello-glm.toml`.
   Green looks like: exit 0; run dir (`~/.fractality/runs/<ulid>/`)
   holds packet.toml, run-spec.toml, worker-stdout.jsonl (stream-json
   WITH usage fields → that's the P2 verdict), status.json, pod.log;
   `work/` holds hello.txt + result.md. Watch with `fractality ps` /
   `show <prefix>`. Debug surfaces: pod.log (pod-side), mc.log
   (daemon), worker-stderr.log.
3. Write the plan §14 Phase 2 ledger entry (commit map b15bd02/10bc4b9 +
   the E2E evidence + P2 verdict), refresh WAL + WORKSPACES row, commit
   `docs(fractality): phase 2 ledger — E2E evidence`, push via
   `cargo xtask mirror`.
4. Delegate worktree-manager tests to GLM (scenario 1; PIN THE CWD in
   the launch command — see the contract's live-observation protocol):
   `crates/fractality-mission-control/tests/workspace.rs`, scratch git
   repo fixture, assertions on provision (branch `fractality/<id>`, wt
   dir, base) / remove_worktree / dir / none.
5. Phase 3 (collect-back) per the plan: tolerant stream-json parser
   (first golden = the E2E transcript), collection
   (result.md/usage.json/status.json + acceptance commands), `run`
   summary upgrade + `show` parity. Manual-test #1 records there.

## Non-obvious findings this session (do not rediscover)

- **Delegates:** opencode inherits the SHELL's cwd — pin it in the
  launch command or the worker roams the wrong tree (it happened: 12
  minutes against the host root). opencode end-buffers stdout under
  redirection → telemetry = file mtimes + worker-printed `PROGRESS:`
  markers (+ `--print-logs` streams to stderr if needed). A delegate
  reads only its target files (~15k cached prefix; the discipline
  corpus is NOT auto-ingested) — hence the contract's two context
  scenarios.
- **tokio:** `Notify::notify_waiters` loses wakeups vs not-yet-polled
  waiters — lifecycle signals must be state (`watch`); an in-process
  `abort()` is NOT a crash (hyper per-connection tasks keep serving
  pooled connections) — crash tests kill real processes.
- **Windows:** `Utf8PathBuf::join` yields backslashes — compare paths
  via join, never string literals; `taskkill //IM x.exe //T //F` (Git
  Bash needs the double slashes).
- **conform:** `env_roots` is the sanctioned home for deliberate
  ambient-env readers (home resolution, binary discovery, identity
  fallbacks, the pod's snapshot root).

## Repository map (workspace)

`packages/org.vibevm.fractality/` — contract (CLAUDE.md), WAL.md, this
file, VIBEVM-BACKLOG.md (pilot findings + verification recipes);
`fractality/v0.1.0/` — the Cargo workspace: `crates/{fractality-core,
-mission-control, -pod, -mc-client, -backend-claude-code, -cli}`,
`spec/` (PROP-001, the campaign plan, refs, examples), `vibedeps/` +
`spec/boot/INDEX.md` (the vibe-generated practice lane), conform.toml /
specmap.toml / specmap.json / discipline/ (the gates).

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
rust-ai-native floor   # or: <host>/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
