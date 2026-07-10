# Study note — ROMA (Recursive Open Meta-Agents) {#root}

_T2 note (boss-authored) for INVENTORY S10 — repo pin `a6e3bb4`
(Apache-2.0, sentient-agi, ~5.1k★, ~63k LOC under src/, DSPy-
based). First-pass survey delegated to GLM-5.2 over a sandboxed
copy under the live-observation law; the survey's mechanism claims
are internally consistent and file-path-anchored (spot-checks on
this repo limited to structure — its scale exceeds line-level
verification budget; treat unusual specifics as survey-sourced).
Decisions and facts only._

## What it is {#what}

A production-scale recursive plan/execute meta-agent: a goal
becomes a `TaskNode` in a dependency DAG; an **Atomizer** (an LLM
judgment: `is_atomic` + PLAN|EXECUTE) decides at every node
whether to decompose further or execute; a Planner emits typed
subtasks (RETRIEVE/WRITE/THINK/CODE_INTERPRET/…) with dependency
edges; executors run leaves; an **Aggregator** synthesizes child
results into the parent's answer. Five thin agent modules over
DSPy signatures; orchestration in a runtime + event-driven
scheduler (priority queue, max_concurrency, dependency waves via
asyncio.gather).

## Facts that matter to us {#facts}

- **The Atomizer IS a need-computation gate:** the decompose-or-
  execute decision is an explicit, isolated model call with a
  typed verdict — not an emergent side effect of prompting. This
  is VISION §V1's "необходимость вычисляется" as running
  architecture, at 5k-star scale.
- **Third depth-boundary strategy:** at `max_depth` (default 2!)
  a node is **force-executed** — `should_force_execute()` flips it
  to EXECUTE and bypasses Atomizer/Planner. Compare: recursive-llm
  soft-refuses, ReDel removes the tool, ROMA forces the leaf to
  just do the work. Note the tiny default depth in a system this
  size.
- **Three-directional context flow is explicit and typed:** down —
  children inherit execution_id/max_depth/parent_id; lateral — a
  ContextManager injects `DependencyResult(goal, output)` from
  completed siblings into executors (planners additionally get
  ParentResult + SiblingResult); up — the Aggregator receives
  `List[SubTask]` (goal, type, result, context_input) and produces
  a `synthesized_result` that **answers the parent's goal, not a
  concatenation**.
- **Resilience is per-task-type policy data:** retry policies
  differ by TaskType (RETRIEVE exponential backoff, WRITE linear,
  CODE_INTERPRET fixed, IMAGE_GENERATION no-retry), with jitter and
  max_delay; module calls wrapped in circuit breakers
  (open/half-open/closed per module); max_retries then
  mark_failed — and the parent subgraph STILL aggregates over
  surviving children (a dead child contributes nothing rather than
  killing the tree).
- **Metrics roll up the tree:** per-module TokenMetrics (tokens,
  cost_usd, model, latency, full messages) in each node's
  execution_history; recursive tree aggregation
  (`get_tree_metrics`); optional Postgres lm_trace rows + MLflow
  spans; event emission is config-filtered with a sample_rate.
- **Checkpoint/replay exists:** DAG snapshots at named moments
  (start / after planning / before aggregation / on failure /
  periodic) with restore and auto-recover.
- **Named gap:** no global token/cost ceiling — rich accounting,
  no enforcement (same hole as everywhere; MC-level quotas remain
  our differentiator).
- No task-level wall-clock timeouts (deferred to HTTP layer).

## Decisions we take {#decisions}

1. **Make the need-gate a first-class, isolated decision point**
   (Atomizer pattern): fractality's promotion/descend decision
   should be one auditable call with a typed verdict (+ recorded
   reason), not prose buried in a boss prompt. Its policy inputs
   come from delegation-rules; its outcomes are journaled (I3) —
   which also makes it trainable later (RAO note, decision 1).
2. **Force-execute at the depth floor** is the right default for
   *work* packets (do it yourself rather than fail), composing
   with ReDel's capability-removal (can't spawn) and
   recursive-llm's soft-refusal (for *spawn requests*): three
   boundary behaviors, each for a different verb. Stage B should
   pick per-verb, not one-size.
3. **Aggregation answers the parent's goal** — packet trees need
   an explicit aggregation contract (who synthesizes, against
   what goal) rather than assuming the boss re-reads all child
   results; this is the 2506.16411 aggregator-noise term made
   concrete.
4. **Per-task-type retry/backoff as policy data** slots directly
   into profiles/delegation-rules (a RETRIEVE-ish worker retries
   differently than a CODE one); circuit-breaker-per-module maps
   to per-backend breakers in MC.
5. **Partial-failure law:** a subtree with dead children still
   aggregates over survivors, with the gap visible — adopt as
   packet-tree semantics (status: completed_with_failures).
6. **Sample-rate on event categories** is the pragmatic answer to
   journal volume at swarm scale — a config knob to remember when
   I3 grows.

**Non-adoptions:** DSPy as substrate (we are process-level, not
in-process modules); depth default 2 is *their* number — ours
comes from our own trials; no global-budget gap to copy (we keep
MC-enforced quotas).
