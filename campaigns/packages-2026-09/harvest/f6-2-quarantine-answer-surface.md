# Ф6.2 — карантин и ответ по имени: замер перед нарезкой

## 0. Как это мерилось

Дата замера: 2026-08-17. Дерево: worktree `wt/F6-2-QUARANTINE` (проверка ветки
не выполнялась — git-глаголы запрещены пакетом; состояние дерева доказывается
прогонами ниже).

**Копия корпуса** (вся работа прогонов — только на ней; ни одна команда не
получила путь внутри `formats/corpora/`):

```
CORPUS_COPY=/tmp/tmp.OanAxY19lf/corpus
            = C:\Users\olegc\AppData\Local\Temp\tmp.OanAxY19lf\corpus
```

Копия полная, включая журнал `state/journal/{2026-07,2026-08}.ndjson`
(первичный листинг `find -maxdepth 2` журнал не показал — он на глубине 3;
повторная проверка подтвердила обе половины корпуса: журнал и каталог).

**Инструменты:**

- Чтение файлов — координаты `файл:строка` сняты чтением, не поиском.
- `rg` (ripgrep) — для полноты перечислений:
  - У5: `rg -i -n "quarantin" . -t rust` с прунами `!.git !target !vibedeps
    !node_modules !.vibe` — вывод НЕ пуст (24 кодовые строки, все перечислены
    в §1/У5); вне `crates/vibe-index` хиты только в чужом vendored
    `core-ai-native-specmap` («quarantined» как статус теста — другой смысл).
  - У10: `grep -c "\.route(" crates/vibe-index/src/server/mod.rs` → `15`.
- **Пустые выводы, читанные как ответ, и их проверка по §0.8:**
  1. У9 (`--log-level` отсутствует): `rg "log[-_]level|log_level" crates/` →
     пусто. Проверка: тот же паттерн против
     `campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md` ловит
     строки 149 и 519 (`--log-level`) — инструмент исправен, пустота — факт.
  2. «Тестов на логирование vibe-index нет»: подмножество
     `crates/vibe-index/tests/**` паттерна `VIBE_LOG|init_tracing|
     tracing_subscriber` → пусто. Проверка: тот же паттерн по
     `crates/vibe-index/src/` ловит `main.rs:25` (`VIBE_LOG`) — инструмент
     исправен; все 6 хитов по крейту сидят в `src/`, ни одного в тестах.
  3. В3 (машинерии текстов из данных нет в продукте): `rg "REGISTRY\.toml|
     breaks/|hash_recipes|vocabularies\.json" crates xtask tools` — непусто,
     но ВСЕ хиты в `xtask/**` и `tools/**` (гейты сборки); подмножество
     продуктовых крейтов (`vibe-index`, `vibe-registry`, `vibe-cli`) пусто.
     Проверка: тот же паттерн по `xtask/` ловит `wire_diff.rs:38` и далее —
     инструмент исправен.
- Прогоны: `cargo check`, `cargo build`, `cargo test`, `cargo run -p
  vibe-index --bin vibe-index -- <подкоманда>` (бинарь собран заранее, `cargo
  run -q` после сборки не шумит, stderr принадлежит программе). Коды выхода —
  самой программы, без пайпа.

## 1. Утверждения пакета: подтверждено / опровергнуто

