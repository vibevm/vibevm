# CONTINUE — cold-resume snapshot (2026-08-04, wind-down №8: ВОЛНА Б ЗАКРЫТА ЦЕЛИКОМ)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

Один длинный автономный прогон **закрыл батчи 3 и 4, то есть ВСЮ волну Б** —
48 коммитов, панель зелёная на каждой посадке, зеркала синхронны. Построены
**пять механизмов**: три правила, которых не было ни в одном языке
(`invariant-comment-position`, `cell-name-is-computed`, `declared-test-matrices`),
TS-слой кастомных линтов, статус находки «отступление признано» и ингест
SARIF. Написаны **три карточки из семи pending** — каждая потому, что у
правила появился чекер. Владелец взял **развилку №1 карты** (вычисляемые
имена) — исполнено 16 переименований. Заведены **B-050…B-053**. Реестр
пересужен и запечатан: 41 новый вердикт, 0 отказов.

**Веха M-PARITY: планка записанной причины ДОСТИГНУТА** (ни одна языковая
ячейка не слабее другой молча); сборочная полнота — нет, между ними ровно
четыре названные вещи. **Дальше — ВОЛНА В.** Мандат Б/В/Г стоит, паузы нет.

## Где стоит работа

- Ветка `main` @ `414b7224`; зеркала (gitverse + github) синхронны — раскатано
  `cargo xtask mirror` этим wind-down'ом. Дерево чистое.
- Панель зелёная — «self-check: all green» прочитан хвостом (bare-форма).
  Conform: **27 находок, все `DeviationAcknowledged`, 0 живых, 0 новых** — это
  не долг, это новая видимость (см. B-025 ниже).
- Реестр: **87 обязательств / 176 drift-вердиктов, 98.1 %**, подтверждённых
  утверждений **11 252** — меряй командами.
- Открытые аудит-строки: `AUDIT.md` §2026-08-03 (cargo-outdated, dead_code
  shadow; DBT-0023).
- Активного блокера НЕТ. Ближайшее настоящее решение владельца — развилки
  волны В (см. ниже: B-024 и B-014 решаются внутри неё).
- `.wt/` держит семь handle-locked leftover-каталогов (gitignored, git-prune
  чист) — снести позже командой из `spec/WAL.md#constraints`.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — волна В (мандат стоит, паузы нет)

Порядок задаёт карта `TOOLING-MAP.md` §4 `##WAVE-V`:

> B-013 (done) → **один формат-чейндж** (B-019а фингерпринты + B-016 половина 1
> «карта едет в пакете» + B-017 контрактные поля — вместе, это правило самих
> записей) → **B-018.1/.2** → **B-018.4 + B-016.2** → **B-020 + B-021**;
> решения B-024 и B-014 принимаются внутри волны. Выход — **M-ASK + M-DRIFT**.

**Три развилки владельца ждут внутри волны В** (`TOOLING-MAP.md` §5): №3
фингерпринты (сырой текст против токен-потока — сперва замер шума), №4 что
такое фрагмент кода, №5 содержание privacy-тира `contract`, №6 язык запросов
к карте v0, №7 судьба `disputed` при слиянии словарей. Выносить **по одной**,
деревом, `AskUserQuestion` с рекомендацией — форма работает.

**Каденс тот же, что дал волну Б:** цензус (read-only claudez-воркеры) →
босс-дизайн в `spec/design/` → claudez-стройки → панель → луп аудита →
пересуд якорей → зеркала.

## Что построила волна Б (карта посадки)

**Батч 3** — три класса правил, каждое обещано корпусом и не существовавшее:

- `invariant-comment-position` (B-036): новый `Fact::InvariantComment`, три
  экстрактора, монтаж в три гейта, шесть фикстур, два корневых ключа конфига,
  три гайда, карточка `rule-position-is-a-resource`.
- `cell-name-is-computed` (B-038, развилка №1): одно правило на Rust+Go, Go
  доносит `//spec:cell` до движка через мост в растовом написании, 16
  переименований в хосте, карточка `rule-closed-vocabulary-naming`.
