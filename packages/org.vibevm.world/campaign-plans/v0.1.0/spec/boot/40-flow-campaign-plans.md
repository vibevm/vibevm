# Flow: Campaign Plans {#root}

This project executes large changes as **campaigns**: multi-commit
work planned in a written campaign plan, executed as gated phases,
resumable cold by any session.

## When to propose a campaign {#when}

When the owner commissions work that spans **more than one session or
more than a handful of commits**, propose a campaign plan before
touching the tree. The plan is one document carrying five roles: the
recipe (PLAN), the frozen starting numbers (BASELINE), falsifiable
expectations (PREDICTIONS), the running record (LOG), and the closing
verdict (REPORT). Format:
[`../flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md`](../flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md).

## The plan runs cold {#cold}

Write the plan so a fresh session — or a different person — executes
it with no memory of the planning conversation: the owner's mandate
quoted verbatim and dated, baseline and exit state as exact counts,
current-state facts verified at writing time, decisions with their
rejected options and reasons, a literal quick-start block, and a
runnable whole-campaign acceptance script. Wrong current-state facts
are the most expensive class of plan bug — verify while writing.

## Phases gate on green {#gates}

Phase 0 is spikes and probes — no commits — and it gates everything
after. Every later phase ends with the project's full gate panel
green, and any phase boundary is a safe stop; the plan plus its
execution ledger are the resume pointer. Mechanics:
[`../flows/campaign-plans/phase-gates.md`](../flows/campaign-plans/phase-gates.md);
the record half:
[`../flows/campaign-plans/execution-ledger.md`](../flows/campaign-plans/execution-ledger.md).

## At every phase boundary {#boundary}

1. Run the full gate panel; the floor must be green.
2. Write the phase's commit-map entry in the execution ledger —
   hashes, subjects, what each commit confirmed or falsified.
3. Refresh the plan's status line ("Phase N landed, floor green,
   next: Phase N+1").
4. Escalate anything only the owner can decide as a review point:
   OPEN, then RESOLVED with the ruling verbatim.

## Never {#never}

- Never start Phase 1 while a Phase 0 spike is red — a red spike
  rewrites the affected Decision first, in the plan, in place.
- Never commit during Phase 0. Spikes leave findings, not tree
  changes.
- Never do discovered work silently "while I was here" — it enters
  the phase and the ledger explicitly, or it is deferred by name.
- Never close a campaign without the report checking every
  prediction — a campaign that skips the report learns nothing
  durable.
- Never carry a deferral outside the plan file — the deferrals
  ledger is where deferrals live, and the next campaign's mandate
  drains from it.
