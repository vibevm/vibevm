---
name: wal-status
description: Read the project WAL end to end and emit a ten-line orientation — current phase, attention items, next step — warning first when the WAL is stale. Use at session start or whenever the user asks where things stand.
---

<status stage="impl" state="done"/>

# WAL status — the ten-line orientation {#root}

##PRODUCING-THE-FAST-MORNING-READ You are producing the fast morning read of the project WAL — the skim
form of the ritual in `spec/flows/wal/morning-routine.md`. @impl/done

## Procedure {#procedure}

1. ##READ-THE-WAL-END-TO-END Read `spec/WAL.md` end to end. If the file does not exist, say so,
   point at `spec/flows/wal/WAL-PROTOCOL.md`, and stop. @impl/done
2. ##STALENESS-WARNING-IS-THE-FIRST-OUTPUT-LINE Check the `_Updated:` line (the first line after the title). Older
   than 24 hours? The warning is your FIRST output line:
   `WARNING: WAL updated <N> hours ago — may be stale.` @impl/done
3. ##FLAG-A-CONTINUE-DIVERGENCE-AND-TREAT-THE-WAL-AS-CANONICAL If `CONTINUE.md` exists at the repository root and disagrees with
   the WAL on phase, blockers, or next step, flag the divergence and
   treat the WAL as canonical. @impl/done

## Output — at most ten lines {#output}

- ##OUTPUT-ONE-LINE-CURRENT-PHASE One line: current phase and its status. @impl/done
- ##OUTPUT-UP-TO-THREE-ATTENTION-BULLETS Up to three bullets: what needs attention (blockers, risks, pending
  decisions). @impl/done
- ##OUTPUT-ONE-LINE-NEXT-PRIORITY-STEP One line: the next priority step. @impl/done

##example-shape-lead Example shape: @impl/done

```
WARNING: WAL updated 26 hours ago — may be stale.

Phase: PROP-003 verification engine, ~70% done

Needs attention:
- reconcile_pending() is a stub, blocked by issue #12
- media_refs protobuf schema (#15) — needed before PROP-004

Next step: resolve #12, or start PROP-004 in parallel.
```

##REPORT-ONLY-DO-NOT-EDIT-OR-EXECUTE Report only: do not edit the WAL, and do not start the next step. @impl/done
