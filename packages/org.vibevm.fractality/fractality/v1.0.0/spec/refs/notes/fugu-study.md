# Study note — Sakana Fugu (the anchor project) {#root}

_T1 note (boss-authored) for INVENTORY S27 — tech report arXiv
2606.21228 (read at **v2**, 2026-06-23; boss-read via pdftotext)
+ sakana.ai/fugu + /fugu-release + /fugu-beta pages (snapshots
2026-07-11) + official repo SakanaAI/fugu (**no license**, Shell,
799★ — README-level only: an installer piping Fugu into Codex as
`codex-fugu`, plus docs pointers). One project, one note (D-R7).
Clean-room: facts and decisions only._

## What it is {#what}

A **multi-agent system exposed as a single model**: Fugu models
are themselves LMs trained to take a user query and dynamically
devise the agentic scaffold that solves it over a pool of frontier
workers (Gemini-3.1-Pro, Opus-4.8, GPT-5.5, …) — which workers,
what instructions/roles, who sees whose outputs, when to
synthesize. One OpenAI-compatible endpoint; the scaffold is
generated at inference time, per query. Framing: orchestration as
a **new scaling axis** — capability from composition, not bigger
training runs; pool is swappable/configurable (favor/exclude
providers, compliance constraints) without retraining. Launched
2026-06-22; explicit geopolitical positioning (frontier capability
"without the risk of export controls" — Fable/Mythos are named as
not-publicly-accessible and excluded from the pool).

## Architecture facts {#architecture}

- **Fugu (latency tier) = Trinity, productized:** backbone LM +
  **lightweight selection head over a hidden state** → N logits
  (one per worker); **decision-from-logits, no autoregressive
  decode** — hidden state at an early token position dispatches
  the query; **no roles** (unlike Trinity) — always dispatch-as-
  worker, shrinking the coordination space for latency. Backbone
  adapted by **singular-value fine-tuning** (train only
  singular-value scales of selected matrices) + the head — a tiny
  trainable set.
- **Two-stage training:** (1) SFT on single-step verifiable tasks
  where the label is a **soft distribution over workers derived
  from measured per-worker rewards** (run every worker k times on
  each task, average rewards, softmax with temperature; KL loss)
  — richer than a hard best-worker label, robust when several
  workers tie. (2) **sep-CMA-ES over end-to-end multi-turn
  trajectories** in real coding-assistant harnesses (Claude Code,
  Codex, OpenCode) with terminal 0/1 reward — chosen because
  end-to-end signals are sparse/noisy and ES needs no ranking
  labels; end-to-end reveals capabilities invisible to single-shot
  scores (models strong at reasoning but weak at tools, and vice
  versa).
- **Fugu-Ultra (quality tier) = Conductor, scaled:** GRPO-trained
  (no KL penalty) reasoning model that emits full workflows —
  up to 5 steps, each (NL subtask, worker id, **access list** =
  which previous steps' outputs enter this worker's context).
  Reward: format-parseable gate, then correctness 1 / 0.5.
  The orchestrator may name **itself as a worker** → recursive
  topologies.
- **Function calling at multi-agent scale (their §3.2.2, the
  production meat):** any agent may call tools at any time, so the
  orchestrator must track which agent emitted every call and route
  each function-call loop back to that agent within the topology.
  Two laws resolve the memory tension: **intra-workflow agent
  isolation** — an agent sees other agents' work ONLY through the
  access list, otherwise its transcript contains only its own
  actions — explicitly to prevent **"orchestration collapse"**
  (the first agent to touch the environment steers everyone;
  later agents become redundant followers); and **persistent
  inter-workflow shared memory** — agents DO see tool calls from
  previous workflows of the conversation, so background context
  survives turns without redundant rediscovery.

## Results facts {#results}

