# F3-PROJECT — анатомия состояния `Index` и форма проектора

Чем мерил: статическое чтение рабочего дерева `wt/F3-PROJECT` инструментами Read/Grep по `crates/**`. Что НЕ запускалось: `cargo` (любой — пакет запрещает), `git` (любой — пакет запрещает), тесты, сервер; все числа ниже — из чтения исходников, каждое утверждение о дереве несёт `файл:строку`. Периметр чтения соблюдён: `vibedeps/**` и `packages/**` не открывались; план кампании `TZ-CHANGE-NATIVE-FORMATS-v0.1.md` тоже не открывался (вне периметра `crates/**`) — девять вариантов `Event` взяты из текста пакета §3.4. Дата замера: 2026-08-14.

## 1. ВЕРДИКТ

НЕТ, ПОКА НЕ — сегодняшних типов и операций недостаточно, чтобы `project(events) -> Index` была чистой функцией; Ф3 обязана сначала завести носители. Что мешает, по убыванию веса:

1. **Самого типа события не существует.** В `crates/vibe-index/src` нет идентификаторов `Event`, `JournalRecord`, `journal`, `project`, `replay` (grep даёт только прозу док-комментариев: `crates/vibe-index/src/index/memory.rs:85`, `crates/vibe-index/src/types/entry/mod.rs:127`, TOML-секции `[project]` в фикстурах `crates/vibe-index/src/scanner/manifest.rs:54`). Журнал Ф3 — тип с нуля.
2. **Четыре каталог-уровневых поля не выводимы из потока событий.** `schema_version`, `registry_url`, `naming`, `generator` не встречаются ни в `VersionEntry` (кроме `registry` — `crates/vibe-index/src/types/entry/mod.rs:62`), ни в каком-либо факте; сегодня их несёт `repomd.json` (`crates/vibe-index/src/index/memory.rs:354-358`) или аргументы `Index::new` (`crates/vibe-index/src/index/memory.rs:94-98`). Сигнатура А.3 `project(events) -> Index` не имеет для них входа.
3. **Прямое противоречие с уже севшей Ф2.2.** Ф2.2 требует: прочитанная версия каталога сохраняется, константу штампует только `Index::new` (`crates/vibe-index/src/index/memory.rs:287-290`, тест `crates/vibe-index/src/index/memory/tests.rs:266-291`). Проектор строит `Index` с нуля — если он воспользуется `Index::new` (единственный способ завести пустой `Index`), он перетрёт версию константой `SCHEMA_VERSION = 1` (`crates/vibe-index/src/index/memory.rs:101`, константа `:29`). Предвестник этого бага уже сидит в дереве: `reindex` читает каталог через `load_from` (`crates/vibe-index/src/cli/reindex.rs:224`), но пересобирает через `Index::new` (`:252`) и теряет прочитанную `schema_version` — он переносит `registry`/`registry_url`/`naming`/`generator` (`:253-258`), а версию — нет. Скажу громко: **путь `reindex` сегодня нарушает инвариант Ф2.2, и проектор, повторяющий его форму, нарушит его тем же способом.**
4. **Пять из девяти событий А.2 не имеют операции-приёмника.** `Yanked`, `Frozen`, `Removed`, `Renamed`, `ChannelSet`/`ChannelUnset`, `Notice` — поля-приёмники у части есть, ни одна операция `Index` их не исполняет (детально в §6).
5. **`PackageEntry.indexed_at` — функция порядка, а не множества событий** (`crates/vibe-index/src/index/memory.rs:122-124`), и ни один пересчёт его не чинит (`PackageEntry::finalise` его не трогает — `crates/vibe-index/src/types/entry/aggregate.rs:47-55`). Свёртка одного и того же набора событий в разном порядке даст разные байты by-name файлов (доказательство в §5).

Что уже готово: свёртка `by_pkgref` через `upsert`/`remove_*` воспроизводима и (кроме п. 5) детерминирована сортировками `finalise`; приёмник `tombstones` существует и переживает round-trip; `quarantined` при проекции пуст по построению (§7).

