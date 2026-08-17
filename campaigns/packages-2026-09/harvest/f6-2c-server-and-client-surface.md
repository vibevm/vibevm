# Ф6.2c-ЗАМЕР: серверная поверхность vibe-index и клиентская сторона vibe-registry после Ф6.2a

Жанр — замер (пакет F62C-PROBE). Ничего не чинилось и не правилось: два
созданных файла — эта находка и `WORKER-REPORT-F62C-PROBE.md`. Все факты
ниже добыты чтением и грепом дерева ворктри `F62C-PROBE`; расхождение с
числами пакета называется по §0.8 (дерево право).

## 0. Как это мерилось

- Инструменты: Read/Glob/Grep (ripgrep) по дереву ворктри; дословные
  команды: `grep -c "\.route(" crates/vibe-index/src/server/mod.rs` → **16**
  и периметр `find` из §0.2 (вывод — в отчёте воркера). Pruning: чтение
  ограничено `crates/vibe-index/**`, `crates/vibe-registry/**`,
  `crates/vibe-wire/src/{generated,behaviour}/**`, `schemas/**`,
  `formats/{corpora,vocabularies.json}`, `campaigns/packages-2026-09/harvest/f6-2*.md`.
- **Cargo не запускался ни разу** (запрещён пакетом §0.5). Ни одно
  утверждение ниже не проверялось прогоном; там, где поведение pinned
  тестом, даётся координата теста как чтение-подкрепление.
- Пустые выводы грепа и их контрольные проверки (§0.7, каждая обязана была поймать):
  1. `must_understand` в `crates/vibe-registry/**` → **0 совпадений**.
     Контроль: тот же паттерн по `crates/` ловит
     `crates/vibe-index/src/index/quarantine.rs:1` (и ещё ~30 хитов в
     vibe-index/vibe-wire); контроль в самом vibe-registry: `by-name`
     ловит `crates/vibe-registry/tests/index_handshake.rs:36`.
  2. `unavailable` в `schemas/**` → **0**. Контроль: `latest_stable`
     ловит `schemas/index/e1/by_name.jtd.json:55`.
  3. `unavailable` в `formats/vocabularies.json` → **0**. Контроль:
     `"yanked"` ловит `formats/vocabularies.json:451`.
  4. `must_understand` в `schemas/**` → **0** (словарь живёт не там:
     см. §3). Контроль: `version_entry` ловит
     `schemas/index/e1/entry.jtd.json:6`.
  5. `hello` в `campaigns/.../f6-2-quarantine-answer-surface.md` → **0**
     (проверка дельты маршрутов §1). Контроль: `healthz` ловит строку 100
     того же файла.
- Сверка с прежним замером (§1 пакета): прежний считал **15** маршрутов
  (`f6-2-quarantine-answer-surface.md:29`), сегодня дерево даёт **16** —
  добавился `GET /v1/index/hello.json` (`server/mod.rs:61`, рука Р39–Р41
  после того замера: в таблице прежнего замера его нет — п. 5 выше).
  Отвечающих по имени осталось **шесть**, состав тот же.

## 1. Маршруты сервера сегодня: полная таблица

Точное число: `grep -c "\.route(" crates/vibe-index/src/server/mod.rs` →
**16** (строки 56, 57, 61, 62, 66, 70, 74, 78, 82, 87, 91, 95, 99, 103,
105, 107). Прежнее «15» (§1 пакета) — устарело: дерево право (§0.8),
дельта — `hello.json`. Методов-обработчиков 19: у трёх маршрутов по два
метода (`/v1/packages` GET+POST, `…/{group}/{name}` GET+DELETE,
`…/{version}` GET+DELETE).

