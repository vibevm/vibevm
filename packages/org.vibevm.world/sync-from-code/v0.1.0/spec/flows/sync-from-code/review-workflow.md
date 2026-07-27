# Sync-from-Code review workflow {#root}

<status stage="spec" state="done"/>

##SYNC-ALWAYS-ENDS-WITH-A-HUMAN-APPROVAL-STEP Sync-from-Code always ends with a human approval step. @impl/done

##this-document-is-the-checklist-for-that-step This document
is the checklist the human runs at that step. @impl/done

## What the agent hands you {#input}

##a-proposal-in-three-parts-lead A proposal in three parts: @impl/done

1. ##PART-A-SPEC-DIFF **A spec diff**, shown as a unified diff against the current spec
   file (not a rewritten file). @impl/done
2. ##PART-AN-INTENT-STATEMENT **An intent statement** — one sentence per logical change, naming
   *why* the code changed. @impl/done
3. ##PART-A-REVISIT-TRIGGER **A revisit trigger** — the condition under which the decision
   should be re-examined. @impl/done

##IF-ANY-OF-THE-THREE-IS-MISSING-THE-PROPOSAL-IS-INCOMPLETE If any of the three is missing, the proposal is incomplete. @impl/done

##ASK-THE-AGENT-TO-FILL-THE-GAP-BEFORE-APPROVING Ask the
agent to fill the gap before approving. @impl/done

##DO-NOT-APPROVE-AN-INCOMPLETE-SYNC Do not approve an incomplete
sync: a missing reason or trigger today is a lost decision in six
months. @impl/done

## The review checklist {#checklist}

##RUN-EVERY-ITEM-BEFORE-APPROVING Run every item before approving. @impl/done

### 1. Does the intent match reality? {#check-intent}

##READ-THE-INTENT-AGAINST-WHAT-YOU-REMEMBER-DOING Read the intent sentence against what you remember doing. @impl/done

##CORRECT-A-MISMATCHED-INTENT-BEFORE-APPROVING If the agent
wrote "changed to 600 s because users on VPN need more time" and you
actually changed it because "300 s was arbitrary and 600 s felt
safer", correct the intent before approving. @impl/done

##AN-INCORRECT-INTENT-IS-THE-MOST-DANGEROUS-THING-A-SYNC-CAN-LAND An incorrect intent is
the single most dangerous thing a sync can land — it encodes a
fiction that the future reader will trust. @impl/done

### 2. Is the reason durable? {#check-durability}

##ASK-WILL-THIS-REASON-STILL-BE-VALID-IN-A-YEAR Ask: will this reason still be valid in a year? @impl/done

- ##example-durable-profiling-spike "Profiling showed a 30 % hot-path spike on this branch" ← yes. @impl/done
- ##example-non-durable-i-was-testing-something "I was testing something" ← no. Revert the code. @impl/done
- ##example-durable-library-api-change "The library we use changed its public API in 0.9" ← yes, but cite
  the library and the version. @impl/done

##a-non-durable-reason-means-a-non-durable-change A non-durable reason means the code change itself is probably
non-durable. @spec/done

##REVERT-RATHER-THAN-SYNC Revert rather than sync. @impl/done

### 3. Is the revisit trigger concrete? {#check-trigger}

##WHEN-IT-BREAKS-IS-NOT-A-TRIGGER "When it breaks" is not a trigger. @impl/done

##a-trigger-is-a-measurable-signal-lead A trigger is a measurable signal: @impl/done

- ##example-trigger-latency-threshold "When p99 network latency drops below 100 s, per mon/latency-p99" ← good. @impl/done
- ##example-trigger-cpu-threshold "When CPU usage exceeds 80 % on the hot path" ← good. @impl/done
- ##example-trigger-later-or-at-some-point "Later" / "at some point" / "when we refactor" ← bad. Rewrite. @impl/done

##NO-TRIGGER-MEANS-NO-AUDIT-PATH No trigger means no audit path. @impl/done

##a-permanent-fact-with-a-provisional-label You are shipping a permanent fact
with a provisional label. @spec/done

### 4. Does the diff touch only the affected section? {#check-scope}

