# CONTINUE — cold resume

**Do not quote the numbers in this file. Measure them.**

```bash
python campaigns/packages-2026-09/tasks/batch-progress.py
```

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

Everything below that looks like a measurement was true when written and is a
hint about where to look, not a fact to repeat.

---

## TL;DR

**Phase C of the PROP-043 wave-2 campaign is CLOSED.** 6847 / 6847 anchors, zero
owed, all seven world batches complete, every judged file sealed and re-verified
against the text on disk (259 files, 0 unsealed, 0 stale). The exit gate is
discharged in all three clauses in the plan's §7 LOG.

Branch `main`, clean, in sync with both mirrors.

**Nothing is in flight.** What comes next is a decision, and this file does not
authorise any of it.

---

## The state, in numbers that came from commands

| | | |
|---|---|---|
| Phase C coverage | `batch-progress.py` | **6847/6847**, all seven batches `CLOSED` |
| Corpus verdicts | `summary.py` | **10 700 confirmed / 601 drift / 45 unverifiable** = 11 346, **94.3 %** |
| host | `summary.py` | 4 496 / 0 / 3 over 58 files — 99.9 % |
| ai-native | `summary.py` | 2 470 / 207 / 20 over 80 files — 91.6 % |
| world | `summary.py` | 3 734 / 394 / 22 over 121 files — 90.0 % |
| Self-referential (A2, clause iv) | `summary.py` | **248 of world's 4 150 — 6.0 %** |
| Baseline (A6, clause v) | `baseline.json` | **2 216 units** — 1 706 / 491 / 19 |

---

## The three candidate next steps

Pick one; none is started, and the owner decides.

### 1. Phase D — Stitching. Its entry condition is now met.

The plan's §phase-d says *«Entry: C verdicts exist for the cluster»*, and they
do. Its exit gate is *«ledger empty or every survivor is an owner-ruled
deferral»*. Its input is the **601 drift verdicts**, which cluster hard:

```
org.vibevm.ai-native   207        org.vibevm.world   394
  104  core-ai-native            50  campaign-plans
   36  rust-ai-native-lang       35  addressable-specs
   35  go-ai-native-lang         31  comparative-research
   28  typescript-ai-native-lang 30  managed-blocks
```

`core-ai-native` alone carries 104 — a sixth of everything — and it is the
package whose prose the four language families copy, which is why §phase-d
carries a wave-2-specific rule worth reading before starting: **a finding that
spans a package boundary is a release event.** Fixing `core-ai-native` may need a
version bump and a re-vendor into three family members, so such a finding is not
closed by an edit but by a published version.

**There is no `PHASE-D-*` spec or batch plan in the campaign zone.** Phases C, T
and G each have one; D does not. Drafting it is the first piece of work, and it
has to answer: what an obligation record looks like, how the 601 are clustered
into them, and how the loop-until-dry waves terminate.

### 2. §9 REPORT — the campaign's close-out, filled against §6.

Currently `*(empty — filled at close-out against §6)*`. §6 holds falsifiable
campaign-wide predictions; the report scores them. This is cheap relative to D
and answers «was the campaign worth it» directly.

### 3. Drain the three findings Phase C filed and deliberately did not repair.

Each is small, and each is the owner's call rather than a mechanical fix:

- **The root `README.md:164` still calls vibevm proprietary** — «ships under the
  proprietary EULA placeholder in [`LICENSE.md`](LICENSE.md) for the moment» —
  over a file that has been UPL-1.0 since 2026-07-12, and it links to the file it
  contradicts. It is on none of `CLAUDE.md:132-137`'s enumerated stale-string
  exemptions, while `VIBEVM-SPEC.md:8`, which says the same thing, is. Three more
  `license = "EULA"` strings sit in `docs/authoring-{flow,feat,stack}.md` as
  example manifests package authors copy.
- **The mirrors do not carry the branches.** Both targets in `mirrors.toml`
  declare `refs = ["main", "tags"]`, so `cultural-backup`, `cultural-refactor`,
  `refactor/qualified-address-restructure` and the `fractality/*` branches exist
  on no host. If this machine were lost they would go with it. The repository has
  exactly one tag, so the tags half carries almost nothing.
