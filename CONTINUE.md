# CONTINUE — cold-resume snapshot (2026-08-17, wind-down №28)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

**Закрыты ТРИ фазы подряд: Ф4.2c, Ф4.3 и Ф5.** Двадцать четыре коммита, шесть
посадок кода, панель зелёная на каждой и выросшая с 51 шага до 53.

Главное по существу: **типы индекса перестали быть рукописными** (реэкспорт
сгенерированных плюс слой поведения в крейте провода), **дублирование выхода
кодогена устранено** (102 объявления → 58 при тех же 58 именах), **рост
рукописного провода стал красным** (рехет по крейтам), и **необъявленный
перелом стал невозможен** (золотой корпус + флаги эпох + `wire-diff`).

Ценность сессии не только в коде. **Клауза «верь дереву против числа пакета»
сработала четырежды, и дважды — против БОССА:** пакет утверждал про
`specmark::scope!` в `tests/` то, что было измерено в `src/`; боссов grep
насчитал 143 файла рукописного провода, считая упоминания в ПРОЗЕ, — правильное
число 139. Оба раза исполнитель поверил дереву и назвал расхождение.

## Где стоит работа

- Ветка `main`, дерево чистое, HEAD — сворачивающие коммиты этой сессии.
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**,
  реальный код выхода 0, **53 шага** (было 51: добавились рехет провода и
  `wire-diff`).
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались более сорока посадок подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / **1035** tagged / **973** ребра / 0 подозрений /
  0 сирот. (Было 1038/976 — три ребра ушли с определениями, см. «Долги».)
- Воркеров нет, `.wt/` пуст. Два worktree под `~/.fractality/runs/**` — чужие,
  не трогать.
