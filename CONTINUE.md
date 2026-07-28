# CONTINUE — cold-resume checkpoint

_Written 2026-07-28 (**Phase C: the reviewing debt is CLOSED and `world` batch W1
is CLOSED at 407 of 407**). `spec/WAL.md` is the canonical living state and
supersedes this snapshot wherever they diverge._

## TL;DR

**The 138-row reviewing debt is paid.** Every one was read individually and judged
on its own evidence: **101 confirmed / 36 drift / 3 unverifiable**, against 138
confirmed-or-unverifiable before. The `ai-native` cluster restates from 92.4 % to
**91.6 %** — 2 470 confirmed / 207 drift / 20 unverifiable over 2 697 verdicts — and
that number is now comparable across the three languages, which it was not.

**Five findings opened: F-124 … F-128.** Three are cross-language (unresolvable
evidence ids; one measurement published twice with different numbers; one tool name
carried by two different tools), one is a single missing flag, and **F-128 is the
sharpest: `spec/boot/INLINE.md` does not exist, and line 5 of `CLAUDE.md` /
`AGENTS.md` / `GEMINI.md` says the four non-negotiable commit rules are loaded first
and verbatim from it.**

**`world` batch W1 is CLOSED: 407 of 407 anchors, 368 confirmed / 32 drift / 7
unverifiable — 90.4 %, with 26 self-referential (6.4 %).** The phase predicted
`world` would measure HIGHER than `ai-native`; the first world batch reads **90.4 %
against 91.6 %**, and the reason inverts the prediction's own logic — these flows
make claims about the consuming project, and this consumer is measurable. Thirteen
of the thirty-two drifts are one law broken by its own consumer, and a second family
runs through every package: **69 relative `../flows/…` pointers in the compiled boot
lane, all 69 dangling.**

Nothing is blocked. Tree clean, in sync with `origin/main`, mirrored to both hosts.
Gate: `progress check --exhaustive` **clean, 259 files, 0 warnings**.

## Where the numbers are {#numbers}

| | |
|---|---|
| host | 58 / 58 files, 4 499 verdicts (4 496 confirmed · 3 unverifiable) |
| **ai-native** | **80 / 80 files CLOSED** — 2 697 verdicts, **2 470 / 207 / 20, 91.6 %** |
| **world** | **16 / 121 files** — 407 verdicts, **368 / 32 / 7, 90.4 %**, 26 self-referential; **3 743 anchors owed**, batches W2…W7 |
| phase | **3 104 of 6 847 — 45.3 %** |
| gate | `progress check --exhaustive` clean, 259 files, 0 warnings |
| tree | clean, in sync with `origin/main`, mirrored to GitVerse + GitHub |

**Never decrement these; re-measure.** One command now prints all of it:

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

## The recipe that closed W1, to run again on W2 {#recipe}

W1 cost five delegated tables, 1 645 refs and zero unresolvable after the checker's
fourth narrowing. The loop that produced it, in order:

