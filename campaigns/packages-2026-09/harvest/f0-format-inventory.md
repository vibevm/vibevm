# Ф0.3 — инвентарь форматов (находка спайка)

**Что это.** Шаг Ф0.3 плана TZ-CHANGE-NATIVE-FORMATS-v0.1: список поверхностей для
`formats/REGISTRY.toml` (PROP-044 §4.1, §6.5). Спайк: дерево не тронуто, файл реестра
НЕ создан — здесь только данные для него. Дата: 2026-08-14.

Периметр, источник решений о прочерках (`recoverable`, `foreign_parsers`) —
PROP-044 §5 `##POLICY-IS-COMPUTED` и §3 (`TRUTH-KERNEL`, `DERIVED-IS-DISPOSABLE`),
с уточнениями плана (D8, D10, Б.1, Б.4). Каждое значение несёт обоснование и evidence
`file:line`. «Нет в дереве» — законный результат.

## 1. Сводная таблица

Сокращения в столбце «схема сейчас»: `JTD-файл` — есть `*.jtd.json`; `JSON-Schema-файл` —
есть `*.json` формата JSON Schema; `в коде` — тип/схема описаны Rust-структурой или
инлайн-литералом, файла схемы нет; `нет` — схемы не существует (B10); `план` — путь,
назначенный планом, файла ещё нет.

| id-кандидат | что это | кто пишет | кто читает | recoverable | foreign_parsers | схема сейчас | корпус сейчас | evidence |
|---|---|---|---|---|---|---|---|---|
| cli-init-report | `vibe init --json` | init/helpers.rs | скрипты/агенты (stdout) | да | many | JTD-файл | нет | schemas/init_report.jtd.json; generated/init_report/mod.rs:8 |
| cli-install-plan | `vibe install --plan --json` | install/report.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/install_plan.jtd.json; generated/install_plan/mod.rs:10 |
| cli-install-report | `vibe install --json` | install/report.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/install_report.jtd.json; generated/install_report/mod.rs:8 |
| cli-list-report | `vibe list --json` | list.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/list_report.jtd.json; generated/list_report/mod.rs:8 |
| cli-registry-publish-report | `vibe registry publish --json` | registry/publish.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/registry_publish_report.jtd.json; generated/registry_publish_report/mod.rs:8 |
| cli-registry-sync-report | `vibe registry sync --json` | registry/sync.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/registry_sync_report.jtd.json; generated/registry_sync_report/mod.rs:9 |
| cli-uninstall-report | `vibe uninstall --json` | uninstall.rs | скрипты/агенты | да | many | JTD-файл | нет | schemas/uninstall_report.jtd.json; generated/uninstall_report/mod.rs:8 |
| cli-package-tree | `vibe tree --json` | tree/mod.rs | скрипты/агенты | да | many | JSON-Schema-файл (НЕ JTD) | нет | crates/vibe-cli/resources/package-tree.schema.v1.json:1-2; tree/model.rs:4 |
| index-entry | запись VersionEntry (внутри primary + by-name) | index (memory.rs:180,206) | index_client + чужие | да | many | нет (план) | нет (план) | crates/vibe-index/src/types/entry/mod.rs:43 |
| index-repomd | `repomd.json` (манифест индекса) | memory.rs:255 → repomd.rs:27 | repomd.rs:32; клиент | да | many | нет (план) | нет (план) | crates/vibe-index/src/index/repomd.rs:14; types/repomd.rs:20 |
| index-primary | `primary.jsonl` + `.gz` | memory.rs:180 → primary.rs:43 | primary.rs:75; .gz — чужие | да | many | нет (переиспользует index-entry) | нет (план) | crates/vibe-index/src/index/primary.rs:20-21 |
| index-by-name | `by-name/<name>.json` | memory.rs:206 → by_name.rs:46 | by_name.rs:57; index_client/wire.rs:17 | да | many | нет (план) | нет (план) | crates/vibe-index/src/index/by_name.rs:24; types/entry/aggregate.rs:69 |
| index-by-cap | `by-cap/<slug>.jsonl` | memory.rs:221 → inverted.rs:200 | чужие (HTTP GET) | да | many | нет (разрыв плана) | нет | crates/vibe-index/src/index/inverted.rs:35,66 |
| index-by-purl | `by-purl/<slug>.jsonl` | memory.rs:228 → inverted.rs:215 | чужие (HTTP GET) | да | many | нет (разрыв плана) | нет | crates/vibe-index/src/index/inverted.rs:36,75 |
| manifest | `vibe.toml` | автор (Manifest::write document.rs:307) | Manifest::read document.rs:289; агенты | нет | many | в коде (план: след. ТЗ) | нет (план) | crates/vibe-core/src/manifest/document.rs:67,286 |
| lockfile | `vibe.lock` | Lockfile::write lockfile.rs:454 | Lockfile::read lockfile.rs:428 | да | ours | в коде (план: след. ТЗ) | нет | crates/vibe-core/src/manifest/lockfile.rs:79,410 |
| mcp-tools | input-схемы 8 MCP-инструментов | descriptor() в коде | чужие агенты (tools/list) | да | many | в коде (инлайн json!) | нет | crates/vibe-mcp/src/tools.rs:44-55,88 |
| config | `settings.toml` / `settings.local.toml` | vibe-settings persist | loader.rs:238 | нет | none | в коде (KeyMeta-реестр) | нет | crates/vibe-settings/src/loader.rs:29,32; src/schema/registry.rs |
| journal | журнал фактов реестра (NDJSON) | planned (Ф3.1) | наш проектор | нет | ours | план (schemas/journal/e1/) | нет | нет в дереве; форма — TZ Приложение А.2 |
| handshake | `hello.json` | planned (Ф6.1) | любой чужой клиент | да | many | план (schemas/hello/e1/) | нет | нет в дереве; форма — TZ Приложение А.6; PROP-044 §3 `ONE-ETERNAL-FILE` |
| narrow-public-gate | узкая публичная проекция каталога | planned (D12, волна 3) | чужие инструменты | да | many | нет (план) | нет | нет в дереве; PROP-044 §6.1 `FMT-CATALOG` |

