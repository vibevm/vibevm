# WAL Protocol {#root}

**Scope of this document.** What a WAL is in a spec-driven project,
which files carry the discipline, what belongs in the WAL, how big it
may get, when it must be updated, and who wins when it disagrees with
the rest of the system. The rituals around it live in the siblings:
[`session-end-hook.md`](session-end-hook.md), [`morning-routine.md`](morning-routine.md), [`cold-resume.md`](cold-resume.md).

## The two files {#files}

Session-durable state lives in two repository files:

- **`spec/WAL.md` — the living checkpoint.** The *current* state:
  phase, constraints, done, open, next. Rewritten at every session end.
  **Canonical:** where it and any snapshot disagree, the WAL wins.
- **`CONTINUE.md` (repository root) — the cold-resume snapshot.**
  Written at session end for whoever picks up cold — another machine, a
  teammate, a long gap. Richer than the WAL, strictly subordinate.
  Contract: [`cold-resume.md`](cold-resume.md).

One invariant governs both: **resumption state lives in the repository,
never in a session.** Sessions end, compact, and crash; the repository
outlives them all. The rest of this document specifies the WAL.

## What a WAL is {#what}

WAL stands for **Write-Ahead Log** — a name any developer, and any
coding agent from its training data alone, recognises on sight. But the
mechanism that fits is not the ever-growing database log; it is the
database **checkpoint**: a snapshot of current state, *overwritten*,
not appended to — the old one is meaningless once a new one exists.

Picture a whiteboard by the door. A good one carries three lines:
where we are, what we are doing, what must not be touched; a bad one
carries three hundred lines of stale history, including tasks closed
months ago. **The WAL is a whiteboard, not a journal.**

The file serves both processes in the human-AI system. For the
**agent** it is the only persistent memory: each session starts blank,
and the WAL is read first. For the **human** it is the single morning
re-read that synchronises your memory with the project's actual state.
Everything else in `spec/` is medium-term memory or artefacts; this
file alone exists for session continuity.

## What a WAL is NOT {#not}

- **Not a log.** Rewritten every session, always describing the
  *current* state; an append-only WAL rots into an archive nobody reads.
- **Not a changelog.** History belongs in `git log` and spec changelogs.
- **Not a to-do list.** The "Next" section names *one* next action; the
  backlog belongs in an issue tracker.
- **Not documentation.** A fact stable enough to deserve a permanent
  home belongs in a spec document, not in the WAL.

## Required sections {#sections}

A well-formed `spec/WAL.md` has:

1. **`_Updated: <ISO 8601 UTC>_`** — the very first line after the
   title, always and without exception. An outdated WAL is worse than
   no WAL — false confidence; the date makes staleness detectable fast.
2. **Current phase** — one or two lines naming what the project is
   working on right now.
3. **Constraints** — the "do not touch" list; each entry carries a
   brief *why*, citing a spec anchor or issue: `match_by_hash(): DO NOT
   TOUCH — fragile reconnection logic, issue #12`. The most valuable
   content in the WAL: agents optimise locally, and without the written
   line an enthusiastic session "improves" what was deliberately left
   alone. Every constraint is a prevented regression.
4. **Done** — completed work collapsed to one-line summaries:
   "PROP-001 §§1-4 — complete, all tests pass", not five lines of notes.
5. **In progress** — what is open, with enough detail that the next
   session can pick it up. Cite spec anchors (`spec://…`).
6. **Next** — the single next action. Several candidates? List them
   briefly and mark the default.
7. **Known issues** — open problems not being worked on right now, so
   future sessions don't re-discover them.
8. **Session context** *(optional)* — one-paragraph orientation: what
   file to open first, what to run, what to avoid.

## Write for the autonomy you grant {#precision}

In chat, vague writing gets a question back. In agent mode — unattended
for hours — nobody asks; the WAL is executed literally. Compare:

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

The second is not bureaucracy: it is the difference between an agent
that quietly "optimises" the timeout back to 300 s and one that does
what is needed. **The more autonomy the agent has, the more precise the
WAL must be** — coarse context suffices for one supervised step; hours
unattended demand every constraint explicit and spec-anchored.

## Update triggers {#triggers}

- **At the end of every session.** Non-negotiable; a session that ends
  without updating the WAL has partially broken the next session.
  Procedure: [`session-end-hook.md`](session-end-hook.md).
- **Before any destructive operation.** A big refactor, a `git reset`,
  a migration — checkpoint first; a failure then restarts known-good.
- **When switching context mid-session.** Pivoting from FEAT-007 to
  FEAT-010? Checkpoint FEAT-007 *before* pivoting, or its state is lost.

## The freshness rule {#freshness}

A WAL older than **24 hours** is presumed stale: before trusting any
claim — and before *any* destructive work — verify against reality
(branch, tree, tests) and confirm divergences with the user. Tooling
may warn advisorily; the rule binds regardless.

## Size budget {#size}

The WAL is loaded into the agent's context at the start of every
session. Every token in the WAL is a tax paid per session.

- **Target:** ≤ 3 000 tokens (roughly one page of plain text).
- **Hard limit:** 5 000 tokens — past that the WAL hoards history:
  collapse done items, move stable facts to specs, split epics.
- **Rule of thumb:** unreadable end-to-end in a minute means too long.

## Conflict resolution {#conflicts}

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

The WAL is volatile by design — a *record* of the current state, not a
source of truth about intent. If the WAL says one thing and a spec
document says another, the spec wins and the WAL is stale.

An agent that believes a WAL entry is wrong does not silently rewrite
it: complete the task following the spec (not the stale WAL), add a
`<!-- REVIEW: … -->` marker where the mismatch matters, surface the
disagreement in the end-of-session report, and let the human reconcile.

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

One test decides whether the discipline works: **a stranger with only
the repository resumes work without asking.** A stranger, cold, from
files alone — not the author remembering, not the agent that wrote it
reconstructing. When in doubt what to record or where, apply it.

## Appendix: without a WAL {#without}

A project may decline this convention. The invariant still binds —
resumption state lives in the repository, never in a session — and the
fallback resume path is `git log` plus a fresh look at the tree: commit
messages carry session summaries, a plan's status line carries campaign
state, each session re-derives the rest. That works; the cost is three
places instead of one, no single canonical "now", tokens burned every
session on re-derivation — and constraints missed first. A project that
keeps re-deriving its own state has received the signal to adopt.

## Re-derive for your project {#re-derive}

Copy the task, not the implementation. Hand your agent this prompt and
review its drafts:

```text
Read spec/flows/wal/WAL-PROTOCOL.md end to end, then adapt the WAL
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

- Two files: `spec/WAL.md` — living checkpoint, canonical;
  `CONTINUE.md` — cold-resume snapshot, subordinate.
- A whiteboard, not a journal: rewrite at session end, never append;
  date line first; older than 24 hours → verify before trusting.
- Keep it under a page; stable facts move to specs; constraints are the
  highest-value content — more autonomy demands more precision.
- Spec beats WAL; a stranger with only the repo resumes without asking.
