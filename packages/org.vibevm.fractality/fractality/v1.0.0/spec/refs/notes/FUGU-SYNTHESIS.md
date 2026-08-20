# FUGU-SYNTHESIS — what Sakana's orchestrator teaches the fabric {#root}

_Ф5 deliverable of
[`FRACTALITY-FUGU-RESEARCH-PLAN-v0.1`](../../plans/FRACTALITY-FUGU-RESEARCH-PLAN-v0.1.md).
Sources: the four study notes (`fugu-study.md`, `trinity-study.md`,
`conductor-study.md`, `openfugu-study.md`), which carry quotes and
per-source evidence; re-fetch = INVENTORY S27–S30 (all dated).
Deltas numbered FD-1…16; the mapping onto our corpus is the
separate Ф6 deliverable
[`FUGU-FRACTALITY-MAPPING.md`](FUGU-FRACTALITY-MAPPING.md)._

## 1. The state of the thing in one paragraph {#state}

Fugu is the first shipped **orchestration-as-a-model**: a trained
LM whose only job is to route/conduct a swappable pool of frontier
workers behind one endpoint, in two tiers — Fugu (Trinity-lineage:
hidden-state selection head, no decode, single worker per turn)
and Fugu-Ultra (Conductor-lineage: GRPO reasoning model emitting
workflow steps with per-step access lists, itself allowed as a
worker → recursive topologies). Training: soft-label SFT from
measured per-worker rewards, then sep-CMA-ES over end-to-end
trajectories in real coding harnesses. Sakana-reported results top
10/11 benchmarks (only MRCRv2 lost) with production strategies —
per-step builder/debugger alternation, clean-slate second
opinions, dynamic aggregator choice — and one named failure class,
**orchestration collapse**, answered by intra-workflow isolation +
inter-workflow shared memory. Skeptics' residue: scores of an
orchestrated system are a different category than model scores;
routing is hidden; no independent reproduction; and at runtime
(per OpenFugu's reverse engineering) there is **no cost
accounting and no verifier inside Ultra workflows** — the
enforcement plane is thin.

## 2. Two-way gaps {#gaps}

**Fugu ahead of the fabric:** learned routing/conducting (we have
none — policy is hand-authored data); dynamic per-task aggregator
and verifier CHOICE; per-step worker alternation as a norm;
explicit visibility topologies (access lists); production evidence
that orchestration beats every constituent, including on
autonomous research loops.

**The fabric ahead of Fugu:** runtime enforcement (six-axis
budgets + wall-clock + kill-trees vs turn-caps only and w_cost=0);
**runtime cost accounting** (Fugu leaks only `usage.fugu_turns`);
routing transparency (our journal records every decision with
reasons — Fugu's routing is proprietary and unsteerable);
files-as-IPC durable artifacts; a human at the top (Fugu has no
escalation-to-human concept at all — V3 is OUR axis); process
isolation with env whitelists; and long-context descent (their
MRCRv2 loss — RLM-style context machinery, Stage B's core, is
complementary to orchestration, not superseded by it).

## 3. The deltas (FD-1 … FD-16) {#deltas}

- **FD-1** Two-tier orchestration: a cheap `route` verdict
  (dispatch one worker, no decomposition ceremony) distinct from
  `spawn` (workflow). [need-gate]
- **FD-2** Access lists: per-child context = explicit refs to
  named prior results, not parent-gives-everything. [packets]
- **FD-3** **Orchestration collapse** is a named, testable
  failure: siblings isolated within a fan-out unless topology
  grants visibility; shared memory across turns, not within the
  wave. [packets/MC; trial probe]
- **FD-4** Aggregator and verifier are CHOSEN per task domain,
  never fixtures. [merge node; acceptance profiles]
- **FD-5** Soft-label routing distributions from measured
  per-worker outcomes — the journal's acceptance data IS the
  future training table. [I3 schema; PP-002]
- **FD-6** Worker credibility must be harness-grounded (end-to-end
  under tools), not benchmark-prior — Sakana added a whole ES
  stage because single-shot scores mislead. [PP-002/profiles]
- **FD-7** Per-step alternation: mid-task worker swap must be
  cheap (next packet → different profile; Opus-at-the-debugging-
  moment is where the wins live). [boss verbs]
- **FD-8** Availability masking: route over the AVAILABLE subset
  (−inf the absent); V4's effective-top fallback made mechanical.
  [delegation-rules/profiles]
- **FD-9** Verifier-accept as run-tree completion + cold-verifier
  suppression (no acceptance on an empty tree). [acceptance/MC]
- **FD-10** Thinker/Worker/Verifier ≅ plan/work/acceptance packet
  types — a three-role floor for packet vocabulary. [packets]
- **FD-11** The routing brain can be tiny (20K params; features
  matter more than capacity) — policy-as-data now, hidden-state
  features far-horizon. [delegation-rules]
- **FD-12** Recursion at the orchestration layer is learnable and
  pays (Conductor self-as-worker) — but naive self-REVISION ties
  (OpenFugu) and echoes the overthinking file: recursion wants a
  fresh context or a different specialist. [VISION §V1; RD-2]
- **FD-13** Transparency is a product edge: journal every routing
  decision with reason; surface tree depth/spawn counts in result
  metadata (their `usage.fugu_turns`, done honestly). [journal/
  status]
- **FD-14** Orchestration does NOT fix long context (MRCRv2 loss)
  — descent (RLM machinery) and conducting are complementary
  axes; Stage B keeps both. [Stage B scope]
- **FD-15** Format-gate-then-quality ordering for any scored
  output (parse first, judge second); NL workflows stay
  non-adopted — TOML packets are our auditable equivalent.
  [seam validation]
- **FD-16** Author policies against capability CLASSES with
  randomized/varying pools in mind, never model names — pool
  churn is the design condition, not an edge case. [delegation-
  rules; V4 ladder]

## 4. Prediction verdicts {#verdicts}

- **P-F1 CONFIRMED** (on W2 evidence alone): ≥8 qualifying extras
  surfaced (official repo, OpenFugu, trinity_coordinator, the
  critique shelf, integration guides).
- **P-F2 NOT EVALUATED** — W1 was stopped mid-flight during the
  owner's token pause and abandoned by the mandate revision; no
  overlap metric exists. (Recorded as the stage's deviation.)
- **P-F3 CONFIRMED** — the tech report names Fugu = Trinity
  productized (§3.1) and Fugu-Ultra = Conductor scaled (§3.2);
  same author cluster; one research program.
- **P-F4 CONFIRMED** — see the mapping: ≥6 confirmations/
  strengthenings of existing RD-deltas and **three changes + one
  new decision** applied to the Stage B draft.