Примечание к `index-entry`: это запись `VersionEntry`, физически переносимая — она
является строкой `primary.jsonl` И элементом `versions[]` внутри `by-name/<name>.json`.
Вынесена в отдельную строку, потому что план называет её по имени (`[format.index-entry]`,
Приложение А.1) и даёт ей собственную схему `schemas/index/e1/entry.jtd.json` (Ф4.1).

## 2. По каждому формату — карточка

### cli-init-report
- **что это одной фразой:** machine-JSON-отчёт команды `vibe init --json`.
- **физический носитель:** stdout (поток JSON); тип `InitReport`.
- **писатель:** `crates/vibe-cli/src/commands/init/helpers.rs` (эмитит `InitReport`).
- **читатели:** внешние скрипты/агенты, читающие stdout; тип — `crates/vibe-wire/src/generated/init_report/mod.rs:8`.
- **recoverable:** да — производная поверхность, переэмитится повторным `vibe init --json` из тех же входов (PROP-044 §3 `DERIVED-IS-DISPOSABLE`: «CLI JSON outputs» — derived).
- **foreign_parsers:** many — «scripts and agents parse them» (PROP-044 §6.5 `FMT-UNINVENTORIED`); сырое чтение предполагается (`##RISK-RAW-PARSERS`).
- **схема:** `schemas/init_report.jtd.json` (JTD).
- **корпус:** нет в дереве.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего — уже описана JTD и сгенерирована. Политика строгости: сгенерированный тип permissive (без `deny_unknown_fields`), что есть свойство генератора, а не схемы (`crates/vibe-wire/src/lib.rs:18-43`);Ф4.2 вводит политику по роли.

### cli-install-plan
- **что это одной фразой:** machine-JSON план установки `vibe install --plan --json`.
- **физический носитель:** stdout; тип `InstallPlan`.
- **писатель:** `crates/vibe-cli/src/commands/install/report.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/install_plan/mod.rs:10`.
- **recoverable:** да — derived, переэмитится из того же графа разрешения.
- **foreign_parsers:** many — §6.5 + `##RISK-RAW-PARSERS`.
- **схема:** `schemas/install_plan.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего (описана и сгенерирована); замечание о строгости — как у cli-init-report.

### cli-install-report
- **что это одной фразой:** machine-JSON-отчёт `vibe install --json`.
- **физический носитель:** stdout; тип `InstallReport`.
- **писатель:** `crates/vibe-cli/src/commands/install/report.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/install_report/mod.rs:8`.
- **recoverable:** да — derived.
- **foreign_parsers:** many.
- **схема:** `schemas/install_report.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего.

### cli-list-report
- **что это одной фразой:** machine-JSON-отчёт `vibe list --json`.
- **физический носитель:** stdout; тип `ListReport`.
- **писатель:** `crates/vibe-cli/src/commands/list.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/list_report/mod.rs:8`.
- **recoverable:** да — derived из lockfile/индекса.
- **foreign_parsers:** many.
- **схема:** `schemas/list_report.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего.

### cli-registry-publish-report
- **что это одной фразой:** machine-JSON-отчёт `vibe registry publish --json`.
- **физический носитель:** stdout; тип `RegistryPublishReport`.
- **писатель:** `crates/vibe-cli/src/commands/registry/publish.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/registry_publish_report/mod.rs:8`.
- **recoverable:** да — derived.
- **foreign_parsers:** many.
- **схема:** `schemas/registry_publish_report.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего.