- `declared-test-matrices` (B-038, R-060): битовая маска на любой глубине +
  вложенность ≥ 3 по ДИАПАЗОННЫМ циклам, карточка
  `rule-declared-test-matrices`.
- B-037: TS-плагин `@org.vibevm/eslint-plugin-ai-native` с правилом
  `diagnostic-cites-req`; Rust/Go — записанная причина + маршрут `{#b-050}`.

**Батч 4** — модель находок:

- B-025: `FindingStatus::{Live, DeviationAcknowledged}` + поле вовлечённых
  фактов; шесть правил штампуют вместо пропуска; `baseline::diff` не считает
  признанные новыми; SARIF рендерит `suppressions{kind:"inSource"}`.
- B-026: `Fact::LintDiagnosis`, корневой ключ `sarif_reports`, форма
  цитирования `Fact::cites_lint(tool, id, status)`, правило
  `LintSuppressionNeedsReason` смонтировано во все три драйвера; битый отчёт —
  отсутствие фактов, не отказ.

## Не-очевидные находки прогона (durable-уроки, полностью — в WAL #constraints)

- **Новый синтаксический признак прогоняется по живому дереву ДО посадки
  правила.** Панель дважды поправила не воркеров, а БОССОВ дизайн: голое
  `NEVER` ловило эмфазу в прозе; «вложенность ≥ 3» ловила исчерпание закрытых
  перечислений — то есть как раз ОБЪЯВЛЕННУЮ матрицу. Предъявление на
  фикстурах доказывает, что правило срабатывает, и ничего не говорит о том,
  где оно срабатывает лишнего.
- **Ложную находку НЕЛЬЗЯ замораживать в baseline** — заморозка превращает её
  в ложь, которую ратчет потом защищает. Правится правило.
- **Ручной синк вендор-копий воркером ОБЯЗАТЕЛЕН**: хостовые крейты
  path-зависят от пакетных, поэтому `cargo xtask sync-engines` сам не
  соберётся, пока копии не согласованы. Порядок мержа: применить вендор-правки
  воркера → `sync-engines --check`.
- **Рябь нового варианта `Fact` — четыре места** (два исчерпывающих матча по
  `Fact` + два по `RawFact` в Go/TS health-цензусах), а не два.
- **Грепать лог на `TASK-DONE` бесполезно** — паттерн совпадает с текстом
  инструкции внутри пакета. Завершение — только по нотификации.
- **Быстрый греп босса систематически занижает периметр** — трижды за сессию:
  окно в 8 строк пропустило три ячейки; счёт по файлам вместо правил дал 2
  вместо 6; цензус мерил без хоста и дал 10 вместо 16. Машина точнее.
- **`specmark::scope!` нужен каждому новому `.rs`, у чьих соседей он есть** —
  формулировка «в движковом крейте» пропустила файл фронтенда.

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (загрузка
  PROP-009), `design/` (рационали — non-normative), `terraforms/`, `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/` (цензусы +
  паритет-таблицы, последняя `e14-b035-parity-pass.md`), `tasks/`
  (`summary.py`, `drift-registry.py`, `merge-verdicts.py`, `evidence/`),
  `run/` (генерится), `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml` (закон
  транспорта), `OBLIGATIONS.md`, `BATCH-PLAN.md`.
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (движок conform+specmap + манифест; вендорится ×6),
  `{rust,go,typescript}-ai-native-lang/` (гайды, карточки, экстракторы,
  драйверы, CLI, фикстуры, TS-плагин линтов), `*-mcp/` (близнецы, синкаются).
  `fractality/` — специспейс.
- `packages/org.vibevm.world/` — флоу (git-practices, delegation-*, spec-genres…).
- `crates/`, `xtask/` — хост (`vibe`, `cargo xtask sync-engines`/`mirror`/
  `conform`). `vibedeps/` — рематериализованные копии. `.wt/` — worktree'ы.
- Корень: `BACKLOG.md` (P1/P2/P3, до B-053), `TOOLING-MAP.md` (волны+развилки),
  `AUDIT.md`, `TASKS.md` (**протух — описывает Phase A registry-рефакторинга**),
  `ROADMAP.md`, `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`.

## Действующие архитектурные/полит-решения (в силе)

