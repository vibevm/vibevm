# `flow:wal` — Write-Ahead Log discipline {#root}

A `flow` package that installs the WAL protocol into a project. The WAL
(Write-Ahead Log) is a short checkpoint file (`spec/WAL.md`) that
bridges sessions: a coding agent has no memory between invocations, so
the WAL is the only structured record of "where we are" that survives
session restarts. Since 0.2.0 the discipline spans two files: the
living checkpoint, plus `CONTINUE.md` — a cold-resume snapshot for
whoever picks up cold.

**This package is the canonical home of the WAL convention.** The
AI-Native Discipline (`flow:core-ai-native`) ships a convention
document that defers to this package from its next release.

The package ships five pieces of content plus a skill:

- `spec/flows/wal/WAL-PROTOCOL.md` — full protocol: the two files,
  what a WAL is and isn't, required sections, agent-grade precision,
  update triggers, freshness, size budget, the conflict rule, the
  acceptance test, and a re-derivation prompt.
- `spec/flows/wal/session-end-hook.md` — the wind-down: the
  step-by-step session-end procedure and the trigger phrases that
  invoke it explicitly.
- `spec/flows/wal/morning-routine.md` — the human counterpart: a
  five-minute ritual at the start of each day that keeps the agent's
  read of the state synchronised with your head.
- `spec/flows/wal/cold-resume.md` — the `CONTINUE.md` contract and
  the wind-down / resume session commands.
- `spec/boot/10-flow-wal.md` — boot snippet read by agents at session
  start, pointing them at the protocol and the WAL itself.
- `spec/skills/wal-status/` — an installable agent skill: the
  ten-line orientation read of the WAL, staleness warning first.

## Install {#install}

```bash
vibe install flow:wal
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:wal
```

Uninstalling removes every file the package wrote, including the boot
snippet and the skill, but NEVER touches `spec/WAL.md` or `CONTINUE.md`
(project state, not package state) or user-owned boot files
(`00-core.md`, `90-user.md`).

## What changed in 0.2.0 {#changelog}

- **The two-file model**, absorbed from the AI-Native Discipline's WAL
  convention: `spec/WAL.md` is the canonical living checkpoint;
  `CONTINUE.md` at the repository root is the cold-resume snapshot it
  supersedes.
- **Session commands.** The wind-down (`END SESSION`, `WRAP UP`,
  `CHECKPOINT AND CLOSE`) and resume (`RESUME SESSION`, `RESTORE
  CONTEXT`) contracts: overwrite-wholesale snapshot on wind-down;
  restore, report, and stop on resume.
- **The `wal-status` skill.** The fast skim is now an installable
  skill rather than a hand-pasted prompt snippet.
- **`spec/` layout.** The boot snippet moved from `boot/` to
  `spec/boot/`, matching the current package layout.

## Composition {#composition}

- `flow:sync-from-code` (`20-…`) and `flow:atomic-commits` (`30-…`):
  numeric boot-snippet prefixes are distinct by design. A sync may
  trigger a WAL update; checkpoint commits follow the project's
  commit discipline.
- `flow:conflict-protocol` — the authority hierarchy (Human > Spec >
  Tests > Code > WAL), including the WAL's place at its bottom, is
  defined there.
- `flow:campaign-plans` — a campaign checkpoint updates the WAL at
  phase boundaries.

## Philosophical background {#background}

The underlying ideas — two-process cooperation, files as the IPC
between human and agent, the memory hierarchy (head → WAL → spec →
code), WAL as the only memory that survives session boundaries — are
chapters 2–3 of *AI-native development*, shipped in Russian inside
`flow:redbook` at `spec/book/ru/`. Short version:

- An agent session has no persistent memory. Every morning it wakes
  up blank; the only things that survive are files.
- A well-written WAL tells the agent (and your future self) where
  work stopped, what is safe to change, and what is explicitly
  off-limits.
- Without that discipline, the agent re-derives context from scratch
  every session — wasting tokens, missing constraints, drifting from
  intent.

This package encodes that discipline as plain Markdown, a tiny boot
snippet, and one skill. No magic, no dependency on any particular
agent product.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