### cli-registry-sync-report
- **что это одной фразой:** machine-JSON-отчёт `vibe registry sync --json`.
- **физический носитель:** stdout; тип `RegistrySyncReport`.
- **писатель:** `crates/vibe-cli/src/commands/registry/sync.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/registry_sync_report/mod.rs:9`.
- **recoverable:** да — derived.
- **foreign_parsers:** many.
- **схема:** `schemas/registry_sync_report.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего.

### cli-uninstall-report
- **что это одной фразой:** machine-JSON-отчёт `vibe uninstall --json`.
- **физический носитель:** stdout; тип `UninstallReport`.
- **писатель:** `crates/vibe-cli/src/commands/uninstall.rs`.
- **читатели:** внешние скрипты/агенты; тип — `generated/uninstall_report/mod.rs:8`.
- **recoverable:** да — derived.
- **foreign_parsers:** many.
- **схема:** `schemas/uninstall_report.jtd.json`.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** ничего.

### cli-package-tree
- **что это одной фразой:** machine-JSON-снимок spec/dependency-дерева проекта `vibe tree --json`.
- **физический носитель:** stdout; тип — рукописная модель в `crates/vibe-cli/src/commands/tree/model.rs`.
- **писатель:** `crates/vibe-cli/src/commands/tree/mod.rs` (эмитит дерево, валидируемое против схемы).
- **читатели:** внешние скрипты/агенты; тест `crates/vibe-cli/tests/tree_json.rs:26` валидирует вывод против схемы.
- **recoverable:** да — derived из `vibe.toml`/boot-lane/дерева пакетов.
- **foreign_parsers:** many.
- **схема:** `crates/vibe-cli/resources/package-tree.schema.v1.json` — **JSON Schema 2020-12, НЕ JTD** (`$schema`: `package-tree.schema.v1.json:2`).
- **корпус:** нет.
- **эпоха:** 1 (`schema_version: const 1`, `package-tree.schema.v1.json:10`).
- **что мешает описать схемой прямо сейчас:** это единственный CLI-формат не на JTD, а на JSON Schema; тип рукописный (`tree/model.rs:4` — «field-for-field» со схемой). Решение для реестра: либо перевести в JTD (привести к общему генератору Ф4.2), либо держать как именованное JSON-Schema-исключение — это решение Ф1.1, не Ф0.3.

### index-entry
- **что это одной фразой:** каноническая запись одной версии пакета (`VersionEntry`), переносимая между файлами каталога.
- **физический носитель:** строка `primary.jsonl` И элемент `versions[]` в `by-name/<name>.json`; тип — `crates/vibe-index/src/types/entry/mod.rs:43`.
- **писатель:** `Index::write_to` — `crates/vibe-index/src/index/memory.rs:180` (primary), `:206` (by-name).
- **читатели:** `primary::read` (`primary.rs:75`), `by_name::read` (`by_name.rs:57`), клиентская проекция `index_client/wire.rs:17` (читает только нужные поля, остальное терпит).
- **recoverable:** да — каталог declared disposable (PROP-044 §6.1 `FMT-CATALOG`, §3 `DERIVED-IS-DISPOSABLE`): снести и перепроектировать из журнала/источников.
- **foreign_parsers:** many — `by-name` тянется клиентом резолвера (`wire.rs:17`), `primary`HTTP-отдаётся; сырое чтение предполагается (`##RISK-RAW-PARSERS`).
- **схема:** нет (B10); план — `schemas/index/e1/entry.jtd.json` (Ф4.1).
- **корпус:** нет (B12); план — `formats/corpora/index/e1` (Ф5.1).
- **эпоха:** 1 (`schema_version: u32 = 1`, `entry/mod.rs:124`).
- **что мешает описать схемой прямо сейчас:** рукописный `#[derive(Serialize, Deserialize)]` (`entry/mod.rs:37`) + `deny_unknown_fields` (`:38`) + 21 политика `skip_serializing_if` в атрибутах, а не в схеме (B2); JTD не выразит `deny` (B11, `vibe-wire/src/lib.rs:18-43`). Снимается генератором Ф4.2 + снятием deny в Ф3.3.

### index-repomd
- **что это одной фразой:** манифест индекса `repomd.json` (по образцу RPM `repomd.xml`); пишется последним для консистентности.
- **физический носитель:** файл `repomd.json` в data-каталоге; тип `Repomd` — `crates/vibe-index/src/types/repomd.rs:20`.
- **писатель:** `Index::write_to` → `repomd::write` (`memory.rs:255` → `repomd.rs:27`); константа имени `repomd.rs:14`.
- **читатели:** `repomd::read` (`repomd.rs:32`) → `Index::load_from` (`memory.rs:263`); клиент индекса читает `repomd` первым.
- **recoverable:** да — derived, перепроектируется.
- **foreign_parsers:** many — это «входная точка» каталога для чужого клиента; сырое чтение предполагается.
- **схема:** нет (B10); план — `schemas/index/e1/repomd.jtd.json` (Ф4.1).
- **корпус:** нет; план — `formats/corpora/index/e1`.
- **эпоха:** 1 (`Repomd::SCHEMA_VERSION = 1`, `repomd.rs:38`).
- **что мешает описать схемой прямо сейчас:** полутегированное объединение `#[serde(untagged)]` `RepomdFileEntry` (`repomd.rs:42`) — у варианта `Directory` есть тег (`kind`, `:44-50`), у `File` его нет; JTD не умеет untagged (B11). Снимается решением D2 (Ф1.5): симметричный тег `kind` на обоих плечах. Плюс `deny_unknown_fields` (`repomd.rs:15`) и недетерминированная запись `generated_at: Utc::now()` (`memory.rs:249`, B6).

### index-primary
- **что это одной фразой:** канонический экспорт всех записей — JSON Lines, по `VersionEntry` на строку, отсортированный.
- **физический носитель:** файлы `primary.jsonl` и `primary.jsonl.gz` (детерминированный gzip); константы `crates/vibe-index/src/index/primary.rs:20-21`.
- **писатель:** `primary::write` (`primary.rs:43`, вызов из `memory.rs:180`).
- **читатели:** `primary::read` (`primary.rs:75`); `.gz` — «bandwidth-conscious consumers» (`primary.rs:5`), т.е. внешние.
- **recoverable:** да — strict canonical export, derived.
- **foreign_parsers:** many — `.gz` явно для внешних потребителей; сырое чтение предполагается.
- **схема:** нет отдельной (план Ф4.1 не упоминает `primary.jtd.json`); строки — это `index-entry` → переиспользует `schemas/index/e1/entry.jtd.json`.
- **корпус:** нет; план — `formats/corpora/index/e1`.
- **эпоха:** 1 (наследует от `VersionEntry`).
- **что мешает описать схемой прямо сейчас:** наследует блокеры `index-entry`; gzip-обёртка детерминирована (`primary.rs:65`, тест `:200`), что отдельно полезно как корпус для wire-diff.