1. **Capture the batch's three §3.1 sources into `harvest/`**, each `command → real
   output`. W1's is `harvest/world-w1-git-family.md`: the source-1 link join, the
   source-2/3 boot-lane join, and — for the git family only — this repository's own
   `git log` as source 2.
2. **Commission one `opus5` worker per package**, pointing each at
   `tasks/WORLD-WORKER-BRIEF.md` and the batch's harvest file, with the file list and
   the per-file anchor counts. The anchor list comes from the campaign mirror
   (`run/mirror/<path with / → __>.json`, every fact with `marked` true and a
   non-empty `id`), never from a regex.
3. **`verify-evidence.py` BEFORE reading a word**, then `repair-refs.py` if a file
   moved under the workers, then `show-rows.py --brief` row by row.
4. **Judge every row individually**, write the batch with `src` on every verdict,
   `merge-verdicts.py`, `progress seal`, `summary.py --batch <id>`, commit.

**Two operational lessons W1 paid for, both the boss's:**

- **Do not edit a file a running worker is citing.** An eleven-line insert into the
  harvest mid-run shifted every ref below it; two workers caught it themselves and
  re-anchored, and `repair-refs.py` exists for the set that will not.
- **Do not read a table while its worker may still be writing.** A verify run against
  a half-written file reported 228 fictions that were not fictions, and the
  conclusion drawn from it — a narrowing of the checker — had to be restated as
  justified-but-not-by-that-evidence.

## What W1 found, so W2 does not re-derive it {#w1-reading}

Read in full this session and worth knowing before judging:

- **`git-practices/README.md` is stale about its own family.** It lists **two**
  members in prose (`conventional-commits`, `atomic-commits`) and says the family
  «grows to include human-authored attribution and commit autonomy **as those
  members land**» — while its own `vibe.toml` already pins **all four** at `=0.1.0`
  and `vibe.lock` records the same four as its dependencies. The package ships only
  `LICENSE`, `README.md`, `vibe.toml`, so «no boot snippet of its own» is true of
  the package; the INSTALLED copy at `vibedeps/flow-git-practices/0.1.0/` carries a
  generated `spec/boot/STATIC.md` containing all four members' snippets. That
  generated file is the mechanism of **F-078** — the host compiles each git flow
  twice, once directly and once through the umbrella.
- **The single-place law is broken by the host, measurably.**
  `ATTRIBUTION-POLICY.md#single-place` says the policy is stated in «exactly one
  always-loaded place … and nowhere else». The host states it in **six**:
  `CLAUDE.md:5`, `AGENTS.md:5`, `GEMINI.md:5`, `spec/boot/00-core.md:21`, and
  `spec/boot/STATIC.md` at **:423 and :617**. `spec/common/PROP-000.md:161` does
  *not* restate it — it points at the flow by URI — which matters because
  `00-core.md:21` claims «the rule itself (and its copy in PROP-000 §12.1) is the
  only place in the project where that topic is discussed», and that claim is false
  twice over.
- **F-128's chain, verified end to end.** `link = "inline"` occurs **zero** times in
  every `vibe.toml` under `packages/` and `vibedeps/`, while all four git members
  carry the comment «suggest the inline priority lane» one line above
  `link = "static"`.
- **`AUTONOMY-PROTOCOL.md` and `ATOMIC-COMMITS-PROTOCOL.md` were read end to end.**
  The autonomy red-line list is five items — published-history rewrite, force
  operations, large binary blobs, CI/signing/secrets, anything whose reversal costs
  work — and the host restates the same five in `CLAUDE.md` rule 4 and
  `spec/boot/00-core.md`'s `RULE-AUTONOMY`. A force-push leaves no positive trace in
  a log, so a verdict resting on «no force-push is visible» is an absence asserted,
  not checked.
- The three §3.1 sources for W1 are captured by command in
  [`harvest/world-w1-git-family.md`](campaigns/packages-2026-09/harvest/world-w1-git-family.md) —
  cite it by path and line rather than re-deriving.

## The standard that judges a verdict {#standard}

Written down this session because it had not been, after the same claim turned out
to have been judged two ways in three languages:

> **A fact that PRESCRIBES what the discipline requires** — an intent, a
> participants list, a detector seed, a goal, a tradeoff, an alternative, a risk, a
> routine step — **is confirmed when it is coherent and every referent it names
> resolves**, including a referent the package itself declares as future work (a
> card registry's `specified` column, a brief's «vision, NOT an implementation
> plan» status line).
>
> **A fact that DESCRIBES what this repository already ships or does** — a rule id,
> a CLI signature, a returned key set, a resolution order, a floor's steps, a
> recorded measurement — **is checked against the tree, and a description that does
> not match is drift.**
>
> **A fact whose subject cannot be exercised here** (a toolchain that is not
> installed) **is unverifiable, and says so in its own words** rather than by a
> blanket rule over a filename.

For `world` there is a fourth clause, and it is §3.1's whole point: **a flow's fact
is also checked against the host's observed conformance.** The host is a living
consumer; if a flow states a law and the host breaks it, that is source-2 evidence
and it is what this cluster exists to measure.

## The findings this session opened {#findings}

