# The Campaign Form — plans, baselines, and the paper trail {#root}
**Discipline v0.2 · status: BETA · T1 · language-neutral**

*The [Raid Playbook](03-RAID-PLAYBOOK.md) §1 gives the in-flight skeleton of
a campaign (scope & freeze → card set & order → phases → batches &
checkpoints → differential safety → exit criteria). This document is the
campaign's **paper trail**: the artifacts a campaign writes so it can be
planned cold, executed by someone (or some session) other than its author,
paused at any phase boundary, and audited afterwards. The original
greenfield terraform proved this machinery; brownfield adoption
([BROWNFIELD](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md)) and every relocation
or drain campaign since reuse it. Historical instances live in the pilot
project's tree; this is the distilled form.*

## 1. The artifact set {#artifacts}

| Artifact | Written | Purpose |
|---|---|---|
| **PLAN** | before work starts | the cold-executable recipe (see §2) |
| **BASELINE** | at phase −1 / campaign open | the frozen starting facts: gate panel state, inventory counts, the numbers phases are measured against |
| **PREDICTIONS** | at campaign open | falsifiable expectations — what each phase should change, stated *before* execution so the REPORT can honestly say "confirmed / falsified / surprised" |
| **LOG** | during execution | append-only running record: per-phase entries, deviations from plan, discovered-necessary work, dead ends kept for the record |
| **REPORT** | at campaign close | what the campaign taught: results vs PREDICTIONS, cards/rules that misfired, lessons that feed Discipline revision |

Small campaigns may fold BASELINE and PREDICTIONS into the PLAN's
current-state section and the LOG into commit messages — but a campaign that
skips the REPORT learns nothing durable, and a campaign without a written
PLAN cannot be resumed by anyone but its author's context window.

## 2. The plan shape {#plan}

A campaign plan is written **to be executed cold** — by a fresh session with
no memory of its authoring. The load-bearing sections:

1. **Read-first / boot note** — what to read before this file (the project's
   boot sequence), and the rule that the project's living state supersedes
   the plan where they diverge.
2. **Why this exists** — the reframe: what debt or gap the campaign pays,
   in one screen.
3. **Directives / decisions in force** — the owner's binding choices, so the
   executor never re-litigates them.
4. **Current-state facts, verified** — with file:line pointers, gathered at
   authoring time and marked "do not re-discover". Wrong facts here are the
   most expensive class of plan bug: verify while writing.
5. **Target end-state** — the "what done looks like" tree/diagram.
6. **Design decisions** — each with its rationale and its rejected
   alternative, so a surprise mid-execution can be resolved in the spirit of
   the plan.
7. **Phases** — each with: goal, recipe (commands, paths), **its own
   acceptance gate**, and **its own commit set**. A phase is a safe stop; a
   failed phase rolls back without losing prior ones.
8. **Risks & fallbacks** — named, each with its detection signal and its
   plan-B.
9. **Quick-start** — the copy-paste block the executing session runs first
   (floor verification, baseline capture).
10. **Whole-campaign acceptance** — the end-to-end scenario that defines
    done, ideally frozen as a test.

## 3. Phase-gate discipline {#gates}

- **The floor is green at every phase boundary** — the project's full gate
  panel, not just the parts the phase touched. A phase that leaves the floor
  red is not done.
- **Each phase lands its own topic-grouped commits** (one logical unit per
  commit). The git log is the authoritative per-item record; the LOG
  narrates, the commits *are* the history.
- **Behavior changes carry their differential oracle**
  ([Raid Playbook](03-RAID-PLAYBOOK.md) §1.5): a campaign cannot move
  behavior silently. Where the campaign's point is byte-stability (a
  relocation, a rename), the byte-compare IS the oracle — state it in the
  phase's acceptance.
- **Discovered-necessary work is legal but recorded**: a mid-phase discovery
  either enters the current phase explicitly (LOG entry + the phase's
  commits) or is filed as debt/intent for later — never done silently
  "while I was here".

## 4. Resumability {#resume}

A campaign must survive its executor stopping at any phase boundary
(see [06-WAL-CONVENTION](06-WAL-CONVENTION.md)):

- *With a WAL (recommended):* the campaign updates the WAL's standing line at
  every phase boundary (phase landed, floor state, next phase); a session
  resuming cold reads boot → WAL → the PLAN and continues at the recorded
  phase.
- *Without a WAL:* the PLAN carries a **status line at its top** ("status:
  Phase N landed, floor green, next: Phase N+1") that the executor updates
  as part of each phase's commit set, and the LOG's last entry is the resume
  pointer. The rule is the same either way: **resumption state lives in the
  repository, never in a session.**

## 5. Exit {#exit}

A campaign closes when its whole-campaign acceptance passes on a green
floor. The closing motions: write the REPORT (results vs PREDICTIONS,
lessons, candidate Discipline revisions), file every leftover as debt/intent
with an id (the BROWNFIELD carry-over guarantee: nothing evaporates), update
the resume pointer to "closed", and — where the project mirrors its history —
leave publishing/mirroring as the owner's explicit call, not the campaign's
last step.
