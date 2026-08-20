# Cold resume — `CONTINUE.md` and the session commands {#root}

**Scope of this document.** The second file of the discipline:
`CONTINUE.md`, the cold-resume snapshot at the repository root. What
goes in it, when it is written, the wind-down and resume commands that
bracket a session, and the precedence rule when the snapshot and the
WAL disagree.

## Why a second file {#why}

The WAL is written for the next session on the *same* project rhythm:
terse, current, one page, assuming the reader knows the terrain. A cold
reader does not know the terrain: you on a different machine, a
teammate cloning the repository, a session resuming after weeks or
after a context compaction. The cold reader needs the tour — where
things are, what commands run, what was decided and why — before the
one-page checkpoint means anything.

`CONTINUE.md` is that tour. The pair together passes the discipline's
acceptance test: **a stranger with only the repository resumes work
without asking.**

## The CONTINUE.md contract {#contract}

`CONTINUE.md` lives at the repository root, where a cold reader looks
first. It is **overwritten wholesale** every time it is written — never
appended to, never patched; staleness compounds otherwise. The body
includes, at minimum:

1. A short TL;DR / executive summary at the top.
2. Where work stands: branch, ahead/behind the remote, working-tree
   status.
3. The active blocker, if any, and the exact human action that
   unblocks it.
4. The exact next-steps recipe — commands, file paths, line numbers —
   for whoever picks up cold.
5. Non-obvious findings of the session: API quirks, config gotchas,
   vendor-specific surprises.
6. A repository map: top-level layout and what each directory or
   component holds.
7. The architectural and policy decisions still in force, in long
   form.
8. The recent commit chain (last ~25, oneline format), so the cold
   reader sees velocity and direction.
9. Quick-start commands for the workspace.
10. A pointer noting that the WAL is the canonical living state and
    supersedes this snapshot if they diverge.

Items 1–4 answer "what do I do right now"; items 5–9 answer "what must
I know before I trust myself here"; item 10 keeps the file honest
about its own rank.

## When to write it {#when}

- **At every explicit wind-down** (the command below). Mandatory.
- **Before a machine switch** — the other machine gets the tour.
- **Before a long gap** — a vacation, weeks on another project; future
  you is a cold reader too.

Any session end is a fine time; the wind-down makes it non-optional.

## The wind-down command {#wind-down}

The wind-down is the explicit form of the session-end hook. Ship
trigger phrases: `END SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`.
Recognise the intent, not the exact wording. A project may add
native-language twins in its agent instructions — the origin project
of this flow runs a bilingual Russian/English set.

Required behaviour, in order:

1. **Overwrite `CONTINUE.md`** wholesale, per the contract above.
2. **Rewrite `spec/WAL.md`** per
   [`session-end-hook.md`](session-end-hook.md): fresh date line,
   current phase, constraints, next step.
3. **Commit in topic-grouped commits.** The snapshot and the WAL
   update are checkpoint commits; a code or config change landed by
   the same session is a separate commit.
4. **Push only if the project's autonomy rules sanction it.** No
   standing autonomy — stop at drafts or at local commits, per the
   hook's propose-by-default rule.
5. **Emit a chat TL;DR** of what the wind-down did: files written,
   commits created, push status, what the next session should pick up
   first. One screen; enough detail to verify without opening files.

The point of the command is to make session-boundary loss-of-context
cheap: any session can end at any time and be resumed from
`CONTINUE.md` plus the WAL with no degradation. Treat it as a hard
contract, not a courtesy.

## The resume command {#resume}

Trigger phrases: `RESUME SESSION`, `RESTORE CONTEXT` (and twins —
recognise the intent). The job is to **restore context and report —
nothing else**:

1. Run the project's boot sequence (whatever its agent instructions
   define), then read `CONTINUE.md` and `spec/WAL.md`.
2. Verify the repository state empirically: current branch, sync with
   the remote, working-tree status, recent commits. Never take the
   snapshot's word for what the tree looks like.
3. Emit a status report: where work stands, active blockers, and the
   candidate next steps.
4. **Stop and wait for direction.** No code edits, no commits, no
   pushes.

## Restore is not authorisation {#not-authorisation}

Any "resume work at …" pointer in `CONTINUE.md` or the WAL names the
*candidate* next step for the report — it is not authorisation to
start it. The resume boundary exists so the owner can inspect the
restored state and steer, possibly somewhere other than the recorded
next step. A session that boots straight into execution takes that
decision away from the owner.

This rule is written down because exactly that misfire happened once:
a resumed session read the recorded pointer as a work order and began
executing, when the owner wanted a report. The boundary has been
explicit ever since.

## Precedence {#precedence}

`spec/WAL.md` is canonical; `CONTINUE.md` is a snapshot of the moment
the last wind-down ran. When they disagree — and they will, whenever a
session updates the WAL without a full wind-down — trust the WAL and
flag the divergence in your report. The same rule covers any other
snapshot: a plan document's status line, a README's "current state"
paragraph. The living checkpoint beats a frozen snapshot, always.

## Never {#never}

- Never append to `CONTINUE.md` — overwrite it wholesale.
- Never write the snapshot from memory alone — verify branch, tree,
  and commits empirically first.
- Never start executing after a resume command — report, then wait.
- Never treat `CONTINUE.md` as overriding the WAL.
- Never skip the chat TL;DR on a wind-down: it is how the user
  verifies the checkpoint without opening files.

## Summary {#summary}

- `CONTINUE.md` at the repo root is the cold reader's tour: TL;DR,
  state, blocker, recipe, findings, map, decisions, commits,
  quick-start — overwritten wholesale, never appended.
- Write it at every wind-down, before machine switches, before gaps.
- Wind-down: snapshot + WAL rewrite + topic-grouped commits +
  sanctioned push + chat TL;DR.
- Resume: boot, read both files, verify empirically, report, stop.
- A recorded next step is a candidate, not authorisation.
- The WAL supersedes the snapshot wherever they diverge.
