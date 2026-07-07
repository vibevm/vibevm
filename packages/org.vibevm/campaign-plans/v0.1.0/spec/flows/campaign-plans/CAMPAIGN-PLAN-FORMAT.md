# The Campaign Plan Format {#root}

**Scope of this document.** This file defines *what* a campaign is,
*which* artifact roles its paper trail carries, and the *canonical
section skeleton* of a campaign plan — the one document that lets a
large change be planned once and executed cold. Phase mechanics:
[`phase-gates.md`](phase-gates.md); the record half:
[`execution-ledger.md`](execution-ledger.md).

## What a campaign is {#what}

A campaign is a multi-commit change too big for one session: a
package-family rename, a debt drain across the whole codebase, a new
subsystem landed in six waves. It is executed as **gated phases** —
each ends with the project's full gate panel green — and it is
planned to run **cold**: by a fresh session with no memory of the
planning conversation, by a different person, or by the author after
total context loss. The test of a good plan is not "could I execute
this tomorrow?" but "could a stranger execute this today?". The
format's cost is real: pay it only when the work spans sessions or
more than a handful of commits.

## The five artifact roles {#artifacts}

A campaign's paper trail carries five roles:

| Role | Written | Purpose |
|---|---|---|
| **PLAN** | before work starts | the cold-executable recipe |
| **BASELINE** | at campaign open | the frozen starting facts: gate-panel state, inventory counts, the numbers phases are measured against |
| **PREDICTIONS** | at campaign open | falsifiable expectations, stated *before* execution so the report can honestly say confirmed / falsified / surprised |
| **LOG** | during execution | the running record: per-phase commit maps, deviations, discovered work |
| **REPORT** | at campaign close | results vs predictions; what the campaign taught |

**The one-file dialect (default).** One plan document carries all
five roles: the baseline in the target-arithmetic and current-state
sections, the predictions in their own section, the log in the
execution ledger, the report in the execution-record block prepended
at close — one file, one resume pointer. Very large campaigns may
split the roles into files; the roles, not the count, are the contract.

Two laws govern the set: **a campaign that skips the REPORT learns
nothing durable, and a campaign without a written PLAN cannot be
resumed by anyone but its author's context window.**

## The section skeleton {#skeleton}

The canonical plan, in order. Sections 1 and 3–13 are written before
execution; 2, 14, and 15 are filled by the executing session.

### 1 — Title and status line {#s1-status}

The title names the campaign and its one-line point; under it, an
italic status block: the lifecycle state (`PLANNED` → `EXECUTING` →
`EXECUTED <date>`), the tree the plan was written against, and the
cold-start flag. Owner review may annotate the state
(`ACCEPTED with owner amendments, <date>`) before execution.

```
# CACHE-DRAIN-PLAN v0.1 — retire the legacy cache, consumer by consumer
_Status: PLANNED · written against tree `57fa42e` · cold-executable:
any phase is a safe stop; the floor is green at every boundary._
```

### 2 — Execution record (prepended at close) {#s2-execution-record}

