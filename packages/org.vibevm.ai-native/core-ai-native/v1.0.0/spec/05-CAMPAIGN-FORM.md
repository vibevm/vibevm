# The Campaign Form — plans, baselines, and the paper trail {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · status: BETA · T1 · language-neutral** @status:impl/done

@fact:RAID-PLAYBOOK-GIVES-THE-IN-FLIGHT-SKELETON *The [Raid Playbook](03-RAID-PLAYBOOK.md) §1 gives the in-flight skeleton of
a campaign (scope & freeze → card set & order → phases → batches &
checkpoints → differential safety → exit criteria).* @status:impl/done

@fact:THIS-DOCUMENT-IS-THE-PAPER-TRAIL *This document is the
campaign's **paper trail**: the artifacts a campaign writes so it can be
planned cold, executed by someone (or some session) other than its author,
paused at any phase boundary, and audited afterwards.* @status:impl/done

@fact:greenfield-terraform-proved-this-machinery *The original
greenfield terraform proved this machinery; brownfield adoption
([BROWNFIELD](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md)) and every relocation
or drain campaign since reuse it.* @status:spec/done

@fact:historical-instances-live-in-the-pilot-tree *Historical instances live in the pilot
project's tree; this is the distilled form.* @status:impl/done

## 1. The artifact set {#artifacts}

| Artifact | Written | Purpose |
|---|---|---|
| @fact:ROW-ARTIFACT-PLAN **PLAN** @status:impl/done | before work starts @status:impl/done | the cold-executable recipe (see §2) @status:impl/done |
| @fact:ROW-ARTIFACT-BASELINE **BASELINE** @status:impl/done | at phase −1 / campaign open @status:impl/done | the frozen starting facts: gate panel state, inventory counts, the numbers phases are measured against @status:impl/done |
| @fact:ROW-ARTIFACT-PREDICTIONS **PREDICTIONS** @status:impl/done | at campaign open @status:impl/done | falsifiable expectations — what each phase should change, stated *before* execution so the REPORT can honestly say "confirmed / falsified / surprised" @status:impl/done |
| @fact:ROW-ARTIFACT-LOG **LOG** @status:impl/done | during execution @status:impl/done | append-only running record: per-phase entries, deviations from plan, discovered-necessary work, dead ends kept for the record @status:impl/done |
| @fact:ROW-ARTIFACT-REPORT **REPORT** @status:impl/done | at campaign close @status:impl/done | what the campaign taught: results vs PREDICTIONS, cards/rules that misfired, lessons that feed Discipline revision @status:impl/done |

@fact:SMALL-CAMPAIGNS-MAY-FOLD-BUT-NOT-SKIP Small campaigns may fold BASELINE and PREDICTIONS into the PLAN's
current-state section and the LOG into commit messages — but a campaign that
skips the REPORT learns nothing durable, and a campaign without a written
PLAN cannot be resumed by anyone but its author's context window. @status:impl/done

## 2. The plan shape {#plan}

@fact:PLAN-IS-WRITTEN-TO-BE-EXECUTED-COLD A campaign plan is written **to be executed cold** — by a fresh session with
no memory of its authoring. @status:impl/done

@fact:load-bearing-sections-lead The load-bearing sections: @status:impl/done

1. @fact:PLAN-SECTION-READ-FIRST **Read-first / boot note** — what to read before this file (the project's
   boot sequence), and the rule that the project's living state supersedes
   the plan where they diverge. @status:impl/done
2. @fact:PLAN-SECTION-WHY-THIS-EXISTS **Why this exists** — the reframe: what debt or gap the campaign pays,
   in one screen. @status:impl/done
3. @fact:PLAN-SECTION-DIRECTIVES-IN-FORCE **Directives / decisions in force** — the owner's binding choices, so the
   executor never re-litigates them. @status:impl/done
4. @fact:PLAN-SECTION-CURRENT-STATE-FACTS **Current-state facts, verified** — with file:line pointers, gathered at
   authoring time and marked "do not re-discover". Wrong facts here are the
   most expensive class of plan bug: verify while writing. @status:impl/done
