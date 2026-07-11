# Study note — TRINITY: An Evolved LLM Coordinator {#root}

_T2 note (boss-authored) for INVENTORY S28 — arXiv 2512.04695
(read at **v3**, 2026-04-27; ICLR 2026; Xu, Sun, Schwendeman,
Nielsen, Cetin, Tang — Sakana/UMich/ISCT) + sakana.ai/trinity
snapshot. Fugu-standard's direct ancestor. Facts and decisions._

## What it is {#what}

Test-time model composition via a coordinator so small it is
almost free: a 0.6B SLM whose **hidden state** (penultimate-token
representation over the full transcript) feeds a ~10K-param head;
**<20K trainable parameters total**. Per turn: select an LLM from
the pool AND assign one of three roles — **Thinker** (strategize/
decompose), **Worker** (execute a step), **Verifier** (judge
soundness/completeness). Halting: **the Verifier accepting the
current response ends the run**, or a fixed turn budget. Role
prompts are injected by a message-processing module; the child
sees query + full prior-turn transcript.

## Facts {#facts}

- **Why hidden states:** they contextualize the whole sequence
  cheaply; the coordinator needs understanding-for-routing, not
  generation — skill acquisition is offloaded to the pool.
- **Why evolution:** each parameter has tiny influence on a
  scalar reward (low SNR for per-parameter gradients), and every
  evaluation costs real inference across coordinated agents.
  **sep-CMA-ES** (diagonal covariance) beats RL, imitation, and
  random search in this regime (1.5k–40k evals for a ~10k-dim
  problem); the objective shows block-ε-separability.
- Results: mean relative error reduction 21.9% over second-best
  across Math500/MMLU/RLPR/LCB; **LCB pass@1 86.2%** SOTA at
  publication; beats all single models under fair token budgets;
  zero-shot transfer to AIME/BigCodeBench/MT-Bench/GPQA-D,
  surpassing every constituent.

## Decisions we take {#decisions}

1. **Thinker/Worker/Verifier is a role vocabulary for packets:**
   our packet types map cleanly (plan / work / acceptance); the
   verifier-accepts-⇒-halt loop is an acceptance-driven
   completion criterion — exactly the shape PP-002's
   acceptance-schema needs (a run tree completes when its
   acceptance packet passes, not when workers stop).
2. **The routing brain can be tiny:** 20K trained parameters
   suffice when the pool carries the skills. Fabric translation:
   the need-gate/routing policy (RD-1/RD-2) does not need a big
   model — a small evolved/tabular policy over good FEATURES.
   Near-term our features are journal facts; the SLM-hidden-state
   trick is the far-horizon upgrade.
3. **Verification as a routed ROLE, not a fixed stage:** the
   coordinator decides when verification is worth a turn and who
   verifies — strengthens RD-11 and Fugu's dynamic-aggregator
   lesson: acceptance packets should have a *chosen* profile too.
4. **ES over sparse end-to-end reward** is the named optimizer
   for policy-tuning delegation-rules from journal outcomes
   someday (RD-13/RD-20 lineage) — Sakana proved the regime twice
   (Trinity, then Fugu's stage 2).

**Non-adoptions:** the full-transcript-to-child context model
(their children see everything prior — our fold law RD-5 keeps
parents metadata-only; Fugu-Ultra itself moved to access lists);
0.6B-SLM plumbing in v0.x.
