# FRACTALITY-RLM-RESEARCH-PLAN v0.1 — Campaign 3 · Stage A: the RLM research {#root}

_Status: **DRAFTED 2026-07-10 22:15. NOT EXECUTING — RP-R1 (owner
review of this plan) is OPEN**, per the owner's order verbatim: «Не
исполняй план, вначале дай посмотреть на него». Genre: campaign
plan (flow:campaign-plans — PLAN + BASELINE + PREDICTIONS + LOG +
REPORT in one document). Runs cold: any session executes this with
no memory of the planning conversation._

## 1. The mandate (owner, 2026-07-10, verbatim) {#mandate}

First message (the commission):

> Перед тем как что либо делать с планированием RLM процесса, я
> хочу чтобы ты провел полный рисёч темы. Например, нашел 5 самых
> важных репозиториев на GitHub, посвященным RLM-подобным проектам
> которые рекурсивно запускают агентов, 5 самых важных научных
> исследований, и 5 самых крутых статей в интернете. Я хочу чтобы
> ты сделал это в 3 волны. Одна волна — это Deep Research. Вторая
> волна — поиск того же самого через web search без deep research.
> Третье — объединение результатов. Дальше ты все эти проекты и
> документы качаешь в /refs и изучаешь. Результаты изучения
> записываешь в /refs/notes по каждому проекту или документу. Цель
> изучения — понять всё про RLM и то, какие идеи нам нужно забрать
> себе. Ну и конечно, ты знаешь, есть один самый главный проект и
> документ про это: Recursive Language Models | Alex L. Zhang (есть
> документ на arxiv, есть код на github). Важное: мы именно
> вычленяем и понимаем идеи, мы не должны копировать код. Если нам
> хочется скопировать какой-то код — нужно понять его смысл и
> реализовать Clean Room Implementation. Это очень-очень важно,
> потому что копирование кода приведет к юридическим последствиям.
> Всё вышенаписанное это должно быть частью некоторого плана
> исследования.

Second message (the plan order):

> Теперь я хочу чтобы ты написал финальный большой план, как мы
> будем всё это делать. Ты можешь улучшить уже существующий план
> Campaign 3, или написать новый. Как лучше реши сама. В любом
> случае, после исследования RLM, план Campaign 3 нужно будет
> улучшить нашими идеями.

Third message (the execution gate): «Не исполняй план, вначале дай
посмотреть на него» — encoded as RP-R1 (§10).

## 2. Goal and non-goals {#goal}

**Goal:** understand everything about RLM — Recursive Language
Models and RLM-like systems that recursively launch agents — well
enough to name, with reasons, the ideas fractality takes for
itself; and exit with the Campaign 3 implementation plan seeded by
those ideas.

**Non-goals (named so their absence is not oversight):**

- **No implementation in this stage.** Not one line of product
  code. The stage ends at a drafted Stage-B plan, itself RP-gated.
- **No code copying, ever** — the owner's clean-room order (§4).
- **No exhaustive literature survey.** The cut is 5+5+5 by the §5
  criteria, with a runners-up ledger so nothing found is lost.

## 3. Current-state facts (verified 2026-07-10; do not re-discover) {#facts}

- **No Campaign 3 plan exists.** `spec/plans/` holds exactly two
  closed plans (IGNITION, INITIATIVE). "Campaign 3 (RLM, DEF-2)"
  lives only as the INITIATIVE §15 deferral. Hence D-R1: this is a
  new plan, and "improve the Campaign 3 plan with our ideas" means
  *authoring* Stage B from this stage's synthesis.
- **The anchor source is already inventoried and license-cleared:**
  [`refs/INVENTORY.md`](../refs/INVENTORY.md) S3
  (github.com/alexzhang13/rlm, MIT, pinned `72d6940`) and S4 (arXiv
  2512.24601 PDF, 9.9 MB, downloaded) — deep study deliberately
  deferred to this stage; `notes/rlm-study.md` is reserved as its
  note. The author's blog post (alexzhang13 site) is the third face
  of the same project and joins T1.
- **The refs tree exists and is gitignored wholesale** (verified
  `git check-ignore`): host `<repo>/refs/` with `src/`, `papers/`,
  `book/`, `study/`, `ts/`. `refs/articles/` does not exist yet —
  Ф3 creates it. Study notes are **committed** and live in
  `fractality/v0.1.0/spec/refs/notes/` (3 exist). Mapping of the
  owner's words: «/refs» = host `refs/` (downloads, never
  committed); «/refs/notes» = `spec/refs/notes/` (committed) — the
  established split from IGNITION Ф0.
