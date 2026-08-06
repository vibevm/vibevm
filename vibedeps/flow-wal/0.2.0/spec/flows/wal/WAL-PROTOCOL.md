# WAL Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** What a WAL is in a spec-driven project,
which files carry the discipline, what belongs in the WAL, how big it
may get, when it must be updated, and who wins when it disagrees with
the rest of the system. @status:impl/done

@fact:sibling-document-pointers The rituals around it live in the siblings:
[`session-end-hook.md`](session-end-hook.md), [`morning-routine.md`](morning-routine.md), [`cold-resume.md`](cold-resume.md). @status:impl/done

## The two files {#files}

@fact:two-files-lead Session-durable state lives in two repository files: @status:impl/done

- @fact:FILE-WAL-IS-THE-CANONICAL-LIVING-CHECKPOINT **`spec/WAL.md` — the living checkpoint.** The *current* state:
  phase, constraints, done, open, next. Rewritten at every session end.
  **Canonical:** where it and any snapshot disagree, the WAL wins. @status:impl/done
- @fact:FILE-CONTINUE-IS-THE-SUBORDINATE-COLD-RESUME-SNAPSHOT **`CONTINUE.md` (repository root) — the cold-resume snapshot.**
  Written at session end for whoever picks up cold — another machine, a
  teammate, a long gap. Richer than the WAL, strictly subordinate.
  Contract: [`cold-resume.md`](cold-resume.md). @status:impl/done

@fact:RESUMPTION-STATE-LIVES-IN-THE-REPOSITORY-NEVER-IN-A-SESSION One invariant governs both: **resumption state lives in the repository,
never in a session.** @status:impl/done

@fact:sessions-end-compact-and-crash Sessions end, compact, and crash; the repository
outlives them all. @status:spec/done

@fact:rest-of-the-document-specifies-the-wal The rest of this document specifies the WAL. @status:impl/done

## What a WAL is {#what}

@fact:WAL-STANDS-FOR-WRITE-AHEAD-LOG WAL stands for **Write-Ahead Log** — a name any developer, and any
coding agent from its training data alone, recognises on sight. @status:spec/done

@fact:THE-MECHANISM-IS-THE-CHECKPOINT-NOT-THE-LOG But the
mechanism that fits is not the ever-growing database log; it is the
database **checkpoint**: a snapshot of current state, *overwritten*,
not appended to — the old one is meaningless once a new one exists. @status:impl/done

@fact:picture-a-whiteboard-by-the-door Picture a whiteboard by the door. @status:spec/done

@fact:a-good-whiteboard-and-a-bad-one A good one carries three lines:
where we are, what we are doing, what must not be touched; a bad one
carries three hundred lines of stale history, including tasks closed
months ago. @status:spec/done

@fact:WAL-IS-A-WHITEBOARD-NOT-A-JOURNAL **The WAL is a whiteboard, not a journal.** @status:impl/done

@fact:file-serves-both-processes The file serves both processes in the human-AI system. @status:impl/done

@fact:FOR-THE-AGENT-IT-IS-THE-ONLY-PERSISTENT-MEMORY For the
**agent** it is the canonical persistent memory: each session starts
blank, and the WAL is read first. @status:impl/done

@fact:FOR-THE-HUMAN-IT-IS-THE-SINGLE-MORNING-RE-READ For the **human** it is the single morning
re-read that synchronises your memory with the project's actual state. @status:impl/done

@fact:THIS-FILE-ALONE-EXISTS-FOR-SESSION-CONTINUITY Everything else in `spec/` is medium-term memory or artefacts; this
file alone exists for session continuity. @status:impl/done

## What a WAL is NOT {#not}

- @fact:NOT-A-LOG **Not a log.** Rewritten every session, always describing the
  *current* state; an append-only WAL rots into an archive nobody reads. @status:impl/done
- @fact:NOT-A-CHANGELOG **Not a changelog.** History belongs in `git log` and spec changelogs. @status:impl/done
- @fact:NOT-A-TO-DO-LIST **Not a to-do list.** The "Next" section names *one* next action; the
  backlog belongs in an issue tracker. @status:impl/done
- @fact:NOT-DOCUMENTATION **Not documentation.** A fact stable enough to deserve a permanent
  home belongs in a spec document, not in the WAL. @status:impl/done

## Required sections {#sections}

@fact:required-sections-lead A well-formed `spec/WAL.md` has: @status:impl/done

1. @fact:SECTION-UPDATED-LINE **`_Updated: <ISO 8601 UTC>_`** — the very first line after the
   title, always and without exception. An outdated WAL is worse than
   no WAL — false confidence; the date makes staleness detectable fast. @status:impl/done
2. @fact:SECTION-CURRENT-PHASE **Current phase** — one or two lines naming what the project is
   working on right now. @status:impl/done
