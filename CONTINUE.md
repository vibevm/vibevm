# CONTINUE — cold-resume checkpoint

_Written 2026-07-28 (**Phase C: `flow:wal` is CLOSED at 260 of 260; W2 is half
judged — `ev-W2c` and `ev-W2d` are on disk and unjudged**). `spec/WAL.md` is the
canonical living state and supersedes this snapshot wherever they diverge._

## TL;DR

**W2a and W2b are closed, read row by row. The `wal` package is complete: 260
verdicts, 225 confirmed / 27 drift / 8 unverifiable — 86.5 %**, the lowest-scoring
package in `world` so far. The reason is structural and it sharpens the phase's
whole story: **this flow's facts describe `spec/WAL.md`, and `spec/WAL.md` is on
disk and measurable line by line.** Where W1's git flows could only be checked
against a commit log, W2a's required-sections contract is checked against the
artefact it specifies — and the artefact breaks six clauses of it.

**`world` now measures 88.9 % against `ai-native`'s 91.6 %,** so §5-C's falsifiable
prediction is not merely in trouble, it is inverting further with each batch.

**One finding opened: F-129 — the `wal` package ships two contradictory
wind-downs.** `session-end-hook.md` orders «the full hook, steps 1-6»;
`cold-resume.md` §wind-down orders **five** in a different sequence, with no
stopping-state step and no collapse step; a third fact asserts they are the same
procedure. The host implements `cold-resume.md`'s five exactly, in order.

**And the session opened on a defect of its own making.** W2's four tables were
verified at 3 unresolvable when the last session sealed them, and at **65** when
this one re-verified. Not one was a fiction: the wind-down had overwritten
`CONTINUE.md` and the WAL's `_Updated:` line under tables that cite both.

Nothing is blocked. Tree clean, in sync with `origin/main`, mirrored to both hosts.

## Where the numbers are {#numbers}

| | |
|---|---|
| host | 58 / 58 files, 4 499 verdicts (4 496 confirmed · 3 unverifiable) |
| **ai-native** | **80 / 80 files CLOSED** — 2 697 verdicts, **2 470 / 207 / 20, 91.6 %** |
| **world** | **26 / 121 files** — 755 verdicts, **678 / 62 / 15, 89.8 %**, 39 self-referential (5.2 %); **3 395 anchors owed** |
| `flow:wal` | **7 / 7 files CLOSED** — 260 verdicts, **225 / 27 / 8, 86.5 %** (W2a 81.1 %, W2b 90.6 %) |
| `two-process-model` | **3 / 5 files** — 88 verdicts, **85 / 3 / 0, 96.6 %** (README 100 %, boot 82 %, model 100 %) |
| phase | **3 452 of 6 847 — 50.4 %** |
| gate | `progress check --exhaustive` clean, 259 files, 0 warnings |
| tree | clean, in sync with `origin/main`, mirrored to GitVerse + GitHub |

**Never decrement these; re-measure.** One command prints all of it:

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

## W2's remaining half — read, do not re-commission {#w2-remaining}

**344 anchors are left, all on disk, committed and verified.** Their three §3.1
sources are captured in
[`harvest/world-w2-wal-family.md`](campaigns/packages-2026-09/harvest/world-w2-wal-family.md).

| table | files | rows | state |
|---|---|---:|---|
| `ev-W2a.json` | `wal` — README, boot snippet, `WAL-PROTOCOL.md`, the SKILL | 111 | **closed, 81.1 %** |
| `ev-W2b.json` | `wal` — `cold-resume.md`, `morning-routine.md`, `session-end-hook.md` | 149 | **closed, 90.6 %** |
| `ev-W2c.json` | `two-process-model` — README, boot snippet, `TWO-PROCESS-MODEL.md` | 88 | **closed, 96.6 %** |
| `ev-W2c.json` | `two-process-model` — `cognitive-load-split.md` | 50 | **unjudged** |
| `ev-W2c.json` | `two-process-model` — `files-as-ipc.md` | 41 | **unjudged** |
| `ev-W2d.json` | `sync-from-code` (5 files) + `wal-specspaces` (3) | 253 | **unjudged** |

**One file per slice, merged and sealed on its own, is the granularity that has
worked** — `merge-verdicts.py` takes a subset of a batch's files under the same
batch id without complaint, and a slice that lands is a slice that cannot become
a debt. `ev-W2c` carries **9 unresolvable refs that are not fictions**: all quote
`spec/WAL.md` text a checkpoint deleted, verbatim at `100617b3`. `repair-refs.py`
cannot re-point them because the text is gone; write the verdict's own citation
against the current tree instead.