| id | what | who closes it |
|---|---|---|
| **F-124** | `H4`, `DR1-014`, `DL1-015` cited as evidence ids and resolving in no register; the `H`-series is in daily use across `core-ai-native`'s appendices with no roster anywhere | **owner** |
| **F-125** | `core-ai-native` v0.8.0 publishes one PLDI'25 measurement twice — ATLAS 75.3 % / 70.2 %, CONTRADICTION-MAP 74.8 % — and four documents across three stacks quote whichever they read | **owner** |
| **F-126** | `rust-ai-native-tcg` names both a shipped consultation oracle and an unbuilt token-level masker; with `vibe-tcg-ts` and `vibe-tcg`, three names in one family point elsewhere | **owner** |
| **F-127** | the Go stack prescribes `go test -race` 15 times across 5 documents and passes it 0 times | an edit |
| **F-128** | `spec/boot/INLINE.md` does not exist; `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` line 5 say the four commit rules load from it and are not restated elsewhere | **owner** (3 host files) |

Carried and unchanged: **F-117 … F-123**, plus **F-114**, **F-087 / F-088**,
**F-078**, **F-092**, **F-069**, and the `specmap` ratchet's 37 gated orphans.

## What remains — `world`, W1 … W7 {#world}

Seven batches, 121 files, 4 150 anchors, fixed in
[`tasks/PHASE-C-BATCHES.json`](campaigns/packages-2026-09/tasks/PHASE-C-BATCHES.json):

| batch | packages | files | anchors |
|---|---|---:|---:|
| **W1** | the git family — practices, atomic-commits, conventional-commits, attribution-policy, autonomy | 16 | 407 |
| **W2** | two-process-model, wal, wal-specspaces, sync-from-code | 20 | 692 |
| **W3** | addressable-specs, decision-records, conflict-protocol | 15 | 615 |
| **W4** | campaign-plans, discovery-prompt, comparative-research, redbook | 15 | 564 |
| **W5** | operating-modes, health-audit, manual-tests, secrets-hygiene | 21 | 697 |
| **W6** | licensing, source-mirrors, spec-genres, dev-runtime-docs | 19 | 572 |
| **W7** | managed-blocks, qualified-naming, tool-design-lessons | 15 | 603 |

**W2 and W5 are provisional at 692 and 697.** The unit is anchors; re-measure the
per-anchor cost after W1 closes and split them if it is higher than C1's.

**At the phase close:** the X/Y/Z summary in the LOG, the self-referential count
(`summary.py` produces both), and `baseline.json` (amendment A6). **Phases T and G
are designed and unrun; neither starts without an explicit instruction.**

## Mechanics — fixed, not re-invented {#mechanics}

Verdicts live in the per-file `campaign` map inside `run/cache.json`, **never in
markup** (PROP-043 §7.1/§7.5). Mutate by **load-and-merge only**.

```bash
python campaigns/packages-2026-09/tasks/merge-verdicts.py <batch.json> [--force]
cargo run -q -p vibe-cli --bin vibe -- progress seal --campaign campaigns/packages-2026-09 <paths…>
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py [--batch W1] [--by-file]
```

`merge-verdicts.py` refuses: a non-addressable anchor, a verdict outside
`{confirmed, drift, unverifiable}`, an empty or too-short `ev`, a missing `src` on
`world`, an `src` outside `world`, and a silent overwrite.

**Never hand-write `verified_at` or `processed_hash`** — `seal` writes them, and a
hand-written stamp fails UNSAFE (`moved_crate` calls a crate moved when its commits
are *newer* than the verdict).

## The campaign tools, and what each removes from the reviewer {#tools}

| tool | what it removes |
|---|---|
| `merge-verdicts.py` | the six rules that are trivially broken by hand |
| `verify-evidence.py` | whether a ref points where it says (3 947 refs checked, 12 unresolvable — 0.3 %) |
| `show-rows.py` | opening the cache by hand per anchor — the arithmetic that makes a reviewer skim |
| `summary.py` | the X/Y/Z rollup and the self-referential count the exit gate needs and nothing ships |
| `source1-join.py` | does every cited document exist and carry its anchor |
| `source23-boot-join.py` | does the host receive and carry what each boot snippet claims |
| `scaffold-three-way.py` · `coordinate-divergence.py` | the parallel-corpus diffs |