3. @fact:SECTION-CONSTRAINTS **Constraints** — the "do not touch" list; each entry carries a
   brief *why*, citing a spec anchor or issue: `match_by_hash(): DO NOT
   TOUCH — fragile reconnection logic, issue #12`. The most valuable
   content in the WAL: agents optimise locally, and without the written
   line an enthusiastic session "improves" what was deliberately left
   alone. Every constraint is a prevented regression. @status:impl/done
4. @fact:SECTION-DONE **Done** — completed work collapsed to one-line summaries:
   "PROP-001 §§1-4 — complete, all tests pass", not five lines of notes. @status:impl/done
5. @fact:SECTION-IN-PROGRESS **In progress** — what is open, with enough detail that the next
   session can pick it up. Cite spec anchors (`spec://…`). @status:impl/done
6. @fact:SECTION-NEXT **Next** — the single next action. Several candidates? List them
   briefly and mark the default. @status:impl/done
7. @fact:SECTION-KNOWN-ISSUES **Known issues** — open problems not being worked on right now, so
   future sessions don't re-discover them. @status:impl/done
8. @fact:SECTION-SESSION-CONTEXT **Session context** *(optional)* — one-paragraph orientation: what
   file to open first, what to run, what to avoid. @status:impl/done

## Write for the autonomy you grant {#precision}

@fact:in-chat-vagueness-gets-a-question-back In chat, vague writing gets a question back. @status:spec/done

@fact:IN-AGENT-MODE-THE-WAL-IS-EXECUTED-LITERALLY In agent mode — unattended
for hours — nobody asks; the WAL is executed literally. Compare: @status:spec/done

```markdown
# Chat-grade WAL (tolerable when a human steers every step)
Working on verification; open questions around the timeouts.

# Agent-grade WAL (executed literally, unattended)
## Constraints — do not violate
- Timeout: 600s, not 300s → spec://oproto/PROP-003#verification.timeout
- match_by_hash(): DO NOT TOUCH → issue #12, fragile reconnection logic
## Current task: src/verify.rs → degraded_mode_handler()
Run: cargo test --package oproto-verify. Done when: all tests green.
```

@fact:the-second-is-not-bureaucracy The second is not bureaucracy: it is the difference between an agent
that quietly "optimises" the timeout back to 300 s and one that does
what is needed. @status:spec/done

@fact:MORE-AUTONOMY-DEMANDS-A-MORE-PRECISE-WAL **The more autonomy the agent has, the more precise the
WAL must be** — coarse context suffices for one supervised step; hours
unattended demand every constraint explicit and spec-anchored. @status:impl/done

## Update triggers {#triggers}

- @fact:TRIGGER-END-OF-EVERY-SESSION **At the end of every session.** Non-negotiable; a session that ends
  without updating the WAL has partially broken the next session.
  Procedure: [`session-end-hook.md`](session-end-hook.md). @status:impl/done
- @fact:TRIGGER-BEFORE-ANY-DESTRUCTIVE-OPERATION **Before any destructive operation.** A big refactor, a `git reset`,
  a migration — checkpoint first; a failure then restarts known-good. @status:impl/done
- @fact:TRIGGER-WHEN-SWITCHING-CONTEXT-MID-SESSION **When switching context mid-session.** Pivoting from FEAT-007 to
  FEAT-010? Checkpoint FEAT-007 *before* pivoting, or its state is lost. @status:impl/done

## The freshness rule {#freshness}

@fact:WAL-OLDER-THAN-24-HOURS-IS-PRESUMED-STALE A WAL older than **24 hours** is presumed stale: before trusting any
claim — and before *any* destructive work — verify against reality
(branch, tree, tests) and confirm divergences with the user. @status:impl/done

@fact:THE-RULE-BINDS-WHATEVER-THE-TOOLING-DOES Tooling
may warn advisorily; the rule binds regardless. @status:impl/done

## Size budget {#size}

@fact:WAL-IS-LOADED-AT-THE-START-OF-EVERY-SESSION The WAL is loaded into the agent's context at the start of every
session. @status:impl/done

@fact:EVERY-TOKEN-IS-A-TAX-PAID-PER-SESSION Every token in the WAL is a tax paid per session. @status:spec/done

- @fact:BUDGET-TARGET **Target:** ≤ 3 000 tokens (roughly one page of plain text). @status:impl/done
- @fact:BUDGET-HARD-LIMIT **Hard limit:** 5 000 tokens — past that the WAL hoards history:
  collapse done items, move stable facts to specs, split epics. @status:impl/done
- @fact:BUDGET-RULE-OF-THUMB **Rule of thumb:** unreadable end-to-end in a minute means too long. @status:impl/done

## Conflict resolution {#conflicts}

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

