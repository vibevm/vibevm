# The Sweep Playbook — the standing guardian {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · status: BETA · T1 · language-neutral** @status:impl/done

@fact:SWEEP-IS-THE-RECURRING-COUNTERPART *The recurring counterpart of the
[Raid Playbook](03-RAID-PLAYBOOK.md).* @status:impl/done

@fact:RAID-IS-A-SCHEDULED-CAMPAIGN *A raid is a scheduled campaign — scoped,
planned, finite.* @status:impl/done

@fact:SWEEP-IS-THE-STANDING-RUN *The **sweep** is the standing run that, executed
daily (or several times a day on an active tree), keeps a codebase inside the
Discipline instead of letting it drift back out between campaigns.* @status:impl/done

@fact:TERRAFORM-RAID-SWEEP-DIVISION-OF-LABOUR *Terraforming brings a tree into the
Discipline ([BROWNFIELD](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md)); raids move it
in planned steps; the sweep holds the ground in between.* @status:impl/done

@fact:STACKS-SHIP-TOOLS-THIS-DOCUMENT-IS-METHOD *Each language stack
ships the concrete tools and idioms (the Rust stack:
`rust-ai-native floor | health | test-gate | tripwire`, plus the sweep-idiom
section of its GUIDE); this document is the method.* @status:impl/done

## 0. The two truths {#truths}

1. @fact:TRUTH-GATES-ARE-THE-FLOOR **The gates are the floor; the sweep is the
   ceiling.** The binary gates — format check, tests, lints, the conform gate,
   the specmap check — say pass/fail and MUST be green before any sweep work.
   The collector's advisory facts (coverage gaps, danger bands, backlogs) sit
   *above* that floor and say what to harden next. A green sweep on a red floor
   is a contradiction: fix the floor. @status:impl/done
2. @fact:TRUTH-GATE-IS-TRUTH-COLLECTOR-IS-A-GUIDE **The gate is truth; the
   collector is a guide.** When the collector says a unit is ready to gate, the
   gate confirms or refutes it. Trust the gate. @status:impl/done

@fact:SWEEP-IS-COLLECTOR-FIRST Law 2 governs the sweep's shape: explanation capital
must be runnable capital, so the sweep is **collector-first** — a deterministic,
no-LLM fact-gatherer (the stack's `health` tool) computes the objective state,
and the operator (human or agent) acts on facts, not vibes. @status:impl/done

@fact:UNMECHANISED-CHECKS-ROUTE-TO-JUDGMENT Where a check cannot be mechanised yet
(the WISH rules), the sweep names it and routes it to judgment — it never
pretends prose is a gate. @status:impl/done

## 1. Tier structure {#tiers}

@fact:TIER-0-HARD-FLOOR **Tier 0 — the hard floor (every run, binary, first).**
Never sweep on a red tree. @status:impl/done

@fact:TIER-0-RUN-THE-FLOOR-COMMAND Run the stack's floor command (format →
build/tests → lints → conform gate → traceability check → test-gate where a
baseline registry exists → fast-loop budgets where enforced). @status:impl/done

@fact:TIER-0-RED-GATE-IS-THE-ONLY-WORK If any gate is red, the only work is making
it green. @status:impl/done

@fact:TIER-1-RATCHET **Tier 1 — the ratchet (every run; act on the collector's
facts).** Run the health collector and work its output in cheapest-win-first
order. @status:impl/done

@fact:canonical-moves-lead The canonical moves, each driven by a collector field,
never by memory: @status:impl/done

- @fact:MOVE-SPLIT-THE-DANGER-BAND **Split the danger band before it trips** —
  files approaching the length budget are landmines; measure with the rule
  (physical lines), not the eye. @status:impl/done
- @fact:MOVE-WIDEN-A-GATE-FOR-FREE **Widen a gate for free** — a unit the collector
  shows at zero drain (e.g. full doctest coverage) enters the corresponding gate
  list with zero work; the gate run confirms the prediction. @status:impl/done
- @fact:MOVE-DRAIN-THE-SMALLEST-BACKLOG **Drain the smallest backlog** — take the
  smallest-gap ungated unit, bring it to zero, then flip it into the gate (the
  expand-as-you-conform rhythm: **a unit enters a gate only after it drains to
  zero** — a flip must never widen a baseline). @status:impl/done
