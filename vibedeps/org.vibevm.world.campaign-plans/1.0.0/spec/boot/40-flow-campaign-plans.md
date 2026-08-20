# Flow: Campaign Plans {#root}

<status stage="impl" state="done"/>

@fact:LARGE-CHANGES-ARE-EXECUTED-AS-CAMPAIGNS This project executes large changes as **campaigns**: multi-commit
work planned in a written campaign plan, executed as gated phases,
resumable cold by any session. @status:impl/done

## When to propose a campaign {#when}

@fact:PROPOSE-A-CAMPAIGN-BEFORE-TOUCHING-THE-TREE When the owner commissions work that spans **more than one session or
more than a handful of commits**, propose a campaign plan before
touching the tree. @status:impl/done

@fact:the-plan-carries-five-roles The plan is one document carrying five roles: @status:impl/done

- @fact:ROLE-PLAN the
  recipe (PLAN), @status:impl/done
- @fact:ROLE-BASELINE the frozen starting numbers (BASELINE), @status:impl/done
- @fact:ROLE-PREDICTIONS falsifiable
  expectations (PREDICTIONS), @status:impl/done
- @fact:ROLE-LOG the running record (LOG), @status:impl/done
- @fact:ROLE-REPORT and the closing
  verdict (REPORT). @status:impl/done

@fact:format-pointer Format:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT#root. @status:impl/done

## The plan runs cold {#cold}

@fact:write-the-plan-to-run-cold-lead Write the plan so a fresh session — or a different person — executes
it with no memory of the planning conversation: @status:impl/done

- @fact:COLD-MANDATE-QUOTED-VERBATIM-AND-DATED the owner's mandate
  quoted verbatim and dated, @status:impl/done
- @fact:COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS baseline and exit state as exact counts, @status:impl/done
- @fact:COLD-FACTS-VERIFIED-AT-WRITING-TIME
  current-state facts verified at writing time, @status:impl/done
- @fact:COLD-DECISIONS-WITH-REJECTED-OPTIONS-AND-REASONS decisions with their
  rejected options and reasons, @status:impl/done
- @fact:COLD-A-LITERAL-QUICK-START-BLOCK a literal quick-start block, @status:impl/done
- @fact:COLD-A-RUNNABLE-ACCEPTANCE-SCRIPT and a
  runnable whole-campaign acceptance script. @status:impl/done

@fact:WRONG-CURRENT-STATE-FACTS-ARE-THE-MOST-EXPENSIVE-PLAN-BUG Wrong current-state facts
are the most expensive class of plan bug — verify while writing. @status:spec/done

## Phases gate on green {#gates}

@fact:PHASE-ZERO-COMMITS-NOTHING-AND-GATES-EVERYTHING-AFTER Phase 0 is spikes and probes — no commits — and it gates everything
after. @status:impl/done

@fact:EVERY-LATER-PHASE-ENDS-GREEN-AND-EVERY-BOUNDARY-IS-A-SAFE-STOP Every later phase ends with the project's full gate panel
green, and any phase boundary is a safe stop; the plan plus its
execution ledger are the resume pointer. @status:impl/done

@fact:sibling-document-pointers Mechanics:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/phase-gates#root;
the record half:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/execution-ledger#root. @status:impl/done

## At every phase boundary {#boundary}

1. @fact:BOUNDARY-RUN-THE-FULL-GATE-PANEL Run the full gate panel; the floor must be green. @status:impl/done
2. @fact:BOUNDARY-WRITE-THE-COMMIT-MAP-ENTRY Write the phase's commit-map entry in the execution ledger —
   hashes, subjects, what each commit confirmed or falsified. @status:impl/done
3. @fact:BOUNDARY-REFRESH-THE-STATUS-LINE Refresh the plan's status line ("Phase N landed, floor green,
   next: Phase N+1"). @status:impl/done
4. @fact:BOUNDARY-ESCALATE-OWNER-ONLY-DECISIONS Escalate anything only the owner can decide as a review point:
   OPEN, then RESOLVED with the ruling verbatim. @status:impl/done

## Never {#never}

- @fact:NEVER-START-PHASE-ONE-ON-A-RED-SPIKE Never start Phase 1 while a Phase 0 spike is red — a red spike
  rewrites the affected Decision first, in the plan, in place. @status:impl/done
- @fact:NEVER-COMMIT-DURING-PHASE-ZERO Never commit during Phase 0. Spikes leave findings, not tree
  changes. @status:impl/done
- @fact:NEVER-DO-DISCOVERED-WORK-SILENTLY Never do discovered work silently "while I was here" — it enters
  the phase and the ledger explicitly, or it is deferred by name. @status:impl/done
- @fact:NEVER-CLOSE-A-CAMPAIGN-WITHOUT-THE-REPORT Never close a campaign without the report checking every
  prediction — a campaign that skips the report learns nothing
  durable. @status:impl/done
- @fact:NEVER-CARRY-A-DEFERRAL-OUTSIDE-THE-PLAN-FILE Never carry a deferral outside the plan file — the deferrals
  ledger is where deferrals live, and the next campaign's mandate
  drains from it. @status:impl/done
