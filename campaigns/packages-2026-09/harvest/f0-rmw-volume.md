# Ф0.1 — M-RMW: объём переделки мутаций (находка спайка)

**Что это.** Замер фазы Ф0 плана TZ-CHANGE-NATIVE-FORMATS-v0.1 (решение D3).
Спайк, не правка: дерево не тронуто. Дата замера: 2026-08-13.

Цель — вынести вердикт, укладывается ли переделка read-modify-write
(решение D3: журнал фактов + чистая перепроекция; писатель НИКОГДА не
принимает на вход то, что сам же опубликовал) в бюджет двух рабочих фаз
(Ф3 + хвост). Замер опирается только на чтение дерева и измерения `wc`/`grep`;
сборка не запускалась.

---

## 1. Сверка с базовой линией B3

Базовая линия B3 (2026-08-09) утверждала 6 читающих RMW-путей + 1 пишущий +
стартовая загрузка. Сверка с деревом на 2026-08-13 (источник координат —
`grep -rn "write_to" crates/vibe-index/src` и `grep -rn "load_from" crates/vibe-index/src`):

| № | путь по B3 | координата по B3 | что найдено на самом деле | статус |
|---|---|---|---|---|
| 1 | `cli/add.rs` | 51 → 122 | `Index::load_from` в `add.rs:51`; `index.write_to` в `add.rs:122` | совпало |
| 2 | `cli/remove.rs` | 35 → 57 | `Index::load_from` в `remove.rs:35`; `index.write_to` в `remove.rs:57` | совпало |
| 3 | `cli/reindex.rs` | 218 → 281 (инкрементальный) | `Index::load_from` в `reindex.rs:218`; `next.write_to` в `reindex.rs:281`; инкрементальныйmerge — `reindex.rs:235-277` | совпало |
| 4 | `server/routes/packages.rs` | 256 | `idx.write_to(&state.data_dir)` в `packages.rs:256` (upsert) | совпало |
| 5 | `server/routes/packages.rs` | 304 | `idx.write_to(&state.data_dir)` в `packages.rs:304` (delete_version) | совпало |
| 6 | `server/routes/packages.rs` | 333 | `idx.write_to(&state.data_dir)` в `packages.rs:333` (delete_package) | совпало |
| 7 | `cli/init.rs` (только пишет) | 48 | `index.write_to(&args.data_dir)` в `init.rs:48` | совпало |
| — | `cli/serve.rs` (стартовая загрузка) | 77 | `Index::load_from(&args.data_dir)` в `serve.rs:77` | совпало |

Подтверждающий вывод `grep -rn "write_to" crates/vibe-index/src` (продакшн-сайты,
тестовые `#[cfg(test)]` внутри memory.rs исключены):

```
crates/vibe-index/src/cli/add.rs:122:    index.write_to(&args.data_dir)?;
crates/vibe-index/src/cli/init.rs:48:    index.write_to(&args.data_dir)?;
crates/vibe-index/src/cli/reindex.rs:281:    next.write_to(&plan.data_dir)?;
crates/vibe-index/src/cli/remove.rs:57:    index.write_to(&args.data_dir)?;
crates/vibe-index/src/server/routes/packages.rs:256:        idx.write_to(&state.data_dir)
crates/vibe-index/src/server/routes/packages.rs:304:            idx.write_to(&state.data_dir)
crates/vibe-index/src/server/routes/packages.rs:333:            idx.write_to(&state.data_dir)
```

= ровно 7 продакшн-вызовов `write_to` (6 RMW + 1 пишущий init). Восьмого нет,
пропущенного нет. Все координаты B3 совпали в точности — **расхождений по
координатам нет** (нюанс по характеру «чтения» в серверных путях — в §7).

`load_from` как ВХОД мутации — только три CLI-пути (`add.rs:51`, `remove.rs:35`,
`reindex.rs:218`) плюс стартовая загрузка `serve.rs:77`. Остальные 7 `load_from`
(`capabilities.rs:42`, `dump.rs:33`, `get.rs:44`, `list.rs:58`, `outdated.rs:57`,
`purls.rs:44`, `search.rs:50`) — это ЧИТАЮЩИЕ команды, не мутации; по Ф3.2 они
`load_from` сохраняют (serve-старт и чтение).

---

## 2. Анатомия каждого мутирующего пути