| # | утверждение | вердикт + цитата |
|---|---|---|
| У1 | `quarantine.rs` объявляет `UNDERSTOOD = &[]` | **ПОДТВЕРЖДЕНО** — `crates/vibe-index/src/index/quarantine.rs:18`: `pub const UNDERSTOOD: &[&str] = &[];` |
| У2 | любая запись с непустым `must_understand` уходит в карантин | **ПОДТВЕРЖДЕНО** — `UNDERSTOOD` пуст (У1), значит `missing_capabilities` возвращает всё непустое: `quarantine.rs:33-39` (`filter(//!UNDERSTOOD.contains…)…collect()`); отсекается при загрузке: `memory.rs:328-348` (`pkg.versions.retain(…)` с `false` для `missing` непустого). Показано прогоном: §6, RUN 1 — версия `0.1.0` исчезла из ответа |
| У3 | наполняется ровно в одном месте, `load_from`, ~319–361, с `tracing::warn!` | **ПОДТВЕРЖДЕНО** (координаты чуть шире: цикл 315–364) — единственное наполнение `memory.rs:340` `quarantined.push(Quarantined {` внутри `pub fn load_from` (`memory.rs:315`); warn — `memory.rs:333-339`: `tracing::warn!(group = %pkg.group, name = %pkg.name, version = %v.version, missing = %missing.join(","), "quarantined: must_understand names capabilities this build lacks")`. Инициализация пустым — `memory.rs:108`. Других наполнений нет (У5-перечень) |
| У4 | корпус e1 несёт `org.vibevm/wal@0.1.0` с `must_understand: ["org.vibevm/wal/tombstone@1"]` | **ПОДТВЕРЖДЕНО** — копия корпуса, `by-name/wal.json:116-118`: `"must_understand": [` / `"org.vibevm/wal/tombstone@1"` / `]`, запись `"version": "0.1.0"` (`wal.json:65`), та же строка в `primary.jsonl:3` |
| У5 | никто не читает `Index.quarantined` | **ПОДТВЕРЖДЕНО** — полное перечисление употреблений поля (инструмент и полнота — §0): объявление `memory.rs:83`; пустая инициализация `memory.rs:108`; наполнение `memory.rs:319,340,361` (только `load_from`); чтения ТОЛЬКО в тестах: `index/memory/tests.rs:195-196` (`assert_eq!(back.quarantined.len(), 1); let q = &back.quarantined[0];`) и `journal/project_tests.rs:90` (`assert!(index.quarantined.is_empty());`). Ни один из `cli/**`, `server/**`, `tests/**`, `index_client/**` поле не трогает |
| У6 | при проекции карантин всегда пуст (Ж7); `project_tests.rs` ~90 | **ПОДТВЕРЖДЕНО** — `journal/project_tests.rs:90`: `assert!(index.quarantined.is_empty());`; механизм: `project.rs` рождает индекс через `Index::new` (`project.rs:81`) и ни одно событие не карантинит (`Published` → `idx.upsert(*entry)`, `project.rs:95-97`, без проверки `must_understand`) |
| У7 | `Cli` не несёт глобальных аргументов | **ПОДТВЕРЖДЕНО** — `cli/mod.rs:55-58`: `pub struct Cli { #[command(subcommand)] pub command: Command }` — единственное поле |
| У8 | подписчик ставится безусловно, рычаг `VIBE_LOG`, умолчание `warn`, stderr | **ПОДТВЕРЖДЕНО** — `main.rs:7` (`init_tracing();` первым действием `main`) и `main.rs:25-29`: `let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn")); … .with_writer(std::io::stderr) .try_init();`; doc-строка `main.rs:18-19`: «One lever, `VIBE_LOG` (default `warn`); there is no `RUST_LOG` fallback». Показано прогоном: §6, RUN 1/10/11 |
| У9 | флага `--log-level` нет ни у `vibe-index`, ни у `vibe` | **ПОДТВЕРЖДЕНО** — `rg "log[-_]level|log_level" crates/` → пусто (проверка инструмента — §0.1); рычаг сегодня только `VIBE_LOG` (`main.rs:25` у vibe-index; `vibe-cli/src/main.rs:411` — тот же образец) |
| У10 | таблица маршрутов ~14 вызовов `.route(` | **ПОДТВЕРЖДЕНО С ЧИСЛОМ 15, не 14** — `grep -c "\.route(" server/mod.rs` → `15` (`server/mod.rs:56-104`); см. §10 «Расхождения» |
| У11 | каталога рецептов нет; отказы — литералы «violates spec://…; fix: …» | **ПОДТВЕРЖДЕНО** — перечисление в §5: единственные читатели `formats/**` — гейты `xtask/**`; три дословных литерала в §5 |

