# Health Audit Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what* the periodic
health audit is and is not, *why* a green per-commit gate is not
enough, *where* its findings live and why that home is not the
volatile checkpoint, *how* findings are dispositioned and carried
forward, and the law that keeps the checklist alive. @status:impl/done

@fact:companion-document-pointers The categories
themselves: [`audit-checklist.md`](audit-checklist.md); the run
procedure: [`running-an-audit.md`](running-an-audit.md). @status:impl/done

## The gate versus the audit {#gate-vs-audit}

@fact:EVERY-SERIOUS-PROJECT-HAS-A-GATE Every serious project has a **gate**: some automated per-commit check
— a test suite, a linter, a CI pipeline, or all three — that must be
green before a commit lands. @status:spec/done

@fact:THE-GATE-IS-A-MECHANICAL-REGRESSION-DETECTOR The gate is a fast, mechanical
**regression detector**: it proves, on every commit, that *covered*
code still behaves. @status:spec/done

@fact:THE-GATE-IS-INDISPENSABLE-AND-BLIND-TO-FOUR-THINGS It is indispensable, and it is blind by
construction to four things. @status:spec/done

@fact:THE-AUDIT-IS-A-DIFFERENT-KIND-OF-CHECK The audit is a different kind of check — a deliberate, periodic,
breadth-first sweep, run with human or agent **judgment**, that
inventories what the gate cannot see and records the result durably. @status:impl/done

@fact:THE-GATE-AND-THE-AUDIT-ANSWER-DIFFERENT-QUESTIONS **Where the gate answers *"did this commit regress covered code?"*,
the audit answers *"what is wrong, rotting, or drifting that no commit
will ever flag?"*.** @status:impl/done

@fact:THE-TWO-ARE-COMPLEMENTS-AND-MORE-GATE-NEVER-SUBSTITUTES The two are complements; adding more gate never
substitutes for the audit. @status:impl/done

## The four blind spots {#blind-spots}

| # | Blind spot | Why the gate misses it |
|---|-----------|------------------------|
| @fact:ROW-BLIND-UNCOVERED-CODE 1 @status:spec/done | **Uncovered code** @status:spec/done | A path no test exercises can break with the gate fully green. @status:spec/done |
| @fact:ROW-BLIND-CODE-OUTSIDE-THE-GATE 2 @status:spec/done | **Code outside the gate** @status:spec/done | Anything the test command does not reach — a separate workspace, an unparsed fixture, a manual-test recipe — rots silently. @status:spec/done |
| @fact:ROW-BLIND-DRIFT 3 @status:spec/done | **Drift** @status:spec/done | Docs, spec, the checkpoint, and external state fall out of step with the code without any test failing. @status:spec/done |
| @fact:ROW-BLIND-SLOW-DEBT 4 @status:spec/done | **Slow debt** @status:spec/done | Escape hatches, aging `TODO`s, deferred items, quarantined tests — each individually invisible, collectively corrosive. @status:spec/done |

@fact:none-of-these-is-hypothetical None of these is hypothetical. @status:spec/done

@fact:the-canonical-failure-is-the-broken-initializer The canonical failure is a milestone
that shipped green — every commit passing, hundreds of tests passing —
while the project's own initializer scaffolded *broken* projects and a
test asserted the broken output *as correct*. @status:spec/done

@fact:ONLY-A-READER-CATCHES-A-TEST-THAT-ENCODES-THE-WRONG-ANSWER The gate cannot catch a
test that encodes the wrong answer; a reader judging the assertion
against the intent can — and so can a live end-to-end run of the real
thing, which is what caught the initializer defect above. @status:spec/done

@fact:not-more-gate-but-a-different-activity Not more gate — a different activity. @status:spec/done

## What the audit inventories {#categories}

@fact:AN-AUDIT-RUN-WALKS-ONE-CATEGORY-GROUP-PER-BLIND-SPOT An audit run walks a category checklist breadth-first — one category
group per blind spot: @status:impl/done

- @fact:GROUP-A-TEST-INTEGRITY **A** test integrity (coverage gaps, quarantined
  tests, tests that encode the wrong behavior), @status:impl/done