| путь + метод | обработчик (`файл:строка`) | отвечает по ИМЕНИ? | тип ответа | что отдаёт, когда ничего не найдено |
|---|---|---|---|---|
| GET `/healthz` | `routes/health.rs:16` | нет | рукописный `Health` (`health.rs:10-14`) | всегда 200 |
| GET `/readyz` | `routes/health.rs:25` | нет | рукописный `Health` | всегда 200 |
| GET `/v1/index/hello.json` | `routes/index_files.rs:21` | нет | сырые байты файла (пишет `memory.rs:416-432`) | 404 problem-details (`index_files.rs:131-138`) |
| GET `/v1/index/repomd.json` | `routes/index_files.rs:26` | нет | сырые байты | 404 problem-details |
| GET `/v1/index/primary.jsonl` | `routes/index_files.rs:31` | нет | сырые байты | 404 problem-details |
| GET `/v1/index/primary.jsonl.gz` | `routes/index_files.rs:40` | нет | сырые байты (gzip) | 404 (`index_files.rs:45-52`) |
| GET `/v1/index/by-name/{name}` | `routes/index_files.rs:112` | **ДА** (имя в пути; сырой файл) | сырые байты `by-name/<name>.json` | 404 (нет файла / не тот суффикс, `index_files.rs:117-123`) |
| GET `/v1/index/by-cap/{slug}` | `routes/index_files.rs:80` | нет (capability-слуг) | сырые байты | 404 |
| GET `/v1/index/by-purl/{slug}` | `routes/index_files.rs:96` | нет (purl-слуг) | сырые байты | 404 |
| GET+POST `/v1/packages` | GET `routes/packages.rs:96`, POST `routes/packages.rs:236` | **ДА** при `?q=` (запрос→имена, `packages.rs:105-127`); list-режим — перечисление | рукописные `ListResponse`/`SearchResponse` (`packages.rs:37-77`) | list: 200 `packages:[]`; search: 200 `hits:[]` |
| GET+DELETE `/v1/packages/{group}/{name}` | GET `routes/packages.rs:166`, DELETE `routes/packages.rs:351` | **ДА** | рукописный конверт `PackageVersionsResponse` (`packages.rs:186-194`) с генерированными `VersionEntry` внутри | GET: 404 problem-details (`packages.rs:175`); DELETE отсутствующего: 200 `removed:false` (`packages.rs:374-380`) |
| GET+DELETE `/v1/packages/{group}/{name}/{version}` | GET `routes/packages.rs:196`, DELETE `routes/packages.rs:312` | **ДА** | **генерированный** `VersionEntry` напрямую (`Json<VersionEntry>`, `packages.rs:199`; тип из `vibe_wire::generated::shared`, `types/entry/mod.rs:37`) | GET: 404 problem-details (`packages.rs:211-215`) — карантинная версия даёт ТОТ ЖЕ 404, что и несуществующая (`packages.rs:209-210`) |
| GET `/v1/capabilities/{capability}` | `routes/capabilities.rs:32` | **ДА** (capability→имена) | рукописный `Response`/`Hit` (`capabilities.rs:15-30`) | 200 `hit_count:0` |
| GET `/v1/purls/{purl}` | `routes/purls.rs:32` | **ДА** (purl→имена) | рукописный `Response`/`Hit` (`purls.rs:15-30`) | 200 `hit_count:0` |
| GET `/v1/admin/status` | `routes/admin.rs:25` | нет | рукописный `Status` (`admin.rs:11-23`) | всегда 200 |
| GET `/metrics` | `routes/metrics.rs:11` | нет | prometheus-текст (`server/metrics.rs` render) | всегда 200 |

**Отвечающих по имени — шесть** (по правилу пакета: вход содержит имя
или запрос, разрешаемый в имена; перечисления и мутации не в счёт):
`by-name/{name}`, `GET /v1/packages?q=`, `GET /v1/packages/{g}/{n}`,
`GET /v1/packages/{g}/{n}/{v}`, `GET /v1/capabilities/{cap}`,
`GET /v1/purls/{purl}`. Состав совпадает с прежним замером — дельта
маршрутов (hello.json) в это число не входит.

Пограничные случаи, меняющие число при другом правиле счёта:
- исключить сырые файловые маршруты (не вычисляют ответ) → **5**;
- считать `by-cap/{slug}`/`by-purl/{slug}` именами-разрешимыми входами
  (слуг разрешается в строки с именами) → **8**;
