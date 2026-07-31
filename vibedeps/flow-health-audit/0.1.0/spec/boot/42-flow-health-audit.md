# Flow: Health Audit {#root}

<status stage="impl" state="done"/>

##PROJECT-RUNS-A-PERIODIC-HEALTH-AUDIT This project runs a **periodic health audit**: a recurring,
judgment-heavy sweep over everything the per-commit gate cannot see,
recorded as an append-only trend in `AUDIT.md`. @impl/done

## The one-line law {#law}

##THE-GATE-IS-THE-FLOOR-THE-AUDIT-IS-WHAT-IT-CANNOT-SEE **The gate is the floor; the audit is what the gate cannot see.** @impl/done

##THE-GATE-AND-THE-AUDIT-ANSWER-DIFFERENT-QUESTIONS Where the gate answers *"did this commit regress covered code?"*, the
audit answers *"what is wrong, rotting, or drifting that no commit
will ever flag?"*. @impl/done

##neither-replaces-the-other Neither replaces the other. @impl/done

## When it fires {#when}

##AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR The audit is **owner-triggered**, with a floor of **at least once per
milestone** — run as part of, or right after, a milestone close-out. @impl/done

##A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE A
milestone is never declared done on an un-audited base. @impl/done

##OWNER-RE-RUNS-AT-WILL-AND-NO-CALENDAR-CRON-IS-FIXED The owner
re-runs it at will between milestones; no calendar cron is fixed. @impl/done

## What it produces {#produces}

##ONE-DATED-SECTION-PER-RUN-EACH-FINDING-CARRYING-FIVE-FIELDS One dated section in `AUDIT.md` per run, each finding carrying an ID,
its category, a one-line locator, a severity (P1/P2/P3), and a
disposition (fixed/filed/accepted/open). @impl/done

##AUDIT-MD-IS-COMMITTED-AND-ITS-HISTORY-IS-THE-HEALTH-TREND `AUDIT.md` is committed to
git — its history *is* the project's health trend. @impl/done

##UNRESOLVED-FINDINGS-CARRY-FORWARD-AND-GET-RE-JUDGED Unresolved findings
carry forward to the next run and get re-judged. @impl/done

## How to run one {#run}

##USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE Use the **`health-audit`** skill: it reads the category checklist,
walks it against the repository, and drafts the `AUDIT.md` section for
your approval. @impl/done

##flow-document-pointers Full protocol:
@spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root;
the categories to walk:
@spec://org.vibevm.world/health-audit/flows/health-audit/audit-checklist#root;
the run procedure:
@spec://org.vibevm.world/health-audit/flows/health-audit/running-an-audit#root. @impl/done

## Never {#never}

- ##NEVER-DECLARE-A-MILESTONE-DONE-ON-AN-UN-AUDITED-BASE Never declare a milestone done on an un-audited base — the audit is
  part of the close-out, not an optional extra. @impl/done
- ##NEVER-LET-A-FINDING-VANISH-WITHOUT-A-DISPOSITION Never let a finding vanish without a disposition. Every finding is
  fixed, filed, accepted, or open — silence is not an option. @impl/done
- ##NEVER-KEEP-FINDINGS-ONLY-IN-THE-VOLATILE-CHECKPOINT Never keep findings only in the volatile checkpoint file (the WAL /
  CONTINUE). The durable home is `AUDIT.md`; the checkpoint is
  reconciled *against* it, never a substitute for it. @impl/done
- ##NEVER-LET-THE-CHECKLIST-FOSSILIZE Never let the checklist fossilize. A new defect class a run
  discovers becomes a permanent category, so the same gap is never
  re-missed. @impl/done
- ##NEVER-MISTAKE-A-GREEN-GATE-FOR-A-HEALTHY-PROJECT Never mistake a green gate for a healthy project. The gate is blind
  by construction to uncovered code, out-of-gate trees, drift, and
  slow debt — each individually invisible, collectively corrosive. @impl/done
