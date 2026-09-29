---
name: steward-handoff
description: Prepare, receive, cancel, inspect, or recover an explicitly owner-requested same-machine formal central handoff while preserving the complete plan and acceptance boundary. Never use for ordinary timeout, crash, compaction, new-chat, or model-change continuation.
---

<status stage="impl" state="done"/>

# Steward handoff {#root}

@fact:HANDOFF-CENTRAL-ONLY This skill transfers owner-facing central custody,
not work to a subagent. It never delegates acceptance. @status:impl/done

@fact:HANDOFF-EXPLICIT-ONLY Invoke this skill only from explicit owner intent to
prepare, receive, cancel, inspect or recover a formal central handoff. Ordinary
continuation resolves binding, settings, complete plan and tree directly and
creates no session, handoff, receipt or takeover document. @status:impl/done

@fact:HANDOFF-CONTEXT-COMMAND Recognise `ACCEPT HANDOFF FROM
&lt;context-id&gt;` / `ПРИМИ ХЭНДОФФ ИЗ &lt;context-id&gt;`. Resolve exactly one
unreceipted, uncancelled offered bundle in that context; if several exist,
require an exact handoff id. Read the stored `HANDOFF.md`—never ask the owner to
paste it. An explicit cancelled id reports `HANDOFF_CANCELLED`.
@status:impl/done

## Select the operation {#operation}

- @fact:HANDOFF-PREPARE **Prepare** when the owner explicitly requests formal
  transfer from the current active central writer to another
  session/model/harness. Verify that no conflicting central writer is live;
  advisory holder/session metadata alone does not select the outgoing writer.
  @status:impl/done
- @fact:HANDOFF-RECEIVE **Receive** when an unreceipted offer targets this
  session. @status:impl/done
- @fact:HANDOFF-RECOVER **Recover** only when the holder is unavailable and the
  owner explicitly directs repair of an already-open formal handoff in the
  named context. Stale or mismatched custody outside such an offer is ordinary
  continuation, not takeover. @status:impl/done

## Prepare {#prepare}

@fact:HANDOFF-PREPARE-PROCEDURE Follow
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/custody-and-handoff#offer`:
verify the tree; backscan mandate/plan/evidence/artifacts; write the immutable
offer, comprehensive `HANDOFF.md`, and short receive-by-context pointer; refresh
and hash handoff, plan and selected goal; set custody to `offering`; then become
repository- and plan-read-only. A summary without the created file and fence is
not a handoff. @status:impl/done

@fact:HANDOFF-PREPARE-OUTPUT After sealing, always print exactly:
`HANDOFF CREATED`, `context-id: …`, `handoff-id: …`, and
`receive-command: ПРИМИ ХЭНДОФФ ИЗ …`. Printing is allowed after the write
fence; never make the owner search `~/.vibe` or paste `HANDOFF.md`.
@status:impl/done

## Receive {#receive}

@fact:HANDOFF-RECEIVE-PROCEDURE Read all authorities and the complete bundle,
verify hashes and repository state independently, enumerate discrepancies,
write `accepted`, `accepted-with-exceptions`, or `rejected` receipt, and advance
formal custody only for an accepted receipt. Preserve all dirty/worker work as
candidate. After the accepted receipt ends the `offering` fence, refresh goal,
print full `GOAL.md`, then print the exact
`GOAL-CLAUDE.txt` `/goal …` line separately. Report restored state and wait for
the owner before execution.
@status:impl/done

## Recover {#recover}

@fact:HANDOFF-RECOVER-PROCEDURE Preserve artifacts, verify holder
unavailability and the valid open formal offer, record the owner's repair or
cancellation direction and observed accepted boundary, then follow the formal
protocol's epoch transition. Treat everything after durable project acceptance
as candidate. Never delete first or use staleness alone as authority. If no
explicit formal offer is open, stop this skill and use ordinary continuation
without a takeover record. @status:impl/done