- считать пары (путь, метод) → отвечающих GET-обработчиков те же 6,
  всего обработчиков 19;
- считать POST/DELETE именными (они адресуют пакет) → 7+, но мутации
  исключены правилом пакета.

## 2. Что уже фильтруется, а что нет — после Ф6.2a

Дом предиката — `crates/vibe-index/src/index/quarantine.rs`:
`UNDERSTOOD = &[]` (:27), `missing_capabilities` (:42-48), `is_usable`
(:55-57), `usable_versions` (:63-65), `usable_latest_stable` (:71-76),
`usable_entries` (:80-82), `usable_version_count` (:85-87).

По маршрутам из §1:

| маршрут | спрашивает `usable_*`? | где |
|---|---|---|
| GET `/v1/packages` (list) | да | `packages.rs:93` (kind), `:138` (фильтр kind), `:144` (latest), `:145`, `:146`, `:148` |
| GET `/v1/packages` (search) | да | через `search::search`: `search.rs:76`, `:78` (usable_versions), `search.rs:101` (usable_latest_stable) |
| GET `/v1/packages/{g}/{n}` | да | `packages.rs:181` (usable_latest_stable), `:182` (usable_versions) |
| GET `/v1/packages/{g}/{n}/{v}` | да | `packages.rs:209` (usable_versions) |
| GET `/v1/capabilities/{cap}` | да | через `search::lookup_capability`: `search.rs:123` |
| GET `/v1/purls/{purl}` | да | через `search::lookup_purl`: `search.rs:152` |
| 7 сырых маршрутов (`hello/repomd/primary/.gz/by-name/by-cap/by-purl`) | **не могут по построению** | обработчик читает байты файла с диска (`index_files.rs:129`) и не имеет индекса в руках; фильтрация равна переписыванию файла — см. §3 |
| `/healthz`, `/readyz` | не могут по построению | не несут версионных данных (`health.rs:10-14`) |
| `/v1/admin/status` | нет | `admin.rs:35-36` — счётчики `package_count()`/`version_count()` (писательские, `memory.rs:181-194`, док :185-188: «число писателя, едет в repomd»); поменять = отдельное решение, развилка 5 в §7 |
| `/metrics` | нет | `routes/metrics.rs:14-18` — те же писательские счётчики |
| POST/DELETE (мутации) | нет — намеренно | путь писателя: `packages.rs:270-272` (проверка наличия по сырому `p.versions`), док `quarantine.rs:59-62` |

**Ключевой вопрос блока — ответ: ДА, серверные ответы фильтруют
карантинную версию СЕГОДНЯ, после Ф6.2a, несмотря на пустой носитель
`Index.quarantined`.** Цепочка по чтению:

1. Серверный индекс — проекция журнала: `cli/serve.rs:79` →
   `boot_index` (`serve.rs:153-176`): `replay` (:155) + `project`
   (:165). `load_from` на этом пути нет, а `quarantined` заполняет
   ТОЛЬКО `load_from` (`memory.rs:350`, `:376-381`) → на сервере
   `Index.quarantined` пуст. Носитель действительно пуст.
2. Но проекция сохраняет `must_understand` в самих записях:
   `Event::Published { entry }` → `idx.upsert(*entry)`
   (`journal/project.rs:95-96`); `upsert` кладёт entry целиком, ничего
   не стрипая (`memory.rs:138-139`); `finalise` пересчитывает только
   `latest_stable` (`vibe-wire/src/behaviour/records.rs:90-98`) и полей
   записей не трогает.
3. Предикат читает запись, а не носитель: `usable_versions` итерирует
   `pkg.versions` с фильтром `is_usable(v)` (`quarantine.rs:63-65`);
   `is_usable` = `missing_capabilities(&entry.must_understand).is_empty()`
   (`quarantine.rs:55-57`); `missing_capabilities` сравнивает список
   САМОЙ записи с `UNDERSTOOD` (`quarantine.rs:42-48`), сегодня пустым
   (:27) — любая непустая `must_understand` непригодна.