- Логи, пакеты, отчёты и `meta.md` с вердиктами — в
  `C:\Users\olegc\git\v\cache\agents\sorted\{F42C3-DEDUP,F42C3-BLAST,F42C3A-SHARED,F42C3B-REEXPORT,F42C4-READERS,F43-BAN,F51-CORPUS,F53-EPOCHS,F52-WIREDIFF}\`.
  **Архив живёт ВНЕ чекаута:** путь `cache/agents/...` в текстах — это
  `git/v/cache/...`, а не подкаталог репозитория.

## Блокер и действие человека

**Блокера нет.** За владельцем те же вопросы, и один из них ПОДРОС:

- **B-056** (у JTD нет 64-битного целого) **теперь держит одну конкретную
  вещь**: `Repomd`/`RepomdFileEntry` вышли из периметра реэкспорта именованным
  дефером, потому что их `size` сузился бы с `u64` до `u32`.
- **S2** (org-права), **Ж8** (`--full` и исчезнувшая запись), **B-078**
  (разрешимая правка провода журнала) — как были.

Операционное, не блокирующее: **лан `claudez` исчерпал квоту аккаунта
2026-08-17 в 14:20, сброс в 20:20**; `claudez2` жив и на нём доработана вся
вторая половина сессии.

## Что сделала эта сессия — по существу

| шаг | коммит | что построено |
|---|---|---|
| Ф4.2c-3a | `37496cab` | общий модуль: фрагмент эмитится ОДИН раз; 102 → 58 объявлений |
| Ф4.2c-3b | `53f8c429` | типы индекса стали сгенерированными; слой поведения в `vibe-wire` |
| Ф4.2c-4 | `b7464ea0` | ридеры трёх поверхностей + round-trip (G11) |
| Ф4.3 | `ee4f7230` | рехет: рост рукописного провода краснеет |
| Ф5.1 | `29043890` | золотой корпус, пересобираемый из своего журнала |
| Ф5.3 | `34d01dd2` | `formats/EPOCHS.toml` + загрузчик, который не угадывает |
| Ф5.2 | `ecd2e955` | `wire-diff`: необъявленный перелом невозможен |

Плюс три находки-замера (`467dee60`, `7eab05da`, `7f74a700`), решения Р28–Р37
и четыре новых факта запускалок.

## Шесть вещей, которые эта сессия установила

**1. Реэкспорт был невозможен по причине, которой ЧТЕНИЕ не видело.** Словник
подставляется на ВХОДЕ, поэтому один тип эмитился в семь модулей: пока копий
несколько, реэкспорт не может назвать, какую он реэкспортирует. Чтение назвало
четыре препятствия; компилятор нашёл пять причин на 209 ошибок.

**2. Шаг может разбудить дремавший закон.** Дедупликация подняла число записей
одного файла с одной до трёх, и три прогона подряд упали с `os error 1224`,
каждый на РАЗНОМ модуле. Лечение — правило проекта, не нужное всю жизнь слоя:
содержимое идёт в соседний файл, сосед забирает имя.

**3. Место шага ВЫВОДИТСЯ, а не выбирается.** Дважды за сессию: дедупликация
обязана лечь перед реэкспортом; флаги эпох — перед `wire-diff`. Когда место
кажется выбранным, ограничение обычно ещё не найдено.

**4. Нормативное значение нельзя записать дважды.** Порядок пассов чуть не
завёлся во второй копии (зеркальный конвейер для общего модуля) — и разошлись
бы они ровно там, где шаг этого не переживает.

**5. Срез забирает ХВОСТ — то есть проверку.** Оставленная работа выглядит
законченной именно потому, что недостаёт частей, которые её проверяют: корпус
не перепроецировался, расходясь с журналом на одно поле.

**6. Эталон регенерируют, а не патчат.** Разница была в одном числе; правка
руками дала бы верные байты по неверной причине.

## Что решено НЕ делать, и это решение

**`Repomd`/`RepomdFileEntry` НЕ реэкспортируются** — до рулинга по B-056.
**`CapabilityRow`/`PurlRow` НЕ реэкспортируются** — до Ф4.3-волны, где уходит
рукописный двойник; до тех пор пятый оракул остаётся живым дифференциалом.
**Печать суда (`seal`) не поставлена** — шестое сворачивание подряд; закрывает S7.
**Надгробие в корпус не дописано рукой** — его отсутствие измерено.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **Ф6: хэндшейк и карантин**, две части:

- **Ф6.1 — `hello.json`** (D8, форма в Приложении А.6). Схема
  `schemas/hello/e1/` сегодня НЕ существует, и реестр уже называет её путь —
  пасс строгости пропускает запись ПО ИМЕНИ с явной строкой в выводе. Клиент
  (`index_client`) читает хэндшейк ПЕРЕД `repomd.json`; нет файла → сегодняшний
  путь; есть → выбрать мир своей эпохи; своей нет → отказ С РЕЦЕПТОМ.
- **Ф6.2 — карантин по-настоящему.** Ответ `unavailable {name, version,
  missing, recipe}` на носителе `Index.quarantined` (Приложение А.7), плюс флаг
  `--log-level` из D11.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005`, `PROP-002`,
  `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**двадцать** находок; у
  `f42c-reexport-radius` стоят две поправки), `SUBAGENT-LAUNCHERS.md`
  (§8 — **59** размеченных фактов; `SUBAGENT-MODE.toml` = `claudez`),
  `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей, теперь с ключом `corpus`),
  **`EPOCHS.toml`** (новый: `public`, `break_window_open`),
  **`corpora/index/e1/`** (новый: журнал + спроецированный каталог),
  `vocabularies.json` (18 фрагментов), `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять) и
  `journal/e1/` (одна). `schemas/hello/` НЕ существует — дыра, которую строит Ф6.1.
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  `crates/vibe-index/src/{cli,index,server,journal}/**`,
  `crates/vibe-registry/src/index_client/**`;
  **`crates/vibe-wire/src/behaviour/`** (новый рукописный слой поведения),
  `crates/vibe-wire/src/generated/` (20 файлов, включая `shared/`),
  `xtask/src/{codegen/**,epochs.rs,wire_diff.rs,rebuild.rs}`.
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`, **`wire-derive-baseline.json`** (новый).

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Схема описывает ПРОВОД**; безусловные преобразования — те, у которых ровно
  один законный ответ.
- **Генератор эмитит ФОРМУ и никогда ПОВЕДЕНИЕ** — `Default` за этой линией.
- **Имя, которое выбирает человек, ОБЪЯВЛЯЮТ** (в том числе на корне схемы).
- **Один загрузчик — одна правда**: реестр читают трое через один загрузчик.
- **Нормативное значение не копируют** — порядок пассов живёт в одном месте.
- **Сгенерированный файл заменяют, а не перезаписывают.**
- **Каталог — проекция журнала**, починка идёт в одну сторону.
- Допубликационный режим (D13): ломать бесплатно и без миграций, `wire-diff`
  отчётный. Делегирование по умолчанию; ревью, вердикты, спеки, планы и
  коммиты — никогда не делегируются. Раскатка только `cargo xtask mirror`.
  Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
1e5d6800 docs(campaign): phase 5 closes, and its order was the forced one
ecd2e955 feat(xtask): an unannounced break stops being possible
1d8d7cd7 docs(launchers): the git ban and the perimeter proof were asking for opposites
34d01dd2 feat(formats): a period of stability becomes the state of a flag
cf3f23b2 docs(campaign): the reader cannot land before the thing it reads
c45bebcb docs(campaign): the corpus landed, and its one gap is measured
5c0c859f docs(launchers): a usage limit is a fourth way a run ends
29043890 feat(formats): the catalog gets a corpus that rebuilds from its own truth
90e48b9e docs(campaign): phase 4.3 closes, and the boss's own instrument was the drift
ee4f7230 feat(self-check): handwritten wire can no longer grow unnoticed
65986d2b docs(campaign): the next phase's perimeter aged while this one ran
324e283c docs(campaign): phase 4.2c closes, and the tree corrected the packet
b7464ea0 feat(vibe-index): what we publish, we can now read back
3ec44227 docs(campaign): the reader is generated, and the writer keeps its twin
40efb43f docs(campaign): the landing that unified the types also moved the map
53f8c429 feat(vibe-index): the index types stop being hand-written
8083c487 docs(campaign): three rulings the re-export would otherwise invent
6cfb0d30 docs(campaign): the step that landed also woke a law that had been dormant
37496cab feat(xtask): a shared fragment is emitted once, not once per schema
7f74a700 docs(harvest): the earlier radius is corrected where its own phase falsified it
e2ab2fa6 docs(launchers): a probe that edits the tree to measure it is its own genre
96df7b82 docs(campaign): the re-export cannot name which copy it re-exports
7eab05da docs(harvest): the compiler counted what reading had undercounted
467dee60 docs(harvest): the vocabulary has one home and its output has eight
6674e62a docs(handoff): the entry prompt starts from the re-export
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста, 53 шага
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo xtask wire-diff             # корпуса + флаги эпох → вердикт
cargo xtask rebuild --check formats/corpora/index/e1
cargo test -p xtask               # 217 тестов слоя, флагов и wire-diff
cargo test -p vibe-index --no-fail-fast
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
