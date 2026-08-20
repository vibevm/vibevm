# RLM-SYNTHESIS — what the field knows, what fractality takes {#root}

_Ф5 deliverable of
[`FRACTALITY-RLM-RESEARCH-PLAN-v0.1`](../../plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md),
form per D-R6 (flow:comparative-research: two-way gaps, numbered
deltas, re-fetch list). Synthesized 2026-07-11 from the 11 study
notes in this directory (T1 `rlm-study.md`; T2 ×8; T3 ×2), which
carry the dated verbatim quotes and per-source evidence. The
deltas PROPOSE; acceptance happens in Stage B
(`FRACTALITY-RLM-PLAN-v0.1.md`, drafted alongside). Re-fetch list:
[`rlm-source-selection.md`](rlm-source-selection.md) (all URLs
access-dated 2026-07-10) + paper variants pinned in INVENTORY
(anchor read at v3 2026-05-11; 2506.16411 at v2/ICLR 2026)._

## 1. The state of the field in one paragraph {#state}

"RLM" names one specific mechanism (Zhang/Kraska/Khattab): the
prompt lives OUTSIDE the model as a variable in a persistent REPL;
the model writes code that peeks, carves, and **programmatically**
launches sub-LM calls over slices, keeping only constant-size
metadata in its own history. It demonstrably processes 10M+ tokens,
beats compaction/CodeAct/Claude Code on dense tasks, and its
depth-1 form is the workhorse — but the 2026 wave established the
boundaries: recursion itself is not the magic (SRLM: programmatic
context interaction is; ablations match without recursion); depth
2+ hurts untrained models (overthinking: 86.6%→60% wrapping Kimi
K2; 3.6 s→344.5 s latency); small models cannot drive a REPL
(minRLM −9.5pp on nano) though spawn-shaped recursion RESCUES
small models (+10–50pp, THREAD); theory now says when descent wins
(superlinear model-noise ⇒ past T₀ chunked-weak beats single-
strong) and when it provably cannot (Silo Effect: cross-chunk
dependence ⇒ escalate, don't fan out). Meanwhile the engineering
ecosystem (ROMA, fast-rlm, ReDel) already ships the primitives
fractality planned — need-gates, budget lattices, await-any/all,
event-sourced replay — and production practice (Anthropic,
Cognition) converged on: single writer, artifacts-not-prose,
clean-context verifiers, capability-gated advisors, token spend as
the quality lever.

## 2. Two-way gaps {#gaps}

**Where the field is ahead of the fabric (we trail):**
in-engine budget lattices with six axes (fast-rlm); an isolated,
auditable decompose-or-execute gate (ROMA's Atomizer); typed
aggregation that answers the parent's goal (ROMA, and the theory's
aggregator-noise term); await-any/await-all delegation verbs
shipped since 2024 (ReDel); event-sourced replay with scrubber UIs
(ReDel, fast-rlm TUI); schema-validated child returns with
retry-on-violation (fast-rlm); deterministic context triage
(minRLM entropy map); trained folding/delegation policies
(FoldGRPO, RAO) with cheap distillation recipes (anchor: 1k
trajectories, 48 H100-h).

**Where the fabric is ahead of the field (we lead):** real OS-
process isolation with env whitelists (I1) vs in-process exec
sandboxes; **wall-clock enforcement and kill-trees** — absent in
every surveyed system (recursive-llm's timeout is literally
recorded-but-not-enforced; ROMA defers to httpx; 545 s/query
depth-2 runs went unkilled); honest metering (ACP agents report
zero usage; our workers meter for real); files-as-persistence +
FileRef claim-checks (I2) — independently validated by Anthropic's
production filesystem pattern; one telemetry store with prices
(I3); provider-agnostic profiles; the human-ruling culture (RPs,
manual tests, pre-registered trials) that none of the surveyed
projects has; and the V5 attachable-terminal horizon, which no one
else even names.

## 3. The deltas (RD-1 … RD-21) {#deltas}

_Priority P1 = Stage B core scope; P2 = Stage B strongly
recommended; P3 = named, schedulable later. Target home in
brackets. Sources cited by note._

- **RD-1 (P1) The need-gate is an isolated, auditable call** with
  a typed verdict (descend / inline / escalate) + journaled
  reason; policy inputs from `delegation-rules`. [MC/boss verb;
  VISION §V1] (roma-study; rao-study d.1)
- **RD-2 (P1) Depth & wrap policy as data:** default depth 1;
  hard guards — task fits window → send whole (srlm-study);
  O(1)-retrieval → no machinery; natively-strong long-context
  model → don't wrap (runners-up S24); depth ≥ 2 only for
  super-linear tasks under strong-coder roots, behind an
  experimental flag. [delegation-rules] (rlm-study d.3)
- **RD-3 (P1) Boundary behaviors per verb, catalogued:** spawn
  request at cap → structured soft refusal (recursive-llm); child
  profile at cap → capability removal + leaf surface (redel,
  fast-rlm); work packet at cap → force-execute (roma). [MC +
  profiles]
- **RD-4 (P1) The budget lattice:** depth / per-agent calls /
  per-call token ceiling / cumulative tokens / currency / global
  calls — PLUS wall-clock, which only MC in this whole field
  enforces. Structured failure per axis. [MC quotas → PROP]
  (fast-rlm-study d.1)
- **RD-5 (P1) Result contract law:** children return
  symbols/files, never transcripts; schema-validated at the seam
  with one retry-on-violation; parent history stays
  metadata-only; the fold law — a child's transcript never enters
  a parent's context. [packets/PROP-001 §3] (rlm-study d.4-5;
  fast-rlm d.3; context-folding d.1-2; anthropic S21)
