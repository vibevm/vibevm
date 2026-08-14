# Ф3 — что ломается в шести RMW-путях и как `reindex --full` порождает сброс

Чем мерил: только чтение дерева `crates/**` этого worktree плюс кампейн-документы
(`campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md`,
`…/harvest/f0-rmw-volume.md`). `cargo` и `git` НЕ запускались — ни сборки, ни
статуса, ни лога; длины файлов — `wc -l`, поиски — `grep -rn` (приведены в §11).
Дата замера: 2026-08-14. Дерево — после посадок Ф1.3, Ф1.4, Ф1.5, Ф2.1, Ф2.2,
Ф2.3 (координаты f0 устарели, сверка — в §9).

Все пути ниже относительны корня `crates/vibe-index/`, если не сказано иное.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ.** Все шесть путей механически сводятся к шаблону
`validate → append → project(replay) → write_to`, и существующий писатель
переиспользуется как есть (`Index::write_to` уже принимает `&WriteCtx` и
штампует манифест от `ctx.at` — `index/memory.rs:197`, `index/memory.rs:296`).
Но четыре решения сегодня берутся из прочитанного состояния и потоком
`Published`-событий не выражаются — каждое нужно отдельным решением босса:

1. **Идентичность реестра входит в мутацию из прочитанного.** `cli/add.rs`
   строит `source_url` из прочитанных `index.registry_url` + `index.naming`
   (`cli/add.rs:81`) и штампует `entry.registry` прочитанным `index.registry`
   (`cli/add.rs:99`); `cli/reindex.rs` собирает `FromClonesOptions` и свежий
   `Index::new` из прочитанных `registry`/`registry_url`/`naming`
   (`cli/reindex.rs:234-236`, `cli/reindex.rs:253-255`). В плановом enum
   `Event` (ТЗ А.2, `TZ-CHANGE-NATIVE-FORMATS-v0.1.md:894-909`) этих полей нет
   — проектор их из событий не соберёт. Нужен отдельный вход (§4).
2. **`--full` — это не дозапись событий, а сброс состояния.** Свежий
   `Index::new` получает ТОЛЬКО `report.entries` (`cli/reindex.rs:252-257`,
   `cli/reindex.rs:289-291`): записи, которых сканер не увидел, исчезают молча
   (§5, эффект Э1). Поток `Published` умеет только добавлять/заменять —
   «записи больше нет» он не выражает. Нужно событие сброса или компенсирующие
   `Removed` (открытые вопросы — §5).
3. **Ответы `created` / `removed` / «nothing to remove» — решения из
   прочитанного.** Сервер вычисляет `existed` до мутации
   (`server/routes/packages.rs:253-256`), `r = remove_version/remove_package`
   (`server/routes/packages.rs:317`, `:347`); CLI `remove` на отсутствии
   ОТВЕЧАЕТ ОШИБКОЙ (`cli/remove.rs:50-61`), сервер — 200 с `removed: false`
   (`server/routes/packages.rs:329-335`). При журнале это «проверка по
   проекции», но симметрию ответов (ошибка vs 200) босс не зафиксировал.
4. **`generator` — мутируемое поле каталога.** `run_plan` перетирает его
   версией сегодняшнего бинаря на каждом reindex (`cli/reindex.rs:237`,
   `cli/reindex.rs:258`); событием это не является и в плановый enum не входит.

Сами по себе `upsert`/`remove_*`/`write_to` заменяются событиями без потерь:
`upsert` уже возвращает `changed: bool` (Ф2.3, `index/memory.rs:120-135`),
`remove_*` — `removed: bool` (`index/memory.rs:141-160`), и обе семантики
проектор воспроизводит применением `Published`/`Removed`.

## 2. Сверка опорных координат (B1..B8)

| # | утверждение | вердикт | цитата |
|---|---|---|---|
| B1 | 7 продакшн-вызовов `write_to`: add:130, init:54, reindex:292, remove:62, packages:268/:320/:350 | **ПОДТВЕРЖДЕНО** | `cli/add.rs:130`; `cli/init.rs:54`; `cli/reindex.rs:292`; `cli/remove.rs:62`; `server/routes/packages.rs:268`, `:320`, `:350`. Полный вывод grep (включая тестовые сайты) — в §11; продакшн-сайтов ровно семь, восьмого нет |
| B2 | `load_from` как вход мутации — add:56, remove:40, reindex:224 + стартовая serve:71; остальные — читающие | **ПОДТВЕРЖДЕНО** | входы мутаций: `cli/add.rs:56`, `cli/remove.rs:40`, `cli/reindex.rs:224`; старт: `cli/serve.rs:71`. Читающие (7): `cli/capabilities.rs:42`, `cli/dump.rs:33`, `cli/get.rs:44`, `cli/list.rs:58`, `cli/outdated.rs:57`, `cli/purls.rs:44`, `cli/search.rs:50` |
| B3 | серверные мутации читают in-memory `Index` за `RwLock` в `AppState`, загруженный один раз | **ПОДТВЕРЖДЕНО** | `server/state.rs:30` (`pub index: RwLock<Index>`); загрузка один раз — `cli/serve.rs:71`, передача в `AppState` — `cli/serve.rs:93-99`; per-request чтение — `server/routes/packages.rs:237`, write-lock — `:252` |
| B4 | `run_plan` берёт registry/registry_url/naming из прочитанного и перетирает generator после `Index::new` | **ПОДТВЕРЖДЕНО** | `cli/reindex.rs:216` (объявление); `cli/reindex.rs:234-236` (opts из `existing.*`); `cli/reindex.rs:253-255` (`Index::new` из `existing.*`); `cli/reindex.rs:258` (`next.generator = opts.generator.clone()`). Уточнение: `generator` не читается из каталога — он заново компонуется (`cli/reindex.rs:237`), т. е. reindex ГАРАНТИРОВАННО меняет генератор на текущий бинарь |
| B5 | режим — строковое сравнение `plan.mode == "incremental"`; значения — строковые литералы | **ПОДТВЕРЖДЕНО** | `cli/reindex.rs:241`, `cli/reindex.rs:260`; поле `mode: &'static str` — `cli/reindex.rs:207`; литералы рождаются в `cli/reindex.rs:141-145` и `cli/rescan_org.rs:72`. Других `mode ==` в `crates/vibe-index/src` нет (§11) |
| B6 | при `--full` в свежий `Index::new` попадают только `report.entries` — невиденные записи исчезают молча | **ПОДТВЕРЖДЕНО** | инкрементальный блок переноса — только под `if plan.mode == "incremental"` (`cli/reindex.rs:260-288`); безусловный перенос — `cli/reindex.rs:289-291`; режим по умолчанию — full (`cli/reindex.rs:141-145`: `full` unless `--incremental`) |
| B7 | `checkpoint::save` вызывается независимо от режима; при full чекпойнт перезаписывается свежим сканом | **ПОДТВЕРЖДЕНО** | `cli/reindex.rs:296-301` (безусловный вызов); комментарий-признание: «Persist the new checkpoint regardless of mode — incremental walks pick it up next time, full walks reset it» — `cli/reindex.rs:294-295`; содержимое — только `report.snapshots` (`cli/reindex.rs:299`) |
| B8 | дефер Ф1.4 «полный reindex стирает надгробия» | **ПОДТВЕРЖДЕНО** | дефер записан в ТЗ: «Полный `reindex` стирает надгробия. `cli/reindex.rs` строит свежий `Index::new` с пустым носителем надгробий, а `write_to` чистит каталог `by-name/`» — `TZ-CHANGE-NATIVE-FORMATS-v0.1.md:538-543`. Механика в дереве: `Index::new` даёт пустые `tombstones` (`index/memory.rs:109`); `run_plan` никогда не переносит `existing.tombstones` в `next` (`cli/reindex.rs:252-291` — переноса нет); `write_to` сносит `by-name/` (`index/memory.rs:210`) и надстраивает надгробия только из `self.tombstones` (`index/memory.rs:242-247`). Уточнение: **инкрементальный режим стирает надгробия ровно так же** — fresh `Index::new` и перенос только версий (`iter_versions`, `cli/reindex.rs:261`) не отличают режимов; дефер назван уже, чем дефект |

