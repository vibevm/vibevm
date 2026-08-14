# Ф1.4 — анатомия типов записи каталога: замер

Чем мерил: чтение файлов (`crates/vibe-index/src/types/**`, `crates/vibe-index/src/index/**`,
`crates/vibe-index/src/cli/**`, `crates/vibe-index/src/scanner/**`, `crates/vibe-index/src/server/**`,
`crates/vibe-index/tests/**`, `crates/vibe-index/docs/format.md`) плюс `rg` (ripgrep, в PATH) и `wc -l`
из Git Bash. Периметр чтения — только `crates/**`; `vibedeps/**` и `packages/**` не открывались.
Что НЕ запускалось: `cargo` (любой), `git` (любой). Замер от 2026-08-14, рабочее дерево
`.wt/F1-4-TYPES`, состояние HEAD на момент замера — `ecad5c9e`.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ** — четыре слота ложатся в сегодняшние типы без структурной перестройки
(домашние serde-формы для «пусто ≡ нет поля» уже есть, бюджет длины файлов не лимитирует,
тестов, ломающихся новым опущаемым-когда-пусто полем, в дереве нет), но мешает следующее:

1. **Ни одного поля типа `bool` во всём `crates/vibe-index/src/types/`** — `yanked`/`frozen`
   заводят новую serde-форму, которой в дереве нет (§7); выбирать её надо осознанно.
2. **11 struct-литералов `VersionEntry`, все — исчерпывающие** (ни одного `..Default::default()`
   во всём `crates/`, свип дал 0): добавление поля без `#[serde(default)]`-миграции литералов
   ломает компиляцию на каждом из 11 сайтов (§4).
3. **`#[serde(deny_unknown_fields)]` стоит на всех 15 агрегатах каталога** (B3): старый
   читатель (без нового поля) откажет на новом файле, где слот непуст; выкатка требует
   очереди «писатель после читателя».
4. **`tombstone` на уровне `NameEntry` упрётся в пересборку**: `Index::write_to` строит каждый
   `NameEntry` **с нуля** через `NameEntry::new` (`crates/vibe-index/src/index/memory.rs:200`) —
   поле, не протаскиваемое явно из `by_pkgref`, молча обнуляется на каждой записи каталога (§6, §10).
5. **Гранулярность отказа при чтении — весь файл/вся загрузка**: единственный цикл по отдельным
   записям есть в `primary::parse` (`crates/vibe-index/src/index/primary.rs:88`), но
   `Index::load_from` читает **by-name, а не primary** (`crates/vibe-index/src/index/memory.rs:262-271`),
   а `by_name::parse` парсит файл целиком одним `from_slice` (§6).