## 2. Кто отвечает по имени: CLI

Таблица А — все 15 подкоманд (`cli/mod.rs:61-107`; координаты `run`):
«отвечает по имени» = вход содержит имя (или запрос, разрешаемый в имена), и
подкоманда возвращает состояние этого имени. Прогонные ответы — против копии
корпуса (§6).

| подкоманда | функция (файл:строка) | отвечает по имени? | известное имя | неизвестное имя (текст + код) | `--json` и тип |
|---|---|---|---|---|---|
| `get` | `cli/get.rs:43` | **да** (строго `(group,name)`) | RUN 1/2: перечень версий; `0.1.0` (карантин) бесследно отсутствует, EXIT=0 | текст: `error: invalid input: package …/… is not in the index …`, EXIT=1 (RUN 5); json: `found:false, versions:[]`, EXIT=0 (RUN 6) | есть (`get.rs:31`); рукописный `GetEnvelope` (`get.rs:34-41`) |
| `list` | `cli/list.rs:62` | нет (перечисление; но список скрывает карантинные версии — RUN 7) | перечень всех пакетов, EXIT=0 | — | есть (`list.rs:36`); рукописный `Envelope`/`PackageRow` (`list.rs:39-60`) |
| `search` | `cli/search.rs:54` | **да** (запрос → имена) | RUN 8/9: hits по `wal`; у `flow:wal` `latest_stable=0.2.0` — карантинная `0.1.0` (и её описание!) невидимы, EXIT=0 | `hits:0`, EXIT=0 (`cli_read.rs:268`) | есть (`search.rs:33`); рукописный `Envelope`/`HitRow` (`search.rs:36-52`) |
| `capabilities` | `cli/capabilities.rs:41` | нет (по возможности) | hits по capability, EXIT=0 | `hits:0` | есть (`capabilities.rs:22`); рукописный `Envelope`/`Row` (`capabilities.rs:25-39`) |
| `purls` | `cli/purls.rs:43` | нет (по PURL) | hits, EXIT=0 | `hits:0` | есть (`purls.rs:22`); рукописный (`purls.rs:25-41`) |
| `outdated` | `cli/outdated.rs:56` | **да** (имена из lockfile → статус) | построчный статус `up-to-date/update-available/unknown` | строка `status:unknown` (`cli_read.rs:376-405`) | есть (`outdated.rs:27`); рукописный (`outdated.rs:30-54`) |
| `get --version V` | тот же `run` | да (имя+версия) | RUN 3/4 | карантинная версия неотличима от несуществующей (RUN 3 ≡ RUN 5, RUN 4 ≡ RUN 6) | — |
| `verify` | `cli/verify.rs:25` | нет (целостность файлов) | отчёт о хэшах | — | есть (`verify.rs:22`); рукописный `Report` (`verify.rs:46`) |
| `dump` | `cli/dump.rs:32` | нет (дамп всего) | версии; карантин уже вырезан загрузкой | — | нет флага; `--format json\|jsonl` (`dump.rs:13-30`), сериализует `VersionEntry` |
| `init` | `cli/init.rs:62` | нет | — | — | нет |
| `reindex` | `cli/reindex.rs:107` | нет | — | — | есть (`reindex.rs:104`); отчёт |
| `rescan-org` | `cli/rescan_org.rs:56` | нет | — | — | есть (`rescan_org.rs:53`) |
| `add` | `cli/add.rs:55` | нет (запись) | — | — | нет |
| `remove` | `cli/remove.rs:40` | нет (запись по имени; ответ — отчёт мутации, не ответ каталога) | — | — | нет |
| `serve` | `cli/serve.rs:62` | сервер (см. §3) | — | — | нет |
| `stop` | `cli/stop.rs:32` | нет | — | — | нет |

