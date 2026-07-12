---
name: health-audit
description: Run one periodic health audit: walk the category checklist, record findings with severity and disposition in AUDIT.md, carry forward what stays open. Use when the owner triggers an audit or closes a milestone.
---

# Health audit — one run {#root}

You are running one periodic health audit: a breadth-first judgment
sweep over what the per-commit gate cannot see. Full protocol in
`spec/flows/health-audit/`. You produce a **draft** `AUDIT.md`
section; you do not commit without approval.

## Procedure {#procedure}

1. Read `spec/flows/health-audit/audit-checklist.md` and
   `running-an-audit.md` in full. If neither exists, say so, point at
   `HEALTH-AUDIT-PROTOCOL.md`, and stop.
2. Read the previous `AUDIT.md` section (if any). Note every finding
   still `open`, or `filed` with work not landed — these carry forward.
3. Identify this project's gate (its test / lint / CI commands), so you
   audit what the gate does *not* cover.
4. Walk the checklist breadth-first — A test integrity, B rot outside
   the gate, C drift, D debt, plus any project-specific rows. Run each
   mechanical aid (coverage tool, `grep` for skip markers / `TODO` /
   suppressions, dependency audit, CI-config-vs-tree diff). For A3
   (tests that encode the wrong behavior), read assertions against
   intent — there is no mechanical aid.
5. For each finding, assign an ID (`<date>-NN`), a category, a one-line
   locator, a severity (P1/P2/P3), and a proposed disposition.
6. Carry forward each prior unresolved finding and re-judge its
   severity. Flag any that has recurred without progress.

## Output {#output}

- A draft `AUDIT.md` section: dated heading, the finding table (ID /
  Cat / Finding / Sev / Disp), and notes for `accepted` / `open` rows.
- A short list of the cheap fixes you propose to make in-run versus the
  findings you propose to file.

## Do not {#do-not}

- Do not commit anything, edit `AUDIT.md`, or apply any fix until the
  owner approves the draft.
- Do not write findings only into the checkpoint file — `AUDIT.md` is
  the durable home; the checkpoint is reconciled against it afterward.
- Do not invent findings to fill the table. An honest short audit beats
  a padded one.
