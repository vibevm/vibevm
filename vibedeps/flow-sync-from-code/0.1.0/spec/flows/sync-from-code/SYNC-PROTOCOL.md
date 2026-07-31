# Sync-from-Code Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *what* Sync-from-Code is,
*when* it fires, *what it must produce*, and *where it stops*. @impl/done

##THE-ONLY-SANCTIONED-WAY-TO-CLOSE-A-BOTTOM-UP-GAP It is
the only sanctioned way to close a spec/code gap that was opened by a
bottom-up edit. @impl/done

## What Sync-from-Code is {#what}

##the-normal-flow-is-top-down-lead The **normal** information flow in a spec-driven project is top-down: @impl/done

```
head  →  WAL  →  spec  →  code
```

##THE-TOP-DOWN-CHAIN-FROM-INTENT-TO-CODE Intent forms in the human's head; short-lived state lands in the WAL;
decisions harden into spec; the spec is then implemented in code. @impl/done

##CODE-IS-THE-ARTEFACT-NOT-THE-SOURCE Code
is the artefact, not the source. @impl/done

##SYNC-FROM-CODE-IS-THE-PROTOCOL-FOR-THE-INVERSE-CASE **Sync-from-Code is the protocol for the inverse case**: the code
changed first, and the spec must now follow. @impl/done

##IT-IS-DELIBERATELY-THE-EXCEPTION It is deliberately the
exception — the whole rest of the discipline pushes the other way. @impl/done

## Why it exists {#why}

##two-everyday-situations-lead Two everyday situations break the top-down flow: @impl/done

- ##SITUATION-DIRECT-EDITING **Direct editing.** The human opens the file and edits. Usually because
  it is faster than writing an intent for the agent to execute. Often
  the right call. The issue is not that it happened — the issue is that
  nothing updated the spec. @spec/done
- ##SITUATION-IMPERATIVE-CHAT-COMMANDS **Imperative chat commands.** The human tells the agent "change the
  timeout to 600 s" or "use blake3 instead of SHA-256". The agent does
  the work. Again, legitimate — nobody wants to draft a PROP revision
  for a five-second decision — but the spec is now wrong. @spec/done

##left-unreconciled-either-case-produces-spec-drift Left unreconciled, either case produces spec drift. @spec/done

##the-next-session-corrects-the-code-back The next session
reads the stale spec, sees the 300 s figure, concludes that the code's
600 s is a bug, and "corrects" it. @spec/done

##the-agent-is-technically-right-by-the-spec-wins-hierarchy The agent is technically right by
the spec-wins hierarchy. @spec/done

##the-real-bug-is-upstream The real bug is upstream: the spec lied. @spec/done

##SYNC-FROM-CODE-CLOSES-THE-DRIFT-ON-PURPOSE Sync-from-Code is how the drift is closed *on purpose* rather than
letting it accumulate until a session triggers a wrong-direction "fix". @impl/done

## When to run {#trigger}

##RUN-THE-PROTOCOL-AT-THE-END-OF-THE-SAME-SESSION Run the protocol at the end of the **same session** that produced the
code change. @impl/done

##waiting-even-a-day-is-how-drift-accumulates Waiting even a day is how drift accumulates — by then
other sessions have read the stale spec and made decisions on top of
it, and now two things need reconciling instead of one. @spec/done

##do-not-run-the-protocol-for-lead Do **not** run the protocol for: @impl/done

- ##SKIP-TEMPORARY-HACKS **Temporary hacks.** Debug `println!`s, throwaway probes, reproducers
  that will be reverted within the hour. Record the skip explicitly in
  the WAL so the next session does not try to sync the hack into the
  spec: @impl/done

  ```markdown
  ## Constraints
  - src/verify.rs: temporary trace logging for issue #42, do NOT sync.
  ```

- ##SKIP-MECHANICAL-CHANGES **Mechanical changes.** `cargo fmt`, import reordering, dead-code
  removal flagged by the compiler, rename of a private symbol with no
  spec-level contract. Mechanical changes are below the spec's level of
  resolution. @impl/done

- ##SKIP-CODE-THE-SPEC-DOES-NOT-MENTION **Code that implements something the spec does not mention.** This is
  a forward-flow case, not a sync case: draft a new spec section from
  scratch, then reconcile. Sync-from-Code updates existing spec entries,
  it does not bootstrap them. @impl/done

##sibling-document-pointers Edge case: see [`when-to-apply.md`](when-to-apply.md) for the full
decision table. @impl/done

## Procedure {#procedure}

##FOUR-STEPS-ALWAYS-ENDING-IN-A-HUMAN-APPROVAL Four steps, always ending in a human approval. @impl/done

### 1. Collect the diff {#step-collect}

```
git diff HEAD
```

##A-WIDER-RANGE-WHEN-SEVERAL-COMMITS-HAVE-LANDED …or a wider range (`git diff <base>..HEAD`) if several commits have
landed since the last spec-aligned state. @impl/done

##THE-DIFF-IS-THE-ONLY-SOURCE-OF-TRUTH-FOR-WHAT-CHANGED The diff is the only source
of truth for what actually changed. @impl/done

### 2. Reconstruct intent {#step-intent}

##NAME-THE-WHY-NOT-THE-WHAT For each logical hunk, name the *why* — not the *what*, which the diff
already shows. Example: @impl/done

