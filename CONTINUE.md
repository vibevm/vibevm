# CONTINUE — cold-resume checkpoint

_Written 2026-07-28 (**Phase C: W1, W2 and W3 are CLOSED — 1 714 anchors judged
row by row; the phase is past 64 %**). `spec/WAL.md` is the canonical living
state and supersedes this snapshot wherever they diverge._

## TL;DR

**Three world batches are closed and six packages with them.** W1 407 (90.4 %),
W2 692 (91.9 %), W3 615 (**89.9 %, and zero unverifiable** — the first batch in
the phase where every fact could be settled against the tree). `world` reads
**1 557 / 140 / 17 — 90.8 %** over 51 of its 121 files.

**§5-C's falsifiable prediction is settled and it inverted.** It said `world`
would measure higher than `ai-native`; over 1 714 anchors it reads 90.8 %
against 91.6 %. The reason is the opposite of the one predicted: these flows make
claims about the consuming project, and this consumer is measurable to the line.

**The per-file slice replaced the batch as the unit of work**, and that answers
the split question the plan left open. Seventeen slices landed here, one file
each, 17 to 149 rows, each merged and sealed on its own. **W5 does not need
splitting; it needs twenty-one slices.**

**Nothing is in flight.** No worker running, no table half-read. W4 is unstarted
and its evidence has not been gathered.

## Where the numbers are {#numbers}

| | |
|---|---|
| host | 58 / 58 files, 4 499 verdicts — 99.9 % |
| **ai-native** | **80 / 80 files CLOSED** — 2 697 verdicts, 2 470 / 207 / 20, **91.6 %** |
| **world** | **51 / 121 files** — 1 714 verdicts, **1 557 / 140 / 17, 90.8 %**, 138 self-referential (8.1 %); **2 436 anchors owed** |
| phase | **4 411 of 6 847 — 64.4 %** |
| gate | `progress check --exhaustive` clean, 259 files |
| tree | clean, in sync with `origin/main`, mirrored to GitVerse + GitHub |

**Never decrement these; re-measure.** One command prints all of it:

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

Per package, closed: `two-process-model` 96.6 % · `wal-specspaces` 93.5 % ·
`sync-from-code` 93.7 % · `conflict-protocol` 93.6 % · `decision-records` 92.9 % ·
`addressable-specs` 87.9 % · `flow:wal` 86.5 %.

## What remains {#remains}

| batch | packages | files | anchors |
|---|---|---:|---:|
| **W4** | campaign-plans, discovery-prompt, comparative-research, redbook | 15 | 564 |
| **W5** | operating-modes, health-audit, manual-tests, secrets-hygiene | 21 | 697 |
| **W6** | licensing, source-mirrors, spec-genres, dev-runtime-docs | 19 | 572 |
| **W7** | managed-blocks, qualified-naming, tool-design-lessons | 15 | 603 |

**W4 is unstarted and nothing has been gathered for it.** Both recipe steps
remain: capture its three §3.1 sources into `harvest/world-w4-plans-and-inquiry.md`,
then commission four `opus5` workers, one per package.

**W4's file list with per-file anchor counts, measured from `run/mirror/` — hand
these to the workers verbatim so nobody re-derives them:**

```
campaign-plans (218)
  23  packages/org.vibevm.world/campaign-plans/v0.1.0/README.md
  29  …/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md
  64  …/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md
  48  …/spec/flows/campaign-plans/execution-ledger.md
  54  …/spec/flows/campaign-plans/phase-gates.md
comparative-research (180)
  21  packages/org.vibevm.world/comparative-research/v0.1.0/README.md
  12  …/spec/boot/52-flow-comparative-research.md
  60  …/spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md
  61  …/spec/flows/comparative-research/from-research-to-roadmap.md
  26  …/spec/flows/comparative-research/research-template.md
discovery-prompt (83)
  19  packages/org.vibevm.world/discovery-prompt/v0.1.0/README.md
  10  …/spec/boot/50-flow-discovery-prompt.md
  54  …/spec/flows/discovery-prompt/usage.md
redbook (83)
  44  packages/org.vibevm.world/redbook/v0.2.0/README.md
  39  …/spec/boot/03-flow-redbook.md
```

