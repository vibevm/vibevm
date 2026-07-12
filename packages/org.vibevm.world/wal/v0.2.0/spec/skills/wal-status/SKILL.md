---
name: wal-status
description: Read the project WAL end to end and emit a ten-line orientation — current phase, attention items, next step — warning first when the WAL is stale. Use at session start or whenever the user asks where things stand.
---

# WAL status — the ten-line orientation {#root}

You are producing the fast morning read of the project WAL — the skim
form of the ritual in `spec/flows/wal/morning-routine.md`.

## Procedure {#procedure}

1. Read `spec/WAL.md` end to end. If the file does not exist, say so,
   point at `spec/flows/wal/WAL-PROTOCOL.md`, and stop.
2. Check the `_Updated:` line (the first line after the title). Older
   than 24 hours? The warning is your FIRST output line:
   `WARNING: WAL updated <N> hours ago — may be stale.`
3. If `CONTINUE.md` exists at the repository root and disagrees with
   the WAL on phase, blockers, or next step, flag the divergence and
   treat the WAL as canonical.

## Output — at most ten lines {#output}

- One line: current phase and its status.
- Up to three bullets: what needs attention (blockers, risks, pending
  decisions).
- One line: the next priority step.

Example shape:

```
WARNING: WAL updated 26 hours ago — may be stale.

Phase: PROP-003 verification engine, ~70% done

Needs attention:
- reconcile_pending() is a stub, blocked by issue #12
- media_refs protobuf schema (#15) — needed before PROP-004

Next step: resolve #12, or start PROP-004 in parallel.
```

Report only: do not edit the WAL, and do not start the next step.