- **Harness capabilities:** a deep-research skill (multi-agent
  fan-out + verification + cited report) is available for Wave 1;
  plain WebSearch/WebFetch tools for Wave 2. GitVerse over https
  hangs from this box; **GitHub answers fine** — all clones from
  GitHub. Glob tool proved unreliable on this tree today — recipes
  below use PowerShell/git listings.
- **Standing laws that bind this stage:** the clean-room register
  rules (license verdict BEFORE deep study; notes record decisions,
  never text/code shapes; implementation sessions open the note,
  not the source), the delegation law + live-observation protocol,
  the specmap drift law (anchored spec edits re-mint
  `specmap.json` in the same commit), machine quirks (editor-tool
  edits only; commit via `git commit -F - <<'MSG'`).
- **Context this research serves:**
  [`VISION-RECURSIVE-FABRIC.md`](../VISION-RECURSIVE-FABRIC.md) —
  V2 (the whole RLM procedure) plus the §5 open questions; the
  INITIATIVE §15 deferrals; PP-002 (credibility facts) as a
  candidate C3 companion.

## 4. Decisions {#decisions}

### D-R1 — a new staged plan; Stage B is the exit deliverable {#d-r1}

**Decision:** the research is its own plan (this document, Campaign
3 · Stage A). The Campaign 3 implementation plan
(`FRACTALITY-RLM-PLAN-v0.1.md`, Stage B) is *authored by Ф5* from
the synthesis — that is how the owner's «план Campaign 3 нужно
будет улучшить нашими идеями» lands, given no such plan exists yet.
**Rejected:** embedding the research as Stage B's Phase 0 — the
Phase-0-no-commits law conflicts with committed study notes, and
implementation decisions written before the study would violate the
clean-room order (decisions flow FROM notes). **Revisit:** if Ф5
finds the field too thin to justify a full campaign, Stage B may
shrink to a slice inside another campaign — owner call at RP-R3.

### D-R2 — three waves, first two independent {#d-r2}

**Decision:** Wave 1 = the deep-research harness; Wave 2 = plain
web search executed directly, **without reading Wave 1's output
first**; Wave 3 = merge. Independence is the point: two different
methods surface different corners, and their overlap is itself a
measurement (P-R2). **Rejected:** a single deep-research pass
(cheaper, but self-confirming — no second modality to catch what
the harness misses); seeding W2 queries from W1 results (kills
independence). **Revisit:** never — this is the owner's protocol
verbatim.

### D-R3 — what "most important" means (selection criteria) {#d-r3}

**Decision:** rank candidates per category by, in order:

1. **Mechanism relevance** — recursion is load-bearing: the system
   recursively spawns sub-agents / sub-LM-calls over tasks or
   context. Generic agent frameworks qualify only if recursive
   delegation is a core, documented mechanism.
2. **Idea density for our fabric** — what transfers to
   mission-control + pods + packets (descent, escalation, advisors,
   promotion, budgets/depth, context paging).
3. **Influence** — citations, stars, discourse footprint, who
   builds on it.
4. **Recency-adjusted** — it is mid-2026; the RLM wave dates late
   2025. Follow-ups, reimplementations, and critiques count.

Repos additionally carry a license check before study (INVENTORY
rule 1). The articles list **must include at least one strong
counterpoint** (anti-recursive/anti-multi-agent argument) — the
two-way-gap law of flow:comparative-research applied to selection.
**Rejected:** ranking by stars/citations alone (popularity is not
mechanism relevance).

### D-R4 — the intake pipeline (clean-room, verbatim law) {#d-r4}

**Decision:** every adopted source passes, in order: (1) an
INVENTORY row with source, local path, license verdict, class —
**before any study deeper than LICENSE + README**; (2) download
into the gitignored host tree — repos → `refs/src/<name>/` (clone,
record HEAD sha as pin), papers → `refs/papers/`, articles →
`refs/articles/<slug>.md` snapshot with URL + access date; (3)
study; (4) a committed note in `spec/refs/notes/`. The owner's
clean-room order, operative verbatim: «мы именно вычленяем и
понимаем идеи, мы не должны копировать код. … нужно понять его
смысл и реализовать Clean Room Implementation. … копирование кода
приведет к юридическим последствиям». Methods described in papers
are implementable freely; reference *code* is inspiration-only
regardless of its license; article text is cite-only. **Never:**
port lines, adapt file-by-file, or paste source text into notes.