## 3. Кто отвечает по имени: HTTP

Таблица Б — все 15 маршрутов `server/mod.rs:56-104` (обработчики —
`server/routes/**`):

| путь + метод | обработчик (файл:строка) | отвечает по имени? | тип ответа | когда ничего не найдено |
|---|---|---|---|---|
| `GET /healthz` | `routes/health.rs:16` | нет | рукописный `Health` (`health.rs:10-14`) | всегда 200 |
| `GET /readyz` | `routes/health.rs:25` | нет | `Health` | всегда 200 |
| `GET /v1/index/repomd.json` | `routes/index_files.rs:17` | нет | байты файла с диска | 404 `ApiError` RFC-7807 (`index_files.rs:122-128`) |
| `GET /v1/index/primary.jsonl` | `routes/index_files.rs:22` | нет | байты | 404 |
| `GET /v1/index/primary.jsonl.gz` | `routes/index_files.rs:31` | нет | байты (gzip) | 404 (`index_files.rs:36-43`) |
| `GET /v1/index/by-name/{name}` | `routes/index_files.rs:103` | **да** (сырой файл по имени) | байты `by-name/<name>.json` ДОСЛОВНО (`index_files.rs:115-116`) | 404 «`…` is not present in this index …» |
| `GET /v1/index/by-cap/{slug}` | `routes/index_files.rs:71` | нет (по возможности) | байты | 404 |
| `GET /v1/index/by-purl/{slug}` | `routes/index_files.rs:87` | нет (по PURL) | байты | 404 |
| `GET /v1/packages` (список) | `routes/packages.rs:95` | нет (перечисление) | рукописный `ListResponse` (`packages.rs:36-57`) | пустой `packages`, 200 |
| `GET /v1/packages?q=` (поиск) | `routes/packages.rs:95` (ветка `q`) | **да** (запрос → имена) | рукописный `SearchResponse` (`packages.rs:59-76`) | `hits:0`, 200 |
| `POST /v1/packages` | `routes/packages.rs:235` | нет (мутация) | рукописный `UpsertResponse` (`packages.rs:225-233`) | 400/401/403 по причине |
| `GET /v1/packages/{group}/{name}` | `routes/packages.rs:163` | **да** (строго) | рукописный `PackageVersionsResponse` (`packages.rs:183-191`) | 404 `ApiError` с текстом «`(g/n)` is not in the index (violates …; fix: …)» (`packages.rs:172`) |
| `DELETE /v1/packages/{group}/{name}` | `routes/packages.rs:350` | нет (мутация) | `DeleteResponse` (`packages.rs:302-309`) | `removed:false`, 200 |
| `GET /v1/packages/{group}/{name}/{version}` | `routes/packages.rs:193` | **да** (имя+версия) | сериализованный `VersionEntry` (сгенерированный тип) | 404 «`g/n@v` is not in the index …» (`packages.rs:210-214`) |
| `DELETE /v1/packages/{group}/{name}/{version}` | `routes/packages.rs:311` | нет (мутация) | `DeleteResponse` | `removed:false`, 200 |
| `GET /v1/capabilities/{capability}` | `routes/capabilities.rs:32` | нет (по возможности) | рукописный `Response`/`Hit` (`capabilities.rs:15-30`) | `hits:0`, 200 |
| `GET /v1/purls/{purl}` | `routes/purls.rs:32` | нет (по PURL) | рукописный `Response`/`Hit` (`purls.rs:15-30`) | `hits:0`, 200 |
| `GET /v1/admin/status` | `routes/admin.rs:25` | нет | рукописный `Status` (`admin.rs:11-23`) | всегда 200 |
| `GET /metrics` | `routes/metrics.rs:11` | нет | prometheus-текст | всегда 200 |

