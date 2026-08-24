# Deferrals — campaign `packages-2026-09`

Open tails at close-out land here: obligations ruled deferred, unexecuted
tasks, postponed doc chapters. A tail earns a line **only** if someone decided
to leave it — anything still being worked belongs in `tasks/INDEX.md`.

## Inherited from wave 1 at ratification {#inherited}

Wave 1 closed out on 2026-07-26 and handed this campaign work rather than
merely a method. These are not this campaign's own tails yet — they are its
**inbox**, and they should move out of this section into phases as they are
picked up. The authoritative statement of each is
[wave 1's deferrals](../progress-2026-08/deferrals.md).

- **The judgment-marking pass** — *wave 1's* Phase F (amendment A3.i). **Not this
  campaign's Phase F, which is the credibility report, and not Phase T.** The
  collision of labels is why this line now names the work instead of a letter. Wave 1 marked what
  4 917 facts *are* and was never asked what should *happen* to them, so every
  forward-looking view came out empty. This campaign marks judgment as it marks
  state, in one sweep over both corpora, so the three owner plans have an input.
- **The harvest pass and the two doc trees** — *wave 1's* Phase G (A3.ii). The User Guide
  and the Package Author Guide, the latter documenting the `packages/` corpus
  that is this campaign's own subject.
- **`FACT-GRAIN-EVIDENCE`** — wave 1's single surviving drift row, which no
  work in the host repository could close. It closes at **Phase A step 2**,
  when `rust-ai-native-lang` is re-minted at v0.8.0 with the fact-aware specmap
  engine — **now deferred by owner ruling 2026-07-26** («не перевыпускай
  пакет, сделаем это потом»).

- **F-067 — CLOSED 2026-07-26 by `e9fc7b44`, and amendment A4 is discharged
  with it.** *(Was: `processed_hash` is written only by a real verify batch, so
  a campaign that hand-seals leaves it pointing at superseded text and the
  staleness warning names the freshest files in the corpus.)* `vibe progress
  seal` has written `processed_hash` since DRIFT-026 landed — it is the file's
  own sha256, from the single `content_hash` the parser already computes, so a
  hand seal and a verify batch record the same digest. Verified by running a
  seal on a scratch copy and comparing to `sha256sum`. **F-075 asked for exactly
  this and needed no code**; DRIFT-033 added the test that was missing.
  *This line stood in the present tense for a day after the fix shipped, and two
  task files inherited the stale claim from it — which is the case for
  re-measuring a ledger entry before quoting it, not merely re-reading it.*
- **Two files need re-verifying first**: `MT-02-vibe-tree-tui.xml` and
  `PROP-026-tcg-tool-family.xml` carry wave-1 verdicts formed against text
  Phase D changed afterwards.

## Named by the change-native formats plan, moved here so they survive it {#change-native}

The change-native formats plan (`TZ-CHANGE-NATIVE-FORMATS-v0.1.md`) deferred
four things by name — one in its decision D12, three in its appendix Б.6 — and
kept them in the plan itself. That was correct while the plan was alive and is
wrong the moment it is collapsed into tombstones: **the ledger is where
deferrals live, and the next campaign's mandate drains from the ledger, not
from a dead document.** Measured 2026-08-18 before the collapse: none of the
four was in this file, against a control showing the file and the search both
worked. Each is restated here in full, so nothing has to be read out of the
plan to act on it.

- **Wave 3 of PROP-044 — the narrow public projection, generated clients, the
  codemod bot, sunset rehearsals** *(plan decision D12)*. Not built, not
  designed here, deferred as a set rather than as four separate tails, because
  they share one precondition: they are the machinery of an ecosystem with
  external consumers, and there are none yet. *Trigger:* the owner's
  declaration that the first public presentation has happened — the same single
  line that flips `public = true` in `formats/EPOCHS.toml`
  ([PROP-044 `##THE-PUBLIC-SWITCH`](../../spec/common/PROP-044-change-native-formats.xml#risks)).
  Nothing technical may infer it.
- **The lockfile's own change-native mechanics** *(Б.6, and decision D9 is its
  contract)*. `vibe.lock` should be valid only for the exact (epoch, generator
  hash, recipe ids) that built it, with any mismatch silently regenerating and
  `--locked` turning that into a loud CI error. Deferred because it travels
  with the manifest projection, and that is the next plan's subject; until
  then the lockfile lives exactly as it does today. *Trigger:* the manifest/
  lockfile plan opening.
- **Journal compaction** *(Б.6)*. Shard rotation plus checkpoints are
  sufficient for years at fixture volumes, so no compaction is built and none
  is designed. What is deliberately NOT recorded here is a threshold in
  megabytes or records: naming one now would be a number with no measurement
  behind it. *Trigger:* the first journal whose replay is slow enough for
  someone to notice — and the measurement that notices it is the threshold.
- **The content-addressed source archive** *(Б.6; it is contingency for
  PROP-044's own risk №1)*. If upstream sources vanish — deleted repositories,
  force-pushes, privatisation — rebuild-from-truth stops being possible and the
  index silently becomes authoritative, which is the way this architecture is
  most likely to be wrong. The seed already exists and needs no invention: the
  clone tree `--from-clones` that the indexer supports anyway. *Trigger:* the
  first unreachable source that the journal references — and the trigger is
  stated as an observation rather than a date precisely because the risk is not
  on a schedule.

## The engine re-mint, deferred — and what it blocks {#engine}

**RESOLVED 2026-07-26 — and by a command, not by the re-mint this section was
written to plan.** `vibe update --all` repointed every lockfile entry from the
stale second working copy to this one (`source_kind` `registry` → `local` on all
36) and carried `core-ai-native` to **0.8.0 at the consumer**, pruning the
v0.7.0 slot. Item 2 below — «the host resolves these packages from a second,
stale working copy» — was the whole blocker, and the resolution recorded at the
bottom of this section («repoint the resolve … closes the gap with no
publication at all») is what `vibe update` *already does*. **Phase C is not
blocked. No publication, no re-mint, no Rule 4 red line touched.**

*The section below is kept as written because it names the three things that
would still bind if a real re-mint is ever taken up — but its premise, that
someone must do this by hand, was false.*

~~Phase C cannot open until this is done.~~ Its evidence join needs fact
anchors and the engine the host consumes could not see them; Phase A's exit gate
was partially unreachable for the same reason (the `specmap.json` clause).
Phase B markup was never blocked and proceeded.

Confirmed before deferring, so nobody re-derives it: `is_valid_fact_id` exists
**only** in `core-ai-native/v0.8.0`; `vibe.lock` pins `core-ai-native@=0.7.0`
and `rust-ai-native-lang@=0.7.0`; `cargo xtask sync-engines --check` is green
across 33 pairs in 6 sync sets — **nothing has drifted, the gap is a version**.

Three things must be settled when it is taken up, all of them the owner's:

1. **Publishing is a Rule 4 red line** and stops for him regardless.
2. **The host resolves these packages from a second, stale working copy** —
   `file:///C:/Users/olegc/gits/vibevm/…`, last commit `c112f6f`, weeks behind
   this one. A re-mint in *this* copy is invisible to the host until the
   resolve is repointed or that copy is synced. This is an environmental fact
   about the machine, not about the repository.
3. **The network registries 401 here**, so publishing may be impossible even
   if authorised.

*Most likely resolution, recorded so it is not re-reasoned: repoint the resolve
at this copy's `packages/` and bump the lockfile locally. That closes the gap
with no publication at all — publishing is only needed for external consumers.*

## Снято рулингами релиза 1.0.0 (2026-08-20) — мандат пост-1.0 кампании {#release-1-0}

Владелец, 2026-08-20 (дословно): «Корзина 3 и всё остальное не сделанное
нужно будет сделать отдельной кампанией сразу же после завершения 1.0.0 до
конца — но потерять это ни в коем случае нельзя нужно это записать!!!»
Полный поимённый список с причинами и триггерами —
[`TZ-RELEASE-1.0-v0.1.md` §4](TZ-RELEASE-1.0-v0.1.md#not-in-scope); дубль
указателя в корне — `BACKLOG.md`, раздел «Пост-1.0». Здесь — строки,
которых ещё не было в этом ledger:

- **Фаза T (тест-рой ≥3 вида на факт) — отменена в ЭТОЙ кампании рулингом
  владельца 2026-08-20** («она очень сложная и фиксирует форму созданного,
  а мы как оказалось ещё не нашли много вещей финальной формы — фазу T
  убрать из кампании и сделать когда-нибудь потом»). Спеки
  `PHASE-T-SPEC.md` / `PHASE-T-RUNBOOK.md` / `PHASE-T-WORKER-PROMPT.md`
  остаются авторским материалом будущей кампании; при закрытии зоны их
  сохранить (зона одноразовая, работа — нет). *Триггер:* слово владельца,
  когда формы стабилизируются.
- **Фаза G полная** — пакет `org.vibevm.doc/doc`, `docs-legacy/`-архив,
  генерируемые TOC по audience-разметке (`PHASE-G-SPEC.md` остаётся
  спекой). Релиз несёт только вариант А (временный альфа-слой докуменации,
  слайс С8 ТЗ). *Триггер:* пост-1.0 кампания; предпосылка — суд-разметка
  audience.
- **Потребительская сверка `repomd.json`** (из B-084, решение D5 ТЗ) —
  вместе с вопросом дома предиката читателя (класс B-080).
- **HTTP-триггер переиндексации** (из B-085, решение D6 ТЗ). *Триггер:*
  первый оператор, которому нужен сетевой триггер.
- **CI (матрица сборки Linux/macOS/Windows)** — снят Windows-only рулингом
  дистрибутива; правка CI — красная линия, входит только словом владельца.
- **Дистрибутивы Linux/Mac** — владелец, на других машинах, после его
  ручной инспекции Windows-дистрибутива (рулинг 2026-08-20 дословно в ТЗ
  §0.9). Вне марафона по построению.
- **B-082 (гейт окна перелома меряет уже, чем обещает)** — безвреден при
  `public = false`; **обязателен ДО флипа публичного переключателя**,
  иначе в самый важный день — ложная зелень.
