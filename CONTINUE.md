# CONTINUE — cold resume

**Do not quote the numbers in this file. Measure them.** Two commands, both from
the repository root, both fast:

```bash
python campaigns/packages-2026-09/tasks/batch-progress.py
```

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

The first is the progress bar: owed against judged, per batch, and it *names the
unopened files of an open batch* so the next slice is chosen from its output
rather than by hand. The second is the verdict breakdown by zone. Everything
below that looks like a measurement was true when written and is a hint about
where to look, not a fact to repeat.

---

## TL;DR

**Phase C** of the PROP-043 wave-2 campaign in `campaigns/packages-2026-09` — the
verification pass, where every anchor in every shipped package file gets a
verdict of `confirmed | drift | unverifiable` backed by evidence that resolves to
a real line in a real file.

**PHASE C IS CLOSED — 6847 / 6847 anchors, zero owed.** All seven world batches
complete (W1 407, W2 692, W3 615, W4 564, W5 697, W6 572, W7 603), and the exit
gate discharged in the plan's §7 LOG: the X/Y/Z summary (10 700 / 601 / 45 =
11 346, 94.3 %), the self-referential count (248 world verdicts at `src == [1]`,
6.0 %), and `baseline.json` at 2 216 units.

Branch `main`, clean, pushed to both mirrors.

**What is next is a decision, not a continuation.** §9 REPORT of the campaign
plan is still empty and is filled at close-out against §6. The three findings
below are filed and deliberately unrepaired — draining them is a later wave's
job. Nothing in this file is authorisation to start either.

---

## The one thing that makes this work

**The unit of work is ONE FILE, never a batch.** Read the subject file in full,
then walk its rows with `show-rows.py`, then build a batch containing that file
alone, merge it, seal it, commit it. Every attempt to classify rows in bulk has
produced verdicts that did not survive being read back.

**Every number in a report comes from a command.** A previous session computed a
phase percentage in its head and got it wrong; another wrote 89.1 % where
`summary.py` said 89.3 %.

---

## Where the answers live, and how to restore them {#recovery}

Delegated evidence is **not** recovered from the agent's chat reply. The reply is
a summary; the artefact is a file the worker writes directly into the repository:

```
campaigns/packages-2026-09/tasks/evidence/ev-<batch><pkg>.json
```

So the recovery rule is: **check the disk, not the transcript.** A worker whose
final message was lost has still left its table at that path, and
`git status --porcelain campaigns/packages-2026-09` shows it as untracked. Commit
it as returned, unjudged, and judge it later.

Two shapes seen so far: one file per package (`ev-W6a.json`), or one file per
subject file (`ev-W7a-p1.json` … `-p4.json`). `make-slice.py --file` works on
either, because it filters rows by their `file` field.

| state | how to tell | what to do |
|---|---|---|
| table landed, unjudged | `ev-*.json` exists, `batch-progress.py` lists the files as unopened | judge it, slice by slice |
| table landed, judged | the batch's files stop appearing as `unopened` | nothing |
| worker died before writing | no `ev-*.json` for that package | re-commission from the brief; nothing lost but tokens |

**Tell every worker to flush one output file per SUBJECT file, the moment that
file's anchors are complete.** This is not a preference. Three W7 workers ran on
identical briefs: two wrote per-file partials and their work was judgeable while
they were still running; the third held everything in memory and after five times
its siblings' runtime had written nothing at all, so the phase's last package sat
blocked on an agent whose entire output was unrecoverable. A worker that flushes
per file cannot lose more than one file's work, and the boss can start judging
before it finishes. Add the exact output paths to the brief so there is nothing to
decide mid-run.

**A worker that has gone quiet for several times its siblings' runtime is not
working — it is stuck.** Re-commission rather than wait. If the original returns
later it costs nothing: `make-slice.py` filters by `--file`, so two independent
passes over the same package are two tables to choose between, not a conflict.

Before reading any table, **verify it**, and if the boss has edited a file the
table cites, **repair it**:

```bash
python campaigns/packages-2026-09/tasks/verify-evidence.py campaigns/packages-2026-09/tasks/evidence/ev-W7b.json
```

```bash
python campaigns/packages-2026-09/tasks/repair-refs.py campaigns/packages-2026-09/tasks/evidence/ev-W7b.json --apply
```

This has bitten three times. The instructive one: the durable-citation rule keeps
workers off `CONTINUE.md` and `spec/WAL.md` because a wind-down rewrites them —
but **the harvest and the campaign plan are durable files the BOSS edits**, and
the briefs actively encourage citing the harvest. Editing the W4 harvest (+18
lines) and the campaign plan (+88) broke 17 refs in a table that had verified
clean. `repair-refs.py --apply` re-pointed all 17 by single-hit search and the
shifts were exactly +18 and +88. **After editing any file an evidence table
cites, re-verify and repair every table.**

