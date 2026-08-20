# Sync-from-Code review workflow {#root}

<status stage="spec" state="done"/>

@fact:SYNC-ALWAYS-ENDS-WITH-A-HUMAN-APPROVAL-STEP Sync-from-Code always ends with a human approval step. @status:impl/done

@fact:this-document-is-the-checklist-for-that-step This document
is the checklist the human runs at that step. @status:impl/done

## What the agent hands you {#input}

@fact:a-proposal-in-three-parts-lead A proposal in three parts: @status:impl/done

1. @fact:PART-A-SPEC-DIFF **A spec diff**, shown as a unified diff against the current spec
   file (not a rewritten file). @status:impl/done
2. @fact:PART-AN-INTENT-STATEMENT **An intent statement** — one sentence per logical change, naming
   *why* the code changed. @status:impl/done
3. @fact:PART-A-REVISIT-TRIGGER **A revisit trigger** — the condition under which the decision
   should be re-examined. @status:impl/done

@fact:IF-ANY-OF-THE-THREE-IS-MISSING-THE-PROPOSAL-IS-INCOMPLETE If any of the three is missing, the proposal is incomplete. @status:impl/done

@fact:ASK-THE-AGENT-TO-FILL-THE-GAP-BEFORE-APPROVING Ask the
agent to fill the gap before approving. @status:impl/done

@fact:DO-NOT-APPROVE-AN-INCOMPLETE-SYNC Do not approve an incomplete
sync: a missing reason or trigger today is a lost decision in six
months. @status:impl/done

## The review checklist {#checklist}

@fact:RUN-EVERY-ITEM-BEFORE-APPROVING Run every item before approving. @status:impl/done

### 1. Does the intent match reality? {#check-intent}

@fact:READ-THE-INTENT-AGAINST-WHAT-YOU-REMEMBER-DOING Read the intent sentence against what you remember doing. @status:impl/done

@fact:CORRECT-A-MISMATCHED-INTENT-BEFORE-APPROVING If the agent
wrote "changed to 600 s because users on VPN need more time" and you
actually changed it because "300 s was arbitrary and 600 s felt
safer", correct the intent before approving. @status:impl/done

@fact:AN-INCORRECT-INTENT-IS-THE-MOST-DANGEROUS-THING-A-SYNC-CAN-LAND An incorrect intent is
the single most dangerous thing a sync can land — it encodes a
fiction that the future reader will trust. @status:impl/done

### 2. Is the reason durable? {#check-durability}

@fact:ASK-WILL-THIS-REASON-STILL-BE-VALID-IN-A-YEAR Ask: will this reason still be valid in a year? @status:impl/done

- @fact:example-durable-profiling-spike "Profiling showed a 30 % hot-path spike on this branch" ← yes. @status:impl/done
- @fact:example-non-durable-i-was-testing-something "I was testing something" ← no. Revert the code. @status:impl/done
- @fact:example-durable-library-api-change "The library we use changed its public API in 0.9" ← yes, but cite
  the library and the version. @status:impl/done

@fact:a-non-durable-reason-means-a-non-durable-change A non-durable reason means the code change itself is probably
non-durable. @status:spec/done

@fact:REVERT-RATHER-THAN-SYNC Revert rather than sync. @status:impl/done

### 3. Is the revisit trigger concrete? {#check-trigger}

@fact:WHEN-IT-BREAKS-IS-NOT-A-TRIGGER "When it breaks" is not a trigger. @status:impl/done

@fact:a-trigger-is-a-measurable-signal-lead A trigger is a measurable signal: @status:impl/done

- @fact:example-trigger-latency-threshold "When p99 network latency drops below 100 s, per mon/latency-p99" ← good. @status:impl/done
- @fact:example-trigger-cpu-threshold "When CPU usage exceeds 80 % on the hot path" ← good. @status:impl/done
- @fact:example-trigger-later-or-at-some-point "Later" / "at some point" / "when we refactor" ← bad. Rewrite. @status:impl/done

@fact:NO-TRIGGER-MEANS-NO-AUDIT-PATH No trigger means no audit path. @status:impl/done

@fact:a-permanent-fact-with-a-provisional-label You are shipping a permanent fact
with a provisional label. @status:spec/done

### 4. Does the diff touch only the affected section? {#check-scope}

@fact:A-SYNC-THAT-REACHES-BEYOND-THE-AFFECTED-SECTION-IS-OUT-OF-SCOPE A sync that also reflows paragraphs, renames anchors, or reorders
unrelated sections is out of scope. @status:impl/done

