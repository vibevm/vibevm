# FRACTALITY-FUGU-RESEARCH-PLAN v0.1 — Campaign 3 · Stage A2: the Sakana Fugu research {#root}

_Status: **EXECUTING from birth 2026-07-11 ~01:10** — the owner's
order is «Напиши план и выполни»; no review gate on execution
(unlike Stage A's RP-R1). Genre: campaign plan. Method: inherits
[`FRACTALITY-RLM-RESEARCH-PLAN-v0.1`](FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md)
wholesale — the three-wave protocol (D-R2), selection criteria
(D-R3), clean-room intake pipeline (D-R4), tiered study +
delegation posture (D-R5), note/synthesis forms (D-R6),
one-project-one-note (D-R7) — except where this file states
otherwise. Runs cold._

## 1. The mandate (owner, 2026-07-11, verbatim) {#mandate}

> Нужно еще одно исследование. Как всегда: deep research и обычный
> web search, скачивание документов, анализ, синтез... все как ты
> только что делал. Но теперь тема другая: Sakana Fugu.
> https://sakana.ai/fugu/ Прочитай что это такое, технический
> репорт: https://arxiv.org/abs/2606.21228, TRINITY:
> https://arxiv.org/abs/2512.04695, Learning to Orchestrate:
> https://arxiv.org/abs/2512.04388 и другие документы про нее. Это
> проект, достигший достаточно больших успехов — возможно, часть
> из них относятся и к нам. После исследования и синтеза
> непосредственно данных про Sakana Fugu, нужен еще один большой и
> умный анализ: как то что ты узнал про Fugu ложится на наши
> предыдущие исследования и FRACTALITY-RLM-PLAN-v0.1.md. Возможно,
> нам нужно все это улучшить. Напиши план и выполни.

## 2. Goal and deliverables {#goal}

Understand what Fugu is and which of its successes transfer to the
fabric; then map the findings onto the RLM-research corpus
(RD-1…21), the VISION pillars, and the Stage B draft — and improve
the draft where the mapping demands it.

Deliverables: (1) wave records + merged source selection; (2)
INVENTORY rows with license verdicts before study; (3) study notes
per project/document; (4) `FUGU-SYNTHESIS.md` (FD-deltas, D-R6
form); (5) **`FUGU-FRACTALITY-MAPPING.md`** — the owner's "большой
и умный анализ": Fugu findings × (RD-deltas / VISION / Stage B
D-C3 decisions), each mapping = confirm / strengthen / change /
new; (6) amendments applied to
[`FRACTALITY-RLM-PLAN-v0.1.md`](FRACTALITY-RLM-PLAN-v0.1.md)
(a DRAFT — improving it is in scope; commissioning stays RP-C3-1);
(7) close record with prediction verdicts.

## 3. Deviations from the Stage A method {#deviations}

- **Anchors are owner-named, four of them:** the product page
  (sakana.ai/fugu) + tech report 2606.21228 + TRINITY 2512.04695 +
  Learning to Orchestrate 2512.04388. The waves rank the REST
  («и другие документы»): up to ~8 additional sources (repos with
  licenses, Sakana posts, follow-ups, critiques) by the D-R3
  criteria; no fixed 5/5/5 shape.
- **A second analysis deliverable** (the mapping doc) sits between
  synthesis and close — it is the point of the stage.
- **Stage B draft edits are in scope** (it is uncommissioned; its
  status line gains a revision note citing this stage).

## 4. The waves (frozen) {#waves}

**W1 — deep research, one run:**

> Sakana AI's Fugu (sakana.ai/fugu) — what the project is, its
> architecture, results, and lineage. Anchors, already known: the
> Fugu technical report (arXiv 2606.21228), TRINITY (arXiv
> 2512.04695), Learning to Orchestrate (arXiv 2512.04388). It is
> July 2026. Deliver: (1) what Fugu IS — system shape, components,
> what it orchestrates, how it is trained, benchmark results,
> notable deployments/claims; (2) a ranked list of the most
> important OTHER primary sources about it — Sakana blog posts,
> GitHub repositories (with licenses), follow-up or critique
> papers, high-quality practitioner articles — up to 8, each with
> access-dated URL and a why-it-matters; include at least one
> critical/skeptical source if any exists; (3) for every finding:
> which ideas transfer to a mission-control + pod agent fabric
> with recursive delegation (boss/worker economics, orchestration
> policy, model routing, budgets, escalation, advisors).