Итого: событие журнала с `must_understand: ["x"]` доезжает до
`by_pkgref` серверного индекса, и все шесть отвечающих поверхностей
(кроме сырой `by-name/{name}`, §3) его скрывают. Прогоном не
проверялось (запрещено пакетом §0.5); чтение-подкрепление — юнит-тесты
предиката `quarantine.rs:163-227` и страж Ф6.2a на CLI-пути
`tests/cli_read.rs:443-461` (`found:false`, `versions:[]`).

## 3. Сырые байтовые маршруты: чем они отличаются и что это значит

Все пять отдают байты файла с диска ДОСЛОВНО через `serve_file`
(`index_files.rs:128-163`, чтение `tokio::fs::read` :129) — сервер не
парсит и не фильтрует содержимое; контракт «на диске = в ответе»
заявлен докой модуля (`index_files.rs:1-3`).

- `GET /v1/index/by-name/{name}` (`index_files.rs:112-126`): отдаёт запись
  с непустым `must_understand` ДОСЛОВНО — файл пишет целый
  `PackageEntry` со всеми версиями (`memory.rs:252-258`), а
  `must_understand` сериализуется как обычное опциональное поле
  (`vibe-wire/src/generated/shared/mod.rs:319-320`,
  `skip_serializing_if = "Vec::is_empty"` — непустое всегда в файле).
- `GET /v1/index/primary.jsonl` / `.gz` (`index_files.rs:31`, `:40`):
  то же — писатель берёт `iter_versions()`, ВСЕ версии, карантин
  включён (`memory.rs:232`; док :196-203 «писательские проекции обязаны
  нести всё, что держит журнал»).
- `GET /v1/index/by-cap/{slug}` / `by-purl/{slug}` (`index_files.rs:80`,
  `:96`): инвертированные виды строятся из того же `iter_versions()`
  (`memory.rs:280`) — карантинные записи едут дословно.
- `hello.json`/`repomd.json` версионных записей не несят (рукопожатие —
  проекция реестра форматов, `memory.rs:416-432`; repomd — счётчики и
  хэши файлов, `memory.rs:308-318`).

Агрегат `latest_stable` в файле каталога. Золотой корпус
`formats/corpora/index/e1/by-name/wal.json`, пакет `org.vibevm`:

- дословно, строка 198: `      "latest_stable": "0.2.0"`
- `must_understand` несёт версия `0.1.0` (строка 65:
  `          "version": "0.1.0",`), дословно строки 116-118:
  `          "must_understand": [`
  `            "org.vibevm/wal/tombstone@1"`
  `          ],`

В ЭТОМ корпусе `latest_stable` называет пригодную версию (0.2.0), а
непригодная (0.1.0) — старейшая; слепота агрегата здесь не видна, но
она по построению: `finalise` фильтрует только по `pre`
(`records.rs:90-98`), никаких знаний о возможностях читателя у файла
нет. Патологический случай (новейшая стабильная — карантинная)
закреплён юнит-тестом `quarantine.rs:183-202`: хранимое
`latest_stable = Some("0.2.0")` при `usable_latest_stable = "0.1.0"`.
У пакета `com.example` корпуса — `latest_stable: "1.0.0"` (строка 46),
без `must_understand`.

Может ли маршрут нести `unavailable`: **внутри описанной формы — нет**.
Формат файла — схема `schemas/index/e1/by_name.jtd.json` (корневой тип
`NameEntry`; `latest_stable` — её optionalProperties :55-61), записи
версий — общий словарь `version_entry`, определённый в
`formats/vocabularies.json:288` (`must_understand` — описанное поле
словаря, :443-449; вход.jtd.json — корень, именующий словарь, :6).
Поля `unavailable` нет ни в `schemas/**`, ни в словаре (пустые грепы 2-4
из §0 с пойманными контролями). Внести его в файл = смена схемы+словаря
(с последующей регенерацией и правкой золотого корпуса); внести его как
неизвестное поле = нарушить байт-паритет «на диске = в ответе» для
всех строгих читателей, хотя JTD-сгенерированные читатели незнакомое
поле молча проглотят (`types/entry/mod.rs:18-21`).