- @fact:GROUP-B-ROT-OUTSIDE-THE-GATE **B** rot outside the
  gate, @status:impl/done
- @fact:GROUP-C-DRIFT **C** drift (docs, specs, checkpoint, external state), @status:impl/done
- @fact:GROUP-D-DEBT **D** debt
  (deferred items, aging markers, escape hatches, stale dependencies). @status:impl/done

@fact:checklist-pointer-and-the-list-is-not-fixed Every sub-item, its mechanical aid, and what "bad" looks like lives in
[`audit-checklist.md`](audit-checklist.md); the list is not fixed
(see [§living](#living)). @status:impl/done

## `AUDIT.md` is the durable home {#audit-md}

@fact:EACH-RUN-RECORDS-ITS-FINDINGS-IN-AUDIT-MD Each run records its findings in **`AUDIT.md`** at the repository root:
a curated, **append-only chronicle**, one dated section per run — the
shape of a `CHANGELOG.md`. @status:impl/done

@fact:AUDIT-MD-IS-COMMITTED-TO-GIT **`AUDIT.md` is committed to git.** @status:impl/done

@fact:ITS-HISTORY-IS-THE-PROJECTS-HEALTH-TREND **Its
history *is* the project's health trend.** @status:impl/done

@fact:A-READER-CAN-DIFF-TWO-RUNS-AND-SEE-THE-TREND A reader can diff two runs
and see whether open P1s are climbing or falling, whether a finding has
recurred untouched for three runs, whether the gate is absorbing
categories over time. @status:impl/done

@fact:THE-DURABLE-HOME-IS-NOT-THE-CHECKPOINT-FILE The durable home is deliberately **not** the checkpoint file (the WAL,
`CONTINUE.md`, or whatever living state file the project keeps). @status:impl/done

@fact:a-volatile-file-erases-a-finding-tracked-only-there That
file is *volatile* — rewritten every session to reflect the current
state, not the history — so a finding tracked only there is erased at
the next refresh and its trend is lost. @status:spec/done

@fact:THE-CHECKPOINT-IS-RECONCILED-BUT-THE-INVENTORY-IS-THE-SOURCE The checkpoint's "known issues"
section is *reconciled against* `AUDIT.md` at the end of a run, but the
append-only inventory is the source of truth; the checkpoint merely
points at the active subset. @status:impl/done

## The finding record {#finding}

@fact:EVERY-FINDING-CARRIES-FIVE-FIELDS Every finding carries five fields: @status:impl/done

| Field | Content |
|-------|---------|
| @fact:ROW-FIELD-ID **ID** @status:impl/done | `<run-date>-NN` — unique within the run, stable across carry-forward. @status:impl/done |
| @fact:ROW-FIELD-CATEGORY **Category** @status:impl/done | The checklist row it came from (`A1` … `D4`, or a project-specific code). @status:impl/done |
| @fact:ROW-FIELD-LOCATOR **Locator** @status:impl/done | A one-line description with enough of a file/module pointer to act on. @status:impl/done |
| @fact:ROW-FIELD-SEVERITY **Severity** @status:impl/done | `P1` / `P2` / `P3` (below). @status:impl/done |
| @fact:ROW-FIELD-DISPOSITION **Disposition** @status:impl/done | `fixed` / `filed` / `accepted` / `open` (next section). @status:impl/done |

@fact:SEVERITY-IS-THE-COST-OF-LEAVING-IT Severity is the cost of leaving it: @status:impl/done

- @fact:SEVERITY-P1-BLOCKER **P1 — blocker.** A correctness gap, or a defect that can ship wrong
  behavior. Must be resolved before the next milestone is declared
  shipped. @status:impl/done
- @fact:SEVERITY-P2-DEBT **P2 — debt.** Real and scheduled — fixed in a dedicated pass, or
  opportunistically when the area is next touched. @status:impl/done
- @fact:SEVERITY-P3-NOTE **P3 — note.** Low cost of leaving; recorded so the next run
  re-judges it rather than re-discovering it. @status:impl/done

## Disposition and carry-forward {#disposition}

@fact:EVERY-FINDING-IS-DISPOSITIONED-BEFORE-THE-RUN-CLOSES Every finding is dispositioned before the run closes. @status:impl/done

@fact:NOTHING-IS-LEFT-SILENT Nothing is left
silent: @status:impl/done

| Disposition | Meaning |
|-------------|---------|
| @fact:ROW-DISP-FIXED **fixed** @status:impl/done | Resolved inside the run. Small findings are fixed on the spot; the fix is a normal commit and the finding records its hash. @status:impl/done |
| @fact:ROW-DISP-FILED **filed** @status:impl/done | Too large to fix in the run. It becomes tracked work — a checkpoint "known issues" entry, a `TASKS.md` line, or a design note — and the finding records where it was filed. @status:impl/done |
| @fact:ROW-DISP-ACCEPTED **accepted** @status:impl/done | A deliberate decision *not* to act, recorded with its reason. Re-judged next run. This is a decision record: it carries a why and a revisit trigger. @status:impl/done |
| @fact:ROW-DISP-OPEN **open** @status:impl/done | Not yet dispositioned. @status:impl/done |

@fact:AN-OPEN-OR-UNLANDED-FILED-FINDING-CARRIES-FORWARD An `open` finding — or a `filed` one whose work has not landed —
**carries forward**: the next run re-lists it and re-judges its
severity. @status:impl/done

@fact:A-FINDING-THAT-RECURS-WITHOUT-PROGRESS-IS-ITSELF-A-SIGNAL This is the whole point of a durable inventory: **a finding
that recurs across runs without progress is itself a signal.** @status:impl/done

@fact:A-P2-RIDING-THREE-AUDITS-IS-REALLY-A-P1-OR-AN-ACCEPTANCE A P2
that has ridden three consecutive audits untouched is really a P1
nobody will schedule, or should be honestly *accepted* rather than
perpetually *open*. @status:impl/done

## The checklist is living {#living}

@fact:THE-CHECKLIST-IS-NOT-FROZEN The checklist is not frozen. @status:impl/done

@fact:TWO-FORCES-RESHAPE-THE-CHECKLIST-EVERY-RUN Two forces reshape it over time, each when its own
condition holds: @status:impl/done

- @fact:FORCE-A-NEW-DEFECT-CLASS-BECOMES-A-PERMANENT-CATEGORY **A new defect class becomes a permanent category.** When a run
  discovers a kind of rot the checklist did not name, that kind is
  added as a standing row — so the same gap is never re-missed. The
  broken-initializer defect above is exactly what turns "the untested
  default path" into a permanent coverage line. @status:impl/done
- @fact:FORCE-A-MECHANISABLE-CATEGORY-MIGRATES-INTO-THE-GATE **A mechanisable category migrates into the gate.** When a category
  can be checked by a script, it *should* — over time it moves out of
  the manual audit into the linter, the test suite, or CI, becoming an
  automatic per-commit guard. The audit is the judgment-heavy
  *superset*; the gate is the automated *subset* it keeps feeding. The
  long-run goal: each run finds *fewer* things the gate could have
  caught and *more* that genuinely need judgment. @status:impl/done

@fact:one-corollary-deserves-its-own-line One corollary deserves its own line. @status:impl/done

@fact:ADD-A-CATEGORY-MEASURING-DEPTH-OF-ADOPTION **A project that has adopted a
rule framework should add a category measuring how deep the adoption
actually goes, not just that it exists.** @status:impl/done

@fact:adopted-is-true-at-the-surface-first "Adopted" is true at the
surface long before it is true in depth; a row that audits *depth of
adoption* — how many modules actually carry the convention, not merely
that it is documented — is the kind of category a real project grows
into. @status:spec/done

@fact:THE-CHECKLIST-IS-YOURS-TO-EXTEND The checklist is yours to extend; these are starting categories. @status:impl/done

## Cadence {#cadence}

@fact:AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR The audit is **owner-triggered**, with a recommended floor of **once
per milestone** — run as part of, or immediately after, a milestone
close-out, so **a milestone is never declared done on an un-audited
base**. @status:impl/done

@fact:OWNER-RE-RUNS-AT-WILL-AND-NO-CALENDAR-CRON-IS-FIXED The owner re-runs it at will between milestones; no calendar
cron is fixed. @status:impl/done

@fact:A-RUN-MUST-FINISH-THE-INVENTORY-NOT-EVERY-FIX A run need not finish every *fix* — it must finish the
*inventory*. @status:impl/done

@fact:fixing-is-the-work-the-inventory-schedules Fixing is the work the inventory schedules. @status:impl/done

## Why not the simpler options {#why-not}

@fact:three-rejected-shapes-lead Three simpler shapes were considered and rejected: @status:spec/done

- @fact:REJECTED-A-ONE-TIME-HARDENING-PASS **A one-time hardening pass instead of a recurring process** —
  rejected: a one-shot pass decays the day after it lands and rot
  resumes. The value is the *recurrence* and the *trend record*, not
  the single cleanup. @status:spec/done
- @fact:REJECTED-RELY-ON-THE-GATE-ALONE **Rely on the gate alone** — rejected: the gate is a regression
  detector, structurally blind to uncovered code, out-of-gate trees,
  and drift. More gate is good but never sufficient; the
  broken-initializer defect passed the gate on every commit. @status:spec/done
- @fact:REJECTED-TRACK-FINDINGS-ONLY-IN-THE-CHECKPOINT **Track findings only in the checkpoint file** — rejected: the
  checkpoint is volatile, rewritten each session. A durable,
  append-only history is what lets the project see whether it trends
  healthier or worse. Hence a separate `AUDIT.md`. @status:spec/done

@fact:FULL-AUTOMATION-IS-DEFERRED-NOT-REJECTED Full automation is *deferred, not rejected*: the audit's value is
breadth *plus judgment* — "this test encodes a bug" is not mechanically
detectable — so it grows category by category (§living), never
replacing the process. @status:spec/done

## Re-derive for your project {#re-derive}

@fact:COPY-THE-TASK-NOT-THE-CATEGORY-LETTERS Do not copy the category letters verbatim — copy the *task*, and let
the agent re-derive the checklist this project actually needs: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-health-audit/<version>/spec/flows/health-audit/`, check `vibe.lock`) in full, then adapt the audit to this
project:
1. Name this project's per-commit gate exactly — the commands that
   must be green before a commit lands (tests, linter, CI jobs).
2. For each of the four blind spots (uncovered code, out-of-gate
   trees, drift, slow debt), list what specifically escapes THIS
   gate: which trees the test command misses, which docs/specs drift,
   which markers and escape hatches accumulate here.
3. Turn each into a concrete checklist row with a mechanical aid — the
   grep pattern, the coverage tool, the CI-config diff that surfaces
   it in this repo.
4. Add one row measuring depth-of-adoption for any convention this
   project claims to follow (not just that it is documented).
5. Draft the AUDIT.md skeleton (one dated section, the five finding
   fields) and show it to me. Create nothing until I approve.
```

## Summary {#summary}

- @fact:SUM-GATE-AND-AUDIT-ARE-COMPLEMENTS The gate is a per-commit regression detector; the audit is a
  periodic judgment sweep over what the gate cannot see. Complements,
  not substitutes. @status:impl/done
- @fact:SUM-FOUR-BLIND-SPOTS Four blind spots: uncovered code, out-of-gate trees, drift, slow
  debt — each individually invisible, collectively corrosive. @status:spec/done
- @fact:SUM-FINDINGS-LIVE-IN-AUDIT-MD Findings live in `AUDIT.md`: append-only, dated, committed to git —
  its history is the project's health trend, not the volatile
  checkpoint. Five fields per finding; four dispositions; unresolved
  findings carry forward, and one that recurs without progress is
  itself a signal. @status:impl/done
- @fact:SUM-THE-CHECKLIST-IS-LIVING The checklist is living: new defect classes join it, mechanisable
  ones migrate into the gate. @status:impl/done
- @fact:SUM-OWNER-TRIGGERED-FLOOR-ONCE-PER-MILESTONE Owner-triggered, floor once per milestone. A milestone is never
  declared done on an un-audited base. @status:impl/done
