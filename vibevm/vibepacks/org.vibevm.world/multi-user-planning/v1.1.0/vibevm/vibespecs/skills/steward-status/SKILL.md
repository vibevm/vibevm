---
name: steward-status
description: Resolve and verify the current user-local stewardship context and report the campaign frontier. Use at central-session start, after interruption or compaction, or for a status request; continue execution only when the current owner request includes it.
---

<status stage="impl" state="done"/>

# Steward status {#root}

@fact:STATUS-CENTRAL-ONLY This skill is for an owner-facing central session.
If the current task is a worker/reviewer packet, report that the packet—not
central custody—governs and stop. @status:impl/done

## Procedure {#procedure}

@fact:STATUS-VERSUS-CONTINUE Determine the current owner's requested scope
first. A status, review or planning-only request ends with that deliverable;
neither a stored next atom nor a generated goal starts implementation. If the
request also authorizes continuation, report the verified state and continue
within that scope. A stop or narrower instruction takes precedence over the
standing campaign route. @status:impl/done

1. @fact:STATUS-RESOLVE Resolve the exact repository/worktree/revision binding
   under `~/.vibe/steward/contexts/*/binding.toml`. An explicit context named by
   the owner wins. Do not guess between duplicate exact matches.
   @status:impl/done
2. @fact:STATUS-READ On cold start, resume, compaction, or model/account change,
   read global/context settings, advisory custody, the whole
   `plan.toml`, and the repository's actual branch, HEAD, ahead/behind and dirty
   state. Inspect relevant surviving worker jobs and candidate artifacts before
   restarting work. In an uninterrupted session, reuse the already-read complete
   plan when its revision and relevant input changes are accounted for; unknown
   changes require a fresh read. Read handoff material only when the owner explicitly requests formal
   handoff inspection or receipt. Session/handoff/takeover records are not
   prerequisites for ordinary status or continuation. @status:impl/done
3. @fact:STATUS-VERIFY Compare stored claims with the tree. Separate
   project-accepted evidence from local claims and candidates; warn first when
   a valid explicit formal offer is active or an actual conflicting writer is
   observed. Recapture expected own writes and explained input changes; resolve
   unexplained semantic conflicts before writing. `vacant`, `held`, stale heartbeat and mismatched
   holder/session are advisory diagnostics, not authority or a write gate.
   Accepted evidence for an unchanged exact subject remains accepted; an
   interrupted thought or absent transcript does not prove acceptance or failure.
   Resolve uncertain tool outcomes before replay under
   `spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/interruption-recovery#root`.
   @status:impl/done
4. @fact:STATUS-BACKSCAN Check every non-terminal mandate item has a plan node
   and every candidate artifact named by the plan still exists. Do not pay or
   repair the debt during a status-only request. @status:impl/done

## Output {#output}

@fact:STATUS-OUTPUT Report: context and effective modes; custody holder/epoch;
repository state; accepted boundary; candidate/unaccepted work and surviving
jobs; full remaining
epic route in compressed form; expanded current frontier; blockers and the
candidate next atom. Status reporting does not change product work, plan nodes
or acceptance, or claim custody. Ordinary context resolution may create missing
documented defaults and refresh stale derived goals as the boot contract
requires; an explicitly read-only request or valid formal-offer fence instead
reports what is missing/stale without writing. Stop after a status-only report; when continuation is
already authorized, proceed after the report without requesting that authority
again or creating a handoff ritual. @status:impl/done
