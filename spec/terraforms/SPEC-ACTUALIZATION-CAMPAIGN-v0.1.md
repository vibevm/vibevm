# Spec-Actualization Campaign v0.1 — mark, verify, and de-drift the whole spec tree {#root}

<status stage="spec" state="done" action="continue" actionstage="impl" comment="plan authored; scaffold underway (Phase A); campaign proper not started"/>

**status: AUTHORED 2026-07-24 · NOT STARTED · vibevm-specific · first consumer of PROP-043 (Progress Control) · Phase A is the scaffold build; the campaign proper starts at Phase B**

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

### Phase C — Verification (evidence pass) {#phase-c}

*Entry:* B closed (per-cluster start allowed once a cluster's files are
marked). *Executor:* Fable + machine evidence. *Steps:* every marker gets a
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
- **Next step:** B2… batches — `spec/modules/**` at fact grain (the
  largest cluster), then `spec/design` / `spec/research` /
  `spec/terraforms` including the two pilot files' re-mark. Journal step
  per file; batch commits of ~3–6 files (fact density triples the diff
  per file); ledger findings in passing; semantic edits forbidden.
  Opus lands DRIFT-002 first so the floor returns green (Fable
  reviews); DRIFT-003 restores an honest phase lane.

## 10. Deferrals {#deferrals}

*(empty — drained into `campaigns/<id>/deferrals.md` at close-out)*

## 11. REPORT (filled at close-out against §8) {#report}

*(empty)*