### index-by-name
- **что это одной фразой:** файл кандидатов одного короткого имени `by-name/<name>.json` — все пакеты `(group,name)` с этим `name` и их версии.
- **физический носитель:** файл `<data>/by-name/<name>.json` (pretty-print); тип `NameEntry` — `crates/vibe-index/src/types/entry/aggregate.rs:69`; константа каталога `by_name.rs:24`.
- **писатель:** `by_name::write` (`by_name.rs:46`, вызов из `memory.rs:206`).
- **читатели:** `by_name::read` (`by_name.rs:57`) → `Index::load_from`; клиентская проекция `index_client/wire.rs:17` (`NameEntryView` — терпимая, только нужные поля).
- **recoverable:** да — derived; сегодня это read-for-rewrite ядро (перечитывается ради перезаписи, B3), после Ф3 — проекция из журнала.
- **foreign_parsers:** many — тянутый клиентом файл (`wire.rs:17`), HTTP-отдаётся; сырое чтение предполагается.
- **схема:** нет (B10); план — `schemas/index/e1/by_name.jtd.json` (Ф4.1).
- **корпус:** нет; план — `formats/corpora/index/e1`.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** `deny_unknown_fields` на `NameEntry` (`aggregate.rs:64`) и `PackageEntry` (`:21`); внутри — тот же `VersionEntry` со своими блокерами; `indexed_at` берётся как максимум по кандидатам (`aggregate.rs:92`).

### index-by-cap
- **что это одной фразой:** инвертированный индекс по capability — `by-cap/<slug>.jsonl`, по строке на пакет, предоставляющий capability.
- **физический носитель:** файл `<data>/by-cap/<slug>.jsonl`; строка — `CapabilityRow` (`crates/vibe-index/src/index/inverted.rs:66`); константа `inverted.rs:35`.
- **писатель:** `inverted::write_capability` (`inverted.rs:200`, вызов из `memory.rs:221`).
- **читатели:** внешние потребители через HTTP GET (`inverted.rs:3-4`); своего ридера в дереве нет.
- **recoverable:** да — derived (перестраивается из `VersionEntry` в `InvertedView::from_entries`, `inverted.rs:105`).
- **foreign_parsers:** many — явно «let consumers fetch… with a single HTTP GET» (`inverted.rs:3-4`).
- **схема:** нет; **разрыв плана** — Ф4.1 перечисляет `entry/repomd/by_name/journal`, но не `by-cap` (и не `by-purl`), хотя это публикуемая поверхность со своим типом строки.
- **корпус:** нет.
- **эпоха:** 1 (наследует концептуально; явного поля версии у строки нет).
- **что мешает описать схемой прямо сейчас:** рукописный `#[derive(Serialize, Deserialize)]` `CapabilityRow` (`inverted.rs:65`); тип живёт только в коде; нет своего ридера — нарушает G11 «у каждого публикуемого формата есть generated reader в round-trip-тесте».

### index-by-purl
- **что это одной фразой:** инвертированный индекс по upstream PURL — `by-purl/<slug>.jsonl`.
- **физический носитель:** файл `<data>/by-purl/<slug>.jsonl`; строка — `PurlRow` (`inverted.rs:75`); константа `inverted.rs:36`.
- **писатель:** `inverted::write_purl` (`inverted.rs:215`, вызов из `memory.rs:228`).
- **читатели:** внешние потребители через HTTP GET; своего ридера в дереве нет (есть только клиентская проекция `PurlLookupHit` в `wire.rs:84`, но она читает HTTP-ответ сервера, а не сам файл).
- **recoverable:** да — derived.
- **foreign_parsers:** many.
- **схема:** нет; **разрыв плана** (как у by-cap — Ф4.1 не называет схему).
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** рукописный `PurlRow` (`inverted.rs:74`); дубль `BindingSite` с другой политикой тегирования — писатель `kebab-case` (`inverted.rs:88`), читатель `lowercase` (`wire.rs:93`); совпадают лишь по случайности однословности (B7); нет ридера файла (G11).

### manifest
- **что это одной фразой:** единый манифест проекта `vibe.toml` (один файл, переменные роли по секциям).
- **физический носитель:** файл `vibe.toml` (TOML); тип `Manifest` — `crates/vibe-core/src/manifest/document.rs:67`; имя `Manifest::FILENAME = "vibe.toml"` (`document.rs:286`).
- **писатель:** автор (через `Manifest::write`, `document.rs:307`).
- **читатели:** `Manifest::read` (`document.rs:289`) / `Manifest::parse_str` (`:298`); «агенты читают файлы, а не API» (PROP-044 §6.2 `FMT-MANIFEST`).
- **recoverable:** нет — авторский, resides в чужих репозиториях, невосстановим из истины (PROP-044 §6.2: «authored, unrecoverable, resident in foreign repositories»).
- **foreign_parsers:** many — «an external surface de facto because agents read files, not APIs» (§6.2); сырое чтение предполагается.
- **схема:** в коде — рукописный `Manifest` + `deny_unknown_fields` (`document.rs:66`); формального файла схемы нет. План: это ТЗ намеренно НЕ описывает манифест схемой (только поле `epoch` в Ф1.2/D5), схема — следующее ТЗ.
- **корпус:** нет; план (Б.4) — корпус ридера №0 = три реальных манифеста из `fixtures/`.
- **эпоха:** сегодня поле эпохи отсутствует (B13); Ф1.2 вводит `epoch = 1` (отсутствие = «до эпох», НЕ «эпоха 1»).
- **что мешает описать схемой прямо сейчас:** рукописный `Manifest` + `deny_unknown_fields` (`document.rs:66`); множество вложенных секций с такой же рукописной сериализацией; это намеренно отложено (следующее ТЗ). Сегодня — строгий TOML-парсер без эпохи.