**Число отвечающих по имени: 6** — при правиле «вход — имя или запрос,
разрешаемый в имена; чистые перечисления и мутации не в счёт»:
CLI `get`, `search`, `outdated`; HTTP `by-name/{name}`,
`packages/{group}/{name}`, `packages/{group}/{name}/{version}`. Это совпадает
с числом «шесть», записанным в WAL. Пограничные случаи (меняют число при
другом правиле, отдельной строкой): raw-файл `by-name/{name}` — это ответ
байтами, без структуры (исключить → 5); `GET /v1/packages?q=` — поиск по
именам (включить → 7).

**Архитектурный факт, критичный для нарезки:** две поверхности читают РАЗНЫЕ
источники истины. CLI грузит каталог с диска (`Index::load_from` — здесь
карантин наполняется), а сервер грузится ИЗ ЖУРНАЛА и каталог не читает
вовсе: `cli/serve.rs:79` → `boot_index` (`serve.rs:153-176`) → `replay` +
`project`; проекция НЕ карантинит (`project.rs:95-97`, upsert без проверки
`must_understand`). Значит на сервере `Index.quarantined` ВСЕГДА пуст (Ж7),
и серверный `unavailable` в текущей форме носителя возникнуть не может — при
этом сырой маршрут `by-name/{name}` отдаёт карантинную запись файла
дословно, а клиент (`index_client`) читает её, не глядя на
`must_understand` (`index_client/mod.rs:272` — собирает только `version`).
См. §11, развилка 1.

## 4. Куда встаёт unavailable

По каждой отвечающей поверхности (§2/§3):

- **тип ответа и происхождение**: ВСЕ конверты CLI и HTTP — РУКОПИСНЫЕ
  (координаты в таблицах): `get.rs:34-41`, `list.rs:39-60`,
  `search.rs:36-52`, `capabilities.rs:25-39`, `purls.rs:25-41`,
  `outdated.rs:30-54`, `verify.rs:46`; `packages.rs:36-57/59-76/183-191`,
  `capabilities.rs:15-30`, `purls.rs:15-30`, `admin.rs:11-23`,
  `health.rs:10-14`. Реэкспорт Ф4.2c затронул ТОЛЬКО типы ЗАПИСЕЙ каталога
  (`types/entry/mod.rs:9-11`: «The definitions live in
  `vibe_wire::generated` (JTD is the source of truth)», `NameEntry`/
  `VersionEntry`/`PackageEntry` — сгенерированные из
  `schemas/index/e1/*.jtd.json` и реэкспортированные), но НЕ типы ОТВЕТА:
  конверты рукописные до сих пор. Исключение — `GET /v1/packages/{g}/{n}/{v}`
  возвращает сам `VersionEntry` (сгенерированный тип).
- **ни один формат ответа не входит в `formats/REGISTRY.toml`**: реестр
  (`formats/REGISTRY.toml:33-203`) содержит 7 `cli-*` отчётов `vibe` +
  `cli-package-tree`, файловые форматы индекса (`index-entry/repomd/primary/
  by-name/by-cap/by-purl`, строки 102-148), `manifest`, `lockfile`,
  `mcp-tools`, `config`, `journal`, `handshake` — НО НЕ конверты `vibe-index`
  (CLI `--json`) и НЕ HTTP-ответы `/v1/**`. Это находка: поверхности,
  которые читают чужие (клиент `index_client` расшифровывает их в
  `index_client/wire.rs:44-99`), не инвентаризованы. Схемы у ответов нет
  никакой — ни JTD, ни JSON Schema.
- **один общий тип или N независимых?** N независимых: у CLI и HTTP —
  РАЗНЫЕ рукописные типы на одну и ту же семантику (ср. `cli/list.rs:39-60`
  `Envelope` против `server/routes/packages.rs:36-57` `ListResponse`;
  `cli/search.rs:44-52` `HitRow` против `packages.rs:67-76` `SearchHit`;
  клиентская копия — третья: `index_client/wire.rs:55-67`). Шаг не может быть
  одной правкой типа — он N правок конвертов + 1 (возможно) правка схемы
  записи, если `unavailable` уходит в `by-name`-файл.
