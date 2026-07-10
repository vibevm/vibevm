# Study note — SRLM: Self-Reflective Program Search for Long Context {#root}

_T2 note (boss-authored) for INVENTORY S16 — arXiv 2603.15653
(Alizadeh, Shojaee, Cho, Farajtabar — Apple). Read 2026-07-11 from
local text. Decisions and facts only._

## What it is {#what}

The strongest successor-critique of the anchor. Keeps the RLM
posture (context externalized as a variable in a programming
environment; the model writes programs that query/slice it) but
replaces **recursion** with **trajectory selection under
uncertainty**: candidate context-interaction programs are compared
via three intrinsic signals — self-consistency, reasoning-trace
length, verbalized confidence — and the model self-reflectively
picks/refines the winner.

- **Findings (their words condensed):** recursion is NOT the
  primary driver of RLM performance; self-reflective program
  search matches or beats RLM **without any self-query/recursion**,
  up to +22% over RLM at the same wall-clock budget. For contexts
  *within* the native window, RLM-style recursion often *degrades*
  performance vs the base model, while SRLM stays robust across
  short and long contexts. On semantically intensive tasks,
  heuristic program search is insufficient — the uncertainty
  signals supply the semantic steering RLM lacks.
- Reframing: recursion is one component of long-context reasoning,
  not its defining feature; *how candidate interaction programs
  are selected* is the under-explored bottleneck.

## Decisions we take {#decisions}

1. **Adopt the reframe wholesale:** what fractality takes from the
   RLM wave is (a) context-as-external-object and (b) programmatic
   interaction with it — NOT depth for its own sake. This is now
   the second independent source (with the anchor's own depth-0
   results and 2603.02615's overthinking data) that descent
   machinery must be *gated*, not default.
2. **In-window guard becomes policy:** if the task fits the
   worker's native window with margin, do NOT decompose — send it
   whole. A concrete delegation-rules row: `context_fits_window →
   no descent`. (Cheapest rule in the whole research, strong
   evidence.)
3. **Uncertainty signals are delegation telemetry:** self-
   consistency across N cheap probes, trace length, verbalized
   confidence — all measurable on GLM workers today. They are
   candidate inputs to (a) acceptance verdicts (PP-002), (b) the
   advisor trigger (V4: "confidence low → ask a bigger model"),
   and (c) escalation triggers (V3). The advisor channel gains a
   principled firing condition instead of vibes.
4. **Budget-matched comparison discipline:** their +22% is at
   equal wall-clock — when Stage B designs descent-vs-flat trials
   (MT-C3-xx), comparisons must be budget-matched or they are
   marketing. Carry this into the trial protocol.

**Non-adoptions:** the specific three-signal ensemble and its
weighting are their tuned artifact — we take the *category*
(intrinsic uncertainty as routing signal), not the recipe.