### lockfile
- **что это одной фразой:** локфайл проекта `vibe.lock` — пин установленных пакетов по `(group, name, version, content_hash)`.
- **физический носитель:** файл `vibe.lock` (TOML, `[[package]]` array-of-tables); тип `Lockfile` — `crates/vibe-core/src/manifest/lockfile.rs:79`; имя `Lockfile::FILENAME = "vibe.lock"` (`lockfile.rs:410`).
- **писатель:** `Lockfile::write` (`lockfile.rs:454`); перегенерируется каждым `vibe install`.
- **читатели:** `Lockfile::read` (`lockfile.rs:428`); чужих независимых читателей сегодня нет.
- **recoverable:** да — «cheapest format, becomes free by construction» (PROP-044 §6.3 `FMT-LOCKFILE`); снести и пересобрать `vibe install`.
- **foreign_parsers:** ours — читается только нашим кодом (`Lockfile::read`), чужих независимых читателей нет и не ожидается (committed-файл, но не опубликованная поверхность). Попадает в квадрант «recoverable + one parser» (PROP-044 §5).
- **схема:** в коде — рукописный `Lockfile` + `deny_unknown_fields` (`lockfile.rs:78`); план (Б.6): механика D9 (эпоха + хэш генератора + id рецептов вместо `schema_version`) — следующее ТЗ.
- **корпус:** нет.
- **эпоха:** сегодня — `CURRENT_SCHEMA_VERSION = 5` (`lockfile.rs:50`) с отказом по `!=` (`lockfile.rs:430`, B14); D9 меняет модель на (epoch, generator hash, recipe ids).
- **что мешает описать схемой прямо сейчас:** рукописный `Lockfile`/`LockedPackage`/`LockedSubskill` + deny (`lockfile.rs:78,254,375`); это намеренно отложено (Б.6, следующее ТЗ).

### mcp-tools
- **что это одной фразой:** input-схемы встроенных MCP-инструментов — контракт с чужими агент-рантаймами.
- **физический носитель:** JSON, отдаваемый через MCP `tools/list` (структура `ToolDescriptor { name, description, input_schema }`, `crates/vibe-mcp/src/tools.rs:17`); не файл.
- **писатель:** метод `descriptor()` каждого инструмента, инлайн `json!({...})` в коде.
- **читатели:** чужие агент-рантаймы (Claude, и др.) через MCP-протокол.
- **recoverable:** да — описатели эмитятся из Rust-кода при старте сервера; снести и перезапустить → регенерируется.
- **foreign_parsers:** many — «literally a contract with foreign agents» (PROP-044 §6.5 `FMT-UNINVENTORIED`).
- **схема:** в коде — input_schema собираются инлайн `json!`-литералами (`tools.rs:88,126,213,344,482`; `tools/explain.rs`, `tools/query.rs`, `tools/select.rs`); файла схемы нет.
- **корпус:** нет.
- **эпоха:** 1.
- **что мешает описать схемой прямо сейчас:** схема живёт только в коде (восемь разрозненных инлайн-литералов), нет её единого источника; нет файла, который генерирует `tools/list`. Перечень инструментов: `list_tools`, `query_package`, `read_subskill`, `materialise_subskill`, `agentic_explain`, `explain`, `query`, `select` (`tools.rs:44-55`).

### config
- **что это одной фразой:** пользовательские/репо-настройки vibe (Vibe Tree UI и будущие app-настройки) — НЕ проектный манифест.
- **физический носитель:** файлы `settings.toml` (L1 `~/.vibe/`, L2 `<repo>/.vibe/`) и `settings.local.toml` (L3, gitignored); TOML `toml::Table`. Константы `crates/vibe-settings/src/loader.rs:29,32`.
- **писатель:** слой persist (`crates/vibe-settings/src/persist/`); канонизируется opportunistically.
- **читатели:** `load_layer` (`loader.rs:238`) → резолвер/схема.
- **recoverable:** нет — авторские предпочтения, невосстановимы из истины (PROP-044 §6.4 `FMT-CONFIGS` + §5: «Unrecoverable + one parser → epoch-in-file + codemod (configs)»).
- **foreign_parsers:** none — читаются только нашим кодом; не опубликованная поверхность.
- **схема:** в коде — схема-first `KeyMeta`-реестр (`crates/vibe-settings/src/schema/registry.rs`), применяется как валидация/диагностика ПОСЛЕ сырой загрузки, а не как строгий parse-гейт.
- **корпус:** нет.
- **эпоха:** сегодня формальной эпохи не несёт; D10 — `fn load() -> Config` без `Result` (сегодня `load_layer` возвращает `Result` с parse-ошибкой, `loader.rs:238,251`, но missing = пустая таблица, `:242`).
- **что мешает описать схемой прямо сейчас:** схема — Rust-реестр `KeyMeta`, не файл; три уровня слияния (L1/L2/L3); целевая модель D10 (восстанавливаемая канонизация + warning вместо отказа) ещё не построена.

### journal
- **что это одной фразой:** append-only журнал фактов реестра (publication, yank, rename, …) — часть истины.
- **физический носитель:** NDJSON, шардирование по месяцу (`journal/<YYYY-MM>.ndjson`); запись `{at, actor, event}`.
- **писатель:** нет в дереве — planned Ф3.1.
- **читатели:** наш проектор (closed `Event.kind`, Б.1); клиентам НЕ отдаётся (рулинг Ф3.1: журнал вне клиентской поверхности).
- **recoverable:** нет — это ИСТИНА (PROP-044 §3 `TRUTH-KERNEL`(3)): авторитетное ядро, не производное; удаление = безвозвратная потеря фактов.
- **foreign_parsers:** ours — читается проектором; multi-worker через git-CAS, но всё «наше»; чужим клиентам не отдаётся.
- **схема:** план — `schemas/journal/e1/` (Ф3.1/Ф4.1).
- **корпус:** нет.
- **эпоха:** н/д (planned; форма события — TZ Приложение А.2; будет e1).
- **что мешает описать схемой прямо сейчас:** формат ещё не построен; закрытый enum `Event` (Б.1) проектируется под генератор Ф4.2.

