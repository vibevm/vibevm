# Study note — OpenFugu (open reimplementation of Fugu) {#root}

_T2 note (boss-authored) for INVENTORY S30 — repo pin `7ad7ccf`
(Apache-2.0, trotsky1997, ~3.85k LOC Python). GLM survey over a
sandboxed copy; boss spot-checked verbatim (MAX_TURNS=5 +
agent_mask k-of-n in mini.py, visible_indices/MAX_STEPS=5 in
ultra.py, suppress_cold_verifier) — all held. Value: the mechanics
Sakana's proprietary API hides, reverse-engineered with
evidence-graded honesty tags. Facts and decisions._

## Facts the API hides {#facts}

- **TRINITY loop mechanics:** router = Qwen3-0.6B + bias-free
  linear head, two argmaxes per turn (agent_id, role_id); the
  backbone's generated text is DISCARDED — logits are the whole
  action. `MAX_TURNS=5`; Verifier `ACCEPT` terminates
  (`terminated_by="verifier_accept"` / `"max_turns"`). A Thinker
  turn emits a suggestion + `suggested_role` that OVERRIDES the
  head next turn; Thinker/Verifier turns do not update the router
  observation. `suppress_cold_verifier` reroutes a verifier picked
  before any work to Worker (no cold ACCEPTs). Workers never see
  raw transcripts — one evolving observation string of distilled
  thoughts.
- **Adaptive pool (`agent_mask`):** absent workers masked to −inf
  → the router picks the best AVAILABLE worker; pool size (7) is
  checkpoint-baked but slot labels are remappable metadata;
  workers are any callable (mock / litellm / local HF).
- **Conductor executor:** the three-list grammar parsed with
  balanced-bracket + literal_eval fallbacks; DAG executed in topo
  order; `visible_indices` REJECTS forward references; equal-list
  validation; **no verifier step in Ultra workflows — the last
  step's output is the answer by fiat** (quality gate exists only
  in the TRINITY loop).
- **Budgets:** turn/step caps only; **no runtime cost accounting**
  (a `w_cost=0` shaping term exists, disabled, in training only);
  the served response leaks orchestration depth as
  `usage.fugu_turns`.
- **Training reconstructions:** sep-CMA-ES over ~19.5K params (SVF
  offsets on 9 matrices + 10×1024 head), fitness = terminal reward
  of full rollouts; shaping bonuses `w_div=0.15`, `w_turn=0.10`;
  GRPO conductor (beta=0, format+action reward) published as a 3B
  checkpoint. **Honest negative: their two-round recursive-GRPO
  eval is a TIE** (round-1 revision ≠ better).
- Serving: stdlib HTTP, fresh Coordinator per request, pool hidden
  from the caller.

## Decisions we take {#decisions}

1. **suppress_cold_verifier is our cold-board lesson generalized:**
   never let an acceptance step fire before any work exists —
   packet-tree semantics should refuse acceptance packets on an
   empty tree (composes with F25 and RD-11).
2. **agent_mask availability routing** = profiles must declare
   availability and the need-gate must route over the AVAILABLE
   subset (a delegation-rules mechanic, and V4's
   "effective-top-of-ladder" fallback made concrete).
3. **Thinker's suggested_role override** is a cheap advisory
   channel INSIDE an orchestration loop — rhymes with our
   mid-work nudge: advice that steers the next routing decision
   without seizing control.
4. **The Ultra gap (no verifier in workflows, no cost runtime)**
   marks exactly where fractality's MC is ahead: enforcement and
   acceptance live in OUR runtime, not in the model's plan. Keep
   it that way (RD-4/RD-11).
5. **Depth-as-usage transparency** (`usage.fugu_turns`) is a nice
   consumer contract: our run results should surface tree
   depth/spawn counts in status metadata by default.
6. Their recursion TIE is a second data point (with 2603.02615)
   that **naive recursive self-revision does not pay** — recursion
   needs fresh context or a different specialist, not the same
   model re-reading itself (aligns with Fugu's clean-slate
   pattern and our RD-2 guards).

**Non-adoptions:** nothing structural — this is a study mirror;
Apache-2.0 but the clean-room law stands (no code carried).