## 4. Клиент: что он читает и что сделает с новым полем

Каталог клиента: `crates/vibe-registry/src/index_client/{mod,auth,handshake,wire,tests}.rs`.

**Какие ответы читает (маршруты и типы):**

| маршрут | метод клиента (`mod.rs`) | тип разбора (`wire.rs`) |
|---|---|---|
| `hello.json` (зонд, оба кандидата `<base>/v1/index` и `<base>`) | `probe` `index_client/mod.rs:159-233` (через `handshake::probe_candidate`) | генерированный `Handshake` (не wire.rs) |
| `repomd.json` (зонд без рукопожатия) | `probe` `mod.rs:198-229` | только статус 200, тело не парсится |
| `by-name/<name>.json` | `list_versions` `mod.rs:288-329`; `name_candidates` `mod.rs:343-375` | `NameEntryView`/`PackageEntryView`/`VersionEntryView` (`wire.rs:17-33`) |
| `/v1/purls/{purl}` | `lookup_purl` `mod.rs:391-433` | `PurlLookupResults`/`PurlLookupHit` (`wire.rs:71-89`) |
| `/v1/packages?q=` | `search` `mod.rs:449-490` | `SearchResults`/`SearchHit` (`wire.rs:44-67`) |

- **Читает ли `must_understand`** — НЕТ, нигде: греп по
  `crates/vibe-registry/**` → 0 (контроль §0.1). `VersionEntryView`
  имеет единственное поле `version` (`wire.rs:30-33`).
- **Незнакомое поле `unavailable`** — клиент терпим: ни одна view-структура
  не несёт `#[serde(deny_unknown_fields)]` (`wire.rs:17`, `:23`, `:30`,
  `:44`, `:55`, `:71`, `:83`; генерированные — тоже,
  `vibe-wire/src/generated/shared/mod.rs:253`). Это заявлено дважды
  докой: «Only the fields the resolver's version selector needs are
  read; the rest of the on-disk shape is tolerated» (`wire.rs:14-17`) и
  «Extra fields on the wire (today: `command`) are tolerated silently»
  (`wire.rs:36-39`). Новое поле в любом ответе будет молча игнорировано.
- **Кандидаты резолвера**: `list_versions` возвращает ВСЕ версии
  подходящей группы из by-name файла, без какого-либо знания о
  карантине (`mod.rs:326-328` — map `v.version`, сортировка). Резолвер
  `GitPerPackageRegistry::resolve` берёт из этого списка:
  `Latest` → новейшая без `pre` (`git_package_registry/lookup.rs:209`),
  `Req` → новейшая по требованию (`lookup.rs:210-215`). Следствие:
  карантинная версия, записанная в by-name файле, ПОПАДАЕТ в кандидаты
  и может быть выбрана как latest — фильтру на клиенте просто нечего
  спросить (поля не читает).
- **`latest_stable`**: клиент читает его ровно в одном месте —
  `SearchHit.latest_stable` (`wire.rs:59-60`) из структурного поиска, и
  сервер вычисляет его уже фильтрованным (`search.rs:101`,
  `usable_latest_stable`). Агрегат `latest_stable` из САМОГО by-name
  файла клиент НЕ читает вовсе — у `VersionEntryView` нет такого поля
  (`wire.rs:30-33`), «latest» резолвер выводит сам из списка
  (`lookup.rs:209`).

**Полный перечень тестов клиента, утверждающих форму ответа:**
- `src/index_client/wire.rs`: `search_results_decode_minimal_envelope`
  (:115), `search_hit_tolerates_missing_optional_fields` (:150),
  `purl_lookup_results_decode_full_envelope` (:165),
  `binding_site_display_renders_lowercase_word` (:195),
  `name_entry_view_extracts_candidate_groups` (:201).
