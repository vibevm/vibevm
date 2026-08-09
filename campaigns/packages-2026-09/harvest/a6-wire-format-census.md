# WIRE-CENSUS — каталог пакетов vibevm (формат «на проводе»)

Замер, а не правка. Цель — исчерпывающий и точный список того, что сегодня
выходит на провод в опубликованном каталоге пакетов (`repomd.json`,
`primary.jsonl`, `by-name/*.json`, `by-cap/*.jsonl`, `by-purl/*.jsonl`) и в
живых HTTP-ответах сервера индекса. Никаких рекомендаций.

**Периметр** (считано по нему): `crates/vibe-index/src/types/**`,
`crates/vibe-index/src/index/**`, `crates/vibe-index/src/server/**`,
`schemas/**`, `crates/vibe-registry/src/index_client/**`.

**Ключевые числа (итог, подробности ниже):**

| # | Метрика | Значение |
|---|---|---|
| 1 | Публичных сериализуемых типов в периметре | **23** (18 каталог-файловых: 17 структур + `RepomdFileEntry`; +5 закрытых словарей-enum) + внешний `Group` из `vibe-core` на проводе |
| 2 | Полей всего | **86** (82 именованных поля структур + 4 поля в вариантах `RepomdFileEntry`) |
| 3 | Полей-коллекций, у которых «пусто» СЕЙЧАС неотличимо от «нет поля» | **14** (подтверждено; см. 3.2) |
| 4 | Объединений без тега на проводе | **1** (`RepomdFileEntry`; второго нет) |
| 5 | Мест, где каталог перечитывается и переписывается | **6** call-сайтов `write_to` поверх существующего каталога (3 CLI + 3 серверных обработчика) + `init` пишет с нуля |

---

## 3.1 Таблица типов

Обозначения: «DUF» = `deny_unknown_fields` стоит. «полей» — число полей (для
enum-вариантов — сумма полей по вариантам). Внешний `Group` приведён отдельно.

### Каталог-файловые типы (сериализуются в опубликованные файлы)

| Тип (`файл:строка`) | Роль (какой файл/часть каталога) | DUF | полей | `skip_serializing_if` | `rename`/`rename_all` | `default` |
|---|---|---|---|---|---|---|
| `Repomd` `types/repomd.rs:20` | `repomd.json` — манифест индекса | ✅ | 9 | нет (на контейнере) | нет | нет |
| `RepomdFileEntry` `types/repomd.rs:43` | значения карты `Repomd.files` (файл/директория) | ❌ (`untagged`) | 4 (2+2) | нет | нет (варианты без переимен.) | нет |
| `VersionEntry` `types/entry/mod.rs:43` | одна строка `primary.jsonl`; элемент `versions[]` | ✅ | 30 | 13 полей (см. 3.2) | нет | на 13 полях |
| `PackageEntry` `types/entry/aggregate.rs:26` | элемент `NameEntry.packages[]` | ✅ | 5 | `latest_stable` | нет | `latest_stable` |
| `NameEntry` `types/entry/aggregate.rs:69` | `by-name/<name>.json` | ✅ | 3 | нет | нет | нет |
| `WorkspaceOriginEntry` `types/entry/content.rs:19` | `VersionEntry.workspace_origin` | ✅ | 5 | `commit` | нет | `commit` |
| `FeaturesEntry` `types/entry/content.rs:38` | `VersionEntry.features` | ✅ | 2 | `features`, `exclusive` | нет | оба поля |
| `SubskillEntry` `types/entry/content.rs:61` | элемент `VersionEntry.subskills[]` | ✅ | 5 | `describes`,`description`,`channels` | нет | те же 3 |
| `I18nEntry` `types/entry/content.rs:74` | `VersionEntry.i18n` | ✅ | 2 | `available`,`default` | нет | оба |
| `BootSnippetEntry` `types/entry/content.rs:93` | `VersionEntry.boot_snippet` | ✅ | 2 | `category` | нет | `category` |
| `CompatibilityEntry` `types/entry/relations.rs:15` | `VersionEntry.compatibility` | ✅ | 2 | оба поля | нет | оба |
| `ProvidesEntry` `types/entry/relations.rs:30` | `VersionEntry.provides` | ✅ | 1 | `capabilities` | нет | да |
| `RequiresEntry` `types/entry/relations.rs:43` | `VersionEntry.requires` | ✅ | 2 | оба | нет | оба |
| `RequiresAnyEntry` `types/entry/relations.rs:58` | элемент `VersionEntry.requires_any[]` | ✅ | 1 | нет | нет | нет |
| `ObsoletesEntry` `types/entry/relations.rs:64` | `VersionEntry.obsoletes` | ✅ | 1 | `packages` | нет | да |
| `ConflictsEntry` `types/entry/relations.rs:77` | `VersionEntry.conflicts` | ✅ | 1 | `packages` | нет | да |
| `CapabilityRow` `index/inverted.rs:66` | строка `by-cap/<slug>.jsonl` | ❌ | 5 | нет | нет | нет |
| `PurlRow` `index/inverted.rs:75` | строка `by-purl/<slug>.jsonl` | ❌ | 6 | нет | нет | нет |