5. @fact:PLAN-SECTION-TARGET-END-STATE **Target end-state** — the "what done looks like" tree/diagram. @status:impl/done
6. @fact:PLAN-SECTION-DESIGN-DECISIONS **Design decisions** — each with its rationale and its rejected
   alternative, so a surprise mid-execution can be resolved in the spirit of
   the plan. @status:impl/done
7. @fact:PLAN-SECTION-PHASES **Phases** — each with: goal, recipe (commands, paths), **its own
   acceptance gate**, and **its own commit set**. A phase is a safe stop; a
   failed phase rolls back without losing prior ones. @status:impl/done
8. @fact:PLAN-SECTION-RISKS-AND-FALLBACKS **Risks & fallbacks** — named, each with its detection signal and its
   plan-B. @status:impl/done
9. @fact:PLAN-SECTION-QUICK-START **Quick-start** — the copy-paste block the executing session runs first
   (floor verification, baseline capture). @status:impl/done
10. @fact:PLAN-SECTION-WHOLE-CAMPAIGN-ACCEPTANCE **Whole-campaign acceptance** — the end-to-end scenario that defines
    done, ideally frozen as a test. @status:impl/done

## 3. Phase-gate discipline {#gates}

- @fact:GATE-FLOOR-GREEN-AT-EVERY-BOUNDARY **The floor is green at every phase boundary** — the project's full gate
  panel, not just the parts the phase touched. A phase that leaves the floor
  red is not done. @status:impl/done
- @fact:GATE-EACH-PHASE-LANDS-ITS-OWN-COMMITS **Each phase lands its own topic-grouped commits** (one logical unit per
  commit). The git log is the authoritative per-item record; the LOG
  narrates, the commits *are* the history. @status:impl/done
- @fact:GATE-BEHAVIOR-CHANGES-CARRY-THEIR-ORACLE **Behavior changes carry their differential oracle**
  ([Raid Playbook](03-RAID-PLAYBOOK.md) §1.5): a campaign cannot move
  behavior silently. Where the campaign's point is byte-stability (a
  relocation, a rename), the byte-compare IS the oracle — state it in the
  phase's acceptance. @status:impl/done
- @fact:GATE-DISCOVERED-WORK-IS-LEGAL-BUT-RECORDED **Discovered-necessary work is legal but recorded**: a mid-phase discovery
  either enters the current phase explicitly (LOG entry + the phase's
  commits) or is filed as debt/intent for later — never done silently
  "while I was here". @status:impl/done

## 4. Resumability {#resume}

@fact:campaign-survives-stopping-lead A campaign must survive its executor stopping at any phase boundary
(see [06-WAL-CONVENTION](06-WAL-CONVENTION.md)): @status:impl/done

- @fact:CAMPAIGN-RESUME-WITH-A-WAL *With a WAL (recommended):* the campaign updates the WAL's standing line at
  every phase boundary (phase landed, floor state, next phase); a session
  resuming cold reads boot → WAL → the PLAN and continues at the recorded
  phase. @status:impl/done
- @fact:CAMPAIGN-RESUME-WITHOUT-A-WAL *Without a WAL:* the PLAN carries a **status line at its top** ("status:
  Phase N landed, floor green, next: Phase N+1") that the executor updates
  as part of each phase's commit set, and the LOG's last entry is the resume
  pointer. The rule is the same either way: **resumption state lives in the
  repository, never in a session.** @status:impl/done

## 5. Exit {#exit}

@fact:CAMPAIGN-CLOSES-ON-A-GREEN-FLOOR A campaign closes when its whole-campaign acceptance passes on a green
floor. @status:impl/done

@fact:closing-motions-lead The closing motions: @status:impl/done

- @fact:CLOSING-WRITE-THE-REPORT write the REPORT (results vs PREDICTIONS,
  lessons, candidate Discipline revisions), @status:impl/done
- @fact:CLOSING-FILE-EVERY-LEFTOVER file every leftover as debt/intent
  with an id (the BROWNFIELD carry-over guarantee: nothing evaporates), @status:impl/done
- @fact:CLOSING-UPDATE-THE-RESUME-POINTER update the resume pointer to "closed", @status:impl/done
- @fact:CLOSING-PUBLISHING-IS-THE-OWNERS-CALL and — where the project mirrors its history —
  leave publishing/mirroring as the owner's explicit call, not the campaign's
  last step. @status:impl/done
