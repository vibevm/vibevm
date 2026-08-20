# Study note — RLM runners-up (grouped T3) {#root}

_T3 grouped note (boss-authored, survey depth per D-R5) for
INVENTORY S23 (THREAD), S24 (Think-But-Don't-Overthink), S25
(claude_code_RLM, README-level), S26 (tinyagents, README-level).
Sources: local PDFs (S23/S24), web READMEs fetched 2026-07-11
(S25/S26). Each verdict one paragraph; facts and decisions only._

## S23 — THREAD (arXiv 2405.17402, NAACL 2025) {#thread}

Generation as a **thread of execution** that can run to completion
or dynamically spawn child threads; a child conditions on context
derived from the parent's tokens, does work (thinking, retrieval,
acting) on the parent's behalf, and **returns only the tokens the
parent needs** (Thread.join analogy). Few-shot, no training. SOTA
at the time on ALFWorld/TextCraft/WebShop + two QA benchmarks;
the load-bearing fact for us: **+10–50 absolute points with SMALL
models** (Llama-3-8b, CodeLlama-7b). Read against minRLM's
"GPT-5-nano −9.5pp": *REPL-driving* recursion needs a strong coder
root, but *spawn-shaped* recursion with token-filtered returns is
precisely what rescues small models. **Decision:** the fabric's
descent for GLM-tier roots should be spawn-shaped (packets), not
REPL-shaped — which is what fractality is anyway; record the
filtered-return contract as the reason child transcripts never
reach parents. Verdict: subsumed mechanically by the anchor +
fast-rlm, kept as the peer-reviewed citation for filtered returns
and small-model gains.

## S24 — Think, But Don't Overthink (arXiv 2603.02615) {#overthink}

Independent reproduction (CUHK; DeepSeek v3.2 + Kimi K2 on S-NIAH
+ OOLONG subsets, N=20/condition). The counterpoint numbers the
field needed: OOLONG — DeepSeek 0.0% → 42.1% at depth-1 (the
paradigm works) but 33.7% at depth-2; **Kimi K2 native 86.6% →
60.0% wrapped at depth-1 → 55.0% at depth-2** (wrapping a natively
strong long-context model HARMS it); S-NIAH latency 3.6 s → 89.3 s
(d1) → 344.5 s (d2), Kimi peaking 545.5 s/query; token usage
sometimes *plateaus* at depth-2 — a crash-early signature (format
collapse, isolated recursive loops), not efficiency. Failure modes
documented: parametric hallucination (context-anchoring loss —
the model answers from priors, ignoring the variable), format
collapse, redundant sub-call loops. Their viability verdict:
industrial deployment is premature; simple tasks and
natively-capable models are actively hurt. **Decisions:** (a) the
delegation-rules need-gate gains two hard rows — `task is O(1)
retrieval → no RLM machinery` and `model has proven native
long-context strength → send whole, don't wrap`; (b) depth-2
stays behind an experimental flag with per-run wall-clock caps
(their 545 s/query is what MC quotas exist to kill); (c) token-
plateau-with-time-explosion becomes a journaled anomaly signature
(I3) for stall detection. Verdict: T3 paragraph mandatory —
delivered; the strongest single counterweight in the corpus.

## S25 — brainqub3/claude_code_RLM (MIT, 393★) {#cc-rlm}

RLM mapped onto **our exact substrate**: the root Claude Code
conversation never loads the context (it lives in a persistent
Python REPL as a variable); sub-LM calls are **nested headless
`claude -p` instances with tools disabled**; deeper recursion =
nested CC **with bash + the RLM skill enabled**; driven by a
slash command + skill; emits audit-replay packages per REPL step.
Self-described "basic, not for production"; no benchmarks. The
transferable idea is the **capability ladder per depth**: the
same binary becomes leaf/mid/root purely by what surfaces it is
granted (tools off / bash+skill on) — ReDel's tool-removal and
VISION §V1 promotion expressed in Claude Code's own vocabulary,
proving the promotion mechanism needs zero new process machinery
on CC. Also a second confirmation that audit-replay artifacts are
the expected observability shape. Verdict: no deep study needed;
the pattern is recorded and already ours.

## S26 — tinyhumansai/tinyagents (GPL-3.0, Rust, crates v1.5) {#tinyagents}

A typed, durable **Rust** RLM harness: runs carry
root_run_id/parent_run_id lineage; a configurable recursion limit
bounds tree depth; **children's events, usage, and cost roll up
to parents as first-class observable runs**; sub-agents are tools
exposed to parents; durable checkpoints + replay; provider-
neutral (OpenAI-compatible default, Ollama). A model can emit a
`.rag` blueprint that compiles through the same registry path as
human-authored files and runs on the same runtime
(self-authoring). Effectively a single-process miniature of
fractality-core's run-tree — independent convergent evolution on
our exact shape (ids, rollup, caps, replay), in our language.
**Decision:** treat as a design-space confirmation only — GPL-3.0
plus clean-room law means nothing is ever taken from it but
courage; its existence supports the run-tree-with-rollup schema
as the industry-normal answer. The self-authoring blueprint idea
(agents emitting compilable workflow files) is noted for the far
horizon, unadopted.