- `src/index_client/tests.rs` (конструкция/auth, НЕ форма ответа):
  :13, :22, :30, :39, :71, :99.
- `tests/index_search.rs`: :122, :177, :201, :220, :240, :289, :309.
- `tests/index_fast_path.rs`: :194, :237, :275, :290, :301.
- `tests/index_handshake.rs`: :225, :273, :303, :353, :399, :432, :460.
- `tests/index_auth.rs`: :163, :181, :196, :216, :255, :277, :293, :312.
(`tests/registry_cells_oracle.rs` — локальный реестр, index-клиента не
касается: `:44-69` ходит через `LocalRegistry`.)

## 5. Как построить серверный тест, который ВИДИТ карантинную запись

- **Как `server_e2e.rs` строит состояние сегодня**: функция
  `populated_state()` (`tests/server_e2e.rs:77-134`) — чисто RAM:
  `Index::new` (:79-84), три `upsert` (:85-108), ручной push пакета
  `rust` (:117-129), затем `idx.write_to(tmp, WriteCtx)` (:131) —
  пишет КАТАЛОГ (`by-name/`, `primary.jsonl`, `repomd.json`,
  `hello.json` — `memory.rs:214-327`), **журнал не пишет вообще**: ни
  `append`, ни `replay`, ни `project` в файле нет. Состояние:
  `AppState::new(tmp, /*read_only=*/ true, idx)` (:132) — индекс
  передан напрямую, бут-путь `boot_index` не задействован.
- **Пишет ли журнал** — нет (выше). Каким событием пишут другие тесты:
  `server_writes.rs:88-102` `seed_initialised` — `append(&default_dir(dir),
  &JournalRecord { at, actor: default_generator(), event:
  Event::Initialised { registry, registry_url, naming } })`;
  `Published`-записи пишет прод-код мутаций (`packages.rs:265-267`,
  `:421-427`).
- **ТОЧНОЕ изменение фикстуры** (поле, тип, координата) — два уровня:
  - (A) минимальный, в духе нынешнего `server_e2e`: в конструкторе
    `entry(...)` поле `must_understand: vec![]`
    (`server_e2e.rs:69`; тип `Vec<String>` —
    `vibe-wire/src/generated/shared/mod.rs:319-320`) заменить для одной
    из версий на `vec!["some-future-capability".into()]`. Тот же файл
    фикстуры в `server_writes.rs:56`. `AppState::new`
    (`server/state.rs:66-68`) поднимет этот индекс, и все `usable_*`
    сайты ответов (§2) её скроют — запись видна как отсутствие.
  - (B) бут-истинный (сервер реально поднимается ИЗ ЖУРНАЛА, как в
    проде): сидировать журнал по образцу `server_writes.rs:88-102`
    (`Initialised`) плюс добавить запись
    `event: Event::Published { entry: Box<VersionEntry> }`, где у entry
    непустое `must_understand` (в фикстуре `server_writes.rs` поле
    ставится в `vec![]` на :56), затем индекс поднять через
    `serve::boot_index(tmp.path())` (`cli/serve.rs:153-176`; живой
    образец вызова — `server_writes.rs:469`, тест
    `server_boots_from_journal_and_serves_reads` :450) и отдать его в
    `AppState::new`. Проекция `must_understand` сохранит
    (`project.rs:95-96` → `memory.rs:138-139`), а `quarantined`
    останется ПУСТ (его заполняет только `load_from`,
    `memory.rs:376-381`) — тест увидит ровно рабочий факт: фильтрация
    живёт в предикате записи, не в носителе.
- **AppState без биндинга порта — способ ЕСТЬ**: `build_app(state)` +
  `tower::util::ServiceExt::oneshot` — импорт
  `server_e2e.rs:11`, первый вызов :156-158, декларация в доке :1-3
  («no actual TCP listener bound»). Сам `AppState` порта не знает
  вовсе (`state.rs:66`). Пункта «способа нет» не потребовалось.

