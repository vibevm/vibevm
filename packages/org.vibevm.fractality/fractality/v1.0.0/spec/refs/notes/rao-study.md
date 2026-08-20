# Study note — Recursive Agent Optimization (RAO) {#root}

_T2 note (boss-authored) for INVENTORY S17 — arXiv 2605.06639
(Gandhi, Chakraborty, Wang, Kumar, Neubig — CMU/Amazon AGI Labs).
Read 2026-07-11 from local text. Decisions and facts only._

## What it is {#what}

RL for the **delegation policy itself**: one LLM policy is
instantiated at every node of a dynamically generated execution
tree; the same weights learn to (a) solve assigned tasks, (b)
decide WHEN to delegate, (c) write the delegated sub-task specs,
(d) communicate across levels, (e) aggregate children's outputs.
Inference surface: agents call `launch_subagent(goal=…)` —
notably inside `asyncio.gather(...)`, i.e. **parallel sibling
spawns are the native idiom** — plus `finish(...)` to return.

- **Claimed/illustrated benefits:** fresh context per child
  (expanded effective working memory); divide-and-conquer
  generalization — trained agents solve tasks *harder and longer*
  than training tasks by recursing (up to 10 levels deep); up to
  2.5× wall-clock reduction from parallel decomposition; better
  *training* efficiency — recursion generates related subproblems
  at graded difficulty (an implicit curriculum), so the recursive
  structure helps learning, not just inference.
- Their premise matches our vision verbatim: existing systems
  bolt recursion on as an untrained scaffold; "if recursive
  execution is going to be a core test-time primitive, the policy
  should be trained to use it well."

## Decisions we take {#decisions}

1. **The need-computation (VISION V1) is, long-term, a learned
   object.** v1 ships it as delegation-rules data (heuristics +
   thresholds); RAO is the named path for v2+: fractality's
   journal (I3) already records exactly the tuples RAO trains on
   (task, delegate-or-not, sub-task spec, outcome, cost). Design
   the journal fields so a future training extract is a query,
   not a migration.
2. **Sub-task spec quality is a trained skill, hence a reviewed
   artifact today:** since the field's evidence says spec-writing
   is the hard half of delegation, our packet-authoring guidance
   (goal, output contract, boundaries) stays boss-side and
   review-worthy; delegating spec-writing to small models is
   against the evidence.
3. **Parallel-first sibling semantics:** their native
   asyncio.gather idiom + 2.5× wall-clock validates MC's
   await-any/all horizon (PROP-001 §7) as the default shape of
   descent, not an optimization.
4. **Depth is not the scary part when trained** (10 levels) — but
   we are UNtrained: v1 keeps hard shallow caps (anchor's and
   2603.02615's evidence), while recording per-level outcomes so
   the cap can be revisited with data (decision-record trigger:
   measured per-level success rates).
5. **Fresh-context-per-child** is already our worker model (clean
   env, scratch home) — keep it invariant under any future
   promotion mechanics (a promoted sub-boss gets a fresh window,
   never the parent's transcript; matches Context-Folding's fold
   law from the other direction).

**Non-adoptions:** no RL training in v0.x; TEXTCRAFT-SYNTH-style
synthetic curricula are out of scope until the fabric itself is
proven.