Empty at authoring. At close, the executing session prepends a block
into the status area: commit range, per-phase deltas, which
predictions held ([`execution-ledger.md` §execution-record](execution-ledger.md#execution-record)).

### 3 — The mandate {#s3-mandate}

The owner's commissioning words, **quoted verbatim and dated** —
never paraphrased into blandness. Scope questions resolve against
this text; the executor never re-litigates it.

```
Mandate (owner, 2026-07-07): "take everything listed in the previous
campaign's deferral ledger and plan its implementation; do not wait
for the full pilot — build a small demo project instead."
```

### 4 — Target arithmetic {#s4-arithmetic}

Baseline and exit state as **exact counts** that reconcile: every
baseline unit ends in the exit state or in a phase that removes it.
Vague targets make the report unwritable.

```
Baseline at plan time: 130 findings = 68 message-format + 28
file-budget + 24 banned-call + 8 owner-gated + 2 one-shots.
Exit state: 10 = 8 owner-gated (out of scope) + 2 parked (named).
Everything else reaches zero.
```

### 5 — Current-state facts (verified; do not re-discover) {#s5-facts}

Facts gathered at authoring, with file-and-line pointers, marked so
the executor trusts them instead of re-investigating. **Wrong facts
here are the most expensive class of plan bug: verify while
writing.** A real campaign recorded three files as stale at
566/556/554 lines; the true sizes were 609/612/608 — the author
counted non-blank lines where the gate counts physical ones. A Phase
0 probe caught what would have pruned live debt as stale.

### 6 — Decisions D1–DN {#s6-decisions}

Every design decision, numbered. Each weighs its options, marks the
chosen one, and gives every rejection a reason — so a mid-execution
surprise is resolved in the spirit of the plan, and nobody re-opens
a settled question. Rejections are as load-bearing as the choice.

```
### D4 — how the shared engine reaches both consumers
- (α) rewrite cross-package paths at install time — real product
  surface; rejected here, named as future work in the follow-up spec.
- (β) align the two layouts — breaks a published invariant; rejected.
- (γ) vendor-sync (CHOSEN): one authored home, byte-identical synced
  copies, a `--check` gate makes drift mechanically impossible.
```

### 7 — Predictions {#s7-predictions}

Numbered, falsifiable, stated before execution, checked one by one in
the report — "P3 — fewer than 10 test expectations break across all
68 message edits: most tests match error kinds, not strings." A
prediction that cannot fail is a hope, not a prediction.

### 8 — Phases {#s8-phases}

Phase 0 is always spikes and probes and produces no commits; every
later phase carries numbered steps, its own exit criterion, its own
prediction, and its planned commit set with subjects spelled in
advance. Full anatomy and gate rules: [`phase-gates.md`](phase-gates.md).

### 9 — Risks and fallbacks {#s9-risks}

Named risks, each with its detection signal and its plan B — "flaky
network: re-probe at each network-facing step; worst case those
steps land red-pending-network, recorded, everything else lands." A
risk without a fallback is a wish that nothing goes wrong.

### 10 — Non-goals {#s10-non-goals}

"What this plan deliberately does NOT do" — **named, so they stay
visible.** Each non-goal carries a reason and a disposition: deferred
to a named follow-up, held by the owner, or rejected outright —
"does NOT extend the gate to the two remaining modules: that is the
NEXT campaign's opening move, after this queue closes."

### 11 — Quick-start for the executing session {#s11-quick-start}

The literal shell block a cold session runs first: confirm the tree,
verify the green floor, capture the baseline numbers.

```sh
git log --oneline -3        # tree must match the status line
<gate-panel command>        # full panel green before Phase 0 opens
<baseline count command>    # must print the §4 baseline figure
```

### 12 — Whole-campaign acceptance {#s12-acceptance}

A runnable script asserting the end state — the campaign's definition
of done, executed on a green floor at close, cited by the report.

```sh
<gate-panel command>; echo "EXIT=$?"    # exit 0
test ! -d src/legacy_cache              # the drained module is gone
<banned-pattern search> | wc -l         # 0 references remain
```

### 13 — Review points {#s13-review-points}

Decisions only the owner can make, escalated as numbered points:
`OPEN` with options and the executor's recommendation, later
annotated `RESOLVED` with the ruling verbatim
([`phase-gates.md` §review-points](phase-gates.md#review-points)).

### 14 — Execution ledger {#s14-ledger}

Filled by the executing session: per-phase commit maps binding hashes
to the planned subjects, with what each commit confirmed or falsified
([`execution-ledger.md`](execution-ledger.md)).

### 15 — Deferrals ledger {#s15-deferrals}

Everything the campaign chose not to do, named, one line each, with
an owner and a disposition. Nothing evaporates: leftover work is in a
commit or in this ledger
([`execution-ledger.md` §deferrals](execution-ledger.md#deferrals)).

## The lineage law {#lineage}

A closed campaign's deferrals ledger **becomes the next campaign's
mandate**: the owner commissions the follow-up by pointing at it, and
the new plan's opening table maps each deferral to a closing phase.
Campaigns form a chain — no work item is lost between links, and no
campaign starts blank ([`execution-ledger.md` §lineage](execution-ledger.md#lineage)).

## Re-derive for your project {#re-derive}

Run this prompt once to adapt the format to a concrete project:

```
Read CAMPAIGN-PLAN-FORMAT.md, phase-gates.md, and execution-ledger.md.
Adapt the campaign-plan format to this project:
1. Name the gate panel: the exact commands that define this project's
   green floor, and how long a full run takes.
2. Name where campaign plans live (a version-controlled directory)
   and the filename convention (<NAME>-PLAN-v<N>.md).
3. Name the owner: who commissions campaigns and rules review points.
4. Write the quick-start block a fresh session runs before Phase 0.
5. Draft the whole-campaign acceptance skeleton: the script shape
   that asserts an end state in this project's tooling.
6. Record the adapted conventions in the project's boot documents so
   every future session loads them.
Do not start a campaign; produce the adapted conventions only.
```

## Summary {#summary}

- A campaign is a multi-commit change executed as gated phases,
  planned to run cold — by a stranger, today.
- Five roles: PLAN, BASELINE, PREDICTIONS, LOG, REPORT — one file by
  default. Skip the report and the campaign learns nothing durable.
- Mandate verbatim; arithmetic exact; facts verified at writing;
  decisions carry their rejections; predictions falsifiable;
  non-goals named so they stay visible.
- The deferrals ledger seeds the next campaign's mandate.