### D-R5 — tiered study depth + delegation posture {#d-r5}

**Decision:** three tiers. **T1 (deep, boss-only):** the anchor
project — S4 paper + S3 repo + the author's blog post → one note
(`rlm-study.md`). **T2 (medium):** the remaining top-5 papers and
top-5 repos → one note per project. **T3 (survey):** the articles
and any runner-up worth a paragraph → grouped notes allowed. Per
the delegation law, T2/T3 *first-pass surveys* SHOULD be delegated
to GLM over sandboxed copies (S2 precedent: survey delegated, boss
spot-checks load-bearing claims) under the live-observation
protocol, inputs under the launch cwd, `.git` stripped from copies.
**The boss authors every note** — notes are decisions, not
summaries; delegation produces raw material only. **Rejected:**
boss-reads-everything (misappropriates boss tokens — the law);
delegate-writes-notes (a delegate cannot decide what fractality
adopts).

### D-R6 — note and synthesis formats {#d-r6}

**Decision:** per-source notes follow the INVENTORY house form —
*what the source achieves, which decisions we take* (and which we
explicitly do not), no text/code shapes from repos; short dated
verbatim quotes are allowed from papers/articles (cite, never
copy into product surfaces). The closing synthesis
(`notes/RLM-SYNTHESIS.md`) follows flow:comparative-research:
quote-first, **two-way gaps** (where the field is ahead of the
fabric / where the fabric is ahead of the field), **numbered
deltas** `RD-1..N` each with priority and target home (Stage B
phase, VISION section, or a PROP anchor), and a **re-fetch list**
(every URL + access date + subject version). The synthesis
proposes; acceptance happens in Stage B (deltas, not decrees).

### D-R7 — one project, one note {#d-r7}

**Decision:** a paper + its code + its blog post are **one project
→ one note** (the owner's «по каждому проекту или документу»
resolved toward projects; S3+S4+blog → `rlm-study.md` is the
worked example). Standalone documents get their own notes.
**Rejected:** one note per artifact — triples the files, splits
one project's decisions across three places.

## 5. The three waves — frozen protocol {#waves}

### Wave 1 — deep research {#wave-1}

Fire the deep-research harness once, with this question (frozen;
edits to it are a plan change):

> Recursive Language Models (RLM) and RLM-like recursive agent
> systems — systems where a language model or agent recursively
> spawns sub-LM calls or sub-agents over context or tasks. Anchor,
> already known: "Recursive Language Models" by Alex L. Zhang and
> Omar Khattab (arXiv 2512.24601; github.com/alexzhang13/rlm; the
> author's blog post). It is mid-2026: include follow-ups,
> reimplementations, critiques, successors. Deliver three ranked
> lists with per-item justification and access-dated URLs: (1) the
> 5 most important GitHub repositories where recursive spawning of
> agents/LM-calls is a core mechanism — reference implementations
> and toolkits, not generic agent frameworks unless recursion is
> load-bearing — each with its license; (2) the 5 most important
> scientific papers on recursive LM inference/decomposition
> (recursive delegation, divide-and-conquer over long context or
> tasks, context folding/paging); (3) the 5 best practitioner
> articles/blog posts, including at least one strong counterpoint
> to recursive/multi-agent designs. For every item: which ideas
> transfer to a mission-control + pod agent fabric (recursive boss
> promotion, escalation, advisor calls, context descent, budgets).

Output lands raw in `refs/study/rlm-waves/wave1-deep-research.md`
(gitignored) — kept verbatim for the merge audit.

### Wave 2 — plain web search, independent {#wave-2}

Executed directly with the WebSearch tool, **before reading Wave
1's output**. The query battery (frozen; ~top-10 organic results
per query, candidates recorded with URL and date; fetching beyond
snippets only to disambiguate a candidate):

```
1.  "recursive language models"
2.  RLM Zhang Khattab arxiv recursive
3.  github recursive LLM agent spawns sub-agents recursion
4.  recursive agent task decomposition paper 2025 2026
5.  ReDel recursive multi-agent delegation
6.  LLM context folding paging virtual context paper
7.  "chain of agents" OR "LLM x MapReduce" long context
8.  anthropic multi-agent research system orchestrator
9.  don't build multi-agents context engineering critique
10. recursive language model reimplementation OR benchmark 2026
11. best articles recursive LLM agents delegation 2026
12. site:github.com recursive LLM REPL context environment
```