Общая механика (источник — `crates/vibe-index/src/index/memory.rs`):
`Index::write_to(&self, data_dir)` (`memory.rs:161-256`) — это writer
полной проекции: он сносит `by-name/`, `by-cap/`, `by-purl/`, пишет
`primary.jsonl[.gz]`, все `by-name/<name>.json`, оба инвертированных индекса и
штампует `repomd.json` последним (`generated_at: Utc::now()`, `memory.rs:249`).
То есть КАЖДАЯ мутация переписывает ВЕСЬ каталог из всего in-memory состояния —
это и есть «писатель принимает на вход то, что опубликовал». `load_from`
(`memory.rs:262-281`) — обратная сторона: читает `repomd.json` + все
`by-name/*.json` и собирает `by_pkgref`. Именно эту пару убивает D3.

### cli/add.rs::run (строки 48-124)
- **читает:** `Index::load_from(&args.data_dir)` (`add.rs:51`) — весь каталог с
  диска. Затем из прочитанного берутся поля реестра: `index.registry_url`,
  `index.naming` (`add.rs:76`), `index.registry` (`add.rs:94`).
- **меняет:** `index.upsert(entry)` (`add.rs:121`) — мутирует `by_pkgref`
  in-memory.
- **пишет:** `index.write_to(&args.data_dir)` (`add.rs:122`) — переписывает весь
  каталог.
- **что мешает journal+projection:** (1) load_from как вход мутации; (2) логика
  `compose_default_repo_url(&index.registry_url, index.naming, …)` опирается на
  прочитанное состояние для построения `source_url`; (3) `entry.registry` берётся
  из `index.registry`. Но все три поля (`registry`, `registry_url`, `naming`) —
  это ИДЕНТИЧНОСТЬ реестра, фиксируется на `init` и хранится в манифесте/журнале,
  а не в потоке записей. Преобразование: убрать load_from, взять идентичность
  реестра из журнала/манифеста, мутация = `validate(manifest)` → `journal::append(Published{entry})`.
- **классификация:** переписать
- **строк затронуто:** ~15 (диапазон `add.rs:51-58` блок load = 8 строк;
  `add.rs:75-77` compose source_url = 3; `add.rs:94` registry = 1; `add.rs:121-122`
  upsert+write_to = 2; плюс `add.rs:113` `Utc::now()` по Ф2). Всего строк файла —
  146 (`wc -l`).

### cli/remove.rs::run (строки 32-68)
- **читает:** `Index::load_from(&args.data_dir)` (`remove.rs:35`) — весь каталог.
- **меняет:** `index.remove_version(...)` / `index.remove_package(...)`
  (`remove.rs:41-43`).
- **пишет:** `index.write_to(&args.data_dir)` (`remove.rs:57`).
- **что мешает journal+projection:** load_from как вход. Решение «nothing to
  remove» (`if !removed`, `remove.rs:45-56`) зависит от прочитанного состояния —
  это `bool`, который при журнале либо убирается (журнал идемпотентен: повторный
  `Removed` безвреден), либо проверяется через проекцию. Мутация =
  `journal::append(Removed{group,name,version: Option})`.
- **классификация:** переписать
- **строк затронуто:** ~23 (диапазон `remove.rs:35-57`: load = 1, блок
  match+проверка `removed` `36-56` = 21, write = 1). Всего строк файла — 78 (`wc -l`).

### cli/reindex.rs::run_plan (строки 215-308)
- **читает:** `Index::load_from(&plan.data_dir)` (`reindex.rs:218`) — весь каталог
  (чтобы сохранить `registry`/`registry_url`/`naming`). Для инкремента — ещё
  `checkpoint::load` (`reindex.rs:236`).
- **меняет:** строит СВЕЖИЙ `Index::new(...)` (`reindex.rs:246`) и переносит в него
 _retained_-записи + отсканированные через `next.upsert(...)` (`reindex.rs:274`,
  `reindex.rs:279`).
- **пишет:** `next.write_to(&plan.data_dir)` (`reindex.rs:281`).
- **что мешает journal+projection:** это САМЫЙ нагруженный путь. Инкрементальный
  merge (`reindex.rs:249-277`, ~29 строк) итерирует `existing.iter_versions()`,
  отображает каждую запись в имя репо через `naming`, решает «сканер пропустил =
  unchanged» (`kept_unchanged`, `reindex.rs:272`) и несёт такие записи вперёд.
  Эта логика СУЩЕСТВЕННО зависит от прочитанного состояния. НО при журнале она в
  основном ИСЧЕЗАЕТ: unchanged-записи уже лежат в журнале как прошлые `Published`,
  проектор их и так удержит; reindex дозаписывает только `Published` для свежих
  сканов. Единственное, что нужно добавить — событие сброса для режима `--full`
  (truncate/reset журнала). Сканер остаётся как источник `published`-событий (Ф3.2).