- **`CLAUDE.md:191` prescribes the push its own boot lane forbids.** END SESSION
  step 4 says «Push to `origin/main`»; `spec/boot/90-user.md:13`, `:35` and
  `PROP-016:15` all name that as *not* the rollout. The reflogs record 130 such
  pushes (69 origin, 61 github) against the `cargo xtask mirror` path. Two host
  documents disagree with each other, and one of them is the session contract.

---

## The prompt for the next session {#prompt}

Paste one of these. They are deliberately different because the three steps are
different kinds of work.

**For Phase D:**

> Начинай **фазу D (Stitching)** кампании PROP-043 wave-2 в
> `campaigns/packages-2026-09`. Фаза C закрыта: 6847/6847, 601 дрейф — это твой
> вход.
>
> **Прочти сначала:** `CONTINUE.md` целиком;
> `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` §5 `#phase-d`, §4.5
> (поправки) и §7 LOG с конца — там записано всё, что фаза C нашла и как;
> `campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md` как образец того, как в этой
> кампании выглядит план фазы.
>
> **ПЕРВОЕ ДЕЙСТВИЕ — план, а не правки.** У фазы D нет ни спеки, ни batch-плана,
> в отличие от C, T и G. Напиши `PHASE-D-BATCH-PLAN.md`: как выглядит запись
> обязательства, как 601 дрейф группируется в них, как волны loop-until-dry
> заканчиваются, и что считается сходимостью. Дрейфы бери из `run/cache.json`, а
> не из отчётов — вердикт с причиной лежит там.
>
> **Правило wave-2, прочти до начала:** находка, пересекающая границу пакета, —
> это релизное событие. `core-ai-native` несёт 104 дрейфа, и его прозу копируют
> четыре языковые семьи: такая находка закрывается не правкой, а опубликованной
> версией с ре-вендором через `cargo xtask sync-engines`.
>
> **Эскалация:** пара, не сошедшаяся за две волны, — концептуальный конфликт,
> и он идёт владельцу. `reality-mismatch` решается через sync-from-code с
> одобрением владельца на КАЖДЫЙ диф спеки.
>
> **КАЖДОЕ ЧИСЛО В ОТЧЁТЕ ПРИХОДИТ ИЗ КОМАНДЫ.**
>
> **АВТОНОМИЯ:** правки, скрипты, cargo, `git commit`, push через `cargo xtask
> mirror` — сам. Останавливайся только на настоящем смысловом решении владельца.
> **НЕ ОСТАНАВЛИВАЙСЯ НА ГРАНИЦАХ РАБОТЫ.** **ТОКЕНЫ НЕ ЭКОНОМЬ.**

**For the report:**

> Заполни **§9 REPORT** в `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`,
> оценивая кампанию против §6 — фальсифицируемых предсказаний, записанных при
> авторстве плана. Каждое предсказание получает вердикт с числом, полученным
> командой, а не оценкой. §7 LOG с конца — источник того, что произошло.
> Мандат владельца записан дословно в §0; отчёт отвечает на него прямо.
> **КАЖДОЕ ЧИСЛО В ОТЧЁТЕ ПРИХОДИТ ИЗ КОМАНДЫ.**

**For the findings:**

> Разбери три находки, зафиксированные фазой C и намеренно не починенные —
> они перечислены в `CONTINUE.md` и в §7 LOG за 2026-07-29. Каждая требует
> решения владельца, а не механической правки: устаревшая лицензионная строка в
> корневом `README.md` и три в `docs/authoring-*`, `refs` зеркал без веток, и
> противоречие между `CLAUDE.md:191` и `spec/boot/90-user.md`. Начни с того,
> что покажи владельцу каждую с измерением и предложи вариант; не правь до ответа.

---

## How delegated evidence works here {#recovery}

Kept because the next phase will delegate too, and this was learned the hard way.

A worker's answer is **not** in its chat reply. The reply is a summary; the
artefact is a file the worker writes directly into the repository. So: **check the
disk, not the transcript.** `git status --porcelain campaigns/packages-2026-09`
shows an untracked table whose worker's message was lost. Commit it as returned,
unjudged, and judge it later.