## 3. Шесть RMW-путей: карточка на каждый

Общая механика для всех шести: `write_to` — писатель ПОЛНОЙ проекции (сносит
`by-name/`, `by-cap/`, `by-purl/`, переписывает `primary.jsonl[.gz]`, оба
инвертированных индекса, штампует `repomd.json` последним —
`index/memory.rs:210-302`). Каждая мутация переписывает весь каталог из всего
in-memory состояния — «писатель принимает на вход то, что опубликовал».

### 3.1 `cli/add.rs::run` (объявление `cli/add.rs:49-132`)

- **Что читает из опубликованного:** весь каталог — `Index::load_from`
  (`cli/add.rs:56`, с маппингом ошибки в «Run `vibe-index init` first» —
  `cli/add.rs:57-63`); из прочитанного — `index.registry_url` и `index.naming`
  (`cli/add.rs:81`), `index.registry` (`cli/add.rs:99`).
- **Решение из прочитанного:** (а) дефолтный `source_url`, когда флаг
  `--repo-url` не задан — `args.repo_url.unwrap_or_else(|| compose_default_repo_url(&index.registry_url, index.naming, …))`
  (`cli/add.rs:80-82`, сама функция — `cli/add.rs:134-144`); (б) штамп
  `entry.registry = index.registry.clone()` (`cli/add.rs:99`). Веток
  created/removed здесь нет: `bool` от `upsert` отброшен (`cli/add.rs:129`).
- **Чем станет при журнале:** (а) → «нужен новый вход» (идентичность реестра
  приходит не из каталога, а из конфига/манифеста/события инициализации, §4);
  (б) → то же самое; upsert+write_to → `append(Published{entry})` +
  `write_to(project(replay()))`. Сама `compose_default_repo_url` чистая и
  выживает как есть (`cli/add.rs:134-144`).
- **Что физически ломается:** связка `index` — продукт `load_from`
  (`cli/add.rs:56`), на ней держатся `cli/add.rs:81`, `cli/add.rs:99`,
  `cli/add.rs:129-130` — при удалении load_from эти строки перестают
  компилироваться (нет объекта). Поведенческие тесты выживают: `tests/cli_write.rs`
  утверждает на `by-name/*.json` (`tests/cli_write.rs:71-78`, `:110-116`,
  `:145-151`), проекция порождает те же файлы; юнит-тест
  `add_projects_manifest_frozen_into_the_entry` (`cli/add.rs:163-203`) сеет
  каталог через `Index::new().write_to` (`cli/add.rs:168-175`) — seed-форма
  меняется, тело выживает.
- **Классификация:** переписать.
- **Строк затронуто:** ~14 из 204 (load-блок `cli/add.rs:56-63` = 8; compose
  `:80-82` = 3; registry `:99` = 1; upsert+write `:129-130` = 2).

### 3.2 `cli/remove.rs::run` (объявление `cli/remove.rs:34-73`)

- **Что читает:** весь каталог — `Index::load_from` (`cli/remove.rs:40`).
- **Решение из прочитанного:** `removed = index.remove_version(...)` /
  `remove_package(...)` (`cli/remove.rs:46`, `:48`); ветка `if !removed` →
  ошибка «nothing to remove» ДО записи (`cli/remove.rs:50-61`); печать
  результата (`cli/remove.rs:63-71`).
- **Чем станет:** мутация = `append(Removed{group, name, version: Option})`;
  «nothing to remove» → «проверка по проекции» (или исчезает — повторный
  `Removed` безвреден); асимметрия с сервером (CLI — ошибка, HTTP — 200
  `removed: false`, `server/routes/packages.rs:329-335`) становится явным
  решением о ответе из проекции.
- **Что ломается:** связка `index` (`cli/remove.rs:40`) тянет за собой
  `cli/remove.rs:46-62` — не компилируется; тест `remove_unknown_errors`
  (`tests/cli_write.rs:231-239`) утверждает именно ошибку на отсутствии —
  сохранение/смена этого поведения надо подтвердить (сегодня он красный
  свидетель семантики «нет → ошибка»).
- **Классификация:** переписать.
- **Строк затронуто:** ~23 из 83 (`cli/remove.rs:40-62`).

### 3.3 `cli/reindex.rs::run_plan` (объявление `cli/reindex.rs:216-319`) — общий для `reindex` и `rescan-org`

- **Что читает:** весь каталог — `Index::load_from` (`cli/reindex.rs:224-231`);
  из прочитанного — `registry`/`registry_url`/`naming` в opts
  (`cli/reindex.rs:234-236`) и в свежий `Index::new` (`cli/reindex.rs:253-255`);
  `existing.naming.repo_name(...)` для отображения запись→репо
  (`cli/reindex.rs:265-267`); `existing.iter_versions()` как переносимое
  множество (`cli/reindex.rs:261`); при инкременте — `checkpoint::load`
  (`cli/reindex.rs:241-242`). Генератор — НЕ из прочитанного, компонуется
  заново (`cli/reindex.rs:237`).
- **Решения из прочитанного:** (а) `plan.mode == "incremental"` — дважды
  (`cli/reindex.rs:241`, `:260`); (б) `kept_unchanged` — самая нагруженная
  ветка: репо в снимке? есть ли у скана запись с той же (group, name)? —
  (`cli/reindex.rs:268-283`, разбор в §5); (в) `next.generator = …`
  (`cli/reindex.rs:258`).
- **Чем станет:** инкрементальный merge в основном ИСЧЕЗАЕТ (unchanged-записи
  уже лежат в журнале как прошлые `Published`; проектор их удержит);
  сканер остаётся источником `Published`; для `--full` нужно событие сброса
  (§5); идентичность — новый вход (§4); чекпойнт остаётся сканерным
  состоянием. `Summary::from_report` (`cli/reindex.rs:418-468`) читает
  готовый `next` — выживает, если проекция даёт тот же `Index`.
- **Что ломается:** `existing.*` в `cli/reindex.rs:233-239`, `:252-257`,
  `:261`, `:306` перестают компилироваться; блок `kept_unchanged`
  (`cli/reindex.rs:260-288`) удаляется; `next.upsert(...)` (`:285`, `:290`)
  заменяется на append. Тесты `tests/scanner_e2e.rs` — поведенческие
  (счётчики в JSON-сводке `:187-197`, наличие `by-name/*.json` `:211-219`,
  сохранение идентичности `:447-450`) — выживают при эквивалентной проекции;
  инкрементальный тест (`:362-410`) утверждает именно kept-поведение — после
  удаления merge он должен утверждать то же самое через журнал.
- **Классификация:** переписать.
- **Строк затронуто:** ~75 из 506 (`cli/reindex.rs:224-292`: load 224-231,
  opts 233-239, mode 241-245, fresh-build 252-258, merge 260-288, upsert+write
  289-292).

### 3.4 `server/routes/packages.rs::upsert` (объявление `server/routes/packages.rs:231-294`)

- **Что читает:** НЕ диск — in-memory `Index` за `RwLock` (`server/state.rs:30`),
  загруженный один раз (`cli/serve.rs:71`). Per-request: `state.index.read().await.registry`
  — scope-check (`server/routes/packages.rs:237`, повторно в тексте ошибки
  `:243`); под write-lock — `idx.get(...)` для флага `existed`
  (`server/routes/packages.rs:253-256`).