## Decisions still in force {#decisions}

- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling 2026-07-28). Verdicts are never delegated; neither is the review of
  delegated output. Only evidence gathering is.
- **The subject is not modified to make the measurement pass** (batch plan §2.2):
  no `<lang>-ai-native init` against a package under verification. The unmodified
  run, refusal included, is the evidence.
- **`vibedeps/` stands in for §3.1's third source** (§2.3), because
  `files_written = []` for all 36 lockfile packages.
- **A verdict records its source class in a field, not in prose** (§2.1 / A2).
- **Superseded version slots are marked, never verified** (§3.3): `redbook` v0.1.0
  and `core-ai-native` v0.7.0.
- **The perimeter law** (batch plan §4.5): a mechanism's SPEC lives in
  `core-ai-native`, its ENGINE in that package's library crates, its DRIVER in each
  language stack's CLI, its DEPLOYMENT in a consuming project. A `not-found` is a
  fact about the search perimeter until the perimeter has been checked.
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

1. **`run/cache.json` carries every verdict.** Load-and-merge only; no second copy.
2. **Every parsing `progress` subcommand writes the cache — `check` included.**
   Always pass `--campaign`; never point one at `campaigns/progress-2026-08`.
3. **A count including `node_modules`, `.vibe/cache/` or `vibedeps/` is a count of
   somebody else's code** — except when `vibedeps/` is the subject (source 3).
4. **An absence you assert is not an absence you checked.**
5. **A Python `str.replace` with `\n` no-ops on this tree** — CRLF working copy, LF
   blobs. Use an editor tool that errors on a missed match.
6. **Never `git add -A` while a worker is running.** Stage explicit paths.
7. **Do not run a `vibe` command while `tools/self-check.sh` runs.**
8. **`grep -v '\.vibe'` deletes this repository's own packages** — the namespace is
   literally `org.vibevm`.
9. **Set `PYTHONIOENCODING=utf-8`** before any Python that prints this corpus;
   PowerShell 5.1 corrupts UTF-8-no-BOM round-trips, so edit through editor tools.

## The recent commit chain {#commits}

```
dbd20c74 chore(campaign): the derived corpus projection catches up
9281cfe9 docs(campaign): F-128 — the boot lane the contract reads first is absent
f0b8109b feat(campaign): the two counts the exit gate asks for and nothing ships
e8438cda feat(campaign): W1 opens with its three sources on disk
f00f28c9 docs(campaign): the debt's ledger entry and four findings it opened
fcb195b6 chore(campaign): the Rust half, and what comparing twins moved
fc46cfd0 chore(campaign): the Go half of the debt, read row by row
01260aaa feat(campaign): the reviewer's join, so reading is not arithmetic
1824fac9 docs(wal): the cluster that gates everything except itself
a45bb31e docs(continue): cold-resume at the ai-native close
a7556d7e docs(campaign): the resume document, and the debt it opens with
6d82b5cf chore(campaign): the ai-native cluster closes at 80 of 80 files
d9270e75 chore(campaign): C4+C5 in two languages, and a name that outlived its crate
106e09c5 chore(campaign): C6 closes at 92.7 % and corrects C3 twice
6702441a chore(campaign): C3 closes at 89.7 %, and Go's gap is the tree's
55975e60 chore(campaign): C3a — the demos are the consumer, and forty facts turn on it
89c90aed docs(campaign): F-123 — we break a rule we ship, at a fifth of commits
9cabe34d docs(campaign): F-122 — one coordinate, two contents, 173 times
bf679a1c chore(campaign): C7 closes at 99/99, and F-116 is about the family
c8911c29 feat(campaign): F-116 stops being a reading and becomes a command
4b266611 feat(campaign): C4's parallel corpus gets diffed instead of re-read
76c6a142 chore(campaign): C2 closes at 92.4 %, and the drift is one thing said eleven ways
2ff1cbed fix(campaign): an elided quote is one rule, not a list of cases
0413154a chore(campaign): C2a — the ATLAS keeps its books and misstates its own source
666fe2c6 feat(campaign): sources 2 and 3 become one command, over the boot lane
```

