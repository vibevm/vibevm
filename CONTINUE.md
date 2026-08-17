# CONTINUE — cold-resume snapshot (2026-08-17, wind-down №29)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

**Ф6.1 закрыта целиком — пять шагов из пяти, тринадцать коммитов, панель
зелёная на каждой посадке, зеркала раскатаны трижды.**

Вечный файл `hello.json` существует по всей цепи: схема → сгенерированный тип →
писатель → маршрут сервера → клиент → золотой корпус. Ни одного числа в нём не
выдумано — эпохи приходят из реестра форматов через `FormatId`, часов нет,
поэтому две сборки одного состояния дают одни байты.

Ф6.2 **спроектирована и нарезана на четыре шага**, все развилки закрыты
замером (решения Р49–Р51). Следующая сессия начинает не с проектирования, а с
фан-аута.

Питание на машине пропадало один раз посреди сессии. Дерево восстановлено,
сборка доказана прогоном, потерь нет.

## Где стоит работа

- Ветка `main`, HEAD `ea3bc3e0`, дерево чистое, зеркала синхронны
  (gitverse + github).
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**,
  реальный код выхода 0, **53 шага**.
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались более пятидесяти посадок подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / **1041** tagged / **974** ребра / 0 подозрений /
  0 сирот.
- Воркеров нет, **`.wt/` пуст** — пять рабочих каталогов сессии сняты чисто.
  Два worktree под `~/.fractality/runs/**` — чужие, не трогать.
