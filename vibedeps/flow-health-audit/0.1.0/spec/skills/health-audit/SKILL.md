---
name: health-audit
description: Run one periodic health audit: walk the category checklist, record findings with severity and disposition in AUDIT.md, carry forward what stays open. Use when the owner triggers an audit or closes a milestone.
---

<status stage="impl" state="done"/>

# Health audit — one run {#root}

##RUNNING-ONE-PERIODIC-HEALTH-AUDIT You are running one periodic health audit: a breadth-first judgment
sweep over what the per-commit gate cannot see. @impl/done

##full-protocol-pointer Full protocol:
@spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root. @impl/done

##PRODUCE-A-DRAFT-AND-DO-NOT-COMMIT-WITHOUT-APPROVAL You produce a **draft** `AUDIT.md`
section; you do not commit without approval. @impl/done

## Procedure {#procedure}

1. ##READ-THE-CHECKLIST-AND-THE-RUN-PROCEDURE-IN-FULL Read `spec/flows/health-audit/audit-checklist.md` and
   `running-an-audit.md` in full. If neither exists, say so, point at
   `HEALTH-AUDIT-PROTOCOL.md`, and stop. @impl/done
2. ##READ-THE-PREVIOUS-SECTION-AND-NOTE-WHAT-CARRIES-FORWARD Read the previous `AUDIT.md` section (if any). Note every finding
   still `open`, or `filed` with work not landed — these carry forward. @impl/done
3. ##IDENTIFY-THE-PROJECTS-GATE-SO-YOU-AUDIT-WHAT-IT-MISSES Identify this project's gate (its test / lint / CI commands), so you
   audit what the gate does *not* cover. @impl/done
4. ##WALK-THE-CHECKLIST-BREADTH-FIRST-AND-RUN-EACH-AID Walk the checklist breadth-first — A test integrity, B rot outside
   the gate, C drift, D debt, plus any project-specific rows. Run each
   mechanical aid (coverage tool, `grep` for skip markers / `TODO` /
   suppressions, dependency audit, CI-config-vs-tree diff). For A3
   (tests that encode the wrong behavior), read assertions against
   intent — there is no mechanical aid. @impl/done
5. ##ASSIGN-THE-FIVE-FIELDS-TO-EACH-FINDING For each finding, assign an ID (`<date>-NN`), a category, a one-line
   locator, a severity (P1/P2/P3), and a proposed disposition. @impl/done
6. ##CARRY-FORWARD-EACH-PRIOR-UNRESOLVED-FINDING-AND-RE-JUDGE-IT Carry forward each prior unresolved finding and re-judge its
   severity. Flag any that has recurred without progress. @impl/done

## Output {#output}

- ##OUTPUT-A-DRAFT-AUDIT-MD-SECTION A draft `AUDIT.md` section: dated heading, the finding table (ID /
  Cat / Finding / Sev / Disp), and notes for `accepted` / `open` rows. @impl/done
- ##OUTPUT-THE-CHEAP-FIXES-VERSUS-THE-FINDINGS-TO-FILE A short list of the cheap fixes you propose to make in-run versus the
  findings you propose to file. @impl/done

## Do not {#do-not}

- ##DO-NOT-COMMIT-OR-FIX-BEFORE-THE-OWNER-APPROVES Do not commit anything, edit `AUDIT.md`, or apply any fix until the
  owner approves the draft. @impl/done
- ##DO-NOT-WRITE-FINDINGS-ONLY-INTO-THE-CHECKPOINT Do not write findings only into the checkpoint file — `AUDIT.md` is
  the durable home; the checkpoint is reconciled against it afterward. @impl/done
- ##DO-NOT-INVENT-FINDINGS-TO-FILL-THE-TABLE Do not invent findings to fill the table. An honest short audit beats
  a padded one. @impl/done
