# `flow:wal` — Write-Ahead Log discipline

A vibevm `flow` package that installs the WAL protocol into a project. The
WAL (Write-Ahead Log) is a short checkpoint file (`spec/WAL.md`) that bridges
AI sessions: the AI has no memory between invocations, so the WAL is the
only structured record of "where we are" that survives session restarts.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/wal/WAL-PROTOCOL.md` — full protocol: what WAL is, what it
  isn't, the required sections, the size budget, the conflict rule.
- `spec/flows/wal/session-end-hook.md` — step-by-step procedure for the AI
  to follow at the end of every session.
- `spec/flows/wal/morning-routine.md` — the human counterpart: a 5-minute
  ritual at the start of each day that keeps the AI's read of the state
  synchronised with your head.
- `spec/boot/10-flow-wal.md` — a boot snippet read by AI agents at session
  start, pointing them at the protocol and the WAL file itself.

## Install

```bash
vibe install flow:wal
```

## Uninstall

```bash
vibe uninstall flow:wal
```

Uninstalling removes every file the package wrote, including the boot
snippet, but NEVER touches `spec/WAL.md` (that's project state, not package
state) or user-owned boot files (`00-core.md`, `90-user.md`).

## Philosophical background

The underlying ideas — two-process cooperation, files-as-IPC, WAL as the
only memory that survives session boundaries — are described in chapters 1-3
of *AI-native development* (see `refs/book/` in the vibevm source tree).
Short version:

- An AI session has no persistent memory. Every morning it wakes up blank.
- The only things that survive are files.
- A well-written WAL tells the AI (and your future self) where work stopped,
  what is safe to change, and what is explicitly off-limits.
- Without that discipline, the AI re-derives context from scratch every
  session — wasting tokens, missing constraints, drifting from intent.

This package encodes that discipline as plain Markdown and a tiny boot
snippet. No magic, no dependency on any particular agent product.

## License

EULA. See the surrounding registry for distribution terms.
