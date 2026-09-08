---
name: steward-status
description: Resolve and verify the current user-local stewardship context, then report the complete campaign frontier without starting work. Use at central-session start, after compaction, or when the owner asks where work stands.
---

<status stage="impl" state="done"/>

# Steward status {#root}

@fact:STATUS-CENTRAL-ONLY This skill is for an owner-facing central session.
If the current task is a worker/reviewer packet, report that the packet—not
central custody—governs and stop. @status:impl/done

## Procedure {#procedure}

1. @fact:STATUS-RESOLVE Resolve the exact repository/worktree/revision binding
   under `~/.vibe/steward/contexts/*/binding.toml`. An explicit context named by
   the owner wins. Do not guess between duplicate exact matches.
   @status:impl/done
2. @fact:STATUS-READ Read global/context settings, custody, the whole
   `plan.toml`, and the repository's actual branch, HEAD, ahead/behind and dirty
   state. Read handoff material only when the owner explicitly requests formal
   handoff inspection or receipt. Session/handoff/takeover records are not
   prerequisites for ordinary status or continuation. @status:impl/done
3. @fact:STATUS-VERIFY Compare stored claims with the tree. Separate
   project-accepted evidence from local claims and candidates; warn first when
   a valid explicit formal offer is active, an actual conflicting writer is
   observed, or inputs move. `vacant`, `held`, stale heartbeat and mismatched
   holder/session are advisory diagnostics, not authority or a write gate.
   @status:impl/done
4. @fact:STATUS-BACKSCAN Check every non-terminal mandate item has a plan node
   and every candidate artifact named by the plan still exists. Do not pay or
   repair the debt during this status-only skill. @status:impl/done

## Output {#output}

@fact:STATUS-OUTPUT Report: context and effective modes; custody holder/epoch;
repository state; accepted boundary; candidate/unaccepted work; full remaining
epic route in compressed form; expanded current frontier; blockers and the
candidate next atom. Then stop and wait for owner direction. Do not edit, claim
custody, run implementation or declare acceptance. @status:impl/done

@fact:STATUS-VERSUS-CONTINUE The wait above belongs to a status-only request.
When the owner's current request is to continue work, ordinary continuation may
proceed after the same resolution and verification without a custody claim or
handoff ritual; an observed live conflicting writer or moving input still
stops writes. @status:impl/done