## The resume prompt {#prompt}

```text
ВОССТАНОВИ СЕССИЮ

Затем продолжи Phase C кампании PROP-043 волны 2 (campaigns/packages-2026-09).
Долг ревью ЗАКРЫТ, батч W1 ЗАКРЫТ: 407 якорей, 368 / 32 / 7 — 90.4 %, из них 26
self-referential. Кластер ai-native пересчитан в 91.6 % (2 470 / 207 / 20). Открыты
находки F-124…F-128. Осталось world W2…W7: 105 файлов, 3 743 якоря.

Перед началом прочитай, в этом порядке:
  1. CONTINUE.md — целиком, особенно §in-flight, §w1-reading и §standard
  2. campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md — §2.1 (поле src),
     §4.5 (закон периметра), §5 (как цитируется harvest)
  3. campaigns/packages-2026-09/tasks/WORLD-WORKER-BRIEF.md — контракт делегата
  4. campaigns/packages-2026-09/harvest/world-w1-git-family.md — три источника W1
  5. spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md §3.1, §3.2, §5
     и §9 LOG с конца — записи этой фазы, включая стандарт вердикта

ПЕРВОЕ ДЕЙСТВИЕ — батч W2 (two-process-model, wal, wal-specspaces,
sync-from-code: 20 файлов, 692 якоря). Рецепт, которым закрыт W1, лежит в
CONTINUE.md §recipe — выполняй его, а не изобретай: снять три источника §3.1 в
harvest/, заказать по воркеру на пакет по tasks/WORLD-WORKER-BRIEF.md, прогнать
verify-evidence.py ДО чтения, читать show-rows.py поштучно, судить каждую строку.
Перемерь стоимость на якорь — W2 и W5 стоят условными ~695 и делятся, если дороже
C1.

Каждый world-вердикт ОБЯЗАН нести src — непустое подмножество [1,2,3] по §3.1;
merge-verdicts.py откажет без него. src == [1] считается отдельно как
self-referential: `python campaigns/packages-2026-09/tasks/summary.py --batch W1`.

Стандарт вердикта зафиксирован в CONTINUE.md §standard и в §9 LOG — не изобретай
заново. Для world есть четвёртый пункт: факт флоу проверяется ещё и наблюдаемым
поведением хоста (§3.1 источник 2), и хост — живой потребитель.

Механика: вердикт живёт в per-file карте campaign внутри run/cache.json и НИКОГДА
в разметке; мутировать только load-and-merge; verified_at и processed_hash пишет
только `vibe progress seal`. Каждое число в отчёте приходит из команды. Вердикт
без evidence ref отклоняется — «probably true» не вердикт, пиши unverifiable.

ДЕЛЕГИРОВАНИЕ: во встроенные агенты opus5 через инструмент Agent. Fractality НЕ
использовать. Вердикт не делегируется никогда — делегируется только сбор
доказательств; ревью делегированного — тоже твоё.

АВТОНОМИЯ: работай полностью автоматически. Все механические операции — сама:
правки, скрипты, cargo, git commit, git push через `cargo xtask mirror`.
Останавливайся ТОЛЬКО когда нужно настоящее смысловое решение владельца.
Находка — не повод останавливаться: фаза C находки заводит, а не чинит.

НЕ ОСТАНАВЛИВАЙСЯ НА ГРАНИЦАХ РАБОТЫ. Закрытый батч, закрытый кластер, написанный
отчёт, зафиксированный коммит — это не точки сдачи хода. Сводку пиши и СРАЗУ
переходи к следующей единице работы в том же ходе.

ТОКЕНЫ НЕ ЭКОНОМЬ. Не классифицируй строки пачкой ради экономии — именно так
возник долг из 138 строк. Если контекст подходит к концу: остановись, сохрани
состояние (CONTINUE.md + spec/WAL.md + запись в §9 LOG кампании), напиши новый
промт для продолжения по образцу этого, зафиксируй и раскатай — и скажи прямо.
```

**The WAL is the canonical living state.** If this file and `spec/WAL.md` disagree,
the WAL wins.