### handshake
- **что это одной фразой:** вечный хэндшейк `hello.json` в корне индекса — вход, через который клиент любого возраста узнаёт миры.
- **физический носитель:** файл `hello.json`; ключи `{vibe, worlds[], min_client?, notice?, successor?}`.
- **писатель:** нет в дереве — planned Ф6.1 (`write_to` пишет в корень данных).
- **читатели:** любой чужой клиент (читает ПЕРЕД `repomd.json`).
- **recoverable:** да — явно назначено планом Ф6.1: «запись в реестре (`recoverable=true`, `foreign_parsers=many`)»; регенерируется из конфигурации миров.
- **foreign_parsers:** many — единственный вечный файл, читается максимально терпимо (PROP-044 §3 `ONE-ETERNAL-FILE`).
- **схема:** план — `schemas/hello/e1/` (Ф6.1).
- **корпус:** нет.
- **эпоха:** н/д (planned; форма — TZ Приложение А.6; будет hello/1).
- **что мешает описать схемой прямо сейчас:** формат ещё не построен; вырожденная первая реализация (один мир).

### narrow-public-gate
- **что это одной фразой:** узкая публичная проекция каталога — name, version, hash+recipe, URL, yanked, tombstone; единственная поверхность, защищаемая перед чужими инструментами.
- **физический носитель:** нет в дереве — новый артефакт (PROP-044 §6.1 `FMT-CATALOG`).
- **писатель:** planned (D12, волна 3).
- **читатели:** чужие инструменты.
- **recoverable:** да — производная проекция.
- **foreign_parsers:** many — «the one surface we actually defend before foreign tools» (§6.1).
- **схема:** нет (plan, D12).
- **корпус:** нет.
- **эпоха:** н/д (planned; не в этом ТЗ).
- **что мешает описать схемой прямо сейчас:** сознательно отложено (D12); черпает поля из `index-entry`, но проекция у́же.

## 3. Три судьбы каталога (PROP-044 §6.1)

Контракт делит поверхности каталога на три группы; конкретные файлы:

**(а) Read-for-rewrite ядро** — то, что сегодня перечитывается ради перезаписи и после
Ф3 вообще перестаёт читаться писателем:
- `by-name/<name>.json` (`by_name.rs:24`) — `Index::load_from` перечитывает все
  кандидаты (`memory.rs:264`), чтобы потом переписать (`memory.rs:206`); один из шести
  RMW-путей B3 (напр. `cli/add.rs:51→122`, `cli/remove.rs:35→57`, `cli/reindex.rs:218→281`,
  серверные `server/routes/packages.rs:256/304/333`).
- `repomd.json` (`repomd.rs:14`) — перечитывается на старте (`memory.rs:263`), версия
  затирается константой писателя (`memory.rs:245`, B5); `generated_at` недетерминирован
  (`memory.rs:249`, B6).
- Внутренний тип `VersionEntry` (`entry/mod.rs:43`) — прочитанное переписывается
  обратно; после Ф3 это запрещается модулем (G4): писатель не принимает wire-прочитанный тип.

**(б) Экспортные файлы** — строгие канонические экспорты; каждый обязан иметь
сгенерированный ридер в round-trip-тесте (G11):
- `primary.jsonl` + `primary.jsonl.gz` (`primary.rs:20-21`) — есть ридер `primary::read`
  (`primary.rs:75`); gzip детерминирован (`primary.rs:65`).
- `by-cap/<slug>.jsonl` (`inverted.rs:35`, строка `CapabilityRow` `:66`) — **ридера
  файла нет** (разрыв G11).
- `by-purl/<slug>.jsonl` (`inverted.rs:36`, строка `PurlRow` `:75`) — **ридера файла нет**
  (разрыв G11); клиентская `PurlLookupHit` (`wire.rs:84`) читает HTTP-ответ сервера, не файл.

**(в) Кандидат в узкие публичные ворота** — новая, малая, медленная проекция:
name, version, hash+recipe, URL, yanked, tombstone (`narrow-public-gate` выше). Её цель —
чтобы всё богаче за ней могло меняться еженедельно (PROP-044 §6.1). Сегодня не существует;
строится в волне 3 (D12).

Замечание: `by-name` сегодня двойствен — это и read-for-rewrite ядро (а), и файл,
который тянет клиент (`wire.rs:17`). После Ф3 read-for-rewrite умирает, и `by-name`
остаётся только экспортной/клиентской поверхностью.

## 4. Черновик `formats/REGISTRY.toml`

Это черновик ВНУТРИ находки; файл в дереве НЕ создаётся. Форма записи — Приложение А.1
плана. Пути схем существующих форматов — фактические; пути для каталога/journal/handshake —
назначенные планом (Ф4.1/Ф3.1/Ф6.1) и помечены `# planned`. Записи planned включены, чтобы
реестр был полон по периметру PROP-044 §6.5;Ф1.1 решает, входят ли они в `enum FormatId`
до постройки.

