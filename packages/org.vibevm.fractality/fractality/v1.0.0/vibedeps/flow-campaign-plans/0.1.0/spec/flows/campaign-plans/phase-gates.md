# Phase Gates {#root}

**Scope of this document.** This file defines *how* a campaign's
phases are cut, gated, and resumed: the Phase 0 spike discipline, the
anatomy of an executable phase, the safe-stop law, review points, and
the rule for discovered-necessary work. The surrounding plan format:
[`CAMPAIGN-PLAN-FORMAT.md`](CAMPAIGN-PLAN-FORMAT.md); the record
half: [`execution-ledger.md`](execution-ledger.md).

## Phase 0 — spikes and probes, no commits {#phase-zero}

Every campaign opens with a phase that produces **no commits**. Its
job is to convert the plan's riskiest assumptions into observed facts
before anything lands: probe the environment the phases depend on,
spike the one mechanism the design leans on, re-measure the numbers
the arithmetic trusts, verify that whatever the plan calls inert
really is inert.

Three rules:

- **No tree changes survive the spikes.** Scratch directories,
  throwaway scripts, temporary workspaces — all discarded. The only
  durable output is findings, written into the plan itself.
- **Phase 0 gates everything after.** A red spike does not "get
  noted for later" — it **rewrites the affected Decision in the plan,
  in place, before Phase 1 commits anything**. The plan is cheap to
  change while nothing is committed; that is the entire point of
  spiking first.
- **Findings are recorded even when green.** The phase's exit is a
  findings list appended under the phase, not a feeling of readiness.
  Each finding is marked binding on the phases it affects.

Two real corrections Phase 0 has bought, anonymized: a build-topology
spike revealed that a shared engine depended on a fourth component
the plan's move-set had not listed — the decision was corrected from
"move three" to "move four" before any phase committed to the wrong
topology. A protocol spike proved the transport was line-delimited
where the draft decision said length-prefixed framing — the decision
was rewritten before the transport layer was authored. In both cases
the fix cost one edit; discovered in Phase 3, it would have cost a
rollback.

## Anatomy of a phase {#anatomy}

Every phase after Phase 0 is written as a self-contained executable
unit carrying four elements:

| Element | Content |
|---|---|
| **Numbered steps** | the recipe: commands, paths, exact edits, in order |
| **Exit / acceptance** | the criterion that closes the phase, checkable by command |
| **Prediction** | the phase's own falsifiable expectation |
| **Commit set** | the planned commits, subjects spelled in advance |

A worked phase, condensed from a real campaign:

```
## 5. Phase 2 — the 24 banned-call sites: convert or testify

Judgment rule per site, in priority order: (a) real fallibility →
route through the layer's error type; (b) a true invariant,
unreachable by construction → an annotated waiver carrying a reason;
(c) test-support code that leaked into production → move it out.

1. Batch 2a — the workspace layer (11 sites, mostly (a)).
2. Batch 2b — the resolver layer (13 sites, prime (b) candidates —
   "the checker already validated this branch" is a construction
   invariant).

*Exit:* banned-call findings = 0; every waiver carries a reason a
reviewer can argue with.
*Prediction:* at least a third of the 24 land as waivers — the ban's
value here is the testimony, not the conversion count.
*Commits:* `refactor(workspace): route fallible edges through the
error type` · `refactor(resolver): waive construction invariants
with reasons`.
```

Why subjects are spelled in advance: the split of the work into
commits-by-meaning happens at planning time, when the whole change is
visible. Execution then stages onto a pre-drawn map, and the ledger
later binds real hashes to the planned subjects — any drift between
the two is itself a recorded finding.

## The safe-stop law {#safe-stop}

**Any phase boundary is a safe stop, and the project's green floor
holds at every boundary.** The *full* gate panel — build,
verification suite, every standing check — not just the parts the
phase touched. A phase that leaves the floor red is not done; it is
open, whatever its steps say.

What the law buys:

- The executor can stop at any boundary — end of day, session death,
  an owner interrupt — and leave no broken tree behind.
- A failed phase rolls back without losing prior phases.
- Phases must be *cut* to make this possible: a restructuring that
  breaks the build across two phases is one phase, not two. If two
  steps cannot be separated by a green boundary, they are one phase.

## Resumability {#resume}

The plan plus its execution ledger are the resume pointer — **no
conversational context needed**. A fresh session resumes by reading
the project's boot documents, then the plan top to bottom: the status
line names the campaign's state, the ledger's last entry names the
last landed phase, and the first unexecuted phase is the work.

If the project keeps a working journal, the campaign updates the
journal's standing line at every boundary ("Phase N landed, floor
green, next: Phase N+1") and the journal points at the plan; the plan
file, not the journal, carries the campaign detail. Either way the
rule is the same: **resumption state lives in the repository, never
in a session.**

## Review points {#review-points}

A decision that only the owner can make — a scope call, a policy
reversal, a trade-off between published invariants — becomes a
numbered review point instead of a silent executor guess. It is
stated `OPEN` with the options and the executor's recommendation;
when the owner rules, it is annotated `RESOLVED` with the ruling
**verbatim** and dated, and the affected Decisions are rewritten in
place. The review point keeps the history; the Decisions carry the
outcome.

```
3. Package topology — OPEN: one package, or one per language?
   Executor recommends per-language (skew analysis in D2).
   → RESOLVED (owner, 2026-07-07, reverses the draft): "make it a
   separate kind; more kinds may follow later." D1 rewritten in
   place; consequences surfaced in the amendment discussion.
```

Work that depends on an `OPEN` review point does not start; phases
that do not depend on it may proceed. A ruling that reverses a
Decision is normal, not exceptional — the format exists so reversals
land in the plan, not in a chat scrollback.

## Discovered-necessary work {#discovered-work}

Mid-phase discoveries are **legal but recorded — never done silently
"while I was here."** A discovery has exactly two dispositions:

1. **Enter the current phase explicitly:** a ledger entry naming what
   was discovered and why it cannot wait, plus its own commit in the
   phase's commit set.
2. **Defer by name:** a line in the deferrals ledger with an owner
   and a disposition, drained by a later phase or a later campaign.

Silent scope growth is how campaigns lose auditability: the diff
stops mapping to the plan, and the next reader cannot tell the
planned from the improvised. The rule is not "never deviate" — the
ledger records deviations precisely because they happen — the rule is
that every deviation leaves a written trace at the moment it is made.

## Summary {#summary}

- Phase 0 spikes, probes, and commits nothing; a red spike rewrites
  the affected Decision before Phase 1 lands a single commit.
- Every later phase carries steps, an exit criterion, its own
  prediction, and its commit set with subjects spelled in advance.
- Any phase boundary is a safe stop; the full gate panel is green at
  every boundary, or the phase is still open.
- The plan plus the ledger are the resume pointer; resumption state
  lives in the repository, never in a session.
- Owner-only decisions become review points: OPEN, then RESOLVED with
  the ruling verbatim.
- Discovered work enters the phase explicitly or is deferred by name
  — never done silently.