- **Принцип паритета — закон дисциплины** (`##PARITY-ACROSS-PROJECTIONS`), и
  его близнец `##PARITY-GAP-IS-NEVER-SILENT`: пробел несёт причину И маршрут.
- **Вычисляемые имена ячеек** (развилка №1, владелец 2026-08-04):
  `Pascal(variant)` + шов как записан; проверяется одним правилом у Rust и Go.
- **Помечать, а не гасить** (владелец 2026-08-01): признанное отступление
  рождает находку со статусом, ничего не выпадает из IR.
- **Конфиг пер-язык** (v2): корень = языко-нейтральное (`max_file_lines`,
  словарь маркеров, минимальная длина файла, `sarif_reports`) + однородные
  секции `[rust]`/`[go]`/`[typescript]`; fractality-конфиг сознательно flat.
- **Хост — пакет** `org.vibevm.core/vibevm`; `spec://vibevm/…` не резолвится.
- **BUILD-FIRST**; **T/F/G вне добра**; публикация — после рефакторинга;
  версии не бампать до пред-публикации; **замеров нет** — стоячий ответ.
- **Делегация:** claudez-воркеры (GLM-5.2), закон транспорта
  `SUBAGENT-LAUNCHERS.md`; ревью/вердикты/коммиты — босс; Rules 1–4 биндят
  делегированное как прямое (человеческая атрибуция, без AI-трейлеров).
- **Роллаут — ТОЛЬКО `cargo xtask mirror`** (fast-forward, никогда `--force`).

## Недавняя цепочка коммитов (последние 26, сверху — свежие)

```
414b7224 docs(campaign): волна Б exits — the parity bar it was built for is met, and the four gaps that remain are named
27df7a8f chore(packages): vendor the SARIF ingest into the twins and rematerialise
0c6012fd feat(packages): the three drivers load lint reports before the rules run
decc5d0a feat(core-ai-native): a foreign linter's diagnosis becomes a fact the gate can cite
54712ca1 chore(packages): vendor the finding status into the twins and rematerialise
d2111008 feat(packages): six rules stamp the acknowledgement, in every language
05f8bdff feat(core-ai-native): an acknowledged deviation is MARKED, never suppressed
c626946b docs(campaign): the B-035 parity loop re-cuts after batch 3 — and names the inversion it created
52528323 docs(campaign): batch 3's anchors are judged and its eleven files re-sealed
e758db8c chore(packages): sync the narrowed sweep predicate and rematerialise
4f53e053 fix(core-ai-native): exhausting a closed set of axes IS the declared form
f71b5f26 chore(packages): vendor the matrix rule into the twins and rematerialise
834fd362 feat(packages): three extractors see a swept matrix, and R-060 gets its card
809b3af0 feat(core-ai-native): a test matrix is declared as data, never swept
6c51a357 docs(backlog): B-052 — the three halves of R3-004 that stayed unbuilt
310047f2 chore(packages): rematerialise the naming card and the guide clauses
c6f6ed4a docs(packages): the naming rule gets its card, and each half of R3-004 carries its true marker
5b697484 refactor(crates): three more cells take their computed names — the checker found what the survey missed
302ade09 chore(packages): vendor the naming rule into the six copies and rematerialise
10b4437b feat(rust-ai-native): the gate mounts the cell-name rule and the test cells take their computed names
7ea543af feat(go-ai-native): the cell manifest reaches the engine, in Rust's spelling
d990f779 feat(core-ai-native): a cell's name is checked against the name its manifest computes
a2a562e8 chore(packages): sync the narrowed vocabulary to the twins and rematerialise
712fa86d docs(packages): the position rule is described, and its card is authored
d9922aa5 fix(core-ai-native): an invariant marker is a labelled tag, not a word in prose
01c55f7d docs(backlog): B-051 — the pilot language has no conform surface spec
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # exit — настоящий, хвостом; фоном — bare-форма
cargo xtask sync-engines   # вендор ×6 после движковых правок
cargo run -q -p vibe-cli --bin vibe -- install --assume-yes   # рематериализация
cargo xtask mirror         # раскатка, fast-forward-only
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