- **классификация:** переписать
- **строк затронуто:** ~50 (диапазон `reindex.rs:218-225` блок load = 8;
  `reindex.rs:246-281` fresh-build + инкрементальный merge + write = ~36;
  плюс `reindex.rs:232` `Utc::now()` по Ф2). Это самый большой по объёму и
  единственный с реальной design-сложностью блок. Всего строк файла — 495 (`wc -l`).

### server/routes/packages.rs::upsert (строки 229-279)
- **читает:** НЕ `load_from` с диска — сервер держит `Index` в памяти за
  `RwLock` (`AppState.index`), загруженный ОДИН РАЗ на старте (`serve.rs:77`).
  Чтение per-request: `state.index.read().await.registry` (`packages.rs:235`,
  `packages.rs:241` — scope-check) и `idx.get(...)` (`packages.rs:251-254` — флаг
  `created`).
- **меняет:** `idx.upsert(entry)` (`packages.rs:255`).
- **пишет:** `idx.write_to(&state.data_dir)` (`packages.rs:256`).
- **что мешает journal+projection:** read-modify-write идёт по IN-MEMORY проекции
  (а не по диску), но суть та же: мутация берёт опубликованное состояние, меняет,
  переписывает ВЕСЬ каталог. Преобразование: scope-check берёт `registry` из
  конфига сервера (а не из `index.registry`); мутация =
  `validate(entry)` → `journal::append(Published{entry})` → `project(replay())`
  → `write_to(dir, ctx)`. `created` — lookup по проекции.
- **классификация:** переписать
- **строк затронуто:** ~11 (диапазон `packages.rs:249-259` — блок write-lock +
  get→created + upsert + write_to; плюс re-source registry в `235`/`241`).

### server/routes/packages.rs::delete_version (строки 290-320)
- **читает:** in-memory `state.index` (RwLock), как upsert.
- **меняет:** `idx.remove_version(&group, &name, &v)` (`packages.rs:302`).
- **пишет:** `idx.write_to(&state.data_dir)` (`packages.rs:304`, только если `removed`).
- **что мешает journal+projection:** `removed` (`packages.rs:303`) — из прочитанного
  состояния. Мутация = `journal::append(Removed{…, version: Some(v)})` + reproject.
- **классификация:** переписать
- **строк затронуто:** ~9 (диапазон `packages.rs:300-308`).

### server/routes/packages.rs::delete_package (строки 322-349)
- **читает:** in-memory `state.index` (RwLock).
- **меняет:** `idx.remove_package(&group, &name)` (`packages.rs:331`).
- **пишет:** `idx.write_to(&state.data_dir)` (`packages.rs:333`, только если `removed`).
- **что мешает journal+projection:** симметрично delete_version. Мутация =
  `journal::append(Removed{…, version: None})` + reproject.
- **классификация:** переписать
- **строк затронуто:** ~9 (диапазон `packages.rs:329-337`).

### cli/init.rs::run (строки 40-59) — только пишет, не RMW
- **читает:** ничего (проверяет только `repomd::exists`, `init.rs:41`).
- **меняет:** `Index::new(...)` (`init.rs:47`) — пустой каталог.
- **пишет:** `index.write_to(&args.data_dir)` (`init.rs:48`).
- **что мешает journal+projection:** ничего по сути RMW (это seed пустого
  каталога). При журнале либо остаётся как запись пустой проекции, либо
  становится событием инициализации реестра. Минимальное влияние.
- **классификация:** подвинуть
- **строк затронуто:** ~3 (диапазон `init.rs:47-48`). Всего строк файла — 120 (`wc -l`).

### cli/serve.rs::run (строки 77-84) — стартовая загрузка, не мутация
- **читает:** `Index::load_from(&args.data_dir)` (`serve.rs:77`) — один раз, чтобы
  наполнить `AppState`.