##A-SYNC-THAT-REACHES-BEYOND-THE-AFFECTED-SECTION-IS-OUT-OF-SCOPE A sync that also reflows paragraphs, renames anchors, or reorders
unrelated sections is out of scope. @impl/done

##REJECT-AND-ASK-FOR-A-NARROW-DIFF Reject and ask for a narrow diff
that changes only what the code change demands. @impl/done

##omnibus-spec-edits-make-history-unbisectable Omnibus spec edits
are how history becomes unbisectable. @spec/done

### 5. Is the anchor citation correct? {#check-anchor}

##THE-SPEC-DIFF-NAMES-THE-AFFECTED-ANCHOR-EXACTLY The spec diff should name the affected anchor (`{#verification.timeout}`)
exactly. @impl/done

##AN-IMPLEMENTS-MARKER-MUST-MATCH-LETTER-FOR-LETTER If the code carries an `// Implements: spec://…` marker,
the anchor in the marker must match letter-for-letter. @impl/done

##stale-markers-rot-spec-to-code-traceability-silently Stale markers
are how spec-to-code traceability rots silently. @spec/done

### 6. Is the scope of the code change what you expected? {#check-surprise}

##SKIM-THE-ACTUAL-DIFF-NOT-JUST-THE-PROPOSAL Before approving, skim the actual `git diff` — not just the sync
proposal. @impl/done

##AN-UNEXPECTED-SECOND-FILE-IS-EITHER-HIDDEN-OR-UNRELATED If the code touched a second file you did not expect, that
second change is either (a) hidden in the proposal and the sync is
incomplete, or (b) unrelated and should have been a separate change. @impl/done

##HANDLE-THE-SURPRISE-BEFORE-APPROVING-THE-SYNC Either way, handle the surprise before approving the sync. @impl/done

## On approval {#approve}

##the-agent-lead The agent: @impl/done

1. ##APPROVAL-STEP-APPLIES-THE-SPEC-DIFF Applies the spec diff. @impl/done
2. ##APPROVAL-STEP-COMMITS-WITH-CONVENTIONAL-COMMITS Commits with Conventional Commits format (`docs(spec)` type), a body
   that cites the code change driving the sync, and the `spec://…` URI
   of the affected anchor: @impl/done

   ```
   docs(spec): sync timeout into PROP-003 §verification.timeout

   Code changed TIMEOUT from 300 s to 600 s after VPN latency
   measurement (2026-03-05, 847 messages, 128 users). Spec now
   carries the value, the reason, and the revisit trigger.
   Cited by spec://oproto/PROP-003#verification.timeout.
   ```

3. ##APPROVAL-STEP-STOPS Stops. Does not continue into unrelated follow-up work in the
   same run — a sync is its own atomic step. @impl/done

## On rejection {#reject}

##two-paths-lead Two paths: @impl/done

- ##REJECTION-PATH-KEEP-THE-CODE **Reject the sync, keep the code.** The code change was right but
  the proposal's framing was wrong. Ask the agent to redraft with
  the correct intent. No revert. @impl/done
- ##REJECTION-PATH-REVERT-THE-CODE **Reject the sync, revert the code.** The code change itself was
  the problem — the proposal surfaced it. Revert with `git revert`
  or `git checkout --`, and record the lesson in the WAL's Known
  Issues if it is worth carrying forward. @impl/done

##NEITHER-PATH-SILENTLY-ACCEPTS-A-BAD-SYNC Neither path silently accepts a bad sync. @impl/done

##a-silently-accepted-bad-sync-makes-the-spec-fiction A silently accepted bad
sync is how the spec becomes fiction. @spec/done

## Why the checklist is long {#why-long}

##SYNC-IS-THE-ONE-PROTOCOL-THAT-WRITES-SPEC-DRIVEN-BY-CODE Sync-from-Code is the one protocol in the project that writes a
spec change *driven by code*. @impl/done

##EVERY-OTHER-SPEC-CHANGE-IS-HUMAN-INITIATED-FROM-INTENT Every other spec change is
human-initiated from intent. @impl/done

##a-weaker-driver-demands-a-stronger-approval-step Because the driver is weaker — reverse
engineering of intent from a diff — the approval step has to be
stronger. @spec/done

##six-checks-is-not-bureaucracy Six checks is not bureaucracy; it is the reason this flow
does not produce drift of its own. @spec/done