- **Решения из прочитанного:** (а) scope-check: `entry.registry != серверный registry`
  → 400 (`server/routes/packages.rs:237-245`); (б) `existed` → код ответа
  CREATED/OK (`server/routes/packages.rs:271`, `:278-282`); (в) `changed`
  (Ф2.3, `index/memory.rs:120-135`) → писать/публиковать/считать метрику или
  нет (`server/routes/packages.rs:263-277`).
- **Чем станет:** scope-check — registry из конфига сервера (сегодня его там
  нет: `serve` берёт registry только из каталога, `cli/serve.rs:71`); `created`
  — lookup по проекции; `changed` — до append или после проекции (открытый
  вопрос, §6); мутация = `append(Published)` → reproject → `write_to`.
- **Что ломается:** блок `server/routes/packages.rs:251-272` (RMW над
  in-memory копией) переписывается целиком; строки `:237`/`:243` теряют
  источник (`index.read().registry`). Тесты `tests/server_writes.rs`
  (`:117-129`, `:132-153`, `:208-219`) и `tests/seam_fakes.rs:137-149`
  поведенческие (коды/JSON-ответы) — выживают.
- **Классификация:** переписать.
- **Строк затронуто:** ~20 из 431 (scope `:237-245` = 9; write-блок `:251-277`
  ≈ 27, из них RMW-ядро `:252-268`).

### 3.5 `server/routes/packages.rs::delete_version` (объявление `server/routes/packages.rs:305-336`)

- **Что читает:** in-memory индекс под write-lock (`server/routes/packages.rs:316`).
- **Решение из прочитанного:** `r = idx.remove_version(...)` — присутствовала
  ли версия (`server/routes/packages.rs:317`, семантика — `index/memory.rs:141-153`);
  `if r` гейтит запись (`:318-322`), метрику и публикацию (`:325-328`);
  `removed` уходит в ответ (`:329-335`).
- **Чем станет:** `append(Removed{…, version: Some(v)})` + reproject;
  `removed` — проверка по проекции до append (для ответа) или отчёт о
  применённом событии.
- **Что ломается:** `server/routes/packages.rs:315-328` переписывается;
  тесты `delete_version_removes_existing` / `delete_missing_returns_removed_false`
  (`tests/server_writes.rs:222-240`, `:278-291`) утверждают ответ-семантику —
  выживают, но именно они фиксируют «нет → 200/false».
- **Классификация:** переписать.
- **Строк затронуто:** ~14 из 431 (`server/routes/packages.rs:315-328`).

### 3.6 `server/routes/packages.rs::delete_package` (объявление `server/routes/packages.rs:338-366`)

- **Что читает:** in-memory индекс под write-lock (`server/routes/packages.rs:346`).
- **Решение из прочитанного:** `r = idx.remove_package(...)` — существовал ли
  пакет (`server/routes/packages.rs:347`, семантика — `index/memory.rs:156-160`);
  `if r` гейтит запись/метрику/публикацию (`:348-358`); `removed` и
  `version: None` в ответе (`:359-365`).
- **Чем станет:** симметрично 3.5: `append(Removed{…, version: None})` +
  reproject; `removed` из проекции.
- **Что ломается:** `server/routes/packages.rs:345-358` переписывается; тест
  `delete_package_drops_all_versions` (`tests/server_writes.rs:254-275`)
  выживает как поведенческий.
- **Классификация:** переписать.
- **Строк затрануто:** ~14 из 431 (`server/routes/packages.rs:345-358`).

Сводно: переписать — все шесть; объём ~160 строк кода мутаций (14+23+75+20+14+14)
плюс seed-хелперы тестов (§7). Существующий `write_to` и `WriteCtx` выбрасывать
не нужно — целевая форма А.3 (`TZ-CHANGE-NATIVE-FORMATS-v0.1.md:918-928`)
вызывает его над результатом проекции, и сигнатура уже этому соответствует
(`index/memory.rs:197`).

## 4. Идентичность реестра как вход проектора

Пять значений — `registry`, `registry_url`, `naming`, `generator`,
`schema_version` — живут в `repomd.json` (`types/repomd.rs:20-36`) и входят в
мутацию только через `load_from` (`index/memory.rs:353-363`).

**Кто ещё читает эти пять значений (исчерпывающий свип `crates/**`).**
Внутри `crates/vibe-index/src`:

- писатель манифеста: `index/memory.rs:292-297` (поля уходят в `Repomd` при
  каждой записи; `schema_version` — из состояния, не из константы, Ф2.2,
  комментарий `index/memory.rs:287-290`);
- читатель манифеста: `index/memory.rs:354-358` (обратное направление);
- `cli/reindex.rs:234-236`, `:253-255`, `:258`, `:266`, `:306` — opts,
  fresh-index, repo_name-маппинг, summary (разобрано в §3.3);
- `cli/add.rs:81`, `:99` — compose `source_url` + штамп `entry.registry` (§3.1);
- `cli/init.rs:53`, `:56`, `:59-62` — рождение (см. ниже) и печать;
- сканер: `scanner/org_walk.rs:209` (`source_url_for(&opts.registry_url, opts.naming, …)`),
  `:212` (`registry: opts.registry.clone()` в каждую запись),
  `:235` (`indexed_by: opts.generator.clone()`) — то есть сканер
  ТОЖЕ требует идентичность как вход (`FromClonesOptions`,
  `scanner/org_walk.rs:28-37`);
- сервер: снапшот `generator` при рождении `AppState` (`server/state.rs:113`,
  поле `server/state.rs:29`); scope-check upsert
  (`server/routes/packages.rs:237`, `:243`); ответы list/admin/health
  (`server/routes/packages.rs:149`, `server/routes/admin.rs:30-32`,
  `server/routes/health.rs:18`, `:27`);
- читающие CLI: `cli/list.rs:89`, `:102`; `cli/dump.rs:57-62` (дамп всех пяти);
- `cli/verify.rs:26` + `:123-125` — прямой читатель `repomd.json` БЕЗ
  `load_from` (`repomd::read`, затем `manifest.registry`, `package_count`,
  `version_count` в отчёт).

Вне `crates/vibe-index`: потребительская сторона (`crates/vibe-registry`)
идентичность каталога НЕ читает — `index_client` только ПРОБИРУЕТ
`<base>/repomd.json` на HTTP 200 (`crates/vibe-registry/src/index_client/mod.rs:144-153`),
а `naming` в `git_package_registry`/`multi_registry_resolver` берётся из
КОНФИГА реестра пользователя (`[[registry]].naming`,
`crates/vibe-registry/src/git_package_registry/urls.rs:30`, `:81`), не из
каталога. `crates/vibe-cli` этих полей каталога не касается.

**Где рождаются и что, если их нет.** Единственное рождение в продакшне —
`cli/init.rs` из флагов `--registry` / `--registry-url` / `--naming`
(`cli/init.rs:22-34` → `Index::new` `:53` → первый `write_to` `:54`).
`generator` рождается константой сборки (`index/memory.rs:382-384`),
`schema_version` — константой `SCHEMA_VERSION = 1` (`index/memory.rs:29`) с
правилом Ф2.2 «прочитанное состояние важнее константы»
(`index/memory.rs:287-290`, тест `index/memory/tests.rs:266-291`). Если
`repomd.json` нет — `load_from` падает, и все три мутации + serve отвечают
«data-dir … does not look like an initialised index. Run `vibe-index init`
first» (`cli/add.rs:57-63`, `cli/reindex.rs:225-231`, `cli/serve.rs:72-78`).

**Хоть один путь, где значения приходят НЕ из `repomd.json`?** В продакшне —
да, ровно один: флаги `cli/init.rs:22-34` (рождение). Всё остальное —
производные прочитанного манифеста: серверный снапшот `generator`
(`server/state.rs:113`) сделан с загруженного индекса; `verify` читает сам
`repomd.json`, минуя `Index` (`cli/verify.rs:26`), но источник тот же файл.
В тестах значения routinely рождаются из `Index::new(...)` напрямую
(`tests/server_writes.rs:66-71`, `index/memory/tests.rs:28-35`) — это не
продакшн-путь.

