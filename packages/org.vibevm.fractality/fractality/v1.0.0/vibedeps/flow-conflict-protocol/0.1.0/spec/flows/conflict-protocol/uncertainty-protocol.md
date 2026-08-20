# Uncertainty Protocol {#root}

**Scope of this document.** This file defines *what to do when the
spec is silent* — the four-step ladder that ends in a conservative
default plus a REVIEW marker, what "conservative" means precisely,
and the cases where the ladder does not apply and stopping to ask is
the only correct move.

## Silence is not a conflict {#silence}

The conflict protocol handles disagreement: two sources give two
answers, and the hierarchy picks the winner. Uncertainty is the other
case: *no* source answers at all. The spec defines FAILED and RUNNING
but not what `retry` does to a RUNNING job; the spec fixes a timeout
but not what happens to a message that verifies exactly at the
boundary.

Both cases tempt the same failure — inventing an answer silently. In
the conflict case the invention overrides someone; in the silence
case it fabricates semantics nobody decided. Silent invention is
worse than either wrong answer, because it is invisible: the project
now behaves in a way no document predicts, and the next reader —
human or agent — has no way to learn the behavior exists short of
tripping over it.

The rule, compressed: **never silently invent semantic behavior.**
The ladder below is how to make progress without inventing.

## The ladder {#ladder}

When the spec is silent on a question, climb in order; stop at the
first rung that answers.

| Step | Action | What it typically finds |
|------|--------|-------------------------|
| 1 | Re-read the relevant spec section — in full, not the one line you remember. | The answer, two paragraphs away or phrased under a different heading. Most "silence" is a narrow first read. |
| 2 | Re-read the relevant reference material — the book chapter, design note, ADR, or RFC the spec section grew out of. | Intent. The spec records decisions; the reference records *why*, and intent often settles what the decision text left open. |
| 3 | Look at the closest analog — the nearest similar feature in this project, or the project's named reference implementations. | A precedent. Consistency with an existing pattern is itself a decision the project already made. |
| 4 | Pick the **conservative interpretation**, mark it with a REVIEW, proceed, and flag it in the end-of-session report. | Forward progress with a visible, reversible decision instead of a stall or an invention. |

Step 4 in full: the marker carries the question and the reason —

```
<!-- REVIEW: spec silent on <question>; chose <interpretation>
     because it is the cheapest to reverse -->
```

— and the report names it, so the human rules in the next cycle and
the spec gains a sentence. The ladder never ends in silence: either a
source answered, or a marked conservative default did.

## What "conservative" means {#conservative}

Conservative is not "smallest diff" or "least effort". It is the
interpretation that is **cheapest to reverse** when the human rules
the other way next session. The test: *if this choice turns out
wrong, how much work is thrown away — and whose?* Pick the answer
that minimizes it.

| Prefer | Over | Because |
|--------|------|---------|
| No new public surface | A new exported function, flag, endpoint, or format | Additions can wait a day; a published surface someone may already depend on is the most expensive thing in software to retract. |
| Failing loudly | Guessing silently | An explicit error is diagnosed in seconds; a plausible wrong guess is found in production. |
| The narrower behavior | The broader one | Narrow can widen tomorrow without breaking anyone; broad, once observed, cannot narrow. |
| Existing project convention | External best practice | The convention is a recorded project decision; the fashion is not. Overruling it is the human's call. |
| No new dependency | Adding one | A dependency is a permanent tax, and its removal is a migration. |

The first row deserves emphasis: when in doubt, **refuse to invent
public surface**. Internal choices are corrections waiting to happen;
public ones are commitments.

## When to stop and ask instead {#stop-and-ask}

The ladder is a protocol for *reversible* uncertainty. Some questions
have no conservative interpretation because every answer is a
commitment. For these, stop and ask the human — mid-session if the
channel exists, otherwise park the task and say so in the report:

- **Irreversible operations** — deleting or migrating data, releasing
  or publishing artifacts, rewriting pushed history.
- **Security boundaries** — authentication, authorization, secrets
  handling, cryptographic parameters. A "conservative guess" about
  crypto is a contradiction in terms.
- **External side effects** — sending mail, charging accounts,
  calling third-party APIs in ways outsiders observe.
- **Anything whose reversal costs real work.** Re-run the test from
  the previous section: if the cheapest-to-reverse option still takes
  a day to unwind, there is no conservative option. Asking *is* the
  conservative option.

Marking a REVIEW and proceeding is forward motion on a reversible
path. On an irreversible path it is just speed.

## Worked example {#worked-example}

```
Task     Implement `jobs retry` per spec §4: "retries failed jobs."

Silence  A job is RUNNING now, but its previous attempt failed. Does
         it count as failed? §4 does not say.

Step 1   Re-read §4 end to end. FAILED, RUNNING, DONE are defined;
         the overlap case is not addressed. Still silent.

Step 2   Re-read the job-lifecycle design note. Intent recorded:
         "retry exists so an operator can recover a dead batch
         overnight." Suggestive, not decisive. Still silent.

Step 3   Closest analog: `jobs cancel` explicitly skips RUNNING
         jobs, with a comment saying why. Precedent: mutating
         commands do not touch RUNNING jobs.

Step 4   Conservative pick: skip RUNNING jobs. Retrying one risks a
         double execution — expensive to reverse and visible to
         users; skipping costs one more `retry` invocation tomorrow.
         Marker beside the implementation:

         // REVIEW: spec silent on RUNNING jobs in `retry`; skipping
         // them, matching `cancel` — double execution is the
         // costlier mistake.

Report   "`jobs retry` implemented per §4. Spec is silent on RUNNING
         jobs; chose to skip them (REVIEW in the retry module) —
         needs a ruling."

Next     The human confirms "skip is right", adds one sentence to
         §4, and removes the marker in the same commit.
```

The project moved forward the same day, the decision stayed visible
the whole time, and the spec ended one sentence more complete than it
started. That is the ladder working.

## Summary {#summary}

- Uncertainty is not conflict: no source answers. The temptation —
  silent invention — is the same, and it is forbidden.
- Climb in order: spec section → reference material → closest analog
  → conservative default + REVIEW + report.
- Conservative = cheapest to reverse. Refuse to invent public
  surface; fail loudly rather than guess silently.
- Irreversible operations, security boundaries, external side
  effects: the ladder does not apply. Stop and ask — asking is the
  conservative choice there.
- The end state is always visible: an answered question, or a marked
  default awaiting a ruling. Never a silent guess.