- Логи, отчёты и `meta.md` с вердиктами — в
  `C:\Users\olegc\git\v\cache\agents\sorted\{F6-1-HANDSHAKE,F6-2-QUARANTINE,F61B-SCHEMA,F61C-WRITER,F61D-CLIENT}\`.
  **Архив живёт ВНЕ чекаута:** путь `cache/agents/...` в текстах — это
  `git/v/cache/...`, а не подкаталог репозитория.

## Блокер и действие человека

**Блокера нет.** За владельцем четыре вопроса, ни один не блокирует Ф6.2:

- **B-056** (у JTD нет 64-битного целого) — держит реэкспорт
  `Repomd`/`RepomdFileEntry`.
- **S2** (org-права), **Ж8** (`--full` и исчезнувшая запись), **B-078**
  (разрешимая правка провода журнала).

## Что сделала эта сессия — по существу

| шаг | коммит | что построено |
|---|---|---|
| замер Ф6.1 | `0f73c39b` | поверхность хэндшейка измерена перед нарезкой |
| замер Ф6.2 | `24d4cc27` | молчание карантина ПОКАЗАНО прогоном |
| Ф6.1a | `c6a984e9` | поверхность писателя каталога сведена в один дом |
| Ф6.1b-0 | `e1d77d5b` | пасс преобразований учится опциональной дате |
| Ф6.1b | `2668fa55` | схема вечного файла и её оракул |
| Ф6.1c | `69bdad89` | индекс публикует хэндшейк: писатель, маршрут, корпус |
| Ф6.1d | `5c023848` | клиент спрашивает хэндшейк первым |

Плюс решения Р38–Р51 в четырёх docs-коммитах и правка устаревшего числа карты
(`4b021a01`).

## Восемь вещей, которые эта сессия установила

**1. Интерим-решение, дающее верный результат, может быть неверным решением.**
Схема воркера для опциональной даты давала тот же рустовый тип и те же байты
провода — и возвращала в дерево состояние, где у даты ДВЕ законные записи в
схемах. Отвергнута не по качеству работы, а потому что прецедент в вечном файле
дороже одной правки.

**2. Место шага выводится — и впервые окупилось НАБЛЮДАЕМО.** Единый дом
поверхности писателя (Ф6.1a) сделал Ф6.1c правкой одного места вместо двух:
`golden_corpus.rs` не потребовал правки вовсе. Раньше выигрыш вынужденного
порядка был рассуждением; здесь его видно.

**3. Порядок и воздержание доказываются наблюдением, а не прозой.** Мок с
журналом всех запрошенных путей утверждает вектор дословно: хэндшейк найден ⇒
два запроса и ни одного `repomd`. Тем же приёмом доказано, что по `successor`
НЕ ходят — адрес ведёт на путь, которого мок не знает.

**4. Правило старше числа — пять раз за сессию, все пять против автора
пакета.** Маршрутов 15, не «около 14»; карантин на 315–364; `specmark::scope!`
не несёт ни один сосед из 21; четыре теста `rebuild` не краснеют; в
`cli_search.rs` пятнадцать тестов, не восемь.

**5. Инструмент мерит не ту величину — в пятый раз, и дважды у босса.** Опрос
воркера шаблоном `[^"]*cargo` показал ноль cargo-вызовов у сделавшего семь:
класс обрывается на экранированной кавычке внутри `echo \"PROGRESS…\"`. И WAL
записал 1035 tagged в том самом коммите, где карта уже несла 1036.

**6. Дыра в инструменте ждёт того, кто её достанет.** `is_primitive` пасса
перечисляет рустовые записи ВСЕХ форм JTD `type` и несла ровно одну дыру —
`DateTime<FixedOffset>`. Обязательная дата туда не доходит (перекладывается
только опциональная), и ни одной схеме опциональная дата не была нужна до
вечного файла.

**7. Воркер может усилить конструкцию, а не исполнить букву — дважды.** Сторож
эпох сверяет реестр ещё и с ENUM (ловит реестр, отредактированный без
перегенерации). Клиент проверяет строку `vibe`, о которой пакет молчал,
сославшись на док сгенерированного поля.

**8. Развилка бывает поставлена не о том.** Каталог рецептов предлагался тремя
вариантами, и все три считали рецепт свойством ФОРМАТА; он свойство
ВОЗМОЖНОСТИ, поэтому дом уже существует и он единственный.

## Что решено НЕ делать, и это решение

**`hello.json` НЕ входит в карту `repomd.files`** — манифест одного мира не
может быть органом целостности надмирового файла. **По `successor` НЕ ходят** —
авто-переход требует сторожа циклов и правила доверия. **`min_client` в
вырожденной реализации отсутствует** — выдуманный порог пришлось бы отзывать.
**Каталог рецептов НЕ заводится** — рецепт принадлежит возможности.
**Печать суда (`seal`) не поставлена** — седьмое сворачивание подряд;
закрывает S7.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **Ф6.2, четыре шага, все развилки закрыты:**

- **Ф6.2a** — загрузчик перестаёт ВЫБРАСЫВАТЬ карантинную запись; заводится
  именованный предикат; умолчание обязано быть безопасным, и форму умолчания
  шаг называет вслух.
- **Ф6.2b** — семь глаголов CLI отвечают `unavailable {name, version, missing,
  recipe}` вместо молчания, обе ветки (текст и `--json`).
- **Ф6.2c** — шесть отвечающих по имени поверхностей HTTP; здесь же
  закрывается расхождение двух поверхностей.
- **Ф6.2d** — `--log-level` глобальным аргументом, складывающийся с `VIBE_LOG`
  в один рычаг.

Затем — приёмка всего ТЗ (§10) и сверка предсказаний P1–P6.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005`, `PROP-002`,
  `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**двадцать три** находки;
  свежие — `f6-1-handshake-surface.md`, `f6-2-quarantine-answer-surface.md` и
  её приложение), `SUBAGENT-LAUNCHERS.md` (§8 — транспортные факты;
  `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей), `EPOCHS.toml`,
  `corpora/index/e1/` (журнал + спроецированный каталог, **13 файлов**,
  включая `hello.json`), `vocabularies.json`, `hash_recipes/1.toml`,
  `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять),
  `journal/e1/` (одна) и **`hello/e1/hello.jtd.json`** (новая).
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  `crates/vibe-index/src/index/quarantine.rs` (реестр возможностей читателя —
  будущий дом рецептов), `crates/vibe-index/src/index/memory.rs` (`load_from`),
  `crates/vibe-index/src/cli/**` (семь отвечающих глаголов),
  `crates/vibe-index/src/server/routes/**`, `crates/vibe-index/src/main.rs`
  (`init_tracing`), `crates/vibe-registry/src/index_client/**` (клиент и
  `handshake.rs`).
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`, `wire-derive-baseline.json`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Схема описывает ПРОВОД**; терпимость `foreign_parsers = "many"` относится к
  незнакомым КЛЮЧАМ, а не к праву соврать о типе знакомого.
- **Одна вещь пишется одним способом** — интерим-форма со второй законной
  записью отвергается, даже когда работает.
- **Нормативное значение не копируют** — порядок пассов, поверхность писателя
  каталога, эпоха клиента.
- **Хэндшейк стоит НАД мирами** и потому вне манифеста; спрашивается ПЕРВЫМ,
  потому что этого требует `successor`.
- **Сгенерированный файл заменяют, а не перезаписывают**; эталон регенерируют,
  а не патчат.
- **Каталог — проекция журнала**; после Ф3 пути «прочитал → записал» нет.
- Допубликационный режим (D13): ломать бесплатно и без миграций, `wire-diff`
  отчётный. Делегирование по умолчанию; ревью, вердикты, спеки, планы и
  коммиты — никогда не делегируются. Раскатка только `cargo xtask mirror`.
  Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
ea3bc3e0 docs(campaign): both remaining forks were asked about the wrong thing
e2a8b60c docs(campaign): quarantine belongs where the answer is formed
489c56d3 docs(campaign): phase 6.1 closes, and the forced order paid observably
5c023848 feat(vibe-registry): the client asks the handshake before the catalog
69bdad89 feat(vibe-index): the index starts publishing its handshake
0b708bd5 docs(campaign): the eternal file found a hole nothing else could
2668fa55 feat(vibe-wire): the eternal file gets the schema it is read by
e1d77d5b fix(xtask): the shapes pass learns the one form it never met
c6a984e9 refactor(vibe-index): the catalog's surface gets one home before it grows
4b021a01 docs(wal): the map count was quoted where it should have been re-run
0c48b43e docs(campaign): phase 6 gets the slots its measurement forced
24d4cc27 docs(quarantine): the silence is shown by a run, not argued
0f73c39b docs(harvest): the handshake is measured before it is cut
626026cd docs(handoff): the entry prompt starts from a phase nobody has measured
f4fe291f docs(continue): cold-resume checkpoint
846bf7c5 docs(wal): session-end checkpoint
1e5d6800 docs(campaign): phase 5 closes, and its order was the forced one
ecd2e955 feat(xtask): an unannounced break stops being possible
1d8d7cd7 docs(launchers): the git ban and the perimeter proof were asking for opposites
34d01dd2 feat(formats): a period of stability becomes the state of a flag
cf3f23b2 docs(campaign): the reader cannot land before the thing it reads
c45bebcb docs(campaign): the corpus landed, and its one gap is measured
5c0c859f docs(launchers): a usage limit is a fourth way a run ends
29043890 feat(formats): the catalog gets a corpus that rebuilds from its own truth
90e48b9e docs(campaign): phase 4.3 closes, and the boss's own instrument was the drift
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста, 53 шага
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo xtask wire-diff             # корпуса + флаги эпох → вердикт
cargo xtask rebuild --check formats/corpora/index/e1   # 13 файлов
cargo test -p vibe-index --no-fail-fast
cargo test -p vibe-registry --no-fail-fast
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