- **RD-6 (P1) Regime triage before fan-out:** Silo Effect →
  escalate UP (V3); Brain Fog → descend; trivial → inline. Formal
  backing: Proposition 3.1 + the three regimes. [delegation-rules;
  VISION §V3] (dnc-noise-study d.1)
- **RD-7 (P2) Single-writer + goal-answering aggregation:**
  parallel workers never co-write an artifact; every packet tree
  has a designated merge node whose contract is "answer the
  parent's goal", not concatenation. [packets/MC]
  (rlm-articles S20; roma-study d.3)
- **RD-8 (P2) Sub-task spec law:** the parent rewrites child
  goals so results COMPOSE (the planner trick: "2nd smallest" →
  per-chunk "two smallest"); every brief carries
  objective/format/tools/boundaries (Anthropic quartet); MC
  refuses near-duplicate pass-the-buck specs (ReDel's fuzz
  guard). [delegate skill + MC check] (dnc-noise d.2; redel d.2)
- **RD-9 (P2) `await any|all|named` are v1 MC verbs**, not
  horizons — the field shipped them in 2024; parallel siblings
  are the native idiom (RAO's gather; Anthropic's 90% time cut).
  [MC API/PROP] (redel-study d.3; rao-study d.3)
- **RD-10 (P2) Advisor channel, evidence-based (V4):** production
  precedent exists (Cognition "Smart Friend"); the CALLER must
  clear a capability bar to benefit (SWE-1.5→1.6 threshold;
  minRLM nano inversion) → delegation-rules row `advisor_enabled
  ⇐ caller_class ≥ medium`; route by capability, not difficulty;
  trigger on intrinsic uncertainty signals (SRLM's
  self-consistency / trace length / verbalized confidence);
  context hand-off ≈ fork of the caller's context. [VISION §V4;
  delegation-rules] (rlm-articles S20; srlm-study d.3)
- **RD-11 (P2) Verifiers get clean context BY DESIGN** (measured:
  review works best with zero shared context) — acceptance
  packets (PP-002) carry no parent transcript. [PP-002 / MC
  acceptance] (rlm-articles S20)
- **RD-12 (P2) Promotion = config-injected capability grants** on
  the child's own harness (deny-lists / settings at spawn;
  boss-surface may REMOVE work tools — root_has_tools=False):
  V1's mechanics exist today as settings writes our adapter
  already performs. [initiative adapter; VISION §V1]
  (fast-rlm d.5; runners-up S25; redel d.5)
