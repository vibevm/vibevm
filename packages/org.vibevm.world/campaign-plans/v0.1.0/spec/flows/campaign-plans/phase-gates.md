# Phase Gates {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *how* a campaign's
phases are cut, gated, and resumed: the Phase 0 spike discipline, the
anatomy of an executable phase, the safe-stop law, review points, and
the rule for discovered-necessary work. @impl/done

##sibling-document-pointers The surrounding plan format:
[`CAMPAIGN-PLAN-FORMAT.md`](CAMPAIGN-PLAN-FORMAT.md); the record
half: [`execution-ledger.md`](execution-ledger.md). @impl/done

## Phase 0 — spikes and probes, no commits {#phase-zero}

##EVERY-CAMPAIGN-OPENS-WITH-A-PHASE-THAT-PRODUCES-NO-COMMITS Every campaign opens with a phase that produces **no commits**. @impl/done

##phase-zeros-job-lead Its job
is to convert the plan's riskiest assumptions into observed facts
before anything lands: @impl/done

- ##PHASE-ZERO-PROBE-THE-ENVIRONMENT probe the environment the phases depend on, @impl/done
- ##PHASE-ZERO-SPIKE-THE-ONE-MECHANISM spike the one mechanism the design leans on, @impl/done
- ##PHASE-ZERO-RE-MEASURE-THE-NUMBERS re-measure the numbers
  the arithmetic trusts, @impl/done
- ##PHASE-ZERO-VERIFY-WHAT-THE-PLAN-CALLS-INERT verify that whatever the plan calls inert
  really is inert. @impl/done

##three-rules-lead Three rules: @impl/done

- ##RULE-NO-TREE-CHANGES-SURVIVE-THE-SPIKES **No tree changes survive the spikes.** Scratch directories,
  throwaway scripts, temporary workspaces — all discarded. The only
  durable output is findings, written into the plan itself. @impl/done
- ##RULE-PHASE-ZERO-GATES-EVERYTHING-AFTER **Phase 0 gates everything after.** A red spike does not "get
  noted for later" — it **rewrites the affected Decision in the plan,
  in place, before Phase 1 commits anything**. The plan is cheap to
  change while nothing is committed; that is the entire point of
  spiking first. @impl/done
- ##RULE-FINDINGS-ARE-RECORDED-EVEN-WHEN-GREEN **Findings are recorded even when green.** The phase's exit is a
  findings list appended under the phase, not a feeling of readiness.
  Each finding is marked binding on the phases it affects. @impl/done

##the-build-topology-spike Two real corrections Phase 0 has bought, anonymized: a build-topology
spike revealed that a shared engine depended on a fourth component
the plan's move-set had not listed — the decision was corrected from
"move three" to "move four" before any phase committed to the wrong
topology. @spec/done

##the-protocol-spike A protocol spike proved the transport was line-delimited
where the draft decision said length-prefixed framing — the decision
was rewritten before the transport layer was authored. @spec/done

##the-fix-cost-one-edit-not-a-rollback In both cases
the fix cost one edit; discovered in Phase 3, it would have cost a
rollback. @spec/done

## Anatomy of a phase {#anatomy}

##a-phase-carries-four-elements-lead Every phase after Phase 0 is written as a self-contained executable
unit carrying four elements: @impl/done

| Element | Content |
|---|---|
| ##ROW-ELEMENT-NUMBERED-STEPS **Numbered steps** @impl/done | the recipe: commands, paths, exact edits, in order @impl/done |
| ##ROW-ELEMENT-EXIT-ACCEPTANCE **Exit / acceptance** @impl/done | the criterion that closes the phase, checkable by command @impl/done |
| ##ROW-ELEMENT-PREDICTION **Prediction** @impl/done | the phase's own falsifiable expectation @impl/done |
| ##ROW-ELEMENT-COMMIT-SET **Commit set** @impl/done | the planned commits, subjects spelled in advance @impl/done |

##worked-phase-lead A worked phase, condensed from a real campaign: @spec/done

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

##why-subjects-are-spelled-in-advance Why subjects are spelled in advance: the split of the work into
commits-by-meaning happens at planning time, when the whole change is
visible. @spec/done

##EXECUTION-STAGES-ONTO-A-PRE-DRAWN-MAP Execution then stages onto a pre-drawn map, and the ledger
later binds real hashes to the planned subjects — any drift between
the two is itself a recorded finding. @impl/done

## The safe-stop law {#safe-stop}

##ANY-PHASE-BOUNDARY-IS-A-SAFE-STOP **Any phase boundary is a safe stop, and the project's green floor
holds at every boundary.** @impl/done

##THE-PANEL-IS-THE-FULL-ONE-NOT-THE-PARTS-THE-PHASE-TOUCHED The *full* gate panel — build,
verification suite, every standing check — not just the parts the
phase touched. @impl/done

##A-PHASE-THAT-LEAVES-THE-FLOOR-RED-IS-STILL-OPEN A phase that leaves the floor red is not done; it is
open, whatever its steps say. @impl/done

##what-the-law-buys-lead What the law buys: @impl/done

- ##BUYS-THE-EXECUTOR-CAN-STOP-AT-ANY-BOUNDARY The executor can stop at any boundary — end of day, session death,
  an owner interrupt — and leave no broken tree behind. @impl/done