**Two findings from the harvest capture alone still stand against those two files:**
the host's `two-process-model` boot snippet is missing three `{#…}` heading anchors
the package added on 2026-07-27, so three of its four sections cannot be cited (a
stale install, not a changed rule, traced by date in the harvest); and both
`CLAUDE.md:141` and `SPECSPACES.md:8` place the specspaces snippet at «slot 11 of
`spec/boot/INDEX.md`» where `grep -c` on that file returns **0** — the same shape as
F-128.

**What the tables already establish before a verdict**: the prescribed subject
`docs(spec): sync <section> with code` has been used **0 times in 2 041 commits**
while the propose-then-approve path it belongs to IS the practice, recorded three
independent ways — so only the grammar is missing, and of the three mandatory draft
parts the **revisit trigger never lands**. The installed `vibedeps/` payloads for
both W2 flows are stale by 92-176 changed lines per file, carrying 10 and 0 fact
anchors against the package's 39.

## The recipe, unchanged since W1 {#recipe}

1. **`verify-evidence.py` BEFORE reading a word** — even on a table a previous
   session already verified. This session proved why (see §traps 1).
2. **`repair-refs.py --apply`** — it re-points a moved coordinate by single-hit
   search and refuses to guess when the quote occurs twice. It now preserves the
   table's own indent, so its diff is readable.
3. **`show-rows.py --brief` row by row.** Read the subject documents first; judging
   forty rows about one file is cheaper after reading that file once.
4. **Judge every row individually**, write the batch with `src` on every verdict,
   `merge-verdicts.py`, `progress seal`, `summary.py`, commit.

**Do not touch `CONTINUE.md` or `spec/WAL.md` while judging a table that cites
them.** That is what broke 62 refs.

## What W2a and W2b settled, so W2c/W2d do not re-derive it {#w2-judged}

- **The non-adoption line, and the rest of `world` will be judged on it.** A flow's
  prescription the host simply never adopted is **not** drift: a human's morning
  read leaves no repository artefact, and the flow never claims the host performs
  one. Drift is where the host's **own written contract contradicts the flow**.
  `morning-routine.md` is unadopted end to end — no morning ritual, no weekly
  re-read — and scores 39 of 42 confirmed, with the two drifts both about the
  cold-start read order, which CLAUDE.md reverses in writing.
- **Each fact is judged on its own sentence, never on its family.**
  `NEVER-APPEND-TO-THE-WAL` prohibits appending only, and the host never appends —
  confirmed. `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` names patching too, and
  `CLAUDE.md`'s step 2 says «Update … bump … refresh» — drift. Same split for
  `NEVER-APPEND-TO-CONTINUE` against `CONTINUE-IS-OVERWRITTEN-WHOLESALE`.
- **A numeric target the host exceeds is drift**, per W1's precedent
  (`HEADER-TARGET-LENGTH-AND-HARD-LIMIT`), and so is its summary restatement.
- **A fact whose subject IS a `../flows/…` pointer is drift** (W1's 69-dangling
  family); a rule fact that merely contains such a link is judged on the rule.
- **Measured over history, not over today** — every systematic claim in W2 rests on
  a window: `_Updated:` bare in 14 of 14 revisions; Next 4-5 items in 14 of 14;
  `spec://` 0 times in 8 of 8; `_Updated:` left untouched by 10 of 17 body edits
  (58 %); the implicit hook fired on 28 of 37 active days; `CONTINUE.md` wholesale
  at all 7 wind-downs and patched in 7 of 14 commits.
- **Canonicity, with dates.** `wal` 0.2.0 landed 2026-07-07 calling itself «the
  canonical WAL convention»; `core-ai-native` v0.8.0 landed 2026-07-17, ten days
  later, and still ships a complete `06-WAL-CONVENTION.md` with zero occurrences of
  `defer`, `flow:wal` or `org.vibevm.world/wal`. The next release came; it did not
  defer. The host installs and boots both.

## The standard that judges a verdict {#standard}