## 2. Сверка опорных координат (B1..B7)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | `pub struct Index` в `memory.rs:72`, ровно девять полей | ПОДТВЕРЖДЕНО | декларация `crates/vibe-index/src/index/memory.rs:72` (за `#[derive(Debug, Clone)]` на `:71`); поля `schema_version` `:73`, `registry` `:74`, `registry_url` `:75`, `naming` `:76`, `generator` `:77`, `generated_at` `:78`, `by_pkgref` `:79`, `quarantined` `:83`, `tombstones` `:86` — ровно девять |
| B2 | `new` штампует версию из `SCHEMA_VERSION` (`:29`), генератор из `default_generator()` (`:382`) | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/index/memory.rs:94` (сигнатура), `:101` (`schema_version: SCHEMA_VERSION`), `:105` (`generator: default_generator()`); константа `:29`; функция `:382-384` |
| B3 | `load_from` (`:315`) берёт шесть полей из `repomd.json`, три собирает из by-name | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/index/memory.rs:316` (`repomd::read`), `:317` (`by_name::read_all`), `:354-359` (шесть полей из `manifest`), `:318-352` + `:360-362` (`by_pkgref`, `quarantined`, `tombstones` из by-name) |
| B4 | `upsert` (`:120`) возвращает bool по сравнению значения целиком, хост-`PackageEntry` через `PackageEntry::new(…, entry.indexed_at)` (`:123`) | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/index/memory.rs:120` (сигнатура `-> bool`), `:128` (`pkg.versions.contains(&entry)`), `:122-124` (`or_insert_with(\|\| PackageEntry::new(entry.group.clone(), entry.name.clone(), entry.indexed_at))`) |
| B5 | оба поля помечены «in memory only» (`:81-86`), но `tombstones` проектируются в by-name в `write_to` (`:242-247`), `quarantined` — нет | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/index/memory.rs:82` («In memory only — never serialised into any catalog file»), `:84-85` («in memory only; `write_to` projects them back…»), проекция `:242-247`; `quarantined` в `write_to` (`:197-303`) не упоминается вовсе |
| B6 | `UNDERSTOOD` пуст ⇒ любой непустой `must_understand` — карантин | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/index/quarantine.rs:18` (`pub const UNDERSTOOD: &[&str] = &[];`), `:33-39` (`missing_capabilities` фильтрует всё, чего нет в `UNDERSTOOD`), применение `crates/vibe-index/src/index/memory.rs:328-348` |
| B7 | в `crates/vibe-index/src` нет идентификаторов `project`, `journal`, `Event`, `replay` | ПОДТВЕРЖДЕНО | grep `project\|journal\|replay\|Event` по `crates/vibe-index/src`: 14 хитов, все — проза/имена тестов/TOML-фикстуры: `memory.rs:85` («projects them back»), `types/entry/mod.rs:127` («a journal fact»), `[project]`-секции `scanner/manifest.rs:54,374,377`, имена тестов `cli/add.rs:164`, `scanner/org_walk.rs:292`; идентификаторов нет. Уточнение: `enum Event` есть в ДРУГОМ крейте — `crates/progress-core/src/journal.rs:19` (журнал кампаний PROP-043), к индексу пакетов отношения не имеет |

## 3. Поле за полем: состояние `Index` под проекцией

Тип ключа: `pub type PkgKey = (Group, String)` (`crates/vibe-index/src/index/memory.rs:27`).

### 3.1.1. `schema_version: u32`

- Сегодня: `new` — из константы `SCHEMA_VERSION = 1` (`crates/vibe-index/src/index/memory.rs:101`, константа `:29`); `load_from` — из прочитанного манифеста (`:354`), запись в новый манифест — из состояния (`:292`, комментарий Ф2.2 `:287-290`).
- Из событий: НЕТ. Ни один факт уровня каталога версию схемы не несёт; `VersionEntry.schema_version` (`crates/vibe-index/src/types/entry/mod.rs:44`) — версия схемы ЗАПИСИ, не каталога.
- Кандидаты в дереве: (а) аргумент проекции — как `registry`/`naming` сегодня аргументы `Index::new` (`memory.rs:94-98`); (б) событие каталог-уровня «версия установлена» с правилом «последнее событие выигрывает» — готовый прецедент свёртки скаляра из потока есть в `crates/progress-core/src/journal.rs:34-40` (`Phase`, «the value of the LAST `phase` event is the campaign's current phase»); (в) чтение текущего `repomd.json` — запрещено А.3 («без чтения каталога»). Константы-тёзки: `Repomd::SCHEMA_VERSION` (`crates/vibe-index/src/types/repomd.rs:39`), `VersionEntry::SCHEMA_VERSION` (`crates/vibe-index/src/types/entry/mod.rs:145`), `default_schema() = 1` чекпойнта (`crates/vibe-index/src/index/checkpoint.rs:39-41`) — все равны 1, все_catalog-независимые штампы.
- Цена ошибки: будущая версия каталога молча выдаст себя за текущую — ровно то, против чего стоит тест Ф2.2 (`crates/vibe-index/src/index/memory/tests.rs:262-264`: «re-stamping it with the reader's own constant would make a future-version catalog silently claim to be ours»).

### 3.1.2. `registry: String`

- Сегодня: `new` — аргумент (`crates/vibe-index/src/index/memory.rs:102`); `load_from` — из манифеста (`:355`); при `init` — флаг CLI (`crates/vibe-index/src/cli/init.rs:23-25`, вызов `:53`); при `reindex` — переносится из прочитанного (`crates/vibe-index/src/cli/reindex.rs:253`).
- Из событий: ЧАСТИЧНО. Каждая `VersionEntry` несёт `registry: String` (`crates/vibe-index/src/types/entry/mod.rs:62`), так что имя реестра выводимо из любой записи (консистентность даже проверяется на входе сервера: `crates/vibe-index/src/server/routes/packages.rs:237-245`). Но событие удаления последней записи оставит реестр без источника.
- Кандидаты: аргумент проекции (надёжно); либо событие рождения каталога. В записи поле есть (`:62`) — цитата существования кандидата.
- Цена ошибки: сервер начнёт отвергать записи по scope-проверке (`packages.rs:237-245`), CLI будет печатать чужое имя (`crates/vibe-index/src/cli/list.rs:102`).

### 3.1.3. `registry_url: String`

- Сегодня: `new` — аргумент (`memory.rs:103`); `load_from` — из манифеста (`:356`); `init` — флаг (`init.rs:27-29`); `reindex` — перенос (`reindex.rs:254`).
- Из событий: НЕТ — в `VersionEntry` поля нет (полный список полей записи `types/entry/mod.rs:43-142` его не содержит).
- Кандидаты: только аргумент проекции или событие каталог-уровня; в записи его точно нет. Сегодня единственный производитель значения — CLI `init` (`init.rs:53`) и перенос в `reindex` (`reindex.rs:254`).
- Цена ошибки: `vibe-index add` начнёт сочинять битые `source_url` по умолчанию (`crates/vibe-index/src/cli/add.rs:80-82` — URL складывается из `registry_url`).

### 3.1.4. `naming: NamingConvention`

- Сегодня: `new` — аргумент (`memory.rs:104`); `load_from` — из манифеста (`:357`); `init` — флаг с дефолтом `fqdn` (`init.rs:30-34`); `reindex` — перенос (`reindex.rs:255`).
- Из событий: НЕТ — в записи поля нет (`types/entry/mod.rs:43-142`).
- Кандидаты: аргумент проекции. Потребитель при инкрементальном reindex отображает запись → имя репо через `naming.repo_name` (`reindex.rs:265-267`) — неверная конвенция разобьёт инкрементальные запуски.
- Цена ошибки: неверная `repo_name` ⇒ инкрементальный reindex примет живой репо за «unchanged» или наоборот (`reindex.rs:261-288`).

### 3.1.5. `generator: String`

- Сегодня: `new` — `default_generator()` = `"vibe-index {CARGO_PKG_VERSION}"` (`memory.rs:105`, `:382-384`); `load_from` — из манифеста (`:358`); в манифест — из состояния (`:297`). `reindex` после `Index::new` перетирает поле значением, собранным в opts (`reindex.rs:258`; сборка `:237` — та же формула, что `default_generator()`).
- Из событий: ЧАСТИЧНО. Значение по умолчанию выводимо (чистая функция от версии пакета, `memory.rs:382-384`); прочитанное из чужого манифеста — нет.
- Единственность перетирания: ДА — grep `\.generator` по `crates/` даёт ровно одно присваивание после конструкции: `reindex.rs:258`; остальные хиты — чтения (`crates/vibe-index/src/server/state.rs:113`, `crates/vibe-index/src/server/routes/admin.rs:32`, `crates/vibe-index/src/cli/dump.rs:62`) и литералы в `memory.rs:297,358`.
- Цена ошибки: косметическая — строка в `admin:status` и `dump --format json` (`admin.rs:32`, `dump.rs:62`).

### 3.1.6. `generated_at: DateTime<Utc>`

- Сегодня: `new` — аргумент `at` (`memory.rs:106`); `load_from` — из манифеста (`:359`).
- Читатели: в `crates/**` ровно ОДИН продакшн-читатель поля — `crates/vibe-index/src/cli/dump.rs:61` (grep `\.generated_at` по `crates/`: остальные хиты — чужие структуры `WorkspaceOriginEntry`/`lockfile.meta`/манифест сканера). Сервер поле не читает (`server/state.rs` его не копирует — `:112-123`).
- Важная дыра: `write_to` поле ИГНОРИРУЕТ — в манифест идёт `generated_at: ctx.at` из `WriteCtx` (`memory.rs:296`), не `self.generated_at`. Поле — почти мёртвое состояние: загружено/установлено, прочитано одним дампом, никогда не пишется.
- Из событий: НЕТ (множество `indexed_at` записей — не «момент генерации каталога»; см. §5).
- Кандидаты: внешний вход времени — прецедент уже есть: `WriteCtx { at }` (`memory.rs:33-35`), который сегодня и штампует манифест (`:296`); при проекции естественно `generated_at = момент проекции`, переданный извне (иначе `project` перестаёт быть чистой).
- Цена ошибки: минимальная (один dump-вывод); скорее гигиена, чем риск.

### 3.1.7. `by_pkgref: BTreeMap<PkgKey, PackageEntry>`

- Сегодня: `new` — пустая map (`memory.rs:107`); `load_from` — сборка из by-name файлов с карантин-фильтром и `finalise` (`:318-352`); мутации — `upsert`/`remove_version`/`remove_package` (`:120-160`).
- Из событий: ДА — это единственное поле, полностью сворачиваемое из потока `Published`/`Removed`-подобных фактов (см. §4).
- Цена ошибки: содержимое каталога; любые искажения видны сразу в `primary.jsonl`/by-name.

### 3.1.8. `quarantined: Vec<Quarantined>`

- Сегодня: `new` — пусто (`memory.rs:108`); `load_from` — заполняется фильтром `must_understand` (`:328-348`); тип записи `Quarantined` — `crates/vibe-index/src/index/quarantine.rs:24-29`.
- Из событий: НЕТ по построению — карантин возникает только при ЧТЕНИИ чужого каталога читателем, которому не хватает возможностей (`memory.rs:310-314`); `upsert` не карантинит ничего (`:120-135` — фильтра там нет). Журнал — наша истина, отвергнутые записи в него не попадают.
- Откуда при проекции: НИОТКУДА — поле обязано быть пустым (см. §7).
- Читатели: в `crates/**` НЕТ ни одного читателя `index.quarantined` вне `index/memory.rs` и её тестов (`memory/tests.rs:195-199`) — grep по `crates/` подтверждает. Поле сегодня — только WARN-лог при загрузке (`memory.rs:333-339`).
- Цена ошибки: нулевая для файлов (не сериализуется); максимум — вранье оператору.

### 3.1.9. `tombstones: BTreeMap<String, Tombstone>`

- Сегодня: `new` — пусто (`memory.rs:109`); `load_from` — из `name_entry.tombstone` by-name файлов (`:322-324`); `write_to` — проецирует обратно в by-name (`:242-247`), tombstone-only имя получает свой файл (`:232-233`, тест `memory/tests.rs:233-256`).
- Из событий: ДА — `Removed`/`Renamed` несут ровно те данные, что лежат в `Tombstone { reason, superseded_by }` (`crates/vibe-index/src/types/entry/aggregate.rs:62-70`).
- Но: операции-приёмника НЕТ — ни один продакшн-путь не вставляет tombstone; `tombstones.insert` встречается только в тестах (`memory/tests.rs:209,236,344`). Событие положить некуда, кроме как писать в публичное поле напрямую.
- Цена ошибки: молчание имени — нарушение закона no-silence PROP-044 §2 (`aggregate.rs:58-61`).

## 4. Мутирующие операции и свёртка

### 4.1. `upsert`

- Сигнатура и строка: `pub fn upsert(&mut self, entry: VersionEntry) -> bool` — `crates/vibe-index/src/index/memory.rs:120`.
- Шаги по `by_pkgref`: ключ `(group, name)` (`:121`); хост-`PackageEntry` создаётся лениво, `indexed_at` — из ПЕРВОЙ вставленной версии (`:122-124`); дубликат-по-значению → `false` без касания (`:128-130`); иначе старая запись с тем же номером выкидывается (`:131`), новая пушится (`:132`), агрегат пересчитывается `finalise()` (`:133`).
- Возврат и читатели: `bool` «изменилось». Читают: серверный upsert — gate записи и статистика (`crates/vibe-index/src/server/routes/packages.rs:262-277`); CLI `add` — ИГНОРИРУЕТ (`crates/vibe-index/src/cli/add.rs:129`, результат не связывается); CLI `reindex` — игнорирует дважды (`crates/vibe-index/src/cli/reindex.rs:285,290`); тесты Ф2-3 (`memory/tests.rs:300-323`).
- Свёртка: воспроизводима fold'ом по порядку журнала; коммутативна по значению поля КРОМЕ `PackageEntry.indexed_at` (см. §5). «ForceReplaced» — это в точности ветка `:128-134` (отличающаяся запись под тем же номером = замена).

### 4.2. `remove_version`

- Сигнатура: `pub fn remove_version(&mut self, group: &Group, name: &str, version: &semver::Version) -> bool` — `memory.rs:141`.
- Шаги: нет пакета → `false` (`:143-145`); `retain` по номеру (`:147`); если длина изменилась — `finalise()` (`:148-151`). Пустой пакет ОСТАЁТСЯ в map (док `:137-140`).
- Возврат и читатели: серверный delete (`packages.rs:317-328`); CLI `remove` — `false` превращается в ошибку «nothing to remove» (`crates/vibe-index/src/cli/remove.rs:46,50-61`).
- Свёртка: воспроизводима тривиально; результат — функция множества событий (retain + сортировка `finalise`).

### 4.3. `remove_package`

- Сигнатура: `pub fn remove_package(&mut self, group: &Group, name: &str) -> bool` — `memory.rs:156`.
- Шаги: удаление ключа из map целиком (`:157-159`). Надгробие НЕ ставится — имя просто замолкает (подробнее §10).
- Возврат и читатели: сервер (`packages.rs:347-358`); CLI `remove` (`remove.rs:48`).
- Свёртка: воспроизводима; но семантика «удалить молча» противоречит no-silence — для события `Removed` проектор обязан делать БОЛЬШЕ, чем сегодняшняя операция (поставить tombstone).

### 4.4. Пересчёты агрегатов

- `PackageEntry::finalise` (`crates/vibe-index/src/types/entry/aggregate.rs:47-55`): сортирует `versions` по возрастанию `version` (`:48`) и перевычисляет `latest_stable` = максимальная версия без pre-тега (`:49-54`). Вход — только `versions`; детерминирована как функция множества записей; `indexed_at` НЕ трогает.
- `NameEntry::finalise` (`:111-116`): сортирует кандидатов по `group` (`:112`) и ставит `indexed_at` = максимум из кандидатов (`:113-114`). Детерминирована как функция множества кандидатов; но каждое `PackageEntry.indexed_at` само зависит от порядка вставки (§5) — недетерминизм проникает транзитивно.

## 5. Порядок и детерминизм

- Порядок итерации `by_pkgref`: `BTreeMap<PkgKey, PackageEntry>` (`memory.rs:79`) — обход в порядке ключа `(Group, String)`; док прямо это фиксирует (`:169` «`by_pkgref` iterates in `(group, name)` order»). Сортировки на выходе дополнительно страхуют: `primary.jsonl` — `entries.sort_by(sort_key)` (`crates/vibe-index/src/index/primary.rs:28`), by-name кандидаты — `aggregate.rs:112`, инвертированные виды — `crates/vibe-index/src/index/inverted.rs:144-165`.
- `PackageEntry.indexed_at` от порядка: **ДА, зависит.** Доказательство: `or_insert_with(\|\| PackageEntry::new(entry.group.clone(), entry.name.clone(), entry.indexed_at))` (`memory.rs:122-124`) — берётся `indexed_at` той версии, которая пришла ПЕРВОЙ; `finalise` поле не пересчитывает (`aggregate.rs:47-55`). Один и тот же набор событий `{v0.1.0@T1, v0.2.0@T2}` в порядке `(0.2.0, 0.1.0)` даст `indexed_at = T2`, в порядке `(0.1.0, 0.2.0)` — `T1`. Сегодня это маскируется тем, что все записи одной команды несут один `at` (F2-1: `reindex.rs:220`, `add.rs:53`) — при реплее журнала с разными временами событий маскировка исчезает.
- `NameEntry.indexed_at` в `write_to`: рождается из `ctx.at` (`NameEntry::new(pkg.name.clone(), ctx.at)` — `memory.rs:238,245`), затем `NameEntry::finalise` перезапишет его максимумом `indexed_at` кандидатов, если кандидаты есть (`aggregate.rs:113-114`); tombstone-only имя оставляет `ctx.at`. Как следствие — значение транзитивно наследует порядко-зависимость `PackageEntry.indexed_at` и попадает в байты by-name файла.
- Исчерпывающий список порядко-зависимых агрегатов: (1) `PackageEntry.indexed_at` — первичный источник (`memory.rs:122-124`); (2) `NameEntry.indexed_at` — транзитивно (`aggregate.rs:113-114`); (3) порядок `Vec<Quarantined>` — следует порядку чтения by-name файлов через `WalkDir` (`crates/vibe-index/src/index/by_name.rs:78-95`, пуш `memory.rs:340-345`) — на байты каталога не влияет (не сериализуется). ВСЁ ОСТАЛЬНОЕ (`versions`, `latest_stable`, порядок кандидатов, строки primary/by-cap/by-purl) сортировками детерминировано (`aggregate.rs:48,54,112`; `primary.rs:28`; `inverted.rs:144-165`).

## 6. События А.2 против сегодняшних операций

| событие | операция `Index` сегодня | поле-приёмник в типах | проектор обязан | класс |
|---|---|---|---|---|
| `Published` | `upsert` — `memory.rs:120` | `VersionEntry` целиком (`crates/vibe-index/src/types/entry/mod.rs:43-142`) + агрегаты `PackageEntry` (`aggregate.rs:26`) | вставить/заменить версию, прогнать `finalise` | готово |
| `Frozen` | нет; `frozen` ставится только при рождении записи из манифеста (`crates/vibe-index/src/cli/add.rs:120`, `crates/vibe-index/src/scanner/org_walk.rs:233`) | `VersionEntry.frozen` (`types/entry/mod.rs:138`) | заменить версию со `frozen = true` (переход односторонний — док `:134-137`) | есть поле, нет операции |
| `Yanked` | нет; продакшн-писателей `true` не существует (grep: только `false` в `add.rs:119`, `org_walk.rs:232`, фикстуры; `true` — лишь тест `types/entry/tests.rs:176`) | `VersionEntry.yanked` (`types/entry/mod.rs:130`) | заменить версию со `yanked = true` | есть поле, нет операции |
| `Removed` | `remove_version`/`remove_package` (`memory.rs:141,156`) удаляют, но надгробие НЕ ставят | `Index.tombstones` (`memory.rs:86`) + `Tombstone` (`aggregate.rs:62-70`) существуют; вставка — только тесты (`memory/tests.rs:209,236,344`) | удалить версии/пакет И поставить `Tombstone` (no-silence) | есть поле, нет операции |
| `Renamed` | нет | со стороны смерти имени — `Tombstone.superseded_by` (`aggregate.rs:69`); поля-алиаса/нового имени на `NameEntry` нет (`aggregate.rs:83-96`) | tombstone со `superseded_by` + публикация под новым именем | есть поле, нет операции |
| `Notice` | нет | нет (grep `notice` по `crates/vibe-index/src` — ноль хитов) | завести носитель (поле на записи/имени или отдельный вид) | нет ни поля, ни операции |
| `ChannelSet` | нет; каналы проставляются только при сканировании манифеста (`crates/vibe-index/src/scanner/manifest.rs:222,229-231`) | `SubskillEntry.channels: Vec<String>` (`crates/vibe-index/src/types/entry/content.rs:69`) | дописать канал конкретному сабскиллу версии | есть поле, нет операции |
| `ChannelUnset` | нет | `SubskillEntry.channels` (`content.rs:69`) | выкинуть канал | есть поле, нет операции |
| `ForceReplaced` | `upsert` веткой замены: отличающаяся запись под тем же номером (`memory.rs:128-134`; тест `memory/tests.rs:300-323`) | `VersionEntry` (значение целиком) | заменить значение версии | готово |

Счёт: готово — 2 (`Published`, `ForceReplaced`); есть поле, нет операции — 6; нет ни поля, ни операции — 1 (`Notice`).

## 7. Носители, которых нет в файлах

- Цикл `tombstones`: внутрь — `load_from` собирает из `name_entry.tombstone` каждого by-name файла (`memory.rs:322-324`); наружу — `write_to` для каждого `(name, ts)` находит/создаёт `NameEntry` и ставит `slot.tombstone = Some(ts.clone())` (`:242-247`); tombstone-only имя получает файл (`:232-233`, `:243-245`). Round-trip закрыт тестами (`memory/tests.rs:205-227,233-256`).
- `quarantined` при проекции: **всегда пусто — подтверждено логикой.** Запись попадает в карантин только в `load_from`, при чтении ЧУЖОГО каталога читателем с недостаточными возможностями (`memory.rs:328-348`, `quarantine.rs:33-39`); `upsert` фильтра не имеет (`:120-135`). Журнал — наша истина: отвергнутая запись в него не попадает, значит из событий карантин не восстановим и не должен. Поле при проекции берётся пустым (`Vec::new()`), как и в `Index::new` (`memory.rs:108`).
- Свип «в память, не в файлы» по `crates/vibe-index/src`: ровно ТРИ носителя — (1) `Index.quarantined` («In memory only — never serialised» — `memory.rs:80-83`); (2) `Index.tombstones` («in memory only; `write_to` projects them back» — `memory.rs:84-86` — с оговоркой: в файлы ВСЁ ЖЕ проектируется, «in memory only» значит «не имеет собственного файла»); (3) запись `Quarantined` («Lives in memory only — never written to any catalog file» — `quarantine.rs:20-23`). Отдельная категория — `state/**`: сериализуются на диск, но это рантайм-состояние хоста, не каталог (гитигнорится целиком — `crates/vibe-index/src/cli/init.rs:72-77`): чекпойнт инкрементального reindex (`crates/vibe-index/src/index/checkpoint.rs:51-53`), PID-лок сервера, org-кэш (`crates/vibe-index/src/scanner/org_cache.rs`).

## 8. Кто строит и кто читает `Index`

Конструкции (grep `Index \{` по `crates/vibe-index`: литералы только в `memory.rs:100` (`new`) и `:353` (`load_from`)):

- `Index::new`, продакшн: `cli/init.rs:53`, `cli/reindex.rs:252`.
- `Index::new`, тесты/доки: `server/metrics.rs:96` (`#[cfg(test)]`), `cli/add.rs:168` (`#[cfg(test)]`), доктест `memory.rs:48`, `index/memory/tests.rs:29`, `tests/auto_publish.rs:113`, `tests/rate_limit_e2e.rs:30`, `tests/server_writes.rs:66`, `tests/server_e2e.rs:79`, `tests/seam_fakes.rs:87`.
- `Index::load_from`, продакшн (11): `cli/serve.rs:71`, `cli/search.rs:50`, `cli/remove.rs:40`, `cli/reindex.rs:224`, `cli/purls.rs:44`, `cli/outdated.rs:57`, `cli/list.rs:58`, `cli/get.rs:44`, `cli/dump.rs:33`, `cli/capabilities.rs:42`, `cli/add.rs:56`.
- `Index::load_from`, тесты: `index/memory/tests.rs:108,133,184,219,225,249,254,281`, `cli/add.rs:194`, `tests/auto_publish.rs:157`, `tests/rate_limit_e2e.rs:46`, `tests/server_writes.rs:82`, `tests/seam_fakes.rs:95`.

Читатели полей вне `index/` (кто зависит от девяти полей):

- `schema_version`: `cli/dump.rs:57` — единственный.
- `registry`: `routes/packages.rs:149,237,243` (scope-проверка записи), `routes/admin.rs:30`, `routes/health.rs:18,27`, `cli/add.rs:99`, `cli/list.rs:89,102`, `cli/reindex.rs:234,253,306`, `cli/init.rs:56,59`.
- `registry_url`: `routes/admin.rs:31`, `cli/add.rs:81`, `cli/reindex.rs:235,254`, `cli/init.rs:56,61`.
- `naming`: `cli/add.rs:81`, `cli/reindex.rs:236,255,265-267`, `cli/init.rs:62`, `cli/dump.rs:60`.
- `generator`: `server/state.rs:113` (копия в `AppState.generator` на старте), `cli/dump.rs:62`; читатель копии — `routes/admin.rs:32`.
- `generated_at`: `cli/dump.rs:61` — единственный (см. §3.1.6).
- `by_pkgref` (напрямую): `cli/list.rs:60`, `cli/reindex.rs:433`, `routes/packages.rs:129`; через методы — `index/search.rs:74,123,152` (search/lookup_capability/lookup_purl), `inverted.rs:105` (`InvertedView::from_entries` по `iter_versions`, `memory.rs:263`).
- `quarantined`: читателей вне `index/memory.rs` НЕТ.
- `tombstones`: читателей вне `index/memory.rs` НЕТ.

Тесты, ломающиеся от НОВОГО поля `Index`:

- Поле, заполняемое внутри `new`/`load_from`: внешних поломок нет — `Index` строится литералом ровно в двух местах (`memory.rs:100,353`), `PartialEq` на `Index` не выведен (`memory.rs:71`), ни один тест не сравнивает `Index` целиком.
- Поле, требующее ВНЕШНЕГО входа (сигнатура `new`/`load_from` растёт): ломаются все площадки вызова — для `load_from` это 11 продакшн-вызовов (список выше) и 13 тестовых (`memory/tests.rs` ×8, `add.rs:194`, `auto_publish.rs:157`, `rate_limit_e2e.rs:46`, `server_writes.rs:82`, `seam_fakes.rs:95`); для `new` — 2 продакшн (`init.rs:53`, `reindex.rs:252`) и 8 тестовых/доковых (`metrics.rs:96`, `add.rs:168`, `memory/tests.rs:29`, доктест `memory.rs:48`, `auto_publish.rs:113`, `rate_limit_e2e.rs:30`, `server_writes.rs:66`, `server_e2e.rs:79`).
- Тесты, завязанные на ЗНАЧЕНИЯ полей: round-trip `memory/tests.rs:109-111` (registry/url/naming), Ф2-2 `memory/tests.rs:266-291` (schema_version — покраснеет, если проектор перетрёт версию константой; это желаемое покраснение), byte-детерминизм `memory/tests.rs:331-355` (покраснеет от любого недетерминизма вроде §5).

## 9. Бюджет длины файлов

`wc -l` (бюджет проекта — 600 строк на файл; «тесными» считаю запас < 150):

| файл | строк | запас до 600 |
|---|---|---|
| `crates/vibe-index/src/index/inverted.rs` | 464 | **136 — тесный** |
| `crates/vibe-index/src/index/memory.rs` | 388 | 212 |
| `crates/vibe-index/src/index/by_name.rs` | 247 | 353 |
| `crates/vibe-index/src/index/primary.rs` | 239 | 361 |
| `crates/vibe-index/src/index/search.rs` | 230 | 370 |
| `crates/vibe-index/src/index/repomd.rs` | 90 | 510 |
| `crates/vibe-index/src/index/persistence.rs` | 130 | 470 |
| `crates/vibe-index/src/index/quarantine.rs` | 55 | 545 |
| `crates/vibe-index/src/index/checkpoint.rs` | 120 | 480 |
| `crates/vibe-index/src/index/mod.rs` | 25 | 575 |
| `crates/vibe-index/src/index/memory/tests.rs` | 394 | 206 |

Единственный файл с запасом меньше 150 — `inverted.rs` (136): новый код Ф3 туда без раскола не поместится. `memory.rs` (388) ещё держит ~200 строк, но проектор + журнал в него целиком не войдут — у дерева уже есть прецедент выноса тестов в `memory/tests.rs` через `#[path]` (`memory.rs:386-388`); для Ф3 естественен новый модуль (например, `index/projection.rs`), а не рост `memory.rs`.

## 10. Дыры и неожиданности

1. **`write_to` игнорирует `Index.generated_at`.** Манифест штампуется `ctx.at` (`memory.rs:296`), поле же читает один `dump` (`cli/dump.rs:61`). Два «generated_at» расходятся: после load→write в файле будет время записи, в поле — время чужой генерации. Для Ф3 это подсказка: время — вход проекции (как `WriteCtx`), а не свёртываемое состояние.
2. **`reindex` уже нарушает Ф2.2.** Переносит из прочитанного каталога `registry`/`registry_url`/`naming`/`generator` (`reindex.rs:253-258`), но собирает `next` через `Index::new` (`:252`), который штампует `SCHEMA_VERSION` константой (`memory.rs:101`). Каталог с чужой (большей) версией после reindex молча станет «нашей». Ф2-2-тест (`memory/tests.rs:266-291`) прикрывает только путь load→write, не reindex. Проектор обязан не повторить эту форму — а лучше: починить и reindex.
3. **Присваивание `generator` в `reindex.rs:258` сегодня мёртвое по значению**: `opts.generator` собран той же формулой, что `default_generator()` (`reindex.rs:237` против `memory.rs:382-384`). Асимметрия пункта 2 от этого только заметнее: то, что совпадает, переносят; то, что разъедется (версия), — нет.
4. **`remove_package` нарушает no-silence PROP-044 §2.** Удаляет ключ без надгробия (`memory.rs:156-160`), by-name файл стирается — тест прямо утверждает исчезновение файла (`memory/tests.rs:158-169`, assert `:167`). Закон («a name that ever existed must answer, never fall silent» — `aggregate.rs:58-61`, `memory.rs:232-233`) выполняется только для имён, чьи надгробия уже лежат в файлах, — продакшн-способа их создать нет.
5. **`upsert` не карантинит `must_understand`.** Карантин живёт только в `load_from` (`memory.rs:328-348`); запись с неизвестными возможностями, пришедшая через `add`/сервер, попадает в `by_pkgref` свободно (`add.rs:129`, `packages.rs:262`). Граница «refuse» стоит на чтении чужого каталога, не на записи своего.
6. **`quarantined` — мёртвый груз для оператора**: вне `memory.rs` и тестов (`memory/tests.rs:195-199`) поле никто не читает (grep по `crates/`); ни CLI, ни сервер его не показывают — только WARN в лог (`memory.rs:333-339`).
7. **`AppState.generator` — снимок на старте** (`server/state.rs:113`): если `Index.generator` сменится после подъёма сервера, `admin:status` останется со старым (`routes/admin.rs:32` читает копию, не `index.generator`).
8. **Четыре копии «версии схемы 1»** в одном крейте: `memory.rs:29`, `Repomd::SCHEMA_VERSION` (`types/repomd.rs:39`), `VersionEntry::SCHEMA_VERSION` (`types/entry/mod.rs:145`), `default_schema()` чекпойнта (`checkpoint.rs:39-41`) — сегодня все равны; в день, когда разъедутся, проектору понадобится правило, какую из них несёт событие.
9. **Тёзка-ловушка: `enum Event` существует** — но в `crates/progress-core/src/journal.rs:17-41`, журнал кампаний PROP-043 (append-only JSONL, кеге-case тег, `ts` в каждом варианте, свёртка «последнее событие выигрывает» для `Phase`). Для Ф3 это не помеха, а готовый прецедент формы журнала — и повод не называть свой тип просто `Event` в общем пространстве имён.
10. **Ни один продакшн-путь не создаёт `Tombstone`**: `tombstones.insert` — только тесты (`memory/tests.rs:209,236,344`). Носитель dead-on-arrival: живёт round-trip'ом с файлами, которые некому породить.
11. **`Index::get`/`candidates_for` не отличают yanked** — фильтра по `yanked` нет нигде в `index/**` (grep по `src` даёт `yanked` только в типах, CLI-проекциях и фикстурах): поле пишется в провод и читается потребителем, сам индекс его не использует.
12. **Серверные мутации берут время каждое своё**: `Utc::now()` в каждом хендлере (`packages.rs:267,319,349`) — правильный «краевой» вход, но для журнала Ф3 это значит: время факта обязано стать частью события уже на записи, иначе реплей не воспроизведёт даже `indexed_at`.

## 11. Как воспроизвести этот замер

Команды — из корня рабочего дерева, Git Bash; только чтение:

- Посчитать длины файлов (§9): `wc -l crates/vibe-index/src/index/*.rs crates/vibe-index/src/index/memory/*.rs`
- Снять декларацию и поля `Index` (B1): `grep -n "pub struct Index" -A 16 crates/vibe-index/src/index/memory.rs`
- Найти все конструкции (§8): `grep -rn "Index::new\|Index::load_from" crates/`
- Найти литералы `Index {`: `grep -rn "Index \{" crates/vibe-index`
- Проверить B7: `grep -rn "project\|journal\|replay" crates/vibe-index/src`
- Где читается/пишется версия схемы (§3.1.1): `grep -rn "SCHEMA_VERSION" crates/vibe-index/src`
- Читатели `generated_at` (§3.1.6): `grep -rn "\.generated_at" crates/`
- Читатели/писатели `generator` (§3.1.5): `grep -rn "\.generator" crates/`
- Носители без файлов (§7): `grep -rn "quarantined\|tombstones" crates/vibe-index/src` и `grep -rni "in memory only\|never serialised\|never written" crates/vibe-index/src`
- Вызыватели мутаций (§4): `grep -rn "\.upsert(\|remove_version(\|remove_package(" crates/`
- Пустой `UNDERSTOOD` (B6): `grep -rn "UNDERSTOOD" crates/vibe-index/src`
- Приёмники событий (§6): `grep -rn "yanked\|frozen" crates/vibe-index/src` и `grep -rni "notice\|channel" crates/vibe-index/src`
- Тёзка `Event` (§10.9): `grep -rn "enum Event" crates/`