- **человекочитаемая ветка против `--json`**: у каждого CLI-читателя ДВЕ
  ветки (`if args.json {…} else {…}`): `get.rs:100-115` (+`render_text`
  `get.rs:119-141`), `list.rs:96-128`, `search.rs:63-98`,
  `capabilities.rs:61-83`, `purls.rs:63-85`, `outdated.rs:84-115`. На HTTP
  веток нет — всегда JSON (или байты).

## 5. Каталог рецептов: что есть сегодня

- **Машинерии, строящей пользовательский текст из данных, в дереве НЕТ.**
  Перечисление всего, что похоже (инструмент и проверка — §0.3):
  - `formats/REGISTRY.toml` — читают только `xtask/src/codegen/**`,
    `xtask/src/wire_diff.rs`, `xtask/src/strictness.rs` (гейты генерации и
    сборки);
  - `formats/EPOCHS.toml` — `xtask/src/epochs.rs` (флаги режима гейтов);
  - `formats/hash_recipes/1.toml` — параметры ВЫЧИСЛЕНИЯ хэша
    (`hash_recipe.rs`), не текст;
  - `formats/vocabularies.json` — `xtask/src/codegen/vocabulary.rs`;
  - `formats/breaks/001.md` — записка о переломе; у неё есть раздел «User
    recipe» (`breaks/001.md:67-75` — команда `vibe-index reindex …`), но её
    читают человек и гейт (`wire_diff.rs:154-168` проверяет лишь наличие
    файла через git), ни один продукт её не рендерит.
  Ни один продуктовый крейт (`vibe-index`, `vibe-registry`, `vibe-cli`) эти
  файлы не читает.
- **Домашний стиль сегодняшнего отказа — литерал в `thiserror`/`ApiError`**,
  три дословных примера:
  1. `error.rs:21-25`: `"invalid input: {0} (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli; fix: correct the argument — `vibe-index <subcommand> --help` shows the shape)"`
  2. `error.rs:37-41`: `"malformed index: {0} (violates spec://…/PROP-005#persistence; fix: re-run `vibe-index reindex` to rebuild the on-disk files)"`
  3. `error.rs:50-55`: `"unprojectable journal: {0} (violates spec://…/PROP-044#truth; fix: run `vibe-index init` when the journal carries no identity; update vibe-index when an event names a carrier this build lacks)"`
  Серверная сторона — тот же стиль внутри `ApiError::not_found`-деталей
  (`server/routes/packages.rs:172`: «`{group}/{name}` is not in the index
  (violates spec://…#http; fix: check the (group, name) identity, or publish
  the package first)»).
- **Развилка (решение — боссово, здесь обе стороны с ценой в коде):**
  - **(A) новый файл данных** (например `formats/recipes/index.toml`: ключ →
    текст рецепта). Цена: +1 формат в `formats/REGISTRY.toml` (схема или
    `none`, `foreign_parsers`), +1 загрузчик (по образцу
    `xtask/src/epochs.rs`, но в РАНТАЙМ-крейте — сегодня ни один продукт
    форматы не читает, появится первый читатель), +гейт `cargo xtask
    check-codegen` (если схема), +таблица ключей, которую Ф6.2 вызывает по
    `missing`; тесты на загрузчик.
  - **(B) обогащить запись формата в `formats/REGISTRY.toml`** полем рецепта.
    Цена: правка реестра (генерация `FormatId` не меняется — поле не
    перечисление), чтение реестра из рантайма (сегодня его читает только
    xtask — та же цена «первого читателя», плюс строгий лоадер реестра уже
    есть в `xtask/src/codegen/strictness.rs` и его придётся разделить);
    рецепт привязан к ФОРМАТУ, а не к причине (`missing`-возможности) —
    ключ weaker, текст один на все причины формата.
  - **(C) оставить домашний стиль `thiserror`** (рецепт — литерал по месту в
    новом `unavailable`-ответе). Цена: 0 файлов данных, но нарушение буквы
    Приложения А.7 («recipe — из каталога рецептов, не литерал по месту»);
    N копий текста по N поверхностей; смена текста = N правок.

