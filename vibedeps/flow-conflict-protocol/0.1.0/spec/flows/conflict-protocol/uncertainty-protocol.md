# Uncertainty Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what to do when the
spec is silent* — the four-step ladder that ends in a conservative
default plus a REVIEW marker, what "conservative" means precisely,
and the cases where the ladder does not apply and stopping to ask is
the only correct move. @status:impl/done

## Silence is not a conflict {#silence}

@fact:THE-CONFLICT-PROTOCOL-HANDLES-DISAGREEMENT The conflict protocol handles disagreement: two sources give two
answers, and the hierarchy picks the winner. @status:impl/done

@fact:UNCERTAINTY-IS-THE-OTHER-CASE-NO-SOURCE-ANSWERS Uncertainty is the other
case: *no* source answers at all. @status:impl/done

@fact:two-examples-of-silence The spec defines FAILED and RUNNING
but not what `retry` does to a RUNNING job; the spec fixes a timeout
but not what happens to a message that verifies exactly at the
boundary. @status:impl/done

@fact:BOTH-CASES-TEMPT-THE-SAME-FAILURE Both cases tempt the same failure — inventing an answer silently. @status:spec/done

@fact:invention-overrides-or-fabricates In
the conflict case the invention overrides someone; in the silence
case it fabricates semantics nobody decided. @status:spec/done

@fact:SILENT-INVENTION-IS-WORSE-BECAUSE-IT-IS-INVISIBLE Silent invention is
worse than either wrong answer, because it is invisible: the project
now behaves in a way no document predicts, and the next reader —
human or agent — has no way to learn the behavior exists short of
tripping over it. @status:spec/done

@fact:NEVER-SILENTLY-INVENT-SEMANTIC-BEHAVIOR The rule, compressed: **never silently invent semantic behavior.** @status:impl/done

@fact:the-ladder-is-how-to-progress-without-inventing The ladder below is how to make progress without inventing. @status:impl/done

## The ladder {#ladder}

@fact:CLIMB-IN-ORDER-AND-STOP-AT-THE-FIRST-RUNG-THAT-ANSWERS When the spec is silent on a question, climb in order; stop at the
first rung that answers. @status:impl/done

| Step | Action | What it typically finds |
|------|--------|-------------------------|
| @fact:ROW-LADDER-STEP-1-RE-READ-THE-SPEC-SECTION 1 @status:spec/done | Re-read the relevant spec section — in full, not the one line you remember. @status:spec/done | The answer, two paragraphs away or phrased under a different heading. Most "silence" is a narrow first read. @status:spec/done |
| @fact:ROW-LADDER-STEP-2-RE-READ-THE-REFERENCE-MATERIAL 2 @status:spec/done | Re-read the relevant reference material — the book chapter, design note, ADR, or RFC the spec section grew out of. @status:spec/done | Intent. The spec records decisions; the reference records *why*, and intent often settles what the decision text left open. @status:spec/done |
| @fact:ROW-LADDER-STEP-3-LOOK-AT-THE-CLOSEST-ANALOG 3 @status:spec/done | Look at the closest analog — the nearest similar feature in this project, or the project's named reference implementations. @status:spec/done | A precedent. Consistency with an existing pattern is itself a decision the project already made. @status:spec/done |
| @fact:ROW-LADDER-STEP-4-CONSERVATIVE-DEFAULT-PLUS-REVIEW 4 @status:spec/done | Pick the **conservative interpretation**, mark it with a REVIEW, proceed, and flag it in the end-of-session report. @status:spec/done | Forward progress with a visible, reversible decision instead of a stall or an invention. @status:spec/done |

@fact:STEP-4-IN-FULL-THE-MARKER-CARRIES-THE-QUESTION-AND-THE-REASON Step 4 in full: the marker carries the question and the reason — @status:impl/done

```
<!-- REVIEW: spec silent on <question>; chose <interpretation>
     because it is the cheapest to reverse -->
```

@fact:THE-REPORT-NAMES-THE-MARKER-AND-THE-HUMAN-RULES — and the report names it, so the human rules in the next cycle and
the spec gains a sentence. @status:impl/done

@fact:THE-LADDER-NEVER-ENDS-IN-SILENCE The ladder never ends in silence: either a
source answered, or a marked conservative default did. @status:impl/done

## What "conservative" means {#conservative}

@fact:CONSERVATIVE-IS-NOT-SMALLEST-DIFF-OR-LEAST-EFFORT Conservative is not "smallest diff" or "least effort". @status:impl/done

@fact:CONSERVATIVE-IS-CHEAPEST-TO-REVERSE It is the
interpretation that is **cheapest to reverse** when the human rules
the other way next session. @status:impl/done

@fact:THE-TEST-HOW-MUCH-WORK-IS-THROWN-AWAY-AND-WHOSE The test: *if this choice turns out
wrong, how much work is thrown away — and whose?* @status:impl/done

@fact:PICK-THE-ANSWER-THAT-MINIMIZES-IT Pick the answer
that minimizes it. @status:impl/done

