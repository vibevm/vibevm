# Study note — the five articles (grouped T3) {#root}

_T3 grouped note (boss-authored) for INVENTORY S18 (Zhang blog —
detailed facts folded into `rlm-study.md` per D-R7), S19 (Prime
Intellect), S20 (the Cognition pair), S21 (Anthropic), S22
(minRLM). First-pass survey delegated to GLM-5.2 over the local
snapshots (access 2026-07-10); boss reviewed against the wave
records — numbers consistent. Facts and decisions only._

## S18 — Zhang's RLM blog (the origin post) {#zhang-blog}

Adds to the paper: root sees only query + context SIZE; observed
strategy ladder "peek → grep → partition-map → recurse"; depth-1
suffices in all blog experiments; on OOLONG-132k RLM(GPT-5-mini)
beats plain GPT-5 by >34 points at roughly equal cost (cheaper per
query on average); BrowseComp-Plus at 1 000 docs: RLM(GPT-5) holds
~perfect while the no-recursion ablation drops ~10pp; LoCoDiff:
GPT-5 <10% on >75k-token histories, RLM one-shots them by
*replaying diffs in code* (semantic work → programmatic work); the
stance "RLMs are not agents — LMs should decide how to break down
a problem". **Take:** the strategy ladder is the worked
decomposition example our delegate surface should ship (anchor
note decision 6); the diff-replay trick generalizes — prefer
deterministic computation over LM reading wherever the context is
machine-regenerable.

## S19 — Prime Intellect: "RLM, the paradigm of 2026" {#pi}