## 6. Молчание, показанное прогоном

**Красное доказательство получено.** Запись существует в корпусе
(`by-name/wal.json:65,116-118`), загружается, отправляется в карантин (warn на
stderr) — и исчезает из ответа по имени БЕЗ СЛЕДА: ответ по карантинной версии
байт-в-байт совпадает с ответом по никогда не существовавшему имени.

Все прогоны — против копии `/tmp/tmp.OanAxY19lf/corpus`. Формат: команда,
EXIT, stdout, stderr (дословно).

**RUN 1 — `get`, текстовая форма:**
```
$ cargo run -q -p vibe-index --bin vibe-index -- \
    get /tmp/tmp.OanAxY19lf/corpus org.vibevm wal
EXIT=0
--- stdout:
group         : org.vibevm
name          : wal
kind          : flow
latest stable : 0.2.0
versions      : 2
  - 0.2.0 (commit -)
    content_hash: sha256:3ab76f0d29c4e1a8b5d3f7c9e2a4d6b8f1c3e5a7d9b2f4e6c8a0d3b5f7e9c1a2
    source_url  : https://gitverse.ru/vibevm/org.vibevm.wal.git
  - 1.0.0-rc.1 (commit -)
    Release candidate: hash-chain verification
    content_hash: sha256:5d9e2c7a4b8f1d3e6c9a2b5d8f1e4c7a0b3d6f9e2c5a8b1d4f7e0c3a6b9d2f5e8
    source_url  : https://gitverse.ru/vibevm/org.vibevm.wal.git
--- stderr (без всяких переменных окружения):
2026-08-17T13:21:37.457036Z  WARN quarantined: must_understand names capabilities this build lacks group=org.vibevm name=wal version=0.1.0 missing=org.vibevm/wal/tombstone@1
```
В корпусе у `org.vibevm/wal` ТРИ версии (`wal.json:65,178,192`: 0.1.0, 0.2.0,
1.0.0-rc.1). Ответ несёт две; `0.1.0` отсутствует без пояснения. При этом
именно у `0.1.0` — описание пакета («Write-ahead log discipline…»,
`wal.json:85`) и `latest_stable` в файле заявлен `0.2.0` только потому, что
`0.1.0` frozen; после карантина деградирует и поиск (RUN 8: у `flow:wal`
`description: null` — описание уехало вместе с версией).

**RUN 3 — запрос конкретно карантинной версии, текст:**
```
$ … get /tmp/…/corpus org.vibevm wal --version 0.1.0
EXIT=1
--- stdout: (пусто)
--- stderr:
2026-08-17T13:21:55.656283Z  WARN quarantined: must_understand names capabilities this build lacks group=org.vibevm name=wal version=0.1.0 missing=org.vibevm/wal/tombstone@1
error: invalid input: package `org.vibevm/wal` has no version `0.1.0` in the index (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli; fix: correct the argument — `vibe-index <subcommand> --help` shows the shape)
```

**RUN 4 ≡ RUN 6 — молчание в чистом виде (json):**
```
$ … get /tmp/…/corpus org.vibevm wal --version 0.1.0 --json    # карантинная версия
EXIT=0
{ "command": "get", "found": false, "group": "org.vibevm",
  "name": "wal", "versions": [] }

$ … get /tmp/…/corpus org.vibevm ghost --json                  # имя никогда не существовало
EXIT=0
{ "command": "get", "found": false, "group": "org.vibevm",
  "name": "ghost", "versions": [] }
```
Форма ответа идентична, код одинаков, различие — только в `name`. Существующая
непонятная версия неотличима от несуществующего имени — определение молчания
из PROP-044:78-82.