**W2 — plain web search, before reading W1 output:**

```
1.  Sakana Fugu
2.  sakana.ai fugu announcement blog
3.  arXiv 2606.21228 Fugu technical report
4.  TRINITY Sakana 2512.04695
5.  "Learning to Orchestrate" 2512.04388
6.  Sakana AI orchestration multi-agent 2026
7.  github SakanaAI fugu OR trinity OR orchestrate
8.  Sakana Fugu benchmark results analysis
9.  Sakana Fugu critique OR skeptical
10. Fugu LLM orchestrator model routing
11. Sakana AI agent system 2026 paper
12. Fugu Sakana follow-up OR reproduction
```

**W3 — merge:** dedup, rank per D-R3, verdict table (anchors +
adopted extras + runners-up with why-not), overlap metric on the
adopted-extras set.

## 5. Phases {#phases}

Ф0 plan lands (this commit). Ф1 waves (W2 before W1 output). Ф2
merge → selection section in the wave record or a compact
`fugu-source-selection.md` + INVENTORY rows (licenses BEFORE
study). Ф3 intake (site snapshot, PDFs, repos @pins, article
snapshots — gitignored; INVENTORY pins). Ф4 study (T1 = Fugu
product+tech report, one note; T2 = TRINITY, L2O, adopted extras;
T3 grouped leftovers; GLM surveys for repos/HTML under the
live-observation law — with the new first-output-timeout lesson;
boss reads papers and authors all notes). Ф5 `FUGU-SYNTHESIS.md`.
Ф6 **the mapping** `FUGU-FRACTALITY-MAPPING.md` + Stage B draft
amendments. Ф7 close (verdicts, ledger, dashboard). Every phase a
safe stop; specmap re-mint rides every anchored-spec commit.

## 6. Predictions {#predictions}

- **P-F1** — the waves surface ≥ 8 qualifying additional sources
  beyond the four anchors. (Falsifier: Fugu's public footprint is
  thinner than its успехи suggest — itself a finding.)
- **P-F2** — W1∩W2 overlap on the adopted extras ∈ [50%, 100%).
  (Either falsifier bounds the value of multi-modal search for
  narrow-topic stages.)
- **P-F3** — TRINITY and Learning to Orchestrate are component/
  lineage papers of Fugu (the tech report builds on both).
  (Falsifier: the owner's three links are three unrelated lines —
  changes how the mapping reads them.)
- **P-F4** — ≥ 3 Fugu findings map onto existing RD-deltas
  (confirm/strengthen) AND ≥ 1 forces a *change* (not an
  addition) to the Stage B draft. (Falsifier: Fugu is orthogonal
  to the fabric — recorded as such, the draft stands.)

## 7. Review points {#review-points}

None blocking — the mandate is «выполни». RP-C3-1 (Stage B
commissioning) remains OPEN and untouched by this stage; Stage B
*content* changes land as draft amendments citing this plan.

## 8. Ledger {#ledger}

| phase | commits | confirmed / falsified | notes |
|---|---|---|---|
| Ф0 | this commit | — | plan lands; execution immediate per mandate |

## 9. Log {#log}

- 2026-07-11 ~01:10 — drafted and launched in one act (owner:
  «Напиши план и выполни»); dashboard
  `reports/2026-11-07-01-10-fuguresearch-started-plan.md`.
- 2026-07-11 — **mandate revision (owner, verbatim):** «не нужно
  ждать deep research. Возьми только pdf и скачанные страницы (все
  пейперы по Fugu), и возможно репозитории (анализ репозиториев
  обязательно делегировать в GLM, PDF анализируй сама) и проведи
  изучение и остальные части вплоть до правок черновика и
  закрытие». Context: the owner's token budget paused the session
  mid-Ф1; the W1 workflow (`wf_9315022c-7aa`) was stopped by the
  harness and is abandoned as a deviation. Scope narrows to: the
  four anchors (3 PDFs boss-read + Sakana page snapshots) + repos
  (OpenFugu via GLM delegate; SakanaAI/fugu README-level only —
  no license on the official repo). The W2-articles shelf is cut.
  P-F2 becomes NOT EVALUATED; other predictions judged on the
  narrowed evidence.