- @fact:MOVE-REJUSTIFY-THE-DEVIATION-DEBT **Re-justify the deviation debt** — walk
  each recorded deviation: does its reason still hold? A deviation whose
  invariant has since been encoded in a type/checker is removed and
  restructured. *A deviation with no live reason is a defect.* @status:impl/done
- @fact:MOVE-CATCH-CENSUS-REGRESSIONS **Catch census regressions** — for gated
  units the violation censuses must stay zero; a non-zero is a fresh violation
  that slipped a local check. Restructure beats testify: encode the invariant in
  a type or contract rather than recording an excuse. @status:impl/done

@fact:TIER-2-DRIFT **Tier 2 — drift (weekly; the slow rot the gates don't see).** @status:impl/done

- @fact:DRIFT-DEBT-REGISTRY **Debt registry:** run the tripwire tool; re-disposition
  any touched-and-open entry
  ([BROWNFIELD §3](mechanisms/BROWNFIELD-PROTOCOL-v0.1.md#registries)).
  New deficiencies found while sweeping are *filed* there, not left as prose. @status:impl/done
- @fact:DRIFT-DOC-CODE **Doc/code drift:** state checkpoints vs reality (see
  [WAL convention](06-WAL-CONVENTION.md) — freshness rule); architecture
  docs vs the real layout; roadmap self-staleness. A stale doc that governs
  code is filed as `stale-doc` debt. @status:impl/done
- @fact:DRIFT-MARKER-CENSUS **Marker census:** every TODO/FIXME/REVIEW/HACK is a
  WISH or a deferred decision — load-bearing ones graduate into the debt/intent
  registries, trivial ones are resolved or deleted. Prose that promises and
  doesn't deliver is false training signal. @status:impl/done
- @fact:DRIFT-GOLDEN-TRANSCRIPTS **Golden transcripts:** characterization oracles
  must fail loudly and be re-captured deliberately, never auto-updated — an
  auto-updated golden is a test that always passes. @status:impl/done
- @fact:DRIFT-TRACEABILITY-KIND-HYGIENE **Traceability kind hygiene:** a code edge
  into an unmarked spec unit prompts marking the unit's kind/revision in the
  same change — a unit that code points at is not informative. @status:impl/done

@fact:TIER-3-DEEP-JUDGMENT **Tier 3 — deep judgment (weekly/biweekly; the WISH
rules).** The rules no checker mechanises yet — reviewed by a human or strong
agent so they are not silently skipped, each a candidate to graduate into a real
checker. @status:impl/done

@fact:TIER-3-CONCRETE-LIST-LIVES-IN-THE-STACK The stack's GUIDE and cards carry the
concrete list (typed seams, cell isolation and oracles, uniformity,
contract-first ordering, lying prose, closed-vocabulary naming). @status:impl/done

@fact:TIER-3-MAY-LAUNCH-A-RAID Tier 3 may also launch a full
[raid](03-RAID-PLAYBOOK.md) when a Tier-1 backlog grows campaign-sized —
plan it per the [Campaign Form](05-CAMPAIGN-FORM.md). @status:impl/done

## 2. The collector contract {#collector}

@fact:collector-is-the-sweeps-instrument The health collector is the sweep's
instrument. @status:impl/done

@fact:collector-must-be-lead It MUST be: @status:impl/done

- @fact:COLLECTOR-DETERMINISTIC **deterministic**
  (same tree → byte-identical output, so its committed snapshot's diff IS the
  health trend), @status:impl/done
- @fact:COLLECTOR-ADVISORY **advisory** (it never fails the build — the gates do), @status:impl/done
- @fact:COLLECTOR-NO-LLM **no-LLM**, @status:impl/done
- @fact:COLLECTOR-SINGLE-SOURCED and **single-sourced** (it reads the gate policies — the conform
  and specmap configs — never hardcoded lists, so its numbers cannot drift from
  what the gates enforce). @status:impl/done

@fact:COUNT-THE-LIST-NOT-THE-RECORD Count the list, not the record: any inventory the
sweep acts on comes from the collector or a config, never from memory or a
document's stale snapshot. @status:impl/done

@fact:PROMOTION-LADDER **The promotion ladder** — how the rule set grows: a Tier-3
WISH rule becomes mechanisable → add it to the collector as a census (cheap,
advisory) → once proven, promote it to a blocking gate rule. @status:impl/done

@fact:EXTENDING-THE-COLLECTOR-IS-RATCHET-WORK Extending the collector is
itself ratchet work. @status:impl/done

## 3. Cadence {#cadence}

| Tier | Daily (or per active session) | Weekly |
|---|:---:|:---:|
| @fact:ROW-TIER-0 0 — hard floor @status:impl/done | ✓ @status:impl/done | ✓ @status:impl/done |
| @fact:ROW-TIER-1 1 — ratchet @status:impl/done | ✓ @status:impl/done | ✓ @status:impl/done |
| @fact:ROW-TIER-2 2 — drift @status:impl/done | | ✓ @status:impl/done |
| @fact:ROW-TIER-3 3 — deep judgment / raid @status:impl/done | | ✓ @status:impl/done |

@fact:DAILY-RUN-IS-LIGHT The daily run is light: floor green, then one or two
cheapest Tier-1 wins. @status:impl/done

@fact:WEEKLY-RUN-ADDS-DRIFT-AND-JUDGMENT The weekly run adds the drift and judgment
tiers. @status:impl/done

@fact:ANY-SINGLE-ITEM-IS-A-SAFE-STOP **Any single item is a safe
stop** — the sweep is incremental by construction, and the collector
re-derives the remaining work each run. @status:impl/done

## 4. Output of a sweep {#output}

@fact:SWEEP-LANDS-TOPIC-GROUPED-COMMITS Each sweep that changes the tree lands
topic-grouped commits (one logical unit each), citing the sweep item that drove
them. @status:impl/done

@fact:REFRESH-THE-HEALTH-SNAPSHOT Refresh the committed health snapshot in the same
run — its diff records the trend. @status:impl/done

@fact:resume-pointer-lead **Resume pointer** (see [06-WAL-CONVENTION](06-WAL-CONVENTION.md)): @status:impl/done
- @fact:SWEEP-RESUME-WITH-A-WAL *With a WAL (recommended):* bump the WAL's standing line at any sweep that
  moves a milestone (a gate widened, a backlog unit cleared); the health
  snapshot plus the WAL is the next sweep's starting point. @status:impl/done
- @fact:SWEEP-RESUME-WITHOUT-A-WAL *Without a WAL:* the sweep's closing commit message carries the summary —
  floor state, the Tier-1 items taken, the next candidate — and the committed
  health snapshot is the resume pointer. Nothing may live only in the
  operator's head or the session transcript. @status:impl/done

## 5. What the sweep deliberately does NOT do {#non-goals}

- @fact:NON-GOAL-DOES-NOT-REPLACE-THE-GATES It does **not** replace the gates (Tier 0 is the floor, the sweep hardens
  what sits above it). @status:impl/done
- @fact:NON-GOAL-DOES-NOT-AUTO-FIX It does **not** auto-fix. The collector gathers facts; the operator acts.
  Mechanising a fix is a separate, deliberate promotion (a new gate rule or
  a codemod). @status:impl/done
- @fact:NON-GOAL-DOES-NOT-TOUCH-FROZEN-SURFACES It does **not** touch owner-frozen surfaces or owner-court decisions
  without sanction; drift found in them is *filed* as debt, not fixed. @status:impl/done
- @fact:NON-GOAL-DOES-NOT-MEASURE-EFFECTIVENESS It does **not** measure effectiveness (deferred by design); it records
  objective state. @status:impl/done

## 6. Instantiating for a project {#instantiate}

@fact:CONSUMER-RUNS-THE-SHIPPED-SKILL A consumer project runs the sweep from the
shipped skill (the Rust stack ships `rust-ai-native-sweep`) against the shipped
tools; the only per-project material is the policy files the gates already
require and, optionally, a thin local instance document recording project
snapshot numbers and machine-scoped quirks. @status:impl/done

@fact:KEEP-THE-THREE-LAYERS-APART Keep the three layers apart: **method** (this
document), **language idioms** (the stack's GUIDE/cards), **project instance**
(the consumer's own notes). @status:impl/done

@fact:MACHINE-QUIRK-IS-NOT-PROJECT-FACT A machine quirk is not project fact; a
project number is not method. @status:impl/done
