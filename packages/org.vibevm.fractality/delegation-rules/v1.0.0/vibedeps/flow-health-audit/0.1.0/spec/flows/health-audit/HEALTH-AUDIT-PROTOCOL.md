# Health Audit Protocol {#root}

**Scope of this document.** This file defines *what* the periodic
health audit is and is not, *why* a green per-commit gate is not
enough, *where* its findings live and why that home is not the
volatile checkpoint, *how* findings are dispositioned and carried
forward, and the law that keeps the checklist alive. The categories
themselves: [`audit-checklist.md`](audit-checklist.md); the run
procedure: [`running-an-audit.md`](running-an-audit.md).

## The gate versus the audit {#gate-vs-audit}

Every serious project has a **gate**: some automated per-commit check
— a test suite, a linter, a CI pipeline, or all three — that must be
green before a commit lands. The gate is a fast, mechanical
**regression detector**: it proves, on every commit, that *covered*
code still behaves. It is indispensable, and it is blind by
construction to four things.

The audit is a different kind of check — a deliberate, periodic,
breadth-first sweep, run with human or agent **judgment**, that
inventories what the gate cannot see and records the result durably.
**Where the gate answers *"did this commit regress covered code?"*,
the audit answers *"what is wrong, rotting, or drifting that no commit
will ever flag?"*.** The two are complements; adding more gate never
substitutes for the audit.

## The four blind spots {#blind-spots}

| # | Blind spot | Why the gate misses it |
|---|-----------|------------------------|
| 1 | **Uncovered code** | A path no test exercises can break with the gate fully green. |
| 2 | **Code outside the gate** | Anything the test command does not reach — a separate workspace, an unparsed fixture, a manual-test recipe — rots silently. |
| 3 | **Drift** | Docs, spec, the checkpoint, and external state fall out of step with the code without any test failing. |
| 4 | **Slow debt** | Escape hatches, aging `TODO`s, deferred items, quarantined tests — each individually invisible, collectively corrosive. |

None of these is hypothetical. The canonical failure is a milestone
that shipped green — every commit passing, hundreds of tests passing —
while the project's own initializer scaffolded *broken* projects and a
test asserted the broken output *as correct*. The gate cannot catch a
test that encodes the wrong answer; only a reader judging the assertion
against the intent can. Not more gate — a different activity.

## What the audit inventories {#categories}

