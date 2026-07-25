# Spec-Actualization Campaign v0.1 — mark, verify, and de-drift the whole spec tree {#root}

<status stage="spec" state="done" action="continue" actionstage="impl" comment="plan in execution: A, B, and L closed (L relocated the legacy dirs to legacy-spec/ 2026-07-25); Phase C (verification) awaits the owner's opening call"/>

**status: AUTHORED 2026-07-24 · IN FLIGHT — Phase B CLOSED 2026-07-25 (58 files, 4 880/4 880 facts marked) · Phase L CLOSED 2026-07-25 (terraforms/research/neworder/discipline relocated to root `legacy-spec/`; corpus + crates reference-free modulo this plan's own carve-out; `check --exhaustive` clean + floor green) · next: Phase C (verification) awaits the owner's opening call · vibevm-specific · first consumer of PROP-043 (Progress Control)**

Contract for everything used here: [PROP-043](../modules/vibe-progress/PROP-043-progress-markup.md).
Owner's manual: [OWNER-GUIDE](../modules/vibe-progress/OWNER-GUIDE.md).
Task formats: [templates/](../modules/vibe-progress/templates/impl-task.md).

---

## 0. Mandate (owner's words, 2026-07-24, recorded verbatim) {#mandate}

- «актуализировать ВСЕ спецификации … Это чудовищная огромная работа. Именно
  поэтому я и готовлю scaffold для нее, чтобы не сбиться в ходе обхода
  настолько большого количества документов. Нам нужно разметить корпус
  фактов, которые дальше нужно будет проверять.» Work may take **a month**;
  that is accepted; quality over speed.
- Paragraph-level exhaustiveness is **the point**, not an option: «это
  in-verbatim контроль того, что мы прошли всё, каждую строчку. LLM очень
  любит упрощать … нужен алгоритмический надсмотрщик».
- **No fractality for this campaign.** «Я хочу чтобы Fable сделала максимум
  высокоуровневых задач (анализ и разметку спецификации и тп)» — outputs:
  (a) a corpus of coding tasks for **Opus**, (b) a corpus of spec-improvement
  tasks executed by **whatever model the budget allows** (Fable if it
  stretches, Opus otherwise). This deliberately overrides the standing
  delegation-first default for the duration of the campaign — owner decision;
  do not "optimize" it back.
- Stitching is **non-linear**: reworking B may reopen A and vice versa —
  plan it as a multi-pass fixpoint, not a single sweep.
- Crash-safety: any session may die (budget, power); the next session must
  resume from **one obvious file** with at most one step lost.
- Repeatability: re-runs at ~monthly cadence must cost O(delta), not
  O(corpus).
- Wave 1 = the host `spec/` tree only; `packages/` waves later; the
  fractality specspace excluded until the owner says otherwise.

## 1. Baseline (verified at authoring time, 2026-07-24) {#baseline}

- Host `spec/`: **91 md files, 26 699 lines**. Authored packages (no
  vendored copies): world 154 + ai-native 140 files (~30k lines) — wave 2;
  fractality 700 files — out of scope.
- Free-form `**Status:**` lines to convert mechanically: **~55**.
- specmap: index live; **34 gated orphans** in `vibe-spec` (pre-existing).
- Progress Control: does not exist yet — no crate, no `vibe progress`, no
  `campaigns/` zone, no dashboard. `<status` appears nowhere in the tree
  except PROP-043's own dogfood markers.
- Existing inline grammars that must not collide: `@spec://` (~17 uses),
  `#use`/`#embed`/`#source`, `<!-- REVIEW: -->`.

## 2. Executors and the budget law {#executors}

| Role | Who | What |
|---|---|---|
| Boss / high-level | **Fable** | markup passes, verification judgment, stitching, task authoring, ALL review |
| Coder | **Opus** | IMPL tasks (DRIFT-NNN) exactly as written; stop-rule on ambiguity |
| Spec editor | **budget-dependent** | SPEC tasks (SPEC-NNN): Fable if budget allows, else Opus; Fable reviews regardless |

Rules 1–4 of the repository bind every executor. Worker output is never
credited; commits are human-authored surface; non-routine red lines stop for
the owner no matter who is executing.

## 3. Campaign zone layout {#layout}

```
campaigns/progress-2026-08/        # id fixed at Phase A close
  baseline.json                    # inter-campaign contract (PROP-043 §7.3)
  deferrals.md                     # open tails at close-out; next run drains it
  harvest/                         # doc cards (templates/harvest-card.md)
  tasks/                           # DRIFT-NNN.md / SPEC-NNN.md + INDEX.md
  run/                             # EPHEMERAL: journal.jsonl · state/*.json · RESUME.md · mirror/
```

Excluded from markup scope, packaging, and registries (PROP-043 §7.4).
Committed at batch boundaries — journal in the same commit as the edits it
describes; fan-out via `cargo xtask mirror` at phase checkpoints. `run/` of a
closed campaign may be archived or deleted; the other four entries persist.

## 4. Resume protocol (crash-safety law) {#resume}

1. **Step = unit of atomicity**: mark-file · verify-unit · close-obligation ·
   execute-task. Journal writes `step-start` (intent, actor) before work and
   `step-done` (result ref) after; JSONL, append-only, torn tail discarded.
2. **Recovery rule:** step closed in journal ⇒ its edits stand; step open ⇒
   `git restore` its files and redo the step. Steps are idempotent by
   construction. Maximum loss on any crash = one step.
3. **`RESUME.md` is generated** (`vibe progress resume`) after every
   step-done: where we are · unresolved steps with literal recovery commands ·
   next steps · phase rules pointer · dashboard command. Every session of
   this campaign **starts by reading it** and ends by closing (not starting)
   a step.
4. **Claims and staleness:** journal actors (`fable`, `opus:DRIFT-012`);
   an in-progress task with no journal events past the threshold is returned
   to `queued` as stale by `resume`.
5. git = second echelon: batch commits make the worst disk-loss cost one
   batch, never the campaign.

## 5. Phases {#phases}

Each phase: entry condition → steps → exit gate (+ prediction, per the
campaign-plan discipline). Every session inside any phase obeys §4.

### Phase A — Scaffold {#phase-a}

*Entry:* this plan + PROP-043 exist. *Steps:*

1. Owner ratifies PROP-043 (or amends; amendments land before code).
2. Build the core crate + `vibe progress` adapter: scan / check
   (`--exhaustive`) / report (views, audiences) / mirror / weave
   (`--digest`, `--max-tokens`) / rescan / resume. Fixtures include the
   foreign-grammar non-collision corpus.
3. Create `campaigns/progress-2026-08/` skeleton + journal/state schemas.
4. Dashboard: `tools/progress-dashboard/serve.mjs` — zero-dependency
   `node:http`, one vanilla page, poll `run/state/`, read-only. Screens:
   Resume · Overview · Corpus · Stitching · Tasks.
5. Pilot: hand-mark 2–3 documents of different genres (one PROP, one
   terraform plan, one design doc); run the full loop scan→check→report→
   mirror→weave on the pilot.

*Exit gate:* `self-check` green with the new crate; `check --exhaustive`
correct on pilot (0 unmarked); dashboard renders pilot state; RESUME.md
generates. *Prediction:* the pilot exposes ≤ a handful of grammar/placement
ambiguities — they amend PROP-043 §3 before Phase B, after which the grammar
holds for the whole wave without further amendment.

### Phase B — Markup (facts pass) {#phase-b}

*Entry:* A closed. *Executor:* Fable. *Steps:*

- B0: mechanical conversion of ~55 `**Status:**` lines into document
  markers (script-assisted, reviewed as one diff).
- B1…Bn: file batches (~8–12 files each). Per file: paragraph-exhaustive
  markers; sense-preserving re-splits of under-granular paragraphs; missing
  `{#anchor}`s added; `audience` where obvious; cross-doc findings recorded
  into the ledger **in passing** (first stitching input is free).
- Semantic edits are FORBIDDEN in this phase — a semantic problem found
  becomes a ledger finding, not an edit.

*Exit gate:* `check --exhaustive` green over the whole wave-1 scope; mirror
populated; batch diffs contain markers/splits/anchors only. *Prediction:*
91 files ≈ 9–12 batches; the unmarked counter is what catches skipped
paragraphs, not reviewer attention (expect ≥1 real catch).

### Phase L — Legacy relocation (owner amendment, 2026-07-25) {#phase-l}

*Entry:* B closed. *Ordering law:* L completes **before Phase C opens** —
verification must cover the relocated facts (owner: «Это нужно сделать до
фазы верификации, чтобы верификация проверила еще и эти перенесенные
факты»). *Executor:* Fable (inventory, fact moves, markup); Opus only for
mechanical bulk the §2 calculus clears. *Mandate (owner, 2026-07-25,
verbatim):* «После фазы разметки я хочу добавить еще одну фазу: чистка
terraforms/neworder/discipline/research. Я хочу в итоге убрать их из
дерева spec и переместить в новую директорию в корне: legacy-spec. Но для
этого нужно, чтобы на них перестали ссылаться.» Steps, in the owner's
order:

- L1 — reference inventory: every reference into `spec/terraforms/**`,
  `spec/research/**`, `spec/neworder/**`, `spec/discipline/**` from (a)
  the living corpus (`common/` `design/` `modules/` `manual-tests/`
  `boot/`) and (b) code — specmark scopes / `#[spec(...)]` / `spec://`
  URIs in crates, doctests included. Cross-references *between* the four
  legacy dirs don't count — they relocate together.
- L2 — fact discovery: for each reference, identify the fact(s) the
  referrer actually cites at the target.
- L3 — fact relocation: move those facts into the main corpus
  (`common/design/modules/manual-tests`). Creating new specs is allowed
  where no natural home exists (owner grant, 2026-07-25: «Если при
  переносе фактов из устаревших директорий … придется создать какие-то
  новые спецификации - создавай, это не проблема»).
- L4 — markup: every relocated fact gets fact-grain markup (`##anchor` +
  marker) at its new home; new files enter `progress.toml` scope;
  referrers repoint to the new anchors.
- L5 — relocation: when zero live references remain, `git mv` the four
  directories to `legacy-spec/` at the repo root.

Note *(superseded the same day — see the LOG)*: the original amendment
kept `spec/discipline/README.md` in the Phase B markup scope. The
owner's second 2026-07-25 ruling overrode that: **discipline is out of
the analysed corpus entirely** («теперь Дисциплина — это часть пакетов
ai-native»); `progress.toml` dropped its glob, and the directory awaits
Phase L's reference inventory + relocation like the other three.

*Exit gate:* grep-verified zero references from the living corpus and
crates into the four directories; `check --exhaustive` green over the
(possibly grown) scope; floor green — specmap included, repointed scopes
must resolve. *Review point (RESOLVED, owner, 2026-07-25):* this plan
file **stays in `spec/terraforms/` for now** — «Я передумал. План этой
кампании пока переносить не нужно. Я хочу чтобы у нас остался правильный
набор спецификаций и других инструкций, чтобы мы могли делать
периодические проверки каждый месяц» — the §6 monthly recurrence needs
the plan and its instruction set in place. **L5 therefore excludes this
plan file** from the `spec/terraforms/` move; its eventual home is a
future owner call, no earlier than campaign close-out.

### Phase C — Verification (evidence pass) {#phase-c}

*Entry:* B **and L** closed (owner amendment 2026-07-25; per-cluster
start allowed once a cluster's files are marked and untouched by L). *Executor:* Fable + machine evidence. *Steps:* every marker gets a
verdict in the cache — `confirmed` / `drift` / `unverifiable`:

- machine first: specmap join (implements/verifies), targeted greps, CLI
  `--help` snapshots, manifest checks, test presence;
- Fable judgment where machines are silent; verdict without an evidence ref
  is rejected by `check` (honesty is enforced: not found ⇒ `unverifiable`,
  never "probably done");
- verification runs (`command → real output`) are saved as doc fixtures;
  harvest cards written while knowledge is hot.

*Exit gate:* 100 % of markers carry verdicts; the X/Y/Z summary is recorded
in the LOG — the first measured actuality level of the spec tree.
*Prediction:* drift concentrates in DRAFT/proposed PROPs and terraform plans
whose `**Status:**` promised more than the tree delivers; IMPLEMENTED-marked
units mostly confirm.

### Phase D — Stitching (fixpoint over the ledger) {#phase-d}

*Entry:* C verdicts exist for the cluster. *Executor:* per §2 budget law.
*Mechanics:*

- Obligation types: `contradiction` · `duplication` · `missing-support` ·
  `terminology` · `relocation` · `reality-mismatch`.
- **Waves:** wave N = SPEC tasks over all docs with open incoming
  obligations; closures may open new obligations → wave N+1. Convergence =
  empty ledger; a wave with zero new findings = converged (loop-until-dry).
- **Escalation rule:** a doc pair whose open-obligation count fails to fall
  for two consecutive waves is a conceptual conflict → owner decision;
  iteration on that pair stops.
- Clusters: registry (001/002/008/010/021/023/030) · workspace/boot
  (007/009/011/012/020/022/025/034/035/038) · resolver (003/017) · cli/tui
  (036/037/039/040/041/042) · common/plans/design/research.
- `reality-mismatch` resolves via the sync-from-code flow (owner approves
  spec diffs); `remove` verdicts execute here (delete or demote to
  idea-archive).

*Exit gate:* ledger empty (or every survivor is an owner-ruled deferral);
markers of all touched units updated. *Prediction:* obligations per wave
fall roughly geometrically; ≤3 waves for wave-1 scope; ≤2 owner
escalations.

### Phase E — Coding (drift-correction by tasks) {#phase-e}

*Entry:* per IMPL task — **unit stability**: every anchor the task cites has
no open obligation and no `unknown` marker (clusters release independently;
Opus never codes against a moving spec). *Steps:*

- Fable authors DRIFT-NNN tasks from `drift`+`continue` verdicts, priority:
  user-facing broken promises → internal mechanics → polish.
- Opus executes exactly per template (stop-rule on any ambiguity); Fable
  reviews against §6-acceptance verbatim; markers updated on completion
  (`impl/work → impl/done → test/plan`…); specmap tags on new code shrink
  the orphan count.
- `rework` items: feature-flag disable decision (cargo feature vs runtime
  gate) is recorded as a decision record at this phase's start, then
  executed per item.

*Exit gate:* task queue drained or explicitly deferred; floor green;
`report --view todo` matches the deferrals file exactly. *Prediction:* ≥80 %
of DRIFT tasks land without a `returned` round-trip — the template carries
enough context; `returned` clusters indicate spec gaps, feeding D-waves.

### Phase F — Plans and fold {#phase-f}

Three owner plans generated from views: **release/productization**
(freeze-candidates → showable), **improvement** (rework + disabled),
**global idea ledger** (idea/hold). Marker density folds: agreeing sections
collapse to unit markers (lossless, `check`-verified). `vibe progress check`
enters the standing gate panel. *Exit:* owner accepts the three plans.

### Phase G — Documentation {#phase-g}

Two trees written **from proven behavior** (harvest cards + captured runs),
never from spec prose: **User Guide** (audience=user) and **Package Author
Guide** (audience=author). Chapters release as their features stabilize
(pipeline with E, not a barrier). Each page carries `documents:
spec://…#anchor` metadata → doc-coverage becomes a ratchet. Owner reads for
register and truth. *Exit:* every `--view doc` row is either published or
explicitly deferred; doc-coverage ratchet armed.

### Close-out {#closeout}

`baseline.json` written; open tails → `deferrals.md`; REPORT section below
filled against every prediction; `run/` archived; WAL updated; version tag
proposed to the owner.

## 6. Recurrence (the monthly re-run) {#recurrence}

`vibe progress rescan --baseline <prev>/baseline.json` → new / suspect /
carried-forward lists → mini-B on new+changed → re-verify suspect (+ random
control of carried-forward) → mini-D on new findings → tasks → new baseline.
Cost O(delta). Between runs, the maintenance discipline (PROP-043 §10) and a
health-audit category ("markers vs reality") keep the delta small. This plan
is the standing playbook for those runs; each run appends its own LOG entry
and rewrites `baseline.json`.

## 7. Dashboard contract {#dashboard}

Reads `run/state/*.json` only (never Markdown, never computes). Zero npm
dependencies. Screens: **Resume** (open steps red, next steps, freshness
plaque), **Overview** (phase lane, counters), **Corpus** (tree colored by
rollup, five views + audience filter), **Stitching** (ledger table +
open-obligations-per-wave chart; non-falling pairs highlighted as
escalations), **Tasks** (both queues, statuses, claim owners). Localhost,
read-only, no auth.

## 8. Predictions (falsifiable, campaign-wide) {#predictions}

1. Wave-1 full weave fits ≤2 shards of a 1M window (digest fits trivially).
2. The exhaustive counter catches ≥1 genuinely skipped paragraph that
   review alone would have missed.
3. ≥60 % of `**Status:** IMPLEMENTED/SHIPPED` claims confirm without drift;
   ≤10 % of all units end `unverifiable`.
4. Stitching converges in ≤3 waves with ≤2 owner escalations.
5. ≥80 % of DRIFT tasks land without a returned round-trip.
6. The month budget holds: A ≈ days, B ≈ 1–1.5 weeks, C ≈ 1 week, D ≈ 3–5
   days, E ≈ open-ended by queue, F+G ≈ 1 week overlapping E.

## 9. LOG (execution ledger — append per batch/wave/phase) {#log}

- **2026-07-24 · Phase A CLOSED, exit gate green in full.** Commits
  `9446a2c` `b1276c3` `edd487b` (document package) · `8b18152` (core +
  adapter) · `38855c0` (campaign zone + dashboard) · `ac97f26` (pilot +
  ratification + §3.8 amendment) · `9a69b6f` (WAL). PROP-043 RATIFIED
  in session. Pilot: 3 genres, 46/46 paragraphs marked, one real drift
  caught (design/README index was incomplete — fixed). Predictions:
  "pilot exposes placement ambiguities" CONFIRMED (the preamble-less-H1
  amendment); bonus finding outside any prediction — a live power cut
  exposed missing fsync-before-rename in `write_atomic`; fixed with
  tolerant cache load + tests. Floor (`self-check`) green;
  `progress-core` gated in conform.
- **2026-07-24 · B0 default ruled (Fable, owner-visible in session):**
  the converted `**Status:**` lines are NOT deleted — the document
  marker is ADDED and the legacy line stays (its dates and prose are
  history; deduplication is Phase D material). Phase B makes no
  semantic edits, per its own law.
- **2026-07-24 · Phase B OPENED; B0 landed.** `progress.toml` (wave-1
  scope `spec/**/*.md`) committed as the campaign's first journal step
  (`8d5ccc8`); scan on the narrowed scope: 97 files (§1 counted 91 —
  the six newcomers are the progress-control documents authored after
  the baseline froze), 3 684 paragraphs, 46 pre-marked by the pilot.
  B0 converted **73** legacy status lines (not ~55 — the §1 estimate
  missed the `**status:` / `**Status.**` variants) into document
  markers as one reviewed diff: 73 files, +73 marker lines, 0
  deletions (the B0 ruling above held — legacy lines stay). `check`
  clean; markers 68 → 141. First stitching input recorded in passing:
  **12 ledger findings** (F-001…F-012, `run/state/findings.json`) —
  status lines contradicted by executed plans or shipped crates
  (SELF-SUFFICIENCY / SETTINGS-HOME / CONVERT / CULTURAL-EXTRACTION
  plans; the PROP-019/024/030/036/037/040 family; two missing
  superseded notices: TREE-TUI v0.1, PROP-026).
- **Scope question for the owner (found in B0, parked):**
  `spec/boot/STATIC.md` and `spec/boot/INDEX.md` are *generated by
  vibe* («do not edit») yet sit inside the wave-1 scope
  `spec/**/*.md`. Markup written into them dies on the next
  `vibe install` regeneration. Options: exclude generated boot
  artifacts from `progress.toml`, or carry their markers in the
  generators. Until ruled, B1 batches skip both files — which keeps
  `check --exhaustive` red on them, so the Phase B exit gate needs
  this ruling.
- **2026-07-24 · B1 (spec/common, paragraph grammar) landed — then the
  grain changed.** All 12 `spec/common` files marked
  paragraph-exhaustively (389 paragraphs; PROP-000 got its document
  marker + the missing `{#invariants}` anchor; open-questions sections
  marked `spec/work`), `check` clean, commit `91274c8`. Mid-batch a
  real power cut killed the session; §4 recovery worked as designed —
  journal showed one open step (`b1-prop-000`), rescan verified its
  edits clean, the step closed without redo. In passing: PROP-000 §3
  still describes the pre-2026-07-12 proprietary-EULA posture
  (F-014-to-be), PROP-018's MVP is implemented while its line says
  proposed (F-013-to-be), and the cache does not prune records that
  fall out of scope after `progress.toml` narrowing — corpus.json
  carries 497 entries vs 97 observed, dashboard counters inflated
  (DRIFT-001-to-be).
- **2026-07-24 · FACT-GRAIN DIRECTIVE (owner, in session, verbatim):**
  «Каждый элемент такого списка по сути является самостоятельным
  фактом, и его тоже нужно размечать. В том числе, inline факты
  перечисленные внутри текста … Я хочу чтобы ты для всех текстов
  сделала полное деконструирование всех фактов, имеющихся в системе,
  совершенно всех текстов. … если какой-то абзац можно переписать в
  виде нумерованного или ненумерованного списка фактов и каждому из
  них поставить в соответствие разметку статуса — нужно сделать это. В
  идеале почти все тексты превращаются в какие-то списки … Это
  означает ЗНАЧИТЕЛЬНОЕ УСЛОЖНЕНИЕ кода Системы, инструкций Системы и
  рост количества фактов … Уже проиндексированное и размеченное нужно
  переиндексировать и переразметить». Clarifications the same session:
  list-item markers go at the item's start or end, `@` or XML notation
  both; **table cells are marked the same way**. Ratified into
  PROP-043 as the fact amendment (§3.8 items 4–6, §3.9, §8) — the
  campaign granularity drops from paragraph to **fact**; deconstruction
  of multi-fact prose into lists is now part of the markup pass
  (sense-preserving, wording-preserving; semantic edits still
  forbidden); everything already marked (pilot + B0 doc markers + B1
  common) is re-marked under the fact grammar.
- **2026-07-24 · Fact-anchor addenda (owner, same session):** (1)
  list-item markers sit at the item's start or end, `@` or XML notation
  both; (2) «таблицы тоже нужно в ячейках размечать аналогичным
  способом» — table body cells are units, marked in-cell; (3) list
  items get hashtag addressing — «Элементам списков стоит придумать
  адресацию с помощью хэштегов … формат типа `1. #RULE-001 Текст
  правила @freeze/done`», refined to **`##RULE-001`** (double hash) «чтобы
  их отличать» from the `#use`-family directives; (4) «каждый абзац,
  каждый элемент списка … который имеет статус, нужно разметить с
  помощью якоря» — the anchored-when-marked law: a marked unit without
  a `##<ID>` anchor is a `check` error. All four ratified into
  PROP-043 §3.8 / §8.
- **2026-07-24 · Fact-grain scanner LANDED; fact grammar re-piloted.**
  PROP-043 amendment ratified and committed (`cd2688f`); the scanner
  (`b67fa97`): list items at every nesting level, lead lines, table
  body cells as countable units, `##<ID>` fact anchors in one id space
  with heading anchors, duplicate detection, the anchored-when-marked
  error; 31 tests green, cache schema 2 (the rebuild also flushed the
  400 out-of-scope records — DRIFT-001 still files the general prune
  defect for Opus, `910d545`). Scale shift measured: wave-1 = 3 684
  paragraphs → **8 219 facts**; `check` currently carries **435
  expected MissingAnchor** errors on the pre-amendment markup (pilot +
  B1 common) — they burn down as files are re-marked. Re-pilot
  (`6714876`): PROP-029 deconstructed 9 paragraphs → 30 anchored
  facts, 0 unmarked, 0 issues — the demo the owner reviewed in
  session. Ledger: F-013 (PROP-018 MVP implemented vs proposed),
  F-014 (PROP-000 §3 license text predates the 2026-07-12 UPL-1.0
  relicense).
- **RESOLVED review point (owner, 2026-07-24) — anchor naming
  convention:** the re-pilot mixed `##UPPER-SLUG` for normative facts
  (ADDR-LAW, RULE-style) with `##kebab` for service units
  (status-line, carriers-lead). Keep both registers, or fix one?
  **Ruling: both registers stay** — UPPER = normative fact, kebab =
  service unit; the register itself carries the normativity signal.
  Recorded as a decision at PROP-043 §3.8.
- **RESOLVED review point (owner, 2026-07-24) — generated files in
  scope:** `spec/boot/STATIC.md` + `INDEX.md` are vibe-generated
  («do not edit») yet inside the wave-1 globs; markup written there
  dies on regeneration. Exclude them from `progress.toml`, or carry
  their markers in the generators? **Ruling: exclude from scope.**
  §4 of PROP-043 is include-only by design, so the exclusion is
  expressed by include enumeration (`spec/boot/[0-9]*.md` admits the
  authored snippets, skips the generated pair); cache rebuilt from
  scratch (the DRIFT-001 no-prune defect makes a stale-record purge
  otherwise impossible). Scope: 97 → **95 files**, 8 219 → **7 872
  facts**. This also closes the B0-parked scope question above.
- **2026-07-24 · B1f LANDED — spec/common is fact-grain clean; the two
  review points RESOLVED (above).** The scanner-handover step was closed
  retroactively (`b67fa97` verified committed; RESUME had prescribed a
  redo of finished work). All 11 remaining `spec/common` files re-marked
  under the fact grammar: **386 paragraph-grain units → 979 anchored
  facts** (batch commits `83bed35` / `4aed13f` / `d639bcf`; the batch-1
  message overstates its own counts — 109 units → 296 facts is the true
  figure, corrected here, history left unrewritten). Cluster total:
  **1 009 facts, 0 unmarked, 0 issues** — cluster MissingAnchor 386 → 0;
  the wave's residue is 40 expected errors in the two pilot files
  (SHRINK-PLAN 28, design/README 12), owned by their B2+ batches.
  Grammar traps found and recorded: blockquote units cannot carry
  `##` anchors (ledgered **F-015**; two units re-formed — a bold
  paragraph, a fenced template); a wrapped prose line whose continuation
  opens with `+ ` parses as a phantom list item (two fixed in PROP-019).
  Tasks queued for Opus: **DRIFT-002** (`parse.rs` 809 lines > 600
  budget — the standing floor is RED on this single new conform finding
  until it lands; the B1f batch commits were made against that known,
  ledgered red) and **DRIFT-003** (`campaign.json` phase hardcoded
  `"A"`, dashboard/RESUME render a stale phase). Prediction check in
  passing: the §5-B "unmarked counter catches what review misses"
  prediction keeps confirming — the counter caught both phantom-item
  wraps instantly.
- **2026-07-24 · DRIFT-002 executed and landed — the floor is green
  again.** First DRIFT task through the full loop: Opus executed the
  parse.rs split exactly per the task file (six `parse/` modules, max
  261 lines; motion, not rewrite), Fable reviewed and accepted (spot-diff
  verbatim, differential oracle — corpus identical modulo timestamp, 31
  tests green, conform 0 new, `self-check` all green with the real exit
  code). One accepted deviation: per-file `//!` module docs, the crate's
  own convention. B2 opened in parallel the same evening: templates +
  modules README + PROP-042/025 marked (F-016 structural index drift,
  F-017 code-ahead-of-spec `vibe aiui scrollbar`). The §5-E prediction
  ("≥80 % of DRIFT tasks land without a returned round-trip") starts
  1/1.
- **2026-07-24 · DRIFT-003 landed — the phase lane is honest; B2 runs
  through batch 5.** Second DRIFT task through the loop, again no
  returned round-trip (§5-E prediction 2/2): the phase is now an
  append-only journal event (`{"kind":"phase","value":"B"}`, last
  wins, absent ⇒ "A"), derived by the adapter — never compiled in,
  never parsed from Markdown; `read_journal` distinguishes a torn
  tail (incomplete JSON, stops) from an unknown kind (complete JSON,
  skipped) with both laws test-pinned; the live journal is
  backfilled and `campaign.json`/RESUME render **B**. B2 batches
  3–5 in parallel: PROP-026 (superseded-in-topology arc split
  impl/done vs spec/done per fact), PROP-021/023 (bridge pair),
  PROP-020/022 (F-018: hooks ship while the line says proposed),
  PROP-041 (owner-minted per-REQ `{#anchor}`s reused verbatim as
  `##` ids — one name, two notations; two heading-vs-REQ same-name
  collisions surfaced by the shared id space, F-019 for the stale
  DRAFT line). Ledger: 19 findings. B2 stands at 12/35 files.
- **RESOLVED review point (owner, 2026-07-24, in session) — the WAL
  and the boot files:** «spec/WAL.md не должен участвовать в проверке,
  это генерирующиеся артефакты; также проверь про файлы внутри
  spec/boot». **Ruling applied: `spec/WAL.md` is out of scope** — the
  checkpoint is rewritten wholesale at every session end, so markup
  cannot live in it (the same mortality argument as the generated
  boot pair). The boot check reported back: `STATIC.md`/`INDEX.md`
  are generated and were already excluded by the morning ruling;
  `00-core.md`/`90-user.md` are **authored, user-owned, never
  written by vibe** (PROP-000 invariant 3) — they stay observed.
  Scope: 95 → **94 files**; the `spec/*.md` include is gone (WAL was
  its only match), so a future top-level spec doc must be added to
  `progress.toml` explicitly.
- **2026-07-24 · Owner directive — the coder-tier engine version.** The
  §2 coder tier ("Opus") runs on the owner-designated engine
  `claude-opus-5` from the next session on. Verified: the id is live
  (one-shot CLI probe answered); the session-alias default resolves to
  the previous engine, so two pins are installed — the machine-local
  subagent-model env pin (`.claude/settings.local.json`, blanket) and
  the committed selective agent type (`.claude/agents/opus5.md`) —
  both effective from the next session (agent-type registration and
  settings-env injection are session-start events; verified
  empirically mid-session). Tasks DRIFT-002…005 of this date executed
  on the session-alias engine before the directive; all passed review
  with no returned round-trip.
- **2026-07-24 · DRIFT-005 landed — F-022 closed end to end; the DRIFT
  loop stands 5/5 no-return.** The owner's fact-links commission is
  complete across all three layers: contract (PROP-014 §2.1 + PROP-035
  §5/§7.3 with the reviewed heading-repeat precision), engine
  (DRIFT-004, core v0.8.0 mdspec fact units — all language families
  inherit through the shared engine at their next minting), and host
  compiler (DRIFT-005, vibe-spec: `NodeKind::Fact` IR leaves, per-fact
  override under `:add`, `CompileError::DuplicateId` merged-view gate,
  fact-addressed `#embed`). Code can now cite
  `spec://…#<FACT-ID>` per statement, and the §6 evidence join gains
  the campaign grain for Phase C. Session-end: the coder-tier engine
  pin (claude-opus-5) binds from the next session.
- **2026-07-25 · B2 modules sweep — 18 files in batches 8–18; B2 at
  32/35.** PROP-015/034/027/036/030/011/012/010/040/038/008/001/009/
  017/043/035/037/007 marked at fact grain (commits
  `b27336ae`…`1e7dff01`), ~1 540 units → ~1 770 anchored facts; every
  file 0 unmarked / 0 issues. Grammar precedents set: the
  Decision-paragraph idiom, `##req-*`/`##design-*` lines, `##self-uri`,
  checkbox anchors before `[x]`, `@impl/plan` for unexecuted phase
  plans, superseded-arc spec/done-vs-impl/done, em-dash cells count.
  Ledger +7: F-023 (dangling PROP-043 launcher ref) and the
  stale-header family F-024…F-029 — one Phase C/D sweep fixes all.
  GitVerse SSH down all session (verified clean ancestor via HTTPS;
  plain re-fan on recovery, never `--force`); GitHub carries everything.
- **RESOLVED scope ruling (owner, 2026-07-25, in session):** «я хочу
  исключить из проверки spec/terraforms, spec/research, spec/neworder.
  Это те вещи, которые мы делали в качестве рефакторингов и
  исследований давным-давно». **Ruling applied:** the three subtrees
  leave the include enumeration in `progress.toml` — long-executed
  plans and studies are historical records, not living contracts.
  Scope: 94 → **59 files**, 8 589 → **4 889 facts**; the SHRINK-PLAN
  pilot (28 expected errors) leaves with terraforms, so the expected
  `check` residue drops to **12** (design/README, burns at its
  re-mark). The DRIFT-001 cache prune dropped the out-of-scope records
  cleanly. The campaign plan itself is now out of scope — its LOG stays
  the process record, unmarked.
- **Next step:** finish the B2 tail — PROP-005 → PROP-003 → PROP-002
  (modules to 35/35), then `spec/design` (incl. the README re-mark
  burning the last 12 expected errors), `spec/boot` authored pair
  (additive markers only — user-owned files, zero re-forming),
  `spec/manual-tests` MT-01/02/03, `spec/discipline/README`. Journal
  step per file; batch commits ~1–3 files; then the Phase B exit gate
  (`check --exhaustive` clean over the 59-file scope) and the §4
  boundary ritual.
- **2026-07-25 · B2 batch 20 — PROP-003 marked; third superseded-arc
  split.** 310 units → 313 anchored facts, 0/0 (`d596c631`). The libsolv
  engine sections (§2.2/§2.3/§3.x, phase A, migration step 1) record
  history at spec/done — the SUPERSEDED-by-PROP-017 blockquote re-formed
  verbatim per F-015; the dependency vocabulary is impl/done **verified
  against the shipped crates** (features.rs incl. weak `?/` and exclusive
  groups, activation.rs — `if_os` impl/work per its recorded deviates,
  conditional.rs + fixpoint, manifest/i18n.rs, the four vibe-check
  entries, lockfile meta/package fields). Unshipped details stay
  spec/done: `pin_preferences` (recorded deviates), `VIBE_LANGUAGE`,
  dotted-key translations, `--all-languages`, `vibe review`, `outdated
  --upstream`, the LLM emission engine (`vibe-llm` pending); Phase F
  impl/plan. req-line fact ids dodge the owner-minted `req-*` heading
  anchors via `-req`/`-design` suffixes. Ledger +2: F-030 (stale
  design-proposal status line — F-024 family), F-031 (internal r2
  leftovers: §2.8 fence r1 syntax, §4.3 → §2.5.4 misref, §2.7.5/§2.9
  examples vs shipped `language_chain`/schema-v5 shape).
- **2026-07-25 · Owner amendment — Phase L (legacy relocation) inserted
  between B and C.** Directive quoted verbatim in the §5 Phase L section;
  the four dirs `spec/terraforms` `spec/research` `spec/neworder`
  `spec/discipline` leave the spec tree for root `legacy-spec/` once
  nothing references them: L1 reference inventory (living corpus + code —
  specmark, doctests) → L2 fact discovery → L3 fact relocation into
  `common/design/modules/manual-tests` (new specs allowed — owner grant
  same day) → L4 fact-grain markup at the new homes (+`progress.toml`
  scope grows) → L5 `git mv` to `legacy-spec/`. Ordering law: before
  Phase C, so verification covers the relocated facts. Phase C entry
  updated to "B and L closed". **Review point (OPEN):** the campaign plan
  itself lives in `spec/terraforms/` — relocate mid-campaign or at
  close-out? Owner call before L5.

- **RESOLVED scope ruling (owner, 2026-07-25, second in session):**
  «spec/discipline нужно исключить из анализируемого корпуса, потому что
  теперь Дисциплина - это часть пакетов ai-native, а саму spec/discipline
  после определения и портирования ссылок - перенести в legacy-spec.
  Сейчас она всё ещё в основном корпусе». **Ruling applied:**
  `spec/discipline/**` leaves the `progress.toml` include enumeration —
  the Discipline's living home is the ai-native packages
  (`core-ai-native` + the language stacks), so the host copy is a
  historical record like terraforms/research/neworder. Scope: 59 → **58
  files**; discipline/README's 16 facts leave the corpus; the B2 tail is
  now boot pair + manual-tests only. The Phase L §5 note that kept
  discipline/README in the B scope is superseded (corrected in place);
  Phase L's four-directory relocation list is unchanged — discipline
  still relocates to `legacy-spec/` after L1's reference inventory and
  L3's fact porting.

- **2026-07-25 · PHASE B CLOSED — the corpus is fully marked; exit gate
  green in full.** **Gate:** `progress check --exhaustive` clean over the
  final scope (58 files, 4 880 facts, 4 944 markers, 0 errors, 0
  warnings); floor `bash tools/self-check.sh` → `all green`, real exit
  code 0. **Final scope after the two 2026-07-25 rulings:** 58 files /
  4 880 facts (94→59 terraforms/research/neworder; 59→58 discipline).
  **The B2 tail (batches 20–26, this session):** PROP-003 — 313 facts,
  the third superseded-arc split, vocabulary verified against the
  shipped crates (`d596c631`); PROP-002 — 359 facts, modules close
  35/35 (`9328becb`); design/README re-mark burns the last 12 expected
  errors — the gate reads **0** for the first time in the campaign
  (`cb6e55b0`); loading-and-boot-model + action-system (`d1a09275`);
  workspace-and-qualified-naming + tui-visual-language — design 6/6
  (`91fde06c`); the authored boot pair, additive-only honoured
  line-for-line (`ae10dca2`); manual-tests MT-01/02/03 — genre
  precedent: step items and their Expected paragraphs are separate
  units, pending sign-offs honestly `impl/work` (`7d9dd964`).
  Interleaved the same session: the Phase L owner amendment
  (`c0147947`) and the discipline scope ruling (`1c48019a`). **Ledger
  at close: 35 findings** (F-030…F-034 new — two stale-header rows,
  PROP-003's internal r2 drift, the lore "forthcoming" family).
  **Predictions (§8) at the boundary:** P2 (the counter catches what
  review misses) CONFIRMED repeatedly — this session it caught the
  PROP-003 weak-deps continuation and the workspace-naming §7 intro;
  P5 (DRIFT no-return) holds at 5/5; P6 for B CONFIRMED with room —
  the markup pass ran 2026-07-24→25, two days against the 1–1.5-week
  budget — **while the §5-B batch-count guess (9–12) was falsified by
  the mid-phase fact-grain amendment** (~30 batches at the finer
  grain; the grain change, not slippage); P1/P3/P4 stay open for C/D.
  **Phase lane:** the journal-derived phase stays B until the owner
  opens the next; per the 2026-07-25 amendment the next phase is
  **L (legacy relocation)**, then C. **Awaiting the owner's call to
  open Phase L.**

- **RESOLVED review point + the B→L handoff (owner, 2026-07-25, closing
  the session).** The plan-file question is settled: the first instinct
  («Сам план кампании тоже перенеси в legacy-spec») was reversed in the
  same message — «Я передумал. План этой кампании пока переносить не
  нужно. Я хочу чтобы у нас остался правильный набор спецификаций и
  других инструкций, чтобы мы могли делать периодические проверки
  каждый месяц». **Ruling applied:** the campaign plan stays in
  `spec/terraforms/`; L5's move excludes it (the §5 Phase L review point
  is rewritten RESOLVED in place); its eventual relocation is a future
  owner call, no earlier than close-out — the §6 monthly recurrence
  keeps consuming it in place. And the handoff: «Перенеси все эти
  активности в следующую сессию и напиши как мне ее продолжить» —
  **Phase L execution moves wholesale to the next session**; this
  session ends at the closed-B boundary; `CONTINUE.md` carries the
  cold-resume recipe and the recommended opening phrase by which the
  owner opens Phase L explicitly (the phase event lands in the journal
  only then, per the DRIFT-003 phase lane).

- **2026-07-25 · PHASE L EXECUTED AND CLOSED — the legacy dirs are out
  of the spec tree.** Opened on the owner's recorded phrase (journal
  phase event `L` + `l1-inventory`). **L1 (inventory):** gate-binding
  set = 26 sites in 13 corpus files + 1 crates doc comment
  (`outdated.rs`); `spec/neworder` and `spec/discipline` had **zero**
  corpus inbound; out-of-gate referrers classified into live docs
  (ROADMAP, docs/), historical reports (terraform/), campaign zone,
  and an explicit leave-list (packages vendored comments, neworder2
  baselines, AUDIT quote, closed debt-ledger row DBT-0016,
  VIBEVM-SPEC — no real refs). **L2 (fact discovery) verdict: every
  cited fact was already corpus-resident** — the RP1
  rejected-alternative at design/action-system.md §4 D1, the ten
  design decisions, the DO1–DO18/Δ1–Δ16 sets restated in place, the
  settings deltas named inline, the campaign histories in PROP-038 §6
  / PROP-027 / PROP-036 — so **L3 ported nothing and the owner's
  new-spec grant went unused**; every citation dissolved into
  archive-provenance form instead (the honest inverse of the plan's
  port-then-repoint expectation, recorded here as the L2→L3 finding).
  **L4 (repoints):** four batch commits `83346e78` `f8f347d8`
  `9514e8fb` `1ec6a27c` — 26 sites incl. both `spec://vibevm/research`
  URI retirements, plus **four word-level sites the path greps could
  not see** (PROP-031 status-line, PROP-037 plan pointer, PROP-040
  delta-mapping, PROP-041 `spec.research` §3.7): the literal-backtick
  and dotted forms needed a lookbehind/word sweep — a reusable lesson
  for the §6 recurrence. Scope stayed 58 files (no new files → no
  `progress.toml` growth). **L5 (relocation):** `70f3cbdd` — 35 files
  `git mv`'d (terraforms 25, research 8, neworder 1, discipline 1) to
  root `legacy-spec/`, the campaign plan carve-out honoured; live
  out-of-gate pointers followed in the same commit (ROADMAP 15
  occurrences, docs/ 8, terraform 2 links, findings.json 11 paths,
  discipline.lock recipe, progress.toml comment); historical prose,
  quoted URIs, the closed debt row, and the pre-broken PLAYBOOK link
  stayed verbatim — records are not rewritten. `f311f429` regenerated
  the stale host specmap (absorbed B-phase drift + the move; ratchet
  37 gated orphans within allowance, 0 suspects). **Exit gate:** the
  reference greps read zero into the four dirs from corpus + crates
  (plan carve-out aside); `check --exhaustive` clean (58 files, 4 880
  facts, 0 errors); floor `self-check` all green, real exit 0.
  **Phase C (verification) awaits the owner's opening call** per the
  resume-boundary law.

- **2026-07-25 · PHASE C OPENED — the verify loop is live; the boot pair
  and manual-tests carry verdicts.** Mechanics fixed for the whole phase
  (PROP-043 §7.1/§7.5): verdicts live in the cache's per-file `campaign`
  map — `{verify_batch, verified_at, processed_hash, verdicts{anchor →
  {v, ev[]}}, summary}` — never in markup; `scan` preserves the maps
  (verified live) and projects them into `corpus.json` for the
  dashboard. Verdict semantics by stage: `impl/done` ⇒ presence
  evidence; `spec/done` ⇒ absence (shipped-but-still-marked-spec is the
  stale-header drift); `doc/done` ⇒ no contradiction with the contract;
  dated historical records confirm unless falsified; present-state
  claims blocked by the GitVerse outage go `unverifiable`, never
  "probably fine". Per-file coverage is assert-gated (extractor anchors
  == cache `marker_count` == verdict keys). **c0-boot** (`bb337e90`):
  64 facts — 61 confirmed / 1 drift / 2 unverifiable; the drift is real
  (LAYER-CODE names a nonexistent root `tests/` → F-035, user-owned
  file so the wording fix is the owner's). **c1-manual-tests:** 67
  facts — 61 / 6 / 0; MT-01's EXP-2/6/7/8/9 describe the pre-revision
  TUI keymap (shipped: F1…F6 menus, Shift+arrows tabs, Esc+confirm
  quit) → F-037 re-author; MT-02's footer quote omits F4 and says
  q-quit, its "once a picker lands" is superseded by the F4 settings
  menu → F-038; and the sweep caught a code-side stale clap help on
  `--plain` contradicting the shipped console-TUI default → F-036
  (Phase E DRIFT candidate). MT-03 verified clean 16/16. **Running
  tally: 131 / 4 944 markers judged — 122 confirmed / 7 drift / 2
  unverifiable; findings 38.** Machine-evidence base mapped: specmap
  carries 626 edges into `spec/modules` units and 111 into
  `spec/common` (section grain; facts inherit their section's edges),
  so the module cluster is the evidence-rich grind; design (6 files) →
  common → modules is the queued order.

- **2026-07-25 · c2-design — the design cluster verified; the drift
  is the aged-tense family.** 306 units judged (300 fact anchors + 6
  status-element bundles; coverage law recorded in the cache maps:
  verdicts key on fact anchors, table cell-markers inherit their row,
  `<status>` elements judged as `_elements`): **291 confirmed / 15
  drift / 0 unverifiable**. The drift map: loading-and-boot-model 8 —
  the lore's three inclusion types `inline/static/dynamic` (default
  `static`) against the shipped `link = "static" | "dynamic"` (default
  `dynamic`), plus "forthcoming" ×2 and the §6 `static|static` typo →
  **F-039**; action-system 4 — the F-034 forthcoming family, sharpened
  by `aiui.rs` actually shipping `list_actions` + `invoke` against the
  doc's "Not built now"; workspace-naming 1 — the M1.18-vs-M1.19
  milestone shift → **F-040**; tui-visual-language 2 — the "current
  ASCII scaffolding" present-tense and "When §2.2 carries" against
  five existing anchors → **F-041**. Hard confirmations: the
  `detect_tier` signature matches the lore literally, `PAD_X/PAD_Y/
  GUTTER` exact, palette hexes byte-equal in `rose_pine.rs`/
  `catppuccin.rs`, exit code 7 = `AMBIGUOUS_PACKAGE`, the §2 module
  table maps 1:1 onto `crates/vibe-actions/src`, and the two-way
  design↔PROP backlinks hold 4/4 (structural-loader parked by its own
  `spec/hold`). **Running tally: 437 units judged — 413 confirmed /
  22 drift / 2 unverifiable; findings 41.** Next: `spec/common` →
  `spec/modules` (the specmap-evidence-rich grind).

- **2026-07-25 · c3a-common-small — five common PROPs verified; one
  roster drift.** PROP-006 (frozen pointer), PROP-013 (audit
  instance), PROP-016 (source mirrors), PROP-028 (families), PROP-029
  (FQ addresses): **150 units — 149 confirmed / 1 drift / 0
  unverifiable.** The drift is PROP-028's family roster aged against
  the tree: `core-ai-native` ships v0.8.0 (the fact says 0.7.0), the
  **go-ai-native family is in force** (aggregator v0.1.0 + `-lang` +
  `-mcp`) but absent from §2.2/§2.3, and aggregators carry a
  `LICENSE.md` from the UPL relicense wave against the "vibe.toml +
  README and nothing else" letter → **F-042** (one roster refresh
  fixes all three). Prime confirmations: `cargo xtask mirror` ran
  three times in-session (the tracking-ref refresh of
  HIST-TRACKING-REFS observed live in its output),
  `push_args_never_force` exists at `xtask/src/mirror.rs:426`,
  `mirrors.toml` matches the §2 block verbatim, every §open question
  across the five files verified genuinely open, and PROP-029's three
  carriers match the generated INDEX/STATIC forms character-for-
  character. **Running tally: 587 units — 562 confirmed / 23 drift /
  2 unverifiable; findings 42.** Remaining: the common big seven
  (PROP-000, 018, 019, 024, 031, 032, 033 — 868 markers; specmap-rich
  on 018/019) → the modules cluster (3 300 markers).

- **2026-07-25 · c3b — PROP-000 verified: the foundation aged in six
  spots (the densest drift file of the phase).** 162 units — **149
  confirmed / 12 drift / 1 unverifiable**, all twelve drifts one
  family row **F-043**: §3 still records the proprietary EULA though
  its own revisit trigger fired (UPL-1.0 relicense 2026-07-12, MT-05);
  §4 lists the retired `vibe-package.toml`; §6 records pre-qualified
  identity and **four** kinds while the same file's INV-VOCABULARY
  correctly lists five (internal r2-leftover-class inconsistency); §7
  calls GitVerse "the source-of-truth" against PROP-016's no-primary
  model and pins `KindName` against the Fqdn default; §14's
  WAL-names-the-runs practice lapsed; §18 claims LLM-reviewed semantic
  conflicts whose LLM lane is pending. Hard confirms: all seven §2
  crates exist, the §15 prune of PROP-001 was **executed** (its
  ARG-PRUNED cites §15 back), schemas/ + vibe-wire/generated
  committed, resolvo pinned, both guides exist. The unverifiable is
  the GitVerse-network-bound legacy-registry claim (same verdict as
  c0's twin).
- **2026-07-25 · c3c — PROP-018 + PROP-019 confirm wholesale on
  implementation evidence.** 322 units — **320 confirmed / 2 drift**,
  both drifts the same shape and both already ledgered in Phase B: a
  proposed-era status line over a fully shipped system (**F-013**
  agentic modes, **F-005** vvm). Beneath the headers the content
  carries the densest machine evidence of the phase: the agentic
  relay/skill/affinity/transports/explain sections hold 23 implements
  / 8 verifies into `vibe-mcp`/`vibe-cli` (the `vibe agentic` +
  `vibe command` verbs run live in this very session), and the vvm's
  twelve sections map 1:1 onto `commands/vvm/` exactly as §3 placed
  them (37 implements / 38 verifies; `relocate` alone carries seven).
  Far-backlog sections verified genuinely unbuilt. **Running tally:
  1 071 units — 1 031 confirmed / 37 drift / 3 unverifiable; findings
  43.** Remaining in common: c3d = PROP-024 / 031 / 032 / 033 (384
  markers), then the modules cluster (3 300).

- **2026-07-25 · c3d — the common tail verified; `spec/common` is
  CLOSED 12/12.** PROP-024 / 031 / 032 / 033: **327 units — 322
  confirmed / 5 drift / 0 unverifiable.** All five drifts are
  PROP-024's: the F-006 proposed-era header (ledgered in B) plus a new
  family row **F-044** — the §2.6 deferral **fired**: the TypeScript
  pilot shipped, so the deferred engine split executed and
  `core-ai-native` now *authors* the neutral engines
  (conform/specmap/specmark/mcp cores, vendored byte-identically per
  PROP-028), making CORE-STAYS-PROMPT-ONLY and OOS-TS-CHECKER false as
  present-state; the related line also cites the vanished
  `vibedeps/flow-core-ai-native/0.6.0` slot. The §2.4 consumption
  topology confirmed **verbatim** (root `Cargo.toml`
  `exclude = ["packages", "vibedeps"]`; self-check drives the vendored
  engines by `--manifest-path` exactly as BINARY-RUN-FORM specifies).
  The three design proposals (031/032/033) verified **honest end to
  end**: every "schedules no implementation" claim grep-verified
  (no `move-unit`/`rename-address` in the stack, no `code://` nodes,
  no `[[refactoring]]` manifest table, no `vibe refactor` CLI), and
  PROP-032's decided-in-place q5 record carries its owner date.
  **Running tally: 1 398 units — 1 353 confirmed / 42 drift / 3
  unverifiable; findings 44. Verified clusters: boot ✓ manual-tests ✓
  design ✓ common ✓ (23 of 58 files). Remaining: the modules cluster
  (35 files, 3 300 markers — the specmap-richest).**

- **2026-07-25 · c4a — the modules cluster opens on the campaign's own
  contract; the tool that runs this phase verifies itself.** vibe-progress
  family: PROP-043 + OWNER-GUIDE + templates ×3 — **250 units (245 anchors
  + 5 element bundles): 236 confirmed / 14 drift / 0 unverifiable.**
  PROP-043's ten drifts split two ways. **F-045** — the file's own status
  aged behind the campaign it governs: the status-line still says
  "implementation underway (Phase A)" and holds *(provisional)* sections
  that no longer exist (grep = 1, the sentence itself), while the phase
  lane reads C and §5 is fully shipped (19 implements edges; all seven
  subcommands + every documented flag verified live this session).
  **F-046** — a marker-vs-implementation parity family in *both*
  directions: impl/done over unshipped fragments (EvidenceProvider wired
  nowhere outside the core — the adapter imports everything but
  `evidence`; 0 fact units/edges in host specmap.json — the consumed
  stack engine v0.7.0 predates the fact amendment, PROP-014 v0.8.0 §2.1
  exists authored-side awaiting the re-mint; CMD-CHECK's "lossless
  folds" matches zero code; campaign.json carries no `gates` field; the
  report has no evidence column; `Cache::is_current` is dead outside its
  own tests — every run re-parses the tree), and spec/done under shipped
  code (§7.3: `BaselineUnit` matches the record field-for-field, rescan
  live with hash-suspect + marker-diverged; named-crates and the control
  sample honestly missing). OWNER-GUIDE's four drifts are all one F-020
  refresh sweep (4-of-6 placements, the preamble-less amendment missed
  twice, FOUR-SURVIVORS omits `tasks/`). The templates confirm wholesale:
  impl-task exercised 5/5 (DRIFT-005 checked field-for-field), spec-task
  correctly awaits Phase D, harvest-card consistent with its empty
  `harvest/`. Method note for the batch map: coverage now reads from
  `progress mirror`'s ParsedDoc (authoritative fence-aware parse) — the
  raw-grep extractor over-counts code-span shorthands. **Running tally:
  1 648 / 4 944 — 1 589 confirmed / 56 drift / 3 unverifiable; findings
  46 (next free F-047).**

- **2026-07-25 · c4b — the registry core verified; the evidence-richest
  file confirms at 98 %.** PROP-002 (360 markers, 110 specmap edges) +
  PROP-001 (93): **379 units — 371 confirmed / 8 drift / 0
  unverifiable.** The headline confirmation: **RESOLVO-PRIMARY holds on
  live evidence** — `ResolvoDepSolver` is the shipped production
  default (`registry.rs unwrap_or("resolvo")`, resolvo_engine = "the
  production DepSolver cell"), and the §2.8 fallback-seam story
  (naive + sat as selectable cells behind `DepSolver`) is exactly what
  shipped. That same check exposed **F-047 (code-side)**: the two
  `#[spec(deviates)]` reasons in `naive.rs`/`sat.rs` still claim "no
  ResolvoSolver exists in tree" / "adopting resolvo stays an owner
  decision" — they aged behind the very adoption they awaited. The
  spec-side drift splits into two touch-up families: **F-048**
  (PROP-002 precision: `--trust-mirror` promised twice and shipped
  nowhere — only `--trust-redirect` exists; `vibe list --overrides`
  promised twice and absent; the git-source ref errors ship as
  reason-strings, not the named `MissingRef`/`ConflictingRefs`;
  `source_kind` grew `path`/`embedded` in PROP-007/030; the cache-slot
  example still shows the kind-name era against the live
  `packages/<group>.<name>/clone`) and **F-049** (PROP-001: no crate
  README behind `mechanics-in-readme`; `NO-OFFLINE-YET` aged behind the
  shipped `--offline`; the git-binary parking-lot entry resolved by the
  shipped `VIBE_GIT_BINARY` whose comment cites §6 back). Everything
  else confirms on dense machine evidence: the redirect subsystem
  wholesale (12 implements / 8 verifies, three CLI verbs, hop-limit
  guard, `--trust-redirect` at cli/registry.rs:226), the auth-silencing
  matrix (`apply_common_env` + regime-aware force_silence + 3
  tests_pure verifies), `merge_effective` / `url_is_local` /
  `registry_config_path` live under their exact spec names, the
  `enabled` filter at the R-001 construction point, the mirror
  fall-through loop, token redaction tests under the exact names the
  spec cites, and the lockfile schema chain v2→v3→v4→v5 documented
  end-to-end across PROP-002 → PROP-007 → PROP-008 (live lock:
  schema 5). **Running tally: 2 027 / 4 944 — 1 960 confirmed / 64
  drift / 3 unverifiable; findings 49 (next free F-050).**

- **2026-07-25 · c4c — the resolver pair verified; one c4b verdict
  corrected.** PROP-003 (314 markers) + PROP-017 (106): **372 units —
  359 confirmed / 13 drift / 0 unverifiable.** PROP-017 verifies almost
  wholesale on the c4b code evidence (architecture item-for-item:
  `SemverVersionSet` literal to the spec, NowOrNever sync adapter,
  shared output builder, dominance oracle, capability closure
  pre-scan); its only drift is the known **F-027** status-line
  ("implementation in progress" + an impl/*work* document marker over
  its own §6 "the port is COMPLETE"). PROP-003's eleven split three
  ways: **F-030** (the design-proposal status line over a shipped
  vocabulary), four **F-031** rows landing exactly as the B-phase
  ledgered them (the r1 `__exclusive` fence, the §4.3→§2.5.4 misref,
  and the `language_chain`-vs-`language`+`language_fallback` trio —
  lockfile.rs:130 is one merged field), and a new **F-050**: the
  solver-era tail *outside* the §2.2 supersede marker — §2.1 still
  promises "SatDepSolver becomes the default" with a `[meta].solver`
  selection key and a `naive|sat` CLI (shipped: `naive|sat|resolvo`,
  default resolvo, no meta.solver field), §2.11/§6 still record a
  sat-default flip, and `vibe update --features` was promised but
  never wired. The same evidence trail **corrected c4b**:
  PROP-002's SOLVER-IDENTITY-FIELD had confirmed on a section default,
  but the live lock has no `solver` field and PROP-017 §8 says so —
  verdict amended to drift, F-048 extended (f), and F-047 extended
  with a third stale deviates (lib.rs:288-296 still claims
  "SatDepSolver is not in tree" while sat.rs ships). Subskills
  confirmed with a live wink: this session's own MCP toolbox carries
  `read_subskill` / `materialise_subskill` — the M1.7 lazy-delivery
  surface §2.5 designed. **Running tally: 2 399 / 4 944 — 2 318
  confirmed / 78 drift / 3 unverifiable; findings 50 (next free
  F-051).**

- **2026-07-25 · c4d1 — the workspace big three verified; the loading
  model is the session itself.** PROP-007 + PROP-009 + PROP-035 (386
  markers): **360 units — 351 confirmed / 9 drift / 0 unverifiable —
  and zero new findings**: every drift row lands on ledger entries the
  B-phase already minted. PROP-007's four are all **F-029** — the
  status/milestone lines, §9.3's deferral record, and the document
  marker still call workspace-aware `vibe install` "the remaining
  piece" while M1.18 shipped it (`Workspace::discover` live at
  plan.rs:101 and apply.rs:114). PROP-009's three: the **F-026** DRAFT
  header pair, plus `SURF-SHOW-EFFECTIVE` caught shipped-but-spec/done
  (`vibe show effective` exists in its simple concatenation form; the
  §2.8 engine projection honestly stays v1.5). PROP-035's two are the
  **F-028** DESIGN-provisional pair over a §17 that records the
  compiler shipping three times (vibe-spec 07-15, the link-type rename
  07-16, normal+static AOT 07-20). The confirmations needed no
  reconstruction — the model under test booted this very session:
  STATIC.md read first, the TOML INDEX.md with its `[[entry]]` grammar,
  committed `vibedeps/`, `when = "os:*"` in the renderer, BootCategory
  ordering with the conflict errors deleted, `vibe reinstall` citing
  §2.10 from its own `--help`, and the vibe-spec pipeline (doctree
  fact leaves, `:add`/`:replace` with per-fact override and the
  merged-view DuplicateId gate, embed cycle guards, `PackageFormat`
  simple-by-default) exactly where §5–§13 put it. **Running tally:
  2 759 / 4 944 — 2 669 confirmed / 87 drift / 3 unverifiable;
  findings 50 (next free F-051).**

## 10. Deferrals {#deferrals}

*(empty — drained into `campaigns/<id>/deferrals.md` at close-out)*

## 11. REPORT (filled at close-out against §8) {#report}

*(empty)*
