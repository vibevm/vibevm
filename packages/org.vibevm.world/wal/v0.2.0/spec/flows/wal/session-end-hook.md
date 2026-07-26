# Session-end hook — the wind-down {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The procedure every session ends with:
confirm a good stopping state, rewrite `spec/WAL.md`, overwrite
`CONTINUE.md`, report. @impl/done

##also-defines-the-trigger-phrases It also defines the trigger phrases that invoke
the full wind-down explicitly. @impl/done

##THE-HOOK-TAKES-UNDER-TWO-MINUTES For a well-scoped session the whole hook
takes under two minutes; if it takes longer, the session did too much. @impl/done

## When the hook fires {#when}

##two-ways-in-lead Two ways in: @impl/done

- ##HOOK-FIRES-IMPLICITLY **Implicitly.** Every session that touched project state ends with at
  least steps 1–3. A session that ends without updating the WAL has
  partially broken the next session's context. @impl/done
- ##HOOK-FIRES-EXPLICITLY **Explicitly.** The user issues a wind-down phrase. Ship defaults:
  `END SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`. Recognise the
  intent, not the exact wording — `FINISH SESSION` or `WRAP UP SESSION`
  must fire too. A project may add native-language twins in its agent
  instructions; the origin project of this flow runs a bilingual
  Russian/English set. @impl/done

##AN-EXPLICIT-WIND-DOWN-MEANS-A-FRESH-CONTEXT-FOLLOWS An explicit wind-down means the user is about to close the conversation
and may continue from a fresh context — a new session, another machine,
a different agent. @impl/done

##RUN-THE-FULL-HOOK-AS-A-HARD-CONTRACT Run the full hook, steps 1–6, and treat it as a hard
contract, not a courtesy: its purpose is to make session-boundary
loss-of-context cheap. @impl/done

## 1. Confirm the work is in a good stopping state {#stopping-state}

- ##STOPPING-STATE-TESTS-STILL-PASS Tests that were passing are still passing. @impl/done
- ##STOPPING-STATE-GENERATED-FILES-EXIST Files that were supposed to be generated exist. @impl/done
- ##STOPPING-STATE-NO-HALF-APPLIED-REFACTORS No half-applied refactors are left sitting in the working tree unless
  the user explicitly chose to pause mid-flight. @impl/done

##SAY-SO-RATHER-THAN-PAPER-OVER-A-BROKEN-STATE If any of these fails, say so explicitly at the end of the session — do
not silently paper over a broken state in the WAL. @impl/done

## 2. Rewrite `spec/WAL.md` {#rewrite}

##WAL-IS-A-CHECKPOINT-NOT-AN-APPEND-ONLY-LOG The WAL is a checkpoint, not an append-only log. @impl/done

##REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND **Rewrite** the file —
don't patch it, don't append to it. @impl/done

##an-append-only-wal-rots An append-only WAL rots into an
archive nobody reads; the rewritten file always describes *now*. @spec/done

##structure-lead Structure (from [`WAL-PROTOCOL.md`](WAL-PROTOCOL.md#sections)): @impl/done

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

- ##COLLAPSE-DONE-TO-ONE-LINE-EACH A long "Done" section with implementation notes is a bug. Collapse
  each completed unit into a single line; the details live in commits. @impl/done
- ##MOVE-BALLOONED-MATERIAL-INTO-A-SPEC Anything that ballooned into multiple paragraphs probably belongs in
  a spec, not in the WAL. Move it out, leave a short pointer. @impl/done

## 4. Overwrite `CONTINUE.md` {#continue}

##OVERWRITE-CONTINUE-WHOLESALE-ON-A-WIND-DOWN On an explicit wind-down — and at any session end that precedes a
machine switch or a long gap — overwrite `CONTINUE.md` at the
repository root, wholesale, with the cold-resume snapshot. @impl/done

##NEVER-APPEND-STALENESS-COMPOUNDS Never
append; staleness compounds. @impl/done

