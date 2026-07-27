# Running an audit {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This is the operational procedure for one
audit run, start to finish, plus the exact shape of an `AUDIT.md`
section and a worked example on an invented generic project. @impl/done

##companion-document-pointers The
categories you walk in step 2: [`audit-checklist.md`](audit-checklist.md);
the rationale: [`HEALTH-AUDIT-PROTOCOL.md`](HEALTH-AUDIT-PROTOCOL.md). @impl/done

##A-RUN-MUST-FINISH-THE-INVENTORY A run must finish the **inventory**. @impl/done

##A-RUN-NEED-NOT-FINISH-EVERY-FIX It need not finish every *fix* —
fixing is the work the inventory schedules. @impl/done

## The run, step by step {#steps}

1. ##STEP-OPEN-A-DATED-SECTION **Open a dated section** in `AUDIT.md` at the repository root. If
   the file does not exist yet, create it with a one-line header and
   start the first section. The section heading is the run date. @impl/done

2. ##STEP-WALK-THE-CHECKLIST-BREADTH-FIRST **Walk the checklist breadth-first.** Go category by category
   through [`audit-checklist.md`](audit-checklist.md) — A, then B, then
   C, then D, then any project-specific rows. Run each mechanical aid;
   where there is no aid (A3 especially), read with judgment. Breadth
   first: touch every category once before going deep on any one. @impl/done

3. ##STEP-RECORD-EVERY-FINDING-AS-YOU-GO **Record every finding** as you go — ID, category, locator,
   severity, disposition. Do not batch this to the end; a finding
   un-recorded at the moment of discovery is a finding lost. @impl/done

4. ##STEP-CARRY-FORWARD-AND-RE-JUDGE **Carry forward** every still-`open` finding, and every `filed` one
   whose work has not landed, from the previous run's section.
   Re-list each under the new run and **re-judge its severity**. A
   finding that has recurred untouched across runs is itself a signal —
   escalate it or honestly *accept* it; do not let it ride as `open`
   forever. @impl/done