---

## The prompt for the next session {#prompt}

> Продолжай **фазу C** кампании PROP-043 wave-2 в `campaigns/packages-2026-09`.
>
> **Прочти сначала:** `CONTINUE.md` целиком (особенно §recovery, §standard и
> §judged); `campaigns/packages-2026-09/tasks/WORLD-WORKER-BRIEF.md`;
> `PHASE-C-BATCH-PLAN.md` §2.1 и §4.5;
> `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` §3.1, §3.2, §5 и §7
> LOG с конца. Состояние меряй командами `batch-progress.py` и `summary.py`.
>
> **ПЕРВОЕ ДЕЙСТВИЕ:** `git status --porcelain campaigns/packages-2026-09` —
> закоммить всё, что рабочие дописали, как есть, без вердикта. Потом суди.
>
> **ЕДИНИЦА РАБОТЫ — ОДИН ФАЙЛ, НЕ БАТЧ.** Читаешь файл целиком → `show-rows.py`
> построчно → батч из одного файла → `merge-verdicts.py` → `vibe progress seal
> --campaign campaigns/packages-2026-09 <пути>` → коммит.
>
> **Каждый world-вердикт ОБЯЗАН нести `src`** — непустое подмножество [1,2,3] по
> §3.1: 1 = артефакты пакета, 2 = соответствие хоста, 3 = `vibedeps/`.
>
> **Стандарт вердикта** — §standard плюс §judged ниже. Непринятие хостом
> предписания — НЕ дрейф. Дрейф там, где написанный контракт хоста противоречит
> потоку, или где измеримое правило нарушено на двузначной доле своего окна.
> Каждый факт судится по своему предложению, не по семье. Определение, которое
> верно классифицирует провал, подтверждается ЭТИМ провалом.
>
> **Механика:** вердикт живёт в per-file карте `campaign` внутри `run/cache.json`
> и НИКОГДА в разметке; менять только load-and-merge. Батчи — через
> `make-slice.py` с картой anchor → (verdict, src, reason). **Рулинги пиши через
> `Write`, НЕ через heredoc** — bash съедает бэктики; один раз это запустило
> `tools/self-check.sh` целиком. Один рулинг-файл на ОДИН субъектный файл:
> make-slice отклоняет якоря чужого файла.
>
> **КАЖДОЕ ЧИСЛО В ОТЧЁТЕ ПРИХОДИТ ИЗ КОМАНДЫ.**
>
> **ДЕЛЕГИРОВАНИЕ:** встроенные агенты `opus5` через Agent. Fractality НЕ
> использовать. Делегируется только сбор доказательств. В брифе ОБЯЗАТЕЛЬНО:
> цитировать durable файлы (`CLAUDE.md`, `spec/boot/**`, `spec/common/**`,
> `crates/`), **НЕ цитировать `CONTINUE.md` и `spec/WAL.md`**.
>
> **АВТОНОМИЯ:** правки, скрипты, cargo, `git commit`, push через `cargo xtask
> mirror` — сам. Останавливайся ТОЛЬКО на настоящем смысловом решении владельца.
> Находка — не повод останавливаться: фаза C находки ФИКСИРУЕТ.
>
> **НЕ ОСТАНАВЛИВАЙСЯ НА ГРАНИЦАХ РАБОТЫ.** Закрытый срез, закрытый батч,
> написанный отчёт, легший коммит — не точки передачи хода.
>
> **ТОКЕНЫ НЕ ЭКОНОМЬ, и НЕ ОБЪЯВЛЯЙ ПРЕДЕЛ КОНТЕКСТА, КОТОРЫЙ НЕ ИЗМЕРИЛ.**

---

## What Phase C found

601 drifts, and four shapes recur hard enough to be worth a wave of their own:

- **The dangling sibling pointer**, in **seven consecutive W6/W7 packages**. The
  host has no `spec/flows/` directory, so every boot snippet's `../flows/…` link
  points a session at nothing. The root-relative variant inside two re-derive
  prompts is invisible to the campaign's own `\.\./flows/` scan.
- **A rule with no checker is a wish.** `source-mirrors` is the controlled
  experiment: never-`--force` has a unit test and held; never-push-to-a-replica
  and the ancestry gate have none, and both failed — 130 named-remote pushes and
  zero `merge-base` calls.
- **Verbs specified and never built** — managed-blocks' `remove`,
  qualified-naming's `KindMismatch`. Each costs five to six sentences downstream.
- **Two READMEs over-count their own contents** — `spec-genres` and
  `tool-design-lessons` both say «four pieces of content» over three shipped
  documents, where 14 of 16 siblings say «three».

**Two shape mismatches await the next `rescan`**, recorded in the LOG so they are
not read as change: 60 baseline units were omitted for want of a judged fact and
**will read as `new`**, and 58 verdict keys matched no fact anchor (the per-file
`_elements` bundles).

