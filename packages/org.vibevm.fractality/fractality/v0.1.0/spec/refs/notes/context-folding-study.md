# Study note — Scaling Long-Horizon LLM Agent via Context-Folding {#root}

_T2 note (boss-authored) for INVENTORY S15 — arXiv 2510.11967
(Sun, Lu, Ling, Liu, Yao, Yang, Chen — ByteDance Seed/CMU/
Stanford). Read 2026-07-11 from local text. The official repo
(sunnweiwei/FoldAgent) was NOT intaken — license unverified; this
note is paper-only. Decisions and facts only._

## What it is {#what}

Context descent **without spawning**: one agent, one policy, two
context-management actions. `branch(description, prompt)` opens a
sub-trajectory with the same window to work a localized subtask;
`return(message)` folds the branch — all its action/observation
pairs are REMOVED from the working context, replaced by the
templated return message. The main thread's context stays
undisrupted; token-heavy work (search dumps, codebase exploration)
happens inside branches and leaves only findings.

- **Numbers:** with a 32K active-token budget and ≤10 branches,
  the folding agent hits 62.0% pass@1 on BrowseComp-Plus and 58.0%
  on SWE-Bench Verified — matching/beating ReAct baselines that
  carry a 327K window (~10× smaller active context), and clearly
  beating summarization-based management.
- **FoldGRPO:** end-to-end RL that makes folding learnable — token-
  level process rewards: an **Unfolded Token Penalty** (discourages
  token-heavy operations in the main thread) and an **Out-of-Scope
  Penalty** (keeps a branch on its subtask). RL adds +20.0pp on
  BrowseComp-Plus and +8.8pp on SWE over the untrained scaffold.
- Framing vs alternatives: summarization compresses *after* the
  fact and disrupts reasoning flow; multi-agent splits are
  hand-crafted workflows; folding is the agent's own, learnable,
  in-thread decomposition.

## Decisions we take {#decisions}

1. **Fold-at-subtask-boundaries becomes the boss-session hygiene
   model:** a boss (or promoted sub-boss) should treat each
   delegated packet as a *branch* whose transcript never enters
   the parent context — only the return message (our result.md /
   status.json) does. This is the same discipline as the anchor's
   metadata-only history, stated for the *agent trajectory* plane.
2. **The return message is a first-class contract:** what survives
   the fold is only as good as `return(message)` — packet output
   contracts should mandate "goal-answering summary + pointers",
   which is exactly PROP-001's "workers return compact files".
3. **Branch-shaped work without processes is a real rung:** some
   descent doesn't need a worker spawn (cost, latency); a future
   boss-side verb could open a *local* fold (V5 attachable-terminal
   sub-session or a sub-CC `-p` call) for mid-sized subtasks. The
   need-computation (V1) should therefore choose among three, not
   two, options: inline / fold locally / spawn a pod.
4. **Process rewards name our telemetry:** "unfolded tokens in the
   main thread" and "out-of-scope actions in a branch" are
   *measurable* fabric metrics today (I3 store) — worth recording
   per run long before any RL, as delegation-quality facts (feeds
   PP-002-style credibility surfaces and any future training).
5. **Budget framing:** their 32K-active + ≤10-branches shape maps
   to per-run quota semantics MC already plans (wall/turns/tokens)
   — add "max concurrent/total branches" as a packet budget field
   candidate for Stage B.

**Non-adoptions:** FoldGRPO training is out of scope (we are not
training models in v0.x); the single-window branch (same model,
same window) is *their* constraint — our folds may cross process
and model boundaries.
