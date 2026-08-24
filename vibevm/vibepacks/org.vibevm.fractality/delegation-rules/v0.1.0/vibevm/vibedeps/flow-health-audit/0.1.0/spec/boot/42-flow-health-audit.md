# Flow: Health Audit {#root}

This project runs a **periodic health audit**: a recurring,
judgment-heavy sweep over everything the per-commit gate cannot see,
recorded as an append-only trend in `AUDIT.md`.

## The one-line law {#law}

**The gate is the floor; the audit is what the gate cannot see.**
Where the gate answers *"did this commit regress covered code?"*, the
audit answers *"what is wrong, rotting, or drifting that no commit
will ever flag?"*. Neither replaces the other.

## When it fires {#when}

The audit is **owner-triggered**, with a floor of **at least once per
milestone** — run as part of, or right after, a milestone close-out. A
milestone is never declared done on an un-audited base. The owner
re-runs it at will between milestones; no calendar cron is fixed.

## What it produces {#produces}

One dated section in `AUDIT.md` per run, each finding carrying an ID,
its category, a one-line locator, a severity (P1/P2/P3), and a
disposition (fixed/filed/accepted/open). `AUDIT.md` is committed to
git — its history *is* the project's health trend. Unresolved findings
carry forward to the next run and get re-judged.

## How to run one {#run}

Use the **`health-audit`** skill: it reads the category checklist,
walks it against the repository, and drafts the `AUDIT.md` section for
your approval. Full protocol:
[`../flows/health-audit/HEALTH-AUDIT-PROTOCOL.md`](../flows/health-audit/HEALTH-AUDIT-PROTOCOL.md);
the categories to walk:
[`../flows/health-audit/audit-checklist.md`](../flows/health-audit/audit-checklist.md);
the run procedure:
[`../flows/health-audit/running-an-audit.md`](../flows/health-audit/running-an-audit.md).

## Never {#never}

- Never declare a milestone done on an un-audited base — the audit is
  part of the close-out, not an optional extra.
- Never let a finding vanish without a disposition. Every finding is
  fixed, filed, accepted, or open — silence is not an option.
- Never keep findings only in the volatile checkpoint file (the WAL /
  CONTINUE). The durable home is `AUDIT.md`; the checkpoint is
  reconciled *against* it, never a substitute for it.
- Never let the checklist fossilize. A new defect class a run
  discovers becomes a permanent category, so the same gap is never
  re-missed.
- Never mistake a green gate for a healthy project. The gate is blind
  by construction to uncovered code, out-of-gate trees, drift, and
  slow debt — each individually invisible, collectively corrosive.