**A caution for whoever reads the evidence base.** 174 refs no longer resolve,
and 116 of them sit in the two W2 tables written before the durable-citation rule
existed — they point into `CONTINUE.md` and `spec/WAL.md`, which every wind-down
rewrites wholesale. Those verdicts were judged and sealed against the text as it
stood; it is the trail that rotted. `repair-refs.py --apply` has already
re-pointed everything a single-hit search could find.

---

## Findings this pass surfaced, filed and NOT repaired

Phase C records; the next wave drains. Three are worth an owner's eye:

1. **The root `README.md:164` still says vibevm is proprietary.** «vibevm itself
   ships under the proprietary EULA placeholder in [`LICENSE.md`](LICENSE.md) for
   the moment» — over a file that has been UPL-1.0 since 2026-07-12, linking to
   the file it contradicts. `CLAUDE.md:132-137` enumerates the deliberately-stale
   `"EULA"` strings and `README.md` is on none of them, while `VIBEVM-SPEC.md:8`
   IS. Three further `license = "EULA"` strings sit in
   `docs/authoring-{flow,feat,stack}.md` as example manifests authors copy.
2. **The mirrors do not carry the branches.** Both targets in `mirrors.toml`
   declare `refs = ["main", "tags"]`, so `cultural-backup`, `cultural-refactor`,
   `refactor/qualified-address-restructure` and the `fractality/*` branches exist
   on no host. If this machine were lost they would go with it.
3. **`CLAUDE.md:191` prescribes the push its own boot lane forbids.** END SESSION
   step 4 says «Push to `origin/main`»; `90-user.md:13`, `:35` and `PROP-016:15`
   all name that as *not* the rollout. The reflogs record 130 such pushes (69
   origin, 61 github) against the `cargo xtask mirror` path.

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

---

## Rulings already made, reusable {#judged}

- **A marked exception is not drift.** Where the host writes its exception down —
  an `@spec/hold` marker, a recorded owner decision, a future-trigger note — the
  rule is confirmed at N-of-M with the exception named. Where the same file
  breaks a rule the host never marked, that one is drift. This split decided
  four verdicts in `spec-genres` alone.
- **The measured window.** When a flow's rule has archived host instances and no
  live ones, the window is the current tree; the archive proves the practice was
  once adopted, which makes absence *drift* rather than non-adoption. State the
  window in the reason so it can be re-judged.
- **The 69-dangling family.** The host has no `spec/flows/` directory. A fact
  whose *subject* is a `../flows/…` pointer is drift; a rule that merely contains
  one is judged on the rule. Every W6/W7 boot snippet so far carries 1–3 of them.
  Root-relative `spec/flows/…` inside a re-derive prompt is the same defect and
  the campaign's own `\.\./flows/` scan cannot see it.
- **`UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE`** is confirmed everywhere:
  uninstall removes the vibedeps slot, drops the lockfile entry and the
  `[requires]` declaration, then regenerates boot. `files_written = []` for all
  36 packages, which is why source 3 is `vibedeps/`.
- **Delivery is not compliance.** A rule compiled into the boot lane *is*
  delivered to every session and may still be kept in 3 of 36 commits. Judge the
  sentence written; record the other half.
- **Summary restatements carry their body rule's verdict** (W1 precedent).
- **Do not rule drift on contested evidence.** Where two workers' searches
  disagreed about revisit triggers, the verdict recorded the conflict and the
  command that would settle it. A third worker later settled it.
- **An absence must be checked, not asserted** — the campaign's named trap, which
  has caught the harvest twice.

---

## Repository map

- `crates/` — the Rust workspace. `vibe-core`, `vibe-cli`, `vibe-publish`,
  `xtask` (`mirror`, `conform`, `sync-engines`).
- `spec/` — `spec/boot/` is the compiled boot lane; `spec/common/` holds
  PROP/FEAT; `spec/terraforms/` holds campaign plans; `spec/WAL.md` is the
  living checkpoint.
- `packages/org.vibevm.*/` — the shipped packages; `world/` is what Phase C
  verifies.
- `vibedeps/` — installed dependency copies. Source class 3.
- `campaigns/packages-2026-09/` — `tasks/` the instruments, `harvest/` the
  evidence gathering, `run/cache.json` the verdicts.

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

Both `make-slice.py` and `merge-verdicts.py` refuse rather than guess, and both
refusals caught real mistakes this pass. Trust them over your reading.

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
  owner override of the repo-wide default, recorded in the batch plan §6. The
  verdict is never delegated.
- **Verdicts live in `run/cache.json`, never in markup.** The package files are
  the subject under test.
- **Phase C files findings, it does not fix them.**
- **Workers cite durable files only.** See §recovery for the corollary.

---

**`spec/WAL.md` is the canonical living state.** Where it and this file disagree,
the WAL wins and this file is stale.