Sakana-reported (their own caveat culture: baselines are
provider-reported numbers; Fable 5/Mythos Preview never in the
pool): tops 10/11 benchmarks — SWE-Bench Pro 73.7 (Opus 4.8 =
69.2), Terminal Bench 2.1 82.1, LCB 93.2, GPQA-D 95.5, HLE 50.0,
CharXiv 86.6; loses only MRCRv2 to GPT-5.5 (93.6 vs 94.8) —
long-context retrieval is the one hole. Plain Fugu (single worker
per turn!) beats GPT-5.5 on Terminal Bench by **per-step
alternation** — swapping Opus in at critical debugging moments.
Routing distributions track known specializations (GPT math,
Gemini chem/bio/recall). AutoResearch: best mean BPB over 123
autonomous experiments; orchestration pulls ahead **after
mid-run**, when coarse gains are exhausted and fine judgment
dominates. Kana reading-order: writes a predictor + beam search
(NED 0.776 vs 0.642 best frontier) — orchestrated test-time
scaling on a no-training-data task.

**Observed strategies (§4.4):** dynamic aggregator choice (Gemini
aggregates trivia, GPT aggregates math; fixed-aggregator systems
bottleneck on their aggregator); tree topologies with independent
leaves whose partially-wrong answers a stronger aggregator fuses
into a fully-correct one; **build-and-debug alternation** (GPT
builds, Opus audits at crisis points — and the inverse);
**clean-slate re-examination** (when Opus dead-ended, GPT was
brought in cold and found the real bug — client-side concurrency,
not the server); **bring-in-a-specialist** (Opus writes the
crypto attack, GPT re-derives it bit-by-bit as math).

## Decisions we take {#decisions}

1. **The two-tier shape is ours too:** cheap fast routing rung
   (select one worker, no ceremony) + expensive workflow rung
   (decompose, topology, verify). Maps to need-gate verdicts:
   `route` should be a distinct, cheaper verdict than `spawn`
   (RD-1 gains a rung).
2. **Access lists ARE packet context contracts:** who-sees-what
   as explicit per-child data — our packets' context field should
   name which sibling/parent results are included (the Conductor
   grammar = D-C3-2's FileRef slices, generalized to result refs).
3. **Orchestration collapse is a named failure our fabric must
   test for** — it is the write-side twin of Cognition's context
   dispersal: isolation WITHIN a fan-out, shared memory ACROSS
   turns. Matches RD-11 (clean-context verifiers) and hardens
   RD-7 (single-writer): parallel siblings must NOT see each
   other mid-flight unless the parent's topology says so.
4. **Soft-label routing from measured rewards** is PP-002's
   acceptance data turned into a training target: our journal's
   per-worker outcomes per task-class are exactly the "run k
   times, average reward" table. Design the acceptance schema so
   the soft distribution is a query (feeds delegation-rules now,
   training later).
5. **End-to-end beats single-shot for judging workers** — their
   ES stage exists because benchmark scores mislead about
   in-harness behavior. Our worker-credibility facts must be
   harness-grounded (packets completed under tools), not
   eval-score priors.
6. **Per-step alternation** (not per-task assignment) is where
   plain Fugu's wins come from — our boss surfaces should make
   mid-task worker swap cheap (a run's next packet may go to a
   different profile).
7. **Aggregator selection is a decision, not a fixture** — the
   merge node of a packet tree should carry a profile chosen for
   the aggregation domain (extends RD-7).
8. **The consumer surface is a coding-agent integration**
   (`codex-fugu`) — market signal that orchestrators are consumed
   from inside harnesses, exactly fractality's posture (CLI/MCP
   into the boss's harness), not a separate app.

**Non-adoptions:** training an orchestrator model (v0.x is
policy-as-data; RD-20 already defers the lever); hidden
proprietary routing (our journal makes every routing decision
inspectable — anti-Fugu transparency as a feature); benchmark
category (an orchestrated system's score ≠ a model's score — the
skeptics' point, kept as a trial-design caveat alongside RD-21).