**Tell every worker to flush one output file per SUBJECT file, the moment that
file's anchors are complete, and name the exact paths in the brief.** Three W7
workers ran on identical briefs: two flushed per file and their work was
judgeable while they were still running; the third held everything in memory and
after five times its siblings' runtime had written nothing. A worker quiet for
several times its siblings' runtime is stuck, not thinking — re-commission. If
the original returns later it costs nothing, because `make-slice.py` filters by
`--file` and two passes over one package are two tables to choose between.

Before reading any table, **verify it**; if the boss has edited a file it cites,
**repair it**:

```bash
python campaigns/packages-2026-09/tasks/verify-evidence.py campaigns/packages-2026-09/tasks/evidence/ev-W7c.json
```

```bash
python campaigns/packages-2026-09/tasks/repair-refs.py campaigns/packages-2026-09/tasks/evidence/ev-W7c.json --apply
```

The durable-citation rule got a controlled experiment this phase. Workers are
told to cite `CLAUDE.md`, `spec/boot/**`, `spec/common/**`, `crates/` — and never
`CONTINUE.md` or `spec/WAL.md`, which every wind-down rewrites wholesale. **The
one batch written before that rule carries 116 dead refs today; every batch
written under it verifies clean.** 174 refs in the evidence base no longer
resolve and that is where they are. Those verdicts were judged and sealed against
the text as it stood — it is the trail that rotted, not the judgement.

And the corollary the boss kept forgetting: **the harvest and the campaign plan
are durable files the BOSS edits.** Editing them broke 17 refs in a table that had
verified clean. After editing any file an evidence table cites, re-verify and
repair *every* table.

---

## What Phase C found, in one screen

601 drifts, four recurring shapes:

- **The dangling sibling pointer**, in **seven consecutive W6/W7 packages**. The
  host has no `spec/flows/` directory, so every boot snippet's `../flows/…` link
  points a session at nothing. The root-relative variant inside two re-derive
  prompts is invisible to the campaign's own `\.\./flows/` scan.
- **A rule with no checker is a wish.** `source-mirrors` ran the experiment on
  itself: never-`--force` has a unit test and held; never-push-to-a-replica and
  the ancestry gate have none, and both failed — 130 named-remote pushes and zero
  `merge-base` calls.
- **Verbs specified and never built** — managed-blocks' `remove`,
  qualified-naming's `KindMismatch` (stated three times, implemented zero, its
  reserved exit code `#[allow(dead_code)]`). Each costs five to six sentences.
- **Two READMEs over-count their own contents** — `spec-genres` and
  `tool-design-lessons` say «four pieces of content» over three shipped
  documents, where 14 of 16 siblings say «three».

**Two shape mismatches await the next `rescan`**, recorded in the LOG so they are
not misread as change: 60 baseline units were omitted for want of a judged fact
and **will read as `new`**; 58 verdict keys matched no fact anchor (the per-file
`_elements` bundles).

---

## The verdict standard {#standard}

**confirmed** — the host's behaviour or written contract agrees, or it is a
definition and the thing it defines behaves as defined.

**drift** — the host's own written contract contradicts it, or a measurable rule
is broken over a double-digit share of its window. **Not adopting a package's
prescription is not drift.**

**unverifiable** — the evidence class needed to settle it does not exist here.
Say what would have settled it.

Each fact is judged **on its own sentence**, never on its family. A definition
that correctly classifies a failure is **confirmed by that failure**.

## Rulings that decided the most, reusable {#judged}

- **A marked exception is not drift.** Where the host writes its exception down —
  an `@spec/hold` marker, a recorded owner decision, a future-trigger note — the
  rule is confirmed at N-of-M with the exception named; where the same file breaks
  a rule nobody marked, that one is drift. This split alone decided four verdicts
  in `spec-genres`.
- **Delivery is not compliance.** A rule compiled into the boot lane *is*
  delivered to every session and may still be kept in 3 of 36 commits.
- **The measured window.** Archived host instances and no live ones ⇒ the window
  is the current tree, and the archive proves the practice was once adopted,
  making absence *drift*. State the window so it can be re-judged.
