# Flow: Sync-from-Code {#root}

This project uses the **Sync-from-Code** protocol to reconcile specs with
code when the code changed first.

## Default direction is unchanged

Information flows top-down: head → WAL → spec → code. Sync-from-Code does
not flip that rule. It is the **exceptional path** for two legitimate
cases where the bottom layer moves before the layer above it:

- The user edited code directly because writing five lines in an editor
  was faster than articulating the intent in a spec edit first.
- The user told the agent "change the timeout to 600 s" (or similar) in
  chat. The agent changed the code; nobody touched the spec.

In both cases the spec is now stale. Without Sync-from-Code, the next
session reads the stale spec, concludes the code is wrong, and "fixes"
it back — correctly by the spec-wins rule, but wrong in outcome.

## Trigger

Run the protocol exactly once, as the last step of the same session that
caused the drift. Do not postpone it to "tomorrow" — drift accumulates.

1. Read `git diff HEAD` and reconstruct the *intent* behind each change.
2. Draft a diff against the relevant spec section. Include: new value,
   reason, and the condition under which the decision should be
   revisited.
3. Surface the spec-diff draft to the user. **Do not apply.**
4. On approval, apply + commit (`docs(spec): sync <section> with code`).
   On reject, either revert the code or redraft the proposal.

Full protocol: [`spec/flows/sync-from-code/SYNC-PROTOCOL.md`](../flows/sync-from-code/SYNC-PROTOCOL.md).

## Never

- Never silently update a spec to match the code. A silent update erases
  the human's mental model of what the project intends.
- Never use Sync-from-Code to paper over a temporary hack. For
  throwaway debug code, record in the WAL:
  `<file>: temporary, do not sync to spec`.
- Never batch two unrelated code changes into one spec edit. One intent
  per sync run.

Decision table for whether to run: [`spec/flows/sync-from-code/when-to-apply.md`](../flows/sync-from-code/when-to-apply.md).
Human review checklist: [`spec/flows/sync-from-code/review-workflow.md`](../flows/sync-from-code/review-workflow.md).