**Варианты снабдить проектор идентичностью, доступные сегодня** (перечислены,
не выбраны; у каждого — цитата, что механизм существует):

1. **Аргумент функции.** Механизм существует и уже в форме «пакета
   идентичности»: `Index::new(registry, registry_url, naming, at)`
   (`index/memory.rs:94-98`) и `FromClonesOptions { registry, registry_url,
   naming, generator, indexed_at }` (`scanner/org_walk.rs:28-37`). Цена:
   точке вызова нужен ИСТОЧНИК — у CLI-мутаций таких флагов нет (Args
   `cli/add.rs:26-47` — только data_dir/manifest/repo_url/ref/commit), у
   сервера тоже (`cli/serve.rs:18-58`); придётся либо добавить флаги/конфиг,
   либо читать манифест (что и есть нынешний `load_from`).
2. **Отдельная запись-манифест (аналог «события инициализации»).** Механизм
   «идентичность живёт в отдельной записи, пишется последней» существует:
   `repomd.json` штампуется отдельно и последним
   (`index/memory.rs:291-302`); вспомогательное состояние с собственной
   `schema_version` тоже имеет прецедент — `state/checkpoint.json`
   (`index/checkpoint.rs:16-27`). Журнала как механизма в дереве НЕТ
   (`crates/vibe-index/src/journal/` не существует), и плановый enum `Event`
   (`TZ-CHANGE-NATIVE-FORMATS-v0.1.md:894-909`) варианта «инициализация
   реестра» не несёт — это новый код + новое звено схемы e1.
3. **Конфиг.** Механизм существует: clap-флаги как источник идентичности
   (`cli/init.rs:22-34`); конфиг-снапшот сервера, уже хранящий копию
   `generator` (`server/state.rs:29`, `:113`). Цена: второй источник истины
   рядом с манифестом; серверу понадобится флаг/файл, которого сегодня нет
   (registry приходит только из каталога, `cli/serve.rs:71`).

## 5. `reindex --full` и событие сброса

### 5.1 Полная поверхность флагов

`reindex` (`cli/reindex.rs:31-96`): `data_dir: PathBuf` (`:32`);
`--from-clones ORG-DIR: Option<PathBuf>` (`:36-37`); `--from-github ORG:
Option<String>` (`:40-41`); `--from-gitverse ORG: Option<String>` (`:45-46`,
стаб); `--token-file FILE: Option<PathBuf>` (`:49-50`); `--api-base URL:
String, default "https://api.github.com"` (`:54-55`); `--clone-cache DIR:
Option<PathBuf>` (`:60-61`); `--full: bool` (`:64-65`); `--incremental: bool`
(`:68-69`, `conflicts_with = "full"`); `--cache-org: bool` (`:81-82`);
`--no-cache-org: bool` (`:90-91`); `--json: bool` (`:94-95`). Группы:
`source` (обязательная — `:27`), `scope` (`full`/`incremental`, `:28`),
`cache_mode` (`:29`).

`rescan-org` (`cli/rescan_org.rs:29-54`): `data_dir: PathBuf` (`:30`);
`--from-github ORG: String` (обязательный, `:33-34`); `--token-file FILE:
Option<PathBuf>` (`:37-38`); `--api-base URL: String, default
"https://api.github.com"` (`:42-43`); `--clone-cache DIR: Option<PathBuf>`
(`:48-49`); `--json: bool` (`:52-53`).

### 5.2 Кто вызывает `run_plan` и с каким mode

Ровно два вызывателя: `reindex::run` — mode вычисляется как `"incremental"`
если `args.incremental`, иначе `"full"` (`cli/reindex.rs:141-145`, вызов
`:147-154`); `rescan_org::run` — ВСЕГДА `"full"` (`cli/rescan_org.rs:68-75`,
литерал на `:72`). Инкремент возможен только явным флагом; умолчание и
`rescan-org` — полный сброс.

### 5.3 `--full` по шагам (сегодня)

1. `at = Utc::now()` (`cli/reindex.rs:220`).
2. `existing = Index::load_from(...)` — отказ, если каталог не init
   (`cli/reindex.rs:224-231`).
3. `opts = FromClonesOptions { registry, registry_url, naming ← existing;
   generator ← версия бинаря; indexed_at ← at }` (`cli/reindex.rs:233-239`).
4. `prior = None` (mode ≠ "incremental") (`cli/reindex.rs:241-245`).
5. `report = scanner.scan(&opts, None)` — полный обход всех репо
   (`cli/reindex.rs:247`); сканер кладёт в отчёт каждую прошитую запись
   (`scanner/org_walk.rs:202-236`) и снимок каждого репо
   (`scanner/org_walk.rs:124-128`).
6. `next = Index::new(existing.registry, existing.registry_url,
   existing.naming, at)` — ПУСТОЙ носитель записей, надгробий и карантина
   (`cli/reindex.rs:252-257`; пустые поля — `index/memory.rs:107-109`);
   `next.generator = opts.generator` (`cli/reindex.rs:258`).
7. Инкрементальный блок пропускается целиком (`cli/reindex.rs:260-288` под
   `if plan.mode == "incremental"`).
8. В `next` попадают ТОЛЬКО `report.entries` (`cli/reindex.rs:289-291`).
9. `next.write_to(&plan.data_dir, &WriteCtx { at })` — снос `by-name/`,
   `by-cap/`, `by-purl/`, полная перезапись, манифест последним
   (`cli/reindex.rs:292`; механика — `index/memory.rs:210-302`).
10. Чекпойнт = снимок свежего скана, сохраняется независимо от режима
    (`cli/reindex.rs:296-301`).
11. Сводка и вывод (`cli/reindex.rs:303-317`).

### 5.4 `--incremental` по шагам, включая `kept_unchanged`

Шаги 1-3 те же; далее:

4и. `prior = Some(checkpoint::load(...))` (`cli/reindex.rs:241-242`;
    отсутствие файла → пустой чекпойнт — `index/checkpoint.rs:55-70`).
5и. `scanner.scan(&opts, prior)` — сканер скипает репо, чей HEAD и список
    тегов совпали с записанным снимком (`scanner/org_walk.rs:130-142`:
    `prev == &snapshot` → skip-заметка «unchanged since last checkpoint»);
    скипнутые репо НЕ попадают ни в `entries`, но попадают в `snapshots`
    (`scanner/org_walk.rs:128` — снимок вставляется ДО сравнения с prior).
6и-7и. Как в full: свежий пустой `next` + generator (`cli/reindex.rs:252-258`).
8и. Перенос неизменённого — построчно (`cli/reindex.rs:261-287`):
    - `:261` — для КАЖДОЙ записи из прочитанного каталога
      (`existing.iter_versions()`);
    - `:265-267` — `repo_name = existing.naming.repo_name(entry.kind,
      &entry.group, &entry.name)` — отображение запись→имя репо по
      именовательной конвенции;
    - `:268-282` — `scanned_now`: если `repo_name` ЕСТЬ в
      `report.snapshots` — взять `true`, если среди `report.entries`
      найдётся запись с той же парой `(group, name)` (свежепрошитая), иначе
      `false`; если репо в снимках НЕТ — `unwrap_or(false)` → `false`;
    - `:283` — `kept_unchanged = report.snapshots.contains_key(&repo_name)
      && !scanned_now` — «репо сканер видел, но не шил, ибо unchanged»;
    - `:284-286` — если `kept_unchanged` → `next.upsert(entry.clone())`.
    Граница случая: репо переименовано/перемещено так, что `repo_name`
    больше не совпадает ни с одним снимком → `kept_unchanged = false` →
    запись НЕ переносится и, если сканер её не прошивает, молча исчезает
    даже в инкременте.