- **меняет/пишет:** ничего.
- **что мешает journal+projection:** по Ф3.2 `load_from` ОСТАЁТСЯ в serve-старте
  (это обслуживание чтения, не вход мутации). Опционально можно на старте
  проектировать из журнала вместо загрузки готовой проекции — это «подвинуть», не
  обязательная часть переделки RMW.
- **классификация:** оставить
- **строк затрануто:** 0 (по D3 — без изменений; опционально ~8 при переходе на
  boot-проекцию).

---

## 3. Тесты

Размеры и количество тестов (`wc -l` и
`grep -c "#\[tokio::test\]\|#\[test\]"`):

| файл теста | всего строк | всего тестов | из них утверждают RMW-поведение (имена) | классификация |
|---|---|---|---|---|
| `tests/server_writes.rs` | 286 | 11 | 6: `post_packages_inserts_entry`, `post_packages_upsert_returns_200_for_existing_version`, `post_with_mismatched_registry_is_400`, `delete_version_removes_existing`, `delete_package_drops_all_versions`, `delete_missing_returns_removed_false` | переписать helper `fresh_state` (`60-80`, seed через `write_to`+`load_from`); тела тестов выживают (поведение сохраняется) |
| `tests/auto_publish.rs` | 328 | 4 | 4: `upsert_publishes_with_named_commit_and_pushes_to_remote`, `delete_routes_publish_remove_messages`, `push_failure_keeps_request_alive_and_counts`, `flag_off_runs_no_git` | переписать helpers `setup`/`build` (`103-157`, seed + `load_from`); тела (git-publish) выживают |
| `tests/cli_write.rs` | 239 | 6 | 6 (все): `add_inserts_entry_from_manifest`, `add_upserts_when_version_already_present`, `add_with_repo_url_overrides_default`, `remove_deletes_specific_version`, `remove_drops_entire_package_without_version_flag`, `remove_unknown_errors` | команды `add`/`remove` меняются внутри; утверждения на `by-name/*.json` выживают (проекция даёт те же файлы) |
| `tests/cli_lifecycle.rs` | 212 | 11 | 0 (init/dump/verify — init только-пишущий + чтение; мутаций add/remove/upsert/delete нет) | почти нет; `init` → seed-журнала (~минимально) |

Итого: 32 теста, из них **20 утверждают RMW-поведение** (6 + 4 + 6 + 0 по
мутирующей части; 4 в auto_publish — поведение мутация+публикация). Важный вывод:
подавляющее большинство — **поведенческие тесты на выходные файлы/статусы**, а не
на механизм `write_to`, поэтому ТЕЛА тестов в основном выживают при переходе на
журнал (проекция порождает те же `by-name/*.json` и те же HTTP-ответы). Цена — в
helper’ах, которые сеют состояние через `write_to`+`load_from`, плюс НОВЫЕ тесты
(append/replay, `rebuild --check` байт-в-байт, идемпотентный upsert Ф2.3).

Дополнительно: блок `#[cfg(test)] mod tests` внутри `memory.rs` (`memory.rs:303-470`,
5 вызовов `write_to` и 2 `load_from` — round-trip/`by-name`-тесты) — эти
юнит-тесты механики каталога затронуты сигнатурой `write_to(ctx)` (Ф2), но сама
механика переписывания сохраняется.

---

## 4. Итоговая таблица объёма

«строк затронуто» — оценка строк функции/блока, которые надо переписать или
удалить при переходе на journal+projection (диапазоны — из чтения файлов,
§2; всего строк файла — `wc -l`).

| файл | строк затронуто | всего строк | сложность | почему |
|---|---|---|---|---|
| `cli/add.rs` | ~15 | 146 | средняя | load + 3 поля реестра из прочитанного + upsert/write |
| `cli/remove.rs` | ~23 | 78 | низкая | чистый load→remove→write; цена — в логике `removed`/ошибке |
| `cli/reindex.rs` | ~50 | 495 | высокая | инкрементальный merge `235-277` — единственная реальная design-сложность |
| `server/routes/packages.rs` (upsert+del_ver+del_pkg) | ~29 (11+9+9) | 414 | средняя | три мутации, читают in-memory проекцию; механически однотипны |
| `cli/init.rs` | ~3 | 120 | низкая | только-пишущий seed |
| `cli/serve.rs` | 0 (опц. ~8) | 149 | нет | load остаётся (обслуживание чтения) |
| `index/memory.rs` (механика) | ~3 + НОВЫЙ `project` | 470 | средняя | `write_to` тело ПЕРЕИСПОЛЬЗУЕТСЯ (вызов над проекцией); меняется сигнатура под `WriteCtx` (Ф2) + добавляется `project()` |
| **ИТОГО код-переделка** | **~123** | — | — | (~120 мутации + ~3 init) |

