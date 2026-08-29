---
name: steward-handoff
description: Prepare, receive, or recover a same-machine central-agent custody handoff while preserving the complete plan and acceptance boundary. Use only when the owner requests a central session/model/harness transfer or recovery.
---

<status stage="impl" state="done"/>

# Steward handoff {#root}

@fact:HANDOFF-CENTRAL-ONLY This skill transfers owner-facing central custody,
not work to a subagent. It never delegates acceptance. @status:impl/done

## Select the operation {#operation}

- @fact:HANDOFF-PREPARE **Prepare** when the current session holds custody and
  the owner wants another central session/model/harness to continue.
  @status:impl/done
- @fact:HANDOFF-RECEIVE **Receive** when an unreceipted offer targets this
  session. @status:impl/done
- @fact:HANDOFF-RECOVER **Recover** only when the holder is unavailable and the
  owner explicitly authorizes takeover of the named context. @status:impl/done

## Prepare {#prepare}

@fact:HANDOFF-PREPARE-PROCEDURE Follow
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/custody-and-handoff#offer`:
verify the tree; backscan mandate/plan/evidence/artifacts; write the immutable
offer and comprehensive `HANDOFF.md`; hash both handoff and plan; set custody
to `offering`; then become repository- and plan-read-only. A summary without
that fence is not a handoff. @status:impl/done

## Receive {#receive}

@fact:HANDOFF-RECEIVE-PROCEDURE Read all authorities and the complete bundle,
verify hashes and repository state independently, enumerate discrepancies,
write `accepted`, `accepted-with-exceptions`, or `rejected` receipt, and advance
custody only for an accepted receipt. Preserve all dirty/worker work as
candidate. Report restored state and wait for the owner before execution.
@status:impl/done

## Recover {#recover}

@fact:HANDOFF-RECOVER-PROCEDURE Preserve artifacts, verify holder
unavailability, write a takeover record with the owner's authorization and
observed accepted boundary, advance epoch, and treat everything after durable
project acceptance as candidate. Never delete first or use staleness alone as
authority. @status:impl/done
