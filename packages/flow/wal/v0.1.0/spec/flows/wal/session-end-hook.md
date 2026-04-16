# Session end hook — how to update the WAL {#root}

Every session ends with this procedure. It should take under two minutes
for a well-scoped session. If it takes longer, the session did too much.

## 1. Confirm the work is in a good stopping state {#stopping-state}

- Tests that were passing are still passing.
- Files that were supposed to be generated exist.
- No half-applied refactors are left sitting in the working tree unless
  the user explicitly chose to pause mid-flight.

If any of these fails, say so explicitly at the end of the session — do
not silently paper over a broken state in the WAL.

## 2. Rewrite `spec/WAL.md` {#rewrite}

The WAL is a checkpoint, not an append-only log. **Rewrite** the file —
don't patch it.

Structure (from `spec/flows/wal/WAL-PROTOCOL.md`):

```
# WAL — Project Continuation State
_Updated: <ISO 8601 UTC — right now>_

## Current phase
<what the project is actually doing at this moment>

## Constraints (do not violate without discussion)
- <short line with a brief *why*, citing spec anchors where possible>
- ...

## Done
- [x] <one-line collapsed summary of completed things>

## In progress
- <what is partially done, with enough context to resume>

## Next
<single next action>

## Known issues
- <open problem we chose not to address right now>

## Session context
<what to open / run / avoid at the start of next session>
```

## 3. Collapse aggressively {#collapse}

- A long "Done" section with implementation notes is a bug. Collapse each
  completed PROP/FEAT into a single line. The details live in commits.
- Anything that ballooned into multiple paragraphs probably belongs in a
  spec, not in the WAL. Move it out, leave a short pointer in the WAL.

## 4. Propose, don't commit {#propose}

When running as an AI agent, write the proposed WAL content and surface
it to the user as *a draft*, plus any milestone commit. Do not commit
automatically unless the user's standing instructions say otherwise.

## 5. Report {#report}

Emit a short end-of-session report:

- What changed (in specs, code, tests).
- What decisions were made in this session and why.
- Any open REVIEW markers the user should look at.
- Any Known Issue that was discovered.

The report is for the human's quick scan. The WAL is for the next
session's AI. They serve different readers.