### Закрытые словари (fieldless enum, используются как типы полей внутри записей)

| Тип (`файл:строка`) | Где используется | `rename_all` | число значений |
|---|---|---|---|
| `PackageKind` `types/kinds.rs:21` | `VersionEntry.kind`, `CompatibilityEntry.requires_kinds[]`, inverted-ряды, HTTP | `lowercase` | 6 |
| `NamingConvention` `types/kinds.rs:88` | `Repomd.naming` | варианты `rename` поимённо | 4 |
| `DeliveryMode` `types/entry/content.rs:53` | `SubskillEntry.delivery` | `kebab-case` | 3 |
| `DirectoryTag` `types/repomd.rs:75` | `RepomdFileEntry::Directory.kind` | `lowercase` | 1 |
| `BindingSite` `index/inverted.rs:89` | `PurlRow.binding_site` | `kebab-case` | 2 |

### Внешний тип на проводе

| Тип | Где | Сериализация |
|---|---|---|
| `Group` (`vibe-core/src/package_ref.rs:41`, реэкспорт в `types/mod.rs:28`) | `VersionEntry.group`, `PackageEntry.group`, inverted-ряды, HTTP | newtype поверх `String`, `#[serde(try_from = "String", into = "String")]` → на проводе **обычная JSON-строка** с валидацией при чтении (отклоняет заглавные, пустые сегменты, недопустимые символы) |

**Поправка к «около 18»:** 18 — это в точности число каталог-файловых типов,
если считать 17 структур плюс единственное объединение `RepomdFileEntry`.
Дерево добавляет ещё 5 закрытых словарей-enum (типы полей) и внешний `Group`.
Итого различных публичных сериализуемых типов, которых касается провод каталога:
**23 в периметре + 1 внешний**. Считал прямым обходом `types/**` +
`index/inverted.rs` + реэкспортов `types/mod.rs`.

### Прочие типы «на проводе», но НЕ каталог-файловые (для полноты)

Это ответные конверты живого HTTP-API сервера индекса (`crates/vibe-index/src/server/routes/**`).
Они `#[derive(Serialize)]` (на стороне сервера — только запись), в файлы каталога
не пишутся, и даны здесь отдельным списком, чтобы не смешивать с записями каталога:

- `packages.rs`: `ListResponse`, `PackageRow`, `SearchResponse`, `SearchHit`
  (серверный), `PackageVersionsResponse`, `UpsertResponse`, `DeleteResponse`.
  Маршрут `/v1/packages/{group}/{name}/{version}` отдаёт сам `VersionEntry`
  целиком — т.е. каталог-запись попадает в HTTP-ответ один-в-один.
- `purls.rs`: `Response`, `Hit` (`binding_site` тут — `&'static str`, не enum).
- `capabilities.rs`: `Response`, `Hit`.

Клиент `crates/vibe-registry/src/index_client/wire.rs` читает это своими
**толерантными** view-структурами: `NameEntryView`/`PackageEntryView`/`VersionEntryView`
(для `by-name/*.json`), `SearchResults`/`SearchHit` (клиентский),
`PurlLookupResults`/`PurlLookupHit`, `BindingSite` (клиентский). См. 3.7.

---

## 3.2 Таблица полей

Категория: **СКАЛЯР** (строка/число/bool/версия/Group/закрытый словарь),
**СПИСОК** (`Vec`), **СЛОВАРЬ** (`BTreeMap`), **ВЛОЖЕННЫЙ** (другая каталог-структура
как поле), **ОБЪЕДИНЕНИЕ** (значение `RepomdFileEntry`).

«обяз.?» — обязательно ли поле на проводе: **всегда** (нет `default`/`skip`/`Option`)
/ **может отсутствовать**.

«различимы пусто и нет поля?» — ключевой столбец:
- **ДА** — пустое/нулевое значение сериализуется и отличается от отсутствия поля;
- **НЕТ** — пустое значение опускается (`skip_serializing_if = …::is_empty` / `is_none`),
  поэтому на проводе «пусто» и «нет поля» неразличимы;