Тесты (переделка helpers + новые тесты — см. §3): ~20 RMW-тестов, тела в основном
выживают; helper’ы `server_writes`/`auto_publish` (~15-20 строк) + новые тесты
журнала/projector/`rebuild --check`.

**Главное наблюдение:** объём ПЕРЕДЕЛКИ существующего кода мал (~123 строки
мутаций + ~3 init). Основная масса работы — НОВЫЙ код (журнал + проектор + xtask),
а существующее тело `write_to` (projection writer, `memory.rs:161-256`) не
выбрасывается, а ВЫЗЫВАЕТСЯ над результатом проекции. Это сильно снижает риск
«распутывания» — переделка в основном зелёная поляна поверх переиспользуемого
писателя проекции.

---

## 5. Чего в дереве ещё нет (дистанция до целевой формы)

По Приложениям А.2/А.3 и Фазе 3 плана. Оценки — порядковые, с обоснованием от
аналога в дереве (размеры аналогов — `wc -l` существующих модулей персистентности:
`checkpoint.rs` 120, `primary.rs` 236, `by_name.rs` 244, `inverted.rs` 461,
`repomd.rs` 90, `persistence.rs` 130).

| чего нет | оценка нового кода | обоснование оценки (аналог) |
|---|---|---|
| Модуль `crates/vibe-index/src/journal/` — `JournalRecord{at,actor,event}`, enum `Event` (8+ вариантов: Published/Frozen/Yanked/Removed/Renamed/Notice + ChannelSet/ChannelUnset/ForceReplaced), append NDJSON с fsync, replay, месячное шардирование `journal/2026-08.ndjson`, `checkpoint.json {last_file,last_offset}` (А.2, Ф3.1) | ~250 строк | аналог `checkpoint.rs` (120, load/save JSON) + строчно-ориентированный IO как в `primary.rs` (236); плюс enum-варианты и схема |
| `crates/vibe-index/src/journal/project.rs` — чистая `project(events) -> Index` без часов/IO/чтения каталога; apply/replace/remove-семантика по вариантам `Event` (А.3, Ф3.2) | ~200 строк | аналог — fold в `reindex.rs::run_plan` (`235-280`, сбор `by_pkgref` из записей, ~50 строк) × количество вариантов Event (8+) × логику replace/remove |
| `WriteCtx` + `write_to(&self, dir, &WriteCtx)` + перевод 5 clock-сайтов в аргументы (А.4, Ф2.1) | ~15 затронутых (не новых) | clock-сайты: `memory.rs:86`, `memory.rs:249`, `types/entry/mod.rs:167`, `add.rs:113`, `reindex.rs:232` (5 — `grep -rn "Utc::now\|SystemTime::now"`) |
| `xtask rebuild --check` — снести каталог, спроецировать из журнала, сравнить байты на герметичной фикстуре (Ф3.2 приёмка) | ~120 строк | аналог — существующие xtask’и в `xtask/` |
| Схема `schemas/journal/e1/` + запись в реестр форматов (Ф3.1, Ф4) | малая (файлы схем) | аналог — `schemas/index/e1/` (Ф4.1, параллельная фаза) |
| Событие сброса журнала для `reindex --full` (truncate/reset) | ~20 строк | нужно для режима полного ребилда; новое, аналога нет — это единственное, чего нет прямого аналога |
| self-check arch-гейт: запрет `load_from`/типов из `index_client` в сигнатурах писателей (Ф3.2, G4) | ~10 строк (grep-шаг) | аналог — существующие grep-шаги в `tools/self-check.sh` |
| Снятие `deny_unknown_fields` с типов каталога (Ф3.3, бесплатно) | 0 (удаление) | 15 мест: `aggregate.rs`(2)+`content.rs`(5)+`entry/mod.rs`(1)+`relations.rs`(6)+`repomd.rs`(1) — `grep -rc "deny_unknown_fields" crates/vibe-index/src/types` = 15, точно совпадает с B1 |
| Мульти-воркеры через git-CAS (fast-forward push журнала; проигравший re-fetch+re-append) | операционно/конвенция, малый код | рулинг 2026-08-13 в Ф3.1; не блокирует переделку RMW |