An audit run walks a category checklist breadth-first — one category
group per blind spot: **A** test integrity (coverage gaps, quarantined
tests, tests that encode the wrong behavior), **B** rot outside the
gate, **C** drift (docs, specs, checkpoint, external state), **D** debt
(deferred items, aging markers, escape hatches, stale dependencies).
Every sub-item, its mechanical aid, and what "bad" looks like lives in
[`audit-checklist.md`](audit-checklist.md); the list is not fixed
(see [§living](#living)).

## `AUDIT.md` is the durable home {#audit-md}

Each run records its findings in **`AUDIT.md`** at the repository root:
a curated, **append-only chronicle**, one dated section per run — the
shape of a `CHANGELOG.md`. **`AUDIT.md` is committed to git. Its
history *is* the project's health trend.** A reader can diff two runs
and see whether open P1s are climbing or falling, whether a finding has
recurred untouched for three runs, whether the gate is absorbing
categories over time.

The durable home is deliberately **not** the checkpoint file (the WAL,
`CONTINUE.md`, or whatever living state file the project keeps). That
file is *volatile* — rewritten every session to reflect the current
state, not the history — so a finding tracked only there is erased at
the next refresh and its trend is lost. The checkpoint's "known issues"
section is *reconciled against* `AUDIT.md` at the end of a run, but the
append-only inventory is the source of truth; the checkpoint merely
points at the active subset.

## The finding record {#finding}

Every finding carries five fields:

| Field | Content |
|-------|---------|
| **ID** | `<run-date>-NN` — unique within the run, stable across carry-forward. |
| **Category** | The checklist row it came from (`A1` … `D4`, or a project-specific code). |
| **Locator** | A one-line description with enough of a file/module pointer to act on. |
| **Severity** | `P1` / `P2` / `P3` (below). |
| **Disposition** | `fixed` / `filed` / `accepted` / `open` (next section). |

Severity is the cost of leaving it:

- **P1 — blocker.** A correctness gap, or a defect that can ship wrong
  behavior. Must be resolved before the next milestone is declared
  shipped.
- **P2 — debt.** Real and scheduled — fixed in a dedicated pass, or
  opportunistically when the area is next touched.
- **P3 — note.** Low cost of leaving; recorded so the next run
  re-judges it rather than re-discovering it.

## Disposition and carry-forward {#disposition}

Every finding is dispositioned before the run closes. Nothing is left
silent:

| Disposition | Meaning |
|-------------|---------|
| **fixed** | Resolved inside the run. Small findings are fixed on the spot; the fix is a normal commit and the finding records its hash. |
| **filed** | Too large to fix in the run. It becomes tracked work — a checkpoint "known issues" entry, a `TASKS.md` line, or a design note — and the finding records where it was filed. |
| **accepted** | A deliberate decision *not* to act, recorded with its reason. Re-judged next run. This is a decision record: it carries a why and a revisit trigger. |
| **open** | Not yet dispositioned. |

An `open` finding — or a `filed` one whose work has not landed —
**carries forward**: the next run re-lists it and re-judges its
severity. This is the whole point of a durable inventory: **a finding
that recurs across runs without progress is itself a signal.** A P2
that has ridden three consecutive audits untouched is really a P1
nobody will schedule, or should be honestly *accepted* rather than
perpetually *open*.

## The checklist is living {#living}

The checklist is not frozen. Two forces reshape it every run:

- **A new defect class becomes a permanent category.** When a run
  discovers a kind of rot the checklist did not name, that kind is
  added as a standing row — so the same gap is never re-missed. The
  broken-initializer defect above is exactly what turns "the untested
  default path" into a permanent coverage line.
- **A mechanisable category migrates into the gate.** When a category
  can be checked by a script, it *should* — over time it moves out of
  the manual audit into the linter, the test suite, or CI, becoming an
  automatic per-commit guard. The audit is the judgment-heavy
  *superset*; the gate is the automated *subset* it keeps feeding. The
  long-run goal: each run finds *fewer* things the gate could have
  caught and *more* that genuinely need judgment.

One corollary deserves its own line. **A project that has adopted a
rule framework should add a category measuring how deep the adoption
actually goes, not just that it exists.** "Adopted" is true at the
surface long before it is true in depth; a row that audits *depth of
adoption* — how many modules actually carry the convention, not merely
that it is documented — is the kind of category a real project grows
into. The checklist is yours to extend; these are starting categories.

## Cadence {#cadence}

The audit is **owner-triggered**, with a recommended floor of **once
per milestone** — run as part of, or immediately after, a milestone
close-out, so **a milestone is never declared done on an un-audited
base**. The owner re-runs it at will between milestones; no calendar
cron is fixed. A run need not finish every *fix* — it must finish the
*inventory*. Fixing is the work the inventory schedules.

## Why not the simpler options {#why-not}

Three simpler shapes were considered and rejected:

- **A one-time hardening pass instead of a recurring process** —
  rejected: a one-shot pass decays the day after it lands and rot
  resumes. The value is the *recurrence* and the *trend record*, not
  the single cleanup.
- **Rely on the gate alone** — rejected: the gate is a regression
  detector, structurally blind to uncovered code, out-of-gate trees,
  and drift. More gate is good but never sufficient; the
  broken-initializer defect passed the gate on every commit.
- **Track findings only in the checkpoint file** — rejected: the
  checkpoint is volatile, rewritten each session. A durable,
  append-only history is what lets the project see whether it trends
  healthier or worse. Hence a separate `AUDIT.md`.

Full automation is *deferred, not rejected*: the audit's value is
breadth *plus judgment* — "this test encodes a bug" is not mechanically
detectable — so it grows category by category (§living), never
replacing the process.

## Re-derive for your project {#re-derive}

Do not copy the category letters verbatim — copy the *task*, and let
the agent re-derive the checklist this project actually needs:

```
Read spec/flows/health-audit/ in full, then adapt the audit to this
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

- The gate is a per-commit regression detector; the audit is a
  periodic judgment sweep over what the gate cannot see. Complements,
  not substitutes.
- Four blind spots: uncovered code, out-of-gate trees, drift, slow
  debt — each individually invisible, collectively corrosive.
- Findings live in `AUDIT.md`: append-only, dated, committed to git —
  its history is the project's health trend, not the volatile
  checkpoint. Five fields per finding; four dispositions; unresolved
  findings carry forward, and one that recurs without progress is
  itself a signal.
- The checklist is living: new defect classes join it, mechanisable
  ones migrate into the gate.
- Owner-triggered, floor once per milestone. A milestone is never
  declared done on an un-audited base.