Output lands raw in `refs/study/rlm-waves/wave2-web-search.md`
(gitignored): per-query candidate rows, then one independent
candidate table (category, URL, one-line why).

### Wave 3 — the merge {#wave-3}

Union W1+W2; dedup by canonical URL/repo; score against D-R3;
produce the **5/5/5 verdict table** (item, category, score
rationale one paragraph each, license for repos, W1/W2/both
provenance) plus the **runners-up ledger** (every rejected
candidate with its why-not, so the cut is auditable). Deliverable:
[`spec/refs/notes/rlm-source-selection.md`](../refs/notes/) —
committed; it doubles as the re-fetch list seed. Overlap metric
recorded for P-R2.

## 6. Phases and gates {#phases}

Every phase boundary is a safe stop; the gate panel for a docs-only
phase is "specmap green + the named artifacts committed".

- **Ф0 — plan lands (this session).** This document + the drafted
  dashboard in `reports/`, specmap re-minted. **Gate: RP-R1 OPEN —
  execution stops here until the owner's word.**
- **Ф1 — the waves.** Fire Wave 1 (background); execute Wave 2
  meanwhile (independence per D-R2); raw outputs to
  `refs/study/rlm-waves/`. No commits (gitignored artifacts only).
- **Ф2 — the merge.** Wave 3 → `rlm-source-selection.md` +
  INVENTORY rows (license verdicts) for every adopted source.
  One commit. **Gate: RP-R2 — the 5/5/5 table shown to the owner
  in chat; objections rearrange the cut before heavy study.**
- **Ф3 — intake.** Clones @recorded pins into `refs/src/`, PDFs
  into `refs/papers/`, access-dated article snapshots into
  `refs/articles/`; INVENTORY pins/paths finalized. One commit
  (INVENTORY only — downloads are gitignored).
- **Ф4 — study.** T1 first (`rlm-study.md`), then T2, then T3 per
  D-R5 (delegated surveys under the live-observation law; boss
  authors notes). Commits grouped per tier. **Gate: every adopted
  source is covered by a committed note.**
- **Ф5 — synthesis and exit.** `RLM-SYNTHESIS.md` (D-R6 form,
  RD-deltas numbered) → **author the Stage B draft**
  (`FRACTALITY-RLM-PLAN-v0.1.md`) seeded by the deltas + VISION
  §V2/§5 + INITIATIVE §15 leftovers → close this plan (prediction
  verdicts, ledger, WAL, completed-plan dashboard). **Gate: RP-R3 —
  Stage B execution is a separate owner decision; drafting it is
  this plan's last act.**

## 7. Predictions (checked at close) {#predictions}

- **P-R1** — the waves jointly surface ≥ 12 qualifying repos and
  ≥ 12 qualifying papers before the cut. (Falsifier: the field is
  thinner than the vision assumes — itself a Stage-B-shaping fact.)
- **P-R2** — W1∩W2 agreement on the final 5/5/5 is ≥ 60% but
  < 100%. (Falsifier low: one method would have sufficed — the
  3-wave design was overkill; falsifier high: the methods are
  redundant. Either way, recorded for the next research stage.)
- **P-R3** — after study, the Zhang RLM project (S3+S4+blog)
  remains the single most load-bearing source for Stage B. 
  (Falsifier: something supersedes it — a headline finding.)
- **P-R4** — the counterpoint article generates at least one
  RD-delta (a constraint we adopt). (Falsifier: the critique
  teaches us nothing — suspicious; re-read it.)

## 8. Baseline and exit state {#baseline}

**Baseline (2026-07-10):** INVENTORY 9 rows (S1–S9); S3/S4 status
"study deferred to Campaign 3"; `spec/refs/notes/`: 3 notes;
`refs/src/`: 11 dirs; `refs/articles/`: absent; Campaign 3 plan:
none; VISION §V2 open questions: unanswered.

**Exit:** INVENTORY carries a row for every adopted source (up to
+13 rows; S3/S4 statuses flipped to studied); `refs/` holds all 15
adopted sources locally; `spec/refs/notes/` holds one note per
adopted project (D-R7) + `rlm-source-selection.md` +
`RLM-SYNTHESIS.md` with numbered RD-deltas;
`FRACTALITY-RLM-PLAN-v0.1.md` exists as a draft; this plan carries
the close record and prediction verdicts; WAL and dashboards
current.

## 9. Risks and fallbacks {#risks}

