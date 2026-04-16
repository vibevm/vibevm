# WAL Protocol {#root}

**Scope of this document.** This file defines *what* a WAL is in a vibevm
project, *what* belongs in it, *how big* it is allowed to get, *when* it
must be updated, and *who wins* when the WAL and some other part of the
system disagree.

## What a WAL is {#what}

WAL stands for **Write-Ahead Log**, the name borrowed from database
internals. A WAL in a vibevm project is a short checkpoint file at
`spec/WAL.md` that captures the current state of the project in a form
that lets *both* processes in the human-AI system pick up work across a
session boundary:

- For the **AI**, it is the only source of persistent memory. Each session
  starts blank; the WAL is the first thing the AI reads.
- For the **human**, it is a single place to re-read every morning to
  synchronise your own memory with the project's actual state.

This is the one file in the project that exists *specifically* for session
continuity. Everything else in `spec/` is medium-term memory (stable
decisions) or artefacts (code, tests).

## What a WAL is NOT {#not}

- **Not a log.** Despite the name, a WAL in this sense is not an
  ever-growing record of events. It is rewritten at the end of every
  session and always describes the *current* state, not the history.
- **Not a changelog.** History belongs in `git log` and in the
  changelog sections of spec documents.
- **Not a to-do list.** The "Next" section names *one* next action, not a
  backlog. The backlog belongs in an issue tracker.
- **Not project documentation.** If a fact is stable enough to deserve a
  permanent home, it belongs in a PROP or FEAT document under `spec/`,
  not in the WAL.

## Required sections {#sections}

A well-formed `spec/WAL.md` has:

1. **`_Updated: <ISO 8601 UTC>_`** — the very first line after the title.
   An outdated WAL is worse than no WAL, because it creates false
   confidence. The date lets both human and AI detect staleness in a second.
2. **Current phase** — one or two lines naming what the project is
   working on right now.
3. **Constraints** — the "do not touch" list. Each entry must carry a
   brief *why*. Example: `match_by_hash() — НЕ ТРОГАТЬ (reconnection
   logic is fragile, issue #12)`. Constraints are the single most
   valuable part of the WAL: they are what prevents an enthusiastic AI
   from "improving" something that was deliberately left as-is.
4. **Done** — completed work, collapsed to one-line summaries. "PROP-001
   §§1-4 — complete, all tests pass" beats five lines of implementation
   notes.
5. **In progress** — what is currently open, with enough detail that the
   next session can pick it up. Cite spec anchors (`spec://…`) where
   possible.
6. **Next** — the single next action. If there are multiple candidate
   next actions, list them briefly and indicate which one is default.
7. **Known issues** — open problems not currently being worked on. These
   exist so future sessions don't re-discover them.
8. **Session context** *(optional)* — one-paragraph orientation for the
   next session: what file to open first, what command to run, what to
   avoid.

## Update triggers {#triggers}

Update the WAL:

- **At the end of every session.** Non-negotiable. A session that ends
  without updating the WAL has partially broken the next session's context.
- **Before any destructive operation.** Before a big refactor, before a
  `git reset`, before dropping a database — checkpoint first. If the
  operation goes wrong, you restart from a known-good state.
- **When switching context mid-session.** If you were working on FEAT-007
  and the user pivots you to FEAT-010, update the WAL for FEAT-007
  *before* pivoting. Otherwise FEAT-007's state is lost.

## Size budget {#size}

The WAL is loaded into the AI's context at the start of every session.
Every token in the WAL is a tax paid per session.

- **Target:** ≤ 3 000 tokens (roughly one page of plain text).
- **Hard limit:** 5 000 tokens. At that point the WAL is no longer doing
  its job — it is hoarding history. Collapse completed items; move
  stable facts out to PROP/FEAT documents; split pending work into
  separate epics.
- **Rule of thumb:** if a human cannot read the WAL end-to-end in under
  a minute, it is too long.

## Conflict resolution {#conflicts}

When the WAL disagrees with something else:

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

The WAL is volatile by design. It is a *record* of the current state, not
a source of truth about intended behaviour. If the WAL says one thing
and a PROP document says another, the PROP wins and the WAL is stale.

When the AI believes a WAL entry is wrong, it should not silently
rewrite the entry. It should:

1. Complete the current task following the spec (and not the stale
   WAL), adding a `<!-- REVIEW: … -->` marker where the mismatch matters.
2. Surface the disagreement in its end-of-session report.
3. Let the human reconcile in the next cycle.

## A working example {#example}

```markdown
# WAL — Project Continuation State
_Updated: 2026-04-16T18:23:00Z_

## Current phase
PROP-003 verification engine — IN PROGRESS (~70%)

## Constraints (do not violate without discussion)
- Timeout = 600s (spec://oproto/PROP-003#verification.timeout).
  Reason: 300s caused false positives on VPN users.
- match_by_hash(): DO NOT TOUCH — fragile reconnection logic, issue #12.
- Hashing: blake3, not SHA-256 — avoids OpenSSL dependency.

## Done
- [x] PROP-001 OPROTO base: complete.
- [x] PROP-002 Edge server skeleton: complete, 47/52 tests passing.

## In progress
- PROP-003 verification engine:
  - DONE: hash matcher (spec://oproto/PROP-003#verification.normal).
  - DONE: basic timeout (spec://oproto/PROP-003#verification.timeout).
  - TODO: degraded mode (spec://oproto/PROP-003#verification.degraded).

## Next
src/verify.rs → degraded_mode_handler().
Start with: `cargo test -p oproto-verify`.

## Known issues
- grammers reconnection after network loss is not handled (issue #12).
- protobuf schema for media_refs is ambiguous (issue #15).

## Session context
Start of next session: read spec://oproto/PROP-003#verification.degraded,
then open src/verify.rs. Do NOT touch match_by_hash().
```

## Summary {#summary}

- The WAL is a *checkpoint*, not a log.
- Rewrite it at the end of every session.
- Keep it short; move stable facts to specs.
- Constraints are the highest-value content — treat them as invariants.
- When the WAL and the spec disagree, the spec wins.
