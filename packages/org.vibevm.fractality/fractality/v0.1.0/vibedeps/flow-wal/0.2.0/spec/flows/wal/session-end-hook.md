# Session-end hook — the wind-down {#root}

**Scope of this document.** The procedure every session ends with:
confirm a good stopping state, rewrite `spec/WAL.md`, overwrite
`CONTINUE.md`, report. It also defines the trigger phrases that invoke
the full wind-down explicitly. For a well-scoped session the whole hook
takes under two minutes; if it takes longer, the session did too much.

## When the hook fires {#when}

Two ways in:

- **Implicitly.** Every session that touched project state ends with at
  least steps 1–3. A session that ends without updating the WAL has
  partially broken the next session's context.
- **Explicitly.** The user issues a wind-down phrase. Ship defaults:
  `END SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`. Recognise the
  intent, not the exact wording — `FINISH SESSION` or `WRAP UP SESSION`
  must fire too. A project may add native-language twins in its agent
  instructions; the origin project of this flow runs a bilingual
  Russian/English set.

An explicit wind-down means the user is about to close the conversation
and may continue from a fresh context — a new session, another machine,
a different agent. Run the full hook, steps 1–6, and treat it as a hard
contract, not a courtesy: its purpose is to make session-boundary
loss-of-context cheap.

## 1. Confirm the work is in a good stopping state {#stopping-state}

- Tests that were passing are still passing.
- Files that were supposed to be generated exist.
- No half-applied refactors are left sitting in the working tree unless
  the user explicitly chose to pause mid-flight.

If any of these fails, say so explicitly at the end of the session — do
not silently paper over a broken state in the WAL.

## 2. Rewrite `spec/WAL.md` {#rewrite}

The WAL is a checkpoint, not an append-only log. **Rewrite** the file —
don't patch it, don't append to it. An append-only WAL rots into an
archive nobody reads; the rewritten file always describes *now*.

Structure (from [`WAL-PROTOCOL.md`](WAL-PROTOCOL.md#sections)):

```
# WAL — Project Continuation State
_Updated: <ISO 8601 UTC — right now>_

## Current phase
<what the project is actually doing at this moment>

## Constraints (do not violate without discussion)
- <short line with a brief *why*, citing spec anchors where possible>

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

- A long "Done" section with implementation notes is a bug. Collapse
  each completed unit into a single line; the details live in commits.
- Anything that ballooned into multiple paragraphs probably belongs in
  a spec, not in the WAL. Move it out, leave a short pointer.

## 4. Overwrite `CONTINUE.md` {#continue}

On an explicit wind-down — and at any session end that precedes a
machine switch or a long gap — overwrite `CONTINUE.md` at the
repository root, wholesale, with the cold-resume snapshot. Never
append; staleness compounds. The required contents (TL;DR, where work
stands, blocker and unblocking action, next-steps recipe, non-obvious
findings, repository map, standing decisions, recent commits,
quick-start commands) are specified in
[`cold-resume.md`](cold-resume.md#contract).

## 5. Commit — propose by default {#propose}

Surface the proposed WAL and `CONTINUE.md` content to the user as
*drafts*, plus any milestone commit. Do not commit automatically —
unless the project's standing instructions grant that autonomy, which
many projects do for routine checkpoint commits. Where autonomy is
granted:

- group commits by topic, never by time of edit: the WAL update and
  the snapshot are checkpoint commits, separate from code commits;
- push only if the project's autonomy rules sanction pushing; when in
  doubt, stop at the commit and say so.

## 6. Report {#report}

Emit a short end-of-session report in the chat:

- What changed (in specs, code, tests).
- What decisions were made this session and why.
- Any open REVIEW markers the user should look at.
- Any Known Issue that was discovered.

On an explicit wind-down, extend the report into a TL;DR of what the
wind-down did: which files were written or updated, which commits were
created, push status, and what the next session should pick up first.
Short enough to scan on one screen; detailed enough that the user can
verify nothing was missed without opening the files.

The report is for the human's quick scan. The WAL is for the next
session's agent. They serve different readers.

## Never {#never}

- Never append to the WAL. Rewrite it; the previous version lives in
  git history.
- Never paper over a broken stopping state — a red test suite recorded
  as green poisons every following session.
- Never leave the `_Updated:` line untouched while editing the rest.
- Never push on a wind-down unless the project's standing rules
  sanction it.
- Never skip the `CONTINUE.md` overwrite on an explicit wind-down —
  that is the half of the contract the cold reader depends on.

## Summary {#summary}

- The hook runs at every session end; a wind-down phrase (`END
  SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`, project twins) invokes
  it in full.
- Confirm the stopping state honestly; rewrite the WAL; collapse
  history out of it; overwrite `CONTINUE.md`; report.
- Propose drafts by default; commit and push only under standing
  autonomy, in topic-grouped commits.
- Under two minutes for a well-scoped session.