- ##example-diff *Diff:* `-const TIMEOUT: u64 = 300;` / `+const TIMEOUT: u64 = 600;`. @impl/done
- ##example-intent *Intent:* observed false-positive TIMEOUTs for users on high-latency
  VPNs; 600 s is the empirical threshold at which the false-positive
  rate drops to zero. @impl/done

##THE-INTENT-SENTENCE-IS-WHAT-LANDS-IN-THE-SPEC The intent sentence is what lands in the spec — **not the diff itself**. @impl/done

##IF-YOU-CANNOT-NAME-THE-INTENT-STOP-AND-ASK-THE-HUMAN If you cannot name the intent in one sentence, stop and ask the human. @impl/done

##A-HAND-WAVED-INTENT-ENCODES-A-FICTION A sync with a missing or hand-waved intent encodes a fiction; better to
fail loudly than to record one. @impl/done

### 3. Draft the spec delta {#step-draft}

##PRODUCE-A-UNIFIED-DIFF-PROPOSAL Produce a unified-diff proposal against the relevant spec section. @impl/done

##NOT-A-REWRITTEN-FILE Not
a rewritten file. @impl/done

##three-mandatory-parts-lead Three parts are mandatory in every sync: @impl/done

```diff
# spec/modules/oproto/PROP-003.md §verification.timeout

- Unverified messages older than 300 seconds get status TIMEOUT.
+ Unverified messages older than 600 seconds get status TIMEOUT.
+
+ **Why 600 s:** 300 s produced false positives for VPN users
+ (measured 2026-03-05 on 847 messages from 128 users).
+ **When to revisit:** if p99 network latency drops below 100 s
+ based on mon/latency-p99.
```

1. ##PART-THE-NEW-VALUE **The new value.** The primary change, matching the code. @impl/done
2. ##PART-THE-REASON **The reason.** Concrete, measurable where possible — no "felt
   better". Cite data, an issue number, or a dated observation. @impl/done
3. ##PART-THE-REVISIT-TRIGGER **The revisit trigger.** The condition under which this decision
   should be re-examined. A decision without a revisit trigger becomes
   a sacred cow. @impl/done

##WITHOUT-ANY-OF-THE-THREE-THE-DRAFT-IS-INCOMPLETE Without any of the three, the draft is incomplete. @impl/done

### 4. Surface for approval {#step-approve}

##PRINT-THE-PROPOSED-SPEC-DIFF Print the proposed spec diff. @impl/done

##DO-NOT-APPLY Do not apply. @impl/done

##WAIT-FOR-AN-EXPLICIT-APPLY-FROM-THE-USER Wait for an explicit "apply"
from the user. @impl/done

##ON-APPROVAL-WRITE-THE-DIFF-AND-COMMIT On approval: write the diff, commit it using Conventional Commits. @impl/done

##THE-COMMIT-TYPE-IS-DOCS-SPEC The
commit type is `docs(spec)`, the body names the driving code change: @impl/done

```
docs(spec): sync timeout to 600s in PROP-003 §verification.timeout

Code changed TIMEOUT from 300 s to 600 s after VPN latency
measurement (2026-03-05, 847 messages, 128 users). Spec now
carries the new value, the reason, and the revisit trigger.
```

##ON-REJECT-REVERT-THE-CODE-OR-REDRAFT-THE-PROPOSAL On reject: either revert the code (the code change itself was the
mistake) or redraft the sync proposal (the agent framed the intent
incorrectly). @impl/done

##NEVER-SILENTLY-ACCEPT-A-REJECTED-SYNC Never silently accept a rejected sync. @impl/done

##review-checklist-pointer The full human-side checklist lives in [`review-workflow.md`](review-workflow.md). @impl/done

## What Sync-from-Code does not do {#non-goals}

- ##NON-GOAL-DOES-NOT-REWRITE-CODE-TO-MATCH-THE-SPEC **Does not rewrite code to match the spec.** The opposite direction —
  generation — is a separate flow (`vibe build` territory). @impl/done
- ##NON-GOAL-DOES-NOT-EDIT-THE-WAL **Does not edit the WAL.** A successful sync may later trigger a WAL
  update ("PROP-003 §timeout synced with code"), but that is a separate
  step, handled by `flow:wal`'s session-end protocol. @impl/done
- ##NON-GOAL-DOES-NOT-BATCH **Does not batch.** One intent per run. Batching two unrelated code
  changes into one spec edit defeats intent-per-decision and makes
  future audits impossible. @impl/done

## Summary {#summary}

- ##SUM-NORMAL-DIRECTION-IS-SPEC-TO-CODE Normal direction is spec → code; Sync-from-Code is the exception. @impl/done
- ##SUM-RUNS-ONCE-IN-THE-SESSION-THAT-CAUSED-THE-DRIFT Runs once, immediately, in the session that caused the drift. @impl/done
- ##SUM-OUTPUT-IS-VALUE-REASON-AND-REVISIT-TRIGGER Output: a spec diff with new value + reason + revisit trigger. @impl/done
- ##SUM-ALWAYS-ENDS-WITH-HUMAN-APPROVAL Always ends with human approval. Never silent, never batched, never
  applied to temporary code. @impl/done