@fact:WAL-IS-VOLATILE-BY-DESIGN The WAL is volatile by design — a *record* of the current state, not a
source of truth about intent. @status:impl/done

@fact:SPEC-WINS-AND-THE-WAL-IS-STALE If the WAL says one thing and a spec
document says another, the spec wins and the WAL is stale. @status:impl/done

@fact:SURFACE-A-WRONG-WAL-ENTRY-NEVER-SILENTLY-REWRITE-IT An agent that believes a WAL entry is wrong does not silently rewrite
it: complete the task following the spec (not the stale WAL), add a
`<!-- REVIEW: … -->` marker where the mismatch matters, surface the
disagreement in the end-of-session report, and let the human reconcile. @status:impl/done

## A working example {#example}

```markdown
# WAL — Project Continuation State
_Updated: 2026-04-16T18:23:00Z_

## Current phase
PROP-003 verification engine — IN PROGRESS (~70%)

## Constraints (do not violate without discussion)
- Timeout = 600s (spec://oproto/PROP-003#verification.timeout);
  300s caused false positives on VPN users.
- match_by_hash(): DO NOT TOUCH — fragile reconnection logic, #12.

## Done
- [x] PROP-001 OPROTO base; PROP-002 edge skeleton (47/52 tests).

## In progress
- PROP-003: hash matcher and basic timeout DONE; degraded mode TODO
  (spec://oproto/PROP-003#verification.degraded).

## Next
src/verify.rs → degraded_mode_handler().
Start with: `cargo test -p oproto-verify`.

## Known issues
- reconnection after net loss (#12); media_refs schema ambiguity (#15).
```

## The acceptance test {#acceptance}

@fact:ACCEPTANCE-TEST-IS-A-STRANGER-WITH-ONLY-THE-REPOSITORY One test decides whether the discipline works: **a stranger with only
the repository resumes work without asking.** @status:impl/done

@fact:a-stranger-cold-from-files-alone A stranger, cold, from
files alone — not the author remembering, not the agent that wrote it
reconstructing. @status:impl/done

@fact:APPLY-THE-TEST-WHEN-IN-DOUBT When in doubt what to record or where, apply it. @status:impl/done

## Appendix: without a WAL {#without}

@fact:A-PROJECT-MAY-DECLINE-THIS-CONVENTION A project may decline this convention. @status:impl/done

@fact:THE-INVARIANT-BINDS-AND-THE-FALLBACK-IS-GIT-LOG The invariant still binds —
resumption state lives in the repository, never in a session — and the
fallback resume path is `git log` plus a fresh look at the tree: commit
messages carry session summaries, a plan's status line carries campaign
state, each session re-derives the rest. @status:impl/done

@fact:the-cost-of-the-fallback-path That works; the cost is three
places instead of one, no single canonical "now", tokens burned every
session on re-derivation — and constraints missed first. @status:spec/done

@fact:RE-DERIVING-STATE-IS-THE-SIGNAL-TO-ADOPT A project that
keeps re-deriving its own state has received the signal to adopt. @status:spec/done

## Re-derive for your project {#re-derive}

@fact:COPY-THE-TASK-NOT-THE-IMPLEMENTATION Copy the task, not the implementation. @status:impl/done

@fact:re-derive-prompt-lead Hand your agent this prompt and
review its drafts: @status:impl/done

```text
Read this flow's documents (your project installed them — typically `vibedeps/flow-wal/<version>/spec/flows/wal/`, check `vibe.lock`) WAL-PROTOCOL.md end to end, then adapt the WAL
discipline to this repository:
1. Draft spec/WAL.md with the required sections and a fresh
   _Updated: line, filled from the actual tree, not from guesses.
2. Propose this project's wind-down and resume trigger phrases, with
   native-language twins if the team is bilingual.
3. Draft boot lines (CLAUDE.md, AGENTS.md, or equivalent) so every
   session reads the WAL before anything else.
4. State the autonomy rule: may a wind-down commit and push, or does
   it stop at drafts? Default to drafts.
Show all four as proposals. Apply nothing until I approve.
```

## Summary {#summary}

- @fact:SUM-TWO-FILES Two files: `spec/WAL.md` — living checkpoint, canonical;
  `CONTINUE.md` — cold-resume snapshot, subordinate. @status:impl/done
- @fact:SUM-WHITEBOARD-NOT-JOURNAL A whiteboard, not a journal: rewrite at session end, never append;
  date line first; older than 24 hours → verify before trusting. @status:impl/done
- @fact:SUM-SIZE-AND-CONSTRAINTS Keep it under a page; stable facts move to specs; constraints are the
  highest-value content — more autonomy demands more precision. @status:impl/done
- @fact:SUM-SPEC-BEATS-WAL Spec beats WAL; a stranger with only the repo resumes without asking. @status:impl/done