9и-11и. Как full: `report.entries` поверх (`cli/reindex.rs:289-291`),
    `write_to` (`:292`), чекпойнт (`:296-301`), сводка (`:303-317`).

### 5.5 Наблюдаемые эффекты `--full`, невыразимые потоком `Published`

Проверенные кандидаты, поимённо:

- **Э1. Исчезновение записей, которых сканер не увидел.** ПОДТВЕРЖДЕНО как
  механика (§2 B6). Поток `Published` только добавляет/заменяет;
  «записи нет» выражается лишь отсутствием события, но проекция-fold
  удерживает запись прошлым `Published`. Воспроизвести исчезновение может
  только сброс или компенсирующие `Removed` на каждую живую запись.
- **Э2. Сброс надгробий.** ПОДТВЕРЖДЕНО (§2 B8): надгробия живут в
  `by-name/*.json` и in-memory носителе (`index/memory.rs:84-86`,
  `:242-247`), событий-носителя нет; плановый enum их не содержит
  (`TZ-CHANGE-NATIVE-FORMATS-v0.1.md:894-909`). ТЗ уже обещает «надгробие
  есть событие журнала» (`TZ-CHANGE-NATIVE-FORMATS-v0.1.md:542`) — но
  варианта события в А.2 на деру замера нет.
- **Э3. Сброс карантина.** ПОДТВЕРЖДЕНО, двумя слоями: (а) карантин —
  in-memory, никогда не сериализуется (`index/memory.rs:81-83`,
  `index/quarantine.rs:22-29`); `load_from` ОТКАЗЫВАЕТСЯ от версий с
  непонятым `must_understand` — их нет в `by_pkgref`
  (`index/memory.rs:328-348`); после ЛЮБОЙ записи by-name перестраивается
  из `by_pkgref` (`index/memory.rs:234-255`) — карантинные версии
  физически стираются из каталога первой же мутацией после load. Для
  `--full` это усугубляется Э1: даже сканируемое репо не вернёт запись,
  если она попала в карантин (сканер ставит `must_understand: Vec::new()`
  — `scanner/org_walk.rs:231` — так что обычный скан чист; бьёт по
  записям, добавленным иным путём, например сервером).
- **Э4. Сброс `latest_stable`.** САМ ПО СЕБЕ выражаем: `latest_stable`
  пересчитывается `finalise()` из набора версий (`index/memory.rs:131-134`,
  `:249`; `types/entry/aggregate.rs` — носитель). Но «сброс» как наблюдаемый
  эффект `--full` — СЛЕДСТВИЕ Э1 (исчезла старшая версия — сдвинулся
  latest_stable); отдельно не тянет решения.
- **Э5. Перезапись чекпойнта.** ПОДТВЕРЖДЕНО (§2 B7): снимок свежего скана
  затирает прошлый независимо от режима. Чекпойнт — не каталог, а память
  сканера (`index/checkpoint.rs:1-4`); потоком событий каталога не выражается
  в принципе. Вопрос — статус чекпойнта при журнале (см. ниже).
- **Э6. Смена `generator`.** ПОДТВЕРЖДЕНО (§2 B4): reindex гарантированно
  перезаписывает генератор версией текущего бинаря (`cli/reindex.rs:237`,
  `:258`). Поле это не событие и в плановый enum не входит.

Итого невыражаемых потоком `Published`: Э1, Э2, Э3, Э5, Э6 (Э4 — следствие Э1).

### 5.6 Что значило бы «событие сброса» — ОТКРЫТЫЕ вопросы (ответы — за боссом)

- **В1. Сброс — удаление истории или запись-водораздел?** Truncate журнала
  (физически удалить/архи­вировать события до сброса) против `Reset`-события,
  после которого replay стартует с пустого состояния. Что происходит с
  событиями ДО сброса при повторном replay — игнорируются водоразделом или
  файла больше нет?
- **В2. Шардинг.** Журнал месячно шардирован (`journal/2026-08.ndjson`,
  `TZ-CHANGE-NATIVE-FORMATS-v0.1.md:914-916`). Водораздел в новом шарде,
  ссылающийся на старые, или сброс обязан начинать новый шард? Что с
  `journal/checkpoint.json {last_file, last_offset}` при сбросе?
- **В3. Компенсация вместо водораздела.** Должен ли `--full` эмбировать
  `Removed` на каждую исчезающую запись (аудируемо, O(n) событий, журнал
  сохраняет историю) вместо стирающего сброса? Цена/выгода?
- **В4. Надгробия и карантин при сбросе.** Надгробие — событие (как обещает
  ТЗ) и тогда сброс их НЕ стирает, а обязан ли `--full` повторно эмбировать
  надгробия? Карантин — функция ЧИТАТЕЛЯ (`index/quarantine.rs:10-18`),
  не состояния каталога: проекция его не воспроизводит — где его место в
  целевой форме?
- **В5. Чекпойнт.** Остаётся ли `state/checkpoint.json` сканерным
  состоянием при журнале, или сброс обязан его тоже событийно отражать?
- **В6. Право на сброс.** Сегодня `"full"` — умолчание reindex И жёстко
  зашитый режим `rescan-org` (`cli/rescan_org.rs:72`), чей заявленный смысл
  — «обновить кэш образа ор­ганизации» (`cli/rescan_org.rs:1-15`). Должен ли
  verb «обновить кэш» реально сбрасывать каталог? Кто вообще вправе эмбировать
  сброс?
- **В7. Идемпотентность.** Два сброса подряд подряд / сброс при пустом
  журнале / сброс параллельно append — что наблюдаемо?

## 6. Идемпотентность и счётчики после Ф2.3

**Где проверяется «изменилось ли» на трёх серверных путях:**

- `upsert`: `existed` — присутствие версии до мутации (`idx.get(...).map(|p|
  p.versions.iter().any(|v| v.version == version))`,
  `server/routes/packages.rs:253-256`); `changed = idx.upsert(entry)` —
  сравнение по ЦЕЛОМУ значению, не по номеру версии (`server/routes/packages.rs:262`;
  семантика — `index/memory.rs:126-134`: `pkg.versions.contains(&entry)` →
  `false`, иначе replace + `true`); гейт `if changed` вокруг записи
  (`server/routes/packages.rs:263-270`), метрики и публикации
  (`server/routes/packages.rs:274-277`).
- `delete_version`: `r = idx.remove_version(...)` (`server/routes/packages.rs:317`;
  семантика — `index/memory.rs:146-152`, сравнение длин до/после retain);
  гейт `if r` (`server/routes/packages.rs:318-323`) и `if removed`
  (`:325-328`).
- `delete_package`: `r = idx.remove_package(...)` (`server/routes/packages.rs:347`;
  семантика — `index/memory.rs:156-160`, `is_some`); гейты
  `server/routes/packages.rs:348-353`, `:355-358`.

**Счётчики метрик, двинувшиеся под условие** (`server/metrics.rs`):
`vibe_index_mutations_total` (`server/metrics.rs:48-54`; инкремент
`note_mutation` — `server/state.rs:57-59` — вызывается ТОЛЬКО под гейтом:
`server/routes/packages.rs:275`, `:326`, `:356`) и транзитивно
`vibe_index_publish_failures_total` (`server/metrics.rs:55-61`): публикация
и её неуспех возможны только для сосчитанной мутации
(`server/routes/packages.rs:274-277`, `:402-431`). НЕ гейтится
`vibe_index_requests_total` (`server/metrics.rs:41-47`, `note_request`
безусловно — `server/routes/packages.rs:96`, `:163` и т. д.). Побочный
эффект: help-текст `mutations_total` «Total mutating HTTP requests served»
(`server/metrics.rs:51`) больше не буквальна — no-op upsert (200,
`created: false`) есть мутирующий запрос, но не считается.

