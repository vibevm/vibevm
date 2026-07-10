# Study note — Recursive Language Models (the anchor project) {#root}

_T1 note (boss-authored, deep) for INVENTORY S3 (repo
`alexzhang13/rlm`, MIT, pin `72d6940`), S4 (arXiv 2512.24601, read
at **v3**, 2026-05-11), S18 (the author's blog post). One project,
one note (D-R7). Sources read 2026-07-10/11 from local copies;
clean-room: decisions and facts only, no code shapes carried.
Authors: Zhang, Kraska, Khattab (MIT CSAIL/OASYS)._

## What it is {#what}

An inference paradigm, not a model: the arbitrarily long prompt is
NOT fed to the transformer. It is loaded as a variable inside a
persistent Python REPL environment; the root LM sees only
constant-size *metadata* (length, short prefix, access hints) and
writes code that peeks into, slices, and transforms the variable —
and can invoke a sub-(R)LM **programmatically** (in loops, over
O(|P|) or O(|P|²) slices). Iteration appends only metadata about
stdout to the root's history; the loop ends when the model sets a
designated final variable. The system's external interface stays
"string in → string out" — an RLM *is* a language model built
around recursive LM calls (`rlm.completion` as drop-in for
`llm.completion`).

The paper names three load-bearing design choices, each the
negation of a common scaffold flaw: (1) the prompt must live as a
**symbolic handle** outside the window (not in `hist`); (2) the
final output must come from the environment (a variable), not be
verbalized through the model's bounded output; (3) recursion must
be **symbolic** — sub-calls constructed by running code, not
verbalized one at a time as tool-call turns.

## Verified facts that matter to us {#facts}

- **Headline results (GPT-5 rig: GPT-5 root + GPT-5-mini subs):**
  beats compaction by +26% median, CodeAct-with-sub-calls by +130%,
  Claude Code by +13% across four tasks; handles 6–11M-token
  BrowseComp-Plus at ~$0.99/run — cheaper than a linearly
  extrapolated single ingest ($1.50–2.75) that no window even fits.
  OOLONG-Pairs (quadratic complexity): base GPT-5 F1 ≈ 0.1% → RLM
  depth-1 58.0%, depth-2 76.0%.
- **Depth semantics:** depth-0 = REPL only (no sub-calls); depth-1
  = sub-LLM; depth-N>1 = sub-RLM. Depth 0/coding agents already
  scale past the window (the REPL is the enabler); **sub-calling
  pays only on information-dense tasks** (linear/quadratic). For
  GPT-5, deeper helps on quadratic tasks; for Qwen3-Coder, depth ≥ 2
  *hurts* — its frequent syntax errors propagate into sub-RLMs
  (error amplification down the tree).
- **Task-complexity lens:** effective context window is a function
  of task complexity, not just length — O(1) needle tasks survive
  1M+, linear OOLONG degrades far earlier, quadratic OOLONG-Pairs
  collapses. (Same lens as 2506.16411's noise regimes.)
- **First decomposition is fateful:** in-context decomposition
  examples in the system prompt — even from unrelated tasks —
  improve both the first decomposition attempt and final scores;
  models often but not always recover from a bad first cut.
- **Reasoning beyond long-context:** on LongCoT-mini, RLM(GPT-5.2)
  50.6% vs base 38.7%; with explicit decomposition *hints* 65.6%
  (+69.5% relative) — the model builds a dependency graph and
  solves nodes via sub-calls.
- **Training is cheap and transfers:** RLM-Qwen3-8B = rejection-SFT
  on 1 000 filtered trajectories distilled from Qwen3-Coder-480B
  (48 H100-hours): +28.3% median on unrelated tasks, >3× faster
  trajectories; RLVR on short MRCRv2 splits generalizes to 1M/8-
  needle. Key insight: leaf sub-calls are ordinary LM requests —
  **the trainable skill is being the root** (probe → decompose →
  recurse → aggregate). 16%/13% of raw trajectories misused the
  final-answer tags (FINAL vs FINAL_VAR) and needed programmatic
  patching.
- **Negative results (Appendix B, their words condensed):** one
  system prompt does not fit all models (Qwen needed a line to
  *curb* sub-calling); models without coding strength fail as RLM
  roots; thinking models exhaust per-call output budgets; blocking
  sub-calls make everything slow (async is future work); the
  FINAL()/FINAL_VAR() string-tag protocol is brittle (plans emitted
  as answers).
- **Limitations they name:** guardrails for RLMs unexplored;
  sub-call cost explosion is a real risk; async + sandboxed REPLs
  would cut cost/latency but add complexity.
- **Repo shape (pin `72d6940`, ~5.7k LOC core+clients+envs):**
  backend clients (openai/anthropic/gemini/azure/portkey), a core
  loop (`rlm/core/rlm.py`, handler, wire types), pluggable REPL
  environments (host-process exec by default; ipython, docker,
  modal, daytona, e2b, prime), examples for batched sub-queries,
  depth metadata, logging; a `training/` env on prime-rl. A
  separate `rlm-minimal` repo exists for pedagogy. The README's
  thesis, verbatim short: they want to "move away from the JSON
  tool-calling standard" — sub-agents as *functions in code*,
  prompts as *objects in code*.

## Decisions we take (→ synthesis deltas) {#decisions}

1. **Context descent = handle + carve, never copy-up.** The boss
   holds a symbolic handle to big context; workers get carved
   slices. fractality already has the exact primitive — **FileRef
   (scope, path, byte range)** — so RLM-style descent lands as
   packet fields, not a REPL rewrite. (VISION §V2 becomes concrete:
   "RLM descent" = FileRef slicing + sub-packet fan-out.)
2. **Root/sub model economics validated:** expensive root + cheap
   sub-calls (GPT-5 + GPT-5-mini) is the paper's own cost story —
   our boss/GLM-worker split is the same shape at process scale.
3. **Depth policy is data:** default depth 1; deeper only for
   provably super-linear tasks AND only under strong-coder roots
   (error amplification kills weak-model depth). Goes into
   delegation-rules as columns (max_depth by model class × task
   complexity class).
4. **Metadata-only parent history:** the parent loop must see
   constant-size summaries (length/prefix/status), with full
   payloads in the environment. Our packet results already return
   compact files + status; nudge/scoreboard surfaces must keep the
   same discipline (never inline a transcript).
5. **Structural finish, not string tags:** their FINAL/FINAL_VAR
   brittleness is an argument FOR our files+status.json outcome
   contract; never adopt string-sentinel protocols on any fabric
   surface.
6. **Ship one worked decomposition example** in the delegate skill
   / boss boot snippet — the cheapest measured lever on first-
   decomposition quality.
7. **Async fan-out is our home turf:** their blocking sub-calls are
   the main cost/latency drag; MC's parallel spawn + await-any/all
   (PROP-001 horizon) is the differentiator to keep.
8. **Per-spawn budget guardrails are unsolved upstream** — the
   paper leaves cost-explosion open; fast-rlm's depth/call/$ caps
   (S11) is where the field's answer lives; ours must be MC-level
   quotas (I-budgets), enforced outside the model.
9. **Training lever noted, deferred:** distill-the-root recipe is
   cheap (1k trajectories) and transfers; a future campaign could
   train a GLM-class root on fractality trajectories. Not v0.x.

**Non-adoptions (named):** we do not adopt a Python REPL as the
boss surface (our fabric is process/packet-based; the REPL insight
informs contracts, not architecture); we do not adopt in-process
`exec` sandboxing (their default env runs code in the host process
— our workers are OS processes with I1 env whitelists); no string
sentinels (see 5).

## Open questions carried to synthesis {#open}

- Where does the REPL live if we ever want literal RLM inside a
  worker? (Their sandbox spectrum — docker/e2b/daytona — maps to a
  pod-owned scratch env; V5 attachable terminals could BE that
  environment.)
- Is an "advisor call" (VISION V4) just RLM depth-1 with a
  bigger-model sub-call? Their framing says sub-calls go DOWN to
  cheaper models; our V4 goes UP — same plumbing, inverted
  economics; needs its own cost gate.