##required-contents-pointer The required contents (TL;DR, where work
stands, blocker and unblocking action, next-steps recipe, non-obvious
findings, repository map, standing decisions, recent commits,
quick-start commands) are specified in
[`cold-resume.md`](cold-resume.md#contract). @impl/done

## 5. Commit — propose by default {#propose}

##SURFACE-THE-CONTENT-AS-DRAFTS Surface the proposed WAL and `CONTINUE.md` content to the user as
*drafts*, plus any milestone commit. @impl/done

##DO-NOT-COMMIT-AUTOMATICALLY Do not commit automatically —
unless the project's standing instructions grant that autonomy, which
many projects do for routine checkpoint commits. @impl/done

##where-autonomy-is-granted-lead Where autonomy is
granted: @impl/done

- ##GROUP-COMMITS-BY-TOPIC group commits by topic, never by time of edit: the WAL update and
  the snapshot are checkpoint commits, separate from code commits; @impl/done
- ##PUSH-ONLY-UNDER-SANCTION push only if the project's autonomy rules sanction pushing; when in
  doubt, stop at the commit and say so. @impl/done

## 6. Report {#report}

##end-of-session-report-lead Emit a short end-of-session report in the chat: @impl/done

- ##REPORT-WHAT-CHANGED What changed (in specs, code, tests). @impl/done
- ##REPORT-DECISIONS-AND-WHY What decisions were made this session and why. @impl/done
- ##REPORT-OPEN-REVIEW-MARKERS Any open REVIEW markers the user should look at. @impl/done
- ##REPORT-KNOWN-ISSUES-DISCOVERED Any Known Issue that was discovered. @impl/done

##EXTEND-THE-REPORT-INTO-A-WIND-DOWN-TLDR On an explicit wind-down, extend the report into a TL;DR of what the
wind-down did: which files were written or updated, which commits were
created, push status, and what the next session should pick up first. @impl/done

##one-screen-but-verifiable Short enough to scan on one screen; detailed enough that the user can
verify nothing was missed without opening the files. @impl/done

##THE-REPORT-IS-FOR-THE-HUMAN The report is for the human's quick scan. @impl/done

##THE-WAL-IS-FOR-THE-NEXT-SESSIONS-AGENT The WAL is for the next
session's agent. @impl/done

##they-serve-different-readers They serve different readers. @impl/done

## Never {#never}

- ##NEVER-APPEND-TO-THE-WAL Never append to the WAL. Rewrite it; the previous version lives in
  git history. @impl/done
- ##NEVER-PAPER-OVER-A-BROKEN-STOPPING-STATE Never paper over a broken stopping state — a red test suite recorded
  as green poisons every following session. @impl/done
- ##NEVER-LEAVE-THE-UPDATED-LINE-UNTOUCHED Never leave the `_Updated:` line untouched while editing the rest. @impl/done
- ##NEVER-PUSH-ON-A-WIND-DOWN-WITHOUT-SANCTION Never push on a wind-down unless the project's standing rules
  sanction it. @impl/done
- ##NEVER-SKIP-THE-CONTINUE-OVERWRITE Never skip the `CONTINUE.md` overwrite on an explicit wind-down —
  that is the half of the contract the cold reader depends on. @impl/done

## Summary {#summary}

- ##SUM-WHEN-THE-HOOK-RUNS The hook runs at every session end; a wind-down phrase (`END
  SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`, project twins) invokes
  it in full. @impl/done
- ##SUM-THE-STEPS-IN-ORDER Confirm the stopping state honestly; rewrite the WAL; collapse
  history out of it; overwrite `CONTINUE.md`; report. @impl/done
- ##SUM-PROPOSE-DRAFTS-BY-DEFAULT Propose drafts by default; commit and push only under standing
  autonomy, in topic-grouped commits. @impl/done
- ##SUM-UNDER-TWO-MINUTES Under two minutes for a well-scoped session. @impl/done
