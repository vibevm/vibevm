# Sync-from-Code Protocol {#root}

**Scope of this document.** This file defines *what* Sync-from-Code is,
*when* it fires, *what it must produce*, and *where it stops*. It is
the only sanctioned way to close a spec/code gap that was opened by a
bottom-up edit.

## What Sync-from-Code is {#what}

The **normal** information flow in a spec-driven project is top-down:

```
head  →  WAL  →  spec  →  code
```

Intent forms in the human's head; short-lived state lands in the WAL;
decisions harden into spec; the spec is then implemented in code. Code
is the artefact, not the source.

**Sync-from-Code is the protocol for the inverse case**: the code
changed first, and the spec must now follow. It is deliberately the
exception — the whole rest of the discipline pushes the other way.

## Why it exists {#why}

Two everyday situations break the top-down flow:

- **Direct editing.** The human opens the file and edits. Usually because
  it is faster than writing an intent for the agent to execute. Often
  the right call. The issue is not that it happened — the issue is that
  nothing updated the spec.
- **Imperative chat commands.** The human tells the agent "change the
  timeout to 600 s" or "use blake3 instead of SHA-256". The agent does
  the work. Again, legitimate — nobody wants to draft a PROP revision
  for a five-second decision — but the spec is now wrong.

Left unreconciled, either case produces spec drift. The next session
reads the stale spec, sees the 300 s figure, concludes that the code's
600 s is a bug, and "corrects" it. The agent is technically right by
the spec-wins hierarchy. The real bug is upstream: the spec lied.

Sync-from-Code is how the drift is closed *on purpose* rather than
letting it accumulate until a session triggers a wrong-direction "fix".

## When to run {#trigger}

Run the protocol at the end of the **same session** that produced the
code change. Waiting even a day is how drift accumulates — by then
other sessions have read the stale spec and made decisions on top of
it, and now two things need reconciling instead of one.

Do **not** run the protocol for:

- **Temporary hacks.** Debug `println!`s, throwaway probes, reproducers
  that will be reverted within the hour. Record the skip explicitly in
  the WAL so the next session does not try to sync the hack into the
  spec:

  ```markdown
  ## Constraints
  - src/verify.rs: temporary trace logging for issue #42, do NOT sync.
  ```

- **Mechanical changes.** `cargo fmt`, import reordering, dead-code
  removal flagged by the compiler, rename of a private symbol with no
  spec-level contract. Mechanical changes are below the spec's level of
  resolution.

- **Code that implements something the spec does not mention.** This is
  a forward-flow case, not a sync case: draft a new spec section from
  scratch, then reconcile. Sync-from-Code updates existing spec entries,
  it does not bootstrap them.

Edge case: see [`when-to-apply.md`](when-to-apply.md) for the full
decision table.

## Procedure {#procedure}

Four steps, always ending in a human approval.

### 1. Collect the diff {#step-collect}

```
git diff HEAD
```

…or a wider range (`git diff <base>..HEAD`) if several commits have
landed since the last spec-aligned state. The diff is the only source
of truth for what actually changed.

### 2. Reconstruct intent {#step-intent}

For each logical hunk, name the *why* — not the *what*, which the diff
already shows. Example:

- *Diff:* `-const TIMEOUT: u64 = 300;` / `+const TIMEOUT: u64 = 600;`.
- *Intent:* observed false-positive TIMEOUTs for users on high-latency
  VPNs; 600 s is the empirical threshold at which the false-positive
  rate drops to zero.

The intent sentence is what lands in the spec — **not the diff itself**.
If you cannot name the intent in one sentence, stop and ask the human.
A sync with a missing or hand-waved intent encodes a fiction; better to
fail loudly than to record one.

### 3. Draft the spec delta {#step-draft}

Produce a unified-diff proposal against the relevant spec section. Not
a rewritten file. Three parts are mandatory in every sync:

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

1. **The new value.** The primary change, matching the code.
2. **The reason.** Concrete, measurable where possible — no "felt
   better". Cite data, an issue number, or a dated observation.
3. **The revisit trigger.** The condition under which this decision
   should be re-examined. A decision without a revisit trigger becomes
   a sacred cow.

Without any of the three, the draft is incomplete.

### 4. Surface for approval {#step-approve}

Print the proposed spec diff. Do not apply. Wait for an explicit "apply"
from the user.

On approval: write the diff, commit it using Conventional Commits. The
commit type is `docs(spec)`, the body names the driving code change:

```
docs(spec): sync timeout to 600s in PROP-003 §verification.timeout

Code changed TIMEOUT from 300 s to 600 s after VPN latency
measurement (2026-03-05, 847 messages, 128 users). Spec now
carries the new value, the reason, and the revisit trigger.
```

On reject: either revert the code (the code change itself was the
mistake) or redraft the sync proposal (the agent framed the intent
incorrectly). Never silently accept a rejected sync.

The full human-side checklist lives in [`review-workflow.md`](review-workflow.md).

## What Sync-from-Code does not do {#non-goals}

- **Does not rewrite code to match the spec.** The opposite direction —
  generation — is a separate flow (`vibe build` territory).
- **Does not edit the WAL.** A successful sync may later trigger a WAL
  update ("PROP-003 §timeout synced with code"), but that is a separate
  step, handled by `flow:wal`'s session-end protocol.
- **Does not batch.** One intent per run. Batching two unrelated code
  changes into one spec edit defeats intent-per-decision and makes
  future audits impossible.

## Summary {#summary}

- Normal direction is spec → code; Sync-from-Code is the exception.
- Runs once, immediately, in the session that caused the drift.
- Output: a spec diff with new value + reason + revisit trigger.
- Always ends with human approval. Never silent, never batched, never
  applied to temporary code.
