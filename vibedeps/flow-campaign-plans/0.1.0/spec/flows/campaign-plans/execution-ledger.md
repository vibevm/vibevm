# The Execution Ledger {#root}

**Scope of this document.** This file defines the *record half* of a
campaign: flipping the status line, the execution-record block, the
per-phase commit maps, the honesty rules, the closing report, and the
deferrals ledger that seeds the next campaign. The authoring half:
[`CAMPAIGN-PLAN-FORMAT.md`](CAMPAIGN-PLAN-FORMAT.md); phase
mechanics: [`phase-gates.md`](phase-gates.md).

## Why the record half exists {#why}

The plan says what should happen; the ledger says what did. Commits
alone cannot carry the comparison — a hash proves a change landed,
not that it landed *as planned*, confirmed a prediction, or corrected
a decision. The ledger binds the two: every phase's real commits
mapped onto the plan's intent, with divergence stated where it
occurred. A future reader audits the campaign from this file plus
`git log` and needs nothing else.

## Flipping the status line {#status-flip}

The plan's status line is the campaign's single lifecycle indicator:

| State | When |
|---|---|
| `PLANNED` | authored; optionally annotated `ACCEPTED with owner amendments, <date>` after review |
| `EXECUTING` | flipped when Phase 0 opens; since Phase 0 commits nothing, the flip rides the plan-amendment commit that records the Phase 0 findings, or Phase 1's first commit |
| `EXECUTED <date>` | flipped at close, together with the prepended execution record |

At every later phase boundary the executing session refreshes the
line's tail — "Phase N landed, floor green, next: Phase N+1" — as
part of that phase's commit set. The status line is what a cold
session reads first; it must never lag the tree.

## The execution-record block {#execution-record}

At close, a summary block is **prepended** into the plan's status
area — the first thing any future reader sees, above the now-historic
planning prose. It carries: the commit range, the per-phase deltas
against the target arithmetic, and the verdict on every prediction.

```
_Execution record: all six phases ran to the exit state — baseline
130 → 10, exactly §0's arithmetic; 19 commits (`254b974` …
`475fa75`). Two predictions falsified and recorded in place: the
stale-trio premise (§0 correction) and the one-third waiver rate
(3/24 actual — restructures dominated); the other four held. The
journal carries the checkpoint._
```

Note the shape: exact counts against the plan's own §0, the hash
range, falsifications named with their observed values, and a pointer
to the project's journal for the session-level checkpoint.

## Per-phase commit maps {#commit-maps}

Each executed phase gets a ledger section: `EXECUTED <date>` plus a
commit map. One entry per commit — **hash, the conventional-commit
subject, what it did, and what it confirmed or falsified**:

```
### Phase 3 — EXECUTED (2026-07-07); commit map

- `fdd6baf` feat(packages): the kind's first real inhabitant. TWO
  PLAN SIMPLIFICATIONS recorded: the planned library extraction
  proved unnecessary — the command-line layer already IS the library
  — and the version bump fell away with it: bump only what changes
  (D10 amended by execution).
- `cf2e64c` build(deps): the project repins itself as the first
  consumer.
- Panel at the boundary: full gate panel green; benchmark corpus
  9/9 — no target moved.
```

The map is written at the phase boundary, while the reasoning is
fresh — not reconstructed at close. Planned subjects that drifted
during execution are recorded with the drift ("split into two
commits: the fixture change deserved its own revert point").

## Honesty rules {#honesty}

The ledger's value is exactly proportional to its honesty:

- **Say "no target moved" where it is true.** A close panel that
  reports numbers without saying whether they shifted reads as
  concealment in audit. "Corpus 9/9 — no target moved" costs five
  words and closes the question.
- **Execution may correct the plan's own decisions — and says so.**
  The plan is not sacred; silent divergence is the sin. When
  execution proves a decision wrong or unnecessary, the ledger entry
  states it and the Decision is amended in place, as in the Phase 3
  map above.
- **Falsified predictions are findings, not failures.** They are
  recorded twice: at the prediction (in place, so the planning prose
  never misleads a re-reader) and in the execution record's verdict
  list.
- **Discovered work appears with its commits.** An entry that was not
  in the plan says so explicitly — the
  [`phase-gates.md` §discovered-work](phase-gates.md#discovered-work)
  rule, enforced at write time.

## The closing report {#report}

The campaign closes by checking **every prediction, one by one**:
held (with the observed value), falsified (with the observed value
and where the correction landed), or surprised (something happened
the predictions never addressed). Then the lessons: which decisions
execution amended, which rules or estimates misfired, what the next
campaign inherits.

**A campaign that skips the report learns nothing durable.** The work
survives in the commits either way; the *learning* — which planning
assumptions held, which estimates were systematically off — survives
only here, and it is the only part that compounds across campaigns.

In the one-file dialect the report is not a separate document: it is
the execution-record block plus the per-prediction verdicts recorded
in place. Before writing it, run the whole-campaign acceptance script
on a green floor; the report cites its output.

## The deferrals ledger {#deferrals}

Everything the campaign chose not to do is **named** — one line each,
with an owner (who decides its fate) and a disposition. Nothing
evaporates: at close, every piece of leftover work is either in a
commit or in this ledger.

```
- DEF-3 — mirror publish: EXECUTABLE, held on the owner's word
  (policy, not capability).
- DEF-5 — the second stack's gate step: waits for the demo project
  this campaign built as its prerequisite.
```

A deferral is not a TODO comment in code, not a chat promise, not a
mental note — those all evaporate. It lives in the plan file, under
its campaign, until a later campaign drains it.

## The lineage law {#lineage}

The next campaign's mandate is drained **from** the deferrals ledger:

```
Campaign A closes:  "§15 DEF-3 — mirror publish stays held (owner call)."
Campaign B opens:   Mandate (owner, dated): "close every §15 deferral
                    of campaign A." DEF-3 becomes Phase 4.
```

A real chain, anonymized: a self-sufficiency campaign closed with
seven named deferrals; weeks later the owner's commissioning words
for the follow-up were, verbatim, "take everything listed in the
previous campaign's deferral ledger and plan its implementation" —
and the new plan's opening table mapped each of the seven to the
phase that closes it. The ledger of one campaign *is* the raw mandate
of the next; the chain never breaks, and nothing is re-discovered
from scratch.

## Summary {#summary}

- The status line is the lifecycle: PLANNED → EXECUTING →
  EXECUTED <date>, refreshed at every phase boundary.
- At close, prepend the execution record: commit range, deltas
  against the arithmetic, a verdict on every prediction.
- Per-phase commit maps bind hashes to planned subjects and state
  what each commit confirmed or falsified — written at the boundary,
  not reconstructed later.
- Honesty rules: "no target moved" said aloud; corrected decisions
  said aloud; falsified predictions recorded in place.
- A campaign that skips the report learns nothing durable.
- Deferrals live in the ledger, named and owned — and the next
  campaign's mandate drains from it.