**Two things about W4 worth knowing before the harvest.** `redbook` is the
umbrella that pins the other twenty-two members and ships the book itself at
`spec/book/ru/` — its facts are mostly about the collection's own composition, so
source 1 and the lockfile carry more of the weight than usual. And
`discovery-prompt` ships an artefact the boot lane explicitly tells sessions NOT
to load outside a deployment request, so «is it in use here?» is the wrong
question to ask of it — `F-119` already records that the book's chapter 1 cites a
`safeharbor.md` that exists nowhere, in both redbook slots.

## The recipe {#recipe}

1. **Capture the batch's three §3.1 sources into `harvest/`**, each `command →
   real output`. `harvest/world-w3-ipc-core-ii.md` is the model.
2. **Commission one `opus5` worker per package**, pointing each at
   `tasks/WORLD-WORKER-BRIEF.md` and the harvest, with the file list and per-file
   anchor counts from `run/mirror/`. **Tell them to prefer durable citation
   targets — `CLAUDE.md`, `spec/boot/**`, `spec/common/**`, the crates — and to
   avoid `CONTINUE.md` and `spec/WAL.md`.** That two-sentence addition is what
   gave W3 **1 805 refs and zero unresolvable on the first pass**.
3. **`verify-evidence.py` BEFORE reading a word**, then `repair-refs.py --apply`.
4. **One file per slice.** Read the subject document in full, then
   `show-rows.py --brief --file <name>` row by row, then write that file's
   verdicts alone, `merge-verdicts.py`, `progress seal`, `summary.py`, commit.
   A slice that lands cannot become a debt.

## What W1–W3 settled, so W4–W7 do not re-derive it {#judged}

- **Non-adoption is not drift — the line the whole cluster runs on.** A flow's
  prescription the host simply never adopted is **confirmed**: a human's morning
  read leaves no repository artefact, and no flow claims the host performs one.
  **Drift is where the host's own written contract contradicts the flow**, or
  where a measurable rule is broken over a double-digit share of its window.
  `morning-routine.md` is unadopted end to end and scores 39 of 42 confirmed; its
  two drifts are both a cold-start read order that `CLAUDE.md:205` reverses in
  writing.
- **Each fact is judged on its own sentence, never on its family.**
  `NEVER-APPEND-TO-THE-WAL` prohibits appending only and the host never appends —
  confirmed. `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` names patching too, and
  `CLAUDE.md`'s step 2 says «Update … bump … refresh» — drift.
- **A definition that classifies a failure correctly is confirmed BY that
  failure.** «A decision without a revisit condition becomes a sacred cow» is
  confirmed by 142 sections having none, not refuted by them.
- **A numeric target the host exceeds is drift**, and so is its summary
  restatement (W1's `HEADER-TARGET-LENGTH-AND-HARD-LIMIT` precedent).
- **A fact whose subject IS a `../flows/…` pointer is drift** — the 69-dangling
  family, found in seven packages so far. A rule fact that merely contains such a
  link is judged on the rule.
- **Measurements already in hand, reusable:** boot lane **~16 100 tokens against
  a 500-token budget** (32×) · `spec/WAL.md` ~4 000 against 3 000 · **9 of 47**
  module specs over 5 000 · **4 of 153** decision sections carry four fields, 127
  carry one · **11** revisit triggers exist, **0** carry all three parts ·
  `docs(spec): sync …` typed **0 times in 2 041 commits** against 183
  `docs(spec)` · **857 of 982** headings anchored, the 125 without being **all 23
  in `spec/boot/` and all 8 in `spec/WAL.md`** · **59 duplicate-anchor warnings**
  in the generated boot lane · `Test:` lines in `spec/`: **0**.

## The standard that judges a verdict {#standard}

> **A fact that PRESCRIBES what the discipline requires is confirmed when it is
> coherent and every referent it names resolves**, including a referent the
> package itself declares as future work.
>
> **A fact that DESCRIBES what this repository already ships or does is checked
> against the tree, and a description that does not match is drift.**
>
> **A fact whose subject cannot be exercised here** — a toolchain not installed,
> a human's reading habit, a project that declined the convention — **is
> unverifiable, and says so in its own words.**

For `world` there is a fourth clause: **a flow's fact is also checked against the
host's observed conformance.** A rule-fact drifts on **systematic**
non-compliance — a double-digit share of the measured window — not on single
exceptions the record names. And see §judged: non-adoption is not non-compliance.

## Findings open {#findings}

| id | what | who closes it |
|---|---|---|
| **F-129** | the `wal` package ships two contradictory wind-downs — six steps in `session-end-hook.md`, five in a different order in `cold-resume.md`, and a third fact calling them the same | **owner** (published slot → F-122) |
| **F-124…F-128** | unresolvable evidence ids · one measurement published twice · one tool name on two tools · `-race` prescribed 15× and run 0× · `spec/boot/INLINE.md` does not exist | **owner** / an edit |

Carried: **F-117 … F-123**, **F-114**, **F-087 / F-088**, **F-078**, **F-092**,
**F-069**, the `specmap` ratchet's 37 gated orphans.

**Not yet filed as findings but measured and in the verdicts' reasons:** three
more internal contradictions (record-template vs revisit-triggers on whether a
trigger fires unprompted; cognitive-load-split vs the wal package on whether one
text serves three readers; addressable-specs vs wal on where invariants sit), and
one collision of principle — `uncertainty-protocol` prefers no new dependency,
`PROP-000` §15 decides the opposite at the governing anchor in the four-field
form.

## Mechanics — fixed, not re-invented {#mechanics}

Verdicts live in the per-file `campaign` map inside `run/cache.json`, **never in
markup** (PROP-043 §7.1/§7.5). Mutate by **load-and-merge only**.

```bash
python campaigns/packages-2026-09/tasks/verify-evidence.py <ev-*.json>
python campaigns/packages-2026-09/tasks/repair-refs.py <ev-*.json> --apply
python campaigns/packages-2026-09/tasks/merge-verdicts.py <batch.json> [--force]
cargo run -q -p vibe-cli --bin vibe -- progress seal --campaign campaigns/packages-2026-09 <paths…>
python campaigns/packages-2026-09/tasks/summary.py [--batch W3] [--by-file]
```

`merge-verdicts.py` refuses: a non-addressable anchor, a verdict outside
`{confirmed, drift, unverifiable}`, an empty or too-short `ev`, a missing `src`
on `world`, and a silent overwrite. **It accepts a SUBSET of a batch's files
under the same batch id** — that is what makes the per-file slice safe.

**Never hand-write `verified_at` or `processed_hash`** — `seal` writes them.

**Generate each batch file with a small script** holding an
anchor → (verdict, src, reason) map, defaulting the rest to confirmed with the
worker's own `searched` as the reason; no anchor is then transcribed by hand.
Key by row INDEX only when anchors repeat inside one table. Seventeen such
scripts are in the scratchpad path named in each slice's commit.

## Decisions still in force {#decisions}

- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling 2026-07-28). Verdicts are never delegated; neither is the review
  of delegated output. Only evidence gathering is.
- **The subject is not modified to make the measurement pass** (batch plan §2.2).
- **`vibedeps/` stands in for §3.1's third source** (§2.3).
- **A verdict records its source class in a field, not in prose** (§2.1 / A2).
- **Superseded version slots are marked, never verified** (§3.3).
- **The perimeter law** (batch plan §4.5): SPEC in `core-ai-native`, ENGINE in its
  library crates, DRIVER in each language stack's CLI, DEPLOYMENT in a consuming
  project. A `not-found` is a fact about the search perimeter until the perimeter
  has been checked.
- **A finding spanning a package boundary is a release event** (§5-D).

## Repository map {#map}

```
crates/           the vibe workspace — cli, resolver, registry, progress-core, check, mcp, wire, llm
xtask/            build-side drivers: codegen, specmap, sync-engines, mirror
spec/             the contract tree — boot/, common/PROP-*, modules/, flows/, terraforms/
packages/         the authored corpus: org.vibevm.world (27 flows), org.vibevm.ai-native (11), org.vibevm.fractality
vibedeps/         the installed copies a consumer actually receives
campaigns/        packages-2026-09 (live) · progress-2026-08 (archival)
research/         rust-demo · ts-demo · go-demo — the three consuming projects; tcg-bench
discipline/       the host's own conform / specmap / health artefacts
docs/  legacy-spec/  refs/  neworder2/  fixtures/  schemas/  apps/
```

## Quick start {#quick-start}

```bash
python campaigns/packages-2026-09/tasks/summary.py
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
bash tools/self-check.sh ; echo "EXIT=$?"
cargo xtask mirror --check
```

## Standing traps {#traps}

1. **A wind-down invalidates any sealed evidence table citing `CONTINUE.md` or
   `spec/WAL.md`.** W2's tables went from 3 unresolvable to 65 that way, twice in
   one session, and not one break was a fiction. Re-verify before reading,
   always — and prevent it upstream by telling workers to cite durable files.
2. **`run/cache.json` carries every verdict.** Load-and-merge only.
3. **Every parsing `progress` subcommand writes the cache — `check` included.**
   Always pass `--campaign`; never point one at `campaigns/progress-2026-08`.
4. **A count including `node_modules`, `.vibe/cache/` or `vibedeps/` is a count
   of somebody else's code** — except when `vibedeps/` is the subject.
5. **An absence you assert is not an absence you checked.** A W3 worker wrote
   «no host rule discouraging new dependencies exists» against a `PROP-000` §15
   that exists and rules the opposite way.
6. **Python reading git output must decode UTF-8 explicitly** — this box defaults
   to cp1252 and `subprocess.run(text=True)` dies on the corpus's em-dashes. Use
   `capture_output=True` + `.decode("utf-8", errors="replace")`.
7. **`repair-refs.py` preserves each table's own indent** — it did not until this
   session, and re-dumping at a fixed width buried 51 real repairs in 4 481
   cosmetic ones.
8. **Never `git add -A` while a worker is running.** Stage explicit paths.
9. **Do not run a `vibe` command while `tools/self-check.sh` runs.**
10. **Set `PYTHONIOENCODING=utf-8`** before any Python that prints this corpus;
    edit through editor tools, never a PowerShell round-trip.

## The recent commit chain {#commits}

```
cf6c7927 docs(wal): the checkpoint catches up to three closed batches
a77c4be5 docs(campaign): W2 and W3 in the ledger, and the unit that replaced the batch
c75f4216 chore(campaign): W3 closes at 615, and the addressability flow is least addressable
85fb6fa6 chore(campaign): the boot budget is 500 tokens and the host loads 16 100
10ed70dd chore(campaign): the layout the host runs is not the layout the flow draws
dbdd1091 chore(campaign): the addressability rule is broken by its own checker
3facaed1 chore(campaign): the host decided the opposite, in the prescribed form
b3d48327 chore(campaign): conflict-protocol closes, and the hierarchy is its whole debt
9dd9aa46 chore(campaign): three hierarchies, and no two of them agree
68dc5fb2 chore(campaign): decision-records closes, and its own never is the one broken
8f7850b2 chore(campaign): the trigger document is confirmed by the host failing it
67bce43a chore(campaign): four records of 153 carry the four fields the flow requires
2e86018b chore(campaign): W3's evidence lands at 615 anchors and zero unresolvable refs
582f603e chore(campaign): W2 closes at 692, and a milestone label became measurable
784c5f02 chore(campaign): the grammar the boot snippet prescribes has never been typed
e6e650fd chore(campaign): the sync protocol's third mandatory part never lands
97b8014d chore(campaign): the registry's one-line status is 1 029 characters
1927b4ee chore(campaign): two-process-model closes, and one text does not serve three readers
261a46e2 chore(campaign): the boot budget is 500 tokens and the host loads 22 300
c99b719e chore(campaign): the model document scores 100 %, and that is the finding
689d23c3 chore(campaign): the model's own two files, and a rule its install undercuts
3428fdbf chore(campaign): the checkpoint moved refs under the tables it had just described
958dbcf5 chore(campaign): the wal package's two wind-downs disagree with each other
a43159b9 chore(campaign): the wal flow measured against the WAL it specifies
6e567dee chore(campaign): the session-end moved 65 refs under W2's finished workers
```

## The resume prompt {#prompt}

```text
ВОССТАНОВИ СЕССИЮ

Затем продолжи Phase C кампании PROP-043 волны 2 (campaigns/packages-2026-09).
ЗАКРЫТЫ W1 (407), W2 (692) и W3 (615) — 1 714 якорей, поштучно. Кластер world =
1 557 / 140 / 17, 90.8 % на 51 файле из 121. Фаза C — 64.4 %, осталось 2 436 якорей. Ничего не в полёте:
воркеров нет, недочитанных таблиц нет.

СЛЕДУЮЩЕЕ — БАТЧ W4 (campaign-plans, discovery-prompt, comparative-research,
redbook; 15 файлов, 564 якоря). Он НЕ НАЧАТ и доказательства НЕ СОБРАНЫ.

Перед началом прочитай, в этом порядке:
  1. CONTINUE.md — целиком: §remains, §recipe, §judged (что уже решено — там
     ЛИНИЯ ПРО НЕПРИНЯТИЕ и готовые измерения), §standard, §traps
  2. campaigns/packages-2026-09/harvest/world-w3-ipc-core-ii.md — образец харвеста
  3. campaigns/packages-2026-09/tasks/WORLD-WORKER-BRIEF.md — контракт делегата
  4. campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md — §2.1 (src), §4.5 (периметр)
  5. spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md §3.1, §3.2, §5
     и §7 LOG с конца — там записи W2 и W3

ПЕРВОЕ ДЕЙСТВИЕ: собери харвест W4 (три источника §3.1, каждый как
`команда → реальный вывод`), закоммить его, и только потом комиссуй четырёх
воркеров opus5 — по пакету на воркера. В брифе ОБЯЗАТЕЛЬНО: цитировать durable
файлы (CLAUDE.md, spec/boot/**, spec/common/**, crates/), НЕ цитировать
CONTINUE.md и spec/WAL.md. Эти две фразы дали W3 1 805 ссылок и НОЛЬ
неразрешённых с первого прогона.

ЕДИНИЦА РАБОТЫ — ОДИН ФАЙЛ, НЕ БАТЧ. Прочитай файл-субъект целиком, потом
show-rows.py --brief --file <имя> построчно, потом батч только этого файла,
merge-verdicts.py, progress seal, коммит. merge-verdicts.py спокойно принимает
подмножество файлов батча под тем же id. Семнадцать таких срезов закрыли W2 и W3,
и ни один не стал долгом. W5 (697 якорей) делить НЕ НАДО — ему нужен 21 срез.

Каждый world-вердикт ОБЯЗАН нести src — непустое подмножество [1,2,3] по §3.1.

СТАНДАРТ ВЕРДИКТА — CONTINUE.md §standard, и ключевое из §judged: непринятие
хостом предписания — НЕ дрейф; дрейф — там, где СОБСТВЕННЫЙ ПИСЬМЕННЫЙ контракт
хоста противоречит флоу, либо где измеримое правило нарушено на двузначной доле
окна. Каждый факт судится по своему предложению, а не по семье. Определение,
которое верно классифицирует провал, подтверждается ЭТИМ провалом.

Механика: вердикт живёт в per-file карте campaign внутри run/cache.json и НИКОГДА
в разметке; мутировать только load-and-merge; verified_at и processed_hash пишет
только `vibe progress seal`. Батч-файл генерируй скриптом с картой
якорь → (вердикт, src, причина), остальное по умолчанию confirmed с полем
searched воркера как причиной. Каждое число в отчёте приходит из команды.

ДЕЛЕГИРОВАНИЕ: во встроенные агенты opus5 через инструмент Agent. Fractality НЕ
использовать. Вердикт не делегируется никогда — делегируется только сбор
доказательств; ревью делегированного — тоже твоё.

АВТОНОМИЯ: работай полностью автоматически. Все механические операции — сама:
правки, скрипты, cargo, git commit, git push через `cargo xtask mirror`.
Останавливайся ТОЛЬКО когда нужно настоящее смысловое решение владельца.
Находка — не повод останавливаться: фаза C находки заводит, а не чинит.

НЕ ОСТАНАВЛИВАЙСЯ НА ГРАНИЦАХ РАБОТЫ. Закрытый срез, закрытый батч, написанный
отчёт, зафиксированный коммит — это не точки сдачи хода. Сводку пиши и СРАЗУ
переходи к следующей единице работы в том же ходе.

ТОКЕНЫ НЕ ЭКОНОМЬ, и НЕ ОБЪЯВЛЯЙ ПРЕДЕЛ КОНТЕКСТА, КОТОРЫЙ НЕ ИЗМЕРИЛ — это
записанная ошибка прошлой сессии: контекст здесь суммаризуется, а не обрывается.
Не классифицируй строки пачкой ради экономии. Если действительно упёрся:
сохрани состояние (CONTINUE.md + spec/WAL.md + запись в LOG), напиши новый промт
по образцу этого, зафиксируй и раскатай — и скажи прямо.
```

**The WAL is the canonical living state.** If this file and `spec/WAL.md`
disagree, the WAL wins.
