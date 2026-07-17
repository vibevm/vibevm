# PROP-001 — the miniature reconciler {#root}

The pilot domain of the AI-Native Go discipline (GO-AI-NATIVE-PLAN
D11): a desired-vs-actual state reconciler — the Kubernetes-shaped
rehearsal, small enough to read in one sitting, rich enough to
exercise every scaffold class.

## Cells {#cells}
`req r1`

Cells MUST inject their capabilities (store, clock), MUST NOT touch
ambient defaults, and MUST be constructed through their `New`.

## The planner seam {#req-planner-seam}
`req r1`

A planner MUST compute, from a desired and an actual state, the action
list that transforms actual into desired. Two implementations exist
behind one seam (`naive`, `batch`); the registry selects by
configuration, and replacing one with the other MUST hold behavior
(the differential fuzz oracle is the gate).

## Plans are total {#req-plan-total}
`req r1`

Applying a plan to the actual state MUST yield exactly the desired
state: every missing resource created, every stale revision updated,
every extra resource deleted — nothing else touched.

## Plans are ordered {#req-plan-order}
`req r1`

A plan MUST be deterministically ordered (by resource id within each
operation class, creates → updates → deletes), so identical inputs
yield byte-identical plans — the codebase is the few-shot prompt, and
so is its output.

## Reconciliation converges {#req-converge}
`req r1`

Driving the world by repeated plan/apply steps MUST reach the desired
state in one step and remain there (a fixed point): the second plan
over a converged world MUST be empty.

## The failure set {#req-errors}
`req r1`

The planner's failure set is closed and enumerated (`PlanError`:
`ErrNilState`). Error messages MUST cite the violated REQ URI and a
fix surface (Class F).