**Клиентская сторона для сравнения:** CLI `add` гейта НЕ имеет — `bool`
от `upsert` отброшен (`cli/add.rs:129`), запись безусловна (`:130`); впрочем,
и no-op там не существует: каждая команда пересобирает entry с новым `at`
(`cli/add.rs:53`, `:121`), так что `indexed_at` гарантированно меняет запись.
`remove` в CLI гейтится ошибкой (`cli/remove.rs:50-61`). `reindex` пишет
безусловно (`cli/reindex.rs:292`) — и тоже не бывает no-op по той же причине
(`indexed_at: at` в opts — `cli/reindex.rs:238`).

**«Изменилось ли» при журнале — ДО append или ПОСЛЕ проекции?** Сегодняшний
код отвечает: ДО. Решение принимает сравнение с ТЕКУЩИМ состоянием до/вне
записи: `upsert` возвращает `bool` из сравнения с хранимым
(`index/memory.rs:128`), сервер分支ится на нём ДО `write_to` и до публикации
(`server/routes/packages.rs:262-277`). Дерево допускает два варианта:

1. **Проверка до append** — вычислить `changed` сравнением входа с текущей
   проекцией (сегодняшний механизм `upsert → bool` переносится на
   «сравнить с project(replay()) до append»); инструмент уже есть:
   `Index::get` + пословное сравнение (`index/memory.rs:163-165`) или
   `upsert`-равенство (`index/memory.rs:126-134`).
2. **Проверка после проекции** — append безусловно, затем сравнить
   «проекцию до» с «проекцией после» (или байты двух `write_to`) и лишь
   потом решать о публикации/метрике; инструменты сравнения в дереве есть:
   байт-в-байт сравнение деревьев (`index/memory/tests.rs:331-377`) и
   git-детектор пустого диффа на публикации — `PublishOutcome::NothingToCommit`
   (`server/routes/packages.rs:414-417`, `src/publish.rs`) — который уже
   сегодня различает «изменилось/нет» ПОСЛЕ факта.

Выбор не сделан ни здесь, ни в ТЗ — за боссом.

## 7. Тесты

| файл теста | всего строк | всего тестов | утверждают RMW-поведение | как сеют состояние | классификация |
|---|---|---|---|---|---|
| `tests/server_writes.rs` | 291 | 11 | 6: `post_packages_inserts_entry` (`:117`), `post_packages_upsert_returns_200_for_existing_version` (`:132`), `post_with_mismatched_registry_is_400` (`:208`), `delete_version_removes_existing` (`:222`), `delete_package_drops_all_versions` (`:254`), `delete_missing_returns_removed_false` (`:278`); остальные 5 — auth-гейты | helper `fresh_state` (`:64-85`): `Index::new` → `write_to` (`:72`) → `load_from` (`:82`) → `AppState` | переписать helper (seed каталога → журнал/seed-события); тела выживают (поведение сохраняется) |
| `tests/auto_publish.rs` | 370 | 5 | 5 (все): `upsert_publishes_with_named_commit_and_pushes_to_remote` (`:204`), `delete_routes_publish_remove_messages` (`:232`), `identical_repeat_upsert_publishes_exactly_one_commit` (`:280`, Ф2.3), `push_failure_keeps_request_alive_and_counts` (`:315`), `flag_off_runs_no_git` (`:350`) | helpers `setup` (`:107-154`, `write_to` на `:119`, git init/commit) + `build` (`:156-162`, `load_from` на `:157`) | переписать helpers; тела (git-публикация) выживают |
| `tests/cli_write.rs` | 239 | 6 | 6 (все): `add_inserts_entry_from_manifest` (`:52`), `add_upserts_when_version_already_present` (`:82`), `add_with_repo_url_overrides_default` (`:120`), `remove_deletes_specific_version` (`:155`), `remove_drops_entire_package_without_version_flag` (`:204`), `remove_unknown_errors` (`:231`) | через БИНАРНЫЕ команды: `init_at` (`:13-25`) + `add`/`remove`; `write_to`/`load_from` не трогают | оставить тела; `remove_unknown_errors` — красный свидетель семантики «нет → ошибка» (решение §3.2) |
| `tests/cli_lifecycle.rs` | 212 | 11 | 0 (init/dump/verify; мутаций add/remove/upsert/delete нет) | бинарный `init` (`:193-205`) | почти нет; init → seed журнала минимально |
| `tests/server_e2e.rs` | 449 | 20 | 0 (все маршруты читающие) | helper `populated_state` (`:77-134`): upsert-ы в in-memory idx + `write_to` (`:131`); `AppState::new` получает САМ idx (без `load_from`) | подвинуть seed (upsert-набор → события/проекция); тела читающие, выживают |
| `tests/seam_fakes.rs` | 190 | 3 | 1 касается RMW: `write_is_authorised_by_the_injected_token_store` (`:137`, POST → 201); остальные 2 — отказы | helper `state_with_seams` (`:82-98`): `write_to` (`:93`) + `load_from` (`:95`) | переписать helper; тела выживают |
| `src/index/memory/tests.rs` | 394 | 13 | 12 механики RMW: upsert/remove (`:81`, `:89`), round-trip (`:99`), by-name (`:118`), repomd-хэши (`:140`), stale-файлы (`:158`), карантин (`:175`), надгробия ×2 (`:205`, `:233`), Ф2.2 (`:266`), Ф2.3 (`:300`), Ф2.1 байт-детерминизм (`:331`) | helpers `fresh_index`/`write_ctx`/`entry` (`:28-78`) + `write_to`/`load_from` по тестам | выживают (механика `write_to` сохраняется); tombstone-тесты остаются красными дублерами дефера B8 |
| `src/cli/add.rs` (встроенный `#[cfg(test)]`, `:156-204`) | — | 1 | 1: `add_projects_manifest_frozen_into_the_entry` (`:164`) | `Index::new().write_to` (`:168-175`), проверка `load_from` (`:194`) | seed-форма меняется, тело выживает |
| доп.: `tests/scanner_e2e.rs` | 539 | 7 | 7 (все — reindex-поведение): seam-скан (`:73`), полный проход (`:121`), skip не-v-тегов (`:228`), текстовый вывод (`:276`), инкремент (`:318`), идентичность из init (`:414`), современный манифест (`:454`) | бинарные init + reindex; git-репо программно (`:30-58`) | переписать при смене семантики `--full` (§5); инкремент-тест (`:318-411`) — единственный, кто утверждает kept-поведение |
| доп.: `tests/cli_read.rs` | 405 | — (читающие) | 0; НО сеёт индекс через бинарный `reindex --full` (`:117-126`) | binary init + reindex --full | seed выживает, пока `--full` даёт тот же каталог |
| доп.: `tests/rate_limit_e2e.rs` | 305 | — | 0 (лимиты) | inline: `write_to` (`:36`) + `load_from` (`:46`) | seed-форма меняется |

**Helpers, сеющие состояние через `write_to` + `load_from` (поимённо, это то,
что переделка тронет наверняка):**

1. `tests/server_writes.rs::fresh_state` — `write_to` `:72`, `load_from` `:82`.
2. `tests/auto_publish.rs::setup` — `write_to` `:119`; `::build` — `load_from` `:157`.
3. `tests/seam_fakes.rs::state_with_seams` — `write_to` `:93`, `load_from` `:95`.
4. `tests/rate_limit_e2e.rs` (inline в тестах) — `write_to` `:36`, `load_from` `:46`.
5. `src/index/memory/tests.rs` — round-trip-тесты через `write_to`/`load_from`
   (`:106/:108`, `:126/:133`, `:144`, `:162/:166`, `:182/:184`, `:216/:219/:224/:225`,
   `:243/:249/:253/:254`, `:270/:281/:285`, `:352/:353`).
