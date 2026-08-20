# The Campaign Plan Format {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what* a campaign is,
*which* artifact roles its paper trail carries, and the *canonical
section skeleton* of a campaign plan — the one document that lets a
large change be planned once and executed cold. @status:impl/done

@fact:sibling-document-pointers Phase mechanics:
[`phase-gates.md`](phase-gates.md); the record half:
[`execution-ledger.md`](execution-ledger.md). @status:impl/done

## What a campaign is {#what}

@fact:A-CAMPAIGN-IS-A-MULTI-COMMIT-CHANGE-TOO-BIG-FOR-ONE-SESSION A campaign is a multi-commit change too big for one session: a
package-family rename, a debt drain across the whole codebase, a new
subsystem landed in six waves. @status:impl/done

@fact:A-CAMPAIGN-IS-GATED-BY-PHASES-AND-PLANNED-TO-RUN-COLD It is executed as **gated phases** —
each ends with the project's full gate panel green — and it is
planned to run **cold**: by a fresh session with no memory of the
planning conversation, by a different person, or by the author after
total context loss. @status:impl/done

@fact:THE-TEST-IS-WHETHER-A-STRANGER-COULD-EXECUTE-IT-TODAY The test of a good plan is not "could I execute
this tomorrow?" but "could a stranger execute this today?". @status:impl/done

@fact:PAY-THE-FORMATS-COST-ONLY-FOR-WORK-THAT-SPANS-SESSIONS The
format's cost is real: pay it only when the work spans sessions or
more than a handful of commits. @status:impl/done

@fact:THE-CARRIER-CHOICE-LIVES-IN-FLOW-ADDRESSABLE-SPECS What decides whether to pay this format's cost at all — or take
another carrier for the work — is a choice with its own rule, and
that rule lives in `flow:addressable-specs`'s `#what-goes-where`. @status:impl/done

## The five artifact roles {#artifacts}

@fact:paper-trail-carries-five-roles-lead A campaign's paper trail carries five roles: @status:impl/done

| Role | Written | Purpose |
|---|---|---|
| @fact:ROW-ROLE-PLAN **PLAN** @status:impl/done | before work starts @status:impl/done | the cold-executable recipe @status:impl/done |
| @fact:ROW-ROLE-BASELINE **BASELINE** @status:impl/done | at campaign open @status:impl/done | the frozen starting facts: gate-panel state, inventory counts, the numbers phases are measured against @status:impl/done |
| @fact:ROW-ROLE-PREDICTIONS **PREDICTIONS** @status:impl/done | at campaign open @status:impl/done | falsifiable expectations, stated *before* execution so the report can honestly say confirmed / falsified / surprised @status:impl/done |
| @fact:ROW-ROLE-LOG **LOG** @status:impl/done | during execution @status:impl/done | the running record: per-phase commit maps, deviations, discovered work @status:impl/done |
| @fact:ROW-ROLE-REPORT **REPORT** @status:impl/done | at campaign close @status:impl/done | results vs predictions; what the campaign taught @status:impl/done |

@fact:THE-ONE-FILE-DIALECT-CARRIES-ALL-FIVE-ROLES **The one-file dialect (default).** One plan document carries all
five roles: the baseline in the target-arithmetic and current-state
sections, the predictions in their own section, the log in the
execution ledger, the report in the execution-record block prepended
at close — one file, one resume pointer. @status:impl/done

@fact:THE-ROLES-NOT-THE-COUNT-ARE-THE-CONTRACT Very large campaigns may
split the roles into files; the roles, not the count, are the contract. @status:impl/done

@fact:two-laws-govern-the-set-lead Two laws govern the set: @status:impl/done

- @fact:LAW-SKIPPING-THE-REPORT-LEARNS-NOTHING-DURABLE **a campaign that skips the REPORT learns
  nothing durable,** @status:impl/done
- @fact:LAW-NO-WRITTEN-PLAN-MEANS-NO-RESUMPTION **and a campaign without a written PLAN cannot be resumed
  by anyone but its author's context window.** @status:impl/done

## The section skeleton {#skeleton}

@fact:the-canonical-plan-in-order The canonical plan, in order. @status:impl/done

@fact:WHICH-SECTIONS-ARE-WRITTEN-BEFORE-EXECUTION Sections 1 and 3–13 are written before
execution; 2, 14, and 15 are filled by the executing session. @status:impl/done

### 1 — Title and status line {#s1-status}

@fact:the-title-and-the-status-block-lead The title names the campaign and its one-line point; under it, an
italic status block: @status:impl/done