**RUN 7 — `list`:**
```
$ … list /tmp/…/corpus
EXIT=0
registry  : vibespecs
packages  : 3 (3 returned)
  com.example/wal @ 1.0.0
    A foreign tool that shares the short name
  org.vibevm/golden-probe @ 0.1.0
    Vocabulary probe: an unknown kind riding the open wire
  org.vibevm/wal @ 0.2.0
    Release candidate: hash-chain verification
--- stderr: тот же WARN (см. RUN 1)
```
Пакет виден, карантинная версия и её вклад не видны нигде.

**RUN 8/9 — `search wal`:**
```
$ … search /tmp/…/corpus wal        # и --json — то же самое
EXIT=0
query     : wal
hits      : 2
  tool:wal @ 1.0.0 (score 1)
    A foreign tool that shares the short name
  flow:wal @ 0.2.0 (score 1)
--- stderr: тот же WARN
```
В json-хите `flow:wal`: `"latest_stable": "0.2.0", "description": null` —
деградация описания показана.

**RUN 10/11 — рычаг `VIBE_LOG`:**
```
$ VIBE_LOG=debug … get … org.vibevm wal   → stderr: ровно 1 строка, 0 DEBUG-строк
$ VIBE_LOG=off  … get … org.vibevm wal    → stderr: 0 байт, EXIT=0
```
Умолчание `warn` показывает WARN без всяких переменных; `debug` НЕ добавляет
ни строки — в пути загрузки нет ни одного `debug!`-события; `off` прячет
WARN. Рычаг жив и единственен (`main.rs:25`).

**Сервер против копии не поднимался** — по букве §0.10/В4.6 это бинд порта
(`serve.rs:23` `default_value = "127.0.0.1:8412"`). Отдельно отмечено: прогон
всё равно показал бы не CLI-молчание, а другое — сервер отвечает из журнала
(`serve.rs:79,153-176`), проекция не карантинит (`project.rs:95-97`), так что
`GET /v1/packages/org.vibevm/wal` отдал бы `0.1.0` КАК ОБЫЧНУЮ версию, а
`GET /v1/index/by-name/wal.json` — файл с записью дословно
(`index_files.rs:103-116`). Это утверждение чтения, не прогона (новый тест
вышел бы за периметр записи пакета); развилка — §11.1.

**Полный перечень вызовов `Index::load_from`** (grep по `.rs` с прунами §0.6;
одноимённые методы `UserConfig`/`GlobalRegistryConfig`/`load_from_path` —
другие типы, отсеяны вручную по импортам):
- производственные, все — чтение каталога с диска для ответа:
  `cli/get.rs:44`, `cli/list.rs:63`, `cli/search.rs:55`,
  `cli/capabilities.rs:42`, `cli/purls.rs:44`, `cli/outdated.rs:57`,
  `cli/dump.rs:33` — семь;
- сервер — НЕ вызывает: подъём идёт `cli/serve.rs:79` → `boot_index`
  (`serve.rs:153`) → `journal::replay` + `journal::project`;
- тесты: `index/memory/tests.rs:108,133,184,219,225,249,254,281`;
  `tests/cli_write.rs:354`; `tests/rate_limit_e2e.rs:46`.

**Ж7 цитатой:** `journal/project_tests.rs:90` — `assert!(index.quarantined.is_empty());`
(в тесте `identity_of_a_lone_initialised_record`, комментарий механизма —
`project.rs:59-63`).


---

Разделы 7–11 (Один рычаг; Базовая линия и что покраснеет; Периметр строящего шага; Расхождения с пакетом; Открытые развилки) вынесены в `f6-2-quarantine-answer-surface-appendix.md` — раскол по шву разделов, заранее разрешённый §2 пакета: находка превысила 600 строк. Заголовки всех двенадцати разделов присутствуют дословно и в порядке — распределены по этой паре файлов.
