# fractality — cold-resume checkpoint

_Written 2026-07-09 at workspace bootstrap. `WAL.md` (same directory) is the
canonical living state and supersedes this snapshot wherever they diverge._

## TL;DR

fractality is a new, independent product incubated inside the vibevm
repository as a **workspace**: an agent operating system in its earliest
form. An expensive "boss" agent (Claude Code on the owner's Max
subscription) delegates tasks to swarms of cheap worker agents — Claude Code
processes running under other providers (GLM 5.2 / GLM-5-Turbo via z.ai)
with strictly isolated environments. A Rust daemon,
`fractality-mission-control`, is the scheduler: it spawns workers, holds the
run registry and call tree, meters usage, and will later feed observability
and meta-cognition. All content exchange is files on disk. Everything is
clean-room, cross-platform Rust.

The IGNITION campaign plan is authored and `PLANNED`; nothing is built yet.

## Where work stands

- **Plan status: EXECUTING.** Phase 0 (spikes) landed GREEN 2026-07-09,
  no code committed (spikes commit nothing; findings F1–F10 are folded
  into the plan §8). Next work is **Phase 1**.
- Workspace bootstrapped: contract (`CLAUDE.md`), WAL, this file, and the
  root package `fractality/v0.1.0/` carrying the spec corpus (PROP-001
  foundation, the campaign plan with Phase 0 findings, refs inventory,
  and `spec/refs/notes/{codex-first-study,landscape}.md`). No crates yet
  — Phase 1 creates the Cargo workspace.
- Host wiring done: `WORKSPACES.md` registry at the host root, workspace
  grammar in the host contracts, `flow:org.vibevm/wal-workspaces` 0.1.0
  authored as the canon.
- **Phase 0 facts now trusted (don't re-probe):** z.ai base URL +
  `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL` mapping + `glm-5.2[1m]`/
  `glm-5-turbo` ids; fresh `CLAUDE_CONFIG_DIR` onboards headless;
  `win32job` KILL_ON_JOB_CLOSE reaps the tree AND survives pod-exit; CC
  `permissionDecision`/`defer` surface confirmed; **rustc 1.93.1** on
  this box → `sysinfo =0.37.2`. Full detail: plan §8 findings F1–F10.

## Active blocker

None. Phase 0 is done and green. The only open owner decision is RP3
(publish), which blocks nothing; RP1/RP2/RP4 are RESOLVED. One Phase-1
opening step needs an owner note if preferred: rustc 1.93.1 vs a
toolchain bump (the plan defaults to pinning `sysinfo =0.37.2` and a
`rust-version` floor — no bump required).

## Next-steps recipe (cold start)

1. Boot: host Rules 1–4 → this directory's `CLAUDE.md` → `WAL.md` → this
   file → the plan.
2. Read the plan top to bottom:
   `fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md` —
   status is `EXECUTING`, Phase 0 findings F1–F10 are in §8.
3. Run its §11 quick-start block; confirm the tree matches.
4. Execute **Phase 1** (workspace skeleton + mission-control core): create
   the six-crate Cargo workspace in `fractality/v0.1.0/`, set a
   `rust-version` floor and pin `sysinfo =0.37.2` (F9), build the core
   model + MC lifecycle + pod skeleton per the plan's Phase 1 steps. Green
   floor at the boundary.

## Quick-start

```sh
cd packages/org.vibevm.fractality
head -20 WAL.md                      # state + next
# floor before crates exist: host self-check must stay green
bash tools/self-check.sh; echo "EXIT=$?"    # run from host root
```

Resume phrase: `восстанови сессию fractality` / `RESUME SESSION fractality`
(report-then-wait). Wind-down: `заверши сессию fractality` /
`END SESSION fractality`.