**Итого новый код — порядка ~600 строк** (журнал ~250 + проектор ~200 + xtask
~120 + схема/гейты ~30), против ~123 строк переделки существующего. Плюс новые
тесты.

---

## 6. ВЕРДИКТ

**Укладывается ли переделка в бюджет двух рабочих фаз (Ф3 + хвост)? ДА.**

Обоснование числами:

1. **Переделка существующего кода мала и однотипна:** ~123 строки мутаций
   (add ~15, remove ~23, reindex ~50, packages ×3 ~29, init ~3). Шесть RMW-путей
   механически сводятся к одному шаблону `validate(вход) → journal::append(event)
   → project(replay()) → write_to(dir, ctx)`. Серверные три мутации идентичны по
   форме.
2. **Существующий писатель проекции переиспользуется, не выбрасывается:**
   `Index::write_to` (`memory.rs:161-256`) остаётся — он вызывается над
   результатом `project()`. Меняется только сигнатура (`WriteCtx`, Ф2) и источник
   `generated_at` (из аргумента, а не `Utc::now()`). Это устраняет главный риск
   «большого распутывания».
3. **Новый код — зелёная поляна, не хирургия:** журнал (~250) и проектор (~200) —
   новые модули с аналогами в дереве (`checkpoint.rs`, fold в `run_plan`). Объём
   порядка ~600 строк нового кода — это нормальный размер одной рабочей фазы.
4. **Тесты в основном выживают:** 20 RMW-тестов — поведенческие (утверждения на
   `by-name/*.json` и HTTP-ответах), а не на механизм `write_to`; проекция
   порождает те же артефакты. Цена — в helper’ах-seed (~15-20 строк) + новые тесты
   (append/replay, `rebuild --check`, идемпотентный upsert).

**Неустранимая сложность / что может взорвать оценку:**

- **reindex `--full` vs инкремент (ЕДИНСТВЕННОЕ реальное design-решение):**
  инкрементальный merge `reindex.rs:235-277` при журнале в основном ИСЧЕЗАЕТ
  (unchanged-записи уже в журнале; проектор их удерживает), но режим `--full`
  требует события сброса журнала (truncate/reset). Семантика «сброс + повторная
  эмиссия всех `published`» должна быть определена аккуратно — это не объём, а
  решение, и оно разрешимо (аналог — checkpoint-rotate). Если здесь всплывут
  конфликты семантики каналов/`force_replaced` со сбросом — это единственное, что
  реально может раздуть оценку.
- **Ф2 (детерминизм/`WriteCtx`) — жёсткий пререквизит:** должен лечь первым или
  вместе с Ф3. Он мал (5 clock-сайтов → аргументы, ~15 строк), но без него журнал
  не даст байт-в-байт проекции и `rebuild --check` не загорится. Это зависимость,
  а не риск объёма.
- **Сервер: reproject-per-mutation под write-lock** — O(n) на мутацию при полном
  replay. План сознательно откладывает инкрементальную проекцию (объёмы фикстурные
  и малые). На малых реестрах это не стена; порог оптимизации фиксируется в отчёте
  (Ф3.2). Не блокирует переделку.

Вердикт: переделка ограничена и укладывается в Ф3 + хвост. Риск не в строках
(переделка ~123 + новый ~600 — нормальный объём одной фазы), а в одном
design-решении (семантика сброса журнала для `reindex --full`) и в пререквизите Ф2.

---

## 7. Расхождения с базовой линией и неожиданности

- **По координатам — расхождений нет:** все 6 RMW-путей + init + serve совпали с
  B3 в точности (см. §1). Дерево с 2026-08-09 по этим координатам не сдвинулось.
- **Нюанс характера «чтения» в серверных путях (не ошибка B3, а уточнение):** B3
  формулирует packages.rs как «перечитывается ради перезаписи». Технически сервер
  НЕ делает `load_from` на каждый запрос — он держит `Index` в памяти за `RwLock`
  (`AppState.index`), загруженный один раз на `serve.rs:77`. RMW здесь —
  «изменить in-memory проекцию → переписать ВЕСЬ каталог», и «прочитанное» — это
  опубликованное состояние, просто опосредованное in-memory копией. Суть D3
  («писатель не принимает на вход то, что опубликовал») нарушается точно так же;
  переделка идентична. Уточнение не меняет объёма.