The trainer's view: RLM is the simplest form of context folding
and the right scaffold to RL end-to-end; "never actually
summarizes context" (delegation over lossy summarization);
efficient attention AND folding are both needed. Their verifiers
implementation choices worth keeping: **tools only for sub-LLMs,
never the root** (verbose tool output cannot clog the root);
`llm_batch` parallel fan-out; per-REPL-call timeout 120 s; root-
visible REPL output capped 8 192 chars/turn; answer returned only
via a mutable `answer = {content, ready}` variable (iterative
refinement until ready). Measured: GLM 4.6 ~doubles on DeepDive
under RLM but **collapses to ~half baseline when given sub-LLM
"tips"** (it stops delegating — prompt-shaped guidance killed the
mechanism); stays nonzero on Oolong-real to ~300–400k tokens where
the base scores zero. **Take:** (a) tools-to-workers-only is a
clean default for boss surfaces; (b) the GLM-tips regression is
field evidence for our GLM-C2 finding that guardrail text can
invert behavior — surface wording is a measured variable, test it
(MT-C2-05's fatigue facts generalize); (c) mutable-answer-until-
ready maps to draft-result files a worker updates before final
status.

## S20 — Cognition: the counterpoint arc (2025 → 2026) {#cognition}

2025: rule out architectures that violate two principles — share
FULL traces (not messages: the Flappy-Bird example — subagents
each saw the task, still built a Mario background + an off-brand
bird), and actions carry implicit decisions, so parallel writers
produce conflicting implicit decisions; default to one
single-threaded agent + a trained compressor for overflow; CC
subagents cited as the safe form (read-only, well-defined
questions). 2026 concessions, all "writes single-threaded, agents
add intelligence": (1) generator–verifier review — Devin Review
catches ~2 bugs/PR, ~58% severe, and works BEST when reviewer
shares NO context with the coder (clean context beats shared
context for verification — deliberate context starvation as a
feature); (2) "Smart Friend" — a fast sub-frontier primary calls
a smarter model as a tool, forking ~80/20 of its context, asking
broad questions; works once the primary is strong enough
(SWE-1.5→1.6 threshold), runs cross-frontier in production as a
**capability router, not a difficulty escalator**; (3) manager
Devin spawning child Devins over an internal MCP —
"map-reduce-and-manage; unstructured swarms are a distraction".
Open problems are communication problems: when to escalate, how
to surface discoveries, how to transfer context without drowning
the receiver. **Take:** (a) single-writer law enters the fabric's
packet semantics (parallel workers never co-write one artifact;
merge is a designated node — cf. ROMA aggregator); (b) the
verifier-with-clean-context pattern is our acceptance-check shape
(PP-002): verification packets deliberately get NO parent
context; (c) "Smart Friend" IS VISION §V4's advisor, with the
measured caveat that the caller must clear a capability bar to
benefit — a delegation-rules row (advisor_enabled requires
caller_class ≥ medium), converging with minRLM's nano result;
(d) their open problems are literally V3/V4's design questions —
Stage B should quote them as demand evidence.

## S21 — Anthropic: multi-agent research system {#anthropic}

Production orchestrator-worker: lead plans (plan persisted to
external Memory BEFORE the 200k truncation point), spawns 3–5
subagents with explicit objective / output format / tool guidance
/ boundaries; subagents parallelize their own tool calls (3+);
artifacts go to the FILESYSTEM with lightweight references
returned (their words: cuts the game of telephone — "the essence
of search is compression"); a CitationAgent post-pass; rainbow
deploys; resume-from-checkpoint on failure. Numbers: +90.2% over
single-agent Opus on their research eval; token usage alone
explains 80% of performance variance (tool-call count and model
choice next); agents ≈ 4× chat tokens, multi-agent ≈ 15×; up to
90% research-time cut from parallelism; a tool-testing agent
rewriting tool descriptions cut task time 40%; effort-scaling
rules (simple fact-find = 1 agent / 3–10 calls; complex = >10
subagents); early failure mode: 50 subagents for trivial queries.
**Take:** (a) files-as-IPC + reference-passing is exactly I2 +
FileRef — production-validated at frontier scale; (b) the
delegation brief quartet (objective/format/tools/boundaries) is
the packet template checklist; (c) effort-scaling rules belong in
delegation-rules as data (query class → swarm size); (d)
plan-persistence-before-truncation = our WAL discipline applied
to the boss's own plan mid-flight; (e) token-spend-as-quality-
driver reframes budgets: quotas are quality knobs, not just cost
caps.

## S22 — minRLM: practical guide + benchmark {#minrlm}

Independent 12-task benchmark (50 runs/task, 6 600 evals): on
GPT-5-mini 72.7% at 8 151 tokens/$2.86 vs vanilla 69.5% at 20 967
/$4.74 vs official RLM 69.7% at 29 327/$7.92 (2.6×/3.6× fewer
tokens, 25.8 s vs 60.9 s); **the advantage GROWS with model
capability** (GPT-5.4-mini +22.3pp; GPT-5.2 +30pp, 11/12 tasks,
AIME 96% vs vanilla 0%) and **inverts on GPT-5-nano (−9.5pp — the
small model cannot write correct REPL code)**; RepoQA is the
consistent loss (62% vs 98% — it generates code instead of
extracting); skip-RLM heuristics: short contexts (<8k) and pure
code-retrieval. Implementation ideas: zlib **entropy map** (20
sections × 500 chars flags where information lives) + head/mid/
tail preview instead of any raw context; step-0 task-type routing
before code generation; deterministic regex for counting (OOLONG
92%); `# REASONING:` comment before code; Docker seccomp
stdlib-only sandbox; side-by-side Gradio visualizer. Their
verbatim scaling law: "The REPL is less useful on smaller models
and more useful on stronger ones." **Take:** (a) capability-gate
the scaffold — REPL-shaped descent only above a model-class bar
(with THREAD's spawn-shape as the small-model alternative; both
now in delegation-rules); (b) the entropy-map + preview is a
cheap, deterministic context-triage pass a boss verb could run
before choosing descent strategy; (c) route-by-task-type-first
echoes ROMA's Atomizer at the cheap end; (d) "don't wrap short
contexts" joins the SRLM in-window guard as a hard rule.

## Cross-cutting (survey's synthesis, boss-endorsed) {#cross}

Single writer, many readers; token spend is the lever (not agent
count); clean context is a feature for verifiers; capability-gate
the scaffold; communicate via artifacts, not relayed prose; the
decomposition/folding policy is the thing to train next. All six
converge with the papers' evidence and land as RD-deltas in the
synthesis.