```toml
# formats/REGISTRY.toml — ЧЕРНОВИК (не создан; данные из находки Ф0.3)
# Графы определены в PROP-044 §5 ##POLICY-IS-COMPUTED.
# recoverable: можно ли восстановить БЕЗ человека (снести и пересобрать из истины).
# foreign_parsers: none | ours | many (сырое чтение ПРЕДПОЛАГАЕТСЯ, ##RISK-RAW-PARSERS).

# ── 7 CLI-отчётов (JTD), PROP-044 §6.5 ──────────────────────────────────────
[format.cli-init-report]
epoch = 1
schema = "schemas/init_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-install-plan]
epoch = 1
schema = "schemas/install_plan.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-install-report]
epoch = 1
schema = "schemas/install_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-list-report]
epoch = 1
schema = "schemas/list_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-registry-publish-report]
epoch = 1
schema = "schemas/registry_publish_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-registry-sync-report]
epoch = 1
schema = "schemas/registry_sync_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.cli-uninstall-report]
epoch = 1
schema = "schemas/uninstall_report.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

# ── 8-й CLI-формат: JSON Schema 2020-12, НЕ JTD ────────────────────────────
[format.cli-package-tree]
epoch = 1
schema = "crates/vibe-cli/resources/package-tree.schema.v1.json"   # JSON Schema, не JTD
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

# ── Каталог (PROP-044 §6.1, три судьбы) ────────────────────────────────────
# Запись VersionEntry — переносится внутри primary + by-name.
[format.index-entry]
epoch = 1
schema = "schemas/index/e1/entry.jtd.json"   # planned (Ф4.1); сегодня нет (B10)
recoverable = true
foreign_parsers = "many"
corpus = "formats/corpora/index/e1"          # planned (Ф5.1); сегодня нет (B12)
sunset = "none"

[format.index-repomd]
epoch = 1
schema = "schemas/index/e1/repomd.jtd.json"  # planned (Ф4.1)
recoverable = true
foreign_parsers = "many"
corpus = "formats/corpora/index/e1"
sunset = "none"

[format.index-primary]
epoch = 1
schema = "schemas/index/e1/entry.jtd.json"   # строки = VersionEntry; отдельной схемы primary нет
recoverable = true
foreign_parsers = "many"
corpus = "formats/corpora/index/e1"
sunset = "none"

[format.index-by-name]
epoch = 1
schema = "schemas/index/e1/by_name.jtd.json" # planned (Ф4.1)
recoverable = true
foreign_parsers = "many"
corpus = "formats/corpora/index/e1"
sunset = "none"

[format.index-by-cap]
epoch = 1
schema = "none"                              # разрыв: Ф4.1 не называет схему by-cap
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

[format.index-by-purl]
epoch = 1
schema = "none"                              # разрыв: Ф4.1 не называет схему by-purl
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

# ── Авторские форматы ──────────────────────────────────────────────────────
[format.manifest]
epoch = 1                                    # поле появилось в Ф1.2/D5; сегодня отсутствует (B13)
schema = "none"                              # намеренно отложено — следующее ТЗ
recoverable = false                          # авторский, невосстановим (PROP-044 §6.2)
foreign_parsers = "many"                     # агенты читают файлы
corpus = "none"                              # корпус ридера №0 planned (Б.4)
sunset = "none"

[format.lockfile]
epoch = 1                                    # сегодня schema_version=5 (B14); D9 меняет модель
schema = "none"                              # отложено — следующее ТЗ (Б.6)
recoverable = true
foreign_parsers = "ours"                     # читаем только мы (Lockfile::read)
corpus = "none"
sunset = "none"

# ── Неинвентаризованные поверхности (PROP-044 §6.5) ────────────────────────
[format.mcp-tools]
epoch = 1
schema = "none"                              # инлайн json! в коде; файла схемы нет
recoverable = true
foreign_parsers = "many"                     # контракт с чужими агентами
corpus = "none"
sunset = "none"

[format.config]
epoch = 1
schema = "none"                              # KeyMeta-реестр в коде, не файл
recoverable = false                          # авторские предпочтения (PROP-044 §6.4)
foreign_parsers = "none"                     # читаем только мы
corpus = "none"
sunset = "none"

# ── Planned (нет в дереве; строятся поздними фазами) ───────────────────────
[format.journal]
epoch = 1                                    # planned (Ф3.1); будет e1
schema = "schemas/journal/e1/journal.jtd.json"   # planned
recoverable = false                          # это ИСТИНА, не производное (PROP-044 §3)
foreign_parsers = "ours"                     # читает проектор; клиентам не отдаётся
corpus = "none"
sunset = "none"

[format.handshake]
epoch = 1                                    # planned (Ф6.1); будет hello/1
schema = "schemas/hello/e1/hello.jtd.json"   # planned
recoverable = true                           # явно назначено Ф6.1
foreign_parsers = "many"                     # вечный файл, читается чужими клиентами
corpus = "none"
sunset = "none"

[format.narrow-public-gate]
epoch = 0                                    # planned (D12, волна 3); эпоха не назначена
schema = "none"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"
```

## 5. Дыры и неожиданности

1. **`package-tree` — это `vibe tree --json`, а НЕ `vibe load --json`.** Периметр задачи
   называет его `vibe load --json`; фактически схема потребляется командой `vibe tree --json`
   (описание схемы `crates/vibe-cli/resources/package-tree.schema.v1.json:5`;
   `crates/vibe-cli/src/commands/tree/model.rs:4`; тест `crates/vibe-cli/tests/tree_json.rs:26`).
   Команды `load` в дереве нет. Это единственный CLI-формат не на JTD (JSON Schema 2020-12).

2. **Отдельного файлового формата манифеста skill/subskill нет.** Периметр просил найти
   «файловый формат манифеста skill'а». Skill декларируется таблицей `[[skill]]` внутри
   `vibe.toml` (`SkillDecl`, `crates/vibe-core/src/manifest/package/skill.rs:57`); subskill —
   подзапись индексного `VersionEntry` (`SubskillEntry`, `types/entry/content.rs:61`) и
   локфайла (`LockedSubskill`, `lockfile.rs:377`). Тела skill'ов — это файлы-источники по
   `decl.path`, не структурированный wire-формат. Таким образом PROP-044 §6.5 «skill/subskill
   files» не имеют отдельной registry-записи: объявление покрыто `manifest`, индексная
   подзапись — `index-entry`, локфайльная — `lockfile`.

