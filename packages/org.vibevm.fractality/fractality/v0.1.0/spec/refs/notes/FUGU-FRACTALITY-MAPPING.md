# FUGU ↔ FRACTALITY — the mapping analysis {#root}

_Ф6 deliverable — the owner's «большой и умный анализ»: how the
Fugu findings land on the RLM research corpus (RD-1…21,
[`RLM-SYNTHESIS.md`](RLM-SYNTHESIS.md)), the vision
([`VISION-RECURSIVE-FABRIC.md`](../../VISION-RECURSIVE-FABRIC.md)),
and the Stage B draft
([`FRACTALITY-RLM-PLAN-v0.1.md`](../../plans/FRACTALITY-RLM-PLAN-v0.1.md)).
Verdict vocabulary: **confirms** (independent validation, no
action) / **strengthens** (adds evidence or a mechanism to an
existing delta) / **changes** (the draft said X, evidence says
X′) / **new** (nothing in our corpus covered it). Changes are
APPLIED to the draft in this same commit._

## 1. The verdict in one paragraph {#verdict}

Fugu is the strongest single validation of the fractality bet we
have found: a shipped, benchmark-leading product whose entire
thesis is that **orchestrating a pool of heterogeneous frontier
workers is a scaling axis that beats every constituent** — the
economics of PROP-001 §1, proven at market. Its lineage validates
the vision pillar-by-pillar (learned recursive topologies → V1;
verifier roles → PP-002/V3's acceptance shape; capability ladders
over pools → V4; per-query adaptive scaffolds → the need-gate).
And its gaps map exactly onto what the fabric already owns:
runtime enforcement, cost accounting, transparency, escalation to
a human, durable artifacts. The one thing Fugu proves we must NOT
do is treat descent and conducting as rivals — Fugu loses
precisely where context descent wins (MRCRv2), and our Stage B
carries both axes. Net: the Stage B draft survives with three
changes and one new decision; nothing in it is falsified.

## 2. The mapping table {#table}

| Fugu finding (FD) | Ours it lands on | Verdict → action |
|---|---|---|
| FD-1 two-tier route/conduct | RD-1 need-gate; D-C3-1 | **changes** — verdict set gains `route` (cheap single-worker dispatch); applied. |
| FD-2 access lists | RD-5/RD-8; D-C3-2 | **changes** — packets gain explicit `context_from` result-refs; applied. |
| FD-3 orchestration collapse; isolation-in-wave + memory-across-turns | RD-7 single-writer; RD-11 clean context; D-C3-5 | **changes** — sibling visibility default = isolated, topology explicit; anti-collapse probe added to the trial; applied. |
| FD-4 chosen aggregator/verifier | RD-7 merge node; RD-11; D-C3-5/Ф5 | **strengthens** — merge/acceptance nodes carry domain-chosen profiles (text added to D-C3-5). |
| FD-5 soft-label routing from measured rewards | RD-13 journal-for-learning; PP-002 | **strengthens** — journal schema must make the per-worker×task-class reward table a query (D-C3-8 wording). |
| FD-6 harness-grounded credibility | PP-002; RD-13 | **confirms** — acceptance facts must come from in-fabric runs, not eval priors (already PP-002's design). |
| FD-7 per-step alternation | RD-9 await verbs; V1 | **strengthens** — mid-task profile swap named as a first-class boss move (D-C3-4 note). |
| FD-8 availability masking | V4 effective-top fallback; RD-2 policy data | **new** (mechanism) — folded into the NEW D-C3-10 routing-policy decision. |
| FD-9 verifier-accept completion + cold-verifier suppression | PP-002; F25 cold board; RD-11 | **strengthens** — acceptance-gated completion + refuse-acceptance-on-empty-tree land in D-C3-6/Ф5 wording. |
| FD-10 T/W/V role floor | packet types; VISION §V1 | **confirms** — plan/work/acceptance already the shape; vocabulary noted. |
| FD-11 tiny routing brain | RD-1/RD-2 policy-as-data | **confirms** — features (journal facts) over model capacity; delegation-rules stays tabular in v1. |
| FD-12 learned recursive topologies pay; naive self-revision ties | VISION §V1/V2; RD-2 guards; RD-20 training lever | **confirms + strengthens** — third independent source that recursion needs fresh context or a different specialist; RD-2's guard set unchanged but now triple-sourced. |
| FD-13 transparency as edge | I3; RD-17 replay | **strengthens** — result metadata surfaces depth/spawn counts by default (D-C3-8 note). |
| FD-14 orchestration ≠ long-context fix | Stage B scope §3; RD-6 regimes | **confirms** — descent and conducting are complementary; scope options unchanged. |
| FD-15 format-gate-then-quality | RD-5 seam validation | **confirms** — Ajv-then-retry ordering already matches. |
| FD-16 classes not model names; pool churn as design condition | V4 ladder; RD-10; delegation-rules | **strengthens** — folded into D-C3-10. |

## 3. Changes applied to the Stage B draft {#changes}

1. **D-C3-1** — the need-gate verdict set is now `inline | route |
   fold-local | spawn | escalate` (FD-1): `route` dispatches one
   worker with no workflow ceremony, priced for latency.
2. **D-C3-2** — packets carry `context_from: [result-refs]`
   (FD-2): the access-list contract; parent-gives-everything is
   dead by default.
3. **D-C3-5** — sibling isolation is the default; visibility is
   granted, not assumed (FD-3); the merge node's profile is chosen
   for the aggregation domain (FD-4). The Ф6 trial gains an
   orchestration-collapse probe (two siblings, one seeded with a
   misleading early action — does the fabric keep them
   independent?).
4. **NEW D-C3-10** — routing policy data (FD-8/11/16 + FD-5):
   availability masks over profiles; capability classes instead
   of model names; the journal's per-worker×task-class outcome
   table exposed as a query, feeding delegation-rules today and
   any learned router later.
5. Wording strengthenings (no structural change): D-C3-4 mid-task
   alternation note (FD-7); D-C3-6/Ф5 acceptance-gated completion
   + no-cold-acceptance (FD-9); D-C3-8 depth/spawn-count metadata
   + reward-table query (FD-5/13).

## 4. What Fugu does not change {#moats}

The draft's enforcement plane (six-axis budgets + wall-clock,
RD-4) — Fugu runs on turn caps with cost shaping *disabled*; the
escalation channel (V3) — Fugu has no human in its loop at all;
files-as-IPC and replay (RD-17) — Fugu's routing is opaque by
design; the clean-room posture; and the descent core (RD-5/RD-6)
— which Fugu's own MRCRv2 loss argues FOR. These are the moats;
the mapping found no evidence against any of them.

## 5. P-F4 verdict {#pf4}

**CONFIRMED**: 6 confirmations, 7 strengthenings, 3 changes, 1 new
decision — well past the "≥3 map onto existing deltas and ≥1
forces a change" bar. The falsifier (Fugu orthogonal) is dead:
Fugu is the same thesis, shipped, minus our enforcement and
transparency planes.
