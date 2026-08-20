# RLM source selection — the Wave 3 merge verdict {#root}

_Ф2 deliverable of
[`FRACTALITY-RLM-RESEARCH-PLAN-v0.1`](../../plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md)
(§5 waves, D-R3 criteria). Merged 2026-07-10 ~23:05 from Wave 1
(deep-research harness: 103 agents, 24 claims unanimously verified
3-0 against primary sources; raw: `refs/study/rlm-waves/wave1-deep-research.md`)
and Wave 2 (12 frozen queries, independent; raw:
`refs/study/rlm-waves/wave2-web-search.md`). All URLs accessed
2026-07-10. Provenance membership rule (frozen at merge): an item
"belongs" to a wave iff its name appears anywhere in that wave's
raw record. **Overlap on the final 5/5/5: 9/15 = 60%** (repos 3/5,
papers 2/5, articles 4/5) — P-R2 verdict lands at close._

## The 5 repositories (recursion load-bearing) {#repos}

| # | repo | license | scale | prov | verdict rationale |
|---|---|---|---|---|---|
| R1 | [alexzhang13/rlm](https://github.com/alexzhang13/rlm) | MIT | 5 240★, pushed 2026-06-26 | both | The anchor's official reference: REPL-offloaded context, `rlm_query`/`rlm_query_batched` in-environment recursion, sandbox spectrum (local/ipython/docker/modal/prime/daytona/e2b), plus a training environment. The paradigm's source of truth (paper v3 authors). Already S3, pinned `72d6940`. |
| R2 | [sentient-agi/ROMA](https://github.com/sentient-agi/ROMA) | Apache-2.0 | ~5.1k★ | W1 | Recursive plan/execute meta-agent at production scale: the **Atomizer** decides "atomic or decompose further" — a working need-computation gate (VISION §V1's criteria as running code); three-directional information flow (down/lateral/up) maps onto descent/escalation. The largest non-anchor system where recursion is the core mechanism. |
| R3 | [avbiswas/fast-rlm](https://github.com/avbiswas/fast-rlm) | MIT | 447★, pushed 2026-07-07 | both | The engineering-complete RLM runtime: sub-agents return **symbols, not transcripts**, and the fabric's missing primitives are all present — hard depth caps, call caps, dollar/token budgets per spawn, structured IO, observability. The closest existing analog to what MC quotas + packets must express. |
| R4 | [zhudotexe/redel](https://github.com/zhudotexe/redel) | MIT + Commons Clause | 94★, EMNLP 2024 demo, pushed 2026-05 | W2 | The only toolkit whose recursion is about **agent delegation, not context**: agents decide when to delegate and how to organize (DelegateOne/DelegateWait schemes), with event-sourced logging and replay UI — plus a published failure taxonomy (overcomplication, premature termination). Direct idea feed for delegation-rules and pod observability. Commons Clause is irrelevant to us: study-only, code never adopted (D-R4). |
| R5 | [grishahq/recursive-llm](https://github.com/grishahq/recursive-llm) | MIT | 565★ | both | The minimal faithful reimplementation with **enforced max_depth** — the best small readable reference for the core loop, useful precisely because it is the mechanism with nothing else attached. |

**Runners-up (why not):** sunnweiwei/FoldAgent (W1; official
Context-Folding code — the *paper* P3 carries the idea; license
unverified); brainqub3/claude_code_RLM (W2; MIT, 393★ — RLM scaffold
on our exact substrate (Claude Code); loses R5's slot on mechanism
pedagogy, flagged for a T3 paragraph); tinyhumansai/tinyagents (W2;
GPL-3.0, 16★, **Rust** harness with run-tree/cost-rollup telemetry —
young and tiny, T3 paragraph for its telemetry shape);
RecursiveMAS/RecursiveMAS (W2; MIT, 878★ — latent-space recursion,
not process-level spawning: different mechanism class);
avilum/minrlm (W2; MIT, 72★ — its *article* carries the benchmark
value, selected as A5); hampton-io/RLM, fullstackwebdev/rlm_repl,
petroslamb/rlm, numb3r33/rlm, codecrack3/RLM-with-DSpy,
The-Swarm-Corporation/AdvancedResearch (reimplementations/appliers
without new mechanism ideas).

## The 5 papers {#papers}

| # | paper | venue/date | prov | verdict rationale |
|---|---|---|---|---|
| P1 | [arXiv 2512.24601](https://arxiv.org/abs/2512.24601) — Recursive Language Models (Zhang, Kraska, Khattab) | v1 2025-12, **v3 2026-05-11** | both | The anchor paradigm: context as REPL variable, programmatic carve + recursive self-calls; ~100× beyond window; verified deltas +26% median over compaction, +130% over CodeAct-with-subcalls, +13% over Claude Code on GPT-5 across four long-context tasks. Already S4 (local PDF is v1 — re-fetch v3 at Ф3). |
| P2 | [arXiv 2506.16411](https://arxiv.org/abs/2506.16411) — When Does Divide and Conquer Work for Long Context LLM? (Xu et al.) | ICLR 2026 (v2 2026-02-28) | W1 | The **theory** of descent: task noise vs model noise vs aggregator noise; Proposition 3.1 gives the formal condition when chunked descent beats a monolith; the **Silo Effect** regime (cross-chunk dependence dominates) proves some tasks must escalate to one high-context boss instead of fanning out — the mathematical backbone for V1 need-computation and V3 escalation triggers. |
| P3 | [arXiv 2510.11967](https://arxiv.org/abs/2510.11967) — Scaling Long-Horizon LLM Agent via Context-Folding (Sun et al., ByteDance/CMU) | 2025-10 | both | Context descent **inside one thread**: procedurally branch into a sub-trajectory, fold it back to a summary at the subtask boundary; ~10× smaller active context at equal-or-better quality. The fold-at-boundaries idea is directly portable to boss-session hygiene and packet result contracts, without spawning at all. |
| P4 | [arXiv 2603.15653](https://arxiv.org/abs/2603.15653) — SRLM (Apple) | 2026-03 | W1 | The strongest successor-critique: ablations show **recursion is not the primary driver — programmatic context interaction is**. Reframes what we adopt: the REPL-over-context posture matters more than depth. A constraint-delta generator (P-R4 material). |
| P5 | [arXiv 2605.06639](https://arxiv.org/abs/2605.06639) — Recursive Agent Optimization (Gandhi, Chakraborty, Wang, Kumar, Neubig) | 2026-05 | W2 | The **learned** side of delegation: RL trains a single model to decide when to spawn, what sub-task spec to hand down, serial vs parallel — the promotion/need-computation policy (VISION §V1, delegation-rules) as a trainable object. Field's first end-to-end answer to "when is descent worth it" from data, not heuristics. |

**Runners-up (why not):** THREAD
[arXiv 2405.17402](https://arxiv.org/abs/2405.17402) (W1; NAACL
2025 — peer-reviewed precursor of recursive spawning with
token-filtered child returns; subsumed mechanically by P1/R3, T3
paragraph for the filtered-return contract); Think-But-Don't-
Overthink [arXiv 2603.02615](https://arxiv.org/abs/2603.02615)
(both; the reproduction with the counterpoint numbers — depth-2
"overthinking" degrades accuracy at ~100× latency; Kimi K2 wrapped
drops 86.6%→60%; in-window recursion often below base — cited
heavily in W1's verified claims, T3 paragraph mandatory); ReDel
paper arXiv 2408.02248 (W2; the project already holds slot R4 —
one project, one note, D-R7); Chain of Agents 2406.02818, MemGPT
2310.08560, Demand Paging 2603.09023, RecursiveMAS 2604.25917,
Recursive Agent Harnesses 2606.13643, SearchSwarm 2606.09730, Deep
Research survey 2512.02038, ADAPT, A-MapReduce 2602.01331 (each one
angle, none adds a mechanism the five above don't cover better).

## The 5 articles {#articles}

| # | article | prov | verdict rationale |
|---|---|---|---|
| A1 | [Recursive Language Models — Alex L. Zhang's blog](https://alexzhang13.github.io/blog/2025/rlm/) | both | The paradigm's origin narrative with the practitioner details the paper compresses (rlm.completion as drop-in replacement); part of the anchor project (D-R7 → `rlm-study.md`). |
| A2 | [Prime Intellect — RLM: the paradigm of 2026](https://www.primeintellect.ai/blog/rlm) | both | The ecosystem view: what changed after the paper, ablations of RLM scaffolding for long-context agent work; the best single "state of the wave mid-2026" read. |
| A3 | [Cognition — Don't Build Multi-Agents](https://cognition.com/blog/dont-build-multi-agents) + [Multi-Agents: What's Actually Working (2026)](https://cognition.com/blog/multi-agents-working) | both | **The mandated counterpoint, as one argumentative arc:** context dispersal / game-of-telephone; writes stay single-threaded — then the 2026 follow-up naming which setups survived contact. The constraint set any recursive fabric must answer (P-R4). |
| A4 | [Anthropic — How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | both | Production orchestrator-worker at frontier scale: 90.2% over single-agent, 3-5 parallel subagents, and the task-boundary discipline (objective, output format, tool guidance, boundaries per subagent) — the operational playbook our packets already gesture at. |
| A5 | [minRLM — practical guide & benchmark](https://avilum.github.io/minrlm/recursive-language-model.html) (+ [write-up](https://infosecwriteups.com/minrlm-a-token-efficient-recursive-language-model-implementation-and-benchmark-bdc6840a3b00)) | W2 | The independent practitioner benchmark: 12 tasks, 4 models, 6 600 evals — 72.7% at 3.6× fewer tokens on GPT-5-mini; +30pp on GPT-5.2; and the sizing fact that matters to us: **GPT-5-nano loses 9.5pp — small models cannot drive the REPL** (a hard datum for GLM-tier worker/boss sizing). |

**Runners-up (why not):** fast-rlm docs site (W1-verified but
project-doc, folded into R3's note); LangChain "How and when to
build multi-agent systems" (both; decision framework — T3
paragraph); TDS deep-dive ×2, ArXivIQ review, RLM-in-ADK (Connell),
TechTalks, MarkTechPost ROMA, priyank766 commodity-hw reproduction,
Daytona RLM guide, Introl, jxnl (digests/derivatives of sources
already selected).

## Wave-disagreement notes (for P-R2's verdict at close) {#overlap}

- W1 alone surfaced: ROMA, FoldAgent, the ICLR theory paper
  (2506.16411), SRLM, THREAD — the *mechanism landscape*.
- W2 alone surfaced: ReDel, RAO, minRLM, claude_code_RLM,
  tinyagents, and the whole practitioner-counterpoint shelf
  (Cognition/Anthropic/LangChain full texts) — the *engineering
  and discourse shelf*.
- Verdict math: 9/15 both-wave = exactly 60%. Neither wave alone
  would have produced this cut: W1 missed 3 of the final 15, W2
  missed 3 others (ROMA, 2506.16411, SRLM — arguably the three
  highest-idea-density non-anchor items).

## Intake map (Ф3) {#intake-map}

Repos → `refs/src/{roma,fast-rlm,redel,recursive-llm}` (rlm already
present; record HEAD pins). Papers → `refs/papers/{2506.16411,
2510.11967,2603.15653,2605.06639}.pdf` + re-fetch 2512.24601 **v3**
+ runner-up PDFs 2405.17402, 2603.02615. Articles →
`refs/articles/{zhang-rlm-blog,primeintellect-rlm,cognition-dont-
build-multi-agents,cognition-multi-agents-working,anthropic-multi-
agent-research,minrlm-guide}.html|md` with access dates.