> **A fact that PRESCRIBES what the discipline requires** — an intent, a
> participants list, a detector seed, a goal, a tradeoff, an alternative, a risk, a
> routine step — **is confirmed when it is coherent and every referent it names
> resolves**, including a referent the package itself declares as future work.
>
> **A fact that DESCRIBES what this repository already ships or does** — a rule id,
> a CLI signature, a returned key set, a resolution order, a floor's steps, a
> recorded measurement — **is checked against the tree, and a description that does
> not match is drift.**
>
> **A fact whose subject cannot be exercised here** (a toolchain that is not
> installed, a human's reading habit, a project that declined the convention) **is
> unverifiable, and says so in its own words** rather than by a blanket rule over a
> filename.

For `world` there is a fourth clause: **a flow's fact is also checked against the
host's observed conformance.** A rule-fact drifts on **systematic** non-compliance —
a double-digit share of the measured window — not on single exceptions the record
names by name. And see §w2-judged: non-adoption is not non-compliance.

## The findings still open {#findings}

| id | what | who closes it |
|---|---|---|
| **F-129** | the `wal` package ships two contradictory wind-downs — six steps in `session-end-hook.md`, five in a different order in `cold-resume.md`, and a third fact calling them the same | **owner** (published slot → F-122) |
| **F-124** | `H4`, `DR1-014`, `DL1-015` cited as evidence ids and resolving in no register | **owner** |
| **F-125** | `core-ai-native` v0.8.0 publishes one PLDI'25 measurement twice — 75.3 % / 70.2 % vs 74.8 % | **owner** |
| **F-126** | `rust-ai-native-tcg` names both a shipped oracle and an unbuilt masker; three names in one family point elsewhere | **owner** |
| **F-127** | the Go stack prescribes `go test -race` 15 times and passes it 0 times | an edit |
| **F-128** | `spec/boot/INLINE.md` does not exist; `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` line 5 say the four commit rules load from it | **owner** (3 host files) |

Carried and unchanged: **F-117 … F-123**, **F-114**, **F-087 / F-088**, **F-078**,
**F-092**, **F-069**, and the `specmap` ratchet's 37 gated orphans.

## What remains — `world`, W2c/W2d then W3 … W7 {#world}

Fixed in [`tasks/PHASE-C-BATCHES.json`](campaigns/packages-2026-09/tasks/PHASE-C-BATCHES.json):

| batch | packages | files | anchors |
|---|---|---:|---:|
| **W1** | the git family | 16 | 407 · **closed 90.4 %** |
| **W2** | two-process-model, wal, wal-specspaces, sync-from-code | 20 | 692 · **260 judged, 432 left** |
| **W3** | addressable-specs, decision-records, conflict-protocol | 15 | 615 |
| **W4** | campaign-plans, discovery-prompt, comparative-research, redbook | 15 | 564 |
| **W5** | operating-modes, health-audit, manual-tests, secrets-hygiene | 21 | 697 |
| **W6** | licensing, source-mirrors, spec-genres, dev-runtime-docs | 19 | 572 |
| **W7** | managed-blocks, qualified-naming, tool-design-lessons | 15 | 603 |

**Re-measure the per-anchor cost when W2 closes**; W5 is provisional at ~697 and
splits if the cost is higher than C1's. **At the phase close:** the X/Y/Z summary in
the LOG, the self-referential count (`summary.py` produces both), and
`baseline.json` (amendment A6). **Phases T and G are designed and unrun; neither
starts without an explicit instruction.**

## Mechanics — fixed, not re-invented {#mechanics}

Verdicts live in the per-file `campaign` map inside `run/cache.json`, **never in
markup** (PROP-043 §7.1/§7.5). Mutate by **load-and-merge only**.

```bash
python campaigns/packages-2026-09/tasks/verify-evidence.py <ev-*.json>
python campaigns/packages-2026-09/tasks/repair-refs.py <ev-*.json> --apply
python campaigns/packages-2026-09/tasks/merge-verdicts.py <batch.json> [--force]
cargo run -q -p vibe-cli --bin vibe -- progress seal --campaign campaigns/packages-2026-09 <paths…>
python campaigns/packages-2026-09/tasks/summary.py [--batch W1] [--by-file]
```

`merge-verdicts.py` refuses: a non-addressable anchor, a verdict outside
`{confirmed, drift, unverifiable}`, an empty or too-short `ev`, a missing `src` on
`world`, an `src` outside `world`, and a silent overwrite. **Never hand-write
`verified_at` or `processed_hash`** — `seal` writes them.

The batch files for W2a/W2b were generated by a small script holding an
index → (verdict, src, reason) map, so no anchor is transcribed by hand; W2a's is
kept at the scratchpad path named in its commit. Key by **row index**, not anchor —
two anchors repeat across files inside one table.

## Decisions still in force {#decisions}

- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling 2026-07-28). Verdicts are never delegated; neither is the review of
  delegated output. Only evidence gathering is.
- **The subject is not modified to make the measurement pass** (batch plan §2.2).
- **`vibedeps/` stands in for §3.1's third source** (§2.3), because
  `files_written = []` for all 36 lockfile packages.