- ##BUYS-A-FAILED-PHASE-ROLLS-BACK-ALONE A failed phase rolls back without losing prior phases. @impl/done
- ##BUYS-PHASES-MUST-BE-CUT-TO-MAKE-THIS-POSSIBLE Phases must be *cut* to make this possible: a restructuring that
  breaks the build across two phases is one phase, not two. If two
  steps cannot be separated by a green boundary, they are one phase. @impl/done

## Resumability {#resume}

##THE-PLAN-PLUS-THE-LEDGER-ARE-THE-RESUME-POINTER The plan plus its execution ledger are the resume pointer — **no
conversational context needed**. @impl/done

##a-fresh-session-reads-boot-then-the-plan-lead A fresh session resumes by reading
the project's boot documents, then the plan top to bottom: @impl/done

- ##RESUME-THE-STATUS-LINE-NAMES-THE-STATE the status
  line names the campaign's state, @impl/done
- ##RESUME-THE-LEDGERS-LAST-ENTRY-NAMES-THE-LAST-LANDED-PHASE the ledger's last entry names the
  last landed phase, @impl/done
- ##RESUME-THE-FIRST-UNEXECUTED-PHASE-IS-THE-WORK and the first unexecuted phase is the work. @impl/done

##the-journal-points-at-the-plan If the project keeps a working journal, the campaign updates the
journal's standing line at every boundary ("Phase N landed, floor
green, next: Phase N+1") and the journal points at the plan; the plan
file, not the journal, carries the campaign detail. @impl/done

##RESUMPTION-STATE-LIVES-IN-THE-REPOSITORY Either way the
rule is the same: **resumption state lives in the repository, never
in a session.** @impl/done

## Review points {#review-points}

##AN-OWNER-ONLY-DECISION-BECOMES-A-NUMBERED-REVIEW-POINT A decision that only the owner can make — a scope call, a policy
reversal, a trade-off between published invariants — becomes a
numbered review point instead of a silent executor guess. @impl/done

##A-REVIEW-POINT-IS-STATED-OPEN-THEN-ANNOTATED-RESOLVED It is stated `OPEN` with the options and the executor's
recommendation; when the owner rules, it is annotated `RESOLVED` with
the ruling **verbatim** and dated, and the affected Decisions are
rewritten in place. @impl/done

##THE-REVIEW-POINT-KEEPS-THE-HISTORY The review point keeps the history; the Decisions carry the
outcome. @impl/done

```
3. Package topology — OPEN: one package, or one per language?
   Executor recommends per-language (skew analysis in D2).
   → RESOLVED (owner, 2026-07-07, reverses the draft): "make it a
   separate kind; more kinds may follow later." D1 rewritten in
   place; consequences surfaced in the amendment discussion.
```

##WORK-BLOCKED-ON-AN-OPEN-REVIEW-POINT-DOES-NOT-START Work that depends on an `OPEN` review point does not start; phases
that do not depend on it may proceed. @impl/done

##a-reversal-is-normal-not-exceptional A ruling that reverses a
Decision is normal, not exceptional — the format exists so reversals
land in the plan, not in a chat scrollback. @spec/done

## Discovered-necessary work {#discovered-work}

##MID-PHASE-DISCOVERIES-ARE-LEGAL-BUT-RECORDED Mid-phase discoveries are **legal but recorded — never done silently
"while I was here."** @impl/done

##two-dispositions-lead A discovery has exactly two dispositions: @impl/done

1. ##DISPOSITION-ENTER-THE-CURRENT-PHASE-EXPLICITLY **Enter the current phase explicitly:** a ledger entry naming what
   was discovered and why it cannot wait, plus its own commit in the
   phase's commit set. @impl/done
2. ##DISPOSITION-DEFER-BY-NAME **Defer by name:** a line in the deferrals ledger with an owner
   and a disposition, drained by a later phase or a later campaign. @impl/done

##silent-scope-growth-loses-auditability Silent scope growth is how campaigns lose auditability: the diff
stops mapping to the plan, and the next reader cannot tell the
planned from the improvised. @spec/done

##EVERY-DEVIATION-LEAVES-A-WRITTEN-TRACE-WHEN-IT-IS-MADE The rule is not "never deviate" — the
ledger records deviations precisely because they happen — the rule is
that every deviation leaves a written trace at the moment it is made. @impl/done

## Summary {#summary}

- ##SUM-PHASE-ZERO-COMMITS-NOTHING Phase 0 spikes, probes, and commits nothing; a red spike rewrites
  the affected Decision before Phase 1 lands a single commit. @impl/done
- ##SUM-EVERY-LATER-PHASE-CARRIES-FOUR-ELEMENTS Every later phase carries steps, an exit criterion, its own
  prediction, and its commit set with subjects spelled in advance. @impl/done
- ##SUM-ANY-BOUNDARY-IS-A-SAFE-STOP Any phase boundary is a safe stop; the full gate panel is green at
  every boundary, or the phase is still open. @impl/done
- ##SUM-RESUMPTION-STATE-LIVES-IN-THE-REPOSITORY The plan plus the ledger are the resume pointer; resumption state
  lives in the repository, never in a session. @impl/done
- ##SUM-OWNER-ONLY-DECISIONS-BECOME-REVIEW-POINTS Owner-only decisions become review points: OPEN, then RESOLVED with
  the ruling verbatim. @impl/done
- ##SUM-DISCOVERED-WORK-IS-ENTERED-OR-DEFERRED Discovered work enters the phase explicitly or is deferred by name
  — never done silently. @impl/done