- @fact:STATUS-BLOCK-THE-LIFECYCLE-STATE the lifecycle state (`PLANNED` → `EXECUTING` →
  `EXECUTED <date>`), @status:impl/done
- @fact:STATUS-BLOCK-THE-TREE-THE-PLAN-WAS-WRITTEN-AGAINST the tree the plan was written against, @status:impl/done
- @fact:STATUS-BLOCK-THE-COLD-START-FLAG and the cold-start flag. @status:impl/done

@fact:OWNER-REVIEW-MAY-ANNOTATE-THE-STATE Owner review may annotate the state
(`ACCEPTED with owner amendments, <date>`) before execution. @status:impl/done

```
# CACHE-DRAIN-PLAN v0.1 — retire the legacy cache, consumer by consumer
_Status: PLANNED · written against tree `57fa42e` · cold-executable:
any phase is a safe stop; the floor is green at every boundary._
```

### 2 — Execution record (prepended at close) {#s2-execution-record}

@fact:EMPTY-AT-AUTHORING Empty at authoring. @status:impl/done

@fact:at-close-a-block-is-prepended-lead At close, the executing session prepends a block
into the status area: @status:impl/done

- @fact:RECORD-THE-COMMIT-RANGE commit range, @status:impl/done
- @fact:RECORD-THE-PER-PHASE-DELTAS per-phase deltas, @status:impl/done
- @fact:RECORD-WHICH-PREDICTIONS-HELD which
  predictions held ([`execution-ledger.md` §execution-record](execution-ledger.md#execution-record)). @status:impl/done

### 3 — The mandate {#s3-mandate}

@fact:THE-MANDATE-IS-QUOTED-VERBATIM-AND-DATED The owner's commissioning words, **quoted verbatim and dated** —
never paraphrased into blandness. @status:impl/done

@fact:SCOPE-QUESTIONS-RESOLVE-AGAINST-THE-MANDATE Scope questions resolve against
this text; the executor never re-litigates it. @status:impl/done

```
Mandate (owner, 2026-07-07): "take everything listed in the previous
campaign's deferral ledger and plan its implementation; do not wait
for the full pilot — build a small demo project instead."
```

### 4 — Target arithmetic {#s4-arithmetic}

@fact:BASELINE-AND-EXIT-STATE-ARE-EXACT-COUNTS-THAT-RECONCILE Baseline and exit state as **exact counts** that reconcile: every
baseline unit ends in the exit state or in a phase that removes it. @status:impl/done

@fact:VAGUE-TARGETS-MAKE-THE-REPORT-UNWRITABLE Vague targets make the report unwritable. @status:impl/done

```
Baseline at plan time: 130 findings = 68 message-format + 28
file-budget + 24 banned-call + 8 owner-gated + 2 one-shots.
Exit state: 10 = 8 owner-gated (out of scope) + 2 parked (named).
Everything else reaches zero.
```

### 5 — Current-state facts (verified; do not re-discover) {#s5-facts}

@fact:FACTS-ARE-GATHERED-AT-AUTHORING-WITH-POINTERS Facts gathered at authoring, with file-and-line pointers, marked so
the executor trusts them instead of re-investigating. @status:impl/done

@fact:WRONG-FACTS-HERE-ARE-THE-MOST-EXPENSIVE-PLAN-BUG **Wrong facts
here are the most expensive class of plan bug: verify while
writing.** @status:spec/done

@fact:the-stale-trio-a-real-campaign-recorded A real campaign recorded three files as stale at
566/556/554 lines; the true sizes were 609/612/608 — the author
counted non-blank lines where the gate counts physical ones. @status:spec/done

@fact:a-phase-zero-probe-caught-it A Phase
0 probe caught what would have pruned live debt as stale. @status:spec/done

### 6 — Decisions D1–DN {#s6-decisions}

@fact:EVERY-DESIGN-DECISION-IS-NUMBERED Every design decision, numbered. @status:impl/done

@fact:EACH-DECISION-WEIGHS-ITS-OPTIONS-AND-REASONS-EVERY-REJECTION Each weighs its options, marks the
chosen one, and gives every rejection a reason — so a mid-execution
surprise is resolved in the spirit of the plan, and nobody re-opens
a settled question. @status:impl/done

@fact:REJECTIONS-ARE-AS-LOAD-BEARING-AS-THE-CHOICE Rejections are as load-bearing as the choice. @status:impl/done

```
### D4 — how the shared engine reaches both consumers
- (α) rewrite cross-package paths at install time — real product
  surface; rejected here, named as future work in the follow-up spec.
- (β) align the two layouts — breaks a published invariant; rejected.
- (γ) vendor-sync (CHOSEN): one authored home, byte-identical synced
  copies, a `--check` gate makes drift mechanically impossible.
```

### 7 — Predictions {#s7-predictions}

@fact:PREDICTIONS-ARE-NUMBERED-FALSIFIABLE-AND-CHECKED-ONE-BY-ONE Numbered, falsifiable, stated before execution, checked one by one in
the report — "P3 — fewer than 10 test expectations break across all
68 message edits: most tests match error kinds, not strings." @status:impl/done

@fact:A-PREDICTION-THAT-CANNOT-FAIL-IS-A-HOPE A prediction that cannot fail is a hope, not a prediction. @status:impl/done

### 8 — Phases {#s8-phases}

@fact:PHASE-ZERO-PRODUCES-NO-COMMITS-AND-LATER-PHASES-CARRY-FOUR-ELEMENTS Phase 0 is always spikes and probes and produces no commits; every
later phase carries numbered steps, its own exit criterion, its own
prediction, and its planned commit set with subjects spelled in
advance. @status:impl/done

@fact:phase-anatomy-pointer Full anatomy and gate rules: [`phase-gates.md`](phase-gates.md). @status:impl/done

@fact:A-PLAN-REFERENCES-FEATURE-DOCUMENTS-IT-DOES-NOT-RESTATE-THEM **A plan references feature documents; it does not restate them.**
Where a campaign executes work that has its own feature documents,
the plan's phases and batches name those documents and stop there.
The plan stays what it is — a statement of order, intent and
acceptance — while each feature's own contract stays in one place
with one author. @status:impl/done

@fact:the-plan-is-the-copy-that-goes-stale This is the general addressing law
(`flow:addressable-specs`'s `##A-NORMATIVE-VALUE-LIVES-AT-EXACTLY-ONE-ANCHOR`)
applied to this genre, and the echo names its anchor as that law requires.
Restating a feature's substance inside the plan produces two statements of one
truth with nothing forcing them to agree, and the plan is the copy that will go
stale, because it is the one built to be thrown away. @status:spec/done

### 9 — Risks and fallbacks {#s9-risks}

@fact:EVERY-RISK-CARRIES-A-DETECTION-SIGNAL-AND-A-PLAN-B Named risks, each with its detection signal and its plan B — "flaky
network: re-probe at each network-facing step; worst case those
steps land red-pending-network, recorded, everything else lands." @status:impl/done

@fact:A-RISK-WITHOUT-A-FALLBACK-IS-A-WISH A risk without a fallback is a wish that nothing goes wrong. @status:impl/done

### 10 — Non-goals {#s10-non-goals}

@fact:NON-GOALS-ARE-NAMED-SO-THEY-STAY-VISIBLE "What this plan deliberately does NOT do" — **named, so they stay
visible.** @status:impl/done

@fact:EVERY-NON-GOAL-CARRIES-A-REASON-AND-A-DISPOSITION Each non-goal carries a reason and a disposition: deferred
to a named follow-up, held by the owner, or rejected outright —
"does NOT extend the gate to the two remaining modules: that is the
NEXT campaign's opening move, after this queue closes." @status:impl/done

### 11 — Quick-start for the executing session {#s11-quick-start}

@fact:the-quick-start-block-lead The literal shell block a cold session runs first: @status:impl/done

- @fact:QUICK-START-CONFIRM-THE-TREE confirm the tree, @status:impl/done
- @fact:QUICK-START-VERIFY-THE-GREEN-FLOOR verify the green floor, @status:impl/done
- @fact:QUICK-START-CAPTURE-THE-BASELINE-NUMBERS capture the baseline numbers. @status:impl/done

```sh
git log --oneline -3        # tree must match the status line
<gate-panel command>        # full panel green before Phase 0 opens
<baseline count command>    # must print the §4 baseline figure
```

### 12 — Whole-campaign acceptance {#s12-acceptance}

@fact:ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE A runnable script asserting the end state — the campaign's definition
of done, executed on a green floor at close, cited by the report. @status:impl/done

```sh
<gate-panel command>; echo "EXIT=$?"    # exit 0
test ! -d src/legacy_cache              # the drained module is gone
<banned-pattern search> | wc -l         # 0 references remain
```

### 13 — Review points {#s13-review-points}

@fact:REVIEW-POINTS-GO-OPEN-THEN-RESOLVED Decisions only the owner can make, escalated as numbered points:
`OPEN` with options and the executor's recommendation, later
annotated `RESOLVED` with the ruling verbatim
([`phase-gates.md` §review-points](phase-gates.md#review-points)). @status:impl/done

### 14 — Execution ledger {#s14-ledger}

@fact:THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS Filled by the executing session: per-phase commit maps binding hashes
to the planned subjects, with what each commit confirmed or falsified
([`execution-ledger.md`](execution-ledger.md)). @status:impl/done

### 15 — Deferrals ledger {#s15-deferrals}

@fact:EVERY-DEFERRAL-IS-NAMED-WITH-AN-OWNER-AND-A-DISPOSITION Everything the campaign chose not to do, named, one line each, with
an owner and a disposition. @status:impl/done

@fact:NOTHING-EVAPORATES Nothing evaporates: leftover work is in a
commit or in this ledger
([`execution-ledger.md` §deferrals](execution-ledger.md#deferrals)). @status:impl/done

## The lineage law {#lineage}

@fact:A-CLOSED-CAMPAIGNS-DEFERRALS-BECOME-THE-NEXT-MANDATE A closed campaign's deferrals ledger **becomes the next campaign's
mandate**: the owner commissions the follow-up by pointing at it, and
the new plan's opening table maps each deferral to a closing phase. @status:impl/done

@fact:CAMPAIGNS-FORM-A-CHAIN Campaigns form a chain — no work item is lost between links, and no
campaign starts blank ([`execution-ledger.md` §lineage](execution-ledger.md#lineage)). @status:impl/done

## A plan is a temporary artifact {#temporary}

@fact:A-PLAN-MUST-BE-DELETABLE-WHEN-EXECUTED A campaign plan is **temporary by design**. When it has been
executed it must be possible to delete it and have nothing break. If
deleting a finished plan breaks something, that something was
depending on scaffolding. @status:impl/done

@fact:THE-TEST-IS-DELETION-NOT-TIDINESS The test is deletion, and it is worth stating as a test rather
than as a preference: a project cannot tell, by looking, whether its
plans are load-bearing. Asking "could this file be removed today?"
answers it in one question, and answers it the same way for every
plan. @status:impl/done

@fact:SIGNIFICANT-CONTENT-MOVES-INTO-THE-SPECS-ON-CLOSURE Significant content — a rule the campaign established, a
decision with its reasons, a measurement that will be re-read — moves
**into the specifications** as the item closes. What stays in the plan
is the record of doing, which git already keeps. @status:impl/done

@fact:TOMBSTONES-ARE-PROCESS-SUPPORT-NOT-STRUCTURE A **tombstone** — the one-line trace a closed row leaves in
the plan — is process support for whoever is walking the plan, and
nothing more. It is not part of the project's structure, nothing may
cite it, and it goes when the plan goes. @status:impl/done

@fact:WHY-DISPOSABLE-ZONES-KEEP-DURABLE-FINDINGS The failure this prevents has a shape worth recognising: a
correct, carefully-argued finding written into a campaign document,
which behaves exactly as the zone promises — it is disposable, so it
is disposed of, and the finding is rediscovered months later at full
cost. The zone was not wrong; filing something durable there was. @status:spec/done

@fact:THE-ADDRESSING-HALF-LIVES-IN-ADDRESSABLE-SPECS The other half of this rule — that a statement cites the spec
element and never the plan row, and that re-pointing citations is part
of closing — belongs to the addressable-specs flow, at
`spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#disposable-targets`.
One law, two homes, one pointer. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:re-derive-lead Run this prompt once to adapt the format to a concrete project: @status:impl/done

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

- @fact:SUM-A-CAMPAIGN-IS-GATED-PHASES-PLANNED-TO-RUN-COLD A campaign is a multi-commit change executed as gated phases,
  planned to run cold — by a stranger, today. @status:impl/done
- @fact:SUM-FIVE-ROLES-ONE-FILE-BY-DEFAULT Five roles: PLAN, BASELINE, PREDICTIONS, LOG, REPORT — one file by
  default. Skip the report and the campaign learns nothing durable. @status:impl/done
- @fact:SUM-THE-PLANS-STANDING-OBLIGATIONS Mandate verbatim; arithmetic exact; facts verified at writing;
  decisions carry their rejections; predictions falsifiable;
  non-goals named so they stay visible. @status:impl/done
- @fact:SUM-THE-DEFERRALS-LEDGER-SEEDS-THE-NEXT-MANDATE The deferrals ledger seeds the next campaign's mandate. @status:impl/done