3. **`by-cap`/`by-purl` — разрыв с базовой линией B10/Ф4.1.** Ф4.1 перечисляет схемы
   каталога: `entry.jtd.json`, `repomd.jtd.json`, `by_name.jtd.json`, `journal.jtd.json`.
   Но `by-cap/<slug>.jsonl` и `by-purl/<slug>.jsonl` — публикуемые HTTP-поверхности со
   своими типами строк (`CapabilityRow` `inverted.rs:66`, `PurlRow` `inverted.rs:75`),
   причём **без своего ридера файла** (нарушение G11). Им нужны собственные схемы и
   round-trip-тесты; в плане они не упомянуты. Это наиболее значимая дыра инвентаря.

4. **Дубль `BindingSite` с разными правилами тегирования (подтверждение B7).** Писатель
   `by-purl` — `#[serde(rename_all = "kebab-case")]` (`inverted.rs:88`); клиентский ридер —
   `#[serde(rename_all = "lowercase")]` (`wire.rs:93`). Совпадают только потому, что
   значения (`package`, `subskill`) однословные; паритет-теста нет. Ф4.2 закрывает структурно
   (одно определение + реэкспорт).

5. **Схема настроек и схема MCP-инструментов живут в коде, а не файлом.** Для `config`
   это `KeyMeta`-реестр (`crates/vibe-settings/src/schema/registry.rs`), для `mcp-tools` —
   восемь инлайн `json!`-литералов. В обоих случаях «схема» (документ формы) отсутствует как
   файл — это блокер для попадания в реестр «по форме», пока генератор/извлечение не дотянут.

6. **`PackageKind` записан в дереве дважды (подтверждение B7).** Помимо `vibe-core`
   (`crates/vibe-core/src/package_ref/kind.rs:31`) есть копия в индексе
   (`crates/vibe-index/src/types/kinds.rs:21`); держится паритет-тестом. Плюс словарь вида
   дублирован в CLI-схемах и `package-tree.schema.v1.json`. G9 («словарь существует ровно в
   одной схеме») закрывается в Ф4.2.

7. **specmap-схема существует, но вне периметра.** `xtask/src/codegen.rs:63-70` маршрутизирует
   второе семейство JTD-схем — движка трассировки в `packages/org.vibevm.ai-native/.../schemas`.
   Это не host-формат и в реестр host-форматов не входит; отмечено для полноты, чтобы Ф1.1
   случайно не подхватил его.

8. **Расхождение с D10 у config сегодня.** Целевая модель (D10): `fn load() -> Config` без
   `Result`, unknown→warning. Сегодня `load_layer` возвращает `Result` и parse-ошибка
   short-circuit'ит `load_all` (`loader.rs:238,251,536`); зато missing = пустая таблица
   (`:242`). Текущее состояние чуть «строже» целевого; для реестра это отмечено как
   `recoverable=false, foreign_parsers=none` без изменения.

## 6. Как проверить этот инвентарь

Команды воспроизводят ключевые пункты; все пути — от корня рабочего дерева.

- Семь JTD-схем и сгенерированных модулей:
  - `ls schemas/` → семь `*.jtd.json`.
  - `ls crates/vibe-wire/src/generated/` → семь подмодулей + `mod.rs`.
- Восьмой формат (JSON Schema, не JTD), эмиттер — `tree`:
  - `grep -rn "package-tree.schema" crates/ xtask/ tools/` → потребители в `tree/model.rs`, `tests/tree_json.rs`.
- Маршрутизация генератора и строгость сгенерированных типов:
  - `grep -n "fn generated_dir_for" xtask/src/codegen.rs` → одна ветка `match` (B10).
  - `crates/vibe-wire/src/lib.rs:18-43` → note об отсутствии `deny_unknown_fields`.
- 15 мест `deny_unknown_fields` каталога (B1):
  - `grep -rn "deny_unknown_fields" crates/vibe-index/src` → 15 строк (aggregate ×2, content ×5, mod ×1, relations ×6, repomd ×1).
- Полутегированное объединение (B4):
  - `grep -n "untagged" crates/vibe-index/src/types/repomd.rs` → `:42`.
- Файлы, которые пишет каталог, и их имена:
  - `grep -n "pub const FILENAME\|pub const DIRNAME\|pub const BY_" crates/vibe-index/src/index/{primary,repomd,by_name,inverted}.rs`.
- Запись каталога перечитывается ради перезаписи (B3):
  - `grep -n "load_from\|write_to" crates/vibe-index/src/index/memory.rs`.
- Клиентский reader каталога:
  - `grep -n "struct NameEntryView\|rename_all" crates/vibe-registry/src/index_client/wire.rs`.
- Дубль `BindingSite` (B7):
  - `grep -rn "enum BindingSite" crates/vibe-index/src crates/vibe-registry/src` → писатель `inverted.rs:88` (kebab), ридер `wire.rs:93` (lowercase).
- Манифест/локфайл:
  - `grep -n "pub struct Manifest\|deny_unknown_fields\|FILENAME" crates/vibe-core/src/manifest/document.rs` (`:67, :66, :286`).
  - `grep -n "CURRENT_SCHEMA_VERSION\|!= \|UnsupportedLockfile" crates/vibe-core/src/manifest/lockfile.rs` (`:50`, отказ `:430`).
- Восемь MCP-инструментов и что схема инлайн в коде:
  - `grep -n "Box::new(\|name:\|input_schema" crates/vibe-mcp/src/tools.rs`.
- Skill — нет отдельного файла манифеста:
  - `grep -rn "pub struct SkillDecl" crates/vibe-core/src/manifest/` → `package/skill.rs:57` (внутри `vibe.toml`).
- Настройки — схема в коде, TOML:
  - `grep -n "const SETTINGS_FILE\|const LOCAL_SETTINGS_FILE\|toml::Table" crates/vibe-settings/src/loader.rs`.
  - `ls crates/vibe-settings/src/schema/` → Rust-реестр, не файл схемы.