6. `src/cli/add.rs::tests` — `write_to` `:174`, `load_from` `:194`.
7. `tests/server_e2e.rs::populated_state` — только `write_to` `:131` (AppState
   получает in-memory idx, `load_from` нет) — половина шаблона, тоже сеет
   через писателя.

**Есть ли тест, утверждающий, что `--full` УДАЛЯЕТ запись, исчезнувшую из
скана? — НЕТ.** `tests/scanner_e2e.rs` утверждает только положительное
присутствие (`:187-219` — счётчики и наличие by-name-файлов) и, для
инкремента, СОХРАНЕНИЕ (`:376-377`, `:405-406`); `tests/cli_read.rs` сеет
через `--full`, но ничего не удаляет. Сторожа на исчезновение нет — поведение
B6 сегодня ничем не зафиксировано и может измениться без красного теста.

## 8. Гейт G4 и форма шагов панели

**Шаги `tools/self-check.sh` по порядку (678 строк):** 0b «floor denominator»
(`tools/self-check.sh:159-197`); 0c «instruction files identical»
(`:199-220`); 0 (снапшот tripwire, `:274-286`); 1 `cargo fmt --all --check`
(`:293-295`); 2 `cargo test --workspace` (`:297-298`); 2b tripwire после
тестов (`:300-303`); 3 clippy `-D warnings` (`:305-307`); 4 `vibe check`
(`:309-317`); 5 `cargo xtask conform check` (`:319-325`); 6
`sync-engines --check` (`:327-332`); 6b `check-codegen` (`:334-342`); 6c
`specmap --check` (`:344-352`); 7 core-ai-native: guard authored-slot
(`:354-379`) + fmt/test/clippy (`:380-385`); 8 language stacks fmt/test/clippy
(`:387-410`); 9 specmap self-traces ×3 (`:412-430`); 10 mcp-пакеты
fmt/test/clippy/self-trace (`:432-458`); 10b conform per slot (`:460-486`);
10c mcp authored denominator (`:488-532`); **10d index clock gate (Ф2.1)**
(`:534-570`); 11b lane-citation lint (`:578-601`); 11c licence keys
(`:603-639`); markup validation `progress check --exhaustive` (`:641-662`);
12 tripwire whole run (`:664-668`). Форма шага: bash-функция `check_*` +
`run_step "заголовок" check_* || OVERALL=$?` (`:91-110`) — новый шаг панели
ложится в этот же каркас, как 10d.

**Гейт часов Ф2.1 целиком (ближайший образец, `tools/self-check.sh:549-570`):**

```bash
check_index_clock_gate() {
  local hits
  hits=$(grep -rnE 'Utc::now\(|SystemTime::now\(' \
      crates/vibe-index/src/index \
      crates/vibe-index/src/types \
      2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*//')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'self-check: the index writer modules call the clock directly.\n' >&2
    printf 'self-check: the rule — time enters at the edge (CLI command or\n' >&2
    printf 'self-check: server mutation event) and never inside index/ or types/:\n' >&2
    printf 'self-check: one state must produce one byte sequence, or "rebuild and\n' >&2
    printf 'self-check: compare" measures nothing (PROP-044 §4.3, F2-1).\n' >&2
    printf 'self-check: fix: pass the time as an argument — a WriteCtx for\n' >&2
    printf 'self-check: write_to, an `at` for Index::new / VersionEntry::minimal.\n' >&2
    return 1
  fi
  return 0
}
run_step "index clock gate (no Utc::now/SystemTime::now in index/ or types/)" \
  check_index_clock_gate || OVERALL=$?
```

Его форма: периметр поимённо каталогами (`crates/vibe-index/src/index`,
`crates/vibe-index/src/types`, рекурсивно — комментарий `:539-543`),
комментарий-фильтр по форме строки (`:544-548`), развёрнутый рецепт в
сообщении об ошибке. G4-шаг ляжет так же: периметр = файлы писателей,
паттерн = `load_from`/типы чтения.

**Какие сигнатуры нарушили бы G4 сегодня.** Сначала факт: типы
`vibe-registry::index_client` в `crates/vibe-index` не встречаются вообще
(свип — §11; `index_client` живёт только в `crates/vibe-registry/src/`).
Значит, практический объект гейта — «прочитанное состояние каталога в
писателе». Нарушители при включении прямо сейчас:

1. `cli/add.rs::run` — `load_from` (`:56`) питает решатель `source_url`/
   `registry` и писателя (`:81`, `:99`, `:129-130`).
2. `cli/remove.rs::run` — `load_from` (`:40`) питает `remove_*` + `write_to`
   (`:46-62`).
3. `cli/reindex.rs::run_plan` — `load_from` (`:224`) питывает opts, fresh-index
   и merge (`:233-291`).
4. Серверные три мутации — оперируют `AppState`, в чьём состоянии лежит
   загруженный каталог `index: RwLock<Index>` (`server/state.rs:30`;
   RMW-использование — `server/routes/packages.rs:252-268`, `:316-323`,
   `:346-353`): тип чтения каталога встроен в состояние писателя.
5. `Index::write_to(&self, …)` (`index/memory.rs:197`) — метод над `Index`,
   который одновременно и тип чтения; в целевой форме это ЗАКОННО (аргумент =
   результат проекции), но механический гейт по типу различить «прочитанный»
   и «спроецированный» `Index` не может — отсюда форма гейта должна быть
   «запрет `load_from` в файлах/модулях писателей», как у 10d (периметр
   каталогами + паттерн), а не запрет типа.

## 9. Расхождения с находкой f0-rmw-volume

f0 (`campaigns/packages-2026-09/harvest/f0-rmw-volume.md`) датирован
2026-08-13 и мерил дерево ДО Ф1.3/Ф1.4/Ф1.5/Ф2.1/Ф2.2/Ф2.3. Устаревшие
координаты — поимённо (слева — f0, справа — сегодня):