- **A verdict records its source class in a field, not in prose** (§2.1 / A2).
- **Superseded version slots are marked, never verified** (§3.3).
- **The perimeter law** (batch plan §4.5): SPEC in `core-ai-native`, ENGINE in its
  library crates, DRIVER in each language stack's CLI, DEPLOYMENT in a consuming
  project. A `not-found` is a fact about the search perimeter until the perimeter
  has been checked.
- **A finding spanning a package boundary is a release event** (§5-D), not an edit.

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
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py
bash tools/self-check.sh ; echo "EXIT=$?"
cargo xtask mirror --check
```

## Standing traps {#traps}

1. **A wind-down invalidates any sealed evidence table that cites `CONTINUE.md` or
   `spec/WAL.md`.** W2's tables went from 3 unresolvable to 65 that way, none a
   fiction. Re-verify before reading, always.
2. **`run/cache.json` carries every verdict.** Load-and-merge only; no second copy.
3. **Every parsing `progress` subcommand writes the cache — `check` included.**
   Always pass `--campaign`; never point one at `campaigns/progress-2026-08`.
4. **A count including `node_modules`, `.vibe/cache/` or `vibedeps/` is a count of
   somebody else's code** — except when `vibedeps/` is the subject (source 3).
5. **An absence you assert is not an absence you checked.**
6. **Python that reads git output must decode UTF-8 explicitly** — this box's
   default codec is cp1252 and `subprocess.run(text=True)` dies on the corpus's
   em-dashes. Use `capture_output=True` + `.decode("utf-8", errors="replace")`.
7. **Never `git add -A` while a worker is running.** Stage explicit paths.
8. **Do not run a `vibe` command while `tools/self-check.sh` runs.**
9. **`grep -v '\.vibe'` deletes this repository's own packages** — the namespace is
   literally `org.vibevm`.
10. **Set `PYTHONIOENCODING=utf-8`** before any Python that prints this corpus; edit
    through editor tools, never a PowerShell round-trip.

## The recent commit chain {#commits}

```
7c1349ee docs(campaign): W2's ledger entry, and a package that disagrees with itself
958dbcf5 chore(campaign): the wal package's two wind-downs disagree with each other
a43159b9 chore(campaign): the wal flow measured against the WAL it specifies
6e567dee chore(campaign): the session-end moved 65 refs under W2's finished workers
3173ad24 fix(campaign): a repair must be readable as a diff, not as a rewrite
48025285 docs(campaign): the session-end record, and a prompt that starts by reading
8406eb2a docs(continue): W2's evidence is complete, and what it already establishes
d0e8c9f0 chore(campaign): W2's evidence is complete at 692 anchors
6fa91cdf chore(campaign): W2's last table, and a grammar nobody has ever typed
0f2991d1 chore(campaign): W2's cold-resume table, and it catches this session
100617b3 fix(campaign): the three missing words are anchors, not rule words
6726f160 chore(campaign): stop tracking bytecode the task scripts generate
6a026de1 docs(wal,continue): W2 opens with four workers out
62fc8ce8 feat(campaign): W2 opens, and source 3 already has a finding
c64fe993 fix(continue): the TL;DR and the resume prompt catch up to W1
95937de5 docs(wal,continue): W1 closed, and the loop that closed it
114b6d8d docs(campaign): W1's ledger entry, and a prediction in trouble
d0d17e9e chore(campaign): W1 closes at 407 of 407, and world reads lower
927d3cd4 chore(campaign): a procedure document closes at zero drift
6a150df1 chore(campaign): the format flow, measured against the log it governs
d1b6ca3b chore(campaign): the attribution flow measures its own law and fails it
7151142f chore(campaign): the world cluster's first verdicts, 54 of 4 150
835f07b4 chore(campaign): three of W1's five evidence tables land
902daf51 fix(campaign): the checker's fourth narrowing, and a tool for stale refs
ba3b18ca fix(campaign): four of the five git flows are stale, not all five
```

## The resume prompt {#prompt}

```text
ВОССТАНОВИ СЕССИЮ

Затем продолжи Phase C кампании PROP-043 волны 2 (campaigns/packages-2026-09).
ЗАКРЫТЫ поштучно: W2a, W2b и три файла из пяти в W2c. Пакет flow:wal завершён
(260 вердиктов, 225 / 27 / 8 — 86.5 %); two-process-model — 88 из 179, 96.6 %.
Кластер world = 678 / 62 / 15, 89.8 % на 26 файлах. Фаза C — 50.4 %.
Открыта находка F-129 (в пакете wal ДВА противоречащих друг другу wind-down).