- **`rescan-org` — не отдельный RMW-путь:** `rescan_org.rs:68` вызывает тот же
  `reindex::run_plan` (`grep -rn "run_plan\|rescan"`). То есть это второй
  вызыватель пути №3 (load `218` → write `281`), а не седьмой RMW-путь.
  B3 корректно его не считает отдельным; фиксирую, чтобы будущий исполнитель не
  принял его за пропущенную мутацию.
- **`deny_unknown_fields` = 15 — точно совпадает с B1** (`grep -rc` по `types/`:
  aggregate 2 + content 5 + entry/mod 1 + relations 6 + repomd 1). Снятие (Ф3.3)
  бесплатно и подтверждено замером.
- **Неожиданность (в плюс оценке):** инкрементальный merge в `reindex` — самое
  «страшное» место — при журнале не растёт, а УСЫХАЕТ (unchanged-записи уже в
  журнале). Это снижает, а не повышает риск переделки.
- **Неожиданность (в минус, учтена):** тесты глубже завязаны на механику, чем
  кажется — helpers `fresh_state`/`setup`/`build` в `server_writes`/`auto_publish`
  сеют состояние через `write_to`+`load_from` и потребуют переделки; тела тестов
  при этом выживают.

---

## 8. Как проверить этот замер

Воспроизвести каждое число документа (одна команда — один глагол, без цепочек):

```sh
# Размеры файлов мутаций и механики (§2, §4 «всего строк»)
wc -l crates/vibe-index/src/cli/add.rs crates/vibe-index/src/cli/remove.rs crates/vibe-index/src/cli/reindex.rs crates/vibe-index/src/cli/init.rs crates/vibe-index/src/cli/serve.rs crates/vibe-index/src/server/routes/packages.rs crates/vibe-index/src/index/memory.rs

# Размеры тестов (§3 «всего строк»)
wc -l crates/vibe-index/tests/server_writes.rs crates/vibe-index/tests/auto_publish.rs crates/vibe-index/tests/cli_write.rs crates/vibe-index/tests/cli_lifecycle.rs

# Все продакшн-вызовы write_to (§1 — 7 сайтов)
grep -rn "write_to" crates/vibe-index/src

# Все вызовы load_from (§1 — входы мутаций: add:51, remove:35, reindex:218 + serve:77)
grep -rn "load_from" crates/vibe-index/src

# Число функций в packages.rs (§2 — 10, из них 3 мутирующих)
grep -c "fn " crates/vibe-index/src/server/routes/packages.rs

# Число тестов на файл (§3)
grep -c "#\[tokio::test\]\|#\[test\]" crates/vibe-index/tests/server_writes.rs crates/vibe-index/tests/auto_publish.rs crates/vibe-index/tests/cli_write.rs crates/vibe-index/tests/cli_lifecycle.rs

# Clock-сайты — область Ф2 (§5)
grep -rn "Utc::now\|SystemTime::now" crates/vibe-index/src/index crates/vibe-index/src/types crates/vibe-index/src/cli crates/vibe-index/src/server

# deny_unknown_fields — 15 мест B1, Ф3.3 (§5)
grep -rc "deny_unknown_fields" crates/vibe-index/src/types

# Аналоги для оценки нового кода журнала/проектора (§5)
wc -l crates/vibe-index/src/index/checkpoint.rs crates/vibe-index/src/index/primary.rs crates/vibe-index/src/index/by_name.rs crates/vibe-index/src/index/inverted.rs crates/vibe-index/src/index/repomd.rs

# rescan-org — второй вызыватель run_plan, не отдельный путь (§7)
grep -rn "run_plan\|rescan" crates/vibe-index/src/cli

# Границы функций для диапазонов «строк затронуто» (§2) — проверка спанов:
grep -n "pub fn run\|pub(crate) fn run_plan\|pub async fn upsert\|pub async fn delete_version\|pub async fn delete_package" crates/vibe-index/src/cli/add.rs crates/vibe-index/src/cli/remove.rs crates/vibe-index/src/cli/reindex.rs crates/vibe-index/src/server/routes/packages.rs
```

Диапазоны «строк затронуто» (§2, §4) — оценки по границам реальных функций из
чтения файлов; их длины и общие размеры файлов восстанавливаются командами выше.