- **Do not rule drift on contested evidence.** Where two workers disagreed about
  revisit triggers, the verdict recorded the conflict and the command that would
  settle it; a third worker settled it later.
- **Summary restatements carry their body rule's verdict** (W1 precedent).
- **An absence must be checked, not asserted** — the campaign's named trap, which
  caught the harvests four times, twice via a truncated `grep` list read as if it
  were output.

---

## Repository map

- `crates/` — the Rust workspace: `vibe-core`, `vibe-cli`, `vibe-publish`,
  `xtask` (`mirror`, `conform`, `sync-engines`).
- `spec/` — `spec/boot/` is the compiled boot lane; `spec/common/` PROP/FEAT;
  `spec/terraforms/` campaign plans; `spec/WAL.md` the living checkpoint.
- `packages/org.vibevm.*/` — the shipped packages; `world/` and `ai-native/` are
  what this campaign judges.
- `vibedeps/` — installed dependency copies. Source class 3.
- `campaigns/packages-2026-09/` — `tasks/` the instruments, `harvest/` the
  evidence gathering, `run/cache.json` the verdicts, `baseline.json` the artefact
  the next campaign's `rescan` consumes.

## The instruments

| script | what it does |
|---|---|
| `batch-progress.py` | owed vs judged per batch; names unopened files |
| `summary.py` | verdict breakdown by zone + the self-referential count |
| `show-rows.py` | reads a worker's table row by row |
| `verify-evidence.py` | every ref resolves to a real line, or it is named |
| `repair-refs.py` | re-points refs the boss moved, by single-hit search |
| `make-slice.py` | builds one file's batch from a table plus a rulings map |
| `merge-verdicts.py` | load-and-merge into `run/cache.json`; `--force` to restate |

`make-slice.py` and `merge-verdicts.py` both refuse rather than guess, and both
refusals caught real mistakes. Trust them over your reading. **Write rulings JSON
with the `Write` tool, never a heredoc** — bash eats backticks and backslashes;
once it executed them and ran `tools/self-check.sh` in full. One rulings file per
ONE subject file: make-slice rejects anchors belonging to a neighbour.

## Quick start

```bash
cargo build -p vibe-cli
```

```bash
tools/self-check.sh
```

```bash
cargo xtask mirror
```

`mirror` is the sanctioned push — GitVerse and GitHub, fast-forward only, never
`--force`.

---

## Decisions still in force

- **This campaign delegates to built-in `opus5` subagents, not fractality** — an
  owner override of the repo-wide default, recorded in the batch plan §6.
  The verdict is never delegated.
- **Verdicts live in `run/cache.json`, never in markup.** `verified_at` and
  `processed_hash` are written only by `vibe progress seal`.
- **A phase files findings; it does not fix them.**
- **The resume boundary exists so the owner can steer.** A pointer to a next step
  in this file or the WAL is a candidate for a report, never authorisation.

---

## Recent commits

```
94e4d25b fix(campaign): that partial table's refs were not clean, and I said they were
443bad93 chore(campaign): the replacement worker's partial pass, kept as a second opinion
fac57627 docs(wal,continue): Phase C is closed, and what comes next is a decision
ef40a1ce docs(campaign): Phase C's exit gate — the summary, the count, and the baseline
7c674c18 docs(campaign): PHASE C CLOSES — qualified-naming lands the last 190 anchors
42de8b5a docs(continue): make workers flush per file, because one of them did not
b4f6cda6 docs(campaign): "word-identical" means the prose agrees, and the lockfile proves it
92874278 docs(campaign): tool-design-lessons CLOSES at 215, and one lesson drifts in its own codebase
0f56d4da docs(campaign): the other two harvest sections were checked, not assumed
4dba52ef docs(wal): W6 closed, W7 alone, and the first honest unverifiables
f12b570e fix(campaign): the durable-citation rule, vindicated by the batch written before it
d4565c7f docs(campaign): managed-blocks CLOSES at 198, and the drill asks for line numbers
```

`git log --oneline -25` for the rest.

---

**`spec/WAL.md` is the canonical living state.** Where it and this file disagree,
the WAL wins and this file is stale.