## 6. Что покраснеет

Форма ответа сегодня закреплена тестами ниже; фраза «что держит» у
каждого. Деление: (а) краснеют при добавлении поля `unavailable`;
(б) краснеют только при смене кода/статуса ответа; (в) не краснеют.

**(а) — только если `unavailable` станет полем ГЕНЕРИРОВАННОГО типа
(`VersionEntry`/`PackageEntry`/`NameEntry`):**
- `tests/wire_parity_entry.rs:145`
  `fully_populated_entry_round_trips_through_the_generated_type` —
  держит ровно 33 ключа записи (`FULLY_POPULATED_KEY_COUNT`, :39;
  assert :150-154).
- `tests/wire_parity_by_name.rs:176`
  `fully_populated_name_entry_round_trips…` — держит 4/5/33 ключа
  (константы :47-49, asserts :180-195).
- `tests/wire_parity_journal.rs` — держит 33 ключа `Published.entry`
  (`PUBLISHED_ENTRY_KEY_COUNT` :62; must_understand в фикстуре :189).
- `src/types/entry/tests.rs:166-189` — держит правило «пустое поле
  невидимо в JSON» (:173); покраснеет, только если поле перестанет
  быть skip-if-empty.
Если же `unavailable` — поле ОТВЕТНОГО КОНВЕРТА (рукописные структуры
сервера), пункт (а) ПУСТ: серверные ассерты читают тело как
`serde_json::Value` по ключам (аддитивно-терпимы), клиентские view
игнорируют незнакомое поле (§4).

**(б) — краснеют при смене кода/статуса ответа:**
- `tests/cli_read.rs:443` `get_stays_silent_about_a_quarantined_version`
  — держит МОЛЧАНИЕ: `found:false`, `versions:[]` для полностью
  карантинного имени (:460-461), патч каталога :412-430. Покраснеет в
  момент, когда ответ заговорит `unavailable` (прямая цель Ф6.2c).
- `tests/server_e2e.rs:371` `single_version_404_for_missing_version` —
  держит 404; сегодня карантинная версия даёт тот же 404 через фильтр
  (`packages.rs:209-215`) — покраснеет, если статус/тело для
  карантинной версии станут иными.
- `tests/server_e2e.rs:410` `admin_status_returns_counts` — держит
  `version_count: 4` — неотфильтрованный писательский счётчик
  (`admin.rs:36` ← `memory.rs:189`); краснеет при переводе счётчиков на
  `usable_version_count`.
- `tests/server_e2e.rs:426` `metrics_route_emits_prometheus_lines` —
  держит `vibe_index_versions_total 4` (`routes/metrics.rs:16-17`) —
  то же условие.
- `tests/server_e2e.rs:314` `packages_list_returns_sorted_envelope` —
  держит `package_count: 3`; краснеет, если список начнёт исключать
  строки полностью карантинных пакетов.
- `tests/server_writes.rs:148`, `:163` — держат конверты/статусы upsert
  (201/200); `:328`, `:359`, `:388` — держат форму журнальных записей
  после мутаций.

**(в) — не краснеют (аддитивная толерантность):** все клиентские из
перечня §4 (незнакомое поле игнорируется, `wire.rs:14-17`, `:36-39`) и
остальные серверные `server_e2e.rs`: :155, :166, :176, :191, :207,
:223, :256, :281 (держит `packages.len()==1`, `versions.len()==2` —
краснеет только при изменении ФИКСТУРЫ, не формы), :299, :328, :343,
:382, :395, :438.

## 7. Развилки для босса