- **Deep-research harness unavailable/degraded** → fallback: Wave 1
  as a manual multi-query fan-out with verification passes;
  recorded as a deviation in the ledger.
- **Link rot / paywalls** → snapshots at first touch (Ф3);
  paywalled papers studied at abstract+blog level and flagged in
  the note; the re-fetch list keeps access dates.
- **License surprises** (a top-5 repo is GPL/AGPL or unlicensed) →
  no crisis: class stays inspiration-only (rule 4), code is never
  adopted from ANY source (D-R4); the license column still gets
  filled for the record.
- **arXiv id sanity** — INVENTORY pins S4 as 2512.24601; verify the
  live id/version at Ф3 and correct the row if the record drifted.
- **Wave overlap too high** (searches collapse onto the same 10
  URLs) → widen W2 with category-specific follow-ups (allowed:
  *adding* queries is not a plan change; *editing* frozen ones is).
- **Volume blowout** (15 sources × deep study exceeds sessions) →
  the tiers ARE the fallback; T3 depth may shrink to grouped
  paragraphs, never below "every source has a recorded verdict".
- **Delegate derailment** on T2/T3 surveys → the live-observation
  law (heartbeats, watcher, kill-correct-relaunch); inputs under
  launch cwd; two cwd strikes already on record — pin cwd in every
  command.
- **Tool quirk** — Glob is unreliable on this tree; recipes use
  PowerShell/git listings.

## 10. Review points {#review-points}

- **RP-R1 — plan review (OPEN).** Owner order verbatim: «Не
  исполняй план, вначале дай посмотреть на него». Nothing past Ф0
  executes until the owner rules here. Ruling recorded verbatim on
  resolution.
- **RP-R2 — the 5/5/5 cut (opens at Ф2).** The verdict table is
  shown in chat; the owner may swap items before heavy study. T1
  study never waits (the owner already named the anchor project
  primary in the mandate).
- **RP-R3 — Stage B launch (opens at Ф5).** Drafting the Stage B
  plan is in scope; executing it is not — separate owner word.

## 11. Quick-start for the executing session {#quick-start}

```sh
# From the workspace root (cwd law — pin it in the command):
cd /c/Users/olegc/gits/vibevm/packages/org.vibevm.fractality

# State: this plan §6 + the ledger §12; then:
#   Ф1: fire the deep-research skill with §5.1's frozen question;
#       run §5.2's frozen queries via WebSearch; raw → refs/study/rlm-waves/
#   Ф2: merge → fractality/v0.1.0/spec/refs/notes/rlm-source-selection.md
#       + INVENTORY rows; commit; show the table (RP-R2)
#   Ф3 clone form (record the pin):
git clone --depth 1 <url> /c/Users/olegc/gits/vibevm/refs/src/<name> \
  && git -C /c/Users/olegc/gits/vibevm/refs/src/<name> rev-parse HEAD
#   Ф4: T1 note first: fractality/v0.1.0/spec/refs/notes/rlm-study.md
#   Ф5: RLM-SYNTHESIS.md → spec/plans/FRACTALITY-RLM-PLAN-v0.1.md (draft)

# Specmap re-mint rides every commit that adds anchored spec files:
cd fractality/v0.1.0 && /c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe specmap
```

## 12. Whole-stage acceptance {#acceptance}

At close, all of the following hold: (1) `rlm-source-selection.md`
exists with 5/5/5 + runners-up + provenance + overlap metric; (2)
every adopted source has an INVENTORY row with pin + license
verdict recorded **before** its deep study began (git history
proves order); (3) all adopted sources present under `refs/`; (4)
one committed note per adopted project; (5) `RLM-SYNTHESIS.md`
carries numbered RD-deltas each with priority + target home + the
re-fetch list; (6) `FRACTALITY-RLM-PLAN-v0.1.md` draft exists and
cites the deltas it adopts; (7) P-R1…P-R4 carry verdicts; (8) zero
product-code changes in the whole stage; (9) specmap green at every
phase boundary.

## 13. Execution ledger {#ledger}

_(filled per phase; empty at draft)_

| phase | commits | confirmed / falsified | notes |
|---|---|---|---|
| Ф0 | — | — | plan drafted; RP-R1 OPEN |

## 14. Log {#log}

- 2026-07-10 22:15 — drafted in-session from the owner's three
  messages (mandate, plan order, execution gate); RP-R1 OPEN;
  dashboard `reports/2026-10-07-22-15-rlmresearch-drafted-plan.md`.