## 2. Сверка опорных координат (B1..B8)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | `VersionEntry` объявлен в `crates/vibe-index/src/types/entry/mod.rs:43` | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/types/entry/mod.rs:43` — `pub struct VersionEntry {` |
| B2 | поле `content_hash` имеет тип `String` и объявлено в `:55` | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/types/entry/mod.rs:55` — `pub content_hash: String,` |
| B3 | `deny_unknown_fields` в `entry/mod.rs:38`, всего по каталогу 15 (aggregate 2, content 5, entry/mod 1, relations 6, repomd 1) | ПОДТВЕРЖДЕНО | атрибут: `crates/vibe-index/src/types/entry/mod.rs:38`; счётчик `rg -c` (§7 отчёта): `entry/mod.rs:1, aggregate.rs:2, relations.rs:6, repomd.rs:1, content.rs:5` → 15 |
| B4 | `VersionEntry::minimal` объявлен в `:167` и штампует `indexed_at` через `Utc::now()` | ОПРОВЕРГНУТО | объявление — `crates/vibe-index/src/types/entry/mod.rs:132` (`pub fn minimal(`); строка 167 — это `indexed_at: Utc::now(),` ВНУТРИ тела (литерал на 138-169). Суть («штампует `Utc::now()`») верна, координата объявления — нет |
| B5 | `sort_key()` в `:174-176`, возвращает `(&Group, &str, &Version)` без `content_hash` | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/types/entry/mod.rs:174-176` — `pub fn sort_key(&self) -> (&Group, &str, &Version) { (&self.group, self.name.as_str(), &self.version) }` |
| B6 | `NameEntry` объявлен в `crates/vibe-index/src/types/entry/aggregate.rs:69` | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/types/entry/aggregate.rs:69` — `pub struct NameEntry {` |
| B7 | во всём `crates/vibe-index/src` нет вхождений `yank`, `tombstone`, `must_understand`, `signature` | ОПРОВЕРГНУТО | `crates/vibe-index/src/scanner/from_github.rs:89` — `/// seam signature.` (doc-коммент поля `opts`, слово «signature» в прозе). `yank`/`tombstone`/`must_understand` — действительно 0 вхождений (та же команда, вывод в §7 отчёта) |
| B8 | поверхность «пусто ≡ нет поля» по `types/entry/**` — 35 мест (Option::is_none 14, Vec::is_empty 12, BTreeMap::is_empty 2, `*Entry::is_empty` 7) | ПОДТВЕРЖДЕНО | `rg -c` по `crates/vibe-index/src/types/entry`: `Option::is_none` = 7+5+1+1 = 14; `Vec::is_empty` = 4+2+6 = 12; `BTreeMap::is_empty` = 2 (content.rs); `Entry::is_empty` = 7 (mod.rs); сумма 35 = общему числу `skip_serializing_if` в entry/** (18+1+9+7) |

## 3. Инвентарь типов записи

Файлы каталога типов: `types/mod.rs` (реэкспорты), `types/kinds.rs`, `types/repomd.rs`,
`types/entry/mod.rs`, `types/entry/aggregate.rs`, `types/entry/content.rs`,
`types/entry/relations.rs`, `types/entry/tests.rs` (только `#[cfg(test)]`, подключён
`entry/mod.rs:179-180`). Других файлов в `types/` нет (ls).

Общие атрибуты записи: у каждого struct-агрегата стоит
`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` + `#[serde(deny_unknown_fields)]`;
у трёх корневых (`VersionEntry`, `PackageEntry`, `NameEntry`) дополнительно
`#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry", r = 1)]`.
Ниже serde-атрибуты уровня ПОЛЯ; «—» = атрибутов нет.

### 3.1. Корневые записи

**VersionEntry** — struct, `crates/vibe-index/src/types/entry/mod.rs:43`. Атрибуты над типом:
`#[serde(deny_unknown_fields)]` (mod.rs:38) + `#[spec(… PROP-005#entry, r=1)]` (mod.rs:39-42).
Содержится в: `PackageEntry.versions` (aggregate.rs:32); корень строки `primary.jsonl`
(primary.rs:1, 20); тело `POST /v1/packages` (packages.rs:232). Поля (30):

| поле | тип | serde-атрибуты поля | строка |
|---|---|---|---|
| schema_version | u32 | — | mod.rs:44 |
| kind | PackageKind | — | mod.rs:46 |
| group | Group (из vibe_core) | — | mod.rs:51 |
| name | String | — | mod.rs:52 |
| version | semver::Version | — | mod.rs:53 |
| content_hash | String | — | mod.rs:55 |
| source_url | String | — | mod.rs:56 |
| source_ref | String | — | mod.rs:57 |
| resolved_commit | Option<String> | `default, skip_serializing_if = "Option::is_none"` | mod.rs:59-60 |
| registry | String | — | mod.rs:62 |
| workspace_origin | Option<WorkspaceOriginEntry> | `default, skip_serializing_if = "Option::is_none"` | mod.rs:67-68 |
| license | Option<String> | `default, skip … Option::is_none` | mod.rs:70-71 |
| authors | Vec<String> | `default, skip … Vec::is_empty` | mod.rs:72-73 |
| description | Option<String> | `default, skip … Option::is_none` | mod.rs:74-75 |
| homepage | Option<String> | `default, skip … Option::is_none` | mod.rs:76-77 |
| keywords | Vec<String> | `default, skip … Vec::is_empty` | mod.rs:78-79 |
| describes | Option<String> | `default, skip … Option::is_none` | mod.rs:84-85 |
| compatibility | CompatibilityEntry | `default, skip … CompatibilityEntry::is_empty` | mod.rs:87-88 |
| provides | ProvidesEntry | `default, skip … ProvidesEntry::is_empty` | mod.rs:90-91 |
| requires | RequiresEntry | `default, skip … RequiresEntry::is_empty` | mod.rs:93-94 |
| requires_any | Vec<RequiresAnyEntry> | `default, skip … Vec::is_empty` | mod.rs:96-97 |
| obsoletes | ObsoletesEntry | `default, skip … ObsoletesEntry::is_empty` | mod.rs:99-100 |
| conflicts | ConflictsEntry | `default, skip … ConflictsEntry::is_empty` | mod.rs:102-103 |
| features | FeaturesEntry | `default, skip … FeaturesEntry::is_empty` | mod.rs:105-106 |
| subskills | Vec<SubskillEntry> | `default, skip … Vec::is_empty` | mod.rs:108-109 |
| i18n | I18nEntry | `default, skip … I18nEntry::is_empty` | mod.rs:111-112 |
| boot_snippet | Option<BootSnippetEntry> | `default, skip … Option::is_none` | mod.rs:114-115 |
| files_count | u32 | — | mod.rs:117 |
| indexed_at | DateTime<Utc> | — | mod.rs:119 |
| indexed_by | String | — | mod.rs:120 |

Методы: `SCHEMA_VERSION: u32 = 1` (mod.rs:124), `minimal()` (mod.rs:132), `sort_key()` (mod.rs:174).

**PackageEntry** — struct, `crates/vibe-index/src/types/entry/aggregate.rs:26`. Над типом:
`deny_unknown_fields` (aggregate.rs:21) + `#[spec(…)]` (22-25). Содержится в:
`NameEntry.packages` (aggregate.rs:75); RAM `Index.by_pkgref: BTreeMap<PkgKey, PackageEntry>`
(memory.rs:70). Поля (5): `group: Group` (27, —), `name: String` (28, —),
`indexed_at: DateTime<Utc>` (29, —), `latest_stable: Option<Version>` (30-31,
`default, skip … Option::is_none`), `versions: Vec<VersionEntry>` (32, —).
Методы: `new` (36-44), `finalise` (47-55 — сортирует versions по version и пересчитывает
latest_stable как последнюю без pre).

**NameEntry** — struct, `crates/vibe-index/src/types/entry/aggregate.rs:69`. Над типом:
`deny_unknown_fields` (64) + `#[spec(…)]` (65-68). Содержится в: корень файла
`by-name/<name>.json` (by_name.rs:30-32, 57-68). Полный дословный вид (ключевой вход шага):

```rust
pub struct NameEntry {
    pub name: String,
    pub indexed_at: DateTime<Utc>,
    /// One entry per `group` that publishes a package called `name`,
    /// sorted by `group`. A length greater than one is a short-name
    /// collision (PROP-008 §2.7).
    pub packages: Vec<PackageEntry>,
}
```
(aggregate.rs:69-76; derive-строка 63, `deny_unknown_fields` — 64.)

То есть **`NameEntry` сегодня не несёт ни одного serde-атрибута уровня поля** — все три поля
обязательные и всегда сериализуются. `tombstone` будет его ПЕРВЫМ опциональным полем.
Методы: `new` (79-85), `finalise` (90-96 — сортирует packages по group и ставит `indexed_at`
по свежайшему кандидату).

### 3.2. Подсекции content (`types/entry/content.rs`)

| тип | вид / строка | поля (тип — serde) | кем содержится |
|---|---|---|---|
| WorkspaceOriginEntry | struct, content.rs:19, `deny_unknown_fields` (18) | upstream: String (21, —); path: String (23, —); commit: Option<String> (25-26, `default, skip…Option::is_none`); generated_by: String (28, —); generated_at: String (30, —) | VersionEntry.workspace_origin (mod.rs:68) |
| FeaturesEntry | struct, content.rs:38, `deny_unknown_fields` (37), `Default` | features: BTreeMap<String, Vec<String>> (39-40, `default, skip…BTreeMap::is_empty`); exclusive: BTreeMap<String, Vec<String>> (41-42, то же) | VersionEntry.features (mod.rs:106) |
| DeliveryMode | enum, content.rs:53, `#[serde(rename_all = "kebab-case")]` (52) | варианты Eager/LazyPush/LazyPull → `eager`/`lazy-push`/`lazy-pull` | SubskillEntry.delivery (content.rs:63) |
| SubskillEntry | struct, content.rs:61, `deny_unknown_fields` (60) | path: String (62, —); delivery: DeliveryMode (63, —); describes: Option<String> (64-65, `default, skip…Option::is_none`); description: Option<String> (66-67, то же); channels: Vec<String> (68-69, `default, skip…Vec::is_empty`) | VersionEntry.subskills (mod.rs:109) |
| I18nEntry | struct, content.rs:74, `deny_unknown_fields` (73), `Default` | available: Vec<String> (75-76, `default, skip…Vec::is_empty`); default: Option<String> (77-78, `default, skip…Option::is_none`) | VersionEntry.i18n (mod.rs:112) |
| BootSnippetEntry | struct, content.rs:93, `deny_unknown_fields` (92) | source: String (95, —); category: Option<String> (99-100, `default, skip…Option::is_none`) | VersionEntry.boot_snippet (mod.rs:115) |

### 3.3. Подсекции relations (`types/entry/relations.rs`)

| тип | строка | поля (тип — serde) | кем содержится |
|---|---|---|---|
| CompatibilityEntry | relations.rs:15, `deny_unknown_fields` (14), `Default` | min_vibe_version: Option<String> (16-17, `default, skip…Option::is_none`); requires_kinds: Vec<PackageKind> (18-19, `default, skip…Vec::is_empty`) | VersionEntry.compatibility (mod.rs:88) |
| ProvidesEntry | relations.rs:30, `deny_unknown_fields` (29), `Default` | capabilities: Vec<String> (31-32, `default, skip…Vec::is_empty`) | VersionEntry.provides (mod.rs:91) |
| RequiresEntry | relations.rs:43, `deny_unknown_fields` (42), `Default` | packages: Vec<String> (44-45, skip Vec); capabilities: Vec<String> (46-47, skip Vec) | VersionEntry.requires (mod.rs:94) |
| RequiresAnyEntry | relations.rs:58, `deny_unknown_fields` (57), БЕЗ `Default` | one_of: Vec<String> (59, — БЕЗ default/skip) | VersionEntry.requires_any (mod.rs:97) |
| ObsoletesEntry | relations.rs:64, `deny_unknown_fields` (63), `Default` | packages: Vec<String> (65-66, skip Vec) | VersionEntry.obsoletes (mod.rs:100) |
| ConflictsEntry | relations.rs:77, `deny_unknown_fields` (76), `Default` | packages: Vec<String> (78-79, skip Vec) | VersionEntry.conflicts (mod.rs:103) |

У каждого (кроме RequiresAnyEntry) есть свой `pub fn is_empty(&self) -> bool`
(relations.rs:23, 36, 51, 70, 83; content.rs:46, 82).

### 3.4. kinds и repomd

| тип | вид / строка | атрибуты над типом / вариантами | кем содержится |
|---|---|---|---|
| PackageKind | enum, kinds.rs:21 | `#[serde(rename_all = "lowercase")]` (19), `#[value(rename_all = "kebab-case")]` (20); варианты Flow/Feat/Stack/Tool/Mcp/Lang | VersionEntry.kind (mod.rs:46); inverted-ряды (inverted.rs:67, 76) |
| NamingConvention | enum, kinds.rs:88 | варианты с точечными `#[serde(rename = …)]`: `fqdn` (94), `kind-name` (99), `name` (104), `kind/name` (109); `#[default]` Fqdn (93) | Repomd.naming (repomd.rs:24); Index.naming (memory.rs:67) |
| Repomd | struct, repomd.rs:20 | `deny_unknown_fields` (15), `#[spec(… PROP-005#layout)]` (16-19) | корень `repomd.json` (index/repomd.rs:14, 32-39) |
| RepomdFileEntry | enum, repomd.rs:43 | `#[serde(untagged)]` (42); Directory { kind: DirectoryTag, entries: u32 } / File { size: u64, sha256: String } — поля без атрибутов | Repomd.files: BTreeMap<String, RepomdFileEntry> (repomd.rs:34) |
| DirectoryTag | enum, repomd.rs:75 | `#[serde(rename_all = "lowercase")]` (74); единственный вариант Directory → `"directory"` | RepomdFileEntry::Directory.kind |

Вне `types/`, но на проводе каталога: `Group` — реэкспорт `vibe_core::Group`
(types/mod.rs:28), сериализуется как строка reverse-FQDN. Inverted-ряды
`CapabilityRow` (inverted.rs:66) и `PurlRow` (inverted.rs:75) + `BindingSite`
(inverted.rs:89, kebab-case) — отдельные провода `by-cap/`/`by-purl/`, записи каталога
не содержат.

## 4. Все места конструирования VersionEntry / NameEntry / PackageEntry

Метод: `rg -n "VersionEntry\s*\{" crates/`, `rg -n "VersionEntry::" crates/`, то же для
NameEntry/PackageEntry (оба написания покрываются `\s*\{` и `::`; пробельных вариантов
`VersionEntry{` без `\s` свип `\s*` уже включает). Контрольный полный свип
`rg -c "\bVersionEntry\b" crates/` не дал ни одного файла-конструктора сверх перечисленных
(остальные попадания — use/сигнатуры/доки: `cli/get.rs`, `cli/reindex.rs`, `cli/init.rs`,
`scanner/manifest.rs`, `vibe-publish/src/post_hook.rs`, `docs/*`).

### VersionEntry — struct-литералы (11 сайтов кода; ВСЕ исчерпывающие, ~30 полей; `..Default::default()` — 0 вхождений на весь `crates/`)

Продакшн:
1. `crates/vibe-index/src/types/entry/mod.rs:138-169` — тело `VersionEntry::minimal`.
2. `crates/vibe-index/src/scanner/org_walk.rs:202-233` — `build_entry` (сканер org-обхода; полный литерал, значения из `mfst::*`/вычислений).
3. `crates/vibe-index/src/cli/add.rs:84-115` — `run` команды `add` (полный литерал из манифеста).

Тесты (unit, `#[cfg(test)]`-модули):
4. `crates/vibe-index/src/types/entry/tests.rs:10-46` — хелпер `sample_entry` (модуль подключён `#[cfg(test)] mod tests;` — entry/mod.rs:179-180).
5. `crates/vibe-index/src/index/primary.rs:117-150` — хелпер `entry` (`#[cfg(test)] mod tests` — primary.rs:108).
6. `crates/vibe-index/src/index/memory.rs:322-353` — хелпер `entry` (`#[cfg(test)]` — memory.rs:303).
7. `crates/vibe-index/src/index/inverted.rs:297-342` — хелпер `entry` (`#[cfg(test)]` — inverted.rs:274).
8. `crates/vibe-index/src/index/by_name.rs:140-171` — хелпер `version_entry` (`#[cfg(test)]` — by_name.rs:121).

Интеграционные тесты:
9. `crates/vibe-index/tests/auto_publish.rs:63-97` — хелпер `entry`.
10. `crates/vibe-index/tests/server_writes.rs:23-57` — хелпер `entry`.
11. `crates/vibe-index/tests/server_e2e.rs:34-70` — хелпер `entry`.

Не-код: `crates/vibe-index/docs/format.md:70` — заголовок раздела документации.

Формы: клона-и-правки как отдельной формы нет; правка после клонирования встречается
локально в тестах (`v1.version = …` — tests.rs:80-84; `e.registry = …` —
tests/seam_fakes.rs:106), но исходник всегда — полный литерал или `minimal()`.

### VersionEntry — прочие пути (`::`)

- `VersionEntry::SCHEMA_VERSION` — в каждом из 8 литералов-хелперов/продакшна (tests.rs:11, primary.rs:118, memory.rs:323, inverted.rs:298, by_name.rs:141, org_walk.rs:203, add.rs:85, auto_publish.rs:64, server_writes.rs:24, server_e2e.rs:35).
- `VersionEntry::sort_key` — вызов `crates/vibe-index/src/index/primary.rs:28` (сортировка primary перед записью).
- `VersionEntry::minimal` — см. §5.

### NameEntry

Литералов `NameEntry {` вне конструктора нет. Конструирование только через `new`:
- продакшн: `crates/vibe-index/src/index/memory.rs:200` — `Index::write_to` строит каждый NameEntry с нуля при каждой записи каталога;
- тесты: `crates/vibe-index/src/types/entry/tests.rs:125`; `crates/vibe-index/src/index/by_name.rs:186, 205, 224, 226` (unit, cfg(test) на 121).

### PackageEntry

Литералов вне конструктора нет. Конструирование через `new`:
- продакшн: `crates/vibe-index/src/index/memory.rs:97` — `Index::upsert` создаёт хост-пакет при первой вставке версии;
- тесты: `crates/vibe-index/src/types/entry/tests.rs:72, 126, 131`; `crates/vibe-index/src/index/by_name.rs:175` (хелпер); интеграционный `crates/vibe-index/tests/server_e2e.rs:115`.
- Ложное попадание свипа: `crates/vibe-cli/src/commands/show/subskills.rs:24` — `SubskillsPackageEntry`, ДРУГОЙ тип в vibe-cli, не конструирует наш.

## 5. VersionEntry::minimal

Сигнатура дословно (`crates/vibe-index/src/types/entry/mod.rs:126-137`):

```rust
    /// An entry carrying just the `(kind, group, name, version)` identity,
    /// every other field empty or placeholder — the shape index tests and
    /// doctests reach for when only identity matters. Production entries
    /// are built field-by-field from a manifest (`vibe-index add`); this
    /// is the fixture builder, public so examples need not restate the
    /// whole struct.
    pub fn minimal(
        kind: PackageKind,
        group: Group,
        name: impl Into<String>,
        version: Version,
    ) -> Self {
```

- Видимость: **`pub`**, НЕ `pub(crate)` и НЕ под `#[cfg(test)]` — обычный метод
  `impl VersionEntry` (блок mod.rs:123), доступен из интеграционных тестов и доктестов.
- Вызыватели (продакшн): **нет ни одного**. Тесты: `crates/vibe-index/tests/seam_fakes.rs:100`
  (хелпер `sample_payload`, клон-и-правка `e.registry = "vibespecs".into()` на :106).
  Доктест: `crates/vibe-index/src/index/memory.rs:49-54` (`idx.upsert(VersionEntry::minimal(…))`
  в док-примере `Index`).
- Проставляемые поля — дословно тело (`crates/vibe-index/src/types/entry/mod.rs:138-169`):
  `schema_version: Self::SCHEMA_VERSION` (=1), `kind`, `group`, `name: name.into()`, `version`,
  `content_hash: "sha256:0".to_string()`, `source_url: String::new()`,
  `source_ref: String::new()`, `resolved_commit: None`, `registry: String::new()`,
  `workspace_origin: None`, `license: None`, `authors: Vec::new()`, `description: None`,
  `homepage: None`, `keywords: Vec::new()`, `describes: None`,
  `compatibility/provides/requires/obsoletes/conflicts/features/i18n: <T>::default()`,
  `requires_any: Vec::new()`, `subskills: Vec::new()`, `boot_snippet: None`,
  `files_count: 0`, `indexed_at: Utc::now()` (строка 167), `indexed_by: "vibe-index".to_string()`.

## 6. Путь чтения каталога и гранулярность отказа

Точки, где байты становятся типами записи (все — `Result` с `?`-распространением):

1. **`primary::parse`** — `crates/vibe-index/src/index/primary.rs:84-101`
   (`pub fn parse(bytes: &[u8]) -> Result<Vec<VersionEntry>>`); читатель `read` — :75-82.
   Гранулярность: построчный цикл **есть** — `for (lineno, line) in text.lines().enumerate()`
   (primary.rs:88); пустые строки пропускаются (:89-91); но неразобранная строка роняет ВЕСЬ
   вызов через `?`:

   ```rust
   let entry: VersionEntry = serde_json::from_str(line).map_err(|e| {
       Error::Malformed(format!(
           "primary.jsonl line {} is malformed: {e}",
           lineno + 1
       ))
   })?;
   ```
   (primary.rs:92-97.) Это единственное место с готовым циклом «по одной записи», внутрь
   которого физически ложится «пропуск с warn». Тест-свидетель: `malformed_line_surfaces_with_lineno`
   (primary.rs:227-235) — одна битая строка = отказ всего `parse`.

2. **`by_name::parse`** — `crates/vibe-index/src/index/by_name.rs:66-68`
   (`pub fn parse(bytes: &[u8]) -> Result<NameEntry>`); читатель `read` — :57-64.
   Гранулярность: файл целиком, одним вызовом:

   ```rust
   pub fn parse(bytes: &[u8]) -> Result<NameEntry> {
       serde_json::from_slice(bytes).map_err(|e| Error::Malformed(format!("by-name JSON: {e}")))
   }
   ```
   Цикла по записям внутри файла НЕТ. Обход `read_all` (:72-97) циклит по ФАЙЛАМ
   (`WalkDir`, :78-95), и `out.push(parse(&bytes)?)` на :94 роняет весь `read_all`
   при одном битом файле.

3. **`repomd::read`** — `crates/vibe-index/src/index/repomd.rs:32-39` — `from_slice` всего
   `repomd.json`; гранулярность — файл.

4. **`Index::load_from`** — `crates/vibe-index/src/index/memory.rs:262-281`:
   `repomd::read` (:263) → `by_name::read_all` (:264) → разворот в `by_pkgref`
   (цикл по name_entry/пакетам, :266-271). **Primary.jsonl при загрузке НЕ читается** —
   источник правды RAM-копии это by-name файлы (док на :258-261 так и говорит: on-disk shape
   is the source of truth). Гранулярность: одна битая запись в ЛЮБОМ by-name файле роняет всю
   загрузку (`?` на :264). Цикл по пакетам есть (:267), по версиям — нет (пакет вставляется целиком).

5. **HTTP-вход** — `crates/vibe-index/src/server/routes/packages.rs:232`:
   `Json(entry): Json<VersionEntry>` в `upsert` (:229-259) — одна запись на запрос;
   отказ локализован в запросе (экстрактор axum отвергнет тело до обработчика).
   Сериализационный выход записей: `single_version` → `Json<VersionEntry>` (:187-211),
   `PackageVersionsResponse.versions` (:177-185), CLI `dump` (cli/dump.rs:41-71).

6. Вне записей каталога: `checkpoint::load` (index/checkpoint.rs:55-70, `checkpoint.json`),
   `search.rs` — десериализаций нет, работает по RAM `Index` (search.rs:68, 120, 149).

Куда ложится «запись с незнакомым `must_understand` — пропуск с warn»: сегодня НИКУДА —
ни один читатель не пропускает запись; готовая точка вставки одна — цикл `primary::parse`
(primary.rs:88-99); для пути реальной загрузки (`load_from` → by-name) потребовался бы
новый построчный/по-записный разбор в `by_name::parse` или цикл :266-271.

## 7. Домашние формы serde

Все `skip_serializing_if` в `crates/vibe-index/src/types` (полный дословный свип — §7 отчёта
работника: relations.rs 7, entry/mod.rs 18, content.rs 9, aggregate.rs 1 = **35**, все в
`entry/**`; в `kinds.rs`/`repomd.rs`/`types/mod.rs` — ни одного) группируются ровно в
четыре предиката:

1. **`Option::is_none`** — 14 мест. Дословный пример
   (`crates/vibe-index/src/types/entry/mod.rs:59-60`):

   ```rust
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub resolved_commit: Option<String>,
   ```

2. **`Vec::is_empty`** — 12 мест. Пример (mod.rs:72-73):

   ```rust
   #[serde(default, skip_serializing_if = "Vec::is_empty")]
   pub authors: Vec<String>,
   ```

3. **`BTreeMap::is_empty`** — 2 места (content.rs:39-40, 41-42). Пример:

   ```rust
   #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
   pub features: BTreeMap<String, Vec<String>>,
   ```

4. **`<Подсекция>::is_empty`** — 7 мест, все в VersionEntry (mod.rs:87, 90, 93, 99, 102,
   105, 111). Пример (mod.rs:87-88):

   ```rust
   #[serde(default, skip_serializing_if = "CompatibilityEntry::is_empty")]
   pub compatibility: CompatibilityEntry,
   ```

- **`#[serde(default)]` без парного skip**: ни одного в `types/` — свип
  `rg -n '#\[serde\(default\)\]' crates/vibe-index/src/types | rg -v skip_serializing_if`
  дал пусто (exit 1). Всякая опциональность в записях каталога — всегда пара
  «default + skip». (Вне records: `checkpoint.rs:20-26` использует `default`/`default = "…"`
  без skip — но это state-файл, не записи каталога.)
- **Поля типа `bool`**: в `crates/vibe-index/src/types` их НЕТ — свип `\bbool\b` находит
  только сигнатуры `pub fn is_empty(&self) -> bool` (content.rs:46, 82; relations.rs:23, 36,
  51, 70, 83). Значит `yanked`/`frozen` заводят новую форму: либо всегда сериализовать
  (bool без атрибутов, как `files_count`), либо `skip_serializing_if = "is_false"`-предикат
  (новое имя), либо `Option<bool>` под домашний `Option::is_none`. Готовой формы в дереве нет.
- Для `must_understand: Vec<String>` домашняя форма существует буквально — (2).
- Для `tombstone` на `NameEntry`: у NameEntry НЕТ ни одного поля с serde-атрибутом (§3.1) —
  форму выбирают с нуля, ориентиром служит (1).

## 8. Тесты, сторожащие вид провода

Тестов, утверждающих КОНКРЕТНЫЙ JSON-вид ЗАПИСИ каталога, немного; полный свип
(`json!|contains(|golden|to_string(&|to_value`) по `crates/vibe-index/{src,tests}`:

| тест | что утверждает | сломает ли новое поле со skip (отсутствует, пока пусто) |
|---|---|---|
| `empty_subsections_are_omitted` — `crates/vibe-index/src/types/entry/tests.rs:62-68` | `!json.contains("provides")`, `!contains("requires_any")`, `!contains("subskills")` на сериализации полной записи | НЕТ (отрицательные утверждения; пока новое поле пусто — его нет в выводе) |
| `workspace_origin_round_trips_through_json` — tests.rs:105-118 | `json.contains("workspace_origin")` (поле установлено в Some) | НЕТ |
| `version_entry_round_trips_through_json` — tests.rs:49-59; `name_entry_finalise…` — tests.rs:120-142 | round-trip равенство структур | НЕТ |
| `delivery_mode_serde_kebab` — tests.rs:96-102 | `"\"lazy-push\""` — вид enum DeliveryMode | НЕТ |
| `package_kind_serde_lowercase`, `naming_convention_serde_matches_vibe_core_wire` — kinds.rs:148-187 | вид enum-строк (`"flow"`, `"fqdn"`, `"kind-name"`, `"kind/name"`) | НЕТ (enum вне записи) |
| `directory_serialises_with_kind_tag`, `file_serialises_with_size_and_sha256` — types/repomd.rs:118-133 | вид RepomdFileEntry, включая `!json.contains("kind")` для файловой вариации | НЕТ (не запись каталога) |
| server_e2e — `crates/vibe-index/tests/server_e2e.rs:213-214, 232-233` | `body.contains("\"capability\":\"interface:wal\"")`, `"binding_site":"package"` — вид inverted-СТРОК (CapabilityRow/PurlRow) | НЕТ — inverted-ряды собираются из отдельных полей (inverted.rs:111-117, 121-128) и новых слотов не понесут |
| cli_lifecycle — `crates/vibe-index/tests/cli_lifecycle.rs:34-36` | вид `repomd.json` (`"registry": "vibespecs"` и т.п.) | НЕТ |
| `malformed_line_surfaces_with_lineno` — primary.rs:227-235 | отказ parse на битой строке с номером строки | НЕТ (но связано с §6: если шаг сменит гранулярность на skip — тест предстоит ПЕРЕПИСАТЬ, сегодня он закрепляет «упасть») |
| content_hash_parity — `crates/vibe-index/tests/content_hash_parity.rs` | golden-хэши ФАЙЛОВ пакета (не JSON-вид записи) | НЕТ |

Golden-файлов JSON-вида записи в дереве нет (свип `golden` — только фикстуры пакетов и
content-hash). Человекочитаемый «сторож» вида — `crates/vibe-index/docs/format.md:70-114`
(не тест): его пример VersionEntry сегодня НЕ совпадает с кодом — см. §10.

Итог: появление НОВОГО поля со `skip_serializing_if` (отсутствующего в выводе, пока пусто)
не ломает ни один существующий тест.

## 9. Бюджет длины файлов

`wc -l` (дословный вывод в §7 отчёта работника): `types/mod.rs` — 28; `types/kinds.rs` — 214;
`types/repomd.rs` — 154; `entry/mod.rs` — 180; `entry/aggregate.rs` — 96; `entry/content.rs` —
101; `entry/relations.rs` — 86; `entry/tests.rs` — 151 (итого 1010). Бюджет 600 строк после
`cargo fmt`: **ни у одного файла не осталось меньше 80 строк до бюджета** — самый длинный
(kinds.rs, 214) отстоит от 600 на 386 строк. Раскол не требуется; добавление четырёх слотов
(≈4-8 строк на файл) в бюджет помещается с большим запасом.

## 10. Дыры и неожиданности

1. **B4 опровергнут по координате**: `minimal` объявлен на `entry/mod.rs:132`, а не :167;
   167 — строка `indexed_at: Utc::now(),` в его теле. План, режущий по :167, попал бы
   в середину литерала.
2. **B7 опровергнут по букве**: `signature` встречается один раз — prose в doc-комменте
   `crates/vibe-index/src/scanner/from_github.rs:89` («…shares [`FromClonesOptions`] through
   the seam signature.»). Идентификаторов/полей с этими именами нет: `yank`/`tombstone`/
   `must_understand` — 0 вхождений. Для нарезки слотов дерево чисто.
3. **`Index::load_from` не читает `primary.jsonl`** (memory.rs:262-271): RAM-копия
   собирается ИЗ BY-NAME ФАЙЛОВ. «Пропуск записи с незнакомым must_understand», встроенный
   только в `primary::parse`, реальную загрузку не спасёт — там другой парсер
   (`by_name::parse`, файл целиком).
4. **Ни одного bool-поля в `types/`** — `yanked`/`frozen` создают новую serde-форму (§7),
   в дереве нет прецедента.
5. **Ни одного `..Default::default()` ни в одном литерале записей во всём `crates/`**
   (свип дал 0): 11 сайтов `VersionEntry` перечисляют все ~30 полей поимённо. Цена нового
   поля без `#[serde(default)]`-парности — правка всех 11 (3 продакшн + 8 тестовых,
   считая minimal). `VersionEntry` не реализует `Default` (derive отсутствует — mod.rs:37).
6. **`deny_unknown_fields` на всех 15 агрегатах** (B3): старый бинарник (без нового поля)
   откажет читать НОВЫЙ файл, где слот непуст — выкатка писателя/читателя должна идти
   в порядке «сначала читатели». Обратная совместимость (новый читатель + старый файл)
   обеспечивается парой `default + skip`.
7. **`tombstone` на `NameEntry` теряется при каждой записи каталога**, если его не
   протащить явно: `Index::write_to` строит NameEntry заново через `NameEntry::new`
   (memory.rs:196-203) из RAM-карты `by_pkgref`, где tombstone жить не должен по плану
   (слот уровня NameEntry). Нужен отдельный носитель tombstone между загрузкой и записью,
   иначе `vibe-index add` сотрёт надгробие.
8. **`docs/format.md` отстал от кода дважды** (не тест, но документ-провод):
   пример VersionEntry (format.md:75-108) не содержит поля `group` (в коде есть —
   mod.rs:51, PROP-008), а заголовок :55 описывает старый путь `by-name/<kind>/<name>.json`
   (код пишет `by-name/<name>.json` — by_name.rs:24, 30-32). Утверждение :111-114 «`null`
   appears for actual operator-omitted optionals (`homepage`, `describes`, `resolved_commit`)»
   неверно: все три несут `skip_serializing_if = "Option::is_none"` (mod.rs:74-85) —
   они ОПУСКАЮТСЯ, не пишутся как null. Шагу Ф1.4 следует править format.md вместе с типами,
   иначе разрыв вырастет ещё на четыре слота.
9. **`PackageEntry::finalise` и `NameEntry::finalise` пересобирают агрегаты** при каждом
   upsert/записи (aggregate.rs:47-55, 90-96) — поля-слоты уровнем ниже переживут (они в
   VersionEntry), но любые ВЫЧИСЛЯЕМЫЕ следствия слотов (например, «не показывать yanked
   в latest_stable») должны лечь в finalise, иначе latest_stable продолжит выбирать
   yanked-версию.
10. **`RequiresAnyEntry` — единственная подсекция без `Default` и без skip на поле**
    (relations.rs:56-60): образец «наоборот», не форма для подражания.
11. **Единственный вызов `minimal` в коде — тест** (seam_fakes.rs:100) плюс доктест
    (memory.rs:49-54): править сигнатуру minimal дёшево, вызывателей два.
12. Свип `PackageEntry` ловит ложное `SubskillsPackageEntry` из vibe-cli
    (commands/show/subskills.rs:24) — при повторных замерах фильтровать по crate.

## 11. Как воспроизвести этот замер

Одна команда на глагол (Git Bash из корня рабочего дерева; `rg` — ripgrep):

1. Перечислить файлы типов: `ls crates/vibe-index/src/types crates/vibe-index/src/types/entry crates/vibe-index/src/index crates/vibe-index/tests`
2. Сосчитать `deny_unknown_fields` по файлам (B3): `rg -c "deny_unknown_fields" crates/vibe-index/src/types/entry/mod.rs crates/vibe-index/src/types/entry/aggregate.rs crates/vibe-index/src/types/entry/content.rs crates/vibe-index/src/types/entry/relations.rs crates/vibe-index/src/types/repomd.rs`
3. Найти зарезервированные слова слотов (B7): `rg -n "yank|tombstone|must_understand|signature" crates/vibe-index/src`
4. Найти литералы VersionEntry: `rg -n "VersionEntry\s*\{" crates/`
5. Найти пути VersionEntry (включая minimal/sort_key/SCHEMA_VERSION): `rg -n "VersionEntry::" crates/`
6. То же для агрегатов: `rg -n "NameEntry\s*\{|NameEntry::" crates/` и `rg -n "PackageEntry\s*\{|PackageEntry::" crates/`
7. Проверить отсутствие `..Default::default()`: `rg -n "\.\.Default::default\(\)|Default::default\(\)" crates/vibe-index/src crates/vibe-index/tests`
8. Все вызовы minimal: `rg -n "\bminimal\(" crates/`
9. Разложить skip-предикаты (B8): `rg -c "Option::is_none" crates/vibe-index/src/types/entry`; `rg -c "Vec::is_empty" crates/vibe-index/src/types/entry`; `rg -c "BTreeMap::is_empty" crates/vibe-index/src/types/entry`; `rg -c "Entry::is_empty" crates/vibe-index/src/types/entry`; контроль суммы: `rg -c "skip_serializing_if" crates/vibe-index/src/types/entry`
10. `default` без парного skip: `rg -n '#\[serde\(default\)\]' crates/vibe-index/src/types | rg -v skip_serializing_if` (пусто = exit 1)
11. Bool-поля: `rg -n "\bbool\b" crates/vibe-index/src/types`
12. Точки десериализации: `rg -n "from_str|from_slice|Json<" crates/vibe-index/src/index crates/vibe-index/src/server`
13. Тесты вида провода: `rg -n "json!|\.contains\(|golden|to_string\(&|to_value" crates/vibe-index/src crates/vibe-index/tests`
14. Бюджет длины: `wc -l crates/vibe-index/src/types/mod.rs crates/vibe-index/src/types/kinds.rs crates/vibe-index/src/types/repomd.rs crates/vibe-index/src/types/entry/mod.rs crates/vibe-index/src/types/entry/aggregate.rs crates/vibe-index/src/types/entry/content.rs crates/vibe-index/src/types/entry/relations.rs crates/vibe-index/src/types/entry/tests.rs`
15. Прочитать декларации и тела: `types/entry/mod.rs`, `types/entry/aggregate.rs`, `types/entry/content.rs`, `types/entry/relations.rs`, `types/entry/tests.rs`, `types/kinds.rs`, `types/repomd.rs`, `index/{primary,by_name,memory,inverted,repomd,checkpoint,search}.rs`, `cli/{add,get,dump,reindex}.rs`, `scanner/org_walk.rs`, `server/routes/packages.rs`, `tests/{auto_publish,server_writes,server_e2e,seam_fakes}.rs`, `docs/format.md`.