- **всегда** — поле не может отсутствовать (состояния «нет поля» не существует).

### `Repomd` (`types/repomd.rs:20`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| Repomd.schema_version | `u32` | СКАЛЯР | всегда | — | всегда |
| Repomd.registry | `String` | СКАЛЯР | всегда | — | всегда |
| Repomd.registry_url | `String` | СКАЛЯР | всегда | — | всегда |
| Repomd.naming | `NamingConvention` | СКАЛЯР (закр. словарь) | всегда | — | всегда |
| Repomd.generated_at | `DateTime<Utc>` | СКАЛЯР | всегда | — | всегда |
| Repomd.generator | `String` | СКАЛЯР | всегда | — | всегда |
| Repomd.package_count | `u32` | СКАЛЯР | всегда | — | всегда |
| Repomd.version_count | `u32` | СКАЛЯР | всегда | — | всегда |
| Repomd.files | `BTreeMap<String, RepomdFileEntry>` | СЛОВАРЬ | всегда | нет `skip` | всегда (даже пустая карта сериализуется как `{}`) |

### `RepomdFileEntry` (`types/repomd.rs:43`, `#[serde(untagged)]`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| Directory.kind | `DirectoryTag` | СКАЛЯР (закр. словарь) | всегда* | — | всегда |
| Directory.entries | `u32` | СКАЛЯР | всегда* | — | всегда |
| File.size | `u64` | СКАЛЯР | всегда* | — | всегда |
| File.sha256 | `String` | СКАЛЯР | всегда* | — | всегда |

\* Внутри выбранного варианта поля обязательны. Само значение —
**ОБЪЕДИНЕНИЕ**: на проводе ровно один из двух наборов ключей (см. 3.3).

### `VersionEntry` (`types/entry/mod.rs:43`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| VersionEntry.schema_version | `u32` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.kind | `PackageKind` | СКАЛЯР (закр. словарь) | всегда | — | всегда |
| VersionEntry.group | `Group` | СКАЛЯР (строка) | всегда | — | всегда |
| VersionEntry.name | `String` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.version | `Version` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.content_hash | `String` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.source_url | `String` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.source_ref | `String` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.resolved_commit | `Option<String>` | СКАЛЯР | может | `Option` + `skip Option::is_none` | ДА (None→нет поля; Some("")→`""`) |
| VersionEntry.registry | `String` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.workspace_origin | `Option<WorkspaceOriginEntry>` | ВЛОЖЕННЫЙ | может | `Option` + `skip Option::is_none` | ДА (None→нет поля; Some(пустая структура)→объект) |
| VersionEntry.license | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| VersionEntry.authors | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| VersionEntry.description | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| VersionEntry.homepage | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| VersionEntry.keywords | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| VersionEntry.describes | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| VersionEntry.compatibility | `CompatibilityEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip CompatibilityEntry::is_empty` | **НЕТ** (пустая структура == нет поля) |
| VersionEntry.provides | `ProvidesEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip ProvidesEntry::is_empty` | **НЕТ** |
| VersionEntry.requires | `RequiresEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip RequiresEntry::is_empty` | **НЕТ** |
| VersionEntry.requires_any | `Vec<RequiresAnyEntry>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| VersionEntry.obsoletes | `ObsoletesEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip ObsoletesEntry::is_empty` | **НЕТ** |
| VersionEntry.conflicts | `ConflictsEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip ConflictsEntry::is_empty` | **НЕТ** |
| VersionEntry.features | `FeaturesEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip FeaturesEntry::is_empty` | **НЕТ** |
| VersionEntry.subskills | `Vec<SubskillEntry>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| VersionEntry.i18n | `I18nEntry` | ВЛОЖЕННЫЙ | может | `default` + `skip I18nEntry::is_empty` | **НЕТ** |
| VersionEntry.boot_snippet | `Option<BootSnippetEntry>` | ВЛОЖЕННЫЙ | может | `Option` + `skip is_none` | ДА |
| VersionEntry.files_count | `u32` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.indexed_at | `DateTime<Utc>` | СКАЛЯР | всегда | — | всегда |
| VersionEntry.indexed_by | `String` | СКАЛЯР | всегда | — | всегда |