5. ##STEP-FIX-THE-CHEAP-FINDINGS-AND-FILE-THE-REST **Fix the cheap findings in-run.** Small P1/P2 findings get fixed on
   the spot as normal commits; the finding's disposition becomes
   `fixed` and records the commit hash. **File the rest** — a finding
   too large for the run becomes tracked work (a checkpoint "known
   issues" entry, a `TASKS.md` line, a design note) and its disposition
   becomes `filed` with a pointer to where it was filed. @impl/done

6. ##STEP-RECONCILE-THE-CHECKPOINT **Reconcile the checkpoint.** Bring the living checkpoint's "known
   issues" (the WAL / `CONTINUE.md` / status file) into agreement with
   this run's `open` and `filed` findings, so the volatile checkpoint
   and the durable inventory tell the same story. The findings live in
   `AUDIT.md`; the checkpoint only points at the active subset. @impl/done

7. ##STEP-COMMIT-THE-SECTION-AND-EACH-FIX-SEPARATELY **Commit.** Commit `AUDIT.md` as its own change — e.g.
   `docs(audit): <run-date> health audit` — and each in-run fix as its
   own separate commit. The audit section and the fixes are different
   ideas; they are different commits. @impl/done

## The `AUDIT.md` section format {#format}

##AUDIT-MD-IS-APPEND-ONLY `AUDIT.md` is append-only: each run **prepends or appends a new dated
section**, never rewrites an old one. @impl/done

##PAST-SECTIONS-ARE-FROZEN-HISTORY Past sections are frozen history
— the trend lives in their accumulation. @impl/done

##one-section-looks-like-this One section looks like this: @impl/done

```markdown
## 2026-07-07 {#2026-07-07}

_Run by: owner + agent. Milestone M4 close-out. Prior run: 2026-05-30._

| ID          | Cat | Finding                                         | Sev | Disp    |
|-------------|-----|-------------------------------------------------|-----|---------|
| 2026-07-07-01 | A3  | `test_checkout_total` asserts the rounding bug  | P1  | fixed a1b2c3d |
| 2026-07-07-02 | A1  | payment-retry path has no direct test           | P1  | filed TASKS.md |
| 2026-07-07-03 | B1  | `scripts/migrate/` runs in no CI job            | P2  | filed known-issues |
| 2026-07-07-04 | C2  | API.md documents removed `--legacy` flag        | P2  | fixed e4f5a6b |
| 2026-07-07-05 | D3  | file-scope lint-disable in `billing.py`         | P3  | accepted |
| 2026-05-30-07 | D4  | `libfoo` two majors behind, open advisory       | P1  | open (carried) |

**Notes.**
- 05 accepted: the suppression guards a known false positive in the
  linter; revisit when the linter ships the fix (tracking upstream #812).
- 2026-05-30-07 carried forward untouched for the second run — escalate
  to a scheduled upgrade pass before M5, or it becomes a standing risk.
```

##EVERY-FINDING-IS-ONE-TABLE-ROW-CARRYING-THE-FIVE-FIELDS Every finding is one table row carrying the five fields. @impl/done

##THE-DISP-COLUMN-RECORDS-WHERE-THE-FINDING-WENT The `Disp`
column records not just the disposition word but *where it went*: the
commit hash for `fixed`, the destination for `filed`, and `accepted` /
`open` findings get a one-line note below the table. @impl/done

## A worked run — "Acme API" {#worked-example}

##take-an-invented-project Take an invented project: a REST service, `pytest` + `ruff` + a GitHub
Actions gate. @impl/done

##a-milestone-closed-and-the-owner-triggers-an-audit A milestone just closed; the owner triggers an audit.
The run, narrated: @impl/done

- ##EXAMPLE-A-TEST-INTEGRITY **A (test integrity).** The coverage tool shows the payment-retry
  branch at 0% — nobody tests the retry path (A1, P1, too large to fix
  now → *filed*). Reading assertions, `test_checkout_total` asserts a
  total that is a known rounding bug; the test guards the bug (A3, P1,
  a two-line fix → *fixed* in-run). Two `@pytest.mark.skip` tests turn
  out stale and green-if-enabled (A2, P3, *fixed* by deleting them). @impl/done

- ##EXAMPLE-B-ROT-OUTSIDE-THE-GATE **B (rot outside the gate).** `scripts/migrate/` has its own tests
  that no CI job runs; diffing the workflow against the tree finds the
  gap (B1, P2, → *filed*). A new `notifications` package was added last
  milestone and is absent from the coverage matrix (B2, P2, → *fixed*
  by adding it to the workflow). @impl/done

- ##EXAMPLE-C-DRIFT **C (drift).** `API.md` still documents a `--legacy` flag removed two
  milestones back (C1/C2, P2, → *fixed*). The `CONTINUE.md` checkpoint
  claims a phase in progress that shipped (C3, reconciled in step 6). @impl/done

- ##EXAMPLE-D-DEBT **D (debt).** `git blame` dates a `FIXME: temporary` in `billing.py`
  to eleven months ago (D2, P2, → *filed*). A file-scope lint-disable
  is deliberate — it guards a linter false positive — so it is
  *accepted* with the reason and a revisit trigger (D3, P3). The
  dependency audit flags `libfoo` two majors behind with an open
  advisory; it was *open* last run too, so it carries forward and gets
  escalated (D4, P1, *open* → escalated). @impl/done

##EXAMPLE-THE-RUN-CLOSES-WITH-THE-SECTION-AND-SEPARATE-COMMITS The run closes: `AUDIT.md` gets the section above, the three in-run
fixes are three separate commits, the checkpoint's known-issues list
now matches the `filed`/`open` findings, and the section is committed
as `docs(audit): 2026-07-07 health audit`. @impl/done

##EXAMPLE-THE-NEXT-RUN-OPENS-BY-RE-READING-THESE-ROWS The next run will open by
re-reading this section's `open` and `filed` rows. @impl/done

## What the run does and does not owe {#owes}

- ##OWES-A-COMPLETE-INVENTORY **Owes:** a complete inventory. Every category walked, every finding
  recorded and dispositioned, every prior `open`/`filed` finding
  carried forward and re-judged, the checkpoint reconciled. @impl/done
- ##DOES-NOT-OWE-EVERY-FIX **Does not owe:** every fix. A P1 that needs a design pass is *filed*,
  not force-fixed inside the audit; the inventory's job is to schedule
  it, not to complete it. @impl/done

## Summary {#summary}

- ##SUM-THE-SEVEN-STEPS Seven steps: open a dated section → walk the checklist breadth-first
  → record each finding → carry forward prior unresolved findings → fix
  the cheap ones and file the rest → reconcile the checkpoint → commit. @impl/done
- ##SUM-AUDIT-MD-IS-APPEND-ONLY `AUDIT.md` is append-only: new dated section each run, old sections
  frozen. One table row per finding, five fields, disposition records
  where it went. @impl/done
- ##SUM-FIX-CHEAP-FILE-THE-REST-COMMIT-SEPARATELY Fix cheap findings in-run as separate commits; file the rest; the
  audit section is its own commit. @impl/done
- ##SUM-THE-RUN-MUST-FINISH-THE-INVENTORY The run must finish the inventory, not every fix. @impl/done