ОСТАЛОСЬ 344 ЯКОРЯ, ТАБЛИЦЫ УЖЕ НА ДИСКЕ, В КОММИТАХ И ПРОВЕРЕНЫ:
  ev-W2c.json  50 строк — cognitive-load-split.md
  ev-W2c.json  41 — files-as-ipc.md
  ev-W2d.json  253 — sync-from-code (5 файлов) + wal-specspaces (3)
Ничего заказывать заново не нужно — начинай с чтения. Режь по одному файлу:
merge-verdicts.py спокойно принимает подмножество файлов батча под тем же id,
а закрытый срез уже не станет долгом.

Перед началом прочитай, в этом порядке:
  1. CONTINUE.md — целиком: §w2-remaining (что осталось), §recipe (цикл),
     §w2-judged (что уже решено — особенно ЛИНИЯ ПРО НЕПРИНЯТИЕ),
     §standard (стандарт вердикта), §traps
  2. campaigns/packages-2026-09/harvest/world-w2-wal-family.md — три источника W2
  3. campaigns/packages-2026-09/tasks/WORLD-WORKER-BRIEF.md — контракт делегата
  4. campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md — §2.1 (src), §4.5 (периметр)
  5. spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md §3.1, §3.2, §5
     и §9 LOG с конца — там четыре записи этой фазы

ПЕРВОЕ ДЕЙСТВИЕ: прогони verify-evidence.py по ev-W2c и ev-W2d ДО чтения —
даже если прошлая сессия их уже проверила (см. §traps 1: wind-down переписал
CONTINUE.md и WAL под уже запечатанными таблицами, 62 ссылки сломались, и ни
одна не была выдумкой). Потом repair-refs.py --apply, потом show-rows.py --brief
построчно. Сначала прочитай сами файлы-субъекты целиком, потом суди их строки.

Каждый world-вердикт ОБЯЗАН нести src — непустое подмножество [1,2,3] по §3.1;
merge-verdicts.py откажет без него. src == [1] считается отдельно как
self-referential.

Стандарт вердикта — CONTINUE.md §standard. КЛЮЧЕВОЕ, что установили W2a/W2b:
непринятие хостом предписания — это НЕ дрейф (утренний ритуал человека не
оставляет следа в репозитории, и флоу не утверждает, что хост его выполняет);
дрейф — там, где СОБСТВЕННЫЙ ПИСЬМЕННЫЙ контракт хоста противоречит флоу.
Каждый факт судится по своему предложению, а не по семье, к которой он принадлежит.
Числовая цель, которую хост превышает, — дрейф (прецедент W1).

Механика: вердикт живёт в per-file карте campaign внутри run/cache.json и НИКОГДА
в разметке; мутировать только load-and-merge; verified_at и processed_hash пишет
только `vibe progress seal`. Батч-файл генерируй скриптом с картой
индекс → (вердикт, src, причина) — ключ по ИНДЕКСУ строки, не по якорю: якоря
повторяются между файлами внутри одной таблицы. Каждое число в отчёте приходит из
команды. Вердикт без evidence ref отклоняется — «probably true» не вердикт.

ПОСЛЕ W2: перемерь стоимость на якорь (W5 стоит условными ~697 и делится, если
дороже C1), затем W3…W7 по campaigns/packages-2026-09/tasks/PHASE-C-BATCHES.json.

ДЕЛЕГИРОВАНИЕ: во встроенные агенты opus5 через инструмент Agent. Fractality НЕ
использовать. Вердикт не делегируется никогда — делегируется только сбор
доказательств; ревью делегированного — тоже твоё. НЕ правь файл, который цитирует
таблица, пока её судишь: это CONTINUE.md и spec/WAL.md.

АВТОНОМИЯ: работай полностью автоматически. Все механические операции — сама:
правки, скрипты, cargo, git commit, git push через `cargo xtask mirror`.
Останавливайся ТОЛЬКО когда нужно настоящее смысловое решение владельца.
Находка — не повод останавливаться: фаза C находки заводит, а не чинит.

НЕ ОСТАНАВЛИВАЙСЯ НА ГРАНИЦАХ РАБОТЫ. Закрытый батч, закрытый кластер, написанный
отчёт, зафиксированный коммит — это не точки сдачи хода. Сводку пиши и СРАЗУ
переходи к следующей единице работы в том же ходе.

ТОКЕНЫ НЕ ЭКОНОМЬ. Не классифицируй строки пачкой ради экономии — именно так
возник долг из 138 строк. Если контекст подходит к концу: остановись, сохрани
состояние (CONTINUE.md + spec/WAL.md + запись в LOG кампании), напиши новый
промт для продолжения по образцу этого, зафиксируй и раскатай — и скажи прямо.
```

**The WAL is the canonical living state.** If this file and `spec/WAL.md` disagree,
the WAL wins.
