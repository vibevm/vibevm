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

- Workspace bootstrapped: contract (`CLAUDE.md`), WAL, this file, and the
  root package `fractality/v0.1.0/` carrying the spec corpus (PROP-001
  foundation, the campaign plan, the refs inventory). No crates yet —
  Phase 1 creates the Cargo workspace.
- Host wiring done: `WORKSPACES.md` registry at the host root, workspace
  grammar in the host contracts, `flow:org.vibevm/wal-workspaces` 0.1.0
  authored as the canon.

## Active blocker

None. The next work needs no owner input until Phase 0 findings arrive
(the plan's review points RP1–RP4 are the open owner decisions; none blocks
Phase 0).

## Next-steps recipe (cold start)

1. Boot: host Rules 1–4 → this directory's `CLAUDE.md` → `WAL.md` → this
   file → the plan.
2. Read the plan top to bottom:
   `fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`.
3. Run its §11 quick-start block; confirm the status line still says
   `PLANNED` and the tree matches.
4. Execute Phase 0 exactly as written (spikes s1–s9; no commits; findings
   rewrite Decisions in place; flip status to `EXECUTING` on the amendment
   commit).

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
