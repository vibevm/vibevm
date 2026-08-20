# Study note — Learning to Orchestrate (the Conductor) {#root}

_T2 note (boss-authored) for INVENTORY S29 — arXiv 2512.04388
(read at **v5**, 2026-05-06; ICLR 2026; Nielsen, Cetin,
Schwendeman, Sun, Xu, Tang) + sakana.ai/learning-to-orchestrate
snapshot. Fugu-Ultra's direct ancestor. Facts and decisions._

## What it is {#what}

A 7B **reasoning model RL-trained (GRPO) to emit whole agentic
workflows in natural language**: the output is a plan — parallel
lists `model_id = [...]`, `subtasks = ["...", ...]`,
`access_list = [[...], ...]` — one step per worker call, each
step carrying a focused NL instruction, the assigned agent, and
exactly which previous steps' outputs that agent may see. The
grammar spans best-of-N, chains, and arbitrary parallelizable
trees. Rewards: format gate (−1 unparseable), correctness
(1 match / −0.5 otherwise); GRPO with KL penalty (Fugu-Ultra
later dropped the KL term). Prompt engineering, refinement, and
meta-prompt optimization **emerge** from reward maximization —
nobody hand-writes coordination strategies (hardmaru's framing:
they trained a model to do prompt engineering).

## Facts {#facts}

- A 7B conductor over strong workers reaches SOTA on LCB and
  GPQA-D, beating self-reflection with any single worker AND
  costlier multi-agent baselines using more calls.
- **Randomized agent pools during training** → the conductor
  generalizes to arbitrary open/closed pools at test time (users
  can swap pools without retraining — the property Fugu ships as
  configurability).
- **Self-as-worker → recursive topologies**: letting the
  conductor assign itself spawns nested conducting, "a new
  tunable axis of inference-time scaling" — learned recursion at
  the orchestration layer (their words for our V1).

## Decisions we take {#decisions}

1. **The workflow-step grammar is a packet-tree schema in
   miniature:** (instruction, assignee, access list) ≅ (packet
   goal, profile, context-refs). Stage B's packet extensions
   should make the access-list equivalent explicit — a
   `context_from: [run-ids]` field — rather than implicit
   parent-gives-everything (hardens D-C3-2).
2. **Pool randomization is profile discipline:** policies that
   must survive pool changes should be trained/authored against
   VARYING pools — delegation-rules should never hard-code a
   model name where a class/capability row will do (V4 ladder
   already says this; now with a training-side reason).
3. **Recursion at the orchestration layer is learnable and
   valuable** — the strongest external validation of VISION §V1's
   recursive-boss bet to date, from a shipped system's ancestor.
4. **Format-gate-then-quality reward** (unparseable = worst,
   wrong-but-well-formed = middle) is a useful shape for any
   future learned policy AND for today's structured-outcome
   validation ordering (schema check before content check —
   matches fast-rlm's Ajv-then-retry seam, RD-5).

**Non-adoptions:** NL-emitted workflows as our execution format
(our packets are TOML contracts, not prose — auditable and
diffable); training a conductor in v0.x (RD-20).