- **RD-13 (P2) Journal schema for future learning:** record per
  decision (task, verdict+reason, sub-spec, outcome, cost, depth,
  parent-id) — the RAO-trainable tuple; add process-metric
  candidates (unfolded-tokens-in-parent, out-of-scope-actions)
  and the anomaly signature token-plateau-with-time-explosion as
  a stall alarm. [I3 schema] (rao-study d.1; context-folding d.4;
  runners-up S24)
- **RD-14 (P2) Effort-scaling + savings framing as data:** query
  class → swarm size (1 agent/3–10 calls … >10 subagents); token
  spend explains 80% of quality variance — quotas are quality
  knobs; feeds the deferred savings methodology. [delegation-
  rules; §15 savings] (rlm-articles S21)
- **RD-15 (P3) Deterministic context-triage verbs:** entropy-map
  + head/mid/tail preview before choosing a strategy; sparse-
  sampling chunk-size calibration; prefer programmatic
  regeneration (diff replay) over LM re-reading. [future boss
  verbs] (rlm-articles S22, S18; dnc-noise d.5)
- **RD-16 (P3) A local-fold rung between inline and spawn:** the
  need-gate chooses among inline / fold-locally (branch-return
  without a pod) / spawn — Context-Folding shows the middle rung
  pays (10× smaller active context). [VISION §V1/V5]
  (context-folding d.3)
- **RD-17 (P3) Replay-grade observability:** delegation edges as
  first-class events; periodic snapshot + event-count checksum;
  a reducer-with-undo scrubber over the journal (pairs with V5 /
  GUI horizon). [journal/GUI] (redel-study d.4; fast-rlm d.6)
- **RD-18 (P3) Resilience policy data:** per-task-type
  retry/backoff, per-backend circuit breakers, and
  `completed_with_failures` aggregation over surviving children.
  [MC resilience] (roma-study d.4-5)
- **RD-19 (P3) Surface wording is a measured variable:** ship one
  worked decomposition example (peek → grep → partition-map →
  recurse); test guidance wording in trials — tips can INVERT
  behavior (PI's GLM regression; our own C2 GLM-quotes-the-matrix
  finding). [initiative surfaces; MT protocol] (rlm-study d.6;
  rlm-articles S19)
- **RD-20 (P3) The training lever, named and deferred:**
  distill-the-root (1k trajectories) / RLVR length
  generalization / FoldGRPO / RAO — deferred to a future
  campaign; RD-13 keeps the data a query away. [future campaign]
- **RD-21 (P3) Trial law:** all MT-C3 comparisons budget-matched
  (equal wall-clock or equal spend), or they measure nothing.
  [MT protocol] (srlm-study d.4)

## 4. What this settles from VISION §5 {#vision-answers}

- *"Is an advisor just RLM depth-1 with a bigger sub-call?"* — No:
  same plumbing, inverted economics, plus a caller capability bar
  and an uncertainty trigger (RD-10).
- *"Where does the REPL live?"* — We don't adopt a REPL as
  architecture; its contracts land in packets (RD-5), and the
  local-fold rung (RD-16) covers the in-thread case; V5 terminals
  remain the candidate host for literal in-worker REPLs.
- *Depth guardrails* — RD-2/RD-3/RD-4 answer the §5 question with
  field data; hard caps now, learned policy later (RD-20).
- *Advisor accounting* — the budget lattice gives the axes; whose
  line pays is a Stage B decision (flagged in the draft).

## 5. Prediction verdicts for the plan close {#verdicts}

- **P-R1 CONFIRMED** — ≥16 qualifying repos, ≥17 papers surfaced
  before the cut (wave records).
- **P-R2 CONFIRMED at the boundary** — overlap exactly 60% (9/15
  under the frozen membership rule); each wave contributed 3 items
  the other never saw. The two-modality design earned its cost;
  the margin says a third modality would likely still add.
- **P-R3 CONFIRMED** — the anchor remains the single most
  load-bearing source; SRLM critiques *within* the anchor's
  posture (context-as-object stands), superseding nothing.
- **P-R4 CONFIRMED** — the counterpoint arc generated multiple
  adopted constraints (RD-2 guards, RD-7 single-writer, RD-11
  clean-context verifier), not one.