| координата у f0 | сегодня | причина сдвига |
|---|---|---|
| `add.rs:51` load; `:122` write; `:76` registry_url/naming; `:94` registry; `:121` upsert | `add.rs:56`; `add.rs:130`; `add.rs:81`; `add.rs:99`; `add.rs:129` | Ф2.1 (`at` + `WriteCtx`) и обработка ошибки init |
| add.rs = 146 строк | 204 (`wc -l`) | появился встроенный `#[cfg(test)]` (`add.rs:156-204`) |
| `remove.rs:35` load; `:57` write; `:45-56` removed-блок | `remove.rs:40`; `remove.rs:62`; `remove.rs:50-61` | Ф2.1 |
| remove.rs = 78 | 83 | Ф2.1 |
| `reindex.rs:218` load; `:281` write; merge `235-277`; checkpoint-load `:236`; `Index::new` `:246` | `reindex.rs:224`; `:292`; `241-288`; `:241-242`; `:252-257` | Ф2.1 + правки сканера |
| reindex.rs = 495 | 506 | эволюция |
| packages.rs: write `:256`/`:304`/`:333`; scope `:235`/`:241`; created `:251-254` | `:268`/`:320`/`:350`; `:237`/`:243`; `:253-256` | Ф2.1 + Ф2.3 (гейт `changed` `:262-277` — у f0 отсутствовал как класс) |
| packages.rs = 414 | 431 | Ф2.3 |
| `init.rs:47-48` | `init.rs:53-54` | Ф2.1 |
| init.rs = 120 | 126 | — |
| `serve.rs:77` load | `serve.rs:71` | — |
| serve.rs = 149 | 143 | — |
| `memory.rs` write_to `161-256`, load_from `262-281`, `generated_at` stamp `:249` | `memory.rs:197-303`, `:315-364`, `:296` (`ctx.at`) | Ф2.1/Ф2.2 |
| memory.rs = 470 строк, `#[cfg(test)]` внутри (`303-470`) | **расколот**: `memory.rs` = 388 + `index/memory/tests.rs` = 394, подключение `#[cfg(test)] #[path]` — `memory.rs:386-388` | Ф1.4-era file-length budget |
| «5 clock-сайтов: memory.rs:86, :249, types/entry/mod.rs:167, add.rs:113, reindex.rs:232» | часы живут ТОЛЬКО на краях: `add.rs:53`, `remove.rs:37`, `reindex.rs:220`, `init.rs:46`, серверно — `packages.rs:267`, `:319`, `:349`; в `index/`/`types/` часов нет вообще (гейт 10d красен бы был) | Ф2.1 посажена |
| `upsert` — «добавляет версию» | возвращает `bool changed` (`memory.rs:120-135`) | Ф2.3 |
| тесты: server_writes 286/11; auto_publish 328/**4**; cli_write 239/6; cli_lifecycle 212/11 | 291/11; 370/**5** (новый `identical_repeat_upsert_publishes_exactly_one_commit`, `tests/auto_publish.rs:280-309`); 239/6; 212/11 | Ф2.3 добавила тест |
| f0 §2 не знала о `tombstones`/`quarantined` полях Index | есть (`memory.rs:83`, `:86`) — и оба стираются reindex (§5 Э2/Э3) | Ф1.4/Ф1.5 |

Качественные выводы f0, ПОДТВЕРДИВШИЕСЯ: семь продакшн-сайтов `write_to`;
`rescan-org` — второй вызыватель `run_plan`, не седьмой RMW-путь
(`cli/rescan_org.rs:68-75`); инкрементальный merge — единственная
design-сложность; писатель переиспользуется. Качественное дополнение к f0:
f0 считала, что при журнале merge просто «усыхает», и назвала сброс
«~20 строк, аналога нет» — замер показывает, что сброс тянет за собой не
только записи (Э1), но и надгробия (Э2), карантин (Э3), чекпойнт (Э5) и
генератор (Э6), т. е. решение шире, чем «truncate».

## 10. Дыры и неожиданности

1. **`reindex`/`rescan-org` не проверяют серверный лок.** `add` и `remove`
   отказываются работать при живом сервере (`cli/add.rs:54`,
   `cli/remove.rs:38`, `refuse_if_server_running` — `cli/add.rs:146-154`),
   а `run_plan` — нет (импорт `ServerLock` в `cli/reindex.rs:8-22`
   отсутствует): reindex может снести каталог под работающим сервером,
   чья in-memory копия этого не узнает до перезапуска.
2. **Инкрементальный reindex стирает надгробия так же, как полный** (fresh
   `Index::new`, перенос только версий — `cli/reindex.rs:252-291`): дефер
   Ф1.4 (ТЗ `:538-543`) назван уже, чем дефект.
3. **Первая же запись после `load_from` физически стирает карантинные
   версии из `by-name/`** (`index/memory.rs:328-348` отказ + `:234-255`
   перестройка): тихая потеря записей с непонятым `must_understand` при
   ЛЮБОЙ мутации, не только reindex.
4. **CLI `add` не бывает no-op**: каждая команда пересобирает entry с новым
   `at` (`cli/add.rs:53`, `:121`), `indexed_at` меняет запись, `upsert`
   вернёт `true`; Ф2.3-идемпотентность существует только на сервере.
5. **`rescan-org` — скрытый полный сброс**: заявлен как «обновить кэш образа
   организации» (`cli/rescan_org.rs:1-15`), а идёт с `mode: "full"`
   (`cli/rescan_org.rs:72`) со всеми эффектами Э1-Э6.
6. **Help-текст `vibe_index_mutations_total` неточен после Ф2.3**
   («Total mutating HTTP requests served», `server/metrics.rs:51`): no-op
   upsert — мутирующий запрос, но не считается (`server/routes/packages.rs:274-277`).
7. **Сторожа на «`--full` удаляет исчезнувшее из скана» нет** (§7) —
   поведение B6 ничем не зафиксировано.
8. **`generator` каталога меняется каждым reindex** на версию текущего
   бинаря (`cli/reindex.rs:237`, `:258`) — «кто строил каталог» не стабильно.
9. **`kept_unchanged` хрупок к переименованиям**: если `repo_name` записи
   (через `naming`) не совпал ни с одним снимком, запись не переносится и
   молча исчезает даже в инкременте (`cli/reindex.rs:268-283`,
   `unwrap_or(false)`).
10. **Мёртвый код в `cli/reindex.rs`**: `_silence_unused`/`_silence_naming`
    (`cli/reindex.rs:497-506`) — уйдут вместе с переделкой.
11. **Журнала в дереве нет вовсе** (`crates/vibe-index/src/journal/`
    отсутствует) — подтверждение f0: вся целевая форма зелёная поляна.
12. **Тесты `cli_lifecycle.rs`/`cli_read.rs` гоняют полный `reindex --full`
    как seed** (`tests/cli_read.rs:117-126`): любая смена семантики сброса
    бьёт по ним первыми.

## 11. Как воспроизвести этот замер

Команды — по одной на глагол, из корня worktree (Git Bash). Ни `cargo`, ни
`git` не запускались.

```sh
# семь продакшн-сайтов write_to + тестовые (B1, §3)
grep -rn "write_to" crates/vibe-index/src

# входы мутаций и читающие load_from (B2)
grep -rn "load_from" crates/vibe-index/src

# строковые сравнения режима (B5) — ровно два
grep -rn "mode ==" crates/vibe-index/src

# читатели пяти полей идентичности (§4)
grep -rn "\.registry\b\|\.registry_url\|\.naming\b\|\.generator\b\|\.schema_version\b" crates/vibe-index/src

# вызыватели run_plan (§5.2)
grep -rn "run_plan" crates/vibe-index/src/cli

# лок сервера — кто проверяет (§10.1)
grep -rn "refuse_if_server_running\|ServerLock" crates/vibe-index/src/cli

# длины файлов (таблицы §3, §7, §9)
wc -l crates/vibe-index/src/cli/add.rs crates/vibe-index/src/cli/remove.rs crates/vibe-index/src/cli/reindex.rs crates/vibe-index/src/cli/init.rs crates/vibe-index/src/cli/serve.rs crates/vibe-index/src/server/routes/packages.rs crates/vibe-index/src/index/memory.rs crates/vibe-index/src/index/memory/tests.rs crates/vibe-index/tests/server_writes.rs crates/vibe-index/tests/auto_publish.rs crates/vibe-index/tests/cli_write.rs crates/vibe-index/tests/cli_lifecycle.rs crates/vibe-index/tests/server_e2e.rs crates/vibe-index/tests/seam_fakes.rs crates/vibe-index/tests/scanner_e2e.rs tools/self-check.sh

# число тестов на файл (§7)
grep -c -E "#\[(tokio::)?test\]" crates/vibe-index/tests/server_writes.rs crates/vibe-index/tests/auto_publish.rs crates/vibe-index/tests/cli_write.rs crates/vibe-index/tests/cli_lifecycle.rs crates/vibe-index/tests/server_e2e.rs crates/vibe-index/tests/seam_fakes.rs crates/vibe-index/tests/scanner_e2e.rs crates/vibe-index/src/index/memory/tests.rs

# встроенные тест-модули в src (§7)
grep -rn "#\[cfg(test)\]" crates/vibe-index/src/cli crates/vibe-index/src/index

# отсутствие журнала (§10.11)
ls crates/vibe-index/src/journal
```

Даты-границы замера: координаты сверены с деревом worktree `F3-RMW`
по состоянию на 2026-08-14; `wc`/`grep` — выводы процитированы по месту
использования.