| Prefer | Over | Because |
|--------|------|---------|
| @fact:ROW-PREFER-NO-NEW-PUBLIC-SURFACE No new public surface @status:spec/done | A new exported function, flag, endpoint, or format @status:spec/done | Additions can wait a day; a published surface someone may already depend on is the most expensive thing in software to retract. @status:spec/done |
| @fact:ROW-PREFER-FAILING-LOUDLY Failing loudly @status:spec/done | Guessing silently @status:spec/done | An explicit error is diagnosed in seconds; a plausible wrong guess is found in production. @status:spec/done |
| @fact:ROW-PREFER-THE-NARROWER-BEHAVIOR The narrower behavior @status:spec/done | The broader one @status:spec/done | Narrow can widen tomorrow without breaking anyone; broad, once observed, cannot narrow. @status:spec/done |
| @fact:ROW-PREFER-EXISTING-PROJECT-CONVENTION Existing project convention @status:spec/done | External best practice @status:spec/done | The convention is a recorded project decision; the fashion is not. Overruling it is the human's call. @status:spec/done |
| @fact:ROW-PREFER-NO-NEW-DEPENDENCY No new dependency @status:spec/done | Adding one @status:spec/done | A dependency is a permanent tax, and its removal is a migration. @status:spec/done |

@fact:REFUSE-TO-INVENT-PUBLIC-SURFACE-WHEN-IN-DOUBT The first row deserves emphasis: when in doubt, **refuse to invent
public surface**. @status:impl/done

@fact:internal-choices-are-corrections-public-ones-are-commitments Internal choices are corrections waiting to happen;
public ones are commitments. @status:spec/done

## When to stop and ask instead {#stop-and-ask}

@fact:THE-LADDER-IS-A-PROTOCOL-FOR-REVERSIBLE-UNCERTAINTY The ladder is a protocol for *reversible* uncertainty. @status:impl/done

@fact:some-questions-have-no-conservative-interpretation Some questions
have no conservative interpretation because every answer is a
commitment. @status:spec/done

@fact:STOP-AND-ASK-THE-HUMAN-FOR-THESE For these, stop and ask the human — mid-session if the
channel exists, otherwise park the task and say so in the report: @status:impl/done

- @fact:STOP-FOR-IRREVERSIBLE-OPERATIONS **Irreversible operations** — deleting or migrating data, releasing
  or publishing artifacts, rewriting pushed history. @status:impl/done
- @fact:STOP-FOR-SECURITY-BOUNDARIES **Security boundaries** — authentication, authorization, secrets
  handling, cryptographic parameters. A "conservative guess" about
  crypto is a contradiction in terms. @status:impl/done
- @fact:STOP-FOR-EXTERNAL-SIDE-EFFECTS **External side effects** — sending mail, charging accounts,
  calling third-party APIs in ways outsiders observe. @status:impl/done
- @fact:STOP-FOR-ANYTHING-WHOSE-REVERSAL-COSTS-REAL-WORK **Anything whose reversal costs real work.** Re-run the test from
  the previous section: if the cheapest-to-reverse option still takes
  a day to unwind, there is no conservative option. Asking *is* the
  conservative option. @status:impl/done

@fact:MARKING-A-REVIEW-IS-FORWARD-MOTION-ON-A-REVERSIBLE-PATH Marking a REVIEW and proceeding is forward motion on a reversible
path. @status:impl/done

@fact:on-an-irreversible-path-it-is-just-speed On an irreversible path it is just speed. @status:spec/done

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

- @fact:EXAMPLE-THE-PROJECT-MOVED-FORWARD-THE-SAME-DAY The project moved forward the same day, @status:impl/done
- @fact:EXAMPLE-THE-DECISION-STAYED-VISIBLE-THE-WHOLE-TIME the decision stayed visible
  the whole time, @status:impl/done
- @fact:EXAMPLE-THE-SPEC-ENDED-ONE-SENTENCE-MORE-COMPLETE and the spec ended one sentence more complete than it
  started. @status:impl/done

@fact:that-is-the-ladder-working That is the ladder working. @status:impl/done

## Summary {#summary}

- @fact:SUM-UNCERTAINTY-IS-NOT-CONFLICT Uncertainty is not conflict: no source answers. The temptation —
  silent invention — is the same, and it is forbidden. @status:impl/done
- @fact:SUM-CLIMB-IN-ORDER Climb in order: spec section → reference material → closest analog
  → conservative default + REVIEW + report. @status:impl/done
- @fact:SUM-CONSERVATIVE-IS-CHEAPEST-TO-REVERSE Conservative = cheapest to reverse. Refuse to invent public
  surface; fail loudly rather than guess silently. @status:impl/done
- @fact:SUM-THE-LADDER-DOES-NOT-APPLY-TO-IRREVERSIBLE-QUESTIONS Irreversible operations, security boundaries, external side
  effects: the ladder does not apply. Stop and ask — asking is the
  conservative choice there. @status:impl/done
- @fact:SUM-THE-END-STATE-IS-ALWAYS-VISIBLE The end state is always visible: an answered question, or a marked
  default awaiting a ruling. Never a silent guess. @status:impl/done
