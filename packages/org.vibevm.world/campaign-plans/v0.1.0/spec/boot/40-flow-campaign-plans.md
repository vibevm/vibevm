# Flow: Campaign Plans {#root}

<status stage="impl" state="done"/>

##LARGE-CHANGES-ARE-EXECUTED-AS-CAMPAIGNS This project executes large changes as **campaigns**: multi-commit
work planned in a written campaign plan, executed as gated phases,
resumable cold by any session. @impl/done

## When to propose a campaign {#when}

##PROPOSE-A-CAMPAIGN-BEFORE-TOUCHING-THE-TREE When the owner commissions work that spans **more than one session or
more than a handful of commits**, propose a campaign plan before
touching the tree. @impl/done

##the-plan-carries-five-roles The plan is one document carrying five roles: @impl/done

- ##ROLE-PLAN the
  recipe (PLAN), @impl/done
- ##ROLE-BASELINE the frozen starting numbers (BASELINE), @impl/done
- ##ROLE-PREDICTIONS falsifiable
  expectations (PREDICTIONS), @impl/done
- ##ROLE-LOG the running record (LOG), @impl/done
- ##ROLE-REPORT and the closing
  verdict (REPORT). @impl/done

##format-pointer Format:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT#root. @impl/done

## The plan runs cold {#cold}

##write-the-plan-to-run-cold-lead Write the plan so a fresh session — or a different person — executes
it with no memory of the planning conversation: @impl/done

- ##COLD-MANDATE-QUOTED-VERBATIM-AND-DATED the owner's mandate
  quoted verbatim and dated, @impl/done
- ##COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS baseline and exit state as exact counts, @impl/done
- ##COLD-FACTS-VERIFIED-AT-WRITING-TIME
  current-state facts verified at writing time, @impl/done
- ##COLD-DECISIONS-WITH-REJECTED-OPTIONS-AND-REASONS decisions with their
  rejected options and reasons, @impl/done
- ##COLD-A-LITERAL-QUICK-START-BLOCK a literal quick-start block, @impl/done
- ##COLD-A-RUNNABLE-ACCEPTANCE-SCRIPT and a
  runnable whole-campaign acceptance script. @impl/done

##WRONG-CURRENT-STATE-FACTS-ARE-THE-MOST-EXPENSIVE-PLAN-BUG Wrong current-state facts
are the most expensive class of plan bug — verify while writing. @spec/done

## Phases gate on green {#gates}

##PHASE-ZERO-COMMITS-NOTHING-AND-GATES-EVERYTHING-AFTER Phase 0 is spikes and probes — no commits — and it gates everything
after. @impl/done

##EVERY-LATER-PHASE-ENDS-GREEN-AND-EVERY-BOUNDARY-IS-A-SAFE-STOP Every later phase ends with the project's full gate panel
green, and any phase boundary is a safe stop; the plan plus its
execution ledger are the resume pointer. @impl/done

##sibling-document-pointers Mechanics:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/phase-gates#root;
the record half:
@spec://org.vibevm.world/campaign-plans/flows/campaign-plans/execution-ledger#root. @impl/done

## At every phase boundary {#boundary}

1. ##BOUNDARY-RUN-THE-FULL-GATE-PANEL Run the full gate panel; the floor must be green. @impl/done
2. ##BOUNDARY-WRITE-THE-COMMIT-MAP-ENTRY Write the phase's commit-map entry in the execution ledger —
   hashes, subjects, what each commit confirmed or falsified. @impl/done
3. ##BOUNDARY-REFRESH-THE-STATUS-LINE Refresh the plan's status line ("Phase N landed, floor green,
   next: Phase N+1"). @impl/done
4. ##BOUNDARY-ESCALATE-OWNER-ONLY-DECISIONS Escalate anything only the owner can decide as a review point:
   OPEN, then RESOLVED with the ruling verbatim. @impl/done

## Never {#never}

- ##NEVER-START-PHASE-ONE-ON-A-RED-SPIKE Never start Phase 1 while a Phase 0 spike is red — a red spike
  rewrites the affected Decision first, in the plan, in place. @impl/done
- ##NEVER-COMMIT-DURING-PHASE-ZERO Never commit during Phase 0. Spikes leave findings, not tree
  changes. @impl/done
- ##NEVER-DO-DISCOVERED-WORK-SILENTLY Never do discovered work silently "while I was here" — it enters
  the phase and the ledger explicitly, or it is deferred by name. @impl/done
- ##NEVER-CLOSE-A-CAMPAIGN-WITHOUT-THE-REPORT Never close a campaign without the report checking every
  prediction — a campaign that skips the report learns nothing
  durable. @impl/done
- ##NEVER-CARRY-A-DEFERRAL-OUTSIDE-THE-PLAN-FILE Never carry a deferral outside the plan file — the deferrals
  ledger is where deferrals live, and the next campaign's mandate
  drains from it. @impl/done