### `PackageEntry` (`types/entry/aggregate.rs:26`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| PackageEntry.group | `Group` | СКАЛЯР | всегда | — | всегда |
| PackageEntry.name | `String` | СКАЛЯР | всегда | — | всегда |
| PackageEntry.indexed_at | `DateTime<Utc>` | СКАЛЯР | всегда | — | всегда |
| PackageEntry.latest_stable | `Option<Version>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| PackageEntry.versions | `Vec<VersionEntry>` | СПИСОК | всегда | нет `skip` | всегда (даже `[]` пишется; «нет поля» невозможно) |

### `NameEntry` (`types/entry/aggregate.rs:69`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| NameEntry.name | `String` | СКАЛЯР | всегда | — | всегда |
| NameEntry.indexed_at | `DateTime<Utc>` | СКАЛЯР | всегда | — | всегда |
| NameEntry.packages | `Vec<PackageEntry>` | СПИСОК | всегда | нет `skip` | всегда |

### `WorkspaceOriginEntry` (`types/entry/content.rs:19`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| WorkspaceOriginEntry.upstream | `String` | СКАЛЯР | всегда | — | всегда |
| WorkspaceOriginEntry.path | `String` | СКАЛЯР | всегда | — | всегда |
| WorkspaceOriginEntry.commit | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| WorkspaceOriginEntry.generated_by | `String` | СКАЛЯР | всегда | — | всегда |
| WorkspaceOriginEntry.generated_at | `String` | СКАЛЯР | всегда | — | всегда |

### `FeaturesEntry` (`types/entry/content.rs:38`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| FeaturesEntry.features | `BTreeMap<String, Vec<String>>` | СЛОВАРЬ | может | `default` + `skip BTreeMap::is_empty` | **НЕТ** |
| FeaturesEntry.exclusive | `BTreeMap<String, Vec<String>>` | СЛОВАРЬ | может | `default` + `skip BTreeMap::is_empty` | **НЕТ** |

### `SubskillEntry` (`types/entry/content.rs:61`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| SubskillEntry.path | `String` | СКАЛЯР | всегда | — | всегда |
| SubskillEntry.delivery | `DeliveryMode` | СКАЛЯР (закр. словарь) | всегда | — | всегда |
| SubskillEntry.describes | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| SubskillEntry.description | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| SubskillEntry.channels | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `I18nEntry` (`types/entry/content.rs:74`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| I18nEntry.available | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| I18nEntry.default | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |

### `BootSnippetEntry` (`types/entry/content.rs:93`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| BootSnippetEntry.source | `String` | СКАЛЯР | всегда | — | всегда |
| BootSnippetEntry.category | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |

### `CompatibilityEntry` (`types/entry/relations.rs:15`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| CompatibilityEntry.min_vibe_version | `Option<String>` | СКАЛЯР | может | `Option` + `skip is_none` | ДА |
| CompatibilityEntry.requires_kinds | `Vec<PackageKind>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `ProvidesEntry` (`types/entry/relations.rs:30`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| ProvidesEntry.capabilities | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `RequiresEntry` (`types/entry/relations.rs:43`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| RequiresEntry.packages | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |
| RequiresEntry.capabilities | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `RequiresAnyEntry` (`types/entry/relations.rs:58`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| RequiresAnyEntry.one_of | `Vec<String>` | СПИСОК | всегда | нет `skip` (но вся запись живёт в `requires_any[]`, который опускается если пуст) | всегда |

### `ObsoletesEntry` (`types/entry/relations.rs:64`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| ObsoletesEntry.packages | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `ConflictsEntry` (`types/entry/relations.rs:77`)
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| ConflictsEntry.packages | `Vec<String>` | СПИСОК | может | `skip Vec::is_empty` | **НЕТ** |

### `CapabilityRow` (`index/inverted.rs:66`) — нет `deny_unknown_fields`
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| CapabilityRow.kind | `PackageKind` | СКАЛЯР (закр. словарь) | всегда | — | всегда |
| CapabilityRow.group | `Group` | СКАЛЯР (строка) | всегда | — | всегда |
| CapabilityRow.name | `String` | СКАЛЯР | всегда | — | всегда |
| CapabilityRow.version | `Version` | СКАЛЯР | всегда | — | всегда |
| CapabilityRow.capability | `String` | СКАЛЯР | всегда | — | всегда |

### `PurlRow` (`index/inverted.rs:75`) — нет `deny_unknown_fields`
| тип.поле | Rust-тип | кат. | обяз.? | почему может отсутствовать | различимы? |
|---|---|---|---|---|---|
| PurlRow.kind | `PackageKind` | СКАЛЯР (закр. словарь) | всегда | — | всегда |
| PurlRow.group | `Group` | СКАЛЯР (строка) | всегда | — | всегда |
| PurlRow.name | `String` | СКАЛЯР | всегда | — | всегда |
| PurlRow.version | `Version` | СКАЛЯР | всегда | — | всегда |
| PurlRow.purl | `String` | СКАЛЯР | всегда | — | всегда |
| PurlRow.binding_site | `BindingSite` | СКАЛЯР (закр. словарь) | всегда | — | всегда |

### Сводка по коллекциям (3.2, итог)

Всего **18 полей-коллекций** (СПИСОК `Vec` или СЛОВАРЬ `BTreeMap`) среди
каталог-типов. Из них у **14** пустое значение СЕЙЧАС неотличимо от
отсутствующего (`skip_serializing_if = …::is_empty` опускает пустоту; на
проводе невозможно отличить `[]`/`{}` от «ключа нет»):

1. `VersionEntry.authors` — `Vec<String>`
2. `VersionEntry.keywords` — `Vec<String>`
3. `VersionEntry.requires_any` — `Vec<RequiresAnyEntry>`
4. `VersionEntry.subskills` — `Vec<SubskillEntry>`
5. `FeaturesEntry.features` — `BTreeMap<String, Vec<String>>`
6. `FeaturesEntry.exclusive` — `BTreeMap<String, Vec<String>>`
7. `SubskillEntry.channels` — `Vec<String>`
8. `I18nEntry.available` — `Vec<String>`
9. `CompatibilityEntry.requires_kinds` — `Vec<PackageKind>`
10. `ProvidesEntry.capabilities` — `Vec<String>`
11. `RequiresEntry.packages` — `Vec<String>`
12. `RequiresEntry.capabilities` — `Vec<String>`
13. `ObsoletesEntry.packages` — `Vec<String>`
14. `ConflictsEntry.packages` — `Vec<String>`

Остальные 4 коллекции всегда присутствуют (нет `skip`), поэтому «пусто» и
«нет поля» для них различимы тривиально (состояния «нет поля» просто нет):
`PackageEntry.versions`, `NameEntry.packages`, `RequiresAnyEntry.one_of`,
`Repomd.files`.

**Перепроверка числа «14» из пакета:** подтверждено в точности. Считал так:
прошёл grep-ом все `skip_serializing_if = "…::is_empty"` по `types/entry/**`
и `types/repomd.rs`, отфильтровал поля, чей Rust-тип — `Vec` или `BTreeMap`.
Получилось ровно 14. Дополнительно отмечу: **7 вложенных структур-полей**
(`VersionEntry.compatibility/provides/requires/obsoletes/conflicts/features/i18n`)
имеют ту же самую неразличимость через собственные `is_empty()`-методы — это
ВЛОЖЕННЫЕ, не коллекции, но семантически та же потеря различия «пусто/нет».

---

## 3.3 Объединения (варианты формы)

Полный обход serde-атрибутов периметра (grep `untagged|tag =|flatten` по
`crates/vibe-index/src` и `crates/vibe-registry/src/index_client`) даёт
**ровно одно** объединение на проводе:

### `RepomdFileEntry` (`types/repomd.rs:43`) — `#[serde(untagged)]`

```rust
#[serde(untagged)]
pub enum RepomdFileEntry {
    Directory { kind: DirectoryTag, entries: u32 },
    File { size: u64, sha256: String },
}
```

- **Как читатель отличает варианты СЕГОДНЯ:** это `untagged` enum —
  внешнего поля-тега на уровне объекта нет. serde перебирает варианты в порядке
  объявления и пробует десериализовать каждый. Вариант `Directory` объявлен
  первым.
- **Поле-тег:** явного тега на контейнере нет, **но** внутри варианта `Directory`
  есть поле `kind: DirectoryTag` со значением всегда `"directory"` — оно
  самодокументирует директорию (см. тест `directory_serialises_with_kind_tag`,
  `repomd.rs:119`).
- **Наборы ключей вариантов:**
  - `Directory` → `{ "kind", "entries" }`
  - `File` → `{ "size", "sha256" }`
- **Пересекаются ли наборы ключей:** **НЕТ.** `{kind, entries} ∩ {size, sha256} = ∅`.
  Поэтому разбор **однозначен**: объект с `size`/`sha256` не совпадёт с
  `Directory` (нет `kind`/`entries`) и уйдёт в `File`; объект с
  `kind`/`entries` совпадёт с `Directory`. Поле `kind` здесь — запасная
  страховка поверх уже непересекающихся ключей.

Замечание о робастности: т.к. enum `untagged` и **без** `deny_unknown_fields`,
лишний ключ в файловой записи (напр. `{"size":..,"sha256":"..","x":1}`) всё
ровно сматчится с `File`. А вот неизвестное значение `kind` (не `"directory"`)
в записи директории завалит вариант `Directory`, затем не сматчится с `File`
(нет `size`/`sha256`) → **ошибка разбора**.

**Второго объединения нет.** Известный кандидат был единственным. Никаких
других `untagged`/`tag =`/ручных `impl Deserialize` в периметре не обнаружено
(`flatten` встречается только в `crates/vibe-index/src/lockfile.rs:22,37` —
это парсинг `vibe.lock`, к каталогу отношения не имеет). Если бы второе
объединение существовало, оно было бы главной находкой; его нет.

---

## 3.4 Закрытые словари на проводе (fieldless enum)

Все закрытые словари — это `enum` без полей с производным `Serialize`/`Deserialize`.
Поведение при **неизвестном значении** для всех одно и то же: производный
`Deserialize` для fieldless enum без `#[serde(other)]` даёт **ошибку разбора**
(`unknown variant`), запасного варианта нет. Никакого `#[serde(other)]` в дереве
нет (grep подтвердил).

| Словарь (`файл:строка`) | Значения на проводе (дословно) | Сериализация | Что делает читатель с неизвестным значением | Вторые копии списка в дереве |
|---|---|---|---|---|
| `PackageKind` `types/kinds.rs:21` | `flow`, `feat`, `stack`, `tool`, `mcp`, `lang` | `rename_all = "lowercase"` → строки (см. `as_str`, `kinds.rs:37`) | **ошибка разбора** (derived Deserialize, unknown variant). `FromStr` тоже ошибается (`kinds.rs:76`) | ДА: (1) преднамеренный дубликат в `vibe-core` (kinds.rs:1-5, parity-тест); (2) внутри файла список повторён в `as_str` match, `all()` (`:48`), `from_str` match **и в строке сообщения об ошибке** `:77` |
| `NamingConvention` `types/kinds.rs:88` | `fqdn`, `kind-name`, `name`, `kind/name` | поимённый `#[serde(rename = …)]` на каждом варианте | **ошибка разбора** | ДА: зеркалирует `vibe-core::manifest::NamingConvention` (kinds.rs:84-86); те же строки дублированы в `as_str` (`:114`) и в `value(name=…)` |
| `DeliveryMode` `types/entry/content.rs:53` | `eager`, `lazy-push`, `lazy-pull` | `rename_all = "kebab-case"` → строки | **ошибка разбора** | вторая копия не найдена (зеркалирует `vibe-core`, но локально не дублируется) |
| `DirectoryTag` `types/repomd.rs:75` | `directory` | `rename_all = "lowercase"` | **ошибка разбора**, но т.к. живёт внутри `untagged` `RepomdFileEntry`, неизвестный `kind` заваливает `Directory`, затем не матчит `File` → в итоге **ошибка разбора всего значения** | вторая копия не найдена |
| `BindingSite` `index/inverted.rs:89` | `package`, `subskill` | `rename_all = "kebab-case"` (одиночные слова → как lowercase) | **ошибка разбора** | **ДА — отдельная находка:** тот же словарь существует **вторым типом** в клиенте `crates/vibe-registry/src/index_client/wire.rs:94` (`rename_all = "lowercase"`). Сегодня значения совпадают (`"package"`/`"subskill"`), но это два независимых Rust-типа в двух crate'ах — точка потенциального расхождения |

---

## 3.5 Версия формата

- **Где объявлена:**
  - `Repomd::SCHEMA_VERSION: u32 = 1` (`types/repomd.rs:38`); поле `Repomd.schema_version` пишется в `repomd.json`.
  - `VersionEntry::SCHEMA_VERSION: u32 = 1` (`types/entry/mod.rs:124`); поле `VersionEntry.schema_version` пишется в каждую строку `primary.jsonl`.
  - Дубль в памяти: `Index SCHEMA_VERSION = 1` (`index/memory.rs:28`), поле `Index.schema_version`.
- **Значение:** `1` (везде).
- **Ветвится ли по этому значению хоть что-нибудь:** **НЕТ.** Поле только
  присваивается, сериализуется, читается обратно и хранится. grep по `crates`
  показывает: `Index::load_from` кладёт `schema_version: manifest.schema_version`
  в память без проверки (`memory.rs:273`); `cli/dump.rs:57` печатает
  `"schema_version": index.schema_version`. **Ни одного сравнения** (`==`/`!=`/`match`)
  для версий `Repomd`/`VersionEntry` в каталоге нет.
- **Контраст:** `vibe.lock` имеет свою, отдельную версию (`CURRENT_SCHEMA_VERSION`,
  `vibe-core/src/manifest/lockfile.rs:430` — `if lockfile.meta.schema_version !=
  CURRENT_SCHEMA_VERSION`), и там по ней **ветвятся/отвергают**. Но это `vibe.lock`
  (TOML, схема 5), а не каталог индекса. Свои `schema_version` есть также у
  побочных файлов (`Checkpoint` `index/checkpoint.rs:21`, сканер-кэш
  `scanner/org_cache.rs:50`) — это не опубликованный каталог.
- **Вывод:** версия каталога сегодня — **ярлык, а не переключатель**. Любое
  значение (в т.ч. неизвестное) молча принимается при чтении.

---

## 3.6 Кругооборот — читается ли каталог, чтобы переписать его

`Index::write_to` (`index/memory.rs:161`) **всегда перегенерирует все файлы
каталога из типизированной in-memory модели** (`by_pkgref: BTreeMap<PkgKey,
PackageEntry>`), а НЕ из сырого прочитанного JSON. Источник in-memory модели
разнится по пути:

| Путь | Читает существующий каталог? | Откуда берутся данные для записи |
|---|---|---|
| `vibe-index init` (`cli/init.rs:48`) | нет | пустой `Index::new`, запись с нуля |
| `vibe-index add` (`cli/add.rs:51,122`) | **ДА** (`Index::load_from`) | существующие записи — из **прочитанного с диска** каталога; одна новая запись — из источника (`vibe.toml`); затем `upsert` + `write_to` |
| `vibe-index remove` (`cli/remove.rs:35,57`) | **ДА** (`Index::load_from`) | все записи — из **прочитанного с диска** каталога; `write_to` перегенерирует |
| `vibe-index reindex` / `rescan-org` (`cli/reindex.rs:218,281`) | **ДА** (грузит `existing` минимум ради `registry`/`naming`) | **incremental**: неизменённые записи берёт из `existing.iter_versions()` (т.е. из прочитанного каталога, `:250`), изменённые — из сканирования источника; **full**: собирает заново из сканирования; оба режима — `write_to` |
| сервер: `POST /v1/packages` upsert (`server/routes/packages.rs:256`) | **ДА** (косвенно) | `Index` загружен с диска один раз на старте (`cli/serve.rs:77` `load_from`), мутация в памяти, `write_to` перегенерирует весь каталог |
| сервер: `DELETE …/{version}` (`packages.rs:304`) | **ДА** (косвенно) | та же in-memory модель со старта → `write_to` |
| сервер: `DELETE …/{group}/{name}` (`packages.rs:333`) | **ДА** (косвенно) | та же in-memory модель со старта → `write_to` |

**Точный ответ на вопрос 3.6:** каталог в нескольких путях **перечитывается с
диска и переписывается** (`add`, `remove`, incremental-`reindex`, а также
серверные upsert/delete поверх стартовой загрузки). При этом `write_to`
**всегда** собирает файлы из типизированной памяти; сырой JSON сквозь не
прокачивается. Следствие для решения о смене формата:

- Поля, которых наш код не знает, **не выживают** при перезаписи — они
  отсутствуют в типизированных структурах и просто не запишутся обратно.
- Сильнее того: поскольку у **каждой** каталог-записи стоит
  `deny_unknown_fields` (см. 3.7), неизвестное поле не даст каталог
  **прочитать** вообще — `load_from` упадёт с ошибкой разбора ещё до того, как
  дойдёт до перезаписи. Исключение — inverted-ряды `CapabilityRow`/`PurlRow`,
  у которых `deny_unknown_fields` нет (но они и не перечитываются для
  перезаписи нигде — их всегда перегенерируют из `VersionEntry`).
- Полностью **с нуля** каталог собирают только `init` и `reindex` в режиме full
  (и то поля ограничены тем, что умеют типы).

Итого call-сайтов `write_to`, переписывающих уже существующий каталог: **6**
(`add`, `remove`, `reindex/rescan`, серверные `upsert`, `delete_version`,
`delete_package`), плюс `init` пишет с нуля.

---

## 3.7 Строгость на чтении

Места десериализации записей каталога (`serde_json::from_*` над каталог-типами):

| Место | Тип | `deny_unknown_fields`? | Лишнее поле → ошибка? |
|---|---|---|---|
| `index/primary.rs:92` (`parse`, `serde_json::from_str` по строке) | `VersionEntry` | ✅ (entry/mod.rs:38) | **ДА, ошибка разбора** (`Error::Malformed`) |
| `index/by_name.rs:67` (`parse`, `from_slice`) | `NameEntry` → `PackageEntry` → `VersionEntry` | ✅ все три | **ДА** |
| `index/by_name.rs:94` (внутри `read_all`) | `NameEntry` | ✅ | **ДА** |
| `index/repomd.rs:38` (`read`, `from_slice`) | `Repomd` | ✅ (repomd.rs:15) | **ДА** |
| `index/memory.rs:263-264` (`load_from` → `repomd::read` + `by_name::read_all`) | `Repomd`, `NameEntry` | ✅ | **ДА** (через подчинённые функции) |
| `index/inverted.rs` | чтения `CapabilityRow`/`PurlRow` **нет** (только запись + подсчёт) | DUF отсутствует | н/д (никто не парсит) |
| `server/routes/packages.rs:232` (тело `POST`) | `VersionEntry` (axum `Json`) | ✅ | **ДА** → HTTP 400 |
| `server/routes/index_files.rs` | **сырые байты файлов отдаются как есть** (`serve_file`) | — | сервер сам не десериализует; строгость — на стороне читателя |

**Итог по записям каталога:** все типизированные чтения **строгие** — запись с
лишним незнакомым полем падает с ошибкой разбора. Сырые файлы сервер отдаёт
байт-в-байт, не разбирая.

### Клиент `crates/vibe-registry/src/index_client/**` — строг или терпим?

**Терпим.** Клиент намеренно использует **отдельные** «view»-структуры с
у́зким набором полей, `#[serde(default)]` и **без** `deny_unknown_fields`:

- `NameEntryView` (`wire.rs:18`) читает только `packages[]`; у `PackageEntryView`
  (`wire.rs:24`) — `group` + `versions[]`; у `VersionEntryView` (`wire.rs:31`) —
  только `version`. Комментарий прямым текстом (`wire.rs:14-16`): *«Only the
  fields the resolver's version selector needs are read; the rest of the
  on-disk shape is tolerated.»*
- `SearchResults`/`SearchHit` (`wire.rs:45,56`): *«Extra fields on the wire
  (today: `command`) are tolerated silently»* (`wire.rs:37-39`); тест
  `search_results_decode_minimal_envelope` (`wire.rs:115`) пропускает лишний
  ключ `command`.
- `PurlLookupResults`/`PurlLookupHit` — те же `#[serde(default)]`, без DUF.

Это наш собственный внешний потребитель, и его поведение — **образец того, что
сделает чужой читатель**: он увидит только те поля, которые знает, а всё
прочее (включая будущие новые поля) тихо проигнорирует. Следствие: добавить
поле в запись не сломает такого читателя; убрать/переименовать известное ему
поле — сломает.

---

## 3.8 Схемы

**Описывает ли хоть одна схема в дереве типы каталога индекса?** — **НЕТ.**

Формат каталога сегодня задан **только** Rust-структурами с `serde` в
`crates/vibe-index/src/types/**` (и `index/inverted.rs`). Никакого
`*.jtd.json`/`*.schema.json` для `VersionEntry`, `Repomd`, `NameEntry`,
`CapabilityRow`, `PurlRow` и т.п. в дереве нет.

Присутствующие в дереве `schemas/*.jtd.json` (JSON Type Definition) описывают
**совсем другой провод** — JSON-отчёты CLI-команд, генерируемые в
`crates/vibe-wire/src/generated/*`:

| Файл | Что описывает |
|---|---|
| `schemas/init_report.jtd.json` | отчёт `vibe init --json` |
| `schemas/install_plan.jtd.json` | план установки (`vibe install --json`) |
| `schemas/install_report.jtd.json` | отчёт об установке |
| `schemas/list_report.jtd.json` | отчёт `vibe list --json` |
| `schemas/registry_publish_report.jtd.json` | отчёт `vibe registry publish --json` |
| `schemas/registry_sync_report.jtd.json` | отчёт `vibe registry sync --json` (metadescription прямо указывает source of truth → `crates/vibe-wire/src/generated/registry_sync_report.rs`) |
| `schemas/uninstall_report.jtd.json` | отчёт об удалении |

Отдельно: существует `crates/vibe-cli/resources/package-tree.schema.v1.json` —
но это схема **`vibe load --json`** (дерево пакетов проекта, `schema_version: 1`,
`const`), к каталогу индекса отношения не имеет (упомянут здесь, чтобы не
оставлять сомнений: это тоже не каталог).

**Вывод:** каталог индекса — единственная крупная часть провода vibevm, у
которой **нет** машинной схемы; его «контракт» существует только как код.
