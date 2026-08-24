# Sync-from-Code review workflow {#root}

Sync-from-Code always ends with a human approval step. This document
is the checklist the human runs at that step.

## What the agent hands you {#input}

A proposal in three parts:

1. **A spec diff**, shown as a unified diff against the current spec
   file (not a rewritten file).
2. **An intent statement** — one sentence per logical change, naming
   *why* the code changed.
3. **A revisit trigger** — the condition under which the decision
   should be re-examined.

If any of the three is missing, the proposal is incomplete. Ask the
agent to fill the gap before approving. Do not approve an incomplete
sync: a missing reason or trigger today is a lost decision in six
months.

## The review checklist {#checklist}

Run every item before approving.

### 1. Does the intent match reality? {#check-intent}

Read the intent sentence against what you remember doing. If the agent
wrote "changed to 600 s because users on VPN need more time" and you
actually changed it because "300 s was arbitrary and 600 s felt
safer", correct the intent before approving. An incorrect intent is
the single most dangerous thing a sync can land — it encodes a
fiction that the future reader will trust.

### 2. Is the reason durable? {#check-durability}

Ask: will this reason still be valid in a year?

- "Profiling showed a 30 % hot-path spike on this branch" ← yes.
- "I was testing something" ← no. Revert the code.
- "The library we use changed its public API in 0.9" ← yes, but cite
  the library and the version.

A non-durable reason means the code change itself is probably
non-durable. Revert rather than sync.

### 3. Is the revisit trigger concrete? {#check-trigger}

"When it breaks" is not a trigger. A trigger is a measurable signal:

- "When p99 network latency drops below 100 s, per mon/latency-p99" ← good.
- "When CPU usage exceeds 80 % on the hot path" ← good.
- "Later" / "at some point" / "when we refactor" ← bad. Rewrite.

No trigger means no audit path. You are shipping a permanent fact
with a provisional label.

### 4. Does the diff touch only the affected section? {#check-scope}

A sync that also reflows paragraphs, renames anchors, or reorders
unrelated sections is out of scope. Reject and ask for a narrow diff
that changes only what the code change demands. Omnibus spec edits
are how history becomes unbisectable.

### 5. Is the anchor citation correct? {#check-anchor}

The spec diff should name the affected anchor (`{#verification.timeout}`)
exactly. If the code carries an `// Implements: spec://…` marker,
the anchor in the marker must match letter-for-letter. Stale markers
are how spec-to-code traceability rots silently.

### 6. Is the scope of the code change what you expected? {#check-surprise}

Before approving, skim the actual `git diff` — not just the sync
proposal. If the code touched a second file you did not expect, that
second change is either (a) hidden in the proposal and the sync is
incomplete, or (b) unrelated and should have been a separate change.
Either way, handle the surprise before approving the sync.

## On approval {#approve}

The agent:

1. Applies the spec diff.
2. Commits with Conventional Commits format (`docs(spec)` type), a body
   that cites the code change driving the sync, and the `spec://…` URI
   of the affected anchor:

   ```
   docs(spec): sync timeout into PROP-003 §verification.timeout

   Code changed TIMEOUT from 300 s to 600 s after VPN latency
   measurement (2026-03-05, 847 messages, 128 users). Spec now
   carries the value, the reason, and the revisit trigger.
   Cited by spec://oproto/PROP-003#verification.timeout.
   ```

3. Stops. Does not continue into unrelated follow-up work in the
   same run — a sync is its own atomic step.

## On rejection {#reject}

Two paths:

- **Reject the sync, keep the code.** The code change was right but
  the proposal's framing was wrong. Ask the agent to redraft with
  the correct intent. No revert.
- **Reject the sync, revert the code.** The code change itself was
  the problem — the proposal surfaced it. Revert with `git revert`
  or `git checkout --`, and record the lesson in the WAL's Known
  Issues if it is worth carrying forward.

Neither path silently accepts a bad sync. A silently accepted bad
sync is how the spec becomes fiction.

## Why the checklist is long {#why-long}

Sync-from-Code is the one protocol in the project that writes a
spec change *driven by code*. Every other spec change is
human-initiated from intent. Because the driver is weaker — reverse
engineering of intent from a diff — the approval step has to be
stronger. Six checks is not bureaucracy; it is the reason this flow
does not produce drift of its own.
