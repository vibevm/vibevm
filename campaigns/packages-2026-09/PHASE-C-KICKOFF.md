# Phase C kick-off — the prompt for a fresh session {#root}

_Written 2026-07-28, at the Phase B close. The block under `## The prompt` is
meant to be pasted verbatim into a new session; everything else is context for
whoever is deciding whether to start._

## Read this before pasting {#before}

**Phase C is large and it is not a batch loop.** Phase B ran sixteen delegated
markup batches; Phase C is judgement, and the plan names the executor as
**"Fable + machine evidence"** — the boss, not a worker. What *is* delegable is
the evidence-gathering (§3.2's checker runs) and per-file mechanical
cross-checks; what is never delegable is the verdict.

**Size, measured 2026-07-28:** **12 797 markers** over 259 files (host 4 988 ·
ai-native 2 993 · world 4 816). `baseline.json` carries **921** judged units
from wave 1, so roughly **11 900 markers carry no verdict**. Wave 1's whole
Phase C was 4 944 markers — this is 2.6× that.

**Do not start it in a session that also intends to do something else.**

## The prompt {#prompt}

```text
ВОССТАНОВИ СЕССИЮ

Затем открой Phase C кампании PROP-043 волны 2 (campaigns/packages-2026-09).
Phase B закрыта: progress check --exhaustive выходит 0 по всем 259 файлам.

Перед началом прочитай, в этом порядке:
  1. campaigns/packages-2026-09/PHASE-C-KICKOFF.md  — этот файл целиком
  2. spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml §3.1, §3.2, §5
     (#world-verdicts, #ai-native-verdicts, #phase-c) — правила вердиктов
     и выходной гейт из пяти пунктов
  3. spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml — запись Phase C
     волны 1 (2026-07-25): механика вердиктов, семантика по стадиям,
     и чем она закрылась (4 944 маркера, 93.0 % confirmed)

Механика зафиксирована волной 1 и не изобретается заново (PROP-043 §7.1/§7.5):
вердикт живёт в per-file карте `campaign` внутри run/cache.json —
{verify_batch, verified_at, processed_hash, verdicts{anchor → {v, ev[]}},
summary} — и НИКОГДА в разметке. Мутировать cache.json только load-and-merge:
scan сохраняет карты, перезапись с нуля их сотрёт.

Каждое число в отчёте должно приходить из команды. Вердикт без evidence ref
отклоняется — «probably true» не вердикт, пиши unverifiable.

Начни с §3.2-кластера (ai-native): его чекеры надо ПРОГНАТЬ по
packages/org.vibevm.ai-native/**, и их вывод и есть доказательство для
большей части этого namespace. Выходной гейт требует, чтобы эти прогоны
лежали ФАЙЛАМИ в campaigns/packages-2026-09/harvest/ как «команда → живой
вывод» — волна 1 этот пункт пропустила и заплатила отложенной фазой
документации.

Сначала измерь и покажи мне план разбиения на батчи. Не начинай выносить
вердикты, пока я его не подтвержу.
```

## What the exit gate actually demands {#gate}

Five clauses, and clause (iii) is the one wave 1 skipped:

1. **100 % of markers carry verdicts** — `confirmed` / `drift` / `unverifiable`.
2. **The X/Y/Z summary in the LOG** — the first measured actuality level of the
   packages.
3. **The §3.2 checker runs exist as FILES** under
   `campaigns/packages-2026-09/harvest/` — floor, `conform`, `specmap` and the
   health collector over `packages/org.vibevm.ai-native/**`, each captured as
   `command → real output`.
4. **Every `world` verdict records which of §3.1's three source classes it rests
   on**, and those resting on source 1 alone are counted separately as
   **self-referential**.
5. **`baseline.json` written at the phase close.**

**The plan's own falsifiable prediction:** `world` measures higher than
`ai-native`. If that inverts, the reason is worth a finding.

## Traps that will cost real time {#traps}

1. **`run/cache.json` carries the verdicts.** Mutate by load-and-merge only. A
   from-scratch rewrite erases the C-phase maps and there is no second copy.
2. **Never point a progress subcommand at `campaigns/progress-2026-08`** — it is
   archival, and **every parsing subcommand writes the cache, `check` included**,
   which looks read-only and is not. Always pass `--campaign`.
3. **Never hand-write `verified_at`.** Sealing by hand once put timestamps in the
   future, and `moved_crate` calls a crate moved when its commits are *newer*
   than the verdict — a future stamp means nothing is ever newer and the
   invalidation rule never fires. It fails UNSAFE. Let `vibe progress seal`
   write it.
4. **Do not run a real `vibe` command while `tools/self-check.sh` is running** —
   the floor snapshots the real `~/.vibe` and a concurrent write turns it red.
5. **Four files already carry a "moved after judged" warning** from the
   `baseline.json` write at the B-close. Re-derive those before sealing.
6. **`grep -v '\.vibe'` deletes this repository's own packages** — the namespace
   is literally `org.vibevm`. Anchor filters on a path segment.

## What Phase B leaves behind that Phase C should use {#inheritance}

- **`cargo xtask batch-review`** — 15 checks, 53 hermetic controls the floor
  runs on every commit. Built for markup batches; its scope/word-stream/anchor
  checks still apply to any batch that edits files.
- **Sixty-one locked rulings** in `tasks/MARKUP-B1.md`, three struck. Phase C
  does not add markup, but the rulings record *what a unit is*, which is what a
  verdict is about.
- **The counting rule** in `BATCH-PLAN.md` and its four corrections — chiefly
  that a derived number in a plan goes stale and **nothing recomputes it**.
  Measure at the boundary; never quote a table into a brief.
- **Sixteen findings, F-102…F-116**, five of them still open on the owner.

## Open on the owner — Phase C does not unblock these {#owner}

| id | what |
|---|---|
| **F-114** | `redbook`'s edition contract falsified by its own manifest — a release decision, not an edit |
| **F-087 / F-088** | commit bodies naming a model; `ATLAS.xml` declaring a generator tracked nowhere |
| **F-078** | boot-lane duplication; DRIFT-035 written and deliberately not dispatched |
| PROP-043 §2 | the spec names what a unit **is** and never what structure **is** — two DRIFTs have moved that boundary in code |

**Phases T and G are designed and unrun. Neither starts without an explicit
instruction.**