1. **Входит ли сырой `by-name/{name}` в обязательство «шесть
   поверхностей отвечают `unavailable`»?**
   - Сторона «не входит»: обязательство — о вычисляющих ответ маршрутах;
     сырой маршрут отдаёт файл (§3), карантинная запись едет дословно.
     Цена: 0 строк кода, НО клиент продолжает ВИДЕТЬ карантинную версию
     в списке и может выбрать её резолвером (`lookup.rs:209` из
     неотфильтрованного `mod.rs:326-328`) — молчание сохраняется именно
     там, где его слышит резолвер.
   - Сторона «входит»: исполнимо только вне тела файла. Варианты и цены:
     (i) заголовок ответа (напр. `x-vibe-unavailable: <version>`) —
     правка `index_files.rs:112-126` + новый ассерт в
     `server_e2e.rs:281`; (ii) отдельный маршрут-спутник — новый
     обработчик + строка в `server/mod.rs` (счёт 16→17), схема не
     трогается; (iii) исключить версию из файла — это смена ФОРМАТА:
     `schemas/index/e1/by_name.jtd.json` + словарь
     `formats/vocabularies.json` + писатель `memory.rs:252-258` +
     parity-тесты §6а + золотой корпус. Чьё обязательство — решает босс.
2. **`latest_stable` в файле capability-слеп.** Расхождение с посылкой
   пакета (§0.8, дерево право): клиент НЕ читает агрегат `latest_stable`
   из by-name файла сырым — у view нет такого поля (`wire.rs:30-33`);
   единственное чтение `latest_stable` — из структурного поиска, где он
   уже фильтрован сервером (`search.rs:101`). Хуже другое: «latest»
   клиент ВЫВОДИТ сам из полного списка версий (`lookup.rs:209`) —
   слепота файла превращается в слепость выбора. Стороны: (A) это
   свойство ФАЙЛА (агрегат пишется проекцией для всех читателей,
   `records.rs:90-98`; лечится не ответом, а потребителем — считать
   самому, как `usable_latest_stable` `quarantine.rs:71-76`); (B) это
   свойство ОТВETA — тогда сырой маршрут обязан нести исправленный
   агрегат, что невозможно без смены формата (см. развилку 1(iii)).
   Цены: (A) — клиентская правка `mod.rs:326-328` + тесты
   `index_fast_path.rs:194`; (B) — схема+словарь+писатель+corpus+parity.
3. **Должен ли `index_client` научиться читать `unavailable` в этом
   шаге?**
   - Да, вместе с сервером: правка `wire.rs` (поле в view) + фильтр в
     `list_versions` (`mod.rs:326-328`) + новые строки в
     `tests/index_fast_path.rs`; клиент перестаёт видеть непригодные
     версии в кандидатах.
   - Нет, отдельный шаг с триггером: сервер говорит `unavailable`, а
     клиент его молча игнорирует (§4) — ничего не краснеет, но
     резолвер продолжает выбирать карантинную версию; триггер — первое
     реальное использование поля потребителем.
4. **Где живёт `unavailable` в структурных ответах?** Внутри
   генерированного `VersionEntry` (тогда краснеют §6а, а док
   `quarantine.rs:50-54` «никогда не хранится на проводе» нарушается
   схемой) ИЛИ отдельная обёртка ответа вокруг entry (правка сигнатуры
   `packages.rs:196-218`, конверты :176-194/:60-77, ассерты
   `server_e2e.rs:357`/`:343`, клиентские view §4).
5. **Операционные счётчики** (`/v1/admin/status` `admin.rs:35-36`,
   `/metrics` `routes/metrics.rs:14-18`): оставить писательские
   «всё, что держит журнал» (`memory.rs:185-194`) или перевести на
   `usable_version_count` (`quarantine.rs:85-87`, сервером сегодня не
   используется нигде). Цена перевода — краснеют `server_e2e.rs:410`,
   `:426` (§6б).
6. **Статус одинокого `unavailable`-ответа**: сегодня карантинная
   версия неотличима от отсутствующей — тот же 404
   (`packages.rs:209-215`). Говорить `unavailable` = менять контракт
   «не найдено» одного маршрута (`{g}/{n}/{v}`) или всех шести сразу;
   держатель молчания — `cli_read.rs:443` (§6б), держатель 404 —
   `server_e2e.rs:371`.
