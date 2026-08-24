# Failure Modes and Recovery {#root}

**Scope of this document.** The conflict protocol fails in three
recurring, recognizable ways. This file names each failure, explains
the mechanics of why it happens — none of them requires a badly
behaved agent, only defaults plus entropy — and gives the recovery
drill for each. Read it when something feels wrong; re-read it
occasionally so the detection signals stay loaded.

| # | Failure | Detection signal | First move |
|---|---------|------------------|------------|
| 1 | Spec changed without a REVIEW | A normative value differs in the diff; no marker, no report line | Revert the spec file; keep the code |
| 2 | Stale state file after a crash | The session died before the end-of-session rewrite; the state file promises work git does not show | Reconstruct from `git log` / `git diff` *before* any new session |
| 3 | Spec contradicts itself | Two sections answer one question differently | Full re-read; the human picks the winner; fix every echo |

## Failure 1 — the spec changed without a REVIEW {#silent-spec-edit}

**What you see.** The diff replaces "600 seconds" with "300 seconds
with exponential backoff" inside a spec document. There is no
`REVIEW:` marker near the change and no line in the session report
announcing it. Typically the edit rides in a long diff that is
otherwise exactly what you asked for.

**Why it happens.** Nothing physical prevents it: the spec is a file,
the agent has a file editor, and the agent's local reasoning ("300 is
more robust") feels to it like a favor, not a violation. The human's
attention is the only tripwire, and attention is exactly what long
diffs exhaust. This failure is the default state of the world — the
protocol is what suppresses it, so any gap in the protocol lets it
back in.

**Recovery drill.**

1. Revert the spec file — and only it — to the last human-approved
   state (`git restore <spec-file>`, or check out that one file from
   the last good commit). Keep the session's code if it is otherwise
   correct.
2. If the code also carries the unauthorized value, you now hold an
   ordinary Spec > Code divergence: schedule the code fix, or let the
   agent dispute the restored value properly — with a marker.
3. Open the next session by stating the correction in plain text:
   "You changed a spec value without a REVIEW marker. I reverted it.
   If you still believe backoff is better, add a REVIEW with the
   reason and we will discuss it."
4. Add the standing rule to the boot file: "Never modify a normative
   spec value without a REVIEW marker." A correction spoken in chat
   evaporates with the session; a correction in the boot file is
   re-read at every future session start and compounds.

Step 4 is the one that matters. Steps 1–3 fix the incident; step 4
lowers the rate.

## Failure 2 — stale state file after a crashed session {#stale-state}

**What you see.** The previous session ended in a crash — context
overflow, out-of-memory, a killed process — and the end-of-session
rewrite of the project state file (WAL or equivalent) never ran. The
file now describes a state that no longer exists: it promises tests
that were later broken, or names an in-progress refactor that was
abandoned halfway.

**Why it happens.** The state file is volatile *by design*: rewritten
at session end, trusted at session start. A crash deletes exactly the
rewrite step and nothing else. The next agent has no memory except
files, so it boots from the stale file and trusts it completely —
executing yesterday's abandoned intent against today's tree.

**Recovery drill.**

1. **Do not start a new session yet.** The next agent's first act is
   reading the state file; feed it a lie and it returns work built on
   the lie.
2. Reconstruct reality from the durable record: `git log` and
   `git diff` over the crashed session's window, plus the test suite
   if it is fast. Git survived the crash; the state file did not.
3. Rewrite the state file by hand — a full rewrite, not an appended
   correction. Append-mode fixes leave the stale text in place for
   the next reader to trip over.
4. Only then start the session.

The human is the live backup for the state file. Heads persist across
crashes; volatile files do not. This recovery cannot be delegated:
an agent booted on a stale file cannot tell which parts are stale,
which is precisely the problem.

## Failure 3 — the spec contradicts itself {#self-contradiction}

**What you see.** After twenty iterations, §2 answers a question one
way and §5 another. Every session that read only one of the two
sections behaved correctly by its own lights; the document as a whole
no longer has a single answer.

**Why it happens.** Edits are local. Each session touches the section
it was pointed at and re-reads little else — pointing sessions at
narrow, addressable targets is the efficient habit, so the same habit
that keeps sessions cheap lets distant sections drift apart. Long
files make it worse: readers, human and agent alike, attend to the
beginning and the end and skim the middle, so contradictions
accumulate precisely where nobody looks.

**Recovery drill.**

1. Put a full re-read of the key specs on a schedule — weekly on an
   active project. A calendar item, not an aspiration.
2. Read end to end, noting every place a normative value or rule is
   stated. Duplicates are pre-contradictions.
3. When two sections disagree, the human picks the winner. This is a
   Human > Spec ruling; no automatic rule can make it, because both
   sections carry equal formal authority.
4. Fix every echo of the losing version in the same change, and note
   the resolution in the spec's changelog so the next full re-read
   has an anchor.

Prevention beats recovery here: give every normative value exactly
one home and cite it from everywhere else. A value stated twice is a
contradiction on a delay timer.

This is garbage collection for shared state, and the human is the
collector. It is boring, it is unskippable, and it does not delegate:
the agent inside a session is the process *generating* the garbage;
only the reader who spans sessions can sweep it.

## Summary {#summary}

- Failure 1, silent spec edit: revert the file, keep the code, state
  the rule next session, and write the rule into the boot file so the
  correction compounds instead of evaporating.
- Failure 2, stale state file: the human is the live backup —
  reconstruct from git, rewrite the file wholesale, and only then let
  a new session boot.
- Failure 3, self-contradicting spec: weekly full re-read, the human
  picks the winner, fix every echo in one change, keep each value in
  exactly one home.
- All three share one mechanic: state that nobody re-reads drifts.
  Every drill is re-reading with authority attached.
