# Flow: Sync-from-Code {#root}

<status stage="impl" state="done"/>

@fact:THE-PROJECT-USES-SYNC-FROM-CODE This project uses the **Sync-from-Code** protocol to reconcile specs with
code when the code changed first. @status:impl/done

## Default direction is unchanged {#direction}

@fact:INFORMATION-FLOWS-TOP-DOWN Information flows top-down: head → WAL → spec → code. @status:impl/done

@fact:SYNC-FROM-CODE-DOES-NOT-FLIP-THAT-RULE Sync-from-Code does
not flip that rule. @status:impl/done

@fact:the-exceptional-path-for-two-legitimate-cases-lead It is the **exceptional path** for two legitimate
cases where the bottom layer moves before the layer above it: @status:impl/done

- @fact:CASE-THE-USER-EDITED-CODE-DIRECTLY The user edited code directly because writing five lines in an editor
  was faster than articulating the intent in a spec edit first. @status:spec/done
- @fact:CASE-THE-USER-GAVE-AN-IMPERATIVE-CHAT-COMMAND The user told the agent "change the timeout to 600 s" (or similar) in
  chat. The agent changed the code; nobody touched the spec. @status:spec/done

@fact:in-both-cases-the-spec-is-now-stale In both cases the spec is now stale. @status:spec/done

@fact:without-sync-from-code-the-next-session-fixes-the-code-back Without Sync-from-Code, the next
session reads the stale spec, concludes the code is wrong, and "fixes"
it back — correctly by the spec-wins rule, but wrong in outcome. @status:spec/done

## Trigger {#trigger}

@fact:RUN-THE-PROTOCOL-EXACTLY-ONCE-IN-THE-SAME-SESSION Run the protocol exactly once, as the last step of the same session that
caused the drift. @status:impl/done

@fact:DO-NOT-POSTPONE-IT-DRIFT-ACCUMULATES Do not postpone it to "tomorrow" — drift accumulates. @status:impl/done

1. @fact:STEP-READ-THE-DIFF-AND-RECONSTRUCT-INTENT Read `git diff HEAD` and reconstruct the *intent* behind each change. @status:impl/done
2. @fact:STEP-DRAFT-A-DIFF-AGAINST-THE-SPEC-SECTION Draft a diff against the relevant spec section. Include: new value,
   reason, and the condition under which the decision should be
   revisited. @status:impl/done
3. @fact:STEP-SURFACE-THE-DRAFT-AND-DO-NOT-APPLY Surface the spec-diff draft to the user. **Do not apply.** @status:impl/done
4. @fact:STEP-ON-APPROVAL-APPLY-AND-COMMIT-ON-REJECT-REVERT-OR-REDRAFT On approval, apply + commit (`docs(spec): sync <section> with code`).
   On reject, either revert the code or redraft the proposal. @status:impl/done

@fact:full-protocol-pointer Full protocol: @spec://org.vibevm.world/sync-from-code/flows/sync-from-code/SYNC-PROTOCOL#root. @status:impl/done

## Never {#never}

- @fact:NEVER-SILENTLY-UPDATE-A-SPEC-TO-MATCH-THE-CODE Never silently update a spec to match the code. A silent update erases
  the human's mental model of what the project intends. @status:impl/done
- @fact:NEVER-PAPER-OVER-A-TEMPORARY-HACK Never use Sync-from-Code to paper over a temporary hack. For
  throwaway debug code, record in the WAL:
  `<file>: temporary, do not sync to spec`. @status:impl/done
- @fact:NEVER-BATCH-TWO-UNRELATED-CODE-CHANGES Never batch two unrelated code changes into one spec edit. One intent
  per sync run. @status:impl/done

@fact:sibling-document-pointers Decision table for whether to run: @spec://org.vibevm.world/sync-from-code/flows/sync-from-code/when-to-apply#root.
Human review checklist: @spec://org.vibevm.world/sync-from-code/flows/sync-from-code/review-workflow#root. @status:impl/done