@fact:REJECT-AND-ASK-FOR-A-NARROW-DIFF Reject and ask for a narrow diff
that changes only what the code change demands. @status:impl/done

@fact:omnibus-spec-edits-make-history-unbisectable Omnibus spec edits
are how history becomes unbisectable. @status:spec/done

### 5. Is the anchor citation correct? {#check-anchor}

@fact:THE-SPEC-DIFF-NAMES-THE-AFFECTED-ANCHOR-EXACTLY The spec diff should name the affected anchor (`{#verification.timeout}`)
exactly. @status:impl/done

@fact:AN-IMPLEMENTS-MARKER-MUST-MATCH-LETTER-FOR-LETTER If the code carries an `// Implements: spec://…` marker,
the anchor in the marker must match letter-for-letter. @status:impl/done

@fact:stale-markers-rot-spec-to-code-traceability-silently Stale markers
are how spec-to-code traceability rots silently. @status:spec/done

### 6. Is the scope of the code change what you expected? {#check-surprise}

@fact:SKIM-THE-ACTUAL-DIFF-NOT-JUST-THE-PROPOSAL Before approving, skim the actual `git diff` — not just the sync
proposal. @status:impl/done

@fact:AN-UNEXPECTED-SECOND-FILE-IS-EITHER-HIDDEN-OR-UNRELATED If the code touched a second file you did not expect, that
second change is either (a) hidden in the proposal and the sync is
incomplete, or (b) unrelated and should have been a separate change. @status:impl/done

@fact:HANDLE-THE-SURPRISE-BEFORE-APPROVING-THE-SYNC Either way, handle the surprise before approving the sync. @status:impl/done

## On approval {#approve}

@fact:the-agent-lead The agent: @status:impl/done

1. @fact:APPROVAL-STEP-APPLIES-THE-SPEC-DIFF Applies the spec diff. @status:impl/done
2. @fact:APPROVAL-STEP-COMMITS-WITH-CONVENTIONAL-COMMITS Commits with Conventional Commits format (`docs(spec)` type), a body
   that cites the code change driving the sync, and the `spec://…` URI
   of the affected anchor: @status:impl/done

   ```
   docs(spec): sync timeout into PROP-003 §verification.timeout

   Code changed TIMEOUT from 300 s to 600 s after VPN latency
   measurement (2026-03-05, 847 messages, 128 users). Spec now
   carries the value, the reason, and the revisit trigger.
   Cited by spec://oproto/PROP-003#verification.timeout.
   ```

3. @fact:APPROVAL-STEP-STOPS Stops. Does not continue into unrelated follow-up work in the
   same run — a sync is its own atomic step. @status:impl/done

## On rejection {#reject}

@fact:two-paths-lead Two paths: @status:impl/done

- @fact:REJECTION-PATH-KEEP-THE-CODE **Reject the sync, keep the code.** The code change was right but
  the proposal's framing was wrong. Ask the agent to redraft with
  the correct intent. No revert. @status:impl/done
- @fact:REJECTION-PATH-REVERT-THE-CODE **Reject the sync, revert the code.** The code change itself was
  the problem — the proposal surfaced it. Revert with `git revert`
  or `git checkout --`, and record the lesson in the WAL's Known
  Issues if it is worth carrying forward. @status:impl/done

@fact:NEITHER-PATH-SILENTLY-ACCEPTS-A-BAD-SYNC Neither path silently accepts a bad sync. @status:impl/done

@fact:a-silently-accepted-bad-sync-makes-the-spec-fiction A silently accepted bad
sync is how the spec becomes fiction. @status:spec/done

## Why the checklist is long {#why-long}

@fact:SYNC-IS-THE-ONE-PROTOCOL-THAT-WRITES-SPEC-DRIVEN-BY-CODE Sync-from-Code is the one protocol in the project that writes a
spec change *driven by code*. @status:impl/done

@fact:EVERY-OTHER-SPEC-CHANGE-IS-HUMAN-INITIATED-FROM-INTENT Every other spec change is
human-initiated from intent. @status:impl/done

@fact:a-weaker-driver-demands-a-stronger-approval-step Because the driver is weaker — reverse
engineering of intent from a diff — the approval step has to be
stronger. @status:spec/done

@fact:six-checks-is-not-bureaucracy Six checks is not bureaucracy; it is the reason this flow
does not produce drift of its own. @status:spec/done
